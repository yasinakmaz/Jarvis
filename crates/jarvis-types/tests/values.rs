//! Hata kodları, risk sıralaması, araç adları ve mesajlar (Tasarım 0003 test planı).

use jarvis_types::{
    Content, ErrorCode, Message, NameError, RiskLevel, Role, ToolName, Trust, Verification,
};
use proptest::prelude::*;

/// Mimari §4'teki ve tasarım 0003'teki tel adları (bağımsız tablo).
const WIRE_NAMES: [(ErrorCode, &str); 10] = [
    (ErrorCode::ApprovalDenied, "approval_denied"),
    (ErrorCode::DesktopUnavailable, "desktop_unavailable"),
    (ErrorCode::LimitReached, "limit_reached"),
    (ErrorCode::Halted, "halted"),
    (ErrorCode::ProviderUnavailable, "provider_unavailable"),
    (ErrorCode::ProviderRejected, "provider_rejected"),
    (ErrorCode::InvalidRequest, "invalid_request"),
    (ErrorCode::Unauthorized, "unauthorized"),
    (ErrorCode::NotFound, "not_found"),
    (ErrorCode::Internal, "internal"),
];

#[test]
fn error_code_wire_names_match_architecture() {
    for (code, name) in WIRE_NAMES {
        assert_eq!(code.as_str(), name);
        assert_eq!(code.to_string(), name);
        assert_eq!(serde_json::to_string(&code).unwrap(), format!("\"{name}\""));
    }
    let listed: Vec<ErrorCode> = WIRE_NAMES.iter().map(|(c, _)| *c).collect();
    assert_eq!(listed, ErrorCode::ALL.to_vec());
}

#[test]
fn risk_levels_are_ordered_by_severity() {
    let ordered = [
        RiskLevel::Read,
        RiskLevel::Act,
        RiskLevel::Sensitive,
        RiskLevel::Destructive,
    ];
    for pair in ordered.windows(2) {
        assert!(pair[0] < pair[1]);
    }
    assert_eq!(
        serde_json::to_string(&RiskLevel::Destructive).unwrap(),
        "\"destructive\""
    );
}

#[test]
fn verification_wire_shape_is_tagged() {
    let failed = Verification::Failed {
        reason: "pencere yok".into(),
    };
    assert_eq!(
        serde_json::to_value(&failed).unwrap(),
        serde_json::json!({"status": "failed", "reason": "pencere yok"})
    );
    assert_eq!(
        serde_json::to_value(Verification::Verified).unwrap(),
        serde_json::json!({"status": "verified"})
    );
}

/// Desen `^[a-z][a-z0-9_]{0,63}$` için bağımsız denetleyici (bayt tabanlı).
fn matches_pattern(name: &str) -> bool {
    let bytes = name.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes.first().is_some_and(u8::is_ascii_lowercase)
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_')
}

#[test]
fn tool_name_examples() {
    assert_eq!(ToolName::parse("read_file").unwrap().as_str(), "read_file");
    assert_eq!(ToolName::parse("a").unwrap().to_string(), "a");
    assert!(ToolName::parse(&format!("a{}", "b".repeat(63))).is_ok());
    for bad in [
        "",
        "Read",
        "1st",
        "_x",
        "a-b",
        "dosya_oku_ğ",
        &"a".repeat(65),
    ] {
        assert_eq!(
            ToolName::parse(bad),
            Err(NameError(bad.to_owned())),
            "{bad}"
        );
    }
    assert!(serde_json::from_str::<ToolName>("\"Bad\"").is_err());
    assert_eq!(
        serde_json::to_string(&ToolName::parse("x1").unwrap()).unwrap(),
        "\"x1\""
    );
}

proptest! {
    #[test]
    fn tool_name_accepts_exactly_the_pattern(name in "[a-zA-Z0-9_ğü-]{0,70}") {
        prop_assert_eq!(ToolName::parse(&name).is_ok(), matches_pattern(&name));
    }
}

#[test]
fn text_message_is_trusted_single_part() {
    let msg = Message::text(Role::User, "merhaba");
    assert_eq!(msg.role, Role::User);
    assert_eq!(msg.content, vec![Content::Text("merhaba".into())]);
    assert!(msg.tool_calls.is_empty());
    assert_eq!(msg.tool_call_id, None);
    assert_eq!(msg.trust, Trust::Trusted);
}

#[test]
fn joined_text_skips_images() {
    let mut msg = Message::text(Role::Tool, "bir");
    msg.content.push(Content::ImagePng(vec![1, 2]));
    msg.content.push(Content::Text("iki".into()));
    assert_eq!(msg.joined_text(), "biriki");
}

#[test]
fn content_and_trust_wire_shapes() {
    assert_eq!(
        serde_json::to_value(Content::Text("x".into())).unwrap(),
        serde_json::json!({"type": "text", "value": "x"})
    );
    assert_eq!(
        serde_json::to_value(Trust::Untrusted {
            source: "web".into()
        })
        .unwrap(),
        serde_json::json!({"trust": "untrusted", "source": "web"})
    );
}
