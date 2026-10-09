//! Jarvis terminal istemcisi. HTTP istemcisi M2'de eklenir (docs/architecture.md §13).

use std::io::Write as _;
use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        let mut out = std::io::stdout().lock();
        return match writeln!(out, "jarvis {}", env!("CARGO_PKG_VERSION")) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        };
    }
    // Sessiz başarı yok (Mimari §1 kural 4): henüz işlev yokken açıkça başarısız ol.
    let mut err = std::io::stderr().lock();
    let _ = writeln!(
        err,
        "jarvis: henüz uygulanmadı; bkz. docs/architecture.md §13"
    );
    ExitCode::FAILURE
}
