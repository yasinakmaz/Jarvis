//! Değişen kodda mutasyon testi (`cargo-mutants --in-diff`): testlerin gerçekten bir şey
//! ölçtüğünü kanıtlar (Mimari §9 Katman 4). Tam koşu gecelik CI'dadır.

use std::path::Path;

use anyhow::bail;

use crate::report::Findings;
use crate::{cmd, workspace};

/// Diff yalnızca `crates/` altını kapsar; boşsa adım açıkça "atlandı" der.
pub fn run(root: &Path, diff: &str) -> anyhow::Result<Findings> {
    let mut findings = Findings::default();
    if !diff
        .lines()
        .any(|l| l.starts_with("+++ b/crates/") && workspace::has_extension(l, "rs"))
    {
        findings.note("atlandı: crates/ altında değişen Rust dosyası yok");
        return Ok(findings);
    }
    let scratch = workspace::scratch(root);
    std::fs::create_dir_all(&scratch)?;
    let diff_path = scratch.join("mutants.diff");
    std::fs::write(&diff_path, diff)?;
    let code = cmd::status(
        cmd::cargo(root)
            .args(["mutants", "--in-diff"])
            .arg(&diff_path)
            .args(["--test-tool", "nextest", "--jobs", "2", "--output"])
            .arg(&scratch),
    )?;
    match code {
        0 => {}
        2 => findings.error(
            "yakalanmayan mutantlar var (target/xtask/mutants.out/missed.txt). Her biri için \
             davranışı ölçen bir test ekleyin; testi zayıflatmayın.",
        ),
        3 => findings.error("mutant testleri zaman aşımına uğradı (target/xtask/mutants.out)."),
        4 => findings.error("mutasyonsuz temel testler kırmızı; önce testleri düzeltin."),
        other => bail!("cargo mutants beklenmeyen çıkış kodu: {other}"),
    }
    Ok(findings)
}
