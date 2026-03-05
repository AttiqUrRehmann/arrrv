use rayon::prelude::*;
use std::collections::HashMap;

const CRANDB_BASE: &str = "https://crandb.r-pkg.org";

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
