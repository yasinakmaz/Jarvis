//! `config.toml` metnini [`Config`]'e çevirir; kuralların hepsi denetlenir, hatalar toplanır.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;

use toml::{Table, Value};

use crate::providers::{Providers, providers};
use crate::reader::{Errors, Section, expect_int, expect_positive, expect_str, expect_table};
use crate::{
    Capability, Config, ConfigError, ConfigErrorKind, ConfigErrors, EnvLookup, Limits,
    ProviderName, Roles, SCHEMA_VERSION, Server,
};

const TOP_LEVEL: &[&str] = &["schema_version", "server", "limits", "providers", "roles"];
const SERVER_FIELDS: &[&str] = &["listen"];
const LIMITS_FIELDS: &[&str] = &["max_steps", "max_run_seconds", "repeat_threshold"];
const ROLE_FIELDS: &[&str] = &["planner", "fast", "vision"];

/// [`Server::DEFAULT_LISTEN`] ile aynı adres.
const DEFAULT_LISTEN: SocketAddr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 7878));

/// Metni ayrıştırır ve doğrular (saf; dosya sistemine dokunmaz).
pub fn parse(text: &str, env: &dyn EnvLookup) -> Result<Config, ConfigErrors> {
    let table: Table = text.parse().map_err(|e: toml::de::Error| {
        let detail = e.to_string().trim_end().to_owned();
        ConfigError::new("", ConfigErrorKind::Syntax { detail })
    })?;
    let mut errors = Errors::new();
    if !schema_version_supported(&table, &mut errors) {
        return Err(ConfigErrors(errors));
    }
    let root = Section::open(String::new(), &table, TOP_LEVEL, &mut errors);
    let server = server(&root, &mut errors);
    let limits = limits(&root, &mut errors);
    let providers = providers(&root, env, &mut errors);
    let roles = roles(&root, &providers, &mut errors);
    match (server, limits, roles) {
        (Some(server), Some(limits), Some(roles)) if errors.is_empty() => Ok(Config {
            server,
            limits,
            providers: providers.valid,
            roles,
        }),
        _ => Err(ConfigErrors(errors)),
    }
}

/// Sürüm farklıysa diğer alanların anlamı bilinmez; başka kural denetlenmez.
fn schema_version_supported(table: &Table, errors: &mut Errors) -> bool {
    let path = "schema_version";
    let Some(value) = table.get(path) else {
        errors.push(ConfigError::new(path, ConfigErrorKind::MissingField));
        return false;
    };
    let Some(found) = expect_int(value, path, errors) else {
        return false;
    };
    if found != SCHEMA_VERSION {
        let kind = ConfigErrorKind::SchemaVersion {
            found,
            expected: SCHEMA_VERSION,
        };
        errors.push(ConfigError::new(path, kind));
        return false;
    }
    true
}

fn server(root: &Section<'_>, errors: &mut Errors) -> Option<Server> {
    let section = root.child("server", SERVER_FIELDS, errors).ok()?;
    let Some((section, value)) = section.and_then(|s| s.optional("listen").map(|v| (s, v))) else {
        return Some(Server {
            listen: DEFAULT_LISTEN,
        });
    };
    let path = section.path("listen");
    let text = expect_str(value, &path, errors)?;
    let Ok(listen) = text.parse::<SocketAddr>() else {
        let kind = ConfigErrorKind::InvalidAddress {
            value: text.to_owned(),
        };
        errors.push(ConfigError::new(path, kind));
        return None;
    };
    let before = errors.len();
    if !listen.ip().is_loopback() {
        let kind = ConfigErrorKind::NonLoopback {
            ip: listen.ip().to_string(),
        };
        errors.push(ConfigError::new(&path, kind));
    }
    if listen.port() == 0 {
        errors.push(ConfigError::new(&path, ConfigErrorKind::PortZero));
    }
    (errors.len() == before).then_some(Server { listen })
}

fn limits(root: &Section<'_>, errors: &mut Errors) -> Option<Limits> {
    let section = root.child("limits", LIMITS_FIELDS, errors).ok()?;
    let mut field = |key: &str, default: u32| match section
        .as_ref()
        .and_then(|s| s.optional(key).map(|v| (s.path(key), v)))
    {
        Some((path, value)) => expect_positive(value, &path, errors),
        None => std::num::NonZeroU32::new(default),
    };
    let max_steps = field("max_steps", Limits::DEFAULT_MAX_STEPS);
    let max_run = field("max_run_seconds", Limits::DEFAULT_MAX_RUN_SECONDS);
    let repeat_threshold = field("repeat_threshold", Limits::DEFAULT_REPEAT_THRESHOLD);
    Some(Limits {
        max_steps: max_steps?,
        max_run: Duration::from_secs(u64::from(max_run?.get())),
        repeat_threshold: repeat_threshold?,
    })
}

fn roles(root: &Section<'_>, providers: &Providers, errors: &mut Errors) -> Option<Roles> {
    let value = root.required("roles", errors)?;
    let path = root.path("roles");
    let section = Section::open(
        path.clone(),
        expect_table(value, &path, errors)?,
        ROLE_FIELDS,
        errors,
    );
    let need = |key: &str| match key {
        "planner" => Some(Capability::Tools),
        "vision" => Some(Capability::Vision),
        _ => None,
    };
    let resolve = |key: &str, value: Option<&Value>, errors: &mut Errors| {
        value.and_then(|v| role(&section.path(key), v, need(key), providers, errors))
    };
    let planner = section.required("planner", errors);
    let planner = resolve("planner", planner, errors);
    let fast = section.required("fast", errors);
    let fast = resolve("fast", fast, errors);
    let vision = section
        .optional("vision")
        .map_or(Some(None), |v| resolve("vision", Some(v), errors).map(Some));
    Some(Roles {
        planner: planner?,
        fast: fast?,
        vision: vision?,
    })
}

/// Rolün sağlayıcı adını doğrular. Dosyada tanımlı ama geçersiz bir sağlayıcıyı gösteriyorsa
/// ek hata üretmez (asıl hata sağlayıcının kendi alanında raporlandı).
fn role(
    path: &str,
    value: &Value,
    need: Option<Capability>,
    providers: &Providers,
    errors: &mut Errors,
) -> Option<ProviderName> {
    let raw = expect_str(value, path, errors)?;
    let Some(name) = ProviderName::parse(raw) else {
        let kind = ConfigErrorKind::InvalidProviderName {
            value: raw.to_owned(),
        };
        errors.push(ConfigError::new(path, kind));
        return None;
    };
    let Some(provider) = providers.valid.get(&name) else {
        if !providers.declared.contains(raw) {
            let kind = ConfigErrorKind::UnknownProvider {
                name: raw.to_owned(),
            };
            errors.push(ConfigError::new(path, kind));
        }
        return None;
    };
    if let Some(capability) = need
        && !provider.capabilities.contains(&capability)
    {
        let kind = ConfigErrorKind::MissingCapability {
            provider: raw.to_owned(),
            capability,
        };
        errors.push(ConfigError::new(path, kind));
        return None;
    }
    Some(name)
}
