use std::cell::RefCell;
use std::collections::HashMap;

use crate::style::ComponentStyle;

thread_local! {
    static REGISTRY: RefCell<HashMap<String, usize>> =
        RefCell::new(HashMap::new());
}

/// Register a new mount of `style`.
///
/// * On the **browser** (Wasm): injects a `<style id="leptos-style-{id}">` into
///   `document.head` the first time this component type is mounted.
/// * On the **server** (SSR): emits a [`leptos_meta::Style`] element, which
///   Leptos places in the `<head>` of the rendered HTML string.
pub(crate) fn inject(style: &'static ComponentStyle) {
    let id = style.scope_id();
    let key = id.as_str().to_owned();
    let css = style.scoped_css().to_owned();

    REGISTRY.with(|reg| {
        let mut map = reg.borrow_mut();
        let count = map.entry(key.clone()).or_insert(0);
        *count += 1;

        if *count == 1 {
            drop(map); // release borrow before DOM work
            insert_style_element(&key, &css);
        }
    });
}

/// Un-register one mount of the component identified by `scope_id_str`.
///
/// When the count reaches zero the `<style>` element is removed from the DOM.
pub(crate) fn release(scope_id_str: &str) {
    let key = scope_id_str.to_owned();

    REGISTRY.with(|reg| {
        let mut map = reg.borrow_mut();
        if let Some(count) = map.get_mut(&key) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                map.remove(&key);
                drop(map);
                remove_style_element(&key);
            }
        }
    });
}

// ── Browser implementation (wasm32) ──────────────────────────────────────

#[cfg(target_arch = "wasm32")]
fn insert_style_element(id: &str, css: &str) {
    let Some(window)   = web_sys::window()   else {
        web_sys::console::error_1(&"[leptos-style] no window".into());
        return;
    };
    let Some(document) = window.document()  else {
        web_sys::console::error_1(&"[leptos-style] no document".into());
        return;
    };
    let Some(head) = document.head() else {
        web_sys::console::error_1(&"[leptos-style] no <head>".into());
        return;
    };

    let element_id = format!("leptos-style-{id}");

    // Hydration guard: skip if SSR already injected this style.
    if document.get_element_by_id(&element_id).is_some() {
        return;
    }

    if css.is_empty() {
        web_sys::console::warn_1(
            &format!("[leptos-style] scoped CSS for '{id}' is empty — check your SCSS for syntax errors").into()
        );
        return;
    }

    let Ok(el) = document.create_element("style") else {
        web_sys::console::error_1(&"[leptos-style] create_element('style') failed".into());
        return;
    };

    el.set_id(&element_id);

    // Use a text node — the most reliable way to inject raw CSS into a
    // <style> element without HTML-encoding concerns.
    let text = document.create_text_node(css);
    let _ = el.append_child(&text);
    let _ = head.append_child(&el);
}

#[cfg(target_arch = "wasm32")]
fn remove_style_element(id: &str) {
    let Some(window)   = web_sys::window()   else { return };
    let Some(document) = window.document()   else { return };

    let element_id = format!("leptos-style-{id}");
    if let Some(el) = document.get_element_by_id(&element_id) {
        el.remove();
    }
}

// ── SSR implementation (non-wasm32) ──────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
fn insert_style_element(_id: &str, _css: &str) {
    // On the server the `<Style>` component is rendered directly from
    // `component.rs` via `leptos_meta::Style`, so there is nothing to do here.
    // The REGISTRY still tracks ref counts so that `release` does not attempt
    // to remove non-existent elements.
}

#[cfg(not(target_arch = "wasm32"))]
fn remove_style_element(_id: &str) {
    // No DOM element to remove on the server.
}
