// The peak at the target RT must beat a neighbour 0.0865 min away that is 6.33% taller (max_drift 0.5).

use crate::{
    traces::FindPeaksOptions,
    utilities::{
        get_peak::get_peak,
        structs::{DataXY, Roi},
    },
};

const SCAN_STEP: f64 = 0.0043;
const POINTS: usize = 400;
const START_RT: f64 = 2.0;
const PEAK_WIDTH: f64 = 0.012;

const NEAR_RT: f64 = 2.4658;
const NEAR_APEX: f64 = 183302.0;
const FAR_RT: f64 = 2.5523;
const FAR_APEX: f64 = 194902.0;

const DEFAULT_MAX_DRIFT: f64 = 0.5;
const RT_TOLERANCE: f64 = 0.01;

fn options() -> FindPeaksOptions {
    FindPeaksOptions {
        min_intensity: 1000.0,
        min_peak_width_points: 10,
        ..Default::default()
    }
}

fn two_peaks() -> DataXY {
    let x: Vec<f64> = (0..POINTS)
        .map(|i| START_RT + SCAN_STEP * i as f64)
        .collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&t| gaussian(t, NEAR_RT, NEAR_APEX) + gaussian(t, FAR_RT, FAR_APEX))
        .collect();
    DataXY { x, y }
}

fn gaussian(t: f64, center: f64, apex: f64) -> f64 {
    let distance = (t - center) / PEAK_WIDTH;
    apex * (-0.5 * distance * distance).exp()
}

fn peak_at(target: f64) -> f64 {
    let data = two_peaks();
    let peak = get_peak(
        &data,
        &Roi {
            rt: target,
            range: DEFAULT_MAX_DRIFT,
        },
        &options(),
    );
    peak.rt
}

#[test]
fn targeted_peak_wins_over_taller_neighbour_7() {
    let found = peak_at(NEAR_RT);
    assert!(
        (found - NEAR_RT).abs() < RT_TOLERANCE,
        "targeting {NEAR_RT} returned {found}: the taller neighbour at {FAR_RT} won"
    );
}

#[test]
fn taller_neighbour_still_wins_when_it_is_the_target_7() {
    let found = peak_at(FAR_RT);
    assert!(
        (found - FAR_RT).abs() < RT_TOLERANCE,
        "targeting {FAR_RT} returned {found}"
    );
}

#[test]
fn nearest_peak_wins_even_when_much_taller_neighbour_is_close_7() {
    let midpoint = (NEAR_RT + FAR_RT) / 2.0;
    let found = peak_at(NEAR_RT - (midpoint - NEAR_RT));
    assert!(
        (found - NEAR_RT).abs() < RT_TOLERANCE,
        "targeting below {NEAR_RT} returned {found}: expected the nearer peak"
    );
}
