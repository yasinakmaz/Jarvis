//! `cargo xtask verify --expect-red <test>`: testin gerçekten başarısız olduğunu kanıtlar
//! (Mimari §9 Katman 4, adım 3). Hiç kırmızı görmemiş test kabul edilmez.

use std::path::Path;
use std::process::ExitCode;

use anyhow::bail;

use crate::cmd;

/// nextest çıkış kodları: 100 = test başarısız, 4 = eşleşen test yok.
const NEXTEST_TEST_FAILED: i32 = 100;
const NEXTEST_NO_TESTS: i32 = 4;

pub fn run(root: &Path, test: &str) -> anyhow::Result<ExitCode> {
    cmd::require_tools(root, &["nextest"])?;
    let filter = format!("test(={test})");
    let code = cmd::status(cmd::cargo(root).args([
        "nextest",
        "run",
        "--workspace",
        "--no-fail-fast",
        "-E",
        &filter,
    ]))?;
    interpret(test, code)?;
    eprintln!("✔ `{test}` kırmızı: test gerçekten başarısız oluyor. Şimdi kodu yazın.");
    Ok(ExitCode::SUCCESS)
}

/// nextest çıkış kodunu yorumlar; yalnızca gerçek test başarısızlığı "kırmızı" sayılır.
pub fn interpret(test: &str, code: i32) -> anyhow::Result<()> {
    match code {
        NEXTEST_TEST_FAILED => Ok(()),
        0 => bail!(
            "`{test}` GEÇTİ. Kod yazılmadan geçen test bir şey ölçmüyor; testi davranışı \
             sınayacak şekilde düzeltin."
        ),
        NEXTEST_NO_TESTS => bail!("`{test}` adlı test bulunamadı (tam yol: modül::test_adı)."),
        other => bail!(
            "nextest {other} koduyla çıktı (derleme hatası?). Derleme hatası kırmızı sayılmaz: \
             yanlış değer döndüren bir taslak imza yazıp testi derlenir hâle getirin."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_real_test_failure_counts_as_red() {
        assert!(interpret("t", NEXTEST_TEST_FAILED).is_ok());
        assert!(interpret("t", 0).unwrap_err().to_string().contains("GEÇTİ"));
        assert!(
            interpret("t", NEXTEST_NO_TESTS)
                .unwrap_err()
                .to_string()
                .contains("bulunamadı")
        );
        assert!(
            interpret("t", 101)
                .unwrap_err()
                .to_string()
                .contains("Derleme hatası")
        );
    }
}
