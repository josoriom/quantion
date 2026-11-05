use ionic::mzml::structs::{CvParam, MzML, Scan, ScanList, Spectrum, SpectrumList};

use crate::{
    ErrorKind, FromTo, GetScansOptions, ParseIonOptions, ScanQuery, WriteIonOptions, get_scans, parse_ion, write_ion,
};
use crate::sample_file::{FileSource, SampleFile};

const SCAN_COUNT: usize = 20;
const PEAK_COUNT: usize = 50;
const MEGABYTE: u64 = 1024 * 1024;
const BLOCK_SIZE_OFFSET: usize = 16;
const MZ_WINDOW_OFFSET: usize = 24;

fn param(accession: &str, name: &str, value: String, unit: Option<(&str, &str)>) -> CvParam {
    CvParam {
        cv_ref: Some("MS".to_string()),
        accession: Some(accession.to_string()),
        name: name.to_string(),
        value: Some(value),
        unit_cv_ref: unit.map(|_| "UO".to_string()),
        unit_name: unit.map(|(name, _)| name.to_string()),
        unit_accession: unit.map(|(_, accession)| accession.to_string()),
    }
}

fn make_spectrum(position: usize) -> Spectrum {
    let mz = (0..PEAK_COUNT).map(|peak| 100.0 + 18.0 * peak as f64 + position as f64 * 0.01).collect();
    let intensity = (0..PEAK_COUNT).map(|peak| 1000.0 + 10.0 * peak as f64 + position as f64).collect();
    let mut spectrum = Spectrum::new(format!("scan={}", position + 1), mz, intensity);
    spectrum.index = Some(position as u32);
    spectrum.cv_params.push(param("MS:1000511", "ms level", "1".to_string(), None));
    let start = param("MS:1000016", "scan start time", (position as f64 * 0.1).to_string(), Some(("minute", "UO:0000031")));
    spectrum.scan_list = Some(ScanList {
        count: Some(1),
        scans: vec![Scan { cv_params: vec![start], ..Default::default() }],
        ..Default::default()
    });
    spectrum
}

fn open_mzml() -> SampleFile {
    let mut mzml = MzML::default();
    mzml.run.spectrum_list = Some(SpectrumList {
        count: Some(SCAN_COUNT),
        spectra: (0..SCAN_COUNT).map(make_spectrum).collect(),
        ..Default::default()
    });
    SampleFile::new(FileSource::Full(Box::new(mzml)))
}

fn read_header(ion: &[u8]) -> (u64, u32) {
    let block_bytes = u64::from_le_bytes(ion[BLOCK_SIZE_OFFSET..BLOCK_SIZE_OFFSET + 8].try_into().expect("8 bytes"));
    let mz_window = u32::from_le_bytes(ion[MZ_WINDOW_OFFSET..MZ_WINDOW_OFFSET + 4].try_into().expect("4 bytes"));
    (block_bytes, mz_window)
}

fn get_peaks(file: &mut SampleFile) -> Vec<(Vec<f64>, Vec<f64>)> {
    let query = ScanQuery::RtRange(FromTo { from: 0.0, to: 100.0 });
    let scans = get_scans(file, query, GetScansOptions { level: 0, ..Default::default() }).expect("scans");
    scans.into_iter().map(|scan| (scan.mz, scan.intensity)).collect()
}

fn write_bytes(file: &mut SampleFile, options: WriteIonOptions) -> Vec<u8> {
    write_ion(file, options).expect("write").expect("bytes")
}

fn write_with_ionic(file: &mut SampleFile, block_bytes: usize, mz_window: f64) -> Vec<u8> {
    let options = ionic::WriteOptions { block_size: block_bytes, mz_window, ..Default::default() };
    file.with_mzml("write_ion", |mzml| {
        let mut bytes = Vec::new();
        crate::utilities::ion::write_mzml_to_ion(mzml, options, &mut bytes).expect("ionic write");
        Ok(bytes)
    })
    .expect("mzML")
}

#[test]
fn defaults_are_8_mb_blocks_and_20_mz_windows() {
    let options = WriteIonOptions::default();
    assert_eq!((options.block_size, options.mz_window), (8, 20));
    assert_eq!(read_header(&write_bytes(&mut open_mzml(), options)), (8 * MEGABYTE, 20));
}

#[test]
fn chosen_values_are_recorded_and_peaks_read_back_the_same() {
    let mut file = open_mzml();
    let expected = get_peaks(&mut file);
    assert_eq!(expected.len(), SCAN_COUNT);
    for (block_size, mz_window) in [(1, 20), (2, 5), (16, 0), (8, 100)] {
        let ion = write_bytes(&mut file, WriteIonOptions { block_size, mz_window, ..Default::default() });
        assert_eq!(read_header(&ion), (u64::from(block_size) * MEGABYTE, mz_window));
        let mut reopened = parse_ion(ion, ParseIonOptions::default()).expect("reopen");
        assert_eq!(get_peaks(&mut reopened), expected, "block_size {block_size}, mz_window {mz_window}");
    }
}

#[test]
fn values_reach_ionic_unchanged() {
    let mut file = open_mzml();
    for (block_size, mz_window) in [(1, 20), (2, 5), (16, 0)] {
        let bytes = write_bytes(&mut file, WriteIonOptions { block_size, mz_window, ..Default::default() });
        let expected = write_with_ionic(&mut file, block_size as usize * MEGABYTE as usize, f64::from(mz_window));
        assert_eq!(bytes, expected, "block_size {block_size}, mz_window {mz_window}");
    }
}

#[test]
fn a_block_size_of_0_is_refused() {
    let error = write_ion(&mut open_mzml(), WriteIonOptions { block_size: 0, ..Default::default() }).expect_err("zero");
    assert_eq!(error.kind, ErrorKind::InvalidArgument);
    assert_eq!(error.argument.as_deref(), Some("block_size"));
}
