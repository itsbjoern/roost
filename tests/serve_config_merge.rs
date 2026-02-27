//! Project overrides global on domain conflict.

mod common;

use roost::serve::config::{merge_configs, merge_configs_with_source, MappingSource, ServeConfig};
use std::fs;

#[test]
fn project_overrides_global_on_conflict() {
    let mut global = ServeConfig::default();
    global.add("api.test".into(), 5000);
    global.add("app.test".into(), 3000);

    let mut project = ServeConfig::default();
    project.add("api.test".into(), 5001); // same domain, different port

    let merged = merge_configs(&project, &global);
    assert_eq!(merged.get("api.test"), Some(&5001), "project overrides global");
    assert_eq!(merged.get("app.test"), Some(&3000), "global retained where no conflict");
}

#[test]
fn merge_with_source_shows_project_wins() {
    let mut global = ServeConfig::default();
    global.add("api.test".into(), 5000);

    let mut project = ServeConfig::default();
    project.add("api.test".into(), 5001);

    let merged = merge_configs_with_source(&project, &global);
    let api = merged.iter().find(|m| m.domain == "api.test").unwrap();
    assert_eq!(api.port, 5001);
    assert_eq!(api.source, MappingSource::Project);
}

#[test]
fn merge_with_source_shows_global_when_no_project() {
    let mut global = ServeConfig::default();
    global.add("api.test".into(), 5000);

    let project = ServeConfig::default();

    let merged = merge_configs_with_source(&project, &global);
    let api = merged.iter().find(|m| m.domain == "api.test").unwrap();
    assert_eq!(api.port, 5000);
    assert_eq!(api.source, MappingSource::Global);
}
