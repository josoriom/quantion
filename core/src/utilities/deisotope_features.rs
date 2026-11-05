use crate::{Feature, options::DeisotopeFeaturesOptions};

/// Mass of 13C minus 12C (Da), from https://en.wikipedia.org/wiki/Carbon-13
const ISOTOPE_DIFF: f64 = 1.003354835;

/// 13C abundance / 12C abundance = 1.07 / 98.93
const CARBON_13_RATIO: f64 = 0.0108157;

pub fn deisotope_features(mut features: Vec<Feature>, options: DeisotopeFeaturesOptions) -> Vec<Feature> {
    let DeisotopeFeaturesOptions {rt_tolerance, mz_tolerance} = options;
    features.sort_unstable_by(|a, b| a.mz.total_cmp(&b.mz));
    let mut is_isotope: Vec<bool> = vec![false; features.len()];
    for idx in 0..features.len() {
        if is_isotope[idx] {
            continue;
        }
        for charge in 1..=2 {
            let mut current = idx;
            loop {
                let target = features[current].mz + ISOTOPE_DIFF / charge as f64;
                let start = features.partition_point(|feature| feature.mz < target - mz_tolerance);
                let mut best: Option<usize> = None;
                for j in start..features.len() {
                    if features[j].mz > target + mz_tolerance {
                        break;
                    } else if is_isotope[j] {
                        continue;
                    } else if (features[j].rt - features[idx].rt).abs() > rt_tolerance {
                        continue;
                    }
                    let height_ok = if current == idx {
                        let max_carbons = features[idx].mz * charge as f64 / 12.0;
                        features[j].intensity / features[idx].intensity <= max_carbons * CARBON_13_RATIO
                    } else {
                        features[j].intensity < features[current].intensity
                    };
                    if !height_ok {
                        continue;
                    }
                    best = match best {
                        Some(b) if (features[b].mz - target).abs() <= (features[j].mz - target).abs() => Some(b),
                        _ => Some(j),
                    };
                }
                match best {
                    Some(b) => {
                        is_isotope[b] = true;
                        current = b;
                    }
                    None => break,
                }
            }
            if current != idx {
                break;
            }
        }
    }
    features
        .into_iter()
        .zip(is_isotope)
        .filter(|(_, isotope)| !isotope)
        .map(|(feature, _)| feature)
        .collect()
}