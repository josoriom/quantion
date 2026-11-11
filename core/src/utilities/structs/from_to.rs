/// **FromTo** · Inputs
///
/// A range: retention-time windows and m/z ranges.
///
/// Fields:
/// - `from`: Start of the range.
/// - `to`: End of the range.
///
/// Example:
/// ```ignore
/// let window = FromTo { from: 1.0, to: 2.5 };
/// ```
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#FromTo)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FromTo {
    pub from: f64,
    pub to: f64,
}
