//! `jarvis` ikilisinin süreç düzeyi davranışı (L5a: gerçek süreç, bağımsız gözlemci = çıkış kodu
//! ve çıktı akışları).

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_jarvis"))
}

#[test]
fn version_flag_prints_package_version() {
    let out = bin().arg("--version").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout, format!("jarvis {}\n", env!("CARGO_PKG_VERSION")));
}

#[test]
fn unimplemented_entry_point_fails_loudly() {
    let out = bin().output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("henüz uygulanmadı"), "{stderr}");
}
