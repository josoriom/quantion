use crate::{
    error::{ErrorKind, QuantionError},
    files::{from_fast_error, read_error},
    sample_file::SampleFile,
    utilities::{
        load_model::{Model, ModelError, ModelInput, load_model as read_model},
        structs::Target,
        warp_model::{PredictedRt, WarpError, WarpModel, load_warp as read_warp},
    },
};

/// **load_model** · Models
///
/// Load an ONNX model from a path, a URL or bytes, and read the keys and values stored in it.
///
/// - `input`: Where the model is: a path, an http(s) URL, or raw bytes.
///
/// Returns: The model, with get_keys(), get_values() and get_value(key).
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#load_model)
pub fn load_model(input: impl Into<ModelInput>) -> Result<Model, QuantionError> {
    const FUNCTION: &str = "load_model";
    read_model(input).map_err(|ModelError::NotRead(reason)| read_error(FUNCTION, reason))
}

/// **load_warp** · Models
///
/// Make a retention-time warp model from a loaded model.
///
/// - `model`: A model from load_model that holds a warp.
///
/// Returns: The warp model.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#load_warp)
pub fn load_warp(model: &Model) -> Result<WarpModel, QuantionError> {
    const FUNCTION: &str = "load_warp";
    read_warp(model).map_err(|error| from_warp_error(FUNCTION, error))
}

impl WarpModel {
    /// **predict_rts** · Models
    ///
    /// Predict where each target elutes in one sample.
    ///
    /// - `file`: The sample to read, from parse_ion or parse_mzml.
    /// - `targets`: Targets with mz and reference rt. id and range are optional and not used. Each Target has `id`, `mz`, `rt`, `range`. See [Target](https://phenological.github.io/quantion/?language=rust#Target).
    ///   Example: `let target = Target { id: Some("compound_1".into()), mz: 260.103, rt: 2.383, range: Some(0.05) };`
    ///
    /// Returns: One PredictedRt per target, in the same order.
    ///
    /// [Documentation](https://phenological.github.io/quantion/?language=rust#predict_rts)
    pub fn predict_rts(
        &self,
        file: &mut SampleFile,
        targets: &[Target],
    ) -> Result<Vec<PredictedRt>, QuantionError> {
        const FUNCTION: &str = "predict_rts";
        self.predict_rts_from_reader(&mut file.reader(), targets)
            .map_err(|error| from_warp_error(FUNCTION, error))
    }

    /// **calibrate** · Models
    ///
    /// Turn the retention times seen in anchor samples (LTRs, QCs) into targets for predict_rts. Each id gets the median rt over the anchors.
    ///
    /// - `anchor_samples`: Anchor samples.
    /// - `target_lists`: For each anchor sample, the targets seen in it with their observed rt. Each Target has `id`, `mz`, `rt`, `range`. See [Target](https://phenological.github.io/quantion/?language=rust#Target).
    ///   Example: `let target = Target { id: Some("compound_1".into()), mz: 260.103, rt: 2.383, range: Some(0.05) };`
    ///
    /// Returns: One Target per id with the median rt.
    ///
    /// [Documentation](https://phenological.github.io/quantion/?language=rust#calibrate)
    pub fn calibrate(
        &self,
        anchor_samples: &mut [SampleFile],
        target_lists: &[Vec<Target>],
    ) -> Result<Vec<Target>, QuantionError> {
        const FUNCTION: &str = "calibrate";
        let mut readers: Vec<_> = anchor_samples.iter_mut().map(SampleFile::reader).collect();
        self.calibrate_from_readers(&mut readers, target_lists)
            .map_err(|error| from_warp_error(FUNCTION, error))
    }
}

pub(crate) fn from_warp_error(function: &'static str, error: WarpError) -> QuantionError {
    let message = error.to_string();
    match error {
        WarpError::TargetMissing(index) => QuantionError {
            kind: ErrorKind::InvalidArgument,
            function,
            argument: Some(format!("targets[{index}]")),
            message,
        },
        WarpError::IdMissing { list, target } => QuantionError {
            kind: ErrorKind::MissingArgument,
            function,
            argument: Some(format!("target_lists[{list}][{target}].id")),
            message,
        },
        WarpError::ListsDiffer { .. } => QuantionError {
            kind: ErrorKind::InvalidArgument,
            function,
            argument: Some("target_lists".to_string()),
            message,
        },
        WarpError::FileNotRead(error) => from_fast_error(function, error),
        _ => QuantionError::new(ErrorKind::ModelMismatch, function, message),
    }
}
