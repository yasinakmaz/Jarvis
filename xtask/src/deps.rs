//! Bağımlılık beyaz listesi (Mimari §9 Katman 4, crate halüsinasyon kontrolü).
//!
//! Her doğrudan üçüncü taraf bağımlılık `supply-chain/allowlist.toml`'da, var olan
//! bir ADR'ye bağlı olarak kayıtlı olmalıdır. Liste yalnızca insan onayıyla değişir.
//! Geçişli bağımlılıklar `cargo-deny` ile denetlenir.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Context as _;
use serde::Deserialize;

use crate::metadata::Metadata;
use crate::report::Findings;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Allowlist {
    #[serde(rename = "crate", default)]
    pub crates: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub name: String,
    /// ADR numarası, ör. "0020".
    pub adr: String,
    pub reason: String,
}

pub fn check(root: &Path, metadata: &Metadata) -> anyhow::Result<Findings> {
    let path = root.join("supply-chain/allowlist.toml");
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("okunamadı: {}", path.display()))?;
    let allowlist: Allowlist =
        toml::from_str(&text).with_context(|| format!("geçersiz: {}", path.display()))?;
    let adrs = adr_numbers(&root.join("docs/adr"))?;
    let declared = workspace_dependencies(root)?;
    let mut used = metadata.external_dependencies();
    used.extend(declared.iter().map(String::as_str));
    Ok(evaluate(&allowlist, &used, &adrs))
}

/// Saf değerlendirme: kullanılan crate'ler, beyaz liste ve mevcut ADR numaraları.
pub fn evaluate(allowlist: &Allowlist, used: &BTreeSet<&str>, adrs: &BTreeSet<String>) -> Findings {
    let mut findings = Findings::default();
    let listed: BTreeSet<&str> = allowlist.crates.iter().map(|e| e.name.as_str()).collect();
    for name in used.difference(&listed) {
        findings.error(format!(
            "`{name}` beyaz listede yok. 1) `cargo xtask crate-info {name}` ile crates.io'da \
             gerçekten var olduğunu, yaşını ve indirme sayısını doğrulayın; 2) gerekçeli bir \
             ADR yazın (docs/adr/); 3) insan onayıyla supply-chain/allowlist.toml'a eklensin."
        ));
    }
    for entry in &allowlist.crates {
        if !adrs.contains(&entry.adr) {
            findings.error(format!(
                "allowlist: `{}` için ADR {} bulunamadı (docs/adr/{}-*.md).",
                entry.name, entry.adr, entry.adr
            ));
        }
        if entry.reason.trim().is_empty() {
            findings.error(format!("allowlist: `{}` için gerekçe boş.", entry.name));
        }
        if !used.contains(entry.name.as_str()) {
            findings.warn(format!(
                "allowlist: `{}` kullanılmıyor; kaldırılabilir.",
                entry.name
            ));
        }
    }
    findings
}

/// Kök `[workspace.dependencies]` anahtarları.
fn workspace_dependencies(root: &Path) -> anyhow::Result<Vec<String>> {
    let text = std::fs::read_to_string(root.join("Cargo.toml"))?;
    let manifest: toml::Value = toml::from_str(&text).context("kök Cargo.toml geçersiz")?;
    Ok(manifest
        .get("workspace")
        .and_then(|w| w.get("dependencies"))
        .and_then(toml::Value::as_table)
        .map(|t| t.keys().cloned().collect())
        .unwrap_or_default())
}

/// `docs/adr/NNNN-*.md` dosyalarındaki numaralar.
fn adr_numbers(dir: &Path) -> anyhow::Result<BTreeSet<String>> {
    let mut numbers = BTreeSet::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("okunamadı: {}", dir.display()))? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if let Some((number, _)) = name.split_once('-')
            && crate::workspace::has_extension(&name, "md")
            && number.len() == 4
            && number.chars().all(|c| c.is_ascii_digit())
        {
            numbers.insert(number.to_owned());
        }
    }
    Ok(numbers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowlist(text: &str) -> Allowlist {
        toml::from_str(text).unwrap()
    }

    #[test]
    fn unlisted_dependency_is_rejected_with_instructions() {
        let list = allowlist("[[crate]]\nname = \"serde\"\nadr = \"0020\"\nreason = \"x\"");
        let adrs = BTreeSet::from(["0020".to_owned()]);
        let findings = evaluate(&list, &BTreeSet::from(["serde", "rand"]), &adrs);
        assert_eq!(findings.errors.len(), 1);
        assert!(findings.errors[0].contains("`rand` beyaz listede yok"));
        assert!(findings.errors[0].contains("cargo xtask crate-info rand"));
    }

    #[test]
    fn entry_requires_existing_adr_and_reason() {
        let list = allowlist("[[crate]]\nname = \"serde\"\nadr = \"0099\"\nreason = \" \"");
        let findings = evaluate(&list, &BTreeSet::from(["serde"]), &BTreeSet::new());
        assert_eq!(findings.errors.len(), 2, "{:?}", findings.errors);
    }

    #[test]
    fn unused_entry_is_only_a_warning() {
        let list = allowlist("[[crate]]\nname = \"serde\"\nadr = \"0020\"\nreason = \"x\"");
        let adrs = BTreeSet::from(["0020".to_owned()]);
        let findings = evaluate(&list, &BTreeSet::new(), &adrs);
        assert!(findings.errors.is_empty());
        assert_eq!(findings.warnings.len(), 1);
    }
}
