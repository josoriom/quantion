/// **Transition** · Inputs
///
/// One SRM transition.
///
/// Fields:
/// - `id`: Your name for it. Optional.
/// - `precursor`: Precursor (Q1) m/z.
/// - `product`: Product (Q3) m/z.
/// - `rt`: Expected retention time.
/// - `range`: Half-width of the search window around rt.
///
/// Example:
/// ```ignore
/// let transition = Transition { id: Some("compound_1".into()), precursor: 89.04768, product: 89.04768, rt: 2.385, range: 0.1 };
/// ```
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#Transition)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Transition {
    pub id: Option<String>,
    pub precursor: f64,
    pub product: f64,
    pub rt: f64,
    pub range: f64,
}
