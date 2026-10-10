//! Boyut sınırları (Mimari §9 Katman 2): dosya uzunluğu, satır genişliği, `mod.rs`
//! içeriği ve crate başına dosya sayısı. Fonksiyon uzunluğu Clippy'dedir.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context as _;

use crate::report::Findings;
use crate::workspace;

pub const MAX_FILE_LINES: usize = 300;
pub const MAX_LINE_WIDTH: usize = 100;
pub const CRATE_FILE_WARNING: usize = 15;
const ALLOW_MARKER: &str = "// xtask: allow-long-file:";

/// Denetlenen kaynak kökleri.
const SOURCE_DIRS: &[&str] = &["crates", "xtask"];

pub fn check(root: &Path) -> anyhow::Result<Findings> {
    let mut findings = Findings::default();
    let mut per_crate: BTreeMap<String, usize> = BTreeMap::new();
    for dir in SOURCE_DIRS {
        for path in rust_files(&root.join(dir))? {
            let rel = workspace::relative(root, &path);
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("okunamadı: {}", path.display()))?;
            check_file(&rel, &text, &mut findings);
            if !is_test_file(&rel) {
                let krate = rel.split("/src/").next().unwrap_or(&rel).to_owned();
                let count = per_crate.entry(krate).or_default();
                *count = count.saturating_add(1);
            }
        }
    }
    for (krate, count) in per_crate {
        if count > CRATE_FILE_WARNING {
            findings.warn(format!(
                "{krate}: {count} kaynak dosya (>{CRATE_FILE_WARNING}). Crate'i sorumluluklarına \
                 göre bölmeyi düşünün (yeni crate = tasarım belgesi + ADR)."
            ));
        }
    }
    Ok(findings)
}

/// Tek dosyanın denetimi (saf; birim testlerle sınanır).
pub fn check_file(rel: &str, text: &str, findings: &mut Findings) {
    for (index, line) in text.lines().enumerate() {
        let width = line.chars().count();
        if width > MAX_LINE_WIDTH {
            findings.error(format!(
                "{rel}:{}: satır {width} sütun (sınır {MAX_LINE_WIDTH}). Uzun dizgeyi \
                 `concat!`/`\\` ile bölün veya yorumu birden çok satıra yayın.",
                index.saturating_add(1)
            ));
        }
    }
    if rel.ends_with("/mod.rs") {
        check_mod_rs(rel, text, findings);
    }
    if is_test_file(rel) {
        return;
    }
    let lines = production_lines(text);
    if lines <= MAX_FILE_LINES {
        return;
    }
    match allow_reason(text) {
        Some(reason) => findings.note(format!(
            "{rel}: {lines} satır, istisna işaretli (gerekçe: {reason}) — PR'da gözden geçirin"
        )),
        None => findings.error(format!(
            "{rel}: {lines} satır (sınır {MAX_FILE_LINES}, `#[cfg(test)]` sonrası hariç). \
             Dosyayı sorumluluklarına göre alt modüllere bölün: tipleri `types.rs`, hataları \
             `error.rs`, ayrıştırmayı `parse.rs` gibi ayrı dosyalara taşıyın; testleri \
             `#[cfg(test)] #[path = \"..._tests.rs\"] mod tests;` ile ayırın."
        )),
    }
}

/// Testler hariç satır sayısı: sütun 0'daki ilk `#[cfg(test)]` satırından öncesi.
pub fn production_lines(text: &str) -> usize {
    text.lines()
        .take_while(|line| line.trim_end() != "#[cfg(test)]")
        .count()
}

/// Dosyanın ilk 5 satırında gerekçeli istisna işareti varsa gerekçeyi döner.
pub fn allow_reason(text: &str) -> Option<&str> {
    text.lines()
        .take(5)
        .find_map(|line| line.trim().strip_prefix(ALLOW_MARKER))
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
}

/// `mod.rs` yalnızca `pub mod`/`pub use` (ve bunların öznitelikleri/yorumları) içerebilir.
pub fn check_mod_rs(rel: &str, text: &str, findings: &mut Findings) {
    let code: String = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//") && !line.starts_with("#["))
        .collect::<Vec<_>>()
        .join(" ");
    let offending = code
        .split(';')
        .map(str::trim)
        .filter(|stmt| !stmt.is_empty())
        .find(|stmt| !(stmt.starts_with("pub mod ") || stmt.starts_with("pub use ")));
    if let Some(stmt) = offending {
        findings.error(format!(
            "{rel}: `mod.rs` yalnızca `pub mod`/`pub use` içerebilir; bulunan: `{stmt}`. \
             Kodu adlandırılmış bir alt modüle taşıyıp buradan yeniden dışa aktarın."
        ));
    }
}

/// Test dosyaları boyut sınırından muaftır (genişlik ve `mod.rs` kuralı yine geçerlidir).
pub fn is_test_file(rel: &str) -> bool {
    rel.contains("/tests/")
        || rel.contains("/benches/")
        || rel.ends_with("/tests.rs")
        || rel.ends_with("_tests.rs")
}

fn rust_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = std::fs::read_dir(&current)
            .with_context(|| format!("okunamadı: {}", current.display()))?;
        for entry in entries {
            let path = entry?.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n != "target") {
                    stack.push(path);
                }
            } else if path.extension().is_some_and(|e| e == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

#[cfg(test)]
#[path = "size_tests.rs"]
mod tests;
