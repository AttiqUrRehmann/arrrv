use rayon::prelude::*;
use std::collections::HashMap;

const CRANDB_BASE: &str = "https://crandb.r-pkg.org";

// Packages that ship with R and must not be treated as CRAN dependencies.
// Kept in sync with the list in index.rs.
const BASE_PACKAGES: &[&str] = &[
    "R",
    "base",
    "compiler",
    "datasets",
    "grDevices",
    "graphics",
    "grid",
    "methods",
    "parallel",
    "splines",
    "stats",
    "stats4",
    "tcltk",
    "tools",
    "translations",
    "utils",
];

/// Queries crandb for the upload date of a specific package version.
/// Returns a date string like "2024-06-05", or None on failure.
fn fetch_upload_date(name: &str, version: &str) -> Option<String> {
    let url = format!("{}/{}/{}", CRANDB_BASE, name, version);
    let response = reqwest::blocking::get(&url).ok()?;
    if !response.status().is_success() {
        return None;
    }
    let json: serde_json::Value = response.json().ok()?;
    // crandb returns "Date/Publication": "2024-06-05 07:30:02 UTC"
    let date_str = json["Date/Publication"].as_str()?;
    Some(date_str[..10].to_string()) // take just "YYYY-MM-DD"
}

/// Fetches all known versions of a package from crandb, sorted ascending.
/// Returns an empty vec if the request fails.
pub fn fetch_available_versions(name: &str) -> Vec<crate::version::RVersion> {
    let url = format!("{}/{}/all", CRANDB_BASE, name);
    let Ok(response) = reqwest::blocking::get(&url) else {
        return vec![];
    };
    if !response.status().is_success() {
        return vec![];
    }
    let Ok(json) = response.json::<serde_json::Value>() else {
        return vec![];
    };
    // crandb /all returns {"versions": {"0.9.0": {...}, "1.0.0": {...}, ...}}
    // where the keys are version strings.
    let mut versions: Vec<crate::version::RVersion> = json["versions"]
        .as_object()
        .map(|obj| {
            obj.keys()
                .filter_map(|k| crate::version::RVersion::parse(k))
                .collect()
        })
        .unwrap_or_default();
    versions.sort();
    versions
}

/// Fetches the dependencies of a specific package version from crandb.
/// Returns None if the version can't be found or the request fails.
/// crandb returns Imports/Depends as objects mapping pkg name → constraint string
/// ("*" means unconstrained, ">= x.y.z" means that constraint).
pub fn fetch_package_deps(name: &str, version: &str) -> Option<Vec<crate::version::Dep>> {
    let url = format!("{}/{}/{}", CRANDB_BASE, name, version);
    let response = reqwest::blocking::get(&url).ok()?;
    if !response.status().is_success() {
        return None;
    }
    let json: serde_json::Value = response.json().ok()?;

    let mut deps = Vec::new();
    for field in &["Imports", "Depends"] {
        if let Some(obj) = json[field].as_object() {
            for (pkg_name, constraint) in obj {
                if BASE_PACKAGES.contains(&pkg_name.as_str()) {
                    continue;
                }
                let req = constraint
                    .as_str()
                    .filter(|s| *s != "*")
                    .and_then(crate::version::VersionReq::parse);
                deps.push(crate::version::Dep::new(pkg_name.clone(), req));
            }
        }
    }
    Some(deps)
}

/// For each (name, version) pair, fetches the CRAN upload date in parallel.
/// Returns a map of package name → date string (e.g. "2024-06-05").
/// Packages that fail the lookup fall back to today's date.
pub fn fetch_upload_dates(packages: &[(String, String)]) -> HashMap<String, String> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    packages
        .par_iter()
        .map(|(name, version)| {
            let date = fetch_upload_date(name, version).unwrap_or_else(|| today.clone());
            (name.clone(), date)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_upload_date_known_version() {
        // ggplot2 3.5.1 was published 2024-04-23
        let date = fetch_upload_date("ggplot2", "3.5.1");
        assert_eq!(date.as_deref(), Some("2024-04-23"));
    }

    #[test]
    fn test_fetch_upload_date_unknown_version() {
        let date = fetch_upload_date("ggplot2", "99.99.99");
        assert!(date.is_none());
    }
}
