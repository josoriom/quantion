use crate::{
    ErrorKind, FromTo, GetScanMetadataOptions, GetScansOptions, ParseIonOptions, ScanParam, ScanParamKind, ScanQuery, WriteIonOptions, get_scan_metadata,
    get_scans, parse_ion, parse_mzml, write_ion,
};
use crate::sample_file::SampleFile;

const SCAN_COUNT: usize = 6;
const BASE64_OF_TWO_FLOATS: &str = "AAAAAAAAWUAAAAAAAABpQA==";

fn get_level(position: usize) -> usize {
    if position.is_multiple_of(2) { 1 } else { 2 }
}

fn write_precursor(position: usize) -> String {
    if get_level(position) == 1 {
        return String::new();
    }
    let mz = 300 + position;
    format!(
        r#"<precursorList count="1"><precursor spectrumRef="scan={position}"><isolationWindow><cvParam cvRef="MS" accession="MS:1000827" name="isolation window target m/z" value="{mz}" unitCvRef="MS" unitAccession="MS:1000040" unitName="m/z"/></isolationWindow><selectedIonList count="2"><selectedIon><cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="{mz}" unitCvRef="MS" unitAccession="MS:1000040" unitName="m/z"/></selectedIon><selectedIon><cvParam cvRef="MS" accession="MS:1000041" name="charge state" value="2"/></selectedIon></selectedIonList><activation><cvParam cvRef="MS" accession="MS:1000133" name="collision-induced dissociation"/></activation></precursor></precursorList>"#
    )
}

fn write_array(accession: &str, name: &str) -> String {
    format!(
        r#"<binaryDataArray encodedLength="24"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/><cvParam cvRef="MS" accession="MS:1000576" name="no compression"/><cvParam cvRef="MS" accession="{accession}" name="{name}"/><binary>{BASE64_OF_TWO_FLOATS}</binary></binaryDataArray>"#
    )
}

fn write_spectrum(position: usize) -> String {
    let level = get_level(position);
    let start = position + 1;
    format!(
        r#"<spectrum index="{position}" id="scan={start}" defaultArrayLength="2" dataProcessingRef="synthetic_processing"><referenceableParamGroupRef ref="common_params"/><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="{level}"/><cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/><cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/><userParam name="library_id" value="SYNTHETIC{position:03}"/><userParam name="formula" value="C{start}H{level}"/><userParam name="library_id" value="REPEATED"/><scanList count="1"><cvParam cvRef="MS" accession="MS:1000795" name="no combination"/><scan instrumentConfigurationRef="synthetic_instrument"><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{start}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/><userParam name="scan_note" value="synthetic note"/><scanWindowList count="1"><scanWindow><cvParam cvRef="MS" accession="MS:1000501" name="scan window lower limit" value="50" unitCvRef="MS" unitAccession="MS:1000040" unitName="m/z"/></scanWindow></scanWindowList></scan></scanList>{}<binaryDataArrayList count="2">{}{}</binaryDataArrayList></spectrum>"#,
        write_precursor(position),
        write_array("MS:1000514", "m/z array"),
        write_array("MS:1000515", "intensity array"),
    )
}

fn open_mzml() -> SampleFile {
    let spectra: String = (0..SCAN_COUNT).map(write_spectrum).collect();
    let text = format!(
        r#"<?xml version="1.0"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><cvList count="1"><cv id="MS" fullName="x" URI="x"/></cvList><referenceableParamGroupList count="1"><referenceableParamGroup id="common_params"><userParam name="group_note" value="shared"/></referenceableParamGroup></referenceableParamGroupList><run id="r"><spectrumList count="{SCAN_COUNT}" defaultDataProcessingRef="d">{spectra}</spectrumList></run></mzML>"#
    );
    parse_mzml(text.into_bytes()).expect("open mzML")
}

fn open_ion() -> SampleFile {
    let bytes = write_ion(&mut open_mzml(), WriteIonOptions::default()).expect("write").expect("bytes");
    parse_ion(bytes, ParseIonOptions::default()).expect("open ion")
}

fn row(path: &str, kind: ScanParamKind, accession: Option<&str>, name: &str, value: Option<&str>, unit: Option<(&str, &str)>) -> ScanParam {
    ScanParam {
        path: path.to_string(),
        kind,
        accession: accession.map(str::to_string),
        name: name.to_string(),
        value: value.map(str::to_string),
        unit: unit.map(|(name, _)| name.to_string()),
        unit_accession: unit.map(|(_, accession)| accession.to_string()),
    }
}

fn expected_rows(position: usize) -> Vec<ScanParam> {
    let level = get_level(position).to_string();
    let start = (position + 1).to_string();
    let mz = (300 + position).to_string();
    let id = format!("scan={start}");
    let library_id = format!("SYNTHETIC{position:03}");
    let formula = format!("C{start}H{level}");
    let scan = "spectrum/scanList/scan[0]";
    let mz_unit = Some(("m/z", "MS:1000040"));
    let mut rows = vec![
        row("spectrum", ScanParamKind::ATTRIBUTE, None, "index", Some(&position.to_string()), None),
        row("spectrum", ScanParamKind::ATTRIBUTE, None, "id", Some(&id), None),
        row("spectrum", ScanParamKind::ATTRIBUTE, None, "defaultArrayLength", Some("2"), None),
        row("spectrum", ScanParamKind::ATTRIBUTE, None, "dataProcessingRef", Some("synthetic_processing"), None),
        row("spectrum/referenceableParamGroupRef[0]", ScanParamKind::ATTRIBUTE, None, "ref", Some("common_params"), None),
        row("spectrum", ScanParamKind::CV, Some("MS:1000511"), "ms level", Some(&level), None),
        row("spectrum", ScanParamKind::CV, Some("MS:1000127"), "centroid spectrum", None, None),
        row("spectrum", ScanParamKind::CV, Some("MS:1000130"), "positive scan", None, None),
        row("spectrum", ScanParamKind::USER, None, "library_id", Some(&library_id), None),
        row("spectrum", ScanParamKind::USER, None, "formula", Some(&formula), None),
        row("spectrum", ScanParamKind::USER, None, "library_id", Some("REPEATED"), None),
        row("spectrum/scanList", ScanParamKind::CV, Some("MS:1000795"), "no combination", None, None),
        row(scan, ScanParamKind::ATTRIBUTE, None, "instrumentConfigurationRef", Some("synthetic_instrument"), None),
        row(scan, ScanParamKind::CV, Some("MS:1000016"), "scan start time", Some(&start), Some(("minute", "UO:0000031"))),
        row(scan, ScanParamKind::USER, None, "scan_note", Some("synthetic note"), None),
        row(&format!("{scan}/scanWindowList/scanWindow[0]"), ScanParamKind::CV, Some("MS:1000501"), "scan window lower limit", Some("50"), mz_unit),
    ];
    if level == "2" {
        let precursor = "spectrum/precursorList/precursor[0]";
        let previous = format!("scan={position}");
        rows.extend([
            row(precursor, ScanParamKind::ATTRIBUTE, None, "spectrumRef", Some(&previous), None),
            row(&format!("{precursor}/isolationWindow"), ScanParamKind::CV, Some("MS:1000827"), "isolation window target m/z", Some(&mz), mz_unit),
            row(&format!("{precursor}/selectedIonList/selectedIon[0]"), ScanParamKind::CV, Some("MS:1000744"), "selected ion m/z", Some(&mz), mz_unit),
            row(&format!("{precursor}/selectedIonList/selectedIon[1]"), ScanParamKind::CV, Some("MS:1000041"), "charge state", Some("2"), None),
            row(&format!("{precursor}/activation"), ScanParamKind::CV, Some("MS:1000133"), "collision-induced dissociation", None, None),
        ]);
    }
    for (position, (accession, name)) in [("MS:1000514", "m/z array"), ("MS:1000515", "intensity array")].into_iter().enumerate() {
        let array = format!("spectrum/binaryDataArrayList/binaryDataArray[{position}]");
        rows.extend([
            row(&array, ScanParamKind::ATTRIBUTE, None, "encodedLength", Some("24"), None),
            row(&array, ScanParamKind::CV, Some("MS:1000523"), "64-bit float", None, None),
            row(&array, ScanParamKind::CV, Some("MS:1000576"), "no compression", None, None),
            row(&array, ScanParamKind::CV, Some(accession), name, None, None),
        ]);
    }
    rows
}

fn every_tag() -> GetScanMetadataOptions {
    GetScanMetadataOptions::default()
}

fn only(tags: &[&str]) -> GetScanMetadataOptions {
    GetScanMetadataOptions { tags: tags.iter().map(|tag| tag.to_string()).collect() }
}

fn whole_run(level: u8) -> GetScansOptions {
    GetScansOptions { level, ..Default::default() }
}

fn every_scan() -> ScanQuery {
    ScanQuery::RtRange(FromTo { from: 0.0, to: 100.0 })
}

#[test]
fn scan_index_is_the_position_in_the_spectrum_list() {
    for mut file in [open_mzml(), open_ion()] {
        let all: Vec<usize> = get_scans(&mut file, every_scan(), whole_run(0)).expect("scans").iter().map(|scan| scan.index).collect();
        assert_eq!(all, vec![0, 1, 2, 3, 4, 5]);
        let level_two: Vec<usize> = get_scans(&mut file, every_scan(), whole_run(2)).expect("scans").iter().map(|scan| scan.index).collect();
        assert_eq!(level_two, vec![1, 3, 5]);
    }
}

#[test]
fn scan_index_is_set_when_reading_chosen_arrays() {
    for mut file in [open_mzml(), open_ion()] {
        let options = GetScansOptions {
            level: 2,
            arrays: vec![crate::ScanArray { key: "intensity".to_string(), accession: "MS:1000515".to_string() }],
            ..Default::default()
        };
        let indices: Vec<usize> = get_scans(&mut file, every_scan(), options).expect("scans").iter().map(|scan| scan.index).collect();
        assert_eq!(indices, vec![1, 3, 5]);
    }
}

#[test]
fn closest_scan_has_its_index() {
    for mut file in [open_mzml(), open_ion()] {
        let scans = get_scans(&mut file, ScanQuery::Mz(303.0), whole_run(2)).expect("scans");
        assert_eq!(scans.len(), 1);
        assert_eq!(scans[0].index, 3);
    }
}

#[test]
fn metadata_has_every_tag_in_tree_order() {
    for mut file in [open_mzml(), open_ion()] {
        for position in 0..SCAN_COUNT {
            assert_eq!(get_scan_metadata(&mut file, position, every_tag()).expect("metadata"), expected_rows(position), "scan {position}");
        }
    }
}

#[test]
fn metadata_is_the_same_from_mzml_and_ion() {
    let mut mzml = open_mzml();
    let mut ion = open_ion();
    for position in 0..SCAN_COUNT {
        assert_eq!(
            get_scan_metadata(&mut mzml, position, every_tag()).expect("mzML"),
            get_scan_metadata(&mut ion, position, every_tag()).expect("ion")
        );
    }
}

#[test]
fn metadata_of_found_scans_matches_by_index() {
    for mut file in [open_mzml(), open_ion()] {
        for scan in get_scans(&mut file, every_scan(), whole_run(2)).expect("scans") {
            let rows = get_scan_metadata(&mut file, scan.index, every_tag()).expect("metadata");
            let library_id = rows.iter().find(|row| row.name == "library_id").and_then(|row| row.value.clone());
            assert_eq!(library_id, Some(format!("SYNTHETIC{:03}", scan.index)));
        }
    }
}

#[test]
fn tags_keep_only_the_chosen_tags() {
    for mut file in [open_mzml(), open_ion()] {
        let rows = get_scan_metadata(&mut file, 1, only(&["selectedIon"])).expect("metadata");
        let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, vec!["selected ion m/z", "charge state"]);
        let rows = get_scan_metadata(&mut file, 1, only(&["activation", "scan"])).expect("metadata");
        let paths: Vec<&str> = rows.iter().map(|row| row.path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "spectrum/scanList/scan[0]",
                "spectrum/scanList/scan[0]",
                "spectrum/scanList/scan[0]",
                "spectrum/precursorList/precursor[0]/activation",
            ]
        );
    }
}

#[test]
fn a_tag_that_is_not_there_gives_no_rows() {
    for mut file in [open_mzml(), open_ion()] {
        assert!(get_scan_metadata(&mut file, 0, only(&["precursor"])).expect("metadata").is_empty());
    }
}

#[test]
fn an_unknown_tag_is_invalid() {
    for mut file in [open_mzml(), open_ion()] {
        for tag in ["cvParam", "Scan", "scan[0]", ""] {
            let error = get_scan_metadata(&mut file, 0, only(&[tag])).expect_err("unknown tag");
            assert_eq!(error.kind, ErrorKind::InvalidArgument);
            assert_eq!(error.argument.as_deref(), Some("tags"));
        }
    }
}

#[test]
fn index_out_of_range_is_invalid() {
    for mut file in [open_mzml(), open_ion()] {
        let error = get_scan_metadata(&mut file, SCAN_COUNT, every_tag()).expect_err("out of range");
        assert_eq!(error.kind, ErrorKind::InvalidArgument);
        assert_eq!(error.argument.as_deref(), Some("index"));
        assert!(error.message.contains("6"), "{}", error.message);
    }
}

#[test]
fn index_cannot_be_an_array_key() {
    let mut file = open_ion();
    let options = GetScansOptions {
        arrays: vec![crate::ScanArray { key: "index".to_string(), accession: "MS:1000786".to_string() }],
        ..Default::default()
    };
    let error = get_scans(&mut file, every_scan(), options).expect_err("reserved key");
    assert_eq!(error.kind, ErrorKind::InvalidArgument);
    assert_eq!(error.argument.as_deref(), Some("arrays.key"));
}
