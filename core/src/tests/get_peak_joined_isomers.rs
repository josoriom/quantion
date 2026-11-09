use helpers::{Eic, load_chromatograms, param_f64};

use super::helpers;
use crate::{
    traces::FindPeaksOptions,
    utilities::{
        get_peak::get_peak,
        structs::{DataXY, Peak, Roi},
    },
};

const RT_TOLERANCE: f64 = 0.02;
const CASES: usize = 19;
const KIND: &str = "joined_isomers";

struct Target {
    name: String,
    target_rt: f64,
    window: f64,
    reference_rt: f64,
}

struct Case {
    name: String,
    data: DataXY,
    isomer: Target,
    neighbour: Target,
}

fn read_target(eic: &Eic, prefix: &str) -> Target {
    let analyte = format!("{prefix}analyte");
    Target {
        name: eic
            .params
            .get(&analyte)
            .unwrap_or_else(|| panic!("param {analyte} missing"))
            .clone(),
        target_rt: param_f64(eic, &format!("{prefix}opt.target")),
        window: param_f64(eic, &format!("{prefix}opt.roi_half_width")),
        reference_rt: param_f64(eic, &format!("{prefix}exp.rt")),
    }
}

fn read_cases() -> Vec<Case> {
    load_chromatograms("test.ion")
        .into_iter()
        .filter(|(_, eic)| eic.params.get("kind").map(String::as_str) == Some(KIND))
        .map(|(id, eic)| Case {
            name: id,
            isomer: read_target(&eic, ""),
            neighbour: read_target(&eic, "neighbour."),
            data: DataXY {
                x: eic.time,
                y: eic.intensity,
            },
        })
        .collect()
}

fn pipeline_options() -> FindPeaksOptions {
    FindPeaksOptions {
        auto_noise: true,
        auto_baseline: true,
        min_snr: 1.0,
        ..Default::default()
    }
}

fn pick(case: &Case, target: &Target) -> Peak {
    get_peak(
        &case.data,
        &Roi {
            rt: target.target_rt,
            range: target.window,
        },
        &pipeline_options(),
    )
}

fn report(failures: &[String], total: usize) {
    assert!(
        failures.is_empty(),
        "{} of {} cases fail:\n{}",
        failures.len(),
        total,
        failures.join("\n")
    );
}

#[test]
fn isomer_is_found_at_its_own_retention_time() {
    let cases = read_cases();
    assert_eq!(cases.len(), CASES);
    let mut failures = Vec::new();
    for case in &cases {
        let peak = pick(case, &case.isomer);
        if (peak.rt - case.isomer.reference_rt).abs() > RT_TOLERANCE {
            failures.push(format!(
                "{}: {} found at {:.3} from {:.3} to {:.3} but the reference is {:.2}",
                case.name, case.isomer.name, peak.rt, peak.from, peak.to, case.isomer.reference_rt
            ));
        }
    }
    report(&failures, cases.len());
}

#[test]
fn neighbour_isomer_keeps_its_own_retention_time() {
    let cases = read_cases();
    assert_eq!(cases.len(), CASES);
    let mut failures = Vec::new();
    for case in &cases {
        let peak = pick(case, &case.neighbour);
        if (peak.rt - case.neighbour.reference_rt).abs() > RT_TOLERANCE {
            failures.push(format!(
                "{}: {} found at {:.3} from {:.3} to {:.3} but the reference is {:.2}",
                case.name,
                case.neighbour.name,
                peak.rt,
                peak.from,
                peak.to,
                case.neighbour.reference_rt
            ));
        }
    }
    report(&failures, cases.len());
}

#[test]
fn two_isomers_do_not_share_one_peak() {
    let cases = read_cases();
    assert_eq!(cases.len(), CASES);
    let mut failures = Vec::new();
    for case in &cases {
        let isomer = pick(case, &case.isomer);
        let neighbour = pick(case, &case.neighbour);
        let (earlier, later) = if isomer.rt <= neighbour.rt {
            (&isomer, &neighbour)
        } else {
            (&neighbour, &isomer)
        };
        if (isomer.rt - neighbour.rt).abs() <= RT_TOLERANCE || earlier.to > later.from {
            failures.push(format!(
                "{}: {} from {:.3} to {:.3} and {} from {:.3} to {:.3} overlap",
                case.name,
                case.isomer.name,
                isomer.from,
                isomer.to,
                case.neighbour.name,
                neighbour.from,
                neighbour.to
            ));
        }
    }
    report(&failures, cases.len());
}
