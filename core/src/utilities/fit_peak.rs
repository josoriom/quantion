use crate::utilities::{
    cheminfo::lm::{Data2D, LevenbergMarquardtOptions, lm},
    functions::{emg_fn, gaussian_fn, sigma_from_fwhm},
    structs::{DataXY, Peak},
};

const FIT_ITERATIONS: usize = 40;
const FIT_TIMEOUT_SECONDS: f64 = 1.0;
const MIN_POINTS: usize = 5;

/// **PeakShape** · Enums
///
/// Curve used to fit a peak.
///
/// Fields:
/// - `GAUSSIAN`: Symmetric Gaussian.
/// - `EMG`: Exponentially modified Gaussian: a Gaussian with a tail on the right (default).
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#PeakShape)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum PeakShape {
    GAUSSIAN,
    EMG,
}

#[derive(Clone, Copy, Debug)]
pub struct PeakSeed {
    pub rt: f64,
    pub intensity: f64,
}

impl From<&Peak> for PeakSeed {
    fn from(peak: &Peak) -> Self {
        Self {
            rt: peak.rt,
            intensity: peak.intensity,
        }
    }
}

/// **PeakParameters** · Results
///
/// One peak curve: returned by fit_peak, or written by hand to simulate. draw_peaks takes a list of them. tail = 0 is a Gaussian, tail above 0 an EMG.
///
/// Fields:
/// - `height`: Height of the curve at its apex.
/// - `center`: Retention time of the apex (minutes).
/// - `fwhm`: Full width at half maximum of the Gaussian part (minutes). With a tail, the drawn peak is a little wider.
/// - `tail`: Length of the right-hand tail. 0 = Gaussian, above 0 = EMG.
/// - `r2`: Fit quality (0–1), set by fit_peak. Not needed to draw.
///
/// Example:
/// ```ignore
/// let peak = PeakParameters { height: 9800.0, center: 2.38, fwhm: 0.021, tail: 0.006, r2: 0.0 };
/// ```
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#PeakParameters)
#[derive(Clone, Debug)]
pub struct PeakParameters {
    pub height: f64,
    pub center: f64,
    pub fwhm: f64,
    pub tail: f64,
    pub r2: f64,
}

pub fn fit_peak(data: &DataXY, seed: &PeakSeed, shape: PeakShape) -> Option<PeakParameters> {
    if data.x.len() < MIN_POINTS || data.x.len() != data.y.len() || !(seed.intensity > 0.0) {
        return None;
    }
    let width = estimate_fwhm(&data.x, &data.y);
    if !width.is_finite() || width <= 0.0 {
        return None;
    }
    let scaled_x: Vec<f64> = data
        .x
        .iter()
        .map(|value| (value - seed.rt) / width)
        .collect();
    let scaled_y: Vec<f64> = data.y.iter().map(|value| value / seed.intensity).collect();

    let mut params = match shape {
        PeakShape::GAUSSIAN => {
            let fwhm = fit_gaussian(&scaled_x, &scaled_y)?;
            PeakParameters {
                height: seed.intensity,
                center: seed.rt,
                fwhm: fwhm * width,
                tail: 0.0,
                r2: 0.0,
            }
        }
        PeakShape::EMG => {
            let fit = fit_emg(&scaled_x, &scaled_y)?;
            let fwhm = fit[2].abs().max(1e-9) * width;
            let tail = fit[3].abs().max(1e-9) * width;
            let apex = get_emg_apex(sigma_from_fwhm(fwhm), tail);
            PeakParameters {
                height: fit[0] * seed.intensity * apex.value,
                center: seed.rt + fit[1] * width + apex.offset,
                fwhm,
                tail,
                r2: 0.0,
            }
        }
    };

    let curve = build_curve(&params);
    params.r2 = r_squared(&data.x, &data.y, curve.as_ref());
    Some(params)
}

pub fn draw_peak(data: &DataXY, params: &PeakParameters) -> DataXY {
    let curve = build_curve(params);
    let y = data.x.iter().map(|&value| curve(value)).collect();
    DataXY {
        x: data.x.clone(),
        y,
    }
}

pub fn draw_peaks(peaks: &[PeakParameters], from: f64, to: f64, points: usize) -> DataXY {
    if peaks.is_empty() || points == 0 {
        return DataXY::empty();
    }
    let (from, to) = if from == 0.0 && to == 0.0 {
        get_drawing_range(peaks)
    } else {
        (from, to)
    };
    let step = if points > 1 {
        (to - from) / (points - 1) as f64
    } else {
        0.0
    };
    let x: Vec<f64> = (0..points)
        .map(|point| from + step * point as f64)
        .collect();
    let curves: Vec<Box<dyn Fn(f64) -> f64>> = peaks.iter().map(build_curve).collect();
    let y = x
        .iter()
        .map(|&value| curves.iter().map(|curve| curve(value)).sum())
        .collect();
    DataXY { x, y }
}

fn get_drawing_range(peaks: &[PeakParameters]) -> (f64, f64) {
    const WIDTHS_BEFORE: f64 = 2.0;
    const WIDTHS_AFTER: f64 = 2.0;
    const TAILS_AFTER: f64 = 10.0;
    let from = peaks
        .iter()
        .map(|peak| peak.center - WIDTHS_BEFORE * peak.fwhm.abs())
        .fold(f64::INFINITY, f64::min);
    let to = peaks
        .iter()
        .map(|peak| peak.center + WIDTHS_AFTER * peak.fwhm.abs() + TAILS_AFTER * peak.tail.abs())
        .fold(f64::NEG_INFINITY, f64::max);
    (from, to)
}

struct EmgApex {
    offset: f64,
    value: f64,
}

/// ## Apex of an exponentially modified Gaussian
/// The EMG has no closed form for its maximum, so the apex of the unit curve (height 1, centre 0) is found by
/// golden-section search (Press et al., Numerical Recipes, section 10.2). The apex lies between the centre of the
/// Gaussian part and that centre plus the tail plus a few widths, and the curve has one maximum there.
/// - `offset`: apex position minus the centre of the Gaussian part.
/// - `value`: curve value at the apex for height 1.
fn get_emg_apex(sigma: f64, tail: f64) -> EmgApex {
    const RATIO: f64 = 0.618_033_988_749_894_9;
    const STEPS: usize = 200;
    let curve = |x: f64| emg_fn(x, 1.0, 0.0, sigma, tail);
    let mut low = -sigma;
    let mut high = tail + 4.0 * sigma;
    let mut left = high - RATIO * (high - low);
    let mut right = low + RATIO * (high - low);
    let mut left_value = curve(left);
    let mut right_value = curve(right);
    for _ in 0..STEPS {
        if left_value < right_value {
            low = left;
            left = right;
            left_value = right_value;
            right = low + RATIO * (high - low);
            right_value = curve(right);
        } else {
            high = right;
            right = left;
            right_value = left_value;
            left = high - RATIO * (high - low);
            left_value = curve(left);
        }
    }
    let offset = 0.5 * (low + high);
    EmgApex {
        offset,
        value: curve(offset),
    }
}

fn build_curve(params: &PeakParameters) -> Box<dyn Fn(f64) -> f64> {
    let height = params.height;
    let center = params.center;
    let sigma = sigma_from_fwhm(params.fwhm.abs().max(1e-9));
    let tail = params.tail.abs();
    if tail > 0.0 {
        let apex = get_emg_apex(sigma, tail);
        let amplitude = height / apex.value.max(1e-300);
        let gaussian_center = center - apex.offset;
        Box::new(move |x| emg_fn(x, amplitude, gaussian_center, sigma, tail))
    } else {
        Box::new(move |x| gaussian_fn(x, height, center, sigma))
    }
}

fn fit_gaussian(x: &[f64], y: &[f64]) -> Option<f64> {
    let options = LevenbergMarquardtOptions {
        initial_values: vec![1.0],
        min_values: Some(vec![0.05]),
        max_values: Some(vec![20.0]),
        max_iterations: Some(FIT_ITERATIONS),
        central_difference: Some(true),
        timeout: Some(FIT_TIMEOUT_SECONDS),
        ..Default::default()
    };
    let data = Data2D {
        x: x.to_vec(),
        y: y.to_vec(),
    };
    lm(&data, &gaussian_unit, &options)
        .ok()
        .map(|fit| fit.parameter_values[0].abs().max(1e-9))
}

fn fit_emg(x: &[f64], y: &[f64]) -> Option<Vec<f64>> {
    let center = -0.4;
    let tail = 0.3;
    let height = 1.0 / emg_fn(0.0, 1.0, center, sigma_from_fwhm(1.0), tail).max(1e-6);
    let options = LevenbergMarquardtOptions {
        initial_values: vec![height, center, 1.0, tail],
        min_values: Some(vec![0.0, -4.0, 0.1, 1e-4]),
        max_values: Some(vec![20.0 * height, 0.3, 8.0, 8.0]),
        max_iterations: Some(FIT_ITERATIONS),
        central_difference: Some(true),
        timeout: Some(FIT_TIMEOUT_SECONDS),
        ..Default::default()
    };
    let data = Data2D {
        x: x.to_vec(),
        y: y.to_vec(),
    };
    lm(&data, &emg_unit, &options)
        .ok()
        .map(|fit| fit.parameter_values)
}

fn gaussian_unit(params: &[f64]) -> Box<dyn Fn(f64) -> f64> {
    let sigma = sigma_from_fwhm(params[0].abs().max(1e-9));
    Box::new(move |x| gaussian_fn(x, 1.0, 0.0, sigma))
}

fn emg_unit(params: &[f64]) -> Box<dyn Fn(f64) -> f64> {
    let height = params[0];
    let center = params[1];
    let sigma = sigma_from_fwhm(params[2].abs().max(1e-9));
    let tail = params[3].abs().max(1e-9);
    Box::new(move |x| emg_fn(x, height, center, sigma, tail))
}

fn estimate_fwhm(x: &[f64], y: &[f64]) -> f64 {
    let mut apex = 0;
    let mut height = y[0];
    for (index, &value) in y.iter().enumerate() {
        if value > height {
            height = value;
            apex = index;
        }
    }
    if !(height > 0.0) {
        return 0.0;
    }
    let half = 0.5 * height;
    let left = cross_before(x, y, apex, half);
    let right = cross_after(x, y, apex, half);
    match (left, right) {
        (Some(left), Some(right)) => (right - left).abs(),
        (Some(left), None) => 2.0 * (x[apex] - left).abs(),
        (None, Some(right)) => 2.0 * (right - x[apex]).abs(),
        (None, None) => (x[x.len() - 1] - x[0]).abs() / 6.0,
    }
}

fn cross_before(x: &[f64], y: &[f64], apex: usize, half: f64) -> Option<f64> {
    let mut index = apex;
    while index > 0 {
        if y[index] >= half && y[index - 1] < half {
            return Some(crossing(
                x[index - 1],
                y[index - 1],
                x[index],
                y[index],
                half,
            ));
        }
        index -= 1;
    }
    None
}

fn cross_after(x: &[f64], y: &[f64], apex: usize, half: f64) -> Option<f64> {
    let mut index = apex;
    while index + 1 < y.len() {
        if y[index] >= half && y[index + 1] < half {
            return Some(crossing(
                x[index],
                y[index],
                x[index + 1],
                y[index + 1],
                half,
            ));
        }
        index += 1;
    }
    None
}

fn crossing(x1: f64, y1: f64, x2: f64, y2: f64, target: f64) -> f64 {
    let slope = y2 - y1;
    if slope.abs() < 1e-18 {
        return x1;
    }
    x1 + (target - y1) / slope * (x2 - x1)
}

fn r_squared(x: &[f64], y: &[f64], curve: &dyn Fn(f64) -> f64) -> f64 {
    let count = y.len();
    if count < 3 {
        return 0.0;
    }
    let mean = y.iter().sum::<f64>() / count as f64;
    let mut residual = 0.0;
    let mut total = 0.0;
    for index in 0..count {
        let expected = curve(x[index]);
        residual += (y[index] - expected).powi(2);
        total += (y[index] - mean).powi(2);
    }
    if total <= 1e-18 {
        return 0.0;
    }
    1.0 - residual / total
}
