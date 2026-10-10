//! Jarvis kalite kapıları (Mimari §9).
//!
//! `cargo xtask verify` geçmeden hiçbir iş bitmiş sayılmaz. Bu araç ve yapılandırma
//! dosyaları yalnızca insan tarafından değiştirilir; yapay zeka hook'larla engellenir.

mod arch;
mod cmd;
mod coverage;
mod crates_io;
mod deps;
mod diff;
mod expect_red;
mod hook;
mod hygiene;
mod metadata;
mod mutants;
mod report;
mod selftest;
mod size;
mod verify;
mod workspace;

use std::process::ExitCode;

const USAGE: &str = "\
kullanım: cargo xtask <komut>

komutlar:
  verify [--fast] [--base <ref>|root]   Tüm kalite kapıları (ilk hatada durur)
  verify --expect-red <test-adı>        Testin gerçekten kırmızı olduğunu kanıtla
  selftest                              Kapıların bozuk kodu reddettiğini kanıtla
  crate-info <crate>...                 crates.io'da varlık, yaş ve indirme denetimi
  hook <post-tool-use|stop>             Claude Code hook giriş noktaları";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("xtask: hata: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> anyhow::Result<ExitCode> {
    let Some((command, rest)) = args.split_first() else {
        eprintln!("{USAGE}");
        return Ok(ExitCode::FAILURE);
    };
    match command.as_str() {
        "verify" => verify::main(rest),
        "selftest" => selftest::main(rest),
        "crate-info" => crates_io::main(rest),
        "hook" => hook::main(rest),
        "help" | "--help" | "-h" => {
            eprintln!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        other => {
            eprintln!("bilinmeyen komut: {other}\n\n{USAGE}");
            Ok(ExitCode::FAILURE)
        }
    }
}
