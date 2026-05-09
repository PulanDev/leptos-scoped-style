//! # leptos-scoped-style
//!
//! Scoped CSS for [Leptos](https://leptos.dev) components with emulated view
//! encapsulation — the same technique used by Angular.
//!
//! ## How it works
//!
//! When a component mounts, `leptos-scoped-style`:
//!
//! 1. **Rewrites CSS selectors** at first mount — every selector gets a unique
//!    attribute appended (e.g. `button[_leptoscontent-c1a2b3c4]`), so styles
//!    are isolated to that component's own DOM nodes.
//! 2. **Injects a `<style>` element** into `<head>` with the scoped CSS.
//! 3. **Tags every child DOM element** with `_leptoscontent-{id}=""` so the
//!    rewritten selectors match.
//! 4. **Tags the host wrapper** with `_leptoshost-{id}=""` for `:host` rules.
//! 5. **Removes the `<style>`** when the last instance of the component is
//!    destroyed.
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use leptos::prelude::*;
//! use leptos_scoped_style::{ComponentStyle, Scoped};
//!
//! // Declare once per component — usually at the top of the module.
//! static BTN: ComponentStyle = ComponentStyle::css(
//!     "my-button",  // must be unique in your application
//!     ":host  { display: inline-block; }
//!      button { color: red; }
//!      button:hover { color: darkred; }",
//! );
//!
//! #[component]
//! fn MyButton(label: String) -> impl IntoView {
//!     view! {
//!         <Scoped style=&BTN>
//!             <button>{label}</button>
//!         </Scoped>
//!     }
//! }
//! ```
//!
//! ## Sass / SCSS (`scss_file` feature)
//!
//! Enable the `scss_file` feature and use [`scss_file!`] with a path **relative
//! to your crate root** (the package that contains the `scss_file!` call).
//! Compilation runs at **macro expansion time** (`cargo build`), so `@use` /
//! `@forward` resolve the same way as on-disk Sass in apps and library crates.
//!
//! ```toml
//! leptos-scoped-style = { version = "0.1", features = ["scss_file"] }
//! ```
//!
//! ```rust,ignore
//! use leptos_scoped_style::{ComponentStyle, scss_file, Scoped};
//!
//! static BTN: ComponentStyle = ComponentStyle::css(
//!     "my-button",
//!     scss_file!("src/components/button/button.scss"),
//! );
//! ```
//!
//! For **global** tokens (`:root`, universal `* { … }`), use
//! [`ComponentStyle::global_css`] and mount once with [`GlobalStyles`].
//!
//! ## Supported CSS patterns
//!
//! | You write | Rendered as |
//! |---|---|
//! | `div { }` | `div[_leptoscontent-{id}] { }` |
//! | `a:hover { }` | `a[_leptoscontent-{id}]:hover { }` |
//! | `input::placeholder { }` | `input[_leptoscontent-{id}]::placeholder { }` |
//! | `:host { }` | `[_leptoshost-{id}] { }` |
//! | `:host(.active) { }` | `[_leptoshost-{id}].active { }` |
//! | `:host .child { }` | `[_leptoshost-{id}] [_leptoscontent-{id}] { }` |
//! | `:host:hover { }` | `[_leptoshost-{id}]:hover { }` |
//! | `:host:hover .x { }` | `[_leptoshost-{id}]:hover .x[_leptoscontent-{id}] { }` |
//! | `:root { }` | unchanged (global document root) |
//! | `:skip-scope(sel)` | `sel` unchanged — no content-attribute rewrite |
//! | `@media (…) { div { } }` | at-rule passed through; inner selectors scoped |

mod css;
mod dom;
mod registry;
mod scope_id;
mod style;

pub mod component;

// ── Public re-exports ─────────────────────────────────────────────────────

pub use component::{GlobalStyles, Scoped, ScopedChildren};
pub use scope_id::ScopeId;
pub use style::ComponentStyle;

/// Compile an SCSS file to CSS at **macro expansion time** and embed the result
/// as a `&'static str` expression.
///
/// The path is **relative to the invoking crate's** `CARGO_MANIFEST_DIR` (the
/// package being compiled), so `@use` / `@forward` work inside component
/// libraries—not only in the app crate.
///
/// Requires the `scss_file` crate feature.
///
/// # Example
/// ```rust,ignore
/// use leptos_scoped_style::{ComponentStyle, scss_file};
///
/// static BTN: ComponentStyle = ComponentStyle::css(
///     "my-button",
///     scss_file!("src/components/button/button.scss"),
/// );
/// ```
#[cfg(feature = "scss_file")]
pub use leptos_scoped_style_macros::scss_file;
