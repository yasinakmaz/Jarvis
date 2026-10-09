//! Claude Code hook giriş noktaları (Mimari §9 Katman 5).
//!
//! * `post-tool-use`: Rust/TOML dosyası yazıldıktan sonra hızlı kapılar; kırmızıysa
//!   çıktı yapay zekaya geri beslenir.
//! * `stop`: tam `verify`; kırmızıysa yapay zekanın "bitti" demesine izin verilmez.
//!
//! Yazma engeli (`PreToolUse`) cargo derlemesine bağımlı olmasın diye
//! `.claude/hooks/pre_tool_use.py` içindedir. Çıkış kodu 2 = engelle/geri besle.

use std::io::Read as _;
use std::process::ExitCode;

use anyhow::{Context as _, bail};
use serde_json::Value;

use crate::verify::{self, Options};
use crate::workspace;

/// Claude Code'un "engelle ve stderr'i modele göster" çıkış kodu.
const BLOCK: u8 = 2;

pub fn main(args: &[String]) -> anyhow::Result<ExitCode> {
    let input = read_input()?;
    match args.first().map(String::as_str) {
        Some("post-tool-use") => Ok(post_tool_use(&input)),
        Some("stop") => Ok(stop()),
        _ => bail!("kullanım: cargo xtask hook <post-tool-use|stop>"),
    }
}

fn read_input() -> anyhow::Result<Value> {
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .context("hook girdisi okunamadı")?;
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).context("hook girdisi JSON değil")
}

/// Yalnızca kaynak/yapılandırma yazımları hızlı kapıları tetikler.
pub fn needs_fast_check(input: &Value) -> bool {
    let tool = input
        .get("tool_name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let path = input
        .pointer("/tool_input/file_path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    matches!(tool, "Write" | "Edit" | "MultiEdit")
        && (workspace::has_extension(path, "rs") || workspace::has_extension(path, "toml"))
}

fn post_tool_use(input: &Value) -> ExitCode {
    if !needs_fast_check(input) {
        return ExitCode::SUCCESS;
    }
    let options = Options {
        fast: true,
        ..Options::default()
    };
    match verify::run(&workspace::root(), &options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("hızlı kapılar kırmızı: {err:#}\nBir sonraki adımdan önce düzeltin.");
            ExitCode::from(BLOCK)
        }
    }
}

fn stop() -> ExitCode {
    let options = Options {
        cache: true,
        ..Options::default()
    };
    match verify::run(&workspace::root(), &options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!(
                "`cargo xtask verify` kırmızı: {err:#}\nKapılar geçmeden iş bitmiş sayılmaz \
                 (Mimari §9). Hatayı düzeltin; kapı yapılandırmasını değiştirmeyin. Çözülemiyorsa \
                 engeli açıkça kullanıcıya bildirin."
            );
            ExitCode::from(BLOCK)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(tool: &str, path: &str) -> Value {
        serde_json::json!({ "tool_name": tool, "tool_input": { "file_path": path } })
    }

    #[test]
    fn only_rust_and_toml_writes_trigger_fast_gates() {
        assert!(needs_fast_check(&input("Write", "/w/crates/a/src/lib.rs")));
        assert!(needs_fast_check(&input("Edit", "/w/Cargo.toml")));
        assert!(needs_fast_check(&input("MultiEdit", "/w/x.rs")));
        assert!(!needs_fast_check(&input("Write", "/w/docs/a.md")));
        assert!(!needs_fast_check(&input("Read", "/w/x.rs")));
        assert!(!needs_fast_check(&Value::Null));
    }
}
