use crate::{
    ErrorKind, FromTo, GetScansOptions, ParseIonOptions, Position, ScanArray, ScanQuery, TimeUnit,
    WriteIonOptions, get_scans, parse_ion, parse_mzml, sample_file::SampleFile, write_ion,
};

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const FINGERPRINT_LENGTH: usize = 256;

fn encode(values: &[f64]) -> String {
    let bytes: Vec<u8> = values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    let mut output = String::new();
    for chunk in bytes.chunks(3) {
        let group = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        output.push(BASE64[(group[0] >> 2) as usize] as char);
        output.push(BASE64[(((group[0] & 3) << 4) | (group[1] >> 4)) as usize] as char);
        output.push(if chunk.len() > 1 {
            BASE64[(((group[1] & 15) << 2) | (group[2] >> 6)) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            BASE64[(group[2] & 63) as usize] as char
        } else {
            '='
        });
    }
    output
}

fn write_array(accession: &str, name: &str, value: &str, values: &[f64]) -> String {
    let encoded = encode(values);
    format!(
        r#"<binaryDataArray arrayLength="{}" encodedLength="{}"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/><cvParam cvRef="MS" accession="MS:1000576" name="no compression" value=""/><cvParam cvRef="MS" accession="{accession}" name="{name}" value="{value}"/><binary>{encoded}</binary></binaryDataArray>"#,
        values.len(),
        encoded.len(),
    )
}

fn fingerprint(position: usize) -> Vec<f64> {
    (0..FINGERPRINT_LENGTH)
        .map(|index| (index as f64 - 128.0) / 256.0 + position as f64)
        .collect()
}

fn write_spectrum(position: usize) -> String {
    let mz = [100.5, 200.25, 300.125];
    let intensity = [10.0, 20.0, 30.0];
    let start = position + 1;
    let precursor = 300.0 + position as f64;
    format!(
        r#"<spectrum index="{position}" id="scan={start}" defaultArrayLength="3"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="2"/><cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/><scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{start}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList><precursorList count="1"><precursor><selectedIonList count="1"><selectedIon><cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="{precursor}"/></selectedIon></selectedIonList></precursor></precursorList><binaryDataArrayList count="3">{}{}{}</binaryDataArrayList></spectrum>"#,
        write_array("MS:1000514", "m/z array", "", &mz),
        write_array("MS:1000515", "intensity array", "", &intensity),
        write_array(
            "MS:1000786",
            "non-standard data array",
            "ms2_fingerprint",
            &fingerprint(position)
        ),
    )
}

fn open_mzml() -> SampleFile {
    let spectra: String = (0..3).map(write_spectrum).collect();
    let text = format!(
        r#"<?xml version="1.0"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><cvList count="1"><cv id="MS" fullName="x" URI="x"/></cvList><run id="r"><spectrumList count="3" defaultDataProcessingRef="d">{spectra}</spectrumList></run></mzML>"#
    );
    parse_mzml(text.into_bytes()).expect("open mzML")
}

fn open_ion() -> SampleFile {
    let bytes = write_ion(&mut open_mzml(), WriteIonOptions::default())
        .expect("write")
        .expect("bytes");
    parse_ion(bytes, ParseIonOptions::default()).expect("open ion")
}

fn level_two(arrays: Vec<ScanArray>) -> GetScansOptions {
    GetScansOptions {
        level: 2,
        time_unit: TimeUnit::MINUTES,
        arrays,
    }
}

fn array(key: &str, accession: &str) -> ScanArray {
    ScanArray {
        key: key.to_string(),
        accession: accession.to_string(),
    }
}

fn whole_run() -> ScanQuery {
    ScanQuery {
        rt: Position::Range(FromTo {
            from: 0.0,
            to: 10.0,
        }),
        ..Default::default()
    }
}

#[test]
fn default_options_read_mz_and_intensity_only() {
    for mut file in [open_mzml(), open_ion()] {
        let options = GetScansOptions {
            level: 2,
            ..Default::default()
        };
        let scans = get_scans(&mut file, whole_run(), options).expect("scans");
        assert_eq!(scans.len(), 3);
        assert_eq!(scans[0].mz, vec![100.5, 200.25, 300.125]);
        assert_eq!(scans[0].intensity, vec![10.0, 20.0, 30.0]);
    }
}

#[test]
fn custom_array_reads_back_exactly() {
    for mut file in [open_mzml(), open_ion()] {
        let options = level_two(vec![array("fingerprint", "MS:1000786")]);
        let scans = get_scans(&mut file, whole_run(), options).expect("scans");
        assert_eq!(scans.len(), 3);
        for (position, scan) in scans.iter().enumerate() {
            assert_eq!(scan["fingerprint"], fingerprint(position));
            assert!(scan.mz.is_empty());
            assert!(scan.intensity.is_empty());
            assert_eq!(scan.rt, (position + 1) as f64);
            assert_eq!(scan.precursor_mz, 300.0 + position as f64);
        }
    }
}

#[test]
fn custom_array_can_come_with_mz_and_intensity() {
    let mut file = open_ion();
    let options = level_two(vec![
        array("mz", "MS:1000514"),
        array("intensity", "MS:1000515"),
        array("fingerprint", "MS:1000786"),
    ]);
    let scans = get_scans(
        &mut file,
        ScanQuery {
            mz: Position::Range(FromTo {
                from: 300.5,
                to: 301.5,
            }),
            ..Default::default()
        },
        options,
    )
    .expect("scans");
    assert_eq!(scans.len(), 1);
    assert_eq!(scans[0].mz, vec![100.5, 200.25, 300.125]);
    assert_eq!(scans[0].intensity, vec![10.0, 20.0, 30.0]);
    assert_eq!(scans[0]["fingerprint"], fingerprint(1));
}

#[test]
fn missing_array_finds_no_scans() {
    let mut file = open_ion();
    let options = level_two(vec![array("missing", "MS:1000999")]);
    assert!(
        get_scans(&mut file, whole_run(), options)
            .expect("scans")
            .is_empty()
    );
}

#[test]
fn closest_scan_gives_one_scan() {
    let mut file = open_ion();
    let options = level_two(vec![array("fingerprint", "MS:1000786")]);
    let scans = get_scans(
        &mut file,
        ScanQuery {
            mz: Position::Closest(301.2),
            ..Default::default()
        },
        options,
    )
    .expect("scans");
    assert_eq!(scans.len(), 1);
    assert_eq!(scans[0]["fingerprint"], fingerprint(1));
}

#[test]
fn bad_arrays_are_refused() {
    let bad: Vec<(Vec<ScanArray>, &str)> = vec![
        (Vec::new(), "arrays"),
        (vec![array("", "MS:1000786")], "arrays.key"),
        (vec![array("rt", "MS:1000786")], "arrays.key"),
        (vec![array("fingerprint", "1000786")], "arrays.accession"),
        (vec![array("fingerprint", "MS:abc")], "arrays.accession"),
        (vec![array("mz", "MS:1000786")], "arrays"),
        (vec![array("masses", "MS:1000514")], "arrays"),
        (
            vec![array("a", "MS:1000786"), array("a", "MS:1000787")],
            "arrays.key",
        ),
    ];
    let mut file = open_ion();
    for (arrays, argument) in bad {
        let error = get_scans(&mut file, whole_run(), level_two(arrays))
            .err()
            .expect("an error");
        assert_eq!(error.kind, ErrorKind::InvalidArgument);
        assert_eq!(error.argument.as_deref(), Some(argument));
    }
}

#[test]
fn a_query_with_neither_rt_nor_mz_is_refused() {
    let error = get_scans(
        &mut open_ion(),
        ScanQuery::default(),
        GetScansOptions::default(),
    )
    .unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidArgument);
    assert_eq!(error.argument.as_deref(), Some("rt"));
}

#[test]
fn a_query_with_rt_and_mz_checks_both() {
    let query = ScanQuery {
        rt: Position::Closest(1.0),
        mz: Position::Closest(-1.0),
    };
    let error = get_scans(&mut open_ion(), query, GetScansOptions::default()).unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidArgument);
    assert_eq!(error.argument.as_deref(), Some("mz"));
}

#[test]
fn rt_and_mz_together_give_the_same_scans_on_ion_and_mzml() {
    let query = ScanQuery {
        rt: Position::Range(FromTo {
            from: 0.0,
            to: 1000.0,
        }),
        mz: Position::Range(FromTo {
            from: 0.0,
            to: 10000.0,
        }),
    };
    let options = GetScansOptions {
        level: 0,
        ..Default::default()
    };
    let from_ion: Vec<usize> = get_scans(&mut open_ion(), query, options.clone())
        .expect("ion")
        .iter()
        .map(|scan| scan.index)
        .collect();
    let from_mzml: Vec<usize> = get_scans(&mut open_mzml(), query, options)
        .expect("mzml")
        .iter()
        .map(|scan| scan.index)
        .collect();
    assert_eq!(from_ion, vec![0, 1, 2]);
    assert_eq!(from_ion, from_mzml);
}
