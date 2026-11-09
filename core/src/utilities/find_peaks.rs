use std::cmp::Ordering;

use crate::{
    traces::{CalculateBaselineOptions, FindPeaksOptions},
    utilities::{
        calculate_baseline::calculate_baseline,
        cheminfo::sgg::{SggOptions, sgg},
        closest_index,
        find_noise_level::find_noise_level,
        get_boundaries::{Boundaries, BoundariesOptions, SmoothedSignal, find_boundaries},
        math::xy_integration,
        resolve_apexes::resolve_apexes,
        scan_for_peaks::scan_for_peaks,
        shape_filter::{Candidate, ShapeFilter},
        structs::{DataXY, Peak},
    },
};

const PEAK_EDGE_SLOPE_LEVEL: u8 = 0;

#[derive(Clone, Debug)]
struct PeakCandidate {
    from: f64,
    to: f64,
    from_idx: usize,
    to_idx: usize,
    rt: f64,
    integral: f64,
    intensity: f64,
    n_points: usize,
    r2: f64,
    noise: f64,
}

impl From<PeakCandidate> for Peak {
    fn from(c: PeakCandidate) -> Self {
        Peak {
            from: c.from,
            to: c.to,
            rt: c.rt,
            integral: c.integral,
            intensity: c.intensity,
            r2: if c.r2.is_finite() { c.r2 } else { 0.0 },
            n_points: c.n_points,
            noise: c.noise,
        }
    }
}

impl Candidate for PeakCandidate {
    fn bounds(&self) -> (usize, usize) {
        (self.from_idx, self.to_idx)
    }

    fn set_score(&mut self, r2: f64) {
        self.r2 = r2;
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Separation {
    pub(crate) valley_drop: f64,
    pub(crate) top_valley_drop: Option<f64>,
}

impl Separation {
    pub(crate) const ANY_PEAK: Self = Self {
        valley_drop: 0.70,
        top_valley_drop: None,
    };
    pub(crate) const EXPECTED_PEAK: Self = Self {
        valley_drop: 0.50,
        top_valley_drop: Some(0.50),
    };
}

pub fn find_peaks(data: &DataXY, options: &FindPeaksOptions) -> Vec<Peak> {
    find_separated_peaks(data, options, Separation::ANY_PEAK)
}

pub(crate) fn find_separated_peaks(
    data: &DataXY,
    options: &FindPeaksOptions,
    separation: Separation,
) -> Vec<Peak> {
    let n = data.y.len();
    if n == 0 {
        return Vec::new();
    }

    let auto_baseline = options.auto_baseline;
    let auto_noise = options.auto_noise;
    let allow_overlap = options.allow_overlap;

    let baseline: Vec<f64> = if auto_baseline {
        let baseline_options = CalculateBaselineOptions {
            lambda: options.lambda,
            max_iterations: options.max_iterations,
        };
        calculate_baseline(&data.y, baseline_options, PEAK_EDGE_SLOPE_LEVEL)
    } else {
        vec![0.0; n]
    };

    let y_center: Vec<f64> = (0..n).map(|i| (data.y[i] - baseline[i]).max(0.0)).collect();

    let normalized_data = DataXY {
        x: data.x.clone(),
        y: y_center,
    };

    let noise: f64 = if auto_noise {
        find_noise_level(&normalized_data.y).intensity
    } else {
        options.noise.unwrap_or(0.0)
    };

    let positions = scan_for_peaks(&normalized_data);
    if positions.is_empty() {
        return Vec::new();
    }

    let bopt = BoundariesOptions::default();

    let smoothed_signal = get_smoothed_signal(
        &normalized_data,
        (options.kernel_size > 0).then_some(options.kernel_size),
    );
    let boundary_input = smoothed_signal.as_ref().unwrap_or(&normalized_data);

    let smoothed = SmoothedSignal::new(boundary_input, &bopt);
    let seeds: Vec<usize> = positions
        .iter()
        .map(|&seed_rt| closest_index(&normalized_data.x, seed_rt))
        .filter(|&seed_index| normalized_data.y[seed_index] > noise)
        .collect();
    let tops = resolve_apexes(
        &boundary_input.y,
        &smoothed,
        &seeds,
        noise,
        separation.top_valley_drop,
    );

    let mut candidates: Vec<PeakCandidate> = Vec::with_capacity(tops.len());
    for (position, &top) in tops.iter().enumerate() {
        let search_start = if position > 0 { tops[position - 1] } else { 0 };
        let search_end = tops.get(position + 1).copied().unwrap_or(n - 1);
        let boundaries = find_boundaries(&smoothed, top, search_start..=search_end, noise, &bopt);
        let (rt, apex_y) = apex_in_window(&normalized_data, &boundaries)
            .unwrap_or((normalized_data.x[top], normalized_data.y[top]));
        if apex_y <= noise {
            continue;
        }

        match (
            boundaries.from.index,
            boundaries.from.value,
            boundaries.to.index,
            boundaries.to.value,
        ) {
            (Some(fi), Some(fx), Some(ti), Some(tx)) if fi < ti => {
                let (integral, intensity) = xy_integration(&data.x[fi..=ti], &data.y[fi..=ti]);
                candidates.push(PeakCandidate {
                    from: fx,
                    to: tx,
                    from_idx: fi,
                    to_idx: ti,
                    rt,
                    integral,
                    intensity,
                    n_points: ti - fi + 1,
                    r2: f64::NAN,
                    noise,
                });
            }
            _ => {}
        }
    }
    keep_peaks_above_floor(&mut candidates, &data.x, &data.y);
    if candidates.is_empty() {
        return Vec::new();
    }

    ShapeFilter::new(options.min_r2, options.shape).retain(
        &mut candidates,
        &data.x,
        &data.y,
        &baseline,
    );

    if candidates.is_empty() {
        return Vec::new();
    }

    let mut peaks = filter_peak_candidates(candidates, options);

    peaks.sort_by(|a, b| a.rt.partial_cmp(&b.rt).unwrap_or(Ordering::Equal));
    peaks = dedupe_near_identical(peaks);

    if !peaks.is_empty() {
        let mut cutoff = 0.0_f64;
        if noise > 0.0 {
            cutoff = options.min_snr * noise;
        }
        if options.min_intensity > cutoff {
            cutoff = options.min_intensity;
        }

        if cutoff > 0.0 {
            peaks.retain(|p| p.intensity > cutoff);
        }
    }

    if peaks.len() > 1 {
        peaks = suppress_contained_peaks(data, &normalized_data.y, noise, peaks);
    }

    if peaks.len() > 1 {
        peaks = keep_prominent_peaks(
            data,
            &normalized_data.y,
            noise,
            separation.valley_drop,
            peaks,
        );
    }

    if !allow_overlap && peaks.len() > 1 {
        peaks = split_overlapping_windows(data, &normalized_data.y, peaks);
    }

    peaks
}

fn get_smoothed_signal(input: &DataXY, kernel_size: Option<usize>) -> Option<DataXY> {
    let window = get_smoothing_window(kernel_size?, input.y.len())?;
    let smoothed = sgg(
        &input.y,
        &input.x,
        SggOptions {
            window_size: window,
            derivative: 0,
            polynomial: 3,
        },
    );
    Some(DataXY {
        x: input.x.clone(),
        y: smoothed.into_iter().map(keep_above_zero).collect(),
    })
}

fn get_smoothing_window(requested: usize, length: usize) -> Option<usize> {
    let mut window = requested.min(length);
    if window.is_multiple_of(2) {
        window -= 1;
    }
    (window >= 5).then_some(window)
}

fn keep_above_zero(value: f64) -> f64 {
    value.max(0.0)
}

pub fn apex_in_window(data: &DataXY, b: &Boundaries) -> Option<(f64, f64)> {
    const APEX_MIN_PAD: usize = 3;

    let n = data.y.len();
    if n == 0 {
        return None;
    }

    let mut l = b.from.index?;
    let mut r = b.to.index?;
    if l >= n || r >= n {
        return None;
    }
    if l > r {
        core::mem::swap(&mut l, &mut r);
    }

    let need = 2 * APEX_MIN_PAD + 1;
    let width = r - l + 1;
    if width < need {
        let c = (l + r) / 2;
        l = c.saturating_sub(APEX_MIN_PAD);
        r = (c + APEX_MIN_PAD).min(n - 1);
    }

    if l >= r {
        return None;
    }

    let mut max_off = 0usize;
    let mut max_y = f64::NEG_INFINITY;
    let mut k = 0usize;
    while k <= (r - l) {
        let y = data.y[l + k];
        if y > max_y {
            max_y = y;
            max_off = k;
        }
        k += 1;
    }
    let i = l + max_off;
    Some((data.x[i], max_y))
}

fn filter_peak_candidates(peaks: Vec<PeakCandidate>, options: &FindPeaksOptions) -> Vec<Peak> {
    peaks
        .into_iter()
        .filter(|peak| peak.intensity >= options.min_intensity)
        .filter(|peak| peak.n_points >= options.min_peak_width_points)
        .filter(|peak| {
            options
                .min_integral
                .is_none_or(|min_integral| peak.integral >= min_integral)
        })
        .map(Peak::from)
        .collect()
}

fn dedupe_near_identical(peaks: Vec<Peak>) -> Vec<Peak> {
    let m = peaks.len();
    if m <= 1 {
        return peaks;
    }

    let eps_rt = 1e-6;
    let eps_w = 1e-6;

    let mut out: Vec<Peak> = Vec::with_capacity(m);
    let mut i = 0usize;

    while i < m {
        let p = &peaks[i];
        let mut best_idx = i;
        let mut j = i + 1;

        while j < m {
            let q = &peaks[j];
            let same = (p.from - q.from).abs() <= eps_w
                && (p.to - q.to).abs() <= eps_w
                && (p.rt - q.rt).abs() <= eps_rt;
            if same {
                if q.intensity > peaks[best_idx].intensity {
                    best_idx = j;
                }
                j += 1;
            } else {
                break;
            }
        }

        out.push(peaks[best_idx]);
        i = j;
    }

    out
}

fn suppress_contained_peaks(
    data: &DataXY,
    centered: &[f64],
    noise: f64,
    mut peaks: Vec<Peak>,
) -> Vec<Peak> {
    let m = peaks.len();
    if m <= 1 {
        return peaks;
    }

    const OVERLAP_FRAC_SHOULDER: f64 = 0.70;
    const INTENSITY_RATIO_SHOULDER: f64 = 0.10;
    const MERGE_SHOULDERS: bool = true;

    let mut order: Vec<usize> = (0..m).collect();
    order.sort_by(|&i, &j| {
        peaks[j]
            .intensity
            .partial_cmp(&peaks[i].intensity)
            .unwrap_or(Ordering::Equal)
    });

    let mut keep = vec![true; m];

    let mut a_rank = 0usize;
    while a_rank < order.len() {
        let ia = order[a_rank];
        if keep[ia] {
            let mut b_rank = a_rank + 1;
            while b_rank < order.len() {
                let ib = order[b_rank];
                if keep[ib] {
                    let la = peaks[ia].from;
                    let ra = peaks[ia].to;
                    let lb = peaks[ib].from;
                    let rb = peaks[ib].to;

                    let wb = (rb - lb).abs();
                    if wb <= 0.0 {
                        keep[ib] = false;
                        b_rank += 1;
                        continue;
                    }

                    let l = if la > lb { la } else { lb };
                    let r = if ra < rb { ra } else { rb };
                    let overlap = (r - l).max(0.0);
                    let frac = overlap / wb;

                    let apex_b_inside_a = peaks[ib].rt >= la && peaks[ib].rt <= ra;
                    let rel = peaks[ib].intensity / peaks[ia].intensity;

                    let apex_a = closest_index(&data.x, peaks[ia].rt);
                    let apex_b = closest_index(&data.x, peaks[ib].rt);
                    let valley = valley_between(centered, apex_a, apex_b);
                    let smaller = height_at(centered, apex_a).min(height_at(centered, apex_b));
                    let prominence = (smaller - valley).max(0.0);
                    let strongly_resolved =
                        noise > 0.0 && prominence > SEPARATE_PROMINENCE_NOISE_MULT * noise;

                    let enough_overlap = frac >= OVERLAP_FRAC_SHOULDER;
                    let inside_and_small = apex_b_inside_a && rel <= INTENSITY_RATIO_SHOULDER;
                    let is_shoulder = (enough_overlap || inside_and_small) && !strongly_resolved;

                    if is_shoulder {
                        if MERGE_SHOULDERS {
                            let new_from = if la < lb { la } else { lb };
                            let new_to = if ra > rb { ra } else { rb };

                            let li = closest_index(&data.x, new_from);
                            let ri = closest_index(&data.x, new_to);

                            let lidx = if li <= ri { li } else { ri };
                            let ridx = if li <= ri { ri } else { li };

                            let (integral, _) =
                                xy_integration(&data.x[lidx..=ridx], &data.y[lidx..=ridx]);

                            peaks[ia].from = new_from;
                            peaks[ia].to = new_to;
                            peaks[ia].integral = integral;
                            peaks[ia].n_points = ridx - lidx + 1;
                        }

                        keep[ib] = false;
                    }
                }
                b_rank += 1;
            }
        }
        a_rank += 1;
    }

    let mut out = Vec::<Peak>::with_capacity(m);
    for i in 0..m {
        if keep[i] {
            out.push(peaks[i]);
        }
    }
    out.sort_by(|a, b| a.rt.partial_cmp(&b.rt).unwrap_or(Ordering::Equal));
    out
}

const COMPARABLE_HEIGHT_RATIO: f64 = 0.50;
const COMPARABLE_MIN_VALLEY_DROP: f64 = 0.10;
const SEPARATE_PROMINENCE_NOISE_MULT: f64 = 5.0;

fn keep_prominent_peaks(
    data: &DataXY,
    centered: &[f64],
    noise: f64,
    valley_drop: f64,
    peaks: Vec<Peak>,
) -> Vec<Peak> {
    let mut kept: Vec<Peak> = Vec::with_capacity(peaks.len());
    for peak in peaks {
        if let Some(&last) = kept.last() {
            let last_apex = closest_index(&data.x, last.rt);
            let peak_apex = closest_index(&data.x, peak.rt);
            let valley = valley_between(centered, last_apex, peak_apex);
            let last_height = height_at(centered, last_apex);
            let peak_height = height_at(centered, peak_apex);
            let smaller = last_height.min(peak_height);
            let taller = last_height.max(peak_height);
            let prominence = (smaller - valley).max(0.0);
            if !are_peaks_resolved(prominence, smaller, taller, noise, valley_drop) {
                let merged = merge_peaks(data, last, peak);
                kept.pop();
                kept.push(merged);
                continue;
            }
        }
        kept.push(peak);
    }
    kept
}

fn valley_between(values: &[f64], first: usize, second: usize) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let lo = first.min(second);
    let hi = first.max(second).min(values.len() - 1);
    let mut lowest = f64::INFINITY;
    for &value in &values[lo..=hi] {
        if value < lowest {
            lowest = value;
        }
    }
    lowest
}

fn height_at(values: &[f64], index: usize) -> f64 {
    values.get(index).copied().unwrap_or(0.0)
}

fn are_peaks_resolved(
    prominence: f64,
    smaller_height: f64,
    taller_height: f64,
    noise: f64,
    valley_drop: f64,
) -> bool {
    if smaller_height <= 0.0 {
        return false;
    }
    let well_resolved = prominence >= valley_drop * smaller_height;
    let comparable_pair = smaller_height >= COMPARABLE_HEIGHT_RATIO * taller_height
        && prominence >= COMPARABLE_MIN_VALLEY_DROP * smaller_height;
    let far_above_noise = noise > 0.0 && prominence > SEPARATE_PROMINENCE_NOISE_MULT * noise;
    well_resolved || comparable_pair || far_above_noise
}

fn merge_peaks(data: &DataXY, keeper: Peak, other: Peak) -> Peak {
    let taller = if keeper.intensity >= other.intensity {
        keeper
    } else {
        other
    };
    let from = keeper.from.min(other.from);
    let to = keeper.to.max(other.to);
    let left = closest_index(&data.x, from);
    let right = closest_index(&data.x, to);
    let (lo, hi) = if left <= right {
        (left, right)
    } else {
        (right, left)
    };
    let (integral, _) = xy_integration(&data.x[lo..=hi], &data.y[lo..=hi]);
    Peak {
        from,
        to,
        integral,
        n_points: hi - lo + 1,
        ..taller
    }
}

fn split_overlapping_windows(data: &DataXY, centered: &[f64], mut peaks: Vec<Peak>) -> Vec<Peak> {
    let mut i = 0;
    while i + 1 < peaks.len() {
        if peaks[i].to > peaks[i + 1].from {
            let apex_left = closest_index(&data.x, peaks[i].rt);
            let apex_right = closest_index(&data.x, peaks[i + 1].rt);
            let split = valley_index(centered, apex_left, apex_right);
            let boundary = data.x[split];
            peaks[i] = reframe(data, peaks[i], peaks[i].from, boundary);
            peaks[i + 1] = reframe(data, peaks[i + 1], boundary, peaks[i + 1].to);
        }
        i += 1;
    }
    peaks
}

fn valley_index(values: &[f64], first: usize, second: usize) -> usize {
    let lo = first.min(second);
    let hi = first.max(second).min(values.len().saturating_sub(1));
    let mut best = lo;
    let mut lowest = f64::INFINITY;
    for k in lo..=hi {
        if values[k] < lowest {
            lowest = values[k];
            best = k;
        }
    }
    best
}

fn reframe(data: &DataXY, peak: Peak, from: f64, to: f64) -> Peak {
    let left = closest_index(&data.x, from);
    let right = closest_index(&data.x, to);
    let (lo, hi) = if left <= right {
        (left, right)
    } else {
        (right, left)
    };
    let (integral, _) = xy_integration(&data.x[lo..=hi], &data.y[lo..=hi]);
    Peak {
        from,
        to,
        integral,
        n_points: hi - lo + 1,
        ..peak
    }
}

const MIN_RISE_ABOVE_FLOOR: f64 = 2.0;
const FLOOR_WINDOW_MINUTES: f64 = 1.5;
const MIN_FLOOR_POINTS: usize = 8;

fn get_values_around_peak(
    times: &[f64],
    values: &[f64],
    apex: usize,
    from: usize,
    to: usize,
) -> Vec<f64> {
    let start = times.partition_point(|&time| time < times[apex] - FLOOR_WINDOW_MINUTES);
    let end = times.partition_point(|&time| time <= times[apex] + FLOOR_WINDOW_MINUTES);
    (start..end)
        .filter(|index| *index < from || *index > to)
        .map(|index| values[index])
        .filter(|value| value.is_finite())
        .collect()
}

fn find_floor(values: &mut [f64]) -> f64 {
    let quarter = values.len() / 4;
    values.select_nth_unstable_by(quarter, |left, right| {
        left.partial_cmp(right).unwrap_or(Ordering::Equal)
    });
    values[quarter].max(1.0)
}

fn rises_above_floor(times: &[f64], values: &[f64], peak: &PeakCandidate) -> bool {
    let apex = closest_index(times, peak.rt);
    if apex >= values.len() || peak.from_idx >= values.len() || peak.to_idx >= values.len() {
        return false;
    }
    if !values[apex].is_finite() {
        return false;
    }
    let mut around = get_values_around_peak(times, values, apex, peak.from_idx, peak.to_idx);
    if around.len() < MIN_FLOOR_POINTS {
        return true;
    }
    values[apex] / find_floor(&mut around) >= MIN_RISE_ABOVE_FLOOR
}

fn keep_peaks_above_floor(peaks: &mut Vec<PeakCandidate>, times: &[f64], values: &[f64]) {
    peaks.retain(|peak| rises_above_floor(times, values, peak));
}
