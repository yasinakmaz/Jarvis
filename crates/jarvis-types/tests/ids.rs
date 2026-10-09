//! Kimlik tipleri (Tasarım 0003 test planı).

use jarvis_types::{EventSeq, IdError, RunId, SessionId, ToolCallId, TraceId};

/// Bilinen bir UUID v7 ve bir UUID v4 (RFC 9562 örnekleri).
const V7: &str = "017f22e2-79b0-7cc3-98c4-dc0c0c07398f";
const V4: &str = "919108f7-52d1-4320-9bac-f847db4148a8";

#[test]
fn v7_text_round_trips_through_parse_and_display() {
    let id = RunId::parse(V7).unwrap();
    assert_eq!(id.to_string(), V7);
    assert_eq!(SessionId::parse(V7).unwrap().to_string(), V7);
    assert_eq!(V7.parse::<TraceId>().unwrap().to_string(), V7);
}

#[test]
fn uppercase_input_is_normalised_to_lowercase() {
    let id = RunId::parse(&V7.to_uppercase()).unwrap();
    assert_eq!(id.to_string(), V7);
}

#[test]
fn non_v7_uuid_is_rejected() {
    assert_eq!(RunId::parse(V4), Err(IdError::NotV7(V4.to_owned())));
}

#[test]
fn garbage_is_rejected() {
    assert_eq!(
        RunId::parse("koşu-1"),
        Err(IdError::InvalidUuid("koşu-1".to_owned()))
    );
    assert!(RunId::parse("").is_err());
}

#[test]
fn new_ids_are_v7_and_increase_in_creation_order() {
    let ids: Vec<RunId> = (0..2_000).map(|_| RunId::new()).collect();
    for pair in ids.windows(2) {
        assert!(pair[0] < pair[1], "{} !< {}", pair[0], pair[1]);
    }
    let text = ids[0].to_string();
    assert_eq!(text.as_bytes()[14], b'7', "sürüm hanesi: {text}");
    assert_eq!(RunId::parse(&text).unwrap(), ids[0]);
}

#[test]
fn ids_serialize_as_plain_strings() {
    let id = SessionId::parse(V7).unwrap();
    assert_eq!(serde_json::to_string(&id).unwrap(), format!("\"{V7}\""));
    let back: SessionId = serde_json::from_str(&format!("\"{V7}\"")).unwrap();
    assert_eq!(back, id);
    assert!(serde_json::from_str::<SessionId>(&format!("\"{V4}\"")).is_err());
}

#[test]
fn tool_call_id_enforces_length_bounds() {
    assert_eq!(
        ToolCallId::new(String::new()),
        Err(IdError::InvalidToolCallId { len: 0 })
    );
    let max = "a".repeat(256);
    assert_eq!(ToolCallId::new(max.clone()).unwrap().as_str(), max);
    assert_eq!(
        ToolCallId::new("a".repeat(257)),
        Err(IdError::InvalidToolCallId { len: 257 })
    );
    // Uzunluk bayt değil karakterle ölçülür.
    assert!(ToolCallId::new("ğ".repeat(256)).is_ok());
}

#[test]
fn tool_call_id_serde_and_display() {
    let id = ToolCallId::new("call_1".to_owned()).unwrap();
    assert_eq!(id.to_string(), "call_1");
    assert_eq!(serde_json::to_string(&id).unwrap(), "\"call_1\"");
    assert!(serde_json::from_str::<ToolCallId>("\"\"").is_err());
}

#[test]
fn event_seq_advances_and_reports_overflow() {
    assert_eq!(EventSeq::new(42).get(), 42);
    assert_eq!(EventSeq::new(41).next().map(EventSeq::get), Some(42));
    assert_eq!(EventSeq::FIRST.get(), 0);
    assert_eq!(EventSeq::FIRST.next(), Some(EventSeq::new(1)));
    assert_eq!(EventSeq::new(u64::MAX).next(), None);
    assert_eq!(serde_json::to_string(&EventSeq::new(42)).unwrap(), "42");
    assert_eq!(EventSeq::new(7).to_string(), "7");
}

#[test]
fn errors_explain_the_problem() {
    let message = IdError::InvalidToolCallId { len: 300 }.to_string();
    assert!(
        message.contains("300") && message.contains("256"),
        "{message}"
    );
}
