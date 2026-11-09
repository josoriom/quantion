use helpers::load_chromatograms;

use super::helpers;
use crate::utilities::{
    fit_peak::{PeakSeed, PeakShape, draw_peak, fit_peak},
    functions::{emg_fn, gaussian_fn, sigma_from_fwhm},
    structs::DataXY,
};

fn linspace(from: f64, to: f64, points: usize) -> Vec<f64> {
    (0..points)
        .map(|i| from + (to - from) * i as f64 / (points - 1) as f64)
        .collect()
}

fn apex(data: &DataXY) -> PeakSeed {
    let mut rt = data.x[0];
    let mut intensity = data.y[0];
    for (x, y) in data.x.iter().zip(data.y.iter()) {
        if *y > intensity {
            intensity = *y;
            rt = *x;
        }
    }
    PeakSeed { rt, intensity }
}

#[test]
fn gaussian_round_trip() {
    let rt = 5.0;
    let intensity = 1000.0;
    let fwhm = 0.2;
    let sigma = sigma_from_fwhm(fwhm);
    let x = linspace(4.0, 6.0, 200);
    let y = x
        .iter()
        .map(|&v| gaussian_fn(v, intensity, rt, sigma))
        .collect();
    let data = DataXY { x, y };

    let params = fit_peak(&data, &PeakSeed { rt, intensity }, PeakShape::GAUSSIAN).expect("fit");
    assert_eq!(params.tail, 0.0);
    assert!(params.r2 > 0.99, "r2 = {:.4}", params.r2);
    assert!(
        (params.fwhm - fwhm).abs() < 0.02,
        "fwhm = {:.4}",
        params.fwhm
    );

    let drawn = draw_peak(&data, &params);
    assert_eq!(drawn.x, data.x);
    let peak = drawn.y.iter().cloned().fold(f64::MIN, f64::max);
    assert!(
        (peak - intensity).abs() < intensity * 0.02,
        "peak = {:.1}",
        peak
    );
}

#[test]
fn emg_round_trip() {
    let x = linspace(4.0, 7.0, 300);
    let y = x
        .iter()
        .map(|&v| emg_fn(v, 1000.0, 4.9, sigma_from_fwhm(0.15), 0.1))
        .collect();
    let data = DataXY { x, y };
    let seed = apex(&data);

    let params = fit_peak(&data, &seed, PeakShape::EMG).expect("fit");
    assert!(params.tail > 0.0);
    assert!(params.r2 > 0.98, "r2 = {:.4}", params.r2);

    let drawn = draw_peak(&data, &params);
    assert_eq!(drawn.x.len(), data.x.len());
    assert_eq!(drawn.x, data.x);
}

#[test]
fn real_peaks_fit_emg() {
    let chromatograms = load_chromatograms("chromatogram.ion");
    let peaks: Vec<(String, helpers::Eic)> = chromatograms
        .into_iter()
        .filter(|(id, _)| id.starts_with("emg_filter/"))
        .collect();
    assert!(!peaks.is_empty(), "no emg_filter chromatograms");

    for (id, eic) in peaks {
        let data = DataXY {
            x: eic.time.clone(),
            y: eic.intensity.clone(),
        };
        let seed = apex(&data);
        let params = fit_peak(&data, &seed, PeakShape::EMG).expect(&id);
        assert!(params.r2 >= 0.9, "{} r2 = {:.3}", id, params.r2);

        let drawn = draw_peak(&data, &params);
        assert_eq!(drawn.x, data.x);
    }
}

#[test]
fn emg_is_finite_far_from_the_center() {
    let sigma = sigma_from_fwhm(0.024);
    for rt in linspace(0.0, 5.0, 1000) {
        let value = emg_fn(rt, 164_000.0, 2.38, sigma, 0.0025);
        assert!(value.is_finite(), "emg at rt {rt} is {value}");
        assert!(value >= 0.0, "emg at rt {rt} is {value}");
    }
    assert!(emg_fn(0.0, 164_000.0, 2.38, sigma, 0.0025) < 1e-12);
    assert!(emg_fn(5.0, 164_000.0, 2.38, sigma, 0.0025) < 1e-12);
}

#[test]
fn emg_matches_the_direct_formula_near_the_center() {
    let sigma = 0.01;
    let tau = 0.02;
    for rt in linspace(2.3, 2.6, 61) {
        let a = sigma / tau;
        let direct = a
            * (std::f64::consts::PI / 2.0).sqrt()
            * (0.5 * a * a - (rt - 2.38) / tau).exp()
            * crate::utilities::math::erfc_approx(
                (a - (rt - 2.38) / sigma) / std::f64::consts::SQRT_2,
            );
        let stable = emg_fn(rt, 1.0, 2.38, sigma, tau);
        assert!(
            (direct - stable).abs() < 1e-12,
            "rt {rt}: direct {direct}, stable {stable}"
        );
    }
}

#[test]
fn erfcx_matches_its_definition() {
    for x in linspace(0.0, 20.0, 201) {
        let expected = (x * x).exp() * crate::utilities::math::erfc_approx(x);
        let actual = crate::utilities::math::erfcx_approx(x);
        assert!(
            (expected - actual).abs() <= 1e-12 * expected.max(1.0),
            "x {x}: {expected} vs {actual}"
        );
    }
    assert!(crate::utilities::math::erfcx_approx(1e6).is_finite());
}

#[test]
fn draw_peaks_has_no_nan_far_from_the_peak() {
    let peaks = [crate::utilities::fit_peak::PeakParameters {
        height: 164_309.66,
        center: 2.3794,
        fwhm: 0.02387,
        tail: 0.002507,
        r2: 0.98,
    }];
    let drawn = crate::utilities::fit_peak::draw_peaks(&peaks, 0.0, 5.0, 1000);
    for (x, y) in drawn.x.iter().zip(drawn.y.iter()) {
        assert!(y.is_finite(), "y at x {x} is {y}");
    }
}

#[test]
fn emg_fit_reports_the_apex_as_height_and_center() {
    let x = linspace(4.0, 7.0, 3001);
    let y: Vec<f64> = x
        .iter()
        .map(|&v| emg_fn(v, 1000.0, 4.9, sigma_from_fwhm(0.15), 0.1))
        .collect();
    let data = DataXY {
        x: x.clone(),
        y: y.clone(),
    };
    let params = fit_peak(&data, &apex(&data), PeakShape::EMG).expect("fit");
    let drawn = draw_peak(&data, &params);
    let mut top = 0;
    for index in 0..drawn.y.len() {
        if drawn.y[index] > drawn.y[top] {
            top = index;
        }
    }
    assert!(
        (drawn.y[top] - params.height).abs() < 1e-4 * params.height,
        "max {} height {}",
        drawn.y[top],
        params.height
    );
    assert!(
        (drawn.x[top] - params.center).abs() < 2e-3,
        "apex at {} center {}",
        drawn.x[top],
        params.center
    );
    let data_apex = apex(&data);
    assert!(
        (params.height - data_apex.intensity).abs() < 0.01 * data_apex.intensity,
        "height {} data apex {}",
        params.height,
        data_apex.intensity
    );
    assert!(
        (params.center - data_apex.rt).abs() < 2e-3,
        "center {} data apex {}",
        params.center,
        data_apex.rt
    );
}

#[test]
fn gaussian_is_unchanged() {
    let params = crate::utilities::fit_peak::PeakParameters {
        height: 500.0,
        center: 3.0,
        fwhm: 0.1,
        tail: 0.0,
        r2: 0.0,
    };
    let drawn = crate::utilities::fit_peak::draw_peaks(&[params], 2.5, 3.5, 1001);
    let middle = drawn.y[500];
    assert!((middle - 500.0).abs() < 1e-9, "middle {middle}");
}
