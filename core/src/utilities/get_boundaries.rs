use std::ops::RangeInclusive;

use crate::utilities::{
    cheminfo::sgg::{SggOptions, sgg},
    closest_index,
    structs::DataXY,
};

#[derive(Clone, Copy, Debug)]
pub struct Boundary {
    pub index: Option<usize>,
    pub value: Option<f64>,
}

#[derive(Clone, Copy, Debug)]
pub struct Boundaries {
    pub from: Boundary,
    pub to: Boundary,
}

#[derive(Clone, Copy, Debug)]
pub struct BoundariesOptions {
    pub min_slope_step: f64,
    pub smooth_window: usize,
    pub smooth_polynomial: usize,
}

impl Default for BoundariesOptions {
    fn default() -> Self {
        Self {
            min_slope_step: 1e-5,
            smooth_window: 7,
            smooth_polynomial: 3,
        }
    }
}

pub(crate) struct SmoothedSignal<'a> {
    pub(crate) x: &'a [f64],
    pub(crate) values: Vec<f64>,
    pub(crate) point_weight: f64,
    pub(crate) half_window: usize,
}

impl<'a> SmoothedSignal<'a> {
    pub(crate) fn new(data: &'a DataXY, options: &BoundariesOptions) -> Self {
        let Some(window) = usable_window(options.smooth_window, data.y.len()) else {
            return Self {
                x: &data.x,
                values: data.y.to_vec(),
                point_weight: 1.0,
                half_window: 0,
            };
        };
        let smoothing = SggOptions {
            window_size: window,
            derivative: 0,
            polynomial: options.smooth_polynomial,
        };
        Self {
            x: &data.x,
            values: sgg(&data.y, &data.x, smoothing),
            point_weight: get_point_weight(smoothing),
            half_window: window / 2,
        }
    }

    pub(crate) fn get_noise_band(&self, noise: f64) -> f64 {
        noise * self.point_weight.sqrt()
    }

    pub(crate) fn find_top(&self, start: usize) -> usize {
        let values = &self.values;
        let mut top = start;
        loop {
            let left = top.checked_sub(1);
            let right = (top + 1 < values.len()).then_some(top + 1);
            let higher = [left, right]
                .into_iter()
                .flatten()
                .filter(|&index| values[index] > values[top])
                .max_by(|&left, &right| values[left].total_cmp(&values[right]));
            match higher {
                Some(index) => top = index,
                None => return top,
            }
        }
    }
}

fn get_point_weight(smoothing: SggOptions) -> f64 {
    let window = smoothing.window_size;
    let mut single_point = vec![0.0; 2 * window + 1];
    single_point[window] = 1.0;
    let equal_step = [1.0];
    sgg(&single_point, &equal_step, smoothing)[window]
}

pub fn get_boundaries(data: &DataXY, peak_x: f64, options: BoundariesOptions) -> Boundaries {
    let n = data.x.len();
    if n < 2 || n != data.y.len() {
        return Boundaries {
            from: Boundary {
                index: None,
                value: None,
            },
            to: Boundary {
                index: None,
                value: None,
            },
        };
    }

    let smoothed = SmoothedSignal::new(data, &options);
    let apex_index = smoothed.find_top(closest_index(&data.x, peak_x));

    find_boundaries(&smoothed, apex_index, 0..=n - 1, 0.0, &options)
}

pub(crate) fn find_boundaries(
    smoothed: &SmoothedSignal,
    apex_index: usize,
    search_range: RangeInclusive<usize>,
    noise: f64,
    options: &BoundariesOptions,
) -> Boundaries {
    let values = &smoothed.values;
    let lowest_value = min_value(&values[search_range.clone()]);
    let floor = lowest_value.max(smoothed.get_noise_band(noise)) + options.min_slope_step;
    let left_path = (*search_range.start()..apex_index).rev();
    let right_path = apex_index + 1..=*search_range.end();

    Boundaries {
        from: boundary_at(smoothed.x, find_edge(values, apex_index, left_path, floor)),
        to: boundary_at(smoothed.x, find_edge(values, apex_index, right_path, floor)),
    }
}

fn min_value(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::INFINITY, f64::min)
}

fn usable_window(requested: usize, length: usize) -> Option<usize> {
    if length < 5 {
        return None;
    }
    let mut window = requested.min(length);
    if window.is_multiple_of(2) {
        window -= 1;
    }
    (window >= 5).then_some(window)
}

fn find_edge(
    values: &[f64],
    apex_index: usize,
    mut path: impl Iterator<Item = usize>,
    floor: f64,
) -> usize {
    let mut lowest_index = apex_index;
    let mut lowest_value = f64::INFINITY;
    while let Some(index) = path.next() {
        if values[index] <= floor {
            return follow_falling_signal(values, index, path);
        }
        if values[index] < lowest_value {
            lowest_index = index;
            lowest_value = values[index];
        }
    }
    lowest_index
}

fn follow_falling_signal(values: &[f64], start: usize, path: impl Iterator<Item = usize>) -> usize {
    let mut current = start;
    for next in path {
        if values[next] >= values[current] {
            break;
        }
        current = next;
    }
    current
}

fn boundary_at(x: &[f64], index: usize) -> Boundary {
    Boundary {
        index: Some(index),
        value: Some(x[index]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(from: f64, to: f64, points: usize) -> Vec<f64> {
        (0..points)
            .map(|index| from + (to - from) * index as f64 / (points - 1) as f64)
            .collect()
    }

    fn bell(x: f64, center: f64, height: f64, fwhm: f64) -> f64 {
        let sigma = fwhm / 2.354_820_045;
        height * (-0.5 * ((x - center) / sigma).powi(2)).exp()
    }

    fn wobble(index: usize) -> f64 {
        let value = (index as f64 * 12.9898).sin() * 43758.547;
        2.0 * (value - value.floor()) - 1.0
    }

    #[test]
    fn brackets_the_apex() {
        let x = grid(4.0, 6.0, 400);
        let y: Vec<f64> = x.iter().map(|&v| 0.05 + bell(v, 5.0, 1.0, 0.2)).collect();
        let data = DataXY { x, y };

        let edges = get_boundaries(&data, 5.0, BoundariesOptions::default());
        let from = edges.from.value.unwrap();
        let to = edges.to.value.unwrap();

        assert!(
            from < 5.0 && to > 5.0,
            "bracket [{from}, {to}] must contain the apex"
        );
        assert!(
            from < 4.9 && to > 5.1,
            "bracket [{from}, {to}] must reach past the flanks"
        );
    }

    #[test]
    fn stops_at_the_valley_between_two_peaks() {
        let x = grid(4.0, 6.5, 500);
        let y: Vec<f64> = x
            .iter()
            .map(|&v| 0.05 + bell(v, 5.0, 1.0, 0.2) + bell(v, 5.35, 0.8, 0.2))
            .collect();
        let cut = x.partition_point(|&v| v <= 5.35);
        let data = DataXY {
            x: x[..cut].to_vec(),
            y: y[..cut].to_vec(),
        };

        let to = get_boundaries(&data, 5.0, BoundariesOptions::default())
            .to
            .value
            .unwrap();

        assert!(
            to > 5.0 && to < 5.35,
            "right edge {to} must land in the valley, not the next peak"
        );
    }

    #[test]
    fn window_is_stable_under_noise() {
        let x = grid(4.0, 6.5, 500);
        let clean: Vec<f64> = x
            .iter()
            .map(|&v| 0.05 + bell(v, 5.0, 1.0, 0.2) + bell(v, 5.35, 0.8, 0.2))
            .collect();
        let noisy: Vec<f64> = clean
            .iter()
            .enumerate()
            .map(|(index, &value)| value + 0.02 * wobble(index))
            .collect();

        let cut = x.partition_point(|&v| v <= 5.35);
        let x = x[..cut].to_vec();
        let clean_to = get_boundaries(
            &DataXY {
                x: x.clone(),
                y: clean[..cut].to_vec(),
            },
            5.0,
            BoundariesOptions::default(),
        )
        .to
        .value
        .unwrap();
        let noisy_to = get_boundaries(
            &DataXY {
                x,
                y: noisy[..cut].to_vec(),
            },
            5.0,
            BoundariesOptions::default(),
        )
        .to
        .value
        .unwrap();

        assert!(
            (clean_to - noisy_to).abs() <= 0.05,
            "right edge moved {clean_to} -> {noisy_to} under noise"
        );
    }

    #[test]
    fn keeps_left_edge_when_right_side_is_truncated() {
        let full_x = grid(4.0, 6.0, 400);
        let full_y: Vec<f64> = full_x
            .iter()
            .map(|&v| 0.05 + bell(v, 5.0, 1.0, 0.2))
            .collect();
        let full_from = get_boundaries(
            &DataXY {
                x: full_x.clone(),
                y: full_y.clone(),
            },
            5.0,
            BoundariesOptions::default(),
        )
        .from
        .value
        .unwrap();

        let cut = full_x.partition_point(|&v| v <= 5.25);
        let cut_x = full_x[..cut].to_vec();
        let cut_y = full_y[..cut].to_vec();
        let cut_edges = get_boundaries(
            &DataXY { x: cut_x, y: cut_y },
            5.0,
            BoundariesOptions::default(),
        );
        let cut_from = cut_edges.from.value.unwrap();
        let cut_to = cut_edges.to.value.unwrap();

        assert!(
            (full_from - cut_from).abs() <= 0.02,
            "left edge moved {full_from} -> {cut_from} after truncation"
        );
        assert!(
            cut_to >= 5.2,
            "right edge {cut_to} should follow the data to its truncated end"
        );
    }

    #[test]
    fn edges_never_return_the_apex() {
        let x = grid(0.0, 1.0, 40);
        let y: Vec<f64> = x.iter().map(|&v| bell(v, 0.5, 1.0, 0.1)).collect();
        let data = DataXY { x, y };
        let apex = closest_index(&data.x, 0.5);

        let edges = get_boundaries(&data, 0.5, BoundariesOptions::default());
        let from = edges.from.index.unwrap();
        let to = edges.to.index.unwrap();

        assert!(
            from < apex && apex < to,
            "edges [{from}, {to}] must bracket the apex {apex}"
        );
    }

    #[test]
    fn walk_starts_from_the_smoothed_top() {
        let x = grid(0.0, 1.0, 200);
        let mut y: Vec<f64> = x.iter().map(|&v| bell(v, 0.5, 1.0, 0.03)).collect();
        let apex = closest_index(&x, 0.5);
        y[apex] *= 0.85;
        let seed = x[apex - 2];
        let data = DataXY { x, y };

        let edges = get_boundaries(&data, seed, BoundariesOptions::default());
        let from = edges.from.value.unwrap();
        let to = edges.to.value.unwrap();

        assert!(
            from < 0.48 && to > 0.52,
            "bracket [{from}, {to}] must span the whole peak"
        );
    }

    #[test]
    fn short_input_does_not_panic() {
        let x = grid(0.0, 1.0, 4);
        let y = vec![0.0, 1.0, 0.5, 0.2];
        let edges = get_boundaries(&DataXY { x, y }, 0.3, BoundariesOptions::default());
        assert!(edges.from.index.is_some() && edges.to.index.is_some());
    }
}
