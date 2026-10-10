//! Jarvis daemon'u. Bileşenler M1'de bağlanır (docs/architecture.md §3, §13).

use std::io::Write as _;
use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        let mut out = std::io::stdout().lock();
        return match writeln!(out, "jarvisd {}", env!("CARGO_PKG_VERSION")) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        };
    }
    // Sessiz başarı yok (Mimari §1 kural 4): henüz işlev yokken açıkça başarısız ol.
    let mut err = std::io::stderr().lock();
    let _ = writeln!(
        err,
        "jarvisd: henüz uygulanmadı; bkz. docs/architecture.md §13"
    );
    ExitCode::FAILURE
}
