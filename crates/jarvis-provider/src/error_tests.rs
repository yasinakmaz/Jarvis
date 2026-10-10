use jarvis_types::ErrorCode;

use super::ProviderError;

#[test]
fn unavailable_mentions_the_status_only_when_there_is_one() {
    let with = ProviderError::Unavailable {
        status: Some(503),
        message: "bakımda".to_owned(),
    };
    assert_eq!(
        with.to_string(),
        "sağlayıcıya ulaşılamadı (HTTP 503): bakımda"
    );
    let without = ProviderError::Unavailable {
        status: None,
        message: "zaman aşımı".to_owned(),
    };
    assert_eq!(without.to_string(), "sağlayıcıya ulaşılamadı: zaman aşımı");
}

#[test]
fn every_variant_maps_to_its_api_error_code() {
    let cases = [
        (
            ProviderError::RateLimited { retry_after: None },
            ErrorCode::ProviderUnavailable,
        ),
        (
            ProviderError::Unavailable {
                status: None,
                message: String::new(),
            },
            ErrorCode::ProviderUnavailable,
        ),
        (
            ProviderError::Rejected {
                status: 401,
                message: String::new(),
            },
            ErrorCode::ProviderRejected,
        ),
        (
            ProviderError::InvalidResponse(String::new()),
            ErrorCode::ProviderUnavailable,
        ),
        (
            ProviderError::InvalidRequest(String::new()),
            ErrorCode::InvalidRequest,
        ),
        (ProviderError::Cancelled, ErrorCode::Halted),
        (
            ProviderError::MissingKey {
                provider: "p".to_owned(),
                env_var: "E".to_owned(),
            },
            ErrorCode::ProviderUnavailable,
        ),
    ];
    for (error, code) in cases {
        assert_eq!(error.code(), code, "{error}");
    }
}
