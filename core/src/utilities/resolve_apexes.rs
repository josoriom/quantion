use crate::utilities::get_boundaries::SmoothedSignal;

const DETECTION_LIMIT_IN_SIGMA: f64 = 3.0;

pub(crate) fn resolve_apexes(
    signal: &[f64],
    smoothed: &SmoothedSignal,
    seeds: &[usize],
    noise: f64,
    top_valley_drop: Option<f64>,
) -> Vec<usize> {
    let mut tops: Vec<usize> = seeds.iter().map(|&seed| smoothed.find_top(seed)).collect();
    tops.sort_unstable();
    tops.dedup();

    let detector = ValleyDetector::new(signal, smoothed, noise, top_valley_drop);
    while let Some(position) = detector.find_least_significant_valley(&tops) {
        tops.remove(position);
    }
    tops
}

struct Valley {
    lower_top_position: usize,
    depth_in_sigma: f64,
}

struct ValleyDetector<'a> {
    signal: &'a [f64],
    smoothed: &'a SmoothedSignal<'a>,
    noise_band: f64,
    depth_sigma_per_residual: f64,
    top_valley_drop: Option<f64>,
}

impl<'a> ValleyDetector<'a> {
    fn new(
        signal: &'a [f64],
        smoothed: &'a SmoothedSignal<'a>,
        noise: f64,
        top_valley_drop: Option<f64>,
    ) -> Self {
        let point_weight = smoothed.point_weight;
        let depth_sigma_per_residual = if point_weight < 1.0 {
            (2.0 * point_weight / (1.0 - point_weight)).sqrt()
        } else {
            0.0
        };
        Self {
            signal,
            smoothed,
            noise_band: smoothed.get_noise_band(noise),
            depth_sigma_per_residual,
            top_valley_drop,
        }
    }

    fn find_least_significant_valley(&self, tops: &[usize]) -> Option<usize> {
        tops.windows(2)
            .enumerate()
            .filter_map(|(position, pair)| self.measure_valley(position, pair[0], pair[1]))
            .filter(|valley| valley.depth_in_sigma < DETECTION_LIMIT_IN_SIGMA)
            .min_by(|left, right| left.depth_in_sigma.total_cmp(&right.depth_in_sigma))
            .map(|valley| valley.lower_top_position)
    }

    fn measure_valley(&self, position: usize, left_top: usize, right_top: usize) -> Option<Valley> {
        let bottom = self.find_lowest_index(left_top, right_top);
        if self.value(bottom) <= self.noise_band {
            return None;
        }
        let (lower_top, lower_top_position) = if self.value(left_top) < self.value(right_top) {
            (left_top, position)
        } else {
            (right_top, position + 1)
        };
        let depth = self.value(lower_top) - self.value(bottom);
        if self
            .top_valley_drop
            .is_some_and(|drop| depth >= drop * self.value(lower_top))
        {
            return None;
        }
        let depth_sigma = self.get_depth_sigma(lower_top.min(bottom), lower_top.max(bottom));
        let depth_in_sigma = if depth > 0.0 {
            depth / depth_sigma
        } else {
            0.0
        };
        Some(Valley {
            lower_top_position,
            depth_in_sigma,
        })
    }

    fn get_depth_sigma(&self, from: usize, to: usize) -> f64 {
        self.get_residual_spread(from, to) * self.depth_sigma_per_residual
    }

    fn get_residual_spread(&self, from: usize, to: usize) -> f64 {
        let half_window = self.smoothed.half_window;
        let from = from.saturating_sub(half_window);
        let to = (to + half_window).min(self.signal.len() - 1);
        let sum_of_squares: f64 = (from..=to)
            .map(|index| (self.signal[index] - self.value(index)).powi(2))
            .sum();
        (sum_of_squares / (to - from + 1) as f64).sqrt()
    }

    fn find_lowest_index(&self, from: usize, to: usize) -> usize {
        (from..=to)
            .min_by(|&left, &right| self.value(left).total_cmp(&self.value(right)))
            .unwrap_or(from)
    }

    fn value(&self, index: usize) -> f64 {
        self.smoothed.values[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utilities::{get_boundaries::BoundariesOptions, structs::DataXY};

    fn bell(index: usize, center: f64, height: f64, fwhm: f64) -> f64 {
        let sigma = fwhm / 2.354_820_045;
        height * (-0.5 * ((index as f64 - center) / sigma).powi(2)).exp()
    }

    fn get_tops(y: Vec<f64>, seeds: &[usize], noise: f64) -> Vec<usize> {
        let data = DataXY {
            x: (0..y.len()).map(|index| index as f64 * 0.0043).collect(),
            y,
        };
        let smoothed = SmoothedSignal::new(&data, &BoundariesOptions::default());
        resolve_apexes(&data.y, &smoothed, seeds, noise, None)
    }

    #[test]
    fn seeds_on_one_noisy_top_become_one_top() {
        let mut y: Vec<f64> = (0..80)
            .map(|index| bell(index, 40.0, 6000.0, 6.0))
            .collect();
        y[39] = 6000.0;
        y[40] = 5400.0;
        y[41] = 5900.0;
        let tops = get_tops(y, &[39, 41], 100.0);
        assert_eq!(tops.len(), 1, "tops {tops:?}");
    }

    #[test]
    fn keeps_two_peaks_with_a_deep_valley() {
        let y: Vec<f64> = (0..100)
            .map(|index| bell(index, 40.0, 5000.0, 6.0) + bell(index, 60.0, 20000.0, 6.0))
            .collect();
        let tops = get_tops(y, &[40, 60], 100.0);
        assert_eq!(tops, vec![40, 60]);
    }

    #[test]
    fn keeps_two_peaks_split_by_baseline() {
        let y: Vec<f64> = (0..100)
            .map(|index| bell(index, 30.0, 300.0, 6.0) + bell(index, 70.0, 20000.0, 6.0))
            .collect();
        let tops = get_tops(y, &[30, 70], 100.0);
        assert_eq!(tops, vec![30, 70]);
    }
}
