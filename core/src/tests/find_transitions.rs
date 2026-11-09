use ionic::{IonReader, ReadOptions, mzml::structs::MzML};

use crate::{FindTransitionsOptions, Transition, utilities::find_transitions::find_transitions};

fn read_transitions() -> MzML {
    let bytes = std::fs::read("tests/fixtures/srm.ion").expect("read srm.ion");
    IonReader::from_bytes(&bytes, &ReadOptions::default())
        .expect("open srm.ion")
        .to_mzml()
        .expect("decode srm.ion")
}

fn get_target(id: &str, precursor: f64, product: f64) -> Transition {
    Transition {
        id: Some(id.to_string()),
        precursor,
        product,
        rt: 2.385,
        range: 0.1,
    }
}

#[test]
fn a_target_finds_the_chromatogram_with_its_precursor_and_product() {
    let targets = [get_target("alanine", 89.04768, 89.04768)];
    let rows = find_transitions(
        &read_transitions(),
        &targets,
        FindTransitionsOptions::default(),
    )
    .expect("chromatograms");
    assert!(rows[0].intensity > 0.0);
    assert!((rows[0].rt - 2.385).abs() < 0.05);
}

#[test]
fn a_target_matches_within_the_tolerance() {
    let targets = [get_target("alanine", 89.0526, 89.0436)];
    let rows = find_transitions(
        &read_transitions(),
        &targets,
        FindTransitionsOptions::default(),
    )
    .expect("chromatograms");
    assert!(rows[0].intensity > 0.0);
}

#[test]
fn a_wrong_product_finds_nothing() {
    let targets = [get_target("alanine", 89.04768, 90.0555)];
    let rows = find_transitions(
        &read_transitions(),
        &targets,
        FindTransitionsOptions::default(),
    )
    .expect("chromatograms");
    assert_eq!(rows[0].intensity, 0.0);
}
