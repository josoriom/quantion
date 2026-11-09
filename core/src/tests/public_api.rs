use std::fs;

use crate::{
    CalculateEicOptions, DataXY, DrawPeaksOptions, ErrorKind, FindTargetsOptions, FitPeakOptions,
    GetIonImageOptions, GetPeakOptions, Header, IonOutput, ParseIonOptions, PeakParameters,
    Position, ScanQuery, Target, WriteIonOptions, WriteMzmlOptions, calculate_eic, draw_peaks,
    find_targets, fit_peak, get_ion_image, get_peak, get_scans, parse_ion, parse_mzml, write_ion,
    write_mzml,
};

const API_FILE: &str = "tests/fixtures/api/api.ion";
const ALANINE_MZ: f64 = 89.04768;

fn open_api_file() -> crate::SampleFile {
    parse_ion(API_FILE, ParseIonOptions::default()).expect("open api.ion")
}

fn get_gaussian(center: f64, height: f64) -> PeakParameters {
    PeakParameters {
        height,
        center,
        fwhm: 0.05,
        tail: 0.0,
        r2: 0.0,
    }
}

#[test]
fn a_missing_path_is_a_file_not_found_error() {
    let error = parse_ion("tests/fixtures/missing.ion", ParseIonOptions::default())
        .err()
        .expect("an error");
    assert_eq!(error.kind, ErrorKind::FileNotFound);
    assert_eq!(error.function, "parse_ion");
}

#[test]
fn a_bad_mz_names_the_function_and_the_argument() {
    let mut file = open_api_file();
    let error =
        calculate_eic(&mut file, -1.0, CalculateEicOptions::default()).expect_err("an error");
    assert_eq!(error.kind, ErrorKind::InvalidArgument);
    assert_eq!(error.argument.as_deref(), Some("mz"));
    assert_eq!(
        error.to_string(),
        "calculate_eic: mz must be greater than 0, got -1"
    );
}

#[test]
fn the_window_is_read_from_the_eic_options() {
    let mut file = open_api_file();
    let whole_run =
        calculate_eic(&mut file, ALANINE_MZ, CalculateEicOptions::default()).expect("whole run");
    let options = CalculateEicOptions {
        from: 2.0,
        to: 3.0,
        ..Default::default()
    };
    let window = calculate_eic(&mut file, ALANINE_MZ, options).expect("window");
    assert!(window.x.len() < whole_run.x.len());
    assert!(window.x.iter().all(|rt| (2.0..=3.0).contains(rt)));
}

#[test]
fn seconds_give_the_same_window_as_minutes() {
    let mut file = open_api_file();
    let minutes = CalculateEicOptions {
        from: 2.0,
        to: 3.0,
        ..Default::default()
    };
    let seconds = CalculateEicOptions {
        from: 120.0,
        to: 180.0,
        time_unit: crate::TimeUnit::SECONDS,
        ..Default::default()
    };
    let in_minutes = calculate_eic(&mut file, ALANINE_MZ, minutes).expect("minutes");
    let in_seconds = calculate_eic(&mut file, ALANINE_MZ, seconds).expect("seconds");
    assert_eq!(in_minutes.x, in_seconds.x);
}

#[test]
fn a_target_without_range_finds_its_peak() {
    let mut file = open_api_file();
    let targets = [Target {
        id: None,
        mz: ALANINE_MZ,
        rt: 2.385,
        range: None,
    }];
    let peaks = find_targets(&mut file, &targets, FindTargetsOptions::default()).expect("peaks");
    assert_eq!(peaks.len(), 1);
    assert!(peaks[0].intensity > 0.0);
}

#[test]
fn ion_bytes_open_again() {
    let mut file = open_api_file();
    let bytes = write_ion(&mut file, WriteIonOptions::default())
        .expect("write")
        .expect("bytes");
    let mut again = parse_ion(bytes, ParseIonOptions::default()).expect("open bytes");
    let first =
        calculate_eic(&mut file, ALANINE_MZ, CalculateEicOptions::default()).expect("first");
    let second =
        calculate_eic(&mut again, ALANINE_MZ, CalculateEicOptions::default()).expect("second");
    assert_eq!(first.y, second.y);
}

#[test]
fn an_existing_output_needs_overwrite() {
    let folder = std::env::temp_dir().join(format!("quantion-write-{}", std::process::id()));
    fs::create_dir_all(&folder).expect("folder");
    let path = folder.join("sample.ion");
    let mut file = open_api_file();
    let options = WriteIonOptions {
        out: Some(IonOutput::Path(path.clone())),
        ..Default::default()
    };
    assert_eq!(
        write_ion(&mut file, options.clone()).expect("first write"),
        None
    );
    let error = write_ion(&mut file, options.clone()).expect_err("exists");
    assert_eq!(error.kind, ErrorKind::FileExists);
    let replace = WriteIonOptions {
        overwrite: true,
        ..options
    };
    assert!(write_ion(&mut file, replace).is_ok());
    fs::remove_dir_all(&folder).expect("clean up");
}

#[test]
fn headers_need_a_url() {
    let mut file = open_api_file();
    let options = WriteIonOptions {
        headers: vec![Header {
            name: "Authorization".to_string(),
            value: "Bearer token".to_string(),
        }],
        ..Default::default()
    };
    let error = write_ion(&mut file, options).expect_err("no url");
    assert_eq!(error.argument.as_deref(), Some("headers"));
}

#[test]
fn a_reserved_header_is_refused() {
    let mut file = open_api_file();
    let options = WriteIonOptions {
        out: Some(IonOutput::Url(
            "https://example.org/upload/sample.ion".to_string(),
        )),
        headers: vec![Header {
            name: "Content-Length".to_string(),
            value: "1".to_string(),
        }],
        ..Default::default()
    };
    let error = write_ion(&mut file, options).expect_err("reserved");
    assert_eq!(error.kind, ErrorKind::InvalidArgument);
    assert_eq!(error.argument.as_deref(), Some("headers[0].name"));
}

#[test]
fn mzml_bytes_open_again() {
    let mut file = open_api_file();
    let bytes = write_mzml(&mut file, WriteMzmlOptions::default())
        .expect("write")
        .expect("bytes");
    let mut again = parse_mzml(bytes).expect("open mzML bytes");
    let scans = get_scans(
        &mut again,
        ScanQuery {
            rt: Position::Closest(2.385),
            ..Default::default()
        },
        Default::default(),
    )
    .expect("scans");
    assert_eq!(scans.len(), 1);
}

#[test]
fn an_mzml_sample_cannot_be_imaged() {
    let mut file = open_api_file();
    let bytes = write_mzml(&mut file, WriteMzmlOptions::default())
        .expect("write")
        .expect("bytes");
    let mut mzml = parse_mzml(bytes).expect("open mzML bytes");
    let error =
        get_ion_image(&mut mzml, ALANINE_MZ, GetIonImageOptions::default()).expect_err("ion only");
    assert_eq!(error.argument.as_deref(), Some("file"));
}

#[test]
fn peaks_add_up_where_they_overlap() {
    let peaks = [get_gaussian(2.0, 100.0), get_gaussian(2.0, 50.0)];
    let options = DrawPeaksOptions {
        from: 1.5,
        to: 2.5,
        points: 101,
    };
    let drawn = draw_peaks(&peaks, options).expect("drawn");
    let apex = drawn.y.iter().copied().fold(f64::MIN, f64::max);
    assert!((apex - 150.0).abs() < 1e-6);
}

#[test]
fn the_drawing_range_fits_the_peaks_when_not_given() {
    let peaks = [get_gaussian(2.0, 100.0), get_gaussian(3.0, 100.0)];
    let drawn = draw_peaks(&peaks, DrawPeaksOptions::default()).expect("drawn");
    assert_eq!(drawn.x.len(), 1000);
    assert!(drawn.x[0] < 2.0 && *drawn.x.last().unwrap() > 3.0);
}

#[test]
fn a_drawn_peak_is_fitted_back() {
    let options = DrawPeaksOptions {
        from: 1.0,
        to: 3.0,
        points: 401,
    };
    let drawn = draw_peaks(&[get_gaussian(2.0, 1000.0)], options).expect("drawn");
    let fitted = fit_peak(
        &drawn,
        2.0,
        FitPeakOptions {
            shape: crate::PeakShape::GAUSSIAN,
            ..Default::default()
        },
    )
    .expect("fit")
    .expect("a curve");
    assert!((fitted.center - 2.0).abs() < 0.01);
    assert_eq!(fitted.tail, 0.0);
}

#[test]
fn without_range_the_whole_trace_is_searched() {
    let options = DrawPeaksOptions {
        from: 0.0,
        to: 10.0,
        points: 2001,
    };
    let trace: DataXY = draw_peaks(&[get_gaussian(8.0, 10000.0)], options).expect("drawn");
    let near = get_peak(
        &trace,
        2.0,
        GetPeakOptions {
            range: 0.5,
            ..Default::default()
        },
    )
    .expect("near");
    let anywhere = get_peak(&trace, 2.0, GetPeakOptions::default()).expect("anywhere");
    assert_eq!(near.intensity, 0.0);
    assert!((anywhere.rt - 8.0).abs() < 0.05);
}
