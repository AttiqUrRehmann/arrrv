use crate::index::Package;
use crate::version::{Dep, RVersion};
use pubgrub::{
    DefaultStringReporter, Dependencies, DependencyConstraints, DependencyProvider, Ranges,
    Reporter,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::Infallible;

/// Abstracts over the remote package database so it can be swapped out in tests.
trait RemoteIndex {
    /// All known versions of `package`, sorted ascending.
    fn available_versions(&self, package: &str) -> Vec<RVersion>;
    /// Dependencies of a specific `version` of `package`, or `None` on failure.
    fn package_deps(&self, package: &str, version: &str) -> Option<Vec<Dep>>;
}

/// Production implementation that talks to crandb.r-pkg.org.
struct CranDb;

impl RemoteIndex for CranDb {
    fn available_versions(&self, package: &str) -> Vec<RVersion> {
        crate::crandb::fetch_available_versions(package)
    }
    fn package_deps(&self, package: &str, version: &str) -> Option<Vec<Dep>> {
        crate::crandb::fetch_package_deps(package, version)
    }
}

/// Wraps the CRAN index so it can be used as a pubgrub DependencyProvider.
struct CranProvider<'a, R: RemoteIndex> {
    index: &'a HashMap<String, Package>,
    remote: R,
    /// Per-package version lists, cached to avoid repeated requests during backtracking.
    version_cache: RefCell<HashMap<String, Vec<RVersion>>>,
}

impl<R: RemoteIndex> DependencyProvider for CranProvider<'_, R> {
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
        if range.contains(&v) {
            return Ok(Some(v));
        }
        // The latest CRAN version is not in the range (e.g. a pinned old version).
        // Fetch all known versions from crandb and pick the highest one that fits.
        // Results are cached so repeated calls during backtracking are free.
        let versions = self
            .version_cache
            .borrow_mut()
            .entry(package.clone())
            .or_insert_with(|| self.remote.available_versions(package))
            .clone();
        Ok(versions.into_iter().rev().find(|v| range.contains(v)))
    }

    fn get_dependencies(
        &self,
        package: &String,
        version: &RVersion,
    ) -> Result<Dependencies<String, Ranges<RVersion>, String>, Infallible> {
        let Some(pkg) = self.index.get(package) else {
            return Ok(Dependencies::Unavailable(format!(
                "{package} not found in CRAN index"
            )));
        };
        // When resolving an old exact pin, fetch the real deps for that
        // version from crandb rather than using the current version's deps.
        let index_version = RVersion::parse(&pkg.version).unwrap_or_else(RVersion::minimum);
        let historical = if *version != index_version {
            self.remote.package_deps(package, &version.to_string())
        } else {
            None
        };
        let dep_list = historical.as_deref().unwrap_or(&pkg.deps);
        let mut deps: DependencyConstraints<String, Ranges<RVersion>> =
            DependencyConstraints::default();
        for dep in dep_list {
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

/// Formats the raw PubGrub no-solution report into a numbered, indented chain.
/// Each "Because …" clause gets its own step, and each step is indented two
/// more spaces than the previous one to make the dependency chain visually clear.
/// The internal synthetic package name is replaced with "your project".
fn format_no_solution(report: &str) -> String {
    let mut out = String::new();
    let mut step = 0usize;
    for line in report.lines() {
        if line.is_empty() {
            continue;
        }
        let line = line.replace("__root__ 0", "your project");
        let indent = "  ".repeat(step);
        out.push_str(&format!("{}{}. {}\n", indent, step + 1, line));
        step += 1;
    }
    out.trim_end().to_string()
}

/// Resolves all transitive dependencies of `root` and returns a map of
/// package name → resolved version. Returns an error string if resolution fails.
/// When `verbose` is true and there is no solution, the error includes the full
/// PubGrub derivation tree explaining which constraints are incompatible.
pub fn resolve(
    root: &str,
    index: &HashMap<String, Package>,
    verbose: bool,
) -> Result<HashMap<String, RVersion>, String> {
    let provider = CranProvider {
        index,
        remote: CranDb,
        version_cache: RefCell::new(HashMap::new()),
    };
    let root_version = index
        .get(root)
        .and_then(|p| RVersion::parse(&p.version))
        .unwrap_or_else(RVersion::minimum);

    pubgrub::resolve(&provider, root.to_string(), root_version)
        .map(|fx_map| fx_map.into_iter().collect::<HashMap<_, _>>())
        .map_err(|e| {
            if verbose && let pubgrub::PubGrubError::NoSolution(tree) = &e {
                let report = DefaultStringReporter::report(tree);
                return format!(
                    "dependency resolution failed:\n{}",
                    format_no_solution(&report)
                );
            }
            format!("dependency resolution failed: {e}")
        })
}

/// Resolves all transitive dependencies for multiple root packages, each with
/// an optional version constraint from ruv.toml. Uses a synthetic root package
/// so that all constraints are fed into a single PubGrub resolution pass.
pub fn resolve_all(
    roots: &[crate::version::Dep],
    index: &HashMap<String, Package>,
    verbose: bool,
) -> Result<HashMap<String, RVersion>, String> {
    // Build a synthetic "__root__" package whose deps are the user's requirements.
    // This lets PubGrub enforce all root constraints in one pass.
    let synthetic_root = "__root__".to_string();
    let mut augmented = index.clone();
    augmented.insert(
        synthetic_root.clone(),
        Package {
            version: "0".to_string(),
            deps: roots.to_vec(),
        },
    );

    let mut resolved = resolve(&synthetic_root, &augmented, verbose)?;
    resolved.remove(&synthetic_root);
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::Dep;

    // ---------------------------------------------------------------------------
    // Mock remote index — lets tests control version lists and historical deps
    // without making real HTTP calls.
    // ---------------------------------------------------------------------------

    #[derive(Default)]
    struct MockRemote {
        /// package name → sorted list of versions it "has"
        versions: HashMap<String, Vec<RVersion>>,
        /// (package, version) → deps returned for that specific version
        deps: HashMap<(String, String), Vec<Dep>>,
    }

    impl MockRemote {
        fn with_versions(mut self, pkg: &str, vs: &[&str]) -> Self {
            let mut parsed: Vec<RVersion> = vs.iter().filter_map(|s| RVersion::parse(s)).collect();
            parsed.sort();
            self.versions.insert(pkg.to_string(), parsed);
            self
        }

        fn with_deps(mut self, pkg: &str, version: &str, deps: Vec<Dep>) -> Self {
            self.deps
                .insert((pkg.to_string(), version.to_string()), deps);
            self
        }
    }

    impl RemoteIndex for MockRemote {
        fn available_versions(&self, package: &str) -> Vec<RVersion> {
            self.versions.get(package).cloned().unwrap_or_default()
        }
        fn package_deps(&self, package: &str, version: &str) -> Option<Vec<Dep>> {
            self.deps
                .get(&(package.to_string(), version.to_string()))
                .cloned()
        }
    }

    /// Like `resolve` but uses the provided mock instead of crandb.
    fn resolve_mock(
        root: &str,
        index: &HashMap<String, Package>,
        mock: MockRemote,
        verbose: bool,
    ) -> Result<HashMap<String, RVersion>, String> {
        let provider = CranProvider {
            index,
            remote: mock,
            version_cache: RefCell::new(HashMap::new()),
        };
        let root_version = index
            .get(root)
            .and_then(|p| RVersion::parse(&p.version))
            .unwrap_or_else(RVersion::minimum);
        pubgrub::resolve(&provider, root.to_string(), root_version)
            .map(|m| m.into_iter().collect())
            .map_err(|e| {
                if verbose && let pubgrub::PubGrubError::NoSolution(tree) = &e {
                    let report = DefaultStringReporter::report(tree);
                    return format!(
                        "dependency resolution failed:\n{}",
                        format_no_solution(&report)
                    );
                }
                format!("dependency resolution failed: {e}")
            })
    }

    // ---------------------------------------------------------------------------

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
        let resolved = resolve("ggplot2", &index, false).unwrap();
        assert!(resolved.contains_key("rlang"));
        assert!(resolved.contains_key("scales"));
    }

    #[test]
    fn test_resolve_deduplicates() {
        // rlang is a dep of both ggplot2 and scales — should only appear once
        let index = make_index();
        let resolved = resolve("ggplot2", &index, false).unwrap();
        assert_eq!(resolved.keys().filter(|k| *k == "rlang").count(), 1);
    }

    #[test]
    fn test_resolve_includes_root() {
        // pubgrub returns the root package itself in the solution
        let index = make_index();
        let resolved = resolve("ggplot2", &index, false).unwrap();
        assert!(resolved.contains_key("ggplot2"));
    }

    #[test]
    fn test_resolve_unknown_package_returns_error() {
        let index = make_index();
        assert!(resolve("nonexistent", &index, false).is_err());
    }

    #[test]
    fn test_resolve_all_unions_results() {
        let index = make_index();
        let roots = vec![dep("ggplot2"), dep("scales")];
        let all = resolve_all(&roots, &index, false).unwrap();
        assert!(all.contains_key("ggplot2"));
        assert!(all.contains_key("scales"));
        assert!(all.contains_key("rlang"));
    }

    #[test]
    fn test_resolve_all_enforces_root_constraints() {
        use crate::version::Op;
        let index = make_index();
        // user pins rlang >= 99.0 in ruv.toml — should fail at the root level
        let roots = vec![constrained("rlang", Op::Gte, "99.0")];
        assert!(resolve_all(&roots, &index, false).is_err());
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
        let resolved = resolve("scales", &index, false).unwrap();
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
        assert!(resolve("scales", &index, false).is_err());
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
        let resolved = resolve("root", &index, false).unwrap();
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
        assert!(resolve("root", &index, false).is_err());
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
        assert!(resolve("root", &index, false).is_err());
    }

    #[test]
    fn test_exact_version_match_passes() {
        use crate::version::Op;
        let mut index = make_index();
        // require exactly rlang 1.1.4 — that's what's available
        index.get_mut("scales").unwrap().deps = vec![constrained("rlang", Op::Eq, "1.1.4")];
        assert!(resolve("scales", &index, false).is_ok());
    }

    #[test]
    fn test_exact_version_pin_in_index_resolves() {
        use crate::version::Op;
        let mut index = make_index();
        // Pin the version that IS in the fake index — no crandb call needed.
        // Pinning an older version that requires crandb is covered by running
        // `ruv lock` against a real ruv.toml.
        index.get_mut("scales").unwrap().deps = vec![constrained("rlang", Op::Eq, "1.1.4")];
        let resolved = resolve("scales", &index, false).unwrap();
        assert_eq!(resolved["rlang"], RVersion::parse("1.1.4").unwrap());
    }

    // ---------------------------------------------------------------------------
    // Version-constraint coverage using MockRemote
    // Each test exercises one row of the choose_version logic:
    //   1. >= (latest satisfies)          — no remote call needed
    //   2. == (exact old pin)             — remote returns that exact version
    //   3. <= (upper bound old pin)       — remote returns highest ≤ bound
    //   4. < (exclusive upper bound)      — remote returns highest < bound
    //   5. conflict (no version fits)     — resolution must fail
    // ---------------------------------------------------------------------------

    // Case 1: >= satisfied by the index version — remote is never consulted.
    #[test]
    fn test_gte_satisfied_by_index_version() {
        use crate::version::Op;
        let mut index = make_index();
        index.get_mut("scales").unwrap().deps = vec![constrained("rlang", Op::Gte, "1.0.0")];
        let resolved = resolve_mock("scales", &index, MockRemote::default(), false).unwrap();
        assert_eq!(resolved["rlang"], RVersion::parse("1.1.4").unwrap());
    }

    // Case 2: == older version — remote supplies the exact version and its deps.
    #[test]
    fn test_eq_pin_resolved_via_mock() {
        use crate::version::Op;
        let mut index = make_index();
        index.get_mut("scales").unwrap().deps = vec![constrained("rlang", Op::Eq, "1.1.3")];
        let mock = MockRemote::default()
            .with_versions("rlang", &["1.1.0", "1.1.1", "1.1.2", "1.1.3", "1.1.4"])
            .with_deps("rlang", "1.1.3", vec![]);
        let resolved = resolve_mock("scales", &index, mock, false).unwrap();
        assert_eq!(resolved["rlang"], RVersion::parse("1.1.3").unwrap());
    }

    // Case 3: <= — remote picks the highest version at or below the bound.
    #[test]
    fn test_lte_pin_picks_upper_bound_via_mock() {
        use crate::version::Op;
        let mut index = make_index();
        index.get_mut("scales").unwrap().deps = vec![constrained("rlang", Op::Lte, "1.1.2")];
        let mock = MockRemote::default()
            .with_versions("rlang", &["1.1.0", "1.1.1", "1.1.2", "1.1.3", "1.1.4"])
            .with_deps("rlang", "1.1.2", vec![]);
        let resolved = resolve_mock("scales", &index, mock, false).unwrap();
        assert_eq!(resolved["rlang"], RVersion::parse("1.1.2").unwrap());
    }

    // Case 4: < — remote picks the highest version strictly below the bound.
    #[test]
    fn test_lt_pin_picks_best_below_bound_via_mock() {
        use crate::version::Op;
        let mut index = make_index();
        index.get_mut("scales").unwrap().deps = vec![constrained("rlang", Op::Lt, "1.1.4")];
        let mock = MockRemote::default()
            .with_versions("rlang", &["1.1.0", "1.1.1", "1.1.2", "1.1.3", "1.1.4"])
            .with_deps("rlang", "1.1.3", vec![]);
        let resolved = resolve_mock("scales", &index, mock, false).unwrap();
        assert_eq!(resolved["rlang"], RVersion::parse("1.1.3").unwrap());
    }

    // Case 5: no version satisfies — resolution must fail.
    #[test]
    fn test_no_satisfying_version_fails() {
        use crate::version::Op;
        let mut index = make_index();
        // Require rlang == 0.9.0; remote doesn't have it.
        index.get_mut("scales").unwrap().deps = vec![constrained("rlang", Op::Eq, "0.9.0")];
        let mock = MockRemote::default()
            .with_versions("rlang", &["1.1.0", "1.1.1", "1.1.2", "1.1.3", "1.1.4"]);
        assert!(resolve_mock("scales", &index, mock, false).is_err());
    }

    // Bonus: conflicting pins from two different packages — both require rlang
    // at incompatible versions simultaneously.
    #[test]
    fn test_conflicting_pins_fail() {
        use crate::version::Op;
        let mut index = make_index();
        // pkg_a needs rlang >= 1.1.4, pkg_b needs rlang <= 1.1.2 — impossible.
        index.get_mut("ggplot2").unwrap().deps =
            vec![constrained("rlang", Op::Gte, "1.1.4"), dep("scales")];
        index.get_mut("scales").unwrap().deps = vec![constrained("rlang", Op::Lte, "1.1.2")];
        let mock = MockRemote::default()
            .with_versions("rlang", &["1.1.0", "1.1.1", "1.1.2", "1.1.3", "1.1.4"]);
        assert!(resolve_mock("ggplot2", &index, mock, false).is_err());
    }
}
