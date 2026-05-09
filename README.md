# leptos-style

Scoped CSS for [Leptos](https://leptos.dev/) with **emulated view encapsulation** (the same attribute-scoping approach Angular uses): styles apply only inside your component unless you deliberately opt out.

[![crates.io](https://img.shields.io/crates/v/leptos-style.svg)](https://crates.io/crates/leptos-style)
[![docs.rs](https://docs.rs/leptos-style/badge.svg)](https://docs.rs/leptos-style)
[![repository](https://img.shields.io/badge/repo-GitHub-181717)](https://github.com/PulanDev/leptos-style)

## Why use this?

- **Real CSS**: write normal rules; no DSL tied to Rust types.
- **Scoped by default**: selectors are rewritten to match only your component subtree.
- **`:host`-style ergonomics**: target the wrapper (or directive root) with `:host`.
- **Ref-counted `<style>` tags**: injected once per component *type*, removed when the last instance unmounts.
- **SSR**: styles are emitted with `leptos_meta::Style`; the client avoids duplicating tags during hydration.

## Installation

Compatible with **Leptos 0.8**.

```toml
[dependencies]
leptos-style = "0.1"
```

By default only plain CSS APIs are available (`ComponentStyle::css`, `global_css`). Enable **`scss_file`** when you want the `scss_file!` macro.

Optional feature:

| Feature      | Enables |
|--------------|---------|
| `scss_file`  | Compile `.scss` files at **build** time (`scss_file!`), with `@use` / `@forward` relative to your crate root. Pulls in `leptos-style-macros`. |

```toml
leptos-style = { version = "0.1", features = ["scss_file"] }
```

## Quick start

Declare one [`ComponentStyle`](https://docs.rs/leptos-style/latest/leptos_style/struct.ComponentStyle.html) per component module (typically a `static`), then wrap markup in [`Scoped`](https://docs.rs/leptos-style/latest/leptos_style/component/fn.Scoped.html).

The `class` argument sets the CSS **class attribute on the scope host** (the wrapper element, or the root element in directive mode).

```rust
use leptos::prelude::*;
use leptos_style::{ComponentStyle, Scoped};

static MY_BUTTON: ComponentStyle = ComponentStyle::css(
    "my-button", // unique name in your app — used to derive stable scope ids
    r#"
    :host { display: inline-block; }
    button { color: crimson; }
    button:hover { color: darkred; }
"#,
);

#[component]
fn MyButton(label: String) -> impl IntoView {
    view! {
        <Scoped style=&MY_BUTTON class="">
            <button type="button">{label}</button>
        </Scoped>
    }
}
```

Without a wrapper (attributes go on the **single root element** only — use directive mode responsibly):

```rust
view! {
    <Scoped style=&MY_BUTTON class="my-button-root" tag="mat-button" is_directive=true>
        <button type="button">{label}</button>
    </Scoped>
}
```

Here `tag="mat-button"` becomes the boolean HTML attribute `mat-button=""` on the root when it is not the default (`"leptos-scope"`).

## `ComponentStyle` constructors

| Constructor | Scoped? | Notes |
|-------------|---------|--------|
| `css(name, css)` | Yes | Plain CSS string (`scss_file!` can produce the argument). |
| `global_css(name, css)` | No | Inject as-is (`:root`, resets, tokens). Pair with [`GlobalStyles`](https://docs.rs/leptos-style/latest/leptos_style/component/fn.GlobalStyles.html). |
| `scss_file!("path/to/file.scss")` *(feature `scss_file`)* | — | Macro expands to `&'static str` compiled CSS at build time. Path is relative to **`CARGO_MANIFEST_DIR`** of the invoking crate. |

Example with compile-time Sass:

```rust
#[cfg(feature = "scss_file")]
use leptos_style::{scss_file, ComponentStyle, Scoped};

#[cfg(feature = "scss_file")]
static WIDGET_STYLES: ComponentStyle = ComponentStyle::css(
    "my-widget",
    scss_file!("src/components/widget/widget.scss"),
);

#[cfg(feature = "scss_file")]
#[component]
fn MyWidget() -> impl IntoView {
    view! {
        <Scoped style=&WIDGET_STYLES class="">
            /* … */
        </Scoped>
    }
}
```

Use `ComponentStyle::global_css` plus one [`GlobalStyles`](https://docs.rs/leptos-style/latest/leptos_style/component/fn.GlobalStyles.html) near the app shell for **`:root`** and **universal** rules so they stay document-wide (use plain CSS strings, or precompile tokens with Sass in your crate if you prefer).

```rust
static TOKENS: ComponentStyle =
    ComponentStyle::global_css("design-tokens", ":root { --brand: hsl(220 70% 50%); }");

// In your shell / router layout:
view! { <GlobalStyles style=&TOKENS /> }
```

## How it works (short)

1. On first mount of a style, the crate **rewrites** your CSS: most selectors gain a `[_leptoscontent-{id}]` attribute selector; `:host`-style patterns map to `[_leptoshost-{id}]`.
2. A **`<style id="leptos-style-{id}">`** is appended to **`document.head`** (browser) or rendered via **`leptos_meta::Style`** (SSR).
3. The **host** element gets **`_leptoshost-{id}`**; **descendants** get **`_leptoscontent-{id}`** in the DOM (Wasm), with recursion stopping at nested [`Scoped`] hosts and **`_leptosslot`** boundaries.
4. A **reference count** tracks live instances; when it hits zero, the client **removes** the `<style>` node.

Lazy compilation and scoping are cached with `OnceLock` per `ComponentStyle`.

## Selector cheat sheet

| You write | Rewritten roughly to |
|-----------|---------------------|
| `div { … }` | `div[_leptoscontent-{id}] { … }` |
| `a:hover { … }` | `a[_leptoscontent-{id}]:hover { … }` |
| `input::placeholder { … }` | `input[_leptoscontent-{id}]::placeholder { … }` |
| `:host { … }` | `[_leptoshost-{id}] { … }` |
| `:host(.active) { … }` | `[_leptoshost-{id}].active { … }` |
| `:host .child { … }` | `[_leptoshost-{id}] .child[_leptoscontent-{id}] { … }` |
| `:host:hover { … }` | `[_leptoshost-{id}]:hover { … }` |
| `:root { … }` | Unchanged |
| `:skip-scope(sel)` | `sel` verbatim (library-only pseudo; escapes scoping — use sparingly for portals / globals). |

At-rules such as `@media` are passed through while **their inner** rule selectors are scoped.

## `Scoped` props (summary)

- **`style`** — `&'static ComponentStyle`.
- **`class`** — `String` applied to the host (wrapper or directive root).
- **`tag`** — Host element name when *not* in directive mode. Default **`"leptos-scope"`** (a custom element; pair with CSS like `leptos-scope { display: contents; }` if you want it layout-neutral).
- **`is_directive`** — If `true`, no extra wrapper; host attrs are spread onto the child view root. Prefer a single root DOM node.

## Non-`Send` children

[`Scoped`](https://docs.rs/leptos-style/latest/leptos_style/component/fn.Scoped.html) uses **`ScopedChildren`**, which does **not** require `Send`. Event handlers captured in `<Scoped>...</Scoped>` therefore work without forcing `Send` on non-Wasm targets.

## SSR and hydration

The server emits scoped CSS via `leptos_meta`; the Wasm client skips inserting a duplicate `<style>` when the SSR id is already present. Content attributes on deep nodes are finalized on the client as the tree mounts.

## Contributing

Issues and pull requests are welcome at [github.com/PulanDev/leptos-style](https://github.com/PulanDev/leptos-style).

## License

Licensed under **MIT OR Apache-2.0**, at your option, as specified in `Cargo.toml`.  
`SPDX-License-Identifier: MIT OR Apache-2.0`

For crates.io conventions, consider adding matching `LICENSE-MIT` / `LICENSE-APACHE` (or equivalent) files to the repo before publishing.

## Publishing (maintainers)

This repository ships two crates: **`leptos-style`** (library) and **`leptos-style-macros`** (proc-macros behind the `scss_file` feature). To publish both to [crates.io](https://crates.io):

1. Publish **`leptos-style-macros`** first (`crates/leptos-style-macros`).
2. In the root `Cargo.toml`, change the optional dependency from `path = "crates/leptos-style-macros"` to **`version = "…"`** matching what you published.
3. Publish **`leptos-style`**.

Add `LICENSE-MIT` and/or `LICENSE-APACHE` (or a single well-known boilerplate layout) matching `license = "MIT OR Apache-2.0"` in `Cargo.toml` before publishing — crates.io and downstream users expect them.

