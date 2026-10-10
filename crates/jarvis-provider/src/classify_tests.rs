use async_openai::error::OpenAIError;

use super::{MAX_MESSAGE_CHARS, Outcome, Retry, from_openai, from_status, sanitize};
use crate::{ProviderError, RetryReason};

fn fatal(status: u16) -> ProviderError {
    ProviderError::Rejected {
        status,
        message: "kötü istek".to_owned(),
    }
}

#[test]
fn retryable_statuses_are_429_408_and_every_5xx() {
    for status in [500_u16, 502, 503, 504, 599, 408] {
        assert_eq!(
            from_status(status, "x", &[]),
            Outcome::Retry(Retry {
                reason: RetryReason::Unavailable,
                status: Some(status),
                message: "x".to_owned()
            }),
            "{status}"
        );
    }
    assert_eq!(
        from_status(429, "yavaş", &[]),
        Outcome::Retry(Retry {
            reason: RetryReason::RateLimited,
            status: Some(429),
            message: "yavaş".to_owned()
        })
    );
}

#[test]
fn other_4xx_are_rejected_without_retry() {
    for status in [400_u16, 401, 403, 404, 409, 413, 422, 451, 499] {
        assert_eq!(
            from_status(status, "kötü istek", &[]),
            Outcome::Fatal(fatal(status)),
            "{status}"
        );
    }
}

#[test]
fn unexpected_statuses_are_invalid_responses() {
    for status in [100_u16, 204, 302, 600] {
        assert!(
            matches!(
                from_status(status, "x", &[]),
                Outcome::Fatal(ProviderError::InvalidResponse(m)) if m.contains(&status.to_string())
            ),
            "{status}"
        );
    }
}

#[test]
fn messages_never_carry_the_key_or_bearer_tokens() {
    let key = "nvapi-SECRET-123";
    let text = format!("Incorrect API key provided: {key}. Authorization: Bearer {key}");
    let Outcome::Fatal(ProviderError::Rejected { message, .. }) = from_status(401, &text, &[key])
    else {
        panic!("401 reddedilmeliydi");
    };
    assert!(!message.contains(key), "{message}");
    assert!(message.contains("***"), "{message}");
}

#[test]
fn long_messages_are_truncated_on_a_char_boundary() {
    let long = "ş".repeat(MAX_MESSAGE_CHARS + 50);
    let clean = sanitize(&long, &[]);
    assert_eq!(clean.chars().count(), MAX_MESSAGE_CHARS + 1);
    assert!(clean.ends_with('…'));
    let exact = "a".repeat(MAX_MESSAGE_CHARS);
    assert_eq!(
        sanitize(&exact, &[]),
        exact,
        "tam sınırdaki metin kısaltılmaz"
    );
}

#[test]
fn library_errors_map_by_kind() {
    let invalid = from_openai(OpenAIError::InvalidArgument("model yok".to_owned()), &[]);
    assert_eq!(
        invalid,
        Outcome::Fatal(ProviderError::InvalidRequest("model yok".to_owned()))
    );

    let parse = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    let garbled = from_openai(
        OpenAIError::JSONDeserialize(parse, "<html>502</html>".to_owned()),
        &[],
    );
    let Outcome::Fatal(ProviderError::InvalidResponse(message)) = &garbled else {
        panic!("InvalidResponse bekleniyordu: {garbled:?}");
    };
    assert!(message.contains("<html>502</html>"), "{message}");
}

#[test]
fn exhausted_retries_become_the_matching_final_error() {
    let rate = from_status(429, "x", &[]).into_final();
    assert_eq!(rate, ProviderError::RateLimited { retry_after: None });
    let down = from_status(503, "bakımda", &[]).into_final();
    assert_eq!(
        down,
        ProviderError::Unavailable {
            status: Some(503),
            message: "bakımda".to_owned()
        }
    );
    assert_eq!(from_status(401, "kötü istek", &[]).into_final(), fatal(401));
}
