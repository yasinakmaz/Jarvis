//! Bilerek bozulmuş değişiklikler ve onları yakalaması gereken kapılar.

use std::fmt::Write as _;
use std::path::Path;

use anyhow::Context as _;

use super::{Case, Gate};

const TYPES_LIB: &str = "crates/jarvis-types/src/lib.rs";
const TYPES_MANIFEST: &str = "crates/jarvis-types/Cargo.toml";

type Sabotage = fn(&Path) -> anyhow::Result<()>;

const CASES: &[(&str, Gate, Sabotage)] = &[
    ("uzun-dosya", Gate::Size, long_file),
    ("uzun-satir", Gate::Size, long_line),
    ("mantikli-mod-rs", Gate::Size, logic_in_mod_rs),
    ("unwrap", Gate::Clippy, unwrap),
    ("allow-ile-susturma", Gate::Clippy, allow_attribute),
    ("unsafe", Gate::Clippy, unsafe_code),
    ("panic", Gate::Clippy, panic),
    ("yukari-bagimlilik", Gate::Arch, upward_dependency),
    ("cekirdekte-http", Gate::Arch, http_in_core),
    ("tanimsiz-crate", Gate::Arch, unlisted_crate),
    ("beyaz-liste-disi", Gate::Deps, unlisted_dependency),
    ("lint-tablosu", Gate::Hygiene, weakened_lints),
    ("crate-lint-mirasi", Gate::Hygiene, dropped_lint_inheritance),
];

pub fn all() -> Vec<Case> {
    CASES
        .iter()
        .map(|&(name, gate, sabotage)| Case {
            name,
            gate,
            sabotage,
        })
        .collect()
}

fn append(root: &Path, rel: &str, text: &str) -> anyhow::Result<()> {
    let path = root.join(rel);
    let mut content = std::fs::read_to_string(&path).with_context(|| rel.to_owned())?;
    content.push_str(text);
    std::fs::write(path, content)?;
    Ok(())
}

/// Manifeste bir bağımlılık satırı ekler: `[dependencies]` tablosu varsa içine, yoksa yeni
/// tablo açarak. (İkinci bir `[dependencies]` başlığı geçersiz TOML olur ve kapı yerine
/// `cargo metadata` hata verirdi.)
fn add_dependency(root: &Path, rel: &str, line: &str) -> anyhow::Result<()> {
    let path = root.join(rel);
    let content = std::fs::read_to_string(&path).with_context(|| rel.to_owned())?;
    let header = "\n[dependencies]\n";
    let updated = if content.contains(header) {
        content.replacen(header, &format!("{header}{line}\n"), 1)
    } else {
        format!("{content}{header}{line}\n")
    };
    std::fs::write(path, updated)?;
    Ok(())
}

fn replace(root: &Path, rel: &str, from: &str, to: &str) -> anyhow::Result<()> {
    let path = root.join(rel);
    let content = std::fs::read_to_string(&path).with_context(|| rel.to_owned())?;
    anyhow::ensure!(content.contains(from), "{rel}: `{from}` bulunamadı");
    std::fs::write(path, content.replace(from, to))?;
    Ok(())
}

fn long_file(root: &Path) -> anyhow::Result<()> {
    let mut text = String::from("//! Uzun.\n");
    for i in 0..320 {
        writeln!(text, "/// Sabit.\npub const C{i}: u32 = {i};")?;
    }
    std::fs::write(root.join("crates/jarvis-types/src/long.rs"), text)?;
    append(root, TYPES_LIB, "pub mod long;\n")
}

fn long_line(root: &Path) -> anyhow::Result<()> {
    append(root, TYPES_LIB, &format!("// {}\n", "x".repeat(120)))
}

fn logic_in_mod_rs(root: &Path) -> anyhow::Result<()> {
    let dir = root.join("crates/jarvis-types/src/m");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("mod.rs"), "//! M.\n/// F.\npub fn f() {}\n")?;
    append(root, TYPES_LIB, "pub mod m;\n")
}

fn unwrap(root: &Path) -> anyhow::Result<()> {
    append(
        root,
        TYPES_LIB,
        "/// İlk.\n#[must_use]\npub fn first(v: Option<u8>) -> u8 {\n    v.unwrap()\n}\n",
    )
}

fn allow_attribute(root: &Path) -> anyhow::Result<()> {
    append(
        root,
        TYPES_LIB,
        "/// İlk.\n#[allow(clippy::unwrap_used)]\n#[must_use]\npub fn first(v: Option<u8>) -> u8 \
         {\n    v.unwrap()\n}\n",
    )
}

fn unsafe_code(root: &Path) -> anyhow::Result<()> {
    append(
        root,
        TYPES_LIB,
        "/// Sıfır.\n#[must_use]\npub fn zero() -> u8 {\n    // SAFETY: test.\n    \
         unsafe { core::mem::zeroed() }\n}\n",
    )
}

fn panic(root: &Path) -> anyhow::Result<()> {
    append(
        root,
        TYPES_LIB,
        "/// Patla.\npub fn boom() {\n    panic!(\"x\");\n}\n",
    )
}

fn upward_dependency(root: &Path) -> anyhow::Result<()> {
    add_dependency(root, TYPES_MANIFEST, "jarvis-config = { path = \"../jarvis-config\" }")
}

fn http_in_core(root: &Path) -> anyhow::Result<()> {
    add_dependency(root, "crates/jarvis-core/Cargo.toml", "axum = \"0.8\"")
}

fn unlisted_crate(root: &Path) -> anyhow::Result<()> {
    let dir = root.join("crates/jarvis-rogue");
    std::fs::create_dir_all(dir.join("src"))?;
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"jarvis-rogue\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
         publish = false\n\n[lints]\nworkspace = true\n",
    )?;
    std::fs::write(dir.join("src/lib.rs"), "//! Kaçak.\n")?;
    Ok(())
}

fn unlisted_dependency(root: &Path) -> anyhow::Result<()> {
    add_dependency(root, TYPES_MANIFEST, "randd = \"0.8\"")
}

fn weakened_lints(root: &Path) -> anyhow::Result<()> {
    replace(
        root,
        "Cargo.toml",
        "unwrap_used = \"deny\"",
        "unwrap_used = \"allow\"",
    )
}

fn dropped_lint_inheritance(root: &Path) -> anyhow::Result<()> {
    replace(root, TYPES_MANIFEST, "[lints]\nworkspace = true\n", "")
}
