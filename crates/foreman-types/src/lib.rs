//! Shared basic types (scaffold)

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionInfo {
    pub name: &'static str,
    pub version: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_info_constructs() {
        let v = VersionInfo { name: "assistant-core", version: "0.0.1" };
        assert_eq!(v.name, "assistant-core");
    }
}

