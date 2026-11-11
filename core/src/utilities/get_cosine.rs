use std::cmp::Ordering;

use crate::utilities::{
    calculate_eic::{lower_bound, upper_bound},
    clean_spectrum::{PEAK_TOLERANCE, Spectrum},
};

struct Pair {
    product: f64,
    query: usize,
    reference: usize,
}

pub(crate) fn get_cosine(query: &Spectrum, reference: &Spectrum) -> f64 {
    if query.mz.is_empty() || reference.mz.is_empty() {
        return 0.0;
    }
    let query_heights: Vec<f64> = query
        .intensity
        .iter()
        .map(|intensity| intensity.sqrt())
        .collect();
    let reference_heights: Vec<f64> = reference
        .intensity
        .iter()
        .map(|intensity| intensity.sqrt())
        .collect();
    let mut pairs = Vec::new();
    for (query_position, mz) in query.mz.iter().enumerate() {
        let first = lower_bound(&reference.mz, mz - PEAK_TOLERANCE);
        let last = upper_bound(&reference.mz, mz + PEAK_TOLERANCE);
        for reference_position in first..last {
            pairs.push(Pair {
                product: query_heights[query_position] * reference_heights[reference_position],
                query: query_position,
                reference: reference_position,
            });
        }
    }
    pairs.sort_by(|left, right| get_pair_order(right, left));
    let mut used_query = vec![false; query.mz.len()];
    let mut used_reference = vec![false; reference.mz.len()];
    let mut total = 0.0;
    for pair in pairs {
        if !used_query[pair.query] && !used_reference[pair.reference] {
            used_query[pair.query] = true;
            used_reference[pair.reference] = true;
            total += pair.product;
        }
    }
    let lengths = get_length(&query_heights) * get_length(&reference_heights);
    if lengths > 0.0 { total / lengths } else { 0.0 }
}

fn get_pair_order(left: &Pair, right: &Pair) -> Ordering {
    left.product
        .total_cmp(&right.product)
        .then(left.query.cmp(&right.query))
        .then(left.reference.cmp(&right.reference))
}

fn get_length(values: &[f64]) -> f64 {
    values.iter().map(|value| value * value).sum::<f64>().sqrt()
}
