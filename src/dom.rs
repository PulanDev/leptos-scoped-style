/// DOM helpers for applying scope attributes to elements at runtime.
///
/// This module is only compiled when targeting WebAssembly (i.e. the browser).
/// On the server (SSR) DOM manipulation is not possible, and the scope
/// attributes are instead applied by the client after hydration.
#[cfg(target_arch = "wasm32")]
pub(crate) mod browser {
    use web_sys::Element;

    /// Recursively walk `parent`'s children and set `content_attr` on each
    /// element, stopping at elements that are themselves component hosts
    /// (i.e. elements that already carry a `_leptoshost-*` attribute).
    ///
    /// Stopping at nested hosts ensures that each component's content
    /// attribute is applied only to its own DOM, not to the internals of a
    /// child scoped component.
    pub(crate) fn tag_subtree(parent: &Element, content_attr: &str) {
        let children = parent.children();
        for i in 0..children.length() {
            if let Some(child) = children.item(i) {
                // Tag every direct child with the host's content attribute —
                // it is part of this component's view.
                let _ = child.set_attribute(content_attr, "");

                // Stop recursing at scope boundaries so we do not bleed the
                // host's content attribute into elements that belong to a
                // different scope:
                //
                //  • `_leptosscope`  — the host element of a nested <Scoped>
                //    component (its internals are tagged by the inner on_load).
                //
                //  • `_leptosslot`   — the <leptos-slot> wrapper that
                //    ScopedChildren wraps around projected children; those
                //    elements belong to the caller's scope, not this one.
                if is_scope_boundary(&child) {
                    continue;
                }

                tag_subtree(&child, content_attr);
            }
        }
    }

    /// Returns `true` if the walker should tag `el` but not recurse into it.
    fn is_scope_boundary(el: &Element) -> bool {
        el.has_attribute("_leptosscope") || el.has_attribute("_leptosslot")
    }
}
