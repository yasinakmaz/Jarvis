//! `Paths::resolve` (Tasarım 0004 test planı). Oracle: Mimari §8 tablosundaki yollar.

use std::collections::BTreeMap;
use std::path::PathBuf;

use jarvis_config::{EnvLookup, Paths, Profile, SystemEnv};

fn env(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

fn messages(profile: Profile, pairs: &[(&str, &str)]) -> Vec<String> {
    // Başarı boş liste verir; beklenen hatalarla karşılaştırma yine başarısız olur.
    Paths::resolve(profile, &env(pairs))
        .err()
        .map(|e| e.iter().map(ToString::to_string).collect())
        .unwrap_or_default()
}

#[test]
fn production_defaults_follow_xdg_under_home() {
    let paths = Paths::resolve(Profile::Production, &env(&[("HOME", "/home/u")])).unwrap();
    assert_eq!(
        paths,
        Paths {
            config: PathBuf::from("/home/u/.config/jarvis/config.toml"),
            secrets: PathBuf::from("/home/u/.config/jarvis/secrets.env"),
            token: PathBuf::from("/home/u/.config/jarvis/token"),
            database: PathBuf::from("/home/u/.local/share/jarvis/jarvis.db"),
        }
    );
}

#[test]
fn development_uses_its_own_app_id_and_explicit_xdg_dirs() {
    let pairs = [("XDG_CONFIG_HOME", "/x/cfg"), ("XDG_DATA_HOME", "/x/data")];
    let paths = Paths::resolve(Profile::Development, &env(&pairs)).unwrap();
    assert_eq!(
        paths,
        Paths {
            config: PathBuf::from("/x/cfg/jarvis-dev/config.toml"),
            secrets: PathBuf::from("/x/cfg/jarvis-dev/secrets.env"),
            token: PathBuf::from("/x/cfg/jarvis-dev/token"),
            database: PathBuf::from("/x/data/jarvis-dev/jarvis.db"),
        }
    );
    assert_eq!(Profile::Production.app_id(), "jarvis");
    assert_eq!(Profile::Development.app_id(), "jarvis-dev");
}

#[test]
fn empty_xdg_variable_means_unset() {
    let pairs = [
        ("HOME", "/h"),
        ("XDG_CONFIG_HOME", ""),
        ("XDG_DATA_HOME", "/d"),
    ];
    let paths = Paths::resolve(Profile::Production, &env(&pairs)).unwrap();
    assert_eq!(paths.config, PathBuf::from("/h/.config/jarvis/config.toml"));
    assert_eq!(paths.database, PathBuf::from("/d/jarvis/jarvis.db"));
}

#[test]
fn missing_home_is_reported_once() {
    assert_eq!(
        messages(Profile::Production, &[]),
        ["HOME: tanımlı değil veya boş; XDG yolları çözülemiyor"]
    );
    assert_eq!(
        messages(Profile::Production, &[("HOME", "")]),
        ["HOME: tanımlı değil veya boş; XDG yolları çözülemiyor"]
    );
}

#[test]
fn relative_paths_are_errors_not_silently_ignored() {
    assert_eq!(
        messages(Profile::Development, &[("XDG_CONFIG_HOME", "rel")]),
        [
            "HOME: tanımlı değil veya boş; XDG yolları çözülemiyor",
            "XDG_CONFIG_HOME: 'rel' göreli; mutlak yol olmalı",
        ]
    );
    assert_eq!(
        messages(
            Profile::Production,
            &[("HOME", "home/u"), ("XDG_DATA_HOME", "/d")]
        ),
        ["HOME: 'home/u' göreli; mutlak yol olmalı"]
    );
}

#[test]
fn system_env_reads_the_process_environment() {
    // Oracle: derleme anında gömülen değer; cargo/nextest aynı değişkeni teste de verir.
    let expected = std::ffi::OsString::from(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(SystemEnv.get("CARGO_MANIFEST_DIR"), Some(expected));
    assert_eq!(SystemEnv.get("JARVIS_CONFIG_TEST_SURELY_UNDEFINED"), None);
}
