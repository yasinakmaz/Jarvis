//! Zaman damgaları ve maskeleme (Tasarım 0003 test planı).

use std::time::Duration;

use jarvis_types::{MASK, TimeError, Timestamp, redact};
use proptest::prelude::*;

/// 9999-12-31T23:59:59.999Z (desteklenen en son an), elle hesaplanmış.
const MAX_MILLIS: i64 = 253_402_300_799_999;
/// 0000-01-01T00:00:00Z.
const MIN_MILLIS: i64 = -62_167_219_200_000;

#[test]
fn epoch_formats_as_rfc3339_utc() {
    let ts = Timestamp::from_unix_millis(0).unwrap();
    assert_eq!(ts.to_rfc3339().unwrap(), "1970-01-01T00:00:00Z");
    assert_eq!(ts.to_string(), "1970-01-01T00:00:00Z");
    assert_eq!(ts.unix_millis(), 0);
}

#[test]
fn known_instant() {
    // 2026-10-09T12:00:00Z = 20 735 gün × 86 400 000 + 12 sa.
    let ts = Timestamp::parse_rfc3339("2026-10-09T12:00:00Z").unwrap();
    assert_eq!(ts.unix_millis(), 1_791_547_200_000);
}

#[test]
fn offsets_are_converted_to_utc() {
    let local = Timestamp::parse_rfc3339("2026-10-09T15:00:00+03:00").unwrap();
    let utc = Timestamp::parse_rfc3339("2026-10-09T12:00:00Z").unwrap();
    assert_eq!(local, utc);
    assert_eq!(local.to_rfc3339().unwrap(), "2026-10-09T12:00:00Z");
}

#[test]
fn invalid_text_is_rejected() {
    assert_eq!(
        Timestamp::parse_rfc3339("dün"),
        Err(TimeError::Parse("dün".to_owned()))
    );
    assert!(serde_json::from_str::<Timestamp>("\"2026-13-01T00:00:00Z\"").is_err());
}

#[test]
fn range_is_years_0_to_9999() {
    assert!(Timestamp::from_unix_millis(MAX_MILLIS).is_ok());
    assert_eq!(
        Timestamp::from_unix_millis(MAX_MILLIS + 1),
        Err(TimeError::OutOfRange)
    );
    assert!(Timestamp::from_unix_millis(MIN_MILLIS).is_ok());
    assert_eq!(
        Timestamp::from_unix_millis(MIN_MILLIS - 1),
        Err(TimeError::OutOfRange)
    );
    assert_eq!(
        Timestamp::from_unix_millis(i64::MIN),
        Err(TimeError::OutOfRange)
    );
}

#[test]
fn checked_add_respects_range() {
    let ts = Timestamp::from_unix_millis(1_000).unwrap();
    let later = ts.checked_add(Duration::from_millis(1_500)).unwrap();
    assert_eq!(later.unix_millis(), 2_500);
    let end = Timestamp::from_unix_millis(MAX_MILLIS).unwrap();
    assert_eq!(end.checked_add(Duration::from_millis(1)), None);
    assert_eq!(ts.checked_add(Duration::MAX), None);
}

#[test]
fn duration_since_saturates_at_zero() {
    let a = Timestamp::from_unix_millis(1_000).unwrap();
    let b = Timestamp::from_unix_millis(3_250).unwrap();
    assert_eq!(b.saturating_duration_since(a), Duration::from_millis(2_250));
    assert_eq!(a.saturating_duration_since(b), Duration::ZERO);
    let start = Timestamp::from_unix_millis(MIN_MILLIS).unwrap();
    let end = Timestamp::from_unix_millis(MAX_MILLIS).unwrap();
    let span = end.saturating_duration_since(start);
    assert_eq!(
        span.as_millis(),
        u128::try_from(MAX_MILLIS - MIN_MILLIS).unwrap()
    );
}

proptest! {
    #[test]
    fn millis_and_serde_round_trip(millis in MIN_MILLIS..=MAX_MILLIS) {
        let ts = Timestamp::from_unix_millis(millis).unwrap();
        prop_assert_eq!(ts.unix_millis(), millis);
        let json = serde_json::to_string(&ts).unwrap();
        let back: Timestamp = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(back, ts);
    }
}

#[test]
fn secrets_are_masked_everywhere() {
    let text = "anahtar=nvapi-123 tekrar nvapi-123";
    assert_eq!(
        redact(text, &["nvapi-123"]),
        format!("anahtar={MASK} tekrar {MASK}")
    );
}

#[test]
fn empty_secrets_are_ignored() {
    assert_eq!(redact("değişmez", &["", "yok"]), "değişmez");
}

#[test]
fn bearer_tokens_are_masked_case_insensitively() {
    assert_eq!(
        redact("Authorization: Bearer abc.def-ghi son", &[]),
        format!("Authorization: Bearer {MASK} son")
    );
    assert_eq!(
        redact("bearer x1\nBEARER y2", &[]),
        format!("bearer {MASK}\nBEARER {MASK}")
    );
}

#[test]
fn bearer_without_token_is_left_alone() {
    assert_eq!(redact("Bearer ", &[]), "Bearer ");
    assert_eq!(redact("çöz Bearer  ", &[]), "çöz Bearer  ");
}

#[test]
fn non_ascii_text_survives_masking() {
    assert_eq!(
        redact("İstanbul ğüşıöç Bearer t0k şifre", &["şifre"]),
        format!("İstanbul ğüşıöç Bearer {MASK} {MASK}")
    );
}

proptest! {
    #[test]
    fn redaction_never_leaks_the_secret(
        prefix in "[a-zA-Zğüş ]{0,20}",
        secret in "[a-z0-9]{6,16}",
        suffix in "[a-zA-Zğüş ]{0,20}",
    ) {
        let text = format!("{prefix}{secret}{suffix}");
        let out = redact(&text, &[&secret]);
        prop_assert!(!out.contains(&secret));
    }
}
