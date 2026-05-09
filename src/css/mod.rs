pub mod scoper;

/// The raw source of a component's styles, before any compilation or scoping.
///
/// `Css` holds a plain CSS string.
pub(crate) enum StyleSource {
    /// A plain CSS string (already valid CSS).
    Css(&'static str),
}
