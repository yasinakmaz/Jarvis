//! Testler (nextest) kapsama ölçümüyle çalışır; kapsama yalnızca değişen satırlarda
//! zorlanır (Mimari §9 Katman 4: başlangıç tabanı %80).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::Context as _;

use crate::report::Findings;
use crate::{cmd, size, workspace};

/// Değişen satırlarda gereken en düşük kapsama yüzdesi (başlangıç değeri).
pub const MIN_PERCENT: u64 = 80;

/// Satır → isabet sayısı, dosya bazında (çalışma alanına göreli yollar).
pub type LineHits = BTreeMap<String, BTreeMap<u32, u64>>;

/// Tüm çalışma alanı testlerini kapsama ölçümüyle çalıştırır ve lcov dosyasını döner.
pub fn run_tests(root: &Path) -> anyhow::Result<PathBuf> {
    let lcov = workspace::scratch(root).join("lcov.info");
    std::fs::create_dir_all(workspace::scratch(root))?;
    cmd::run(
        cmd::cargo(root)
            .args(["llvm-cov", "nextest", "--workspace", "--locked", "--lcov"])
            .args(["--output-path".as_ref(), lcov.as_os_str()]),
    )
    .context("testler kırmızı")?;
    Ok(lcov)
}

/// lcov metnini ayrıştırır (`SF:` ve `DA:satır,isabet` kayıtları).
pub fn parse_lcov(root: &Path, text: &str) -> LineHits {
    let mut hits = LineHits::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        if let Some(file) = line.strip_prefix("SF:") {
            current = Some(workspace::relative(root, Path::new(file)));
        } else if line == "end_of_record" {
            current = None;
        } else if let (Some(file), Some(record)) = (&current, line.strip_prefix("DA:")) {
            let mut fields = record.split(',');
            let line_no = fields.next().and_then(|v| v.parse().ok());
            let count = fields.next().and_then(|v| v.parse().ok());
            if let (Some(line_no), Some(count)) = (line_no, count) {
                let entry = hits
                    .entry(file.clone())
                    .or_default()
                    .entry(line_no)
                    .or_insert(0);
                *entry = entry.saturating_add(count);
            }
        }
    }
    hits
}

/// Değişen ve ölçülebilir (lcov'da DA kaydı olan) satırların kapsamasını denetler.
pub fn check(hits: &LineHits, added: &BTreeMap<String, BTreeSet<u32>>) -> Findings {
    let mut findings = Findings::default();
    let mut total: u64 = 0;
    let mut covered: u64 = 0;
    let mut uncovered: Vec<String> = Vec::new();
    for (file, lines) in added {
        if size::is_test_file(file) || !workspace::has_extension(file, "rs") {
            continue;
        }
        let Some(file_hits) = hits.get(file) else {
            continue;
        };
        for line in lines {
            let Some(count) = file_hits.get(line) else {
                continue;
            };
            total = total.saturating_add(1);
            if *count > 0 {
                covered = covered.saturating_add(1);
            } else {
                uncovered.push(format!("{file}:{line}"));
            }
        }
    }
    if total == 0 {
        findings.note("değişen ölçülebilir satır yok");
        return findings;
    }
    let percent = covered.saturating_mul(100).checked_div(total).unwrap_or(0);
    findings.note(format!(
        "değişen satır kapsaması: %{percent} ({covered}/{total})"
    ));
    if covered.saturating_mul(100) < total.saturating_mul(MIN_PERCENT) {
        findings.error(format!(
            "değişen satırlarda kapsama %{percent} < %{MIN_PERCENT}. Şu satırları sınayan \
             testler ekleyin (testin önce kırmızı olduğunu `--expect-red` ile kanıtlayın): {}",
            uncovered.join(", ")
        ));
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    const LCOV: &str = "\
SF:/w/crates/a/src/lib.rs
DA:1,3
DA:2,0
DA:3,1
DA:3,1
end_of_record
SF:/w/crates/a/tests/t.rs
DA:1,0
end_of_record
";

    fn added(entries: &[(&str, &[u32])]) -> BTreeMap<String, BTreeSet<u32>> {
        entries
            .iter()
            .map(|(f, l)| ((*f).to_owned(), l.iter().copied().collect()))
            .collect()
    }

    #[test]
    fn parses_relative_paths_and_merges_duplicates() {
        let hits = parse_lcov(Path::new("/w"), LCOV);
        assert_eq!(hits["crates/a/src/lib.rs"][&3], 2);
        assert_eq!(hits["crates/a/src/lib.rs"][&2], 0);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn threshold_is_enforced_on_changed_lines_only() {
        let hits = parse_lcov(Path::new("/w"), LCOV);
        // 2/3 = %66 < %80 → kırmızı; ölçülemeyen satır (99) sayılmaz.
        let red = check(&hits, &added(&[("crates/a/src/lib.rs", &[1, 2, 3, 99])]));
        assert_eq!(red.errors.len(), 1);
        assert!(red.errors[0].contains("crates/a/src/lib.rs:2"));
        // Kapsanmış satırlar → yeşil; test dosyaları sayılmaz.
        let green = check(
            &hits,
            &added(&[
                ("crates/a/src/lib.rs", &[1, 3]),
                ("crates/a/tests/t.rs", &[1]),
            ]),
        );
        assert!(green.errors.is_empty(), "{:?}", green.errors);
    }

    #[test]
    fn exactly_at_threshold_passes() {
        let mut hits = LineHits::new();
        let file: BTreeMap<u32, u64> = (1..=5).map(|l| (l, u64::from(l != 5))).collect();
        hits.insert("crates/a/src/x.rs".into(), file);
        let result = check(&hits, &added(&[("crates/a/src/x.rs", &[1, 2, 3, 4, 5])]));
        assert!(result.errors.is_empty(), "4/5 = %80 geçmeli");
    }

    #[test]
    fn nothing_measurable_is_green() {
        let result = check(&LineHits::new(), &added(&[("crates/a/src/x.rs", &[1])]));
        assert!(result.errors.is_empty());
    }
}
