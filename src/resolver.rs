use crate::index::Package;
use crate::version::RVersion;
use pubgrub::{Dependencies, DependencyConstraints, DependencyProvider, Ranges};
use std::collections::HashMap;
use std::convert::Infallible;

/// Wraps the CRAN index so it can be used as a pubgrub DependencyProvider.
struct CranProvider<'a> {
    index: &'a HashMap<String, Package>,
}

impl DependencyProvider for CranProvider<'_> {
    type P = String;
    type V = RVersion;
    type VS = Ranges<RVersion>;
    type M = String;
    type Priority = usize;
    type Err = Infallible;

    fn prioritize(
        &self,
        _package: &String,
        _range: &Ranges<RVersion>,
        _stats: &pubgrub::PackageResolutionStatistics,
    ) -> usize {
        0
    }

    fn choose_version(
        &self,
        package: &String,
        range: &Ranges<RVersion>,
    ) -> Result<Option<RVersion>, Infallible> {
        let Some(pkg) = self.index.get(package) else {
            return Ok(None);
        };
        let v = RVersion::parse(&pkg.version).unwrap_or_else(RVersion::minimum);
        Ok(range.contains(&v).then_some(v))
    }

    fn get_dependencies(
        &self,
        package: &String,
        _version: &RVersion,
    ) -> Result<Dependencies<String, Ranges<RVersion>, String>, Infallible> {
        let Some(pkg) = self.index.get(package) else {
            return Ok(Dependencies::Unavailable(format!(
                "{package} not found in CRAN index"
            )));
        };
        let mut deps: DependencyConstraints<String, Ranges<RVersion>> =
            DependencyConstraints::default();
        for dep in &pkg.deps {
            let range = dep
                .req
                .as_ref()
                .map(|r| r.to_range())
                .unwrap_or_else(Ranges::full);
            deps.insert(dep.name.clone(), range);
        }
        Ok(Dependencies::Available(deps))
    }
}

/// Resolves all transitive dependencies of `root` and returns a map of
/// package name → resolved version. Returns an error string if resolution fails.
pub fn resolve(
    root: &str,
    index: &HashMap<String, Package>,
) -> Result<HashMap<String, RVersion>, String> {
    let provider = CranProvider { index };
    let root_version = index
        .get(root)
        .and_then(|p| RVersion::parse(&p.version))
        .unwrap_or_else(RVersion::minimum);

    pubgrub::resolve(&provider, root.to_string(), root_version)
        .map(|fx_map| fx_map.into_iter().collect::<HashMap<_, _>>())
        .map_err(|e| format!("dependency resolution failed for {root}: {e}"))
}

/// Resolves all transitive dependencies for multiple root packages.
/// Returns a unified map of package name → resolved version.
pub fn resolve_all(
    roots: &[String],
    index: &HashMap<String, Package>,
) -> Result<HashMap<String, RVersion>, String> {
    let mut all: HashMap<String, RVersion> = HashMap::new();
    for root in roots {
        let resolved = resolve(root, index)?;
        for (name, version) in resolved {
            all.insert(name, version);
        }
    }
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::Dep;

    fn dep(name: &str) -> Dep {
        Dep::new(name.to_string(), None)
    }

    fn make_index() -> HashMap<String, Package> {
        let mut index = HashMap::new();
        index.insert(
            "ggplot2".to_string(),
            Package {
                version: "3.5.1".to_string(),
                deps: vec![dep("rlang"), dep("scales")],
            },
        );
        index.insert(
            "rlang".to_string(),
            Package {
                version: "1.1.4".to_string(),
                deps: vec![],
            },
        );
        index.insert(
            "scales".to_string(),
            Package {
                version: "1.3.0".to_string(),
                deps: vec![dep("rlang")],
            },
        );
        index
    }

    #[test]
    fn test_resolve_transitive_deps() {
        let index = make_index();
        let resolved = resolve("ggplot2", &index).unwrap();
        assert!(resolved.contains_key("rlang"));
        assert!(resolved.contains_key("scales"));
    }

    #[test]
    fn test_resolve_deduplicates() {
        // rlang is a dep of both ggplot2 and scales — should only appear once
        let index = make_index();
        let resolved = resolve("ggplot2", &index).unwrap();
        assert_eq!(resolved.keys().filter(|k| *k == "rlang").count(), 1);
    }

    #[test]
    fn test_resolve_includes_root() {
        // pubgrub returns the root package itself in the solution
        let index = make_index();
        let resolved = resolve("ggplot2", &index).unwrap();
        assert!(resolved.contains_key("ggplot2"));
    }

    #[test]
    fn test_resolve_unknown_package_returns_error() {
        let index = make_index();
        assert!(resolve("nonexistent", &index).is_err());
    }

    #[test]
    fn test_resolve_all_unions_results() {
        let index = make_index();
        let roots = vec!["ggplot2".to_string(), "scales".to_string()];
        let all = resolve_all(&roots, &index).unwrap();
        assert!(all.contains_key("ggplot2"));
        assert!(all.contains_key("scales"));
        assert!(all.contains_key("rlang"));
    }

    #[test]
    fn test_resolve_version_constraint_satisfied() {
        use crate::version::{Op, VersionReq};
        let mut index = make_index();
        // scales requires rlang >= 1.0.0 — 1.1.4 satisfies this
        index.get_mut("scales").unwrap().deps = vec![Dep::new(
            "rlang".to_string(),
            Some(VersionReq {
                op: Op::Gte,
                version: RVersion::parse("1.0.0").unwrap(),
            }),
        )];
        let resolved = resolve("scales", &index).unwrap();
        assert_eq!(resolved["rlang"], RVersion::parse("1.1.4").unwrap());
    }

    #[test]
    fn test_resolve_version_constraint_unsatisfied() {
        use crate::version::{Op, VersionReq};
        let mut index = make_index();
        // scales requires rlang >= 99.0 — 1.1.4 does NOT satisfy this
        index.get_mut("scales").unwrap().deps = vec![Dep::new(
            "rlang".to_string(),
            Some(VersionReq {
                op: Op::Gte,
                version: RVersion::parse("99.0").unwrap(),
            }),
        )];
        assert!(resolve("scales", &index).is_err());
    }

    // --- version conflict and diamond dependency tests ---

    fn constrained(name: &str, op: crate::version::Op, version: &str) -> Dep {
        Dep::new(
            name.to_string(),
            Some(crate::version::VersionReq {
                op,
                version: RVersion::parse(version).unwrap(),
            }),
        )
    }

    /// Build an index where two packages (pkg_a, pkg_b) share a common dep,
    /// each with their own constraint on it. Useful for diamond scenarios.
    fn diamond_index(dep_a: Dep, dep_b: Dep, common_version: &str) -> HashMap<String, Package> {
        let mut index = HashMap::new();
        index.insert(
            "root".to_string(),
            Package {
                version: "1.0".to_string(),
                deps: vec![dep("pkg_a"), dep("pkg_b")],
            },
        );
        index.insert(
            "pkg_a".to_string(),
            Package {
                version: "1.0".to_string(),
                deps: vec![dep_a],
            },
        );
        index.insert(
            "pkg_b".to_string(),
            Package {
                version: "1.0".to_string(),
                deps: vec![dep_b],
            },
        );
        index.insert(
            "common".to_string(),
            Package {
                version: common_version.to_string(),
                deps: vec![],
            },
        );
        index
    }

    #[test]
    fn test_diamond_compatible_constraints() {
        use crate::version::Op;
        // pkg_a needs common >= 1.0, pkg_b needs common >= 1.5
        // available is 2.0 — satisfies both
        let index = diamond_index(
            constrained("common", Op::Gte, "1.0"),
            constrained("common", Op::Gte, "1.5"),
            "2.0",
        );
        let resolved = resolve("root", &index).unwrap();
        assert!(resolved.contains_key("common"));
        assert_eq!(resolved["common"], RVersion::parse("2.0").unwrap());
    }

    #[test]
    fn test_diamond_conflicting_constraints() {
        use crate::version::Op;
        // pkg_a needs common >= 2.0, pkg_b needs common < 2.0
        // available is 2.0 — fails the < 2.0 constraint
        let index = diamond_index(
            constrained("common", Op::Gte, "2.0"),
            constrained("common", Op::Lt, "2.0"),
            "2.0",
        );
        assert!(resolve("root", &index).is_err());
    }

    #[test]
    fn test_transitive_conflict_propagates() {
        use crate::version::Op;
        // root -> pkg_a -> common >= 99.0, but common is at 1.0
        let mut index = HashMap::new();
        index.insert(
            "root".to_string(),
            Package {
                version: "1.0".to_string(),
                deps: vec![dep("pkg_a")],
            },
        );
        index.insert(
            "pkg_a".to_string(),
            Package {
                version: "1.0".to_string(),
                deps: vec![constrained("common", Op::Gte, "99.0")],
            },
        );
        index.insert(
            "common".to_string(),
            Package {
                version: "1.0".to_string(),
                deps: vec![],
            },
        );
        assert!(resolve("root", &index).is_err());
    }

    #[test]
    fn test_exact_version_match_passes() {
        use crate::version::Op;
        let mut index = make_index();
        // require exactly rlang 1.1.4 — that's what's available
        index.get_mut("scales").unwrap().deps = vec![constrained("rlang", Op::Eq, "1.1.4")];
        assert!(resolve("scales", &index).is_ok());
    }

    #[test]
    fn test_exact_version_match_fails() {
        use crate::version::Op;
        let mut index = make_index();
        // require exactly rlang 1.1.3 — 1.1.4 is available, not 1.1.3
        index.get_mut("scales").unwrap().deps = vec![constrained("rlang", Op::Eq, "1.1.3")];
        assert!(resolve("scales", &index).is_err());
    }
}
