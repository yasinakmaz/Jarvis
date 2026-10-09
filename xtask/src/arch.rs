//! Katman yönü denetimi: `architecture.toml` ↔ `cargo metadata` (Mimari §2, §9 Katman 3).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Context as _;
use serde::Deserialize;

use crate::metadata::{Dependency, Metadata, Package};
use crate::report::Findings;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchConfig {
    #[serde(default)]
    pub defaults: Defaults,
    pub crates: BTreeMap<String, CrateRule>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Defaults {
    #[serde(default)]
    pub http_crates: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrateRule {
    pub allowed: Vec<String>,
    #[serde(default)]
    pub dev_allowed: Vec<String>,
    #[serde(default)]
    pub forbidden_external: Vec<String>,
    #[serde(default = "yes")]
    pub production: bool,
}

const fn yes() -> bool {
    true
}

impl ArchConfig {
    pub fn load(root: &Path) -> anyhow::Result<Self> {
        let path = root.join("architecture.toml");
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("okunamadı: {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("geçersiz: {}", path.display()))
    }

    /// `@grup` referanslarını `[defaults]` listeleriyle genişletir.
    fn expand<'a>(&'a self, names: &'a [String]) -> BTreeSet<&'a str> {
        names
            .iter()
            .flat_map(|name| match name.as_str() {
                "@http_crates" => self
                    .defaults
                    .http_crates
                    .iter()
                    .map(String::as_str)
                    .collect(),
                other => vec![other],
            })
            .collect()
    }
}

/// Çalışma alanını `architecture.toml` sözleşmesine karşı denetler.
pub fn check(root: &Path, metadata: &Metadata) -> anyhow::Result<Findings> {
    let config = ArchConfig::load(root)?;
    Ok(violations(&config, metadata))
}

/// Saf denetim: ihlaller `Findings::errors` içinde döner.
pub fn violations(config: &ArchConfig, metadata: &Metadata) -> Findings {
    let mut findings = Findings::default();
    let members = metadata.member_names();
    for package in &metadata.packages {
        let Some(rule) = config.crates.get(&package.name) else {
            findings.error(format!(
                "`{}` architecture.toml'da tanımlı değil. Yeni crate eklemek mimari \
                 kararıdır: tasarım belgesi + ADR yazın, insan architecture.toml'a eklesin.",
                package.name
            ));
            continue;
        };
        for dep in &package.dependencies {
            if members.contains(dep.name.as_str()) {
                check_internal(config, package, rule, dep, &mut findings);
            } else if config
                .expand(&rule.forbidden_external)
                .contains(dep.name.as_str())
            {
                findings.error(format!(
                    "`{}` → `{}` yasak (Mimari §2: bu katman bu crate'e bağlanamaz). \
                     İşlevi izin verilen bir katmana taşıyın.",
                    package.name, dep.name
                ));
            }
        }
    }
    findings
}

fn check_internal(
    config: &ArchConfig,
    package: &Package,
    rule: &CrateRule,
    dep: &Dependency,
    findings: &mut Findings,
) {
    let allowed =
        rule.allowed.contains(&dep.name) || (dep.is_dev() && rule.dev_allowed.contains(&dep.name));
    if !allowed {
        findings.error(format!(
            "`{}` → `{}` katman yönünü ihlal ediyor (izinli: [{}]). Bağımlılığı kaldırın \
             veya ortak tipi daha alt bir katmana (ör. jarvis-types) taşıyın.",
            package.name,
            dep.name,
            rule.allowed.join(", ")
        ));
    }
    let target_is_production = config.crates.get(&dep.name).is_none_or(|r| r.production);
    if rule.production && !dep.is_dev() && !target_is_production {
        findings.error(format!(
            "üretim paketi `{}`, üretim dışı `{}` paketine bağlanamaz.",
            package.name, dep.name
        ));
    }
}

#[cfg(test)]
#[path = "arch_tests.rs"]
mod tests;
