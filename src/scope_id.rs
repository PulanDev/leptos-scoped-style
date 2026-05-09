/// A stable, unique identifier for a component's CSS scope.
///
/// Derived from the component name via a djb2 hash. Two components with
/// the same name will always produce the same `ScopeId`, so names must be
/// unique within an application.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScopeId(String);

impl ScopeId {
    /// Derive a `ScopeId` from a component name.
    ///
    /// Uses the djb2 hash algorithm to produce a short, stable hex string.
    pub fn from_name(name: &str) -> Self {
        Self(djb2(name))
    }

    /// The HTML attribute name applied to the component's host element.
    ///
    /// Format: `_leptoshost-{hash}`
    pub fn host_attr(&self) -> String {
        format!("_leptoshost-{}", self.0)
    }

    /// The HTML attribute name applied to every element inside the component.
    ///
    /// Format: `_leptoscontent-{hash}`
    pub fn content_attr(&self) -> String {
        format!("_leptoscontent-{}", self.0)
    }

    /// The raw hex hash string, e.g. `"c1a2b3c4"`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ScopeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// djb2 hash: fast, deterministic, and suitable for short strings like
/// component names.  Returns an 8-character lowercase hex string.
fn djb2(s: &str) -> String {
    let mut h: u32 = 5381;
    for b in s.bytes() {
        h = h.wrapping_mul(33).wrapping_add(u32::from(b));
    }
    format!("{h:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_name_same_id() {
        assert_eq!(ScopeId::from_name("my-button"), ScopeId::from_name("my-button"));
    }

    #[test]
    fn different_names_different_ids() {
        assert_ne!(ScopeId::from_name("my-button"), ScopeId::from_name("my-card"));
    }

    #[test]
    fn attr_format() {
        let id = ScopeId::from_name("my-button");
        assert!(id.host_attr().starts_with("_leptoshost-"));
        assert!(id.content_attr().starts_with("_leptoscontent-"));
    }
}
