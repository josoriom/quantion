/// **Target** · Inputs
///
/// One compound. range limits the search in find_targets; missing or 0 searches the whole EIC. predict_rts and calibrate do not use range.
///
/// Fields:
/// - `id`: Your name for it. Not used in the search. Optional.
/// - `mz`: m/z (Da).
/// - `rt`: Expected retention time.
/// - `range`: Half-width of the search window around rt. Missing or 0 = search the whole EIC. Optional.
///
/// Example:
/// ```ignore
/// let target = Target { id: Some("compound_1".into()), mz: 260.103, rt: 2.383, range: Some(0.05) };
/// ```
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#Target)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Target {
    pub id: Option<String>,
    pub mz: f64,
    pub rt: f64,
    pub range: Option<f64>,
}

impl Target {
    pub(crate) fn get_id(&self) -> &str {
        self.id.as_deref().unwrap_or("")
    }

    pub(crate) fn get_range(&self) -> f64 {
        self.range.unwrap_or(0.0)
    }
}
