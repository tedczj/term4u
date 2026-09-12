use std::io::Write;
use std::path::Path;

use super::imp::load_tab_configs;
use super::*;

#[cfg(feature = "local_fs")]
#[test]
fn test_default_tab_configs_dir_uses_underscores() {
    assert!(default_tab_configs_dir().ends_with("default_tab_configs"));
}

#[cfg(feature = "local_fs")]
#[test]
fn test_is_tab_config_toml_matches_user_tab_configs() {
    let path = tab_configs_dir().join("my_tab_config.toml");
    assert!(is_tab_config_toml(&path));
}

#[cfg(feature = "local_fs")]
#[test]
fn test_is_tab_config_toml_matches_default_tab_configs() {
    let path = default_tab_configs_dir().join("worktree.toml");
    assert!(is_tab_config_toml(&path));
}

#[cfg(feature = "local_fs")]
#[test]
fn test_is_tab_config_toml_rejects_non_toml_paths() {
    let path = tab_configs_dir().join("my_tab_config.yaml");
    assert!(!is_tab_config_toml(&path));
}

#[cfg(feature = "local_fs")]
#[test]
fn test_is_tab_config_toml_rejects_tomls_outside_tab_config_dirs() {
    let path = launch_configs_dir().join("workspace.toml");
    assert!(!is_tab_config_toml(&path));
}

fn write_tab_config_toml(dir: &Path, file_name: &str, config_name: &str) {
    let path = dir.join(file_name);
    let mut f = std::fs::File::create(path).unwrap();
    write!(f, "name = \"{}\"", config_name).unwrap();
}

#[cfg(feature = "local_fs")]
#[test]
fn test_load_tab_configs_sorts_case_insensitive() {
    let dir = tempfile::tempdir().unwrap();
    write_tab_config_toml(dir.path(), "zebra.toml", "Zebra");
    write_tab_config_toml(dir.path(), "alpha.toml", "alpha");
    write_tab_config_toml(dir.path(), "beta.toml", "Beta");

    let (configs, errors) = load_tab_configs(dir.path());

    assert!(errors.is_empty());
    let names: Vec<&str> = configs.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "Beta", "Zebra"]);
}

#[cfg(feature = "local_fs")]
#[test]
fn test_load_tab_configs_deterministic_tie_breaking() {
    let dir = tempfile::tempdir().unwrap();
    write_tab_config_toml(dir.path(), "upper.toml", "Alpha");
    write_tab_config_toml(dir.path(), "lower.toml", "alpha");

    let (configs, errors) = load_tab_configs(dir.path());

    assert!(errors.is_empty());
    let names: Vec<&str> = configs.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["Alpha", "alpha"]);
}

#[cfg(feature = "local_fs")]
#[test]
fn test_load_tab_configs_empty_directory() {
    let dir = tempfile::tempdir().unwrap();

    let (configs, errors) = load_tab_configs(dir.path());

    assert!(configs.is_empty());
    assert!(errors.is_empty());
}

#[cfg(feature = "local_fs")]
#[test]
fn test_load_tab_configs_skips_non_toml_files() {
    let dir = tempfile::tempdir().unwrap();
    write_tab_config_toml(dir.path(), "real.toml", "Real");
    std::fs::write(dir.path().join("readme.md"), "not a config").unwrap();
    std::fs::write(dir.path().join("data.json"), "{}").unwrap();

    let (configs, errors) = load_tab_configs(dir.path());

    assert!(errors.is_empty());
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].name, "Real");
}
