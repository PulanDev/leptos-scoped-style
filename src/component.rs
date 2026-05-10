use leptos::children::ToChildren;
use leptos::html;
use leptos::prelude::*;

use crate::registry;
use crate::style::ComponentStyle;

// ── ScopedChildren ────────────────────────────────────────────────────────

/// A non-`Send` children container for the [`Scoped`] component.
///
/// Leptos' built-in [`Children`] type is `Box<dyn FnOnce() -> AnyView + Send>`,
/// which requires every value captured by the children closure — including
/// event handler callbacks like `on:click` — to implement [`Send`].  That
/// constraint fails on non-wasm targets when handlers are plain
/// `Box<dyn FnMut(Event)>`.
///
/// `ScopedChildren` implements [`ToChildren`] **without** the `Send` bound,
/// so the `view!` macro can package non-`Send` closures into `<Scoped>`
/// children without any changes to calling code:
///
/// ```rust,ignore
/// view! {
///     <Scoped style=&BTN>
///         <button on:click=on_click>Click</button>  // on_click need not be Send
///     </Scoped>
/// }
/// ```
pub struct ScopedChildren(Box<dyn FnOnce() -> AnyView + 'static>);

/// Allow `view!` to build a [`ScopedChildren`] from an arbitrary closure
/// without requiring the closure (or anything it captures) to be [`Send`].
impl<F, IV> ToChildren<F> for ScopedChildren
where
    F: FnOnce() -> IV + 'static,
    IV: IntoView + 'static,
{
    #[inline]
    fn to_children(f: F) -> Self {
        ScopedChildren(Box::new(move || f().into_any()))
    }
}

/// Spread host / scope attrs onto the root DOM node(s) of `inner` without an
/// extra wrapper element. See [`Scoped`] `is_directive`.
///
/// `tag` becomes a boolean attribute on the host when it is not the default
/// `"leptos-scope"` (e.g. `tag="mat-button"` → `mat-button=""`).
///
/// When `content_attr` is `Some`, schedules [`crate::dom::browser::tag_subtree`]
/// after mount (`set_timeout(0)`) so descendant nodes exist.
#[cfg(target_arch = "wasm32")]
fn directive_host_view(
    inner: AnyView,
    host_attr: String,
    content_attr: Option<String>,
    host_class: Option<String>,
    tag: &'static str,
) -> AnyView {
    use crate::dom::browser::tag_subtree;
    use leptos::attr::custom::custom_attribute;
    use leptos::tachys::html::class::class;
    use leptos::tachys::html::directive::directive;
    use std::time::Duration;

    let mut v = inner
        .add_any_attr(custom_attribute(host_attr, true))
        .add_any_attr(custom_attribute("_leptosscope", true));

    if tag != "leptos-scope" {
        v = v.add_any_attr(custom_attribute(tag, true));
    }

    v = v.add_any_attr(class(host_class.unwrap_or_default()));

    if let Some(ca) = content_attr {
        v = v.add_any_attr(directive(
            move |el| {
                let ca = ca.clone();
                set_timeout(
                    move || {
                        tag_subtree(&el, &ca);
                    },
                    Duration::ZERO,
                );
            },
            (),
        ));
    }

    v.into_any()
}

#[cfg(not(target_arch = "wasm32"))]
fn directive_host_view(
    inner: AnyView,
    host_attr: String,
    host_class: Option<String>,
    tag: &'static str,
) -> AnyView {
    use leptos::attr::custom::custom_attribute;
    use leptos::tachys::html::class::class;

    let mut v = inner
        .add_any_attr(custom_attribute(host_attr, true))
        .add_any_attr(custom_attribute("_leptosscope", true));

    if tag != "leptos-scope" {
        v = v.add_any_attr(custom_attribute(tag, true));
    }

    v.add_any_attr(class(host_class.unwrap_or_default())).into_any()
}

// ── Scoped component ──────────────────────────────────────────────────────

/// Wrap a component's template in `<Scoped>` to apply emulated CSS scoping.
///
/// On mount, this component:
///
/// 1. Injects a `<style id="leptos-scoped-style-{id}">` into `<head>` (browser) or
///    emits a [`leptos_meta::Style`] element (SSR).
/// 2. Sets `_leptoshost-{id}=""` on the host element — the wrapper, or (when
///    `is_directive` is `true`) the root element(s) of the child view.
/// 3. Sets `_leptoscontent-{id}=""` on every child element recursively
///    (browser only — applied after hydration on the client).
///
/// On unmount, the `<style>` element is removed once the last instance of
/// the component is destroyed.
///
/// The `tag` prop controls the HTML element used as the host wrapper when
/// `is_directive` is `false`. It defaults to `"leptos-scope"`,
/// a custom element name that is invisible to layout engines when styled with
/// `display: contents`. Any valid HTML element name or custom element name
/// (with a hyphen) works.
///
/// When `is_directive` is `true`, no wrapper element is rendered: attributes
/// are spread onto the **root DOM node(s)** produced by the child view (via
/// tachys attribute spreading). You should pass a **single root element**
/// (for example one `<button>...</button>`). The `tag` value is exposed as a
/// **boolean attribute** on that element when it is not the default
/// `"leptos-scope"` (for example `tag="mat-button"` becomes `mat-button=""`).
///
/// Event handlers (`on:click`, `on:mousedown`, etc.) do **not** need to be
/// `Send`; [`ScopedChildren`] handles that transparently.
///
/// # Example
///
/// ```rust,ignore
/// use leptos::prelude::*;
/// use leptos_scoped_style::{ComponentStyle, Scoped};
///
/// static BTN: ComponentStyle = ComponentStyle::css(
///     "my-button",
///     ":host { display: inline-block; }
///      button { color: red; }",
/// );
///
/// #[component]
/// fn MyButton(label: String, on_click: impl FnMut(MouseEvent) + 'static) -> impl IntoView {
///     view! {
///         // Default tag: renders as <leptos-scope>
///         <Scoped style=&BTN>
///             <button on:click=on_click>{label}</button>
///         </Scoped>
///
///         // Or choose a specific tag:
///         <Scoped style=&BTN tag="my-host">
///             <button>{label}</button>
///         </Scoped>
///     }
/// }
/// ```
#[component]
pub fn Scoped(
    /// The [`ComponentStyle`] static that describes this component's styles.
    style: &'static ComponentStyle,
    #[prop(optional)]
    class: Option<String>,
    /// HTML tag for the host wrapper element.
    ///
    /// Defaults to `"leptos-scope"`.  Use any valid custom-element name
    /// (must contain a hyphen) or a standard HTML element name.
    #[prop(optional, default = "leptos-scope")]
    tag: &'static str,
    /// When `true`, do not render a wrapper element; spread host / scope
    /// attributes onto the child view's root element(s). See component docs.
    #[prop(optional, default = false)]
    is_directive: bool,
    children: ScopedChildren,
) -> impl IntoView {
    let scope_id = style.scope_id();
    let scope_id_str = scope_id.as_str().to_owned();

    // ── 1. Inject styles (browser DOM or SSR head) ────────────────────────
    registry::inject(style);

    // SSR: emit the <Style> element through leptos_meta so it appears in <head>.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let css = style.scoped_css().to_owned();
        let style_id = format!("leptos-scoped-style-{}", scope_id_str);
        let _ = view! {
            <leptos_meta::Style id=style_id>
                {css}
            </leptos_meta::Style>
        };
    }

    // ── 2. Schedule cleanup when this instance is destroyed ───────────────
    on_cleanup(move || {
        registry::release(&scope_id_str);
    });

    let inner = (children.0)();

    #[cfg(target_arch = "wasm32")]
    let shell = {
        use crate::dom::browser::tag_subtree;

        let ha = scope_id.host_attr();
        let ca = scope_id.content_attr();

        if is_directive {
            directive_host_view(inner, ha, Some(ca), class, tag)
        } else {
            let root = NodeRef::<html::Custom<&'static str>>::new();
            let class = class.clone();

            root.on_load(move |el| {
                let _ = el.set_attribute(&ha, "");
                if let Some(class) = class {
                    let _ = el.set_class_name(&class);
                }
                tag_subtree(el.as_ref(), &ca);
            });

            html::custom(tag)
                .attr("_leptosscope", "")
                .node_ref(root)
                .child(inner)
                .into_any()
        }
    };

    #[cfg(not(target_arch = "wasm32"))]
    let shell = if is_directive {
        directive_host_view(inner, scope_id.host_attr(), class, tag)
    } else {
        html::custom(tag)
            .attr("_leptosscope", "")
            .class(class)
            .child(inner)
            .into_any()
    };

    shell
}

// ── GlobalStyles ───────────────────────────────────────────────────────────

/// Mount document-global CSS once: injects a `<style>` / [`leptos_meta::Style`]
/// the same way as [`Scoped`], but renders **no** wrapper and does not stamp
/// `_leptoshost` / `_leptoscontent` on any element.
///
/// Use with [`ComponentStyle::global_css`](crate::ComponentStyle::global_css)
/// so rules such as `:root { … }` or `* { … }`
/// are not rewritten by the [`crate::css::scoper::CssScoper`].
///
/// Place one `<GlobalStyles>` near the app root (for example inside your
/// router) so tokens load before themed components render.
#[component]
pub fn GlobalStyles(
    /// Typically built with [`ComponentStyle::global_css`](crate::ComponentStyle::global_css).
    style: &'static ComponentStyle,
) -> impl IntoView {
    let scope_id = style.scope_id();
    let release_key = scope_id.as_str().to_owned();

    registry::inject(style);

    on_cleanup(move || {
        registry::release(&release_key);
    });

    #[cfg(not(target_arch = "wasm32"))]
    {
        let css = style.scoped_css().to_owned();
        let style_id = format!("leptos-scoped-style-{}", style.scope_id().as_str());
        view! {
            <leptos_meta::Style id=style_id>
                {css}
            </leptos_meta::Style>
        }
        .into_any()
    }

    #[cfg(target_arch = "wasm32")]
    {
        view! { }.into_any()
    }
}
