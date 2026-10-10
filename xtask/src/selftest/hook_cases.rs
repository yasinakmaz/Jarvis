//! `PreToolUse` yazma engelinin uçtan uca sınanması (gerçek betik, gerçek çıkış kodu).

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::Context as _;
use serde_json::json;

/// (açıklama, hook girdisi, engellenmeli mi)
fn cases(root: &Path) -> Vec<(&'static str, serde_json::Value, bool)> {
    let abs = |rel: &str| root.join(rel).to_string_lossy().into_owned();
    let write =
        |rel: &str| json!({ "tool_name": "Write", "tool_input": { "file_path": abs(rel) } });
    let bash = |command: &str| json!({ "tool_name": "Bash", "tool_input": { "command": command } });
    vec![
        ("Write clippy.toml", write("clippy.toml"), true),
        ("Write architecture.toml", write("architecture.toml"), true),
        ("Write xtask/src/main.rs", write("xtask/src/main.rs"), true),
        (
            "Write .github/workflows/ci.yml",
            write(".github/workflows/ci.yml"),
            true,
        ),
        (
            "Write .claude/settings.json",
            write(".claude/settings.json"),
            true,
        ),
        (
            "Write supply-chain/allowlist.toml",
            write("supply-chain/allowlist.toml"),
            true,
        ),
        ("Edit ../ ile kaçış", write("crates/../clippy.toml"), true),
        (
            "Bash sed -i clippy.toml",
            bash("sed -i 's/60/600/' clippy.toml"),
            true,
        ),
        ("Bash > deny.toml", bash("echo '' > deny.toml"), true),
        ("Bash rm -rf xtask", bash("rm -rf xtask"), true),
        (
            "Write crate kaynağı",
            write("crates/jarvis-types/src/lib.rs"),
            false,
        ),
        ("Write kök Cargo.toml", write("Cargo.toml"), false),
        (
            "Bash cargo xtask verify > log",
            bash("cargo xtask verify > /tmp/log 2>&1"),
            false,
        ),
        ("Bash cat clippy.toml", bash("cat clippy.toml"), false),
    ]
}

/// Engelleme beklentisini karşılamayan durumların adlarını döner.
pub fn run(root: &Path) -> anyhow::Result<Vec<&'static str>> {
    let script = root.join(".claude/hooks/pre-tool-use.sh");
    let mut failures = Vec::new();
    for (name, input, expect_block) in cases(root) {
        let code = invoke(root, &script, &input)?;
        let blocked = code == Some(2);
        if blocked == expect_block {
            eprintln!(
                "✔ hook: {name} → {}",
                if blocked {
                    "engellendi"
                } else {
                    "izin verildi"
                }
            );
        } else {
            eprintln!("✘ hook: {name} → beklenmeyen çıkış {code:?}");
            failures.push(name);
        }
    }
    Ok(failures)
}

fn invoke(root: &Path, script: &Path, input: &serde_json::Value) -> anyhow::Result<Option<i32>> {
    let mut child = Command::new(script)
        .env("CLAUDE_PROJECT_DIR", root)
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("çalıştırılamadı: {}", script.display()))?;
    child
        .stdin
        .take()
        .context("stdin yok")?
        .write_all(input.to_string().as_bytes())?;
    Ok(child.wait()?.code())
}
