//! Alt süreç yardımcıları.

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context as _, bail};

/// Kapıların ihtiyaç duyduğu cargo alt komutları ve kurulum ipucu.
pub const REQUIRED_TOOLS: &[&str] = &["nextest", "deny", "llvm-cov", "mutants", "machete", "hack"];

const INSTALL_HINT: &str = "kurulum: cargo binstall cargo-nextest cargo-deny cargo-llvm-cov \
                            cargo-mutants cargo-machete cargo-hack";

/// `root` dizininde çalışacak bir `cargo` komutu hazırlar.
pub fn cargo(root: &Path) -> Command {
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command.current_dir(root);
    command
}

/// `root` dizininde çalışacak bir `git` komutu hazırlar.
pub fn git(root: &Path) -> Command {
    let mut command = Command::new("git");
    command.current_dir(root);
    command
}

/// Komutu çalıştırır (çıktı terminale akar) ve çıkış kodunu döner.
pub fn status(command: &mut Command) -> anyhow::Result<i32> {
    let status = command
        .status()
        .with_context(|| format!("çalıştırılamadı: {command:?}"))?;
    status
        .code()
        .with_context(|| format!("sinyalle sonlandı: {command:?}"))
}

/// Komutu çalıştırır; sıfır dışı çıkışta hata döner.
pub fn run(command: &mut Command) -> anyhow::Result<()> {
    let code = status(command)?;
    if code != 0 {
        bail!("komut {code} koduyla başarısız: {command:?}");
    }
    Ok(())
}

/// Komutu çalıştırıp standart çıktısını döner; sıfır dışı çıkışta stderr ile hata döner.
pub fn output(command: &mut Command) -> anyhow::Result<String> {
    let out = command
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("çalıştırılamadı: {command:?}"))?;
    if !out.status.success() {
        bail!(
            "komut başarısız ({}): {command:?}\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    String::from_utf8(out.stdout).context("komut çıktısı UTF-8 değil")
}

/// Gerekli cargo alt komutlarının kurulu olduğunu doğrular; eksikse açıkça başarısız olur.
pub fn require_tools(root: &Path, tools: &[&str]) -> anyhow::Result<()> {
    let missing: Vec<&str> = tools
        .iter()
        .copied()
        .filter(|tool| {
            !cargo(root)
                .args([tool, "--version"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|s| s.success())
        })
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    bail!(
        "eksik araç(lar): cargo-{}\n{INSTALL_HINT}",
        missing.join(", cargo-")
    )
}
