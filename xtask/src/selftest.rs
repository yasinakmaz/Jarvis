//! `cargo xtask selftest`: M-1 "bitti" ölçütünün mekanik kanıtı (Mimari §13).
//!
//! Çalışma alanının geçici kopyalarına bilerek bozuk değişiklikler uygulanır ve ilgili
//! kapının her birini reddettiği doğrulanır. Ayrıca yazma engeli hook'u sınanır.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, bail};

use crate::metadata::Metadata;
use crate::{arch, cmd, deps, hygiene, size, workspace};

mod cases;
mod hook_cases;

/// Bozulmanın yakalanması beklenen kapı.
#[derive(Debug, Clone, Copy)]
pub enum Gate {
    Size,
    Arch,
    Hygiene,
    Deps,
    Clippy,
}

pub struct Case {
    pub name: &'static str,
    pub gate: Gate,
    pub sabotage: fn(&Path) -> anyhow::Result<()>,
}

/// Kopyalanan çalışma alanı girdileri.
const COPY: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "architecture.toml",
    "clippy.toml",
    "rustfmt.toml",
    "rust-toolchain.toml",
    "crates",
    "xtask",
    "supply-chain",
    "docs/adr",
];

pub fn main(_args: &[String]) -> anyhow::Result<ExitCode> {
    let root = workspace::root();
    let base = workspace::scratch(&root).join("selftest");
    let mut failures = Vec::new();
    control(&root, &base).context("kontrol kopyası (bozulmasız) temiz değil")?;
    eprintln!("✔ kontrol: bozulmasız kopya tüm kapılardan geçiyor");
    for case in cases::all() {
        match run_case(&root, &base, &case) {
            Ok(()) => eprintln!("✔ {} → {:?} kapısı reddetti", case.name, case.gate),
            Err(err) => {
                eprintln!("✘ {}: {err:#}", case.name);
                failures.push(case.name);
            }
        }
    }
    failures.extend(hook_cases::run(&root)?);
    if failures.is_empty() {
        eprintln!("✔ selftest: her bozulma reddedildi, yazma engeli çalışıyor");
        return Ok(ExitCode::SUCCESS);
    }
    eprintln!(
        "✘ selftest: yakalanmayan bozulmalar: {}",
        failures.join(", ")
    );
    Ok(ExitCode::FAILURE)
}

fn control(root: &Path, base: &Path) -> anyhow::Result<()> {
    let copy = fresh_copy(root, base, "control")?;
    for gate in [
        Gate::Size,
        Gate::Arch,
        Gate::Hygiene,
        Gate::Deps,
        Gate::Clippy,
    ] {
        if !gate_passes(root, &copy, gate)? {
            bail!("{gate:?} kapısı bozulmasız kopyada kırmızı");
        }
    }
    Ok(())
}

fn run_case(root: &Path, base: &Path, case: &Case) -> anyhow::Result<()> {
    let copy = fresh_copy(root, base, case.name)?;
    (case.sabotage)(&copy).context("bozulma uygulanamadı")?;
    if gate_passes(root, &copy, case.gate)? {
        bail!("{:?} kapısı bozulmayı YAKALAMADI", case.gate);
    }
    Ok(())
}

/// Kapıyı kopya üzerinde çalıştırır; yeşilse `true`.
fn gate_passes(root: &Path, copy: &Path, gate: Gate) -> anyhow::Result<bool> {
    let findings = match gate {
        Gate::Size => size::check(copy)?,
        Gate::Arch => arch::check(copy, &Metadata::load(copy)?)?,
        Gate::Hygiene => hygiene::check(copy, &Metadata::load(copy)?)?,
        Gate::Deps => deps::check(copy, &Metadata::load(copy)?)?,
        Gate::Clippy => return clippy_passes(root, copy),
    };
    Ok(findings.errors.is_empty())
}

fn clippy_passes(root: &Path, copy: &Path) -> anyhow::Result<bool> {
    let target = workspace::scratch(root).join("selftest-target");
    let code = cmd::status(
        cmd::cargo(copy)
            .env("CARGO_TARGET_DIR", target)
            .args([
                "clippy",
                "--quiet",
                "--offline",
                "--package",
                "jarvis-types",
            ])
            .args(["--all-targets", "--", "-D", "warnings"]),
    )?;
    Ok(code == 0)
}

fn fresh_copy(root: &Path, base: &Path, name: &str) -> anyhow::Result<PathBuf> {
    let dest = base.join(name);
    if dest.exists() {
        std::fs::remove_dir_all(&dest)?;
    }
    for entry in COPY {
        copy_recursive(&root.join(entry), &dest.join(entry))?;
    }
    Ok(dest)
}

fn copy_recursive(from: &Path, to: &Path) -> anyhow::Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            if entry.file_name() != "target" {
                copy_recursive(&entry.path(), &to.join(entry.file_name()))?;
            }
        }
    } else {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(from, to).with_context(|| format!("kopyalanamadı: {}", from.display()))?;
    }
    Ok(())
}
