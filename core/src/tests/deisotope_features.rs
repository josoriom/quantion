use crate::{DeisotopeFeaturesOptions, ErrorKind, Feature, deisotope_features};

const ISOTOPE_DIFF: f64 = 1.003354835;

fn feature(mz: f64, rt: f64, intensity: f64) -> Feature {
    Feature { mz, rt, intensity, ..Default::default() }
}

fn get_mzs(features: &[Feature]) -> Vec<f64> {
    features.iter().map(|item| item.mz).collect()
}

#[test]
fn removes_the_isotope_and_keeps_the_monoisotopic_peak() {
    let features = vec![feature(200.0 + ISOTOPE_DIFF, 5.0, 100.0), feature(200.0, 5.0, 1000.0), feature(300.0, 5.0, 500.0)];
    let found = deisotope_features(features, DeisotopeFeaturesOptions::default()).unwrap();
    assert_eq!(get_mzs(&found), vec![200.0, 300.0]);
}

#[test]
fn follows_a_chain_of_isotopes() {
    let features = vec![feature(200.0, 5.0, 1000.0), feature(200.0 + ISOTOPE_DIFF, 5.0, 100.0), feature(200.0 + 2.0 * ISOTOPE_DIFF, 5.0, 10.0)];
    let found = deisotope_features(features, DeisotopeFeaturesOptions::default()).unwrap();
    assert_eq!(get_mzs(&found), vec![200.0]);
}

#[test]
fn removes_a_doubly_charged_isotope() {
    let features = vec![feature(200.0, 5.0, 1000.0), feature(200.0 + ISOTOPE_DIFF / 2.0, 5.0, 100.0)];
    let found = deisotope_features(features, DeisotopeFeaturesOptions::default()).unwrap();
    assert_eq!(get_mzs(&found), vec![200.0]);
}

#[test]
fn keeps_a_peak_at_another_rt_or_too_high_to_be_an_isotope() {
    let other_rt = vec![feature(200.0, 5.0, 1000.0), feature(200.0 + ISOTOPE_DIFF, 6.0, 100.0)];
    assert_eq!(deisotope_features(other_rt, DeisotopeFeaturesOptions::default()).unwrap().len(), 2);
    let too_high = vec![feature(200.0, 5.0, 100.0), feature(200.0 + ISOTOPE_DIFF, 5.0, 1000.0)];
    assert_eq!(deisotope_features(too_high, DeisotopeFeaturesOptions::default()).unwrap().len(), 2);
}

#[test]
fn keeps_frequency() {
    let mut kept = feature(200.0, 5.0, 1000.0);
    kept.frequency = Some(0.5);
    let found = deisotope_features(vec![kept], DeisotopeFeaturesOptions::default()).unwrap();
    assert_eq!(found[0].frequency, Some(0.5));
}

#[test]
fn accepts_an_empty_list() {
    assert!(deisotope_features(Vec::new(), DeisotopeFeaturesOptions::default()).unwrap().is_empty());
}

#[test]
fn rejects_invalid_values() {
    let negative = DeisotopeFeaturesOptions { rt_tolerance: -1.0, ..Default::default() };
    let error = deisotope_features(Vec::new(), negative).unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidArgument);
    assert_eq!(error.argument.as_deref(), Some("rt_tolerance"));
    let error = deisotope_features(vec![feature(f64::NAN, 5.0, 1.0)], DeisotopeFeaturesOptions::default()).unwrap_err();
    assert_eq!(error.argument.as_deref(), Some("features[0].mz"));
}
