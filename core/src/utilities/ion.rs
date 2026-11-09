use ionic::{
    ArrayKind, IonError, IonReader, IonResult, IonWriter, SpectrumSummary, TimeUnit, WriteOptions,
    mzml::structs::{CvParam, MzML, NumericArray, Spectrum},
    source::WriteBytes,
};

use crate::{files::ScanParam, utilities::get_scan_metadata::get_scan_params};

const MZ_ARRAY: u32 = 1_000_514;
const INTENSITY_ARRAY: u32 = 1_000_515;
const SCAN_START_TIME: u32 = 1_000_016;
const MS_LEVEL: u32 = 1_000_511;
const BASE_PEAK_MZ: u32 = 1_000_504;
const BASE_PEAK_INT: u32 = 1_000_505;
const TOTAL_ION_CURRENT: u32 = 1_000_285;
const SELECTED_ION_MZ: u32 = 1_000_744;
const POSITIVE_SCAN: u32 = 1_000_130;
const NEGATIVE_SCAN: u32 = 1_000_129;
const POSITION_X: u32 = 1_000_050;
const POSITION_Y: u32 = 1_000_051;
const POSITION_Z: u32 = 1_000_052;
const UNIT_MINUTE: u32 = 31;
const UNIT_SECOND: u32 = 10;
const UNIT_MS: u32 = 28;

#[derive(Debug, Clone, Copy)]
pub struct ScanSummary {
    pub rt: f64,
    pub rt_unit: TimeUnit,
    pub ms_level: u8,
    pub polarity: u8,
    pub selected_ion_mz: f64,
    pub base_peak_mz: f64,
    pub base_peak_int: f64,
    pub total_ion_current: f64,
    pub position_x: u32,
    pub position_y: u32,
    pub position_z: u32,
}

pub trait ScanSource {
    fn for_each_summary(&mut self, callback: &mut dyn FnMut(usize, ScanSummary));
    fn load_scan(&mut self, index: usize, mz: &mut Vec<f64>, intensity: &mut Vec<f64>) -> bool;
    fn load_array(&mut self, index: usize, accession: u32, values: &mut Vec<f64>) -> bool;
    fn load_metadata(&mut self, index: usize) -> IonResult<Vec<ScanParam>>;
}

impl ScanSource for IonReader {
    fn for_each_summary(&mut self, callback: &mut dyn FnMut(usize, ScanSummary)) {
        for index in 0..self.spectrum_count() as usize {
            let Some(record) = self.spectrum_summary(index) else {
                continue;
            };
            callback(index, summary_from_record(&record));
        }
    }

    fn load_scan(&mut self, index: usize, mz: &mut Vec<f64>, intensity: &mut Vec<f64>) -> bool {
        mz.clear();
        intensity.clear();
        if self.spectrum_array_into(index, ArrayKind::Mz, mz).is_err() {
            return false;
        }
        if self
            .spectrum_array_into(index, ArrayKind::Intensity, intensity)
            .is_err()
        {
            return false;
        }
        mz.len().min(intensity.len()) > 0
    }

    fn load_array(&mut self, index: usize, accession: u32, values: &mut Vec<f64>) -> bool {
        values.clear();
        self.spectrum_array_into(index, accession, values).is_ok() && !values.is_empty()
    }

    fn load_metadata(&mut self, index: usize) -> IonResult<Vec<ScanParam>> {
        let spectrum = self.spectrum_metadata_at(index)?;
        Ok(get_scan_params(&spectrum))
    }
}

impl ScanSource for MzML {
    fn for_each_summary(&mut self, callback: &mut dyn FnMut(usize, ScanSummary)) {
        let Some(list) = self.run.spectrum_list.as_ref() else {
            return;
        };
        for (index, spectrum) in list.spectra.iter().enumerate() {
            callback(index, summary_from_spectrum(spectrum));
        }
    }

    fn load_scan(&mut self, index: usize, mz: &mut Vec<f64>, intensity: &mut Vec<f64>) -> bool {
        let spectra = self
            .run
            .spectrum_list
            .as_ref()
            .map(|list| list.spectra.as_slice())
            .unwrap_or_default();
        load_scan_from_spectra(spectra, index, mz, intensity)
    }

    fn load_array(&mut self, index: usize, accession: u32, values: &mut Vec<f64>) -> bool {
        values.clear();
        let spectra = self
            .run
            .spectrum_list
            .as_ref()
            .map(|list| list.spectra.as_slice())
            .unwrap_or_default();
        load_array_from_spectra(spectra, index, accession, values)
    }

    fn load_metadata(&mut self, index: usize) -> IonResult<Vec<ScanParam>> {
        let spectra = self
            .run
            .spectrum_list
            .as_ref()
            .map(|list| list.spectra.as_slice())
            .unwrap_or_default();
        let spectrum = spectra.get(index).ok_or(IonError::OutOfRange {
            index,
            count: spectra.len() as u64,
        })?;
        Ok(get_scan_params(spectrum))
    }
}

pub fn write_mzml_to_ion(
    mzml: &MzML,
    options: WriteOptions,
    output: &mut dyn WriteBytes,
) -> IonResult<()> {
    let mut writer = IonWriter::to(output, mzml, &options)?;
    if let Some(list) = &mzml.run.spectrum_list {
        for spectrum in &list.spectra {
            writer.write_spectrum(spectrum)?;
        }
    }
    if let Some(list) = &mzml.run.chromatogram_list {
        for chromatogram in &list.chromatograms {
            writer.write_chromatogram(chromatogram)?;
        }
    }
    writer.finish()
}

fn summary_from_record(record: &SpectrumSummary) -> ScanSummary {
    ScanSummary {
        rt: record.rt,
        rt_unit: time_unit_from_code(record.rt_unit),
        ms_level: record.ms_level,
        polarity: record.polarity,
        selected_ion_mz: record.selected_ion_mz,
        base_peak_mz: record.base_peak_mz,
        base_peak_int: record.base_peak_int,
        total_ion_current: record.total_ion_current,
        position_x: record.position_x,
        position_y: record.position_y,
        position_z: record.position_z,
    }
}

fn time_unit_from_code(code: u8) -> TimeUnit {
    match code {
        1 => TimeUnit::Second,
        2 => TimeUnit::Minute,
        3 => TimeUnit::Millisecond,
        _ => TimeUnit::Other,
    }
}

fn summary_from_spectrum(spectrum: &Spectrum) -> ScanSummary {
    let mut rt = f64::NAN;
    let mut rt_unit = TimeUnit::Other;
    let mut ms_level = spectrum
        .ms_level
        .and_then(|level| u8::try_from(level).ok())
        .unwrap_or(0);
    let mut polarity = 0u8;
    let mut base_peak_mz = f64::NAN;
    let mut base_peak_int = f64::NAN;
    let mut total_ion_current = f64::NAN;

    for param in &spectrum.cv_params {
        match accession_tail(param.accession.as_deref()) {
            MS_LEVEL => {
                if let Some(value) = param.value.as_deref().and_then(|v| v.parse().ok()) {
                    ms_level = value;
                }
            }
            BASE_PEAK_MZ => base_peak_mz = parse_f64(param.value.as_deref()),
            BASE_PEAK_INT => base_peak_int = parse_f64(param.value.as_deref()),
            TOTAL_ION_CURRENT => total_ion_current = parse_f64(param.value.as_deref()),
            POSITIVE_SCAN => polarity = 1,
            NEGATIVE_SCAN => polarity = 2,
            _ => {}
        }
    }

    let scan_list = spectrum.scan_list.as_ref().or_else(|| {
        spectrum
            .spectrum_description
            .as_ref()
            .and_then(|description| description.scan_list.as_ref())
    });

    'find_rt: {
        let Some(scan_list) = scan_list else {
            break 'find_rt;
        };
        for scan in &scan_list.scans {
            if let Some((value, unit)) = rt_from_params(&scan.cv_params) {
                rt = value;
                rt_unit = unit;
                break 'find_rt;
            }
        }
        if let Some((value, unit)) = rt_from_params(&scan_list.cv_params) {
            rt = value;
            rt_unit = unit;
        }
    }

    let mut position_x = 0u32;
    let mut position_y = 0u32;
    let mut position_z = 0u32;
    if let Some(scan_list) = scan_list {
        for scan in &scan_list.scans {
            for param in &scan.cv_params {
                match accession_tail(param.accession.as_deref()) {
                    POSITION_X => position_x = parse_u32(param.value.as_deref()),
                    POSITION_Y => position_y = parse_u32(param.value.as_deref()),
                    POSITION_Z => position_z = parse_u32(param.value.as_deref()),
                    _ => {}
                }
            }
        }
    }

    let selected_ion_mz = spectrum
        .precursor_list
        .as_ref()
        .and_then(|precursor_list| precursor_list.precursors.first())
        .and_then(|precursor| precursor.selected_ion_list.as_ref())
        .and_then(|selected_ion_list| selected_ion_list.selected_ions.first())
        .map(|selected_ion| {
            selected_ion
                .cv_params
                .iter()
                .find(|param| accession_tail(param.accession.as_deref()) == SELECTED_ION_MZ)
                .and_then(|param| param.value.as_deref()?.parse().ok())
                .unwrap_or(f64::NAN)
        })
        .unwrap_or(f64::NAN);

    ScanSummary {
        rt,
        rt_unit,
        ms_level,
        polarity,
        selected_ion_mz,
        base_peak_mz,
        base_peak_int,
        total_ion_current,
        position_x,
        position_y,
        position_z,
    }
}

fn accession_tail(accession: Option<&str>) -> u32 {
    let text = accession.unwrap_or("");
    let tail = text.rsplit_once(':').map(|(_, t)| t).unwrap_or(text);
    let mut value: u32 = 0;
    let mut saw = false;
    for byte in tail.bytes() {
        if byte.is_ascii_digit() {
            saw = true;
            value = match value
                .checked_mul(10)
                .and_then(|v| v.checked_add((byte - b'0') as u32))
            {
                Some(next) => next,
                None => return 0,
            };
        }
    }
    if saw { value } else { 0 }
}

fn rt_from_params(params: &[CvParam]) -> Option<(f64, TimeUnit)> {
    for param in params {
        if accession_tail(param.accession.as_deref()) == SCAN_START_TIME {
            let value: f64 = param.value.as_deref()?.parse().ok()?;
            if !value.is_finite() {
                return None;
            }
            let unit = time_unit_from(param.unit_accession.as_deref(), param.unit_name.as_deref());
            return Some((value, unit));
        }
    }
    None
}

fn time_unit_from(unit_accession: Option<&str>, unit_name: Option<&str>) -> TimeUnit {
    match accession_tail(unit_accession) {
        UNIT_MINUTE => TimeUnit::Minute,
        UNIT_SECOND => TimeUnit::Second,
        UNIT_MS => TimeUnit::Millisecond,
        _ => match unit_name {
            Some("minute" | "minutes") => TimeUnit::Minute,
            Some("second" | "seconds") => TimeUnit::Second,
            Some("millisecond" | "milliseconds") => TimeUnit::Millisecond,
            _ => TimeUnit::Other,
        },
    }
}

fn parse_f64(value: Option<&str>) -> f64 {
    value.and_then(|v| v.parse().ok()).unwrap_or(f64::NAN)
}

fn parse_u32(value: Option<&str>) -> u32 {
    value.and_then(|v| v.parse().ok()).unwrap_or(0)
}

fn binary_pair(spectrum: &Spectrum) -> Option<(&NumericArray, &NumericArray)> {
    let list = spectrum.binary_data_array_list.as_ref()?;
    let mut mz = None;
    let mut intensity = None;
    for array in &list.binary_data_arrays {
        if mz.is_some() && intensity.is_some() {
            break;
        }
        let mut is_mz = false;
        let mut is_intensity = false;
        for param in &array.cv_params {
            match accession_tail(param.accession.as_deref()) {
                MZ_ARRAY => is_mz = true,
                INTENSITY_ARRAY => is_intensity = true,
                _ => {}
            }
            if is_mz && is_intensity {
                break;
            }
        }
        if is_mz {
            mz = array.binary.as_ref();
        }
        if is_intensity {
            intensity = array.binary.as_ref();
        }
    }
    Some((mz?, intensity?))
}

fn load_scan_from_spectra(
    spectra: &[Spectrum],
    index: usize,
    mz: &mut Vec<f64>,
    intensity: &mut Vec<f64>,
) -> bool {
    let Some(spectrum) = spectra.get(index) else {
        return false;
    };
    let Some((mz_data, intensity_data)) = binary_pair(spectrum) else {
        return false;
    };
    let len = mz_data.len().min(intensity_data.len());
    if len == 0 {
        return false;
    }
    mz.clear();
    mz.reserve(len);
    extend_from_binary(mz_data, mz, len);
    intensity.clear();
    intensity.reserve(len);
    extend_from_binary(intensity_data, intensity, len);
    true
}

fn load_array_from_spectra(
    spectra: &[Spectrum],
    index: usize,
    accession: u32,
    values: &mut Vec<f64>,
) -> bool {
    let Some(list) = spectra
        .get(index)
        .and_then(|spectrum| spectrum.binary_data_array_list.as_ref())
    else {
        return false;
    };
    let found = list.binary_data_arrays.iter().find(|array| {
        array
            .cv_params
            .iter()
            .any(|param| accession_tail(param.accession.as_deref()) == accession)
    });
    let Some(data) = found.and_then(|array| array.binary.as_ref()) else {
        return false;
    };
    values.extend_from_slice(&data.to_f64());
    !values.is_empty()
}

fn extend_from_binary(data: &NumericArray, out: &mut Vec<f64>, max: usize) {
    out.extend_from_slice(&data.to_f64()[..max]);
}
