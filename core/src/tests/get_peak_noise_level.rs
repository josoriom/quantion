use helpers::{Eic, load_chromatograms, param_f64};

use super::helpers;
use crate::{
    traces::FindPeaksOptions,
    utilities::{
        get_peak::get_peak,
        structs::{DataXY, Roi},
    },
};

const RT_TOLERANCE: f64 = 0.02;
const KIND: &str = "noise_level";

fn pipeline_options() -> FindPeaksOptions {
    FindPeaksOptions {
        auto_noise: true,
        auto_baseline: true,
        min_snr: 1.0,
        ..Default::default()
    }
}

#[test]
fn small_peak_next_to_large_peak_is_found() {
    let chromatograms = load_chromatograms("test.ion");
    let cases: Vec<(&String, &Eic)> = chromatograms
        .iter()
        .filter(|(_, eic)| eic.params.get("kind").map(String::as_str) == Some(KIND))
        .collect();
    assert_eq!(cases.len(), 10);
    for (id, eic) in cases {
        let data = DataXY {
            x: eic.time.clone(),
            y: eic.intensity.clone(),
        };
        let reference_rt = param_f64(eic, "exp.rt");
        let peak = get_peak(
            &data,
            &Roi {
                rt: param_f64(eic, "opt.target"),
                range: param_f64(eic, "opt.roi_half_width"),
            },
            &pipeline_options(),
        );
        assert!(
            (peak.rt - reference_rt).abs() <= RT_TOLERANCE,
            "{}: found a peak at {} but the reference is {}",
            id,
            peak.rt,
            reference_rt
        );
    }
}
