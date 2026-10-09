//! Örnek dosyalar (Tasarım 0004 test planı, L1). Oracle: elle yazılmış `.expected` dosyaları
//! ve elle kurulmuş `Config` değerleri; doğrulayıcının kendi çıktısından türetilmez.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::time::Duration;

use jarvis_config::{
    BaseUrl, Capability, Config, ConfigErrorKind, EnvVarName, Limits, Provider, ProviderName,
    Roles, Server, load,
};

fn dir(kind: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/config")
        .join(kind)
}

fn env() -> BTreeMap<String, String> {
    [
        ("NVIDIA_API_KEY", "nvapi-test"),
        ("LOCAL_KEY", "local"),
        ("EMPTY_KEY", ""),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect()
}

macro_rules! invalid_fixtures {
    ($($test:ident => $file:literal),* $(,)?) => {
        $(
            #[test]
            fn $test() {
                let path = dir("invalid").join(format!("{}.toml", $file));
                let expected = std::fs::read_to_string(path.with_extension("expected")).unwrap();
                let errors = load(&path, &env()).expect_err("geçersiz örnek kabul edildi");
                assert_eq!(errors.to_string(), expected.trim_end(), "{}", path.display());
                assert_eq!(errors.len(), expected.trim_end().lines().count());
            }
        )*
        const INVALID: &[&str] = &[$($file),*];
    };
}

invalid_fixtures! {
    unknown_field_is_reported_with_suggestion => "unknown-field",
    non_loopback_listen_is_rejected => "non-loopback",
    role_capability_mismatch_is_rejected => "capability-mismatch",
    missing_or_empty_api_key_env_is_rejected => "missing-key",
    other_schema_version_stops_validation => "schema-version",
    missing_and_mistyped_sections_are_reported => "missing-sections",
    all_errors_are_reported_at_once => "many-errors",
}

#[test]
fn every_invalid_fixture_has_a_test() {
    let mut on_disk: Vec<String> = std::fs::read_dir(dir("invalid"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = INVALID.iter().map(|s| (*s).to_owned()).collect();
    listed.sort();
    assert_eq!(on_disk, listed);
}

fn name(s: &str) -> Option<ProviderName> {
    ProviderName::parse(s)
}

fn nvidia(capabilities: &[Capability]) -> Option<Provider> {
    Some(Provider {
        base_url: BaseUrl::parse("https://integrate.api.nvidia.com/v1").ok()?,
        api_key_env: EnvVarName::parse("NVIDIA_API_KEY")?,
        model: "meta/llama-3.3-70b-instruct".to_owned(),
        capabilities: capabilities.iter().copied().collect(),
        requests_per_minute: NonZeroU32::new(40)?,
        timeout: Duration::from_mins(1),
    })
}

#[test]
fn full_example_parses_to_the_expected_config() {
    let config = load(&dir("valid").join("full.toml"), &env()).unwrap();
    let vision = Provider {
        base_url: BaseUrl::parse("http://127.0.0.1:11434/v1").unwrap(),
        api_key_env: EnvVarName::parse("LOCAL_KEY").unwrap(),
        model: "llava".to_owned(),
        capabilities: BTreeSet::from([Capability::Vision]),
        requests_per_minute: NonZeroU32::new(600).unwrap(),
        timeout: Duration::from_mins(2),
    };
    let expected = Config {
        server: Server {
            listen: "127.0.0.1:7878".parse().unwrap(),
        },
        limits: Limits {
            max_steps: NonZeroU32::new(30).unwrap(),
            max_run: Duration::from_mins(15),
            repeat_threshold: NonZeroU32::new(4).unwrap(),
        },
        providers: BTreeMap::from([
            (
                name("nvidia").unwrap(),
                nvidia(&[Capability::Tools, Capability::Streaming]).unwrap(),
            ),
            (name("local-vision").unwrap(), vision.clone()),
        ]),
        roles: Roles {
            planner: name("nvidia").unwrap(),
            fast: name("nvidia").unwrap(),
            vision: Some(name("local-vision").unwrap()),
        },
    };
    assert_eq!(config, expected);
    let chosen = config
        .roles
        .vision
        .as_ref()
        .and_then(|v| config.provider(v));
    assert_eq!(chosen, Some(&vision));
}

#[test]
fn minimal_example_uses_documented_defaults() {
    let config = load(&dir("valid").join("minimal.toml"), &env()).unwrap();
    assert_eq!(config.server.listen, "127.0.0.1:7878".parse().unwrap());
    assert_eq!(config.server.listen.to_string(), Server::DEFAULT_LISTEN);
    assert_eq!(
        config.limits,
        Limits {
            max_steps: NonZeroU32::new(25).unwrap(),
            max_run: Duration::from_mins(10),
            repeat_threshold: NonZeroU32::new(3).unwrap()
        }
    );
    assert_eq!(config.limits.max_steps.get(), Limits::DEFAULT_MAX_STEPS);
    assert_eq!(
        config.limits.max_run.as_secs(),
        u64::from(Limits::DEFAULT_MAX_RUN_SECONDS)
    );
    assert_eq!(
        config.limits.repeat_threshold.get(),
        Limits::DEFAULT_REPEAT_THRESHOLD
    );
    assert_eq!(config.roles.vision, None);
    assert_eq!(
        config.provider(&name("nvidia").unwrap()),
        nvidia(&[Capability::Tools]).as_ref()
    );
    assert_eq!(config.provider(&name("other").unwrap()), None);
}

#[test]
fn unreadable_file_reports_its_path() {
    let path = dir("valid").join("does-not-exist.toml");
    let errors = load(&path, &env()).unwrap_err();
    let error = errors.iter().next().unwrap();
    assert_eq!(errors.len(), 1);
    assert_eq!(error.path, path.display().to_string());
    assert!(matches!(error.kind, ConfigErrorKind::Io { .. }));
    assert!(
        errors
            .to_string()
            .starts_with(&format!("{}: dosya okunamadı: ", path.display()))
    );
}
