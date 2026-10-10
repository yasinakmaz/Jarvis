//! Çalışma alanı hijyeni: lint tablosunun onaylı kopyayla aynı olması ve her crate'in
//! lint'leri çalışma alanından miras alması (Mimari §9 Katman 1).

use std::path::Path;

use anyhow::Context as _;
use toml::Value;

use crate::metadata::Metadata;
use crate::report::Findings;
use crate::workspace;

/// Onaylı lint/profil tablosu; xtask ile birlikte korunur.
const BASELINE: &str = include_str!("baseline/workspace.toml");

pub fn check(root: &Path, metadata: &Metadata) -> anyhow::Result<Findings> {
    let mut findings = Findings::default();
    let root_manifest = read_toml(&root.join("Cargo.toml"))?;
    let baseline: Value = toml::from_str(BASELINE).context("baseline/workspace.toml geçersiz")?;
    compare_baseline(&root_manifest, &baseline, &mut findings);
    for package in &metadata.packages {
        let manifest = read_toml(&package.manifest_path)?;
        let rel = workspace::relative(root, &package.manifest_path);
        check_member_lints(&rel, &manifest, &mut findings);
    }
    Ok(findings)
}

fn read_toml(path: &Path) -> anyhow::Result<Value> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("okunamadı: {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("geçersiz TOML: {}", path.display()))
}

fn lookup<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |v, key| v.get(key))
}

/// Kök manifestteki `[workspace.lints]` ve `[profile]` onaylı kopyayla aynı olmalıdır.
pub fn compare_baseline(manifest: &Value, baseline: &Value, findings: &mut Findings) {
    for path in [&["workspace", "lints"][..], &["profile"][..]] {
        if lookup(manifest, path) != lookup(baseline, path) {
            findings.error(format!(
                "kök Cargo.toml `[{}]` tablosu onaylı kopyadan farklı \
                 (xtask/src/baseline/workspace.toml). Kalite kapısı yapılandırması yalnızca \
                 insan tarafından değiştirilir; değişikliği geri alın.",
                path.join(".")
            ));
        }
    }
}

/// Her paket `[lints] workspace = true` taşımalı ve başka lint anahtarı içermemelidir.
pub fn check_member_lints(rel: &str, manifest: &Value, findings: &mut Findings) {
    let expected: Value = toml::from_str("workspace = true").unwrap_or(Value::Boolean(false));
    if manifest.get("lints") != Some(&expected) {
        findings.error(format!(
            "{rel}: `[lints]` tablosu tam olarak `workspace = true` olmalıdır. Lint'ler \
             crate başına gevşetilemez; uyarıyı kodda düzeltin ya da gerekçeli \
             `#[expect(lint, reason = \"...\")]` kullanın."
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(text: &str) -> Value {
        toml::from_str(text).unwrap()
    }

    #[test]
    fn member_without_workspace_lints_is_rejected() {
        let mut findings = Findings::default();
        check_member_lints(
            "a/Cargo.toml",
            &value("[package]\nname = \"a\""),
            &mut findings,
        );
        check_member_lints(
            "b/Cargo.toml",
            &value("[lints]\nworkspace = true"),
            &mut findings,
        );
        let extra = "[lints]\nworkspace = true\n[lints.clippy]\nunwrap_used = \"allow\"";
        check_member_lints("c/Cargo.toml", &value(extra), &mut findings);
        assert_eq!(findings.errors.len(), 2);
        assert!(findings.errors[0].starts_with("a/Cargo.toml"));
        assert!(findings.errors[1].starts_with("c/Cargo.toml"));
    }

    #[test]
    fn tampered_lint_table_is_rejected() {
        let table = |level: &str| {
            value(&format!(
                "[workspace.lints.clippy]\nunwrap_used = \"{level}\"\n[profile.release]\nlto = 1"
            ))
        };
        let baseline = table("deny");
        let same = baseline.clone();
        let mut findings = Findings::default();
        compare_baseline(&same, &baseline, &mut findings);
        assert!(findings.errors.is_empty());
        let tampered = table("allow");
        compare_baseline(&tampered, &baseline, &mut findings);
        assert_eq!(findings.errors.len(), 1);
        assert!(findings.errors[0].contains("workspace.lints"));
    }

    #[test]
    fn embedded_baseline_matches_root_manifest() {
        let mut findings = Findings::default();
        let root = read_toml(&workspace::root().join("Cargo.toml")).unwrap();
        compare_baseline(&root, &value(BASELINE), &mut findings);
        assert!(findings.errors.is_empty(), "{:?}", findings.errors);
    }
}
