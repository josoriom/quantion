use std::{collections::HashMap, fmt};

use ionic::{IonReader, source::ByteRange};
use runnet::Tensor;

use crate::{
    files::CalculateEicOptions,
    utilities::{
        calculate_eic::{
            EicReader, FastError, MS1_LEVEL, ScanTime, get_scan_times, mz_tolerance_for,
            plan_window_ranges,
        },
        find_targets::{MzWindow, read_eics_one_pass},
        load_model::Model,
        structs::{FromTo, Target},
    },
};

const INPUT_NAMES: [&str; 2] = ["eics", "targets"];
const ALLOWED_EDGE_GAP: f64 = 0.05;

/// **PredictedRt** · Results
///
/// One predicted retention time from predict_rts.
///
/// Fields:
/// - `rt`: Predicted retention time (minutes).
/// - `uncertainty`: How unsure the prediction is (minutes).
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#PredictedRt)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PredictedRt {
    pub rt: f64,
    pub uncertainty: f64,
}

#[derive(Clone, Debug)]
pub enum WarpError {
    WrongInputs,
    ValueMissing(String),
    TargetMissing(usize),
    ListsDiffer {
        samples: usize,
        lists: usize,
    },
    IdMissing {
        list: usize,
        target: usize,
    },
    ScansNotCovered {
        needed: FromTo,
        found: Option<FromTo>,
    },
    NoBackground,
    FileNotRead(FastError),
    PredictFailed(String),
}

impl fmt::Display for WarpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WarpError::WrongInputs => write!(f, "the model needs the inputs eics and targets"),
            WarpError::ValueMissing(name) => {
                write!(f, "the warp model has no valid value for {}", name)
            }
            WarpError::TargetMissing(index) => {
                write!(f, "target {} has no valid m/z or rt", index + 1)
            }
            WarpError::ListsDiffer { samples, lists } => write!(
                f,
                "calibrate needs one target list per sample, got {} samples and {} target lists",
                samples, lists
            ),
            WarpError::IdMissing { list, target } => {
                write!(f, "target {} in list {} has no id", target + 1, list + 1)
            }
            WarpError::ScansNotCovered {
                needed,
                found: Some(found),
            } => write!(
                f,
                "the file's scans cover {} to {} min, the warp model needs {} to {} min",
                found.from, found.to, needed.from, needed.to
            ),
            WarpError::ScansNotCovered {
                needed,
                found: None,
            } => write!(
                f,
                "the file has no scans, the warp model needs {} to {} min",
                needed.from, needed.to
            ),
            WarpError::NoBackground => {
                write!(f, "the warp model found none of its background traces")
            }
            WarpError::FileNotRead(error) => write!(f, "the file was not read: {}", error),
            WarpError::PredictFailed(reason) => write!(f, "the prediction failed: {}", reason),
        }
    }
}

impl std::error::Error for WarpError {}

pub struct WarpModel {
    model: Model,
    points: usize,
    time_range: FromTo,
    context_mz: Vec<f64>,
    eic_options: CalculateEicOptions,
}

pub fn load_warp(model: &Model) -> Result<WarpModel, WarpError> {
    WarpModel::from_model(model.clone())
}

pub fn plan_predict_rts_ranges(
    warp: &WarpModel,
    ion: &mut IonReader,
    targets: &[Target],
) -> Result<Vec<ByteRange>, WarpError> {
    check_targets(targets)?;
    let windows = warp.get_windows(targets);
    let mz_from = windows
        .iter()
        .map(|window| window.from)
        .fold(f64::INFINITY, f64::min);
    let mz_to = windows
        .iter()
        .map(|window| window.to)
        .fold(f64::NEG_INFINITY, f64::max);
    plan_window_ranges(
        ion,
        warp.time_range.from - ALLOWED_EDGE_GAP,
        warp.time_range.to + ALLOWED_EDGE_GAP,
        mz_from,
        mz_to,
    )
    .map_err(WarpError::FileNotRead)
}

impl WarpModel {
    fn from_model(model: Model) -> Result<Self, WarpError> {
        if model.input_names() != INPUT_NAMES {
            return Err(WarpError::WrongInputs);
        }
        let warp = Self {
            points: get_number(&model, "points")? as usize,
            time_range: FromTo {
                from: get_number(&model, "time_from")?,
                to: get_number(&model, "time_to")?,
            },
            context_mz: get_numbers(&model, "context_mz")?,
            eic_options: CalculateEicOptions {
                ppm_tolerance: get_number(&model, "eic_ppm")?,
                mz_tolerance: get_number(&model, "eic_mz_tolerance")?,
                ..CalculateEicOptions::default()
            },
            model,
        };
        if warp.points < 2 || warp.time_range.to <= warp.time_range.from {
            return Err(WarpError::ValueMissing("points or time range".to_string()));
        }
        Ok(warp)
    }

    pub fn points(&self) -> usize {
        self.points
    }

    pub fn time_range(&self) -> FromTo {
        self.time_range
    }

    pub fn context_mz(&self) -> &[f64] {
        &self.context_mz
    }

    pub fn get_keys(&self) -> &[String] {
        self.model.get_keys()
    }

    pub fn get_values(&self) -> &[String] {
        self.model.get_values()
    }

    pub fn get_value(&self, key: &str) -> Option<&str> {
        self.model.get_value(key)
    }

    pub(crate) fn predict_rts_from_reader(
        &self,
        reader: &mut EicReader,
        targets: &[Target],
    ) -> Result<Vec<PredictedRt>, WarpError> {
        check_targets(targets)?;
        if targets.is_empty() {
            return Ok(Vec::new());
        }
        let scans = get_scan_times(
            reader,
            self.time_range.from - ALLOWED_EDGE_GAP,
            self.time_range.to + ALLOWED_EDGE_GAP,
            MS1_LEVEL,
        );
        self.check_scans(&scans)?;

        let windows = self.get_windows(targets);
        let values =
            read_eics_one_pass(reader, &scans, &windows).map_err(WarpError::FileNotRead)?;
        let scan_rts: Vec<f64> = scans.iter().map(|scan| scan.rt).collect();
        self.predict_from_traces(targets, &scan_rts, &values)
    }

    pub(crate) fn calibrate_from_readers(
        &self,
        samples: &mut [EicReader],
        target_lists: &[Vec<Target>],
    ) -> Result<Vec<Target>, WarpError> {
        check_target_lists(samples.len(), target_lists)?;
        let calibrated_lists = samples
            .iter_mut()
            .zip(target_lists)
            .map(|(sample, targets)| self.calibrate_sample(sample, targets))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(get_median_targets(&calibrated_lists))
    }

    pub(crate) fn calibrate_sample(
        &self,
        sample: &mut EicReader,
        targets: &[Target],
    ) -> Result<Vec<Target>, WarpError> {
        let predicted = self.predict_rts_from_reader(sample, targets)?;
        Ok(targets
            .iter()
            .zip(predicted)
            .map(|(target, prediction)| Target {
                id: target.id.clone(),
                mz: target.mz,
                rt: get_calibrated_rt(target.rt, prediction.rt),
                range: target.range,
            })
            .collect())
    }

    pub(crate) fn check_scans(&self, scans: &[ScanTime]) -> Result<(), WarpError> {
        let found = match (scans.first(), scans.last()) {
            (Some(first), Some(last)) => FromTo {
                from: first.rt,
                to: last.rt,
            },
            _ => {
                return Err(WarpError::ScansNotCovered {
                    needed: self.time_range,
                    found: None,
                });
            }
        };
        if found.from > self.time_range.from + ALLOWED_EDGE_GAP
            || found.to < self.time_range.to - ALLOWED_EDGE_GAP
        {
            return Err(WarpError::ScansNotCovered {
                needed: self.time_range,
                found: Some(found),
            });
        }
        Ok(())
    }

    fn get_windows(&self, targets: &[Target]) -> Vec<MzWindow> {
        let target_mz = targets.iter().map(|target| target.mz);
        target_mz
            .chain(self.context_mz.iter().copied())
            .map(|mz| {
                let tolerance = mz_tolerance_for(mz, self.eic_options);
                MzWindow {
                    from: mz - tolerance,
                    to: mz + tolerance,
                }
            })
            .collect()
    }

    pub(crate) fn predict_from_traces(
        &self,
        targets: &[Target],
        scan_rts: &[f64],
        values: &[f64],
    ) -> Result<Vec<PredictedRt>, WarpError> {
        let row_count = targets.len() + self.context_mz.len();
        if scan_rts.is_empty() || values.len() != row_count * scan_rts.len() {
            return Err(WarpError::PredictFailed(
                "the traces do not match the targets".to_string(),
            ));
        }

        let row_mz = targets
            .iter()
            .map(|target| target.mz)
            .chain(self.context_mz.iter().copied());
        let mut eics = vec![0.0_f32; row_count * (self.points + 1)];
        for ((row, mz), trace) in eics
            .chunks_exact_mut(self.points + 1)
            .zip(row_mz)
            .zip(values.chunks_exact(scan_rts.len()))
        {
            row[0] = mz as f32;
            resample_eic(scan_rts, trace, self.time_range, &mut row[1..]);
        }
        let target_values: Vec<f32> = targets
            .iter()
            .flat_map(|target| [target.mz as f32, target.rt as f32])
            .collect();

        let outputs = self
            .model
            .run(vec![
                Tensor::from_floats([row_count, self.points + 1], eics),
                Tensor::from_floats([targets.len(), 2], target_values),
            ])
            .map_err(|error| WarpError::PredictFailed(error.to_string()))?;
        read_predictions(&outputs, targets.len())
    }
}

fn read_predictions(
    outputs: &[Tensor],
    target_count: usize,
) -> Result<Vec<PredictedRt>, WarpError> {
    let [rts, uncertainties, context_found] = [0, 1, 2].map(|index| {
        outputs
            .get(index)
            .and_then(|output| output.floats().ok())
            .unwrap_or_default()
    });
    if context_found.first().is_none_or(|found| *found < 1.0) {
        return Err(WarpError::NoBackground);
    }
    if rts.len() != target_count || uncertainties.len() != target_count {
        return Err(WarpError::PredictFailed(
            "the model gave the wrong number of predictions".to_string(),
        ));
    }
    Ok(rts
        .iter()
        .zip(uncertainties)
        .map(|(rt, uncertainty)| PredictedRt {
            rt: *rt as f64,
            uncertainty: *uncertainty as f64,
        })
        .collect())
}

fn check_targets(targets: &[Target]) -> Result<(), WarpError> {
    match targets
        .iter()
        .position(|target| !target.mz.is_finite() || !target.rt.is_finite())
    {
        Some(index) => Err(WarpError::TargetMissing(index)),
        None => Ok(()),
    }
}

pub(crate) fn resample_eic(
    scan_rts: &[f64],
    input: &[f64],
    time_range: FromTo,
    output: &mut [f32],
) {
    let last_scan = scan_rts.len() - 1;
    let last_point = output.len().saturating_sub(1).max(1) as f64;
    let step = (time_range.to - time_range.from) / last_point;
    let mut next_scan = 0;

    for (point, value) in output.iter_mut().enumerate() {
        let time = time_range.from + step * point as f64;
        while next_scan <= last_scan && scan_rts[next_scan] < time {
            next_scan += 1;
        }
        *value = get_value_at(scan_rts, input, next_scan, time) as f32;
    }
}

fn get_value_at(scan_rts: &[f64], input: &[f64], next_scan: usize, time: f64) -> f64 {
    if next_scan == 0 {
        return input[0];
    }
    if next_scan == scan_rts.len() {
        return input[next_scan - 1];
    }

    let before = next_scan - 1;
    let gap = scan_rts[next_scan] - scan_rts[before];
    if gap <= 0.0 {
        return input[next_scan];
    }
    let fraction = (time - scan_rts[before]) / gap;
    input[before] + fraction * (input[next_scan] - input[before])
}

fn get_calibrated_rt(rt: f64, predicted_rt: f64) -> f64 {
    2.0 * rt - predicted_rt
}

pub(crate) fn check_target_lists(
    sample_count: usize,
    target_lists: &[Vec<Target>],
) -> Result<(), WarpError> {
    if sample_count != target_lists.len() {
        return Err(WarpError::ListsDiffer {
            samples: sample_count,
            lists: target_lists.len(),
        });
    }
    for (list, targets) in target_lists.iter().enumerate() {
        if let Some(target) = targets.iter().position(|target| target.get_id().is_empty()) {
            return Err(WarpError::IdMissing { list, target });
        }
    }
    Ok(())
}

pub(crate) fn get_median_targets(target_lists: &[Vec<Target>]) -> Vec<Target> {
    let mut order: Vec<&Target> = Vec::new();
    let mut rts: HashMap<&str, Vec<f64>> = HashMap::new();
    for target in target_lists.iter().flatten() {
        let found = rts.entry(target.get_id()).or_default();
        if found.is_empty() {
            order.push(target);
        }
        found.push(target.rt);
    }
    order
        .into_iter()
        .map(|target| Target {
            id: target.id.clone(),
            mz: target.mz,
            rt: get_median(&mut rts[target.get_id()].clone()),
            range: target.range,
        })
        .collect()
}

fn get_median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

fn get_number(model: &Model, name: &str) -> Result<f64, WarpError> {
    model
        .get_value(name)
        .and_then(|text| text.trim().parse().ok())
        .ok_or_else(|| WarpError::ValueMissing(name.to_string()))
}

fn get_numbers(model: &Model, name: &str) -> Result<Vec<f64>, WarpError> {
    model
        .get_value(name)
        .ok_or_else(|| WarpError::ValueMissing(name.to_string()))?
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(str::trim)
        .filter(|number| !number.is_empty())
        .map(|number| {
            number
                .parse()
                .map_err(|_| WarpError::ValueMissing(name.to_string()))
        })
        .collect()
}
