use crate::version::Dep;
use serde::Deserialize;
use std::io::ErrorKind;
use std::path::Path;

pub const CONFIG_FILE: &str = "arrrv.toml";

#[derive(Deserialize)]
pub struct ArrrConfig {
    pub project: ProjectConfig,
}

#[derive(Deserialize)]
pub struct ProjectConfig {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub version: String,
    pub dependencies: Vec<String>,
}

pub fn read_config() -> Result<ArrrConfig, String> {
    let text = std::fs::read_to_string(CONFIG_FILE).map_err(|e| {
        if e.kind() == ErrorKind::NotFound {
            format!(
                "could not find {} in this directory — run `arrrv init` first",
                CONFIG_FILE
            )
        } else {
            format!("failed to read {}: {}", CONFIG_FILE, e)
        }
    })?;
    toml::from_str(&text).map_err(|e| format!("failed to parse {}: {}", CONFIG_FILE, e))
}

pub fn init_config(project_name: &str) -> Result<(), String> {
    if Path::new(CONFIG_FILE).exists() {
        return Err(format!(
            "{} already exists in this directory",
            CONFIG_FILE
        ));
    }

    let text = default_config_toml(project_name);
    std::fs::write(CONFIG_FILE, text).map_err(|e| format!("failed to write {}: {}", CONFIG_FILE, e))
}

fn default_config_toml(project_name: &str) -> String {
    format!(
        "[project]\nname = \"{}\"\nversion = \"0.1.0\"\nr-version = \">=4.3\"\ndependencies = []\n",
        project_name
    )
}

/// Parse a dependency string from arrrv.toml into a `Dep`.
/// Handles formats: "ggplot2", "ggplot2>=3.4", "rlang (>= 1.0)"
pub fn parse_dep(s: &str) -> Dep {
    let name: String = s
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '.' || *c == '-')
        .collect();
    // remainder after the name: strip whitespace and optional surrounding parens
    let rest = s[name.len()..]
        .trim()
        .trim_matches(|c| c == '(' || c == ')');
    let req = crate::version::VersionReq::parse(rest);
    Dep::new(name, req)
}

/// Strips version specifier from a dependency string: "ggplot2>=3.4" → "ggplot2"
/// Kept for callers that only need the name.
pub fn parse_dep_name(dep: &str) -> String {
    parse_dep(dep).name
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_config(text: &str) -> ArrrConfig {
        toml::from_str(text).expect("failed to parse arrrv.toml")
    }

    #[test]
    fn test_default_config_toml() {
        let toml = default_config_toml("my-project");
        assert!(toml.contains("name = \"my-project\""));
        assert!(toml.contains("dependencies = []"));
    }

    #[test]
    fn test_read_config_parses_dependencies() {
        let toml = "[project]\nname = \"test\"\nversion = \"0.1.0\"\ndependencies = [\"ggplot2\", \"dplyr\"]";
        let config = parse_config(toml);
        assert_eq!(config.project.dependencies, vec!["ggplot2", "dplyr"]);
    }

    #[test]
    fn test_read_config_empty_dependencies() {
        let toml = "[project]\nname = \"test\"\nversion = \"0.1.0\"\ndependencies = []";
        let config = parse_config(toml);
        assert!(config.project.dependencies.is_empty());
    }

    #[test]
    fn test_parse_dep_name_with_gte() {
        assert_eq!(parse_dep_name("ggplot2>=3.4"), "ggplot2");
    }

    #[test]
    fn test_parse_dep_name_no_version() {
        assert_eq!(parse_dep_name("dplyr"), "dplyr");
    }

    #[test]
    fn test_parse_dep_name_with_spaces() {
        assert_eq!(parse_dep_name("rlang (>= 1.0)"), "rlang");
    }

    #[test]
    fn test_parse_dep_name_preserves_dots_and_dashes() {
        assert_eq!(parse_dep_name("data.table"), "data.table");
        assert_eq!(parse_dep_name("R6"), "R6");
    }

    #[test]
    fn test_parse_dep_no_constraint() {
        use crate::version::Op;
        let d = parse_dep("dplyr");
        assert_eq!(d.name, "dplyr");
        assert!(d.req.is_none());

        let d2 = parse_dep("ggplot2>=3.4");
        assert_eq!(d2.name, "ggplot2");
        let req = d2.req.unwrap();
        assert!(matches!(req.op, Op::Gte));
        assert_eq!(req.version, crate::version::RVersion::parse("3.4").unwrap());
    }

    #[test]
    fn test_parse_dep_space_constraint() {
        use crate::version::Op;
        let d = parse_dep("rlang (>= 1.0)");
        assert_eq!(d.name, "rlang");
        let req = d.req.unwrap();
        assert!(matches!(req.op, Op::Gte));
        assert_eq!(req.version, crate::version::RVersion::parse("1.0").unwrap());
    }
}
