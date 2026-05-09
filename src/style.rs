use std::sync::OnceLock;

use crate::css::{
    scoper::CssScoper,
    StyleSource,
};
use crate::scope_id::ScopeId;

/// Declares the scoped styles for a Leptos component.
///
/// Construct one of these as a `static` — once per component — and pass it to
/// [`Scoped`](crate::component::Scoped). CSS is scoped lazily on first mount,
/// then cached for the lifetime of the process.
///
/// # Examples
///
/// ```rust
/// use leptos_scoped_style::ComponentStyle;
///
/// // Plain CSS
/// static BTN: ComponentStyle = ComponentStyle::css(
///     "my-button",
///     ":host { display: inline-block; }
///      button { color: red; }",
/// );
///
/// ```
pub struct ComponentStyle {
    /// Application-unique component name.  Used to derive the [`ScopeId`].
    name: &'static str,

    /// Raw style source before compilation / scoping.
    source: StyleSource,

    /// When `true`, CSS is injected as-is (document-global). Use for `:root`,
    /// `*` resets, and other rules that must not receive `[_leptoscontent-*]`.
    skip_scoping: bool,

    /// Lazily computed, fully-scoped CSS string.
    ///
    /// Populated on first call to [`scoped_css`].  The `OnceLock` guarantees
    /// that selector rewriting happens at most once, even if
    /// multiple instances of the component are mounted concurrently.
    cached: OnceLock<String>,
}

impl ComponentStyle {
    /// Create a `ComponentStyle` from a plain CSS string.
    ///
    /// `name` must be unique across all components in the application.
    pub const fn css(name: &'static str, css: &'static str) -> Self {
        Self {
            name,
            source: StyleSource::Css(css),
            skip_scoping: false,
            cached: OnceLock::new(),
        }
    }

    /// Like [`Self::css`], but the stylesheet is injected **without** selector
    /// rewriting (no `_leptoshost` / `_leptoscontent` attributes).
    ///
    /// Use this for global design tokens (`:root { --token: … }`), universal
    /// resets (`* { box-sizing: … }`), and any CSS that must apply to the
    /// whole document. Pair with [`GlobalStyles`](crate::GlobalStyles) at the
    /// app root so the `<style>` is mounted once.
    pub const fn global_css(name: &'static str, css: &'static str) -> Self {
        Self {
            name,
            source: StyleSource::Css(css),
            skip_scoping: true,
            cached: OnceLock::new(),
        }
    }

    /// Returns the fully scoped CSS string.
    ///
    /// The result is cached after the first call.
    pub(crate) fn scoped_css(&self) -> &str {
        self.cached.get_or_init(|| {
            let raw_css = self.compile_source();
            if self.skip_scoping {
                raw_css
            } else {
                let id = ScopeId::from_name(self.name);
                let host_attr = id.host_attr();
                let content_attr = id.content_attr();
                CssScoper::new(&host_attr, &content_attr).scope(&raw_css)
            }
        })
    }

    /// Returns the [`ScopeId`] derived from this component's name.
    pub(crate) fn scope_id(&self) -> ScopeId {
        ScopeId::from_name(self.name)
    }

    // ── Private helpers ───────────────────────────────────────────────────

    /// Compile the style source to plain CSS.
    ///
    /// For `StyleSource::Css` this is a no-op.
    fn compile_source(&self) -> String {
        match &self.source {
            StyleSource::Css(css) => css.to_string(),
        }
    }
}

// `ComponentStyle` is used as a `static`, so it must be `Sync`.
// `OnceLock<String>` is `Sync`, and all other fields are `'static` references.
// Safety: the `Sync` bound is automatically derived because all fields are Sync.
unsafe impl Sync for ComponentStyle {}
