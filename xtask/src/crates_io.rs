//! `cargo xtask crate-info <ad>...`: yeni bir bağımlılığın crates.io'da gerçekten var
//! olduğunu, yaşını ve indirme sayısını gösterir (Mimari §9 Katman 4).
//!
//! LLM'ler var olmayan ya da yazım hatasıyla taklit edilen paketler önerebilir; bu komut
//! insan onayından önce kanıt üretir. Ağ erişimi için `curl` kullanılır.

use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, bail};
use serde::Deserialize;

use crate::cmd;

/// Bu eşiklerin altındaki crate'ler ek inceleme ister (başlangıç değerleri).
const MIN_AGE_DAYS: i64 = 180;
const MIN_DOWNLOADS: u64 = 100_000;

#[derive(Debug, Deserialize)]
struct Response {
    #[serde(rename = "crate")]
    krate: CrateInfo,
}

#[derive(Debug, Deserialize)]
struct CrateInfo {
    name: String,
    created_at: String,
    downloads: u64,
    recent_downloads: Option<u64>,
    max_stable_version: Option<String>,
    repository: Option<String>,
}

pub fn main(names: &[String]) -> anyhow::Result<ExitCode> {
    if names.is_empty() {
        bail!("kullanım: cargo xtask crate-info <crate>...");
    }
    let today = days_since_epoch_now()?;
    let mut ok = true;
    for name in names {
        match fetch(name) {
            Ok(info) => ok &= report(&info, today),
            Err(err) => {
                ok = false;
                eprintln!("✘ {name}: crates.io'da bulunamadı veya sorgulanamadı ({err:#}).");
                eprintln!("  Var olmayan bir paket olabilir (halüsinasyon); adı doğrulayın.");
            }
        }
    }
    Ok(if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn fetch(name: &str) -> anyhow::Result<CrateInfo> {
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!("geçersiz crate adı");
    }
    let url = format!("https://crates.io/api/v1/crates/{name}");
    let body = cmd::output(Command::new("curl").args([
        "--silent",
        "--show-error",
        "--fail",
        "--user-agent",
        "jarvis-xtask (https://github.com/yasinakmaz/Jarvis)",
        &url,
    ]))?;
    let response: Response = serde_json::from_str(&body).context("yanıt ayrıştırılamadı")?;
    Ok(response.krate)
}

/// Bilgiyi yazdırır; eşikler karşılanıyorsa `true` döner.
fn report(info: &CrateInfo, today: i64) -> bool {
    let age = parse_date(&info.created_at).map(|created| today.saturating_sub(created));
    let young = age.is_none_or(|days| days < MIN_AGE_DAYS);
    let rare = info.downloads < MIN_DOWNLOADS;
    let mark = if young || rare { "⚠" } else { "✔" };
    eprintln!("{mark} {}", info.name);
    eprintln!(
        "  son kararlı sürüm : {}",
        info.max_stable_version.as_deref().unwrap_or("-")
    );
    eprintln!(
        "  oluşturulma       : {} ({} gün)",
        info.created_at,
        fmt_opt(age)
    );
    eprintln!("  toplam indirme    : {}", info.downloads);
    eprintln!("  son 90 gün        : {}", fmt_opt(info.recent_downloads));
    eprintln!(
        "  depo              : {}",
        info.repository.as_deref().unwrap_or("-")
    );
    if young {
        eprintln!("  DİKKAT: {MIN_AGE_DAYS} günden genç; bakımcıyı ve kaynak kodunu inceleyin.");
    }
    if rare {
        eprintln!("  DİKKAT: {MIN_DOWNLOADS} indirmenin altında; yazım taklidi olabilir.");
    }
    !(young || rare)
}

fn fmt_opt<T: std::fmt::Display>(value: Option<T>) -> String {
    value.map_or_else(|| "-".to_owned(), |v| v.to_string())
}

fn days_since_epoch_now() -> anyhow::Result<i64> {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    i64::try_from(secs / 86_400).context("tarih taşması")
}

/// `YYYY-MM-DD...` biçimindeki tarihi Unix gününe çevirir.
pub fn parse_date(text: &str) -> Option<i64> {
    let mut parts = text.get(..10)?.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    days_from_civil(year, month, day)
}

/// Howard Hinnant'ın `days_from_civil` algoritması (proleptik Gregoryen takvim).
fn days_from_civil(year: i64, month: i64, day: i64) -> Option<i64> {
    let y = if month <= 2 {
        year.checked_sub(1)?
    } else {
        year
    };
    let era = y.checked_div_euclid(400)?;
    let yoe = y.checked_sub(era.checked_mul(400)?)?;
    let shifted = if month > 2 {
        month.checked_sub(3)?
    } else {
        month.checked_add(9)?
    };
    let doy = shifted
        .checked_mul(153)?
        .checked_add(2)?
        .checked_div(5)?
        .checked_add(day)?;
    let doy = doy.checked_sub(1)?;
    let leap = yoe.checked_div(4)?.checked_sub(yoe.checked_div(100)?)?;
    let doe = yoe.checked_mul(365)?.checked_add(leap)?.checked_add(doy)?;
    era.checked_mul(146_097)?
        .checked_add(doe)?
        .checked_sub(719_468)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_dates() {
        assert_eq!(parse_date("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_date("2000-03-01"), Some(11_017));
        assert_eq!(parse_date("2026-10-09T12:00:00.000+00:00"), Some(20_735));
        assert_eq!(parse_date("2024-02-29"), Some(19_782));
    }

    #[test]
    fn rejects_malformed_dates() {
        assert_eq!(parse_date("2026-13-01"), None);
        assert_eq!(parse_date("2026-00-01"), None);
        assert_eq!(parse_date("26-1-1"), None);
        assert_eq!(parse_date(""), None);
    }

    fn info(created_at: &str, downloads: u64) -> CrateInfo {
        CrateInfo {
            name: "x".into(),
            created_at: created_at.into(),
            downloads,
            recent_downloads: None,
            max_stable_version: None,
            repository: None,
        }
    }

    #[test]
    fn thresholds_flag_young_or_rare_crates() {
        let today = parse_date("2026-10-09").unwrap();
        assert!(report(&info("2020-01-01", MIN_DOWNLOADS), today));
        assert!(!report(&info("2026-09-01", 10_000_000), today));
        assert!(!report(&info("2020-01-01", MIN_DOWNLOADS - 1), today));
        assert!(!report(&info("bozuk", 10_000_000), today));
    }
}
