//! Tek tek kurallar (Tasarım 0004 doğrulama akışı). Oracle: elle yazılmış beklenen hatalar ve
//! standart kütüphanenin `IpAddr::is_loopback`'i.

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};

use jarvis_config::{Capability, ConfigError, ConfigErrorKind, ConfigErrors, parse};
use proptest::prelude::*;

fn env() -> BTreeMap<String, String> {
    BTreeMap::from([("KEY".to_owned(), "k".to_owned())])
}

const PROVIDER: &str = r#"
[providers.p]
base_url = "https://example.com/v1"
api_key_env = "KEY"
model = "m"
capabilities = ["tools", "vision"]
requests_per_minute = 1
timeout_seconds = 1
"#;

fn document(head: &str, roles: &str) -> String {
    format!("schema_version = 1\n{head}\n{PROVIDER}\n[roles]\n{roles}\n")
}

fn errors(text: &str) -> Vec<String> {
    // Başarı boş liste verir; beklenen hatalarla karşılaştırma yine başarısız olur.
    parse(text, &env())
        .err()
        .map(|e| e.iter().map(ToString::to_string).collect())
        .unwrap_or_default()
}

#[test]
fn schema_version_must_be_present_and_an_integer() {
    assert_eq!(
        errors("[server]\nbogus = 1\n"),
        ["schema_version: zorunlu alan eksik"]
    );
    assert_eq!(
        errors("schema_version = \"1\"\n"),
        ["schema_version: tamsayı bekleniyordu, metin bulundu"]
    );
    let other = parse("schema_version = 3\n", &env()).unwrap_err();
    assert_eq!(
        other.iter().next().map(|e| &e.kind),
        Some(&ConfigErrorKind::SchemaVersion {
            found: 3,
            expected: 1
        })
    );
}

#[test]
fn syntax_errors_are_reported_without_a_field_path() {
    let errs = parse("schema_version = = 1", &env()).unwrap_err();
    assert_eq!(errs.len(), 1);
    let error = errs.iter().next().unwrap();
    assert_eq!(error.path, "");
    assert!(matches!(error.kind, ConfigErrorKind::Syntax { .. }));
    assert!(errs.to_string().starts_with("TOML ayrıştırılamadı: "));
}

#[test]
fn roles_accept_fully_capable_provider() {
    let config = parse(
        &document("", "planner = \"p\"\nfast = \"p\"\nvision = \"p\""),
        &env(),
    )
    .unwrap();
    assert_eq!(
        config
            .roles
            .vision
            .as_ref()
            .map(jarvis_config::ProviderName::as_str),
        Some("p")
    );
    let provider = config.provider(&config.roles.planner).unwrap();
    assert!(provider.capabilities.contains(&Capability::Tools));
}

#[test]
fn roles_must_be_strings_and_are_required() {
    assert_eq!(
        errors(&document("", "planner = 1\nvision = false")),
        [
            "roles.planner: metin bekleniyordu, tamsayı bulundu",
            "roles.fast: zorunlu alan eksik",
            "roles.vision: metin bekleniyordu, mantıksal bulundu",
        ]
    );
}

#[test]
fn role_pointing_at_an_invalid_provider_is_not_reported_twice() {
    let text = document(
        "[providers.q]\nmodel = \"x\"",
        "planner = \"q\"\nfast = \"p\"",
    );
    assert_eq!(
        errors(&text),
        [
            "providers.q.base_url: zorunlu alan eksik",
            "providers.q.api_key_env: zorunlu alan eksik",
            "providers.q.capabilities: zorunlu alan eksik",
            "providers.q.requests_per_minute: zorunlu alan eksik",
            "providers.q.timeout_seconds: zorunlu alan eksik",
        ]
    );
}

#[test]
fn provider_entries_and_sections_must_be_tables() {
    let text = "schema_version = 1\nlimits = 5\nproviders = { p = 3 }\nroles = []\n";
    assert_eq!(
        errors(text),
        [
            "limits: tablo bekleniyordu, tamsayı bulundu",
            "providers.p: tablo bekleniyordu, tamsayı bulundu",
            "roles: tablo bekleniyordu, dizi bulundu",
        ]
    );
}

#[test]
fn listen_must_be_an_address_string() {
    let roles = "planner = \"p\"\nfast = \"p\"";
    assert_eq!(
        errors(&document("[server]\nlisten = \"localhost:7878\"", roles)),
        ["server.listen: 'localhost:7878' geçerli bir IP:port değil (ör. 127.0.0.1:7878)"]
    );
    assert_eq!(
        errors(&document("[server]\nlisten = 7878.0", roles)),
        ["server.listen: metin bekleniyordu, ondalık bulundu"]
    );
    assert_eq!(
        errors(&document("[server]\nlisten = 1979-05-27", roles)),
        ["server.listen: metin bekleniyordu, tarih bulundu"]
    );
    assert_eq!(
        errors(&document(
            "[server]\nlisten = \"[::]:7878\"\nport = 1",
            roles
        )),
        [
            "server.port: bilinmeyen alan",
            "server.listen: :: reddedildi; 127.0.0.1 kullanın"
        ]
    );
}

#[test]
fn unknown_keys_and_unused_invalid_providers_still_fail() {
    let roles = "planner = \"p\"\nfast = \"p\"";
    assert_eq!(
        errors(&document("extra = 1", roles)),
        ["extra: bilinmeyen alan"]
    );
    assert_eq!(
        errors(&document("[providers.unused]\nmodel = \"\"", roles)),
        [
            "providers.unused.base_url: zorunlu alan eksik",
            "providers.unused.api_key_env: zorunlu alan eksik",
            "providers.unused.model: boş olamaz",
            "providers.unused.capabilities: zorunlu alan eksik",
            "providers.unused.requests_per_minute: zorunlu alan eksik",
            "providers.unused.timeout_seconds: zorunlu alan eksik",
        ]
    );
}

#[test]
fn integer_limits_accept_the_upper_bound() {
    let head = "[limits]\nmax_steps = 4294967295\nmax_run_seconds = 1\nrepeat_threshold = 1";
    let config = parse(&document(head, "planner = \"p\"\nfast = \"p\""), &env()).unwrap();
    assert_eq!(config.limits.max_steps.get(), u32::MAX);
    assert_eq!(config.limits.max_run.as_secs(), 1);
    assert_eq!(config.limits.repeat_threshold.get(), 1);
}

#[test]
fn errors_display_one_per_line() {
    let errs = ConfigErrors(vec![
        ConfigError::new("a", ConfigErrorKind::MissingField),
        ConfigError::new("", ConfigErrorKind::Empty),
    ]);
    assert_eq!(errs.to_string(), "a: zorunlu alan eksik\nboş olamaz");
    assert_eq!((&errs).into_iter().count(), 2);
    assert!(!errs.is_empty());
    assert!(ConfigErrors::default().is_empty());
    let single: ConfigErrors = ConfigError::new("x", ConfigErrorKind::PortZero).into();
    assert_eq!(single.len(), 1);
}

fn listen_document(addr: SocketAddr) -> String {
    document(
        &format!("[server]\nlisten = \"{addr}\""),
        "planner = \"p\"\nfast = \"p\"",
    )
}

proptest! {
    #[test]
    fn only_loopback_listen_addresses_are_accepted(ip in any::<IpAddr>(), port in 1u16..) {
        let addr = SocketAddr::new(ip, port);
        let result = parse(&listen_document(addr), &env());
        prop_assert_eq!(result.is_ok(), ip.is_loopback());
        match result {
            Ok(config) => prop_assert_eq!(config.server.listen, addr),
            Err(errs) => prop_assert_eq!(
                errs.to_string(),
                format!("server.listen: {ip} reddedildi; 127.0.0.1 kullanın")
            ),
        }
    }
}
