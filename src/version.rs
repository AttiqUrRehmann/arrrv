use serde::{Deserialize, Serialize};

/// An R package version: "1.1.0", "4.5", "2.23-26"
/// Stored as a list of numeric parts for comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RVersion {
    parts: Vec<u32>,
}

impl PartialEq for RVersion {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl Eq for RVersion {}

impl RVersion {
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Option<Vec<u32>> = s.split(['.', '-']).map(|p| p.parse().ok()).collect();
        parts.map(|p| RVersion { parts: p })
    }
}

impl PartialOrd for RVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let max_len = self.parts.len().max(other.parts.len());
        for i in 0..max_len {
            let a = self.parts.get(i).copied().unwrap_or(0);
            let b = other.parts.get(i).copied().unwrap_or(0);
            match a.cmp(&b) {
                std::cmp::Ordering::Equal => continue,
                ord => return ord,
            }
        }
        std::cmp::Ordering::Equal
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    Gte,
    Gt,
    Lte,
    Lt,
    Eq,
}

/// A version constraint: ">= 1.1.0"
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionReq {
    pub op: Op,
    pub version: RVersion,
}

impl VersionReq {
    /// Parse from the interior of parentheses, e.g. ">= 1.1.0"
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let (op, rest) = if let Some(r) = s.strip_prefix(">=") {
            (Op::Gte, r)
        } else if let Some(r) = s.strip_prefix('>') {
            (Op::Gt, r)
        } else if let Some(r) = s.strip_prefix("<=") {
            (Op::Lte, r)
        } else if let Some(r) = s.strip_prefix('<') {
            (Op::Lt, r)
        } else if let Some(r) = s.strip_prefix("==") {
            (Op::Eq, r)
        } else if let Some(r) = s.strip_prefix('=') {
            (Op::Eq, r)
        } else {
            return None;
        };
        let version = RVersion::parse(rest.trim())?;
        Some(VersionReq { op, version })
    }

    #[allow(dead_code)] // will be used by the resolver
    pub fn matches(&self, v: &RVersion) -> bool {
        match self.op {
            Op::Gte => v >= &self.version,
            Op::Gt => v > &self.version,
            Op::Lte => v <= &self.version,
            Op::Lt => v < &self.version,
            Op::Eq => v == &self.version,
        }
    }
}

/// A dependency: package name plus an optional version constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dep {
    pub name: String,
    pub req: Option<VersionReq>,
}

impl Dep {
    pub fn new(name: String, req: Option<VersionReq>) -> Self {
        Dep { name, req }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rversion_parse_three_parts() {
        let v = RVersion::parse("1.1.0").unwrap();
        assert_eq!(v.parts, vec![1, 1, 0]);
    }

    #[test]
    fn test_rversion_parse_two_parts() {
        let v = RVersion::parse("4.5").unwrap();
        assert_eq!(v.parts, vec![4, 5]);
    }

    #[test]
    fn test_rversion_parse_dash() {
        // base package style: "2.23-26"
        let v = RVersion::parse("2.23-26").unwrap();
        assert_eq!(v.parts, vec![2, 23, 26]);
    }

    #[test]
    fn test_rversion_ordering() {
        let v110 = RVersion::parse("1.1.0").unwrap();
        let v114 = RVersion::parse("1.1.4").unwrap();
        let v120 = RVersion::parse("1.2.0").unwrap();
        assert!(v110 < v114);
        assert!(v114 < v120);
        assert!(v120 > v110);
    }

    #[test]
    fn test_rversion_ordering_different_lengths() {
        // "1.1" should equal "1.1.0" (missing parts default to 0)
        let v11 = RVersion::parse("1.1").unwrap();
        let v110 = RVersion::parse("1.1.0").unwrap();
        assert_eq!(v11, v110);
    }

    #[test]
    fn test_versionreq_parse_gte() {
        let req = VersionReq::parse(">= 1.1.0").unwrap();
        assert!(matches!(req.op, Op::Gte));
        assert_eq!(req.version.parts, vec![1, 1, 0]);
    }

    #[test]
    fn test_versionreq_matches() {
        let req = VersionReq::parse(">= 1.1.0").unwrap();
        assert!(req.matches(&RVersion::parse("1.1.0").unwrap()));
        assert!(req.matches(&RVersion::parse("1.2.0").unwrap()));
        assert!(!req.matches(&RVersion::parse("1.0.9").unwrap()));
    }

    #[test]
    fn test_dep_parse_with_constraint() {
        let req = VersionReq::parse(">= 1.1.0").unwrap();
        let dep = Dep::new("rlang".to_string(), Some(req));
        assert_eq!(dep.name, "rlang");
        assert!(dep.req.is_some());
    }

    #[test]
    fn test_dep_parse_no_constraint() {
        let dep = Dep::new("scales".to_string(), None);
        assert_eq!(dep.name, "scales");
        assert!(dep.req.is_none());
    }
}
