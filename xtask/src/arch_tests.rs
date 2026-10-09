use super::*;

fn metadata(json: &str) -> Metadata {
    serde_json::from_str(json).unwrap()
}

fn config() -> ArchConfig {
    toml::from_str(
        r#"
        [defaults]
        http_crates = ["axum", "reqwest"]
        [crates.types]
        allowed = []
        [crates.core]
        allowed = ["types"]
        dev_allowed = ["fixture"]
        forbidden_external = ["@http_crates"]
        [crates.fixture]
        allowed = []
        production = false
        "#,
    )
    .unwrap()
}

fn pkg(name: &str, deps: &[(&str, Option<&str>)]) -> String {
    let deps: Vec<String> = deps
        .iter()
        .map(|(d, kind)| {
            let kind = kind.map_or_else(|| "null".to_owned(), |k| format!("\"{k}\""));
            format!(r#"{{"name":"{d}","kind":{kind}}}"#)
        })
        .collect();
    format!(
        r#"{{"name":"{name}","manifest_path":"/w/{name}/Cargo.toml","targets":[],
            "dependencies":[{}]}}"#,
        deps.join(",")
    )
}

fn run(packages: &[String]) -> Vec<String> {
    let meta = metadata(&format!(r#"{{"packages":[{}]}}"#, packages.join(",")));
    violations(&config(), &meta).errors
}

#[test]
fn clean_workspace_has_no_violations() {
    let errors = run(&[
        pkg("types", &[("serde", None)]),
        pkg("core", &[("types", None)]),
    ]);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn upward_dependency_is_rejected() {
    let errors = run(&[pkg("types", &[("core", None)]), pkg("core", &[])]);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("`types` → `core`"));
}

#[test]
fn forbidden_external_group_is_expanded() {
    let errors = run(&[pkg("types", &[]), pkg("core", &[("axum", None)])]);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("`core` → `axum` yasak"));
}

#[test]
fn unlisted_crate_is_rejected() {
    let errors = run(&[pkg("rogue", &[])]);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("`rogue` architecture.toml'da tanımlı değil"));
}

#[test]
fn dev_only_dependency_needs_dev_kind() {
    let fixture = pkg("fixture", &[]);
    assert!(run(&[pkg("core", &[("fixture", Some("dev"))]), fixture.clone()]).is_empty());
    let errors = run(&[pkg("core", &[("fixture", None)]), fixture]);
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(errors[1].contains("üretim dışı `fixture`"));
}
