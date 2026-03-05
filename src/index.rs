use crate::cache::cache_dir;
use crate::version::Dep;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::time::Duration;

#[derive(Serialize, Deserialize)]
pub struct Package {
    pub version: String,
    pub deps: Vec<Dep>,
}

pub fn parse_packages(text: &str) -> HashMap<String, Package> {
    let mut index = HashMap::new();

    for block in text.split("\n\n") {
        // join continuation lines back onto the previous line
        let joined = block.lines().fold(String::new(), |mut acc, line| {
            if line.starts_with(' ') {
                acc.push(' ');
                acc.push_str(line.trim());
            } else {
                if !acc.is_empty() {
                    acc.push('\n');
                }
                acc.push_str(line)
            }
            acc
        });

        let mut name = None;
        let mut version = None;
        let mut deps: Vec<Dep> = Vec::new();

        for line in joined.lines() {
            if let Some((key, val)) = line.split_once(": ") {
                match key {
                    "Package" => name = Some(val.to_string()),
                    "Version" => version = Some(val.to_string()),
                    "Imports" | "Depends" => {
                        for raw in val.split(',') {
                            let raw = raw.trim();
                            // split "rlang (>= 1.1.0)" into name and optional constraint
                            let (dep_name, req) = if let Some((name, rest)) = raw.split_once('(') {
                                let req = rest.trim_end_matches(')').trim();
                                (
                                    name.trim().to_string(),
                                    crate::version::VersionReq::parse(req),
                                )
                            } else {
                                (raw.to_string(), None)
                            };
                            let base_packages = [
                                "R",
                                "base",
                                "utils",
                                "stats",
                                "graphics",
                                "grDevices",
                                "methods",
                                "datasets",
                                "tools",
                                "grid",
                                "compiler",
                            ];
                            if !base_packages.contains(&dep_name.as_str()) && !dep_name.is_empty() {
                                deps.push(Dep::new(dep_name, req));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if let (Some(name), Some(version)) = (name, version) {
            index.insert(name, Package { version, deps });
        }
    }

    index
}

fn is_fresh(path: &std::path::Path) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| t.elapsed().unwrap_or(Duration::MAX) < Duration::from_secs(86400))
        .unwrap_or(false)
}

pub fn fetch_cran_index() -> HashMap<String, Package> {
    let bin_path = cache_dir().join("index/packages.bin");
    let gz_path = cache_dir().join("index/PACKAGES.gz");
    // fast path: deserialise pre-parsed binary cache
    if is_fresh(&bin_path) {
        let bytes = std::fs::read(&bin_path).unwrap();
        if let Ok(index) = bincode::deserialize::<HashMap<String, Package>>(&bytes) {
            return index;
        }
    }

    // fetch or read cached gzip
    let gz_bytes = if is_fresh(&gz_path) {
        std::fs::read(&gz_path).unwrap()
    } else {
        println!("fetching CRAN package index...");
        let response =
            reqwest::blocking::get("https://cloud.r-project.org/src/contrib/PACKAGES.gz").unwrap();
        let bytes = response.bytes().unwrap().to_vec();
        std::fs::create_dir_all(gz_path.parent().unwrap()).unwrap();
        std::fs::write(&gz_path, &bytes).unwrap();
        bytes
    };

    // parse
    let mut decoder = GzDecoder::new(gz_bytes.as_slice());
    let mut text = String::new();
    decoder.read_to_string(&mut text).unwrap();
    let index = parse_packages(&text);

    // write binary cache for next run
    let encoded = bincode::serialize(&index).unwrap();
    std::fs::write(&bin_path, encoded).unwrap();

    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_package() {
        let text = "Package: ggplot2\nVersion: 3.5.1\nImports: rlang, scales\n";
        let index = parse_packages(text);
        let pkg = index.get("ggplot2").unwrap();
        assert_eq!(pkg.version, "3.5.1");
        assert!(pkg.deps.iter().any(|d| d.name == "rlang"));
        assert!(pkg.deps.iter().any(|d| d.name == "scales"));
    }

    #[test]
    fn test_parse_preserves_version_constraints() {
        use crate::version::{Op, RVersion};
        let text = "Package: foo\nVersion: 1.0\nImports: rlang (>= 1.1.0)\n";
        let index = parse_packages(text);
        let pkg = index.get("foo").unwrap();
        assert_eq!(pkg.deps.len(), 1);
        assert_eq!(pkg.deps[0].name, "rlang");
        let req = pkg.deps[0].req.as_ref().unwrap();
        assert!(matches!(req.op, Op::Gte));
        assert_eq!(req.version, RVersion::parse("1.1.0").unwrap());
    }

    #[test]
    fn test_parse_dep_without_constraint() {
        let text = "Package: foo\nVersion: 1.0\nImports: rlang\n";
        let index = parse_packages(text);
        let pkg = index.get("foo").unwrap();
        assert_eq!(pkg.deps[0].name, "rlang");
        assert!(pkg.deps[0].req.is_none());
    }

    #[test]
    fn test_parse_filters_base_packages() {
        let text = "Package: foo\nVersion: 1.0\nDepends: R (>= 4.0), methods, rlang\n";
        let index = parse_packages(text);
        let pkg = index.get("foo").unwrap();
        assert!(!pkg.deps.iter().any(|d| d.name == "R"));
        assert!(!pkg.deps.iter().any(|d| d.name == "methods"));
        assert!(pkg.deps.iter().any(|d| d.name == "rlang"));
    }

    #[test]
    fn test_parse_multiple_packages() {
        let text = "Package: foo\nVersion: 1.0\nImports: bar\n\nPackage: bar\nVersion: 2.0\n";
        let index = parse_packages(text);
        assert!(index.contains_key("foo"));
        assert!(index.contains_key("bar"));
    }
}
