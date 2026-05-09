/// Rewrites CSS selectors to include scope attributes so styles are isolated
/// to a single component.
///
/// ## Supported transformations
///
/// | Input | Output |
/// |---|---|
/// | `div { }` | `div[_leptoscontent-{id}] { }` |
/// | `a:hover { }` | `a[_leptoscontent-{id}]:hover { }` |
/// | `:host { }` | `[_leptoshost-{id}] { }` |
/// | `:host(.cls) { }` | `[_leptoshost-{id}].cls { }` |
/// | `:host .child { }` | `[_leptoshost-{id}] [_leptoscontent-{id}] { }` |
/// | `:host:hover { }`, `:host::before { }` | `[_leptoshost-{id}]:hover`, … |
/// | `:host:hover .x { }` | `[_leptoshost-{id}]:hover .x[_leptoscontent-{id}]` |
/// | `:root { }`, `:root .x { }`, … | passed through unchanged (document-level) |
/// | `:skip-scope(sel)` | `sel` unchanged — no `[_leptoscontent-{id}]` (see module docs) |
/// | `@media (…) { div { } }` | at-rule header passed through; inner selectors scoped |
///
/// `:skip-scope` is compile-time sugar only (invalid in real CSS); use explicit selectors inside.
///
/// Injected `<style>` is global; skipped selectors can match anywhere unless sufficiently specific.
pub struct CssScoper<'a> {
    host_attr: &'a str,
    content_attr: &'a str,
}

impl<'a> CssScoper<'a> {
    /// Create a new scoper.
    ///
    /// * `host_attr`    — attribute name for the host element, e.g. `_leptoshost-c1a2b3c4`
    /// * `content_attr` — attribute name for content elements, e.g. `_leptoscontent-c1a2b3c4`
    pub fn new(host_attr: &'a str, content_attr: &'a str) -> Self {
        Self { host_attr, content_attr }
    }

    /// Rewrite all selectors in `css`, returning the scoped CSS string.
    pub fn scope(&self, css: &str) -> String {
        let mut output = String::with_capacity(css.len() + css.len() / 4);
        self.scope_rules(css.trim(), &mut output);
        output
    }

    // ── Rule iteration (brace counting) ──────────────────────────────────

    fn scope_rules(&self, css: &str, out: &mut String) {
        let mut rest = css;

        while let Some(open) = rest.find('{') {
            let selector_text = rest[..open].trim();
            let after_open = &rest[open..];

            let close = match find_block_end(after_open) {
                Some(i) => i,
                None => {
                    out.push_str(rest);
                    return;
                }
            };

            let block_body = &after_open[1..close];
            let after_block = &after_open[close + 1..];

            if selector_text.starts_with('@') {
                // At-rule (@media, @supports, @layer …): pass the header through
                // and recurse into the block body so inner selectors are scoped.
                out.push_str(selector_text);
                out.push_str(" {\n");
                self.scope_rules(block_body.trim(), out);
                out.push_str("}\n");
            } else {
                let scoped = self.scope_selector_list(selector_text);
                out.push_str(&scoped);
                out.push_str(" {");
                out.push_str(block_body);
                out.push_str("}\n");
            }

            rest = after_block;
        }

        out.push_str(rest);
    }

    // ── Selector list ─────────────────────────────────────────────────────

    fn scope_selector_list(&self, selectors: &str) -> String {
        split_selector_list_top_level(selectors)
            .into_iter()
            .map(|s| self.scope_one_selector(s))
            .collect::<Vec<_>>()
            .join(", ")
    }

    // ── Single selector ───────────────────────────────────────────────────

    fn scope_one_selector(&self, selector: &str) -> String {
        let selector = selector.trim();

        // :skip-scope(inner) → inner (compile-time only; not valid CSS in browsers)
        if let Some(inner) = extract_skip_scope_inner(selector) {
            return inner.to_owned();
        }

        // `:root` and selectors that start with `:root` stay global (CSS variables,
        // theme tokens, etc.) instead of becoming `[_leptoscontent-*]:root`, which
        // would never match the document root.
        if selector == ":root" || selector.starts_with(":root") {
            return selector.to_owned();
        }

        // :host(.foo) → [_leptoshost-{id}].foo
        if let Some(inner) = selector
            .strip_prefix(":host(")
            .and_then(|s| s.strip_suffix(')'))
        {
            return format!("[{}]{}", self.host_attr, inner);
        }

        // :host .child → [_leptoshost-{id}] [_leptoscontent-{id}]
        if let Some(rest) = selector.strip_prefix(":host ") {
            let scoped_rest = self.scope_one_selector(rest.trim());
            return format!("[{}] {}", self.host_attr, scoped_rest);
        }

        // :host>.child → [_leptoshost-{id}]>[_leptoscontent-{id}]
        if let Some(rest) = selector.strip_prefix(":host>") {
            let scoped_rest = self.scope_one_selector(rest.trim());
            return format!("[{}]>{}", self.host_attr, scoped_rest);
        }

        // :host (bare) → [_leptoshost-{id}]
        if selector == ":host" {
            return format!("[{}]", self.host_attr);
        }

        // :host:hover, :host::before, :host[attr], :host.foo, and chains such as
        // `:host:not(.x):hover`, optionally followed by a combinator and more
        // selectors (`:host:hover .child`). Without this, insert_before_pseudo
        // sees the leading `:` and emits [_leptoscontent-*]:host:hover.
        if let Some(rest) = selector.strip_prefix(":host") {
            if rest.starts_with(':')
                || rest.starts_with('.')
                || rest.starts_with('#')
                || rest.starts_with('[')
            {
                return self.scope_host_compound(rest);
            }
        }

        // Regular selector: insert [_leptoscontent-{id}] before the first
        // pseudo-class / pseudo-element that is outside of parentheses.
        let attr = format!("[{}]", self.content_attr);
        insert_before_pseudo(selector, &attr)
    }

    /// `:host` subject tail already stripped (e.g. `:hover`, `:hover .child`).
    fn scope_host_compound(&self, rest: &str) -> String {
        match split_top_level_combinator(rest) {
            Some((host_subject, sep, tail)) => {
                let prefix = format!("[{}]{}", self.host_attr, host_subject);
                let scoped_tail = self.scope_one_selector(tail);
                format!("{prefix}{sep}{scoped_tail}")
            }
            None => format!("[{}]{}", self.host_attr, rest),
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

/// Split a selector list on commas that are **outside** `(...)` and `[...]`.
fn split_selector_list_top_level(s: &str) -> Vec<&str> {
    let mut depth_paren: usize = 0;
    let mut depth_bracket: usize = 0;
    let mut start: usize = 0;
    let mut out = Vec::new();

    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            ',' if depth_paren == 0 && depth_bracket == 0 => {
                let part = s[start..i].trim();
                if !part.is_empty() {
                    out.push(part);
                }
                start = i + 1;
            }
            _ => {}
        }
    }

    let last = s[start..].trim();
    if !last.is_empty() {
        out.push(last);
    }

    out
}

/// Body of `:skip-scope(...)`, with parentheses balanced.
fn extract_skip_scope_inner(selector: &str) -> Option<&str> {
    let inner = selector.strip_prefix(":skip-scope(")?;
    let mut depth: usize = 1;
    for (i, ch) in inner.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(inner[..i].trim());
                }
            }
            _ => {}
        }
    }
    None
}

/// Split `s` at the first top-level combinator (` `, `>`, `+`, `~`).
/// Parentheses depth avoids splitting inside `:not(.a .b)` etc.
fn split_top_level_combinator(s: &str) -> Option<(&str, &'static str, &str)> {
    let mut depth: usize = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 && i > 0 => match ch {
                '>' => {
                    let left = s[..i].trim_end();
                    let right = s[i + 1..].trim_start();
                    return (!right.is_empty()).then_some((left, " > ", right));
                }
                '+' => {
                    let left = s[..i].trim_end();
                    let right = s[i + 1..].trim_start();
                    return (!right.is_empty()).then_some((left, " + ", right));
                }
                '~' => {
                    let left = s[..i].trim_end();
                    let right = s[i + 1..].trim_start();
                    return (!right.is_empty()).then_some((left, " ~ ", right));
                }
                c if c.is_whitespace() => {
                    let left = s[..i].trim_end();
                    let mut j = i;
                    while let Some(cc) = s[j..].chars().next() {
                        if cc.is_whitespace() {
                            j += cc.len_utf8();
                        } else {
                            break;
                        }
                    }
                    let right = s[j..].trim_start();
                    return (!right.is_empty()).then_some((left, " ", right));
                }
                _ => {}
            },
            _ => {}
        }
    }
    None
}

/// Insert `attr` into `selector` immediately before the first `:` that is
/// outside of parentheses (i.e., before a pseudo-class or pseudo-element).
/// If no such colon exists, `attr` is appended at the end.
///
/// Examples:
/// * `div:first-child` + `[c]` → `div[c]:first-child`
/// * `.foo.bar`         + `[c]` → `.foo.bar[c]`
/// * `input::placeholder` + `[c]` → `input[c]::placeholder`
fn insert_before_pseudo(selector: &str, attr: &str) -> String {
    let mut depth: usize = 0;
    for (i, ch) in selector.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ':' if depth == 0 => {
                return format!("{}{}{}", &selector[..i], attr, &selector[i..]);
            }
            _ => {}
        }
    }
    format!("{}{}", selector, attr)
}

/// Find the index of the `}` that closes the first `{` in `s`.
fn find_block_end(s: &str) -> Option<usize> {
    let mut depth: usize = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scoper() -> CssScoper<'static> {
        CssScoper::new("_leptoshost-abc", "_leptoscontent-abc")
    }

    #[test]
    fn host_hover_compound() {
        let out = scoper().scope(":host:hover { color: blue; }");
        assert_eq!(out.trim(), "[_leptoshost-abc]:hover { color: blue; }");
    }

    #[test]
    fn host_before_compound() {
        let out = scoper().scope(":host::before { content: ''; }");
        assert_eq!(out.trim(), "[_leptoshost-abc]::before { content: ''; }");
    }

    #[test]
    fn host_hover_descendant_scopes_tail() {
        let out = scoper().scope(":host:hover .label { opacity: 1; }");
        assert_eq!(
            out.trim(),
            "[_leptoshost-abc]:hover .label[_leptoscontent-abc] { opacity: 1; }"
        );
    }

    #[test]
    fn host_hover_child_combinator() {
        let out = scoper().scope(":host:hover>.track { height: 2px; }");
        assert_eq!(
            out.trim(),
            "[_leptoshost-abc]:hover > .track[_leptoscontent-abc] { height: 2px; }"
        );
    }

    #[test]
    fn plain_element() {
        let out = scoper().scope("div { color: red; }");
        assert_eq!(out.trim(), "div[_leptoscontent-abc] { color: red; }");
    }

    #[test]
    fn pseudo_class() {
        let out = scoper().scope("a:hover { color: blue; }");
        assert_eq!(out.trim(), "a[_leptoscontent-abc]:hover { color: blue; }");
    }

    #[test]
    fn pseudo_element() {
        let out = scoper().scope("input::placeholder { color: grey; }");
        assert_eq!(out.trim(), "input[_leptoscontent-abc]::placeholder { color: grey; }");
    }

    #[test]
    fn host_bare() {
        let out = scoper().scope(":host { display: block; }");
        assert_eq!(out.trim(), "[_leptoshost-abc] { display: block; }");
    }

    #[test]
    fn host_with_class() {
        let out = scoper().scope(":host(.active) { color: green; }");
        assert_eq!(out.trim(), "[_leptoshost-abc].active { color: green; }");
    }

    #[test]
    fn host_descendant() {
        let out = scoper().scope(":host .child { font-size: 1rem; }");
        assert_eq!(
            out.trim(),
            "[_leptoshost-abc] .child[_leptoscontent-abc] { font-size: 1rem; }"
        );
    }

    #[test]
    fn host_child_combinator() {
        let out = scoper().scope(":host>span { margin: 0; }");
        assert_eq!(out.trim(), "[_leptoshost-abc]>span[_leptoscontent-abc] { margin: 0; }");
    }

    #[test]
    fn media_query_recursion() {
        let css = "@media (max-width: 600px) { div { color: red; } }";
        let out = scoper().scope(css);
        assert!(out.contains("@media (max-width: 600px)"));
        assert!(out.contains("div[_leptoscontent-abc]"));
    }

    #[test]
    fn comma_separated_selectors() {
        let out = scoper().scope("h1, h2 { font-weight: bold; }");
        assert!(out.contains("h1[_leptoscontent-abc]"));
        assert!(out.contains("h2[_leptoscontent-abc]"));
    }

    #[test]
    fn root_selector_unscoped() {
        let out = scoper().scope(":root { --x: 1; }");
        assert_eq!(out.trim(), ":root { --x: 1; }");
    }

    #[test]
    fn root_compound_and_descendant_unscoped() {
        let out = scoper().scope(":root.dark { color: red; }");
        assert_eq!(out.trim(), ":root.dark { color: red; }");

        let out = scoper().scope(":root .theme { --y: 2; }");
        assert_eq!(out.trim(), ":root .theme { --y: 2; }");
    }

    #[test]
    fn root_with_sibling_selector_still_unscoped() {
        let out = scoper().scope(":root, .panel { margin: 0; }");
        assert!(out.contains(":root"));
        assert!(out.contains(".panel[_leptoscontent-abc]"));
    }

    #[test]
    fn skip_scope_removes_wrapper() {
        let out = scoper().scope(":skip-scope(.pul-portal) { z-index: 9; }");
        assert_eq!(out.trim(), ".pul-portal { z-index: 9; }");
    }

    #[test]
    fn skip_scope_comma_inside_one_selector() {
        let out =
            scoper().scope(":skip-scope(.a, .b:hover) { color: red; }");
        assert_eq!(out.trim(), ".a, .b:hover { color: red; }");
    }

    #[test]
    fn skip_scope_mixed_with_scoped_selector() {
        let out = scoper().scope(":skip-scope(.global), div { display: block; }");
        assert_eq!(
            out.trim(),
            ".global, div[_leptoscontent-abc] { display: block; }"
        );
    }

    #[test]
    fn comma_inside_is_not_selector_list_split() {
        let out = scoper().scope(":is(.a, .b) { margin: 0; }");
        assert_eq!(
            out.trim(),
            "[_leptoscontent-abc]:is(.a, .b) { margin: 0; }"
        );
    }
}
