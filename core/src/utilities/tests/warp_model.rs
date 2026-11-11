use ionic::{IonReader, ReadOptions};

use crate::utilities::{
    calculate_eic::{EicReader, ScanTime},
    load_model::{ModelError, load_model},
    structs::{FromTo, Target},
    warp_model::{WarpError, WarpModel, load_warp, resample_eic},
};

const MODEL_PATH: &str = "tests/fixtures/shift_model.onnx";
const COVERING_FILE: &str = "tests/fixtures/api/api.ion";
const EMPTY_FILE: &str = "tests/fixtures/test.ion";
const POINTS: usize = 64;
const EXPECTED_RTS: [f64; 4] = [1.2629844, 2.4629846, 3.1635001, 4.062953];
const EXPECTED_UNCERTAINTIES: [f64; 4] = [0.05199894, 0.05199894, 0.05262259, 0.05249792];
const TOLERANCE: f64 = 1e-5;
const RESAMPLE_RANGE: FromTo = FromTo { from: 1.0, to: 2.0 };

fn get_warp() -> WarpModel {
    load_warp(&load_model(MODEL_PATH).expect("model")).expect("warp model")
}

fn get_target(id: &str, mz: f64, rt: f64) -> Target {
    Target {
        id: Some(id.to_string()),
        mz,
        rt,
        range: None,
    }
}

fn get_targets() -> Vec<Target> {
    vec![
        get_target("alanine", 250.1, 1.2),
        get_target("sarcosine", 250.1, 2.4),
        get_target("leucine", 310.2, 3.1),
        get_target("valine", 400.4, 4.0),
    ]
}

fn resample(scan_rts: &[f64], input: &[f64]) -> Vec<f32> {
    let mut output = vec![0.0_f32; 5];
    resample_eic(scan_rts, input, RESAMPLE_RANGE, &mut output);
    output
}

fn get_traces(rows: usize, row_step: usize, point_step: usize, cycle: usize) -> Vec<f64> {
    (0..rows)
        .flat_map(|row| {
            (0..POINTS)
                .map(move |point| ((row * row_step + point * point_step) % cycle) as f64 * 10.0)
        })
        .collect()
}

fn open_file(path: &str) -> IonReader {
    IonReader::open(path, &ReadOptions::default()).expect("ion file")
}

#[test]
fn resample_keeps_the_values_at_the_scan_times() {
    let output = resample(&[1.0, 1.25, 1.5, 1.75, 2.0], &[0.0, 10.0, 20.0, 30.0, 40.0]);
    assert_eq!(output, vec![0.0, 10.0, 20.0, 30.0, 40.0]);
}

#[test]
fn resample_takes_the_value_between_two_scans() {
    let output = resample(&[1.0, 1.5, 2.0], &[0.0, 100.0, 0.0]);
    assert_eq!(output, vec![0.0, 50.0, 100.0, 50.0, 0.0]);
}

#[test]
fn resample_holds_the_edge_values_outside_the_scans() {
    let output = resample(&[1.4, 1.6], &[7.0, 9.0]);
    assert_eq!(output, vec![7.0, 7.0, 8.0, 9.0, 9.0]);
}

#[test]
fn a_warp_model_reads_its_settings() {
    let warp = get_warp();

    assert_eq!(warp.points(), POINTS);
    assert_eq!(warp.time_range().from, 0.5);
    assert_eq!(warp.time_range().to, 5.0);
    assert_eq!(warp.context_mz(), &[300.1, 310.2, 320.3]);
}

#[test]
fn keys_and_values_come_from_the_model() {
    let warp = get_warp();

    assert_eq!(warp.get_value("points"), Some("64"));
    assert_eq!(warp.get_value("missing"), None);
    assert_eq!(warp.get_keys().len(), warp.get_values().len());
    assert!(warp.get_keys().iter().any(|key| key == "targets"));
}

#[test]
fn predictions_match_onnx() {
    let warp = get_warp();
    let targets = get_targets();
    let grid: Vec<f64> = (0..POINTS)
        .map(|point| 0.5 + 4.5 * point as f64 / (POINTS - 1) as f64)
        .collect();
    let mut traces = get_traces(4, 7, 3, 11);
    traces.extend(get_traces(3, 5, 1, 13));

    let predicted = warp
        .predict_from_traces(&targets, &grid, &traces)
        .expect("predictions");

    for (prediction, (rt, uncertainty)) in predicted
        .iter()
        .zip(EXPECTED_RTS.iter().zip(EXPECTED_UNCERTAINTIES))
    {
        assert!(
            (prediction.rt - rt).abs() < TOLERANCE,
            "rt {} should be {}",
            prediction.rt,
            rt
        );
        assert!(
            (prediction.uncertainty - uncertainty).abs() < TOLERANCE,
            "uncertainty {} should be {}",
            prediction.uncertainty,
            uncertainty
        );
    }
}

#[test]
fn predict_rts_gives_one_prediction_per_target_from_a_file() {
    let warp = get_warp();
    let targets = get_targets();
    let mut ion = open_file(COVERING_FILE);

    let predicted = warp
        .predict_rts_from_reader(&mut EicReader::Ion(&mut ion), &targets)
        .expect("predictions");

    assert_eq!(predicted.len(), targets.len());
    assert!(
        predicted
            .iter()
            .all(|prediction| prediction.rt.is_finite() && prediction.uncertainty.is_finite())
    );
}

#[test]
fn a_target_without_mz_is_refused() {
    let warp = get_warp();
    let mut ion = open_file(COVERING_FILE);
    let targets = vec![
        get_targets()[0].clone(),
        get_target("broken", f64::NAN, 2.0),
    ];

    let result = warp.predict_rts_from_reader(&mut EicReader::Ion(&mut ion), &targets);

    assert!(matches!(result, Err(WarpError::TargetMissing(1))));
}

#[test]
fn a_file_without_scans_in_the_model_range_is_refused() {
    let warp = get_warp();
    let targets = get_targets();
    let mut ion = open_file(EMPTY_FILE);

    let result = warp.predict_rts_from_reader(&mut EicReader::Ion(&mut ion), &targets);

    assert!(matches!(
        result,
        Err(WarpError::ScansNotCovered { found: None, .. })
    ));
}

#[test]
fn a_file_whose_scans_cover_part_of_the_model_range_is_refused() {
    let warp = get_warp();
    let scans: Vec<ScanTime> = [1.0, 2.0, 3.0, 4.0]
        .iter()
        .enumerate()
        .map(|(index, rt)| ScanTime { index, rt: *rt })
        .collect();

    let error = warp.check_scans(&scans).expect_err("scans from 1 to 4 min");

    assert_eq!(
        error.to_string(),
        "the file's scans cover 1 to 4 min, the warp model needs 0.5 to 5 min"
    );
}

#[test]
fn calibrate_with_one_sample_moves_each_rt_by_its_predicted_shift() {
    let warp = get_warp();
    let targets = get_targets();
    let mut ion = open_file(COVERING_FILE);
    let predicted = warp
        .predict_rts_from_reader(&mut EicReader::Ion(&mut ion), &targets)
        .expect("predictions");

    let calibrated = warp
        .calibrate_from_readers(
            &mut [EicReader::Ion(&mut ion)],
            std::slice::from_ref(&targets),
        )
        .expect("calibration");

    for ((target, prediction), reference) in targets.iter().zip(predicted).zip(calibrated) {
        assert_eq!(reference.id, target.id);
        assert_eq!(reference.mz, target.mz);
        assert_eq!(reference.rt, 2.0 * target.rt - prediction.rt);
    }
}

#[test]
fn calibrate_takes_the_median_over_the_samples() {
    let warp = get_warp();
    let targets = get_targets();
    let mut early = targets.clone();
    early[0].rt -= 0.2;
    let mut late = targets.clone();
    late[0].rt += 0.2;
    let mut first = open_file(COVERING_FILE);
    let mut second = open_file(COVERING_FILE);
    let mut third = open_file(COVERING_FILE);
    let middle = warp
        .calibrate_from_readers(
            &mut [EicReader::Ion(&mut first)],
            std::slice::from_ref(&targets),
        )
        .expect("one sample");

    let calibrated = warp
        .calibrate_from_readers(
            &mut [
                EicReader::Ion(&mut first),
                EicReader::Ion(&mut second),
                EicReader::Ion(&mut third),
            ],
            &[early, targets.clone(), late],
        )
        .expect("three samples");

    assert_eq!(calibrated, middle);
}

#[test]
fn calibrate_uses_the_samples_where_a_target_was_seen() {
    let warp = get_warp();
    let targets = get_targets();
    let mut first = open_file(COVERING_FILE);
    let mut second = open_file(COVERING_FILE);

    let calibrated = warp
        .calibrate_from_readers(
            &mut [EicReader::Ion(&mut first), EicReader::Ion(&mut second)],
            &[targets[..1].to_vec(), targets.clone()],
        )
        .expect("calibration");

    let ids: Vec<&str> = calibrated.iter().map(|target| target.get_id()).collect();
    assert_eq!(ids, ["alanine", "sarcosine", "leucine", "valine"]);
}

#[test]
fn calibrate_needs_one_target_list_per_sample() {
    let warp = get_warp();
    let mut ion = open_file(COVERING_FILE);

    let error = warp
        .calibrate_from_readers(
            &mut [EicReader::Ion(&mut ion)],
            &[get_targets(), get_targets()],
        )
        .expect_err("two lists for one sample");

    assert_eq!(
        error.to_string(),
        "calibrate needs one target list per sample, got 1 samples and 2 target lists"
    );
}

#[test]
fn calibrate_needs_an_id_for_each_target() {
    let warp = get_warp();
    let mut targets = get_targets();
    targets[1].id = None;
    let mut ion = open_file(COVERING_FILE);

    let error = warp
        .calibrate_from_readers(&mut [EicReader::Ion(&mut ion)], &[targets])
        .expect_err("a target without id");

    assert_eq!(error.to_string(), "target 2 in list 1 has no id");
}

#[test]
fn no_targets_give_no_predictions_without_reading_the_file() {
    let warp = get_warp();
    let mut ion = open_file(EMPTY_FILE);

    let predicted = warp
        .predict_rts_from_reader(&mut EicReader::Ion(&mut ion), &[])
        .expect("predictions");

    assert!(predicted.is_empty());
}

#[test]
fn a_missing_model_file_is_refused() {
    let result = load_model("tests/fixtures/missing.onnx");

    assert!(matches!(result, Err(ModelError::NotRead(_))));
}

#[test]
fn the_warp_keeps_working_after_the_model_is_dropped() {
    let model = load_model(MODEL_PATH).expect("model");
    let warp = load_warp(&model).expect("warp model");
    drop(model);
    let mut ion = open_file(COVERING_FILE);

    let predicted = warp
        .predict_rts_from_reader(&mut EicReader::Ion(&mut ion), &get_targets())
        .expect("predictions");

    assert_eq!(predicted.len(), get_targets().len());
    assert_eq!(warp.get_value("points"), Some("64"));
}

#[test]
fn the_model_keeps_working_after_the_warp_is_made() {
    let model = load_model(MODEL_PATH).expect("model");
    let first = load_warp(&model).expect("first warp model");
    let second = load_warp(&model).expect("second warp model");
    drop(first);

    assert_eq!(model.get_value("points"), Some("64"));
    assert_eq!(model.get_keys(), second.get_keys());
}

#[cfg(not(feature = "http"))]
#[test]
fn a_url_needs_the_http_feature() {
    let result = load_model("http://127.0.0.1:9/shift_model.onnx");

    assert!(matches!(result, Err(ModelError::NotRead(reason)) if reason.contains("http feature")));
}

#[cfg(feature = "http")]
mod from_url {
    use std::{
        io::{BufRead, BufReader, Write},
        net::TcpListener,
        thread,
    };

    use super::*;

    fn serve(bytes: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("address");
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().expect("clone"));
                let mut line = String::new();
                while reader.read_line(&mut line).unwrap_or(0) > 0 && line != "\r\n" {
                    line.clear();
                }
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bytes.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&bytes);
            }
        });
        format!("http://{address}/shift_model.onnx")
    }

    #[test]
    fn a_model_from_a_url_predicts_like_one_from_a_path() {
        let url = serve(std::fs::read(MODEL_PATH).expect("model file"));
        let from_url = load_warp(&load_model(url.as_str()).expect("model from url"))
            .expect("warp model from url");
        let from_path = get_warp();
        let targets = get_targets();
        let mut ion = open_file(COVERING_FILE);
        let mut reader = EicReader::Ion(&mut ion);

        let url_predictions = from_url
            .predict_rts_from_reader(&mut reader, &targets)
            .expect("url predictions");
        let path_predictions = from_path
            .predict_rts_from_reader(&mut reader, &targets)
            .expect("path predictions");

        assert_eq!(url_predictions, path_predictions);
    }
}
