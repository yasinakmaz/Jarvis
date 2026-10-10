//! Serde gidiş-dönüşü: `T → JSON → T` eşitliği (Tasarım 0003 test planı, proptest).

use jarvis_types::{
    Content, Message, RiskLevel, Role, ToolCall, ToolCallId, ToolName, ToolSpec, Trust,
    Verification,
};
use proptest::prelude::*;
use serde_json::{Value, json};

fn tool_name() -> impl Strategy<Value = ToolName> {
    "[a-z][a-z0-9_]{0,20}".prop_filter_map("geçerli ad", |n| ToolName::parse(&n).ok())
}

fn json_value() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(|n| json!(n)),
        "[^\\u{0}]{0,12}".prop_map(Value::String),
        proptest::collection::btree_map("[a-z]{1,6}", any::<i32>(), 0..4).prop_map(|m| json!(m)),
    ]
}

fn role() -> impl Strategy<Value = Role> {
    prop_oneof![
        Just(Role::System),
        Just(Role::User),
        Just(Role::Assistant),
        Just(Role::Tool)
    ]
}

fn content() -> impl Strategy<Value = Content> {
    prop_oneof![
        ".{0,30}".prop_map(Content::Text),
        proptest::collection::vec(any::<u8>(), 0..16).prop_map(Content::ImagePng),
    ]
}

fn trust() -> impl Strategy<Value = Trust> {
    prop_oneof![
        Just(Trust::Trusted),
        "[a-z_]{1,10}".prop_map(|source| Trust::Untrusted { source }),
    ]
}

fn call_id() -> impl Strategy<Value = ToolCallId> {
    "[a-zA-Z0-9_]{1,24}".prop_filter_map("geçerli kimlik", |id| ToolCallId::new(id).ok())
}

fn tool_call() -> impl Strategy<Value = ToolCall> {
    (call_id(), tool_name(), json_value()).prop_map(|(id, name, arguments)| ToolCall {
        id,
        name,
        arguments,
    })
}

fn message() -> impl Strategy<Value = Message> {
    (
        role(),
        proptest::collection::vec(content(), 0..3),
        proptest::collection::vec(tool_call(), 0..3),
        proptest::option::of(call_id()),
        trust(),
    )
        .prop_map(|(role, content, tool_calls, tool_call_id, trust)| Message {
            role,
            content,
            tool_calls,
            tool_call_id,
            trust,
        })
}

fn risk() -> impl Strategy<Value = RiskLevel> {
    prop_oneof![
        Just(RiskLevel::Read),
        Just(RiskLevel::Act),
        Just(RiskLevel::Sensitive),
        Just(RiskLevel::Destructive),
    ]
}

fn verification() -> impl Strategy<Value = Verification> {
    prop_oneof![
        Just(Verification::Verified),
        ".{0,10}".prop_map(|reason| Verification::Unverifiable { reason }),
        ".{0,10}".prop_map(|reason| Verification::Failed { reason }),
    ]
}

fn round_trip<T>(value: &T) -> Result<T, serde_json::Error>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    serde_json::from_str(&serde_json::to_string(value)?)
}

proptest! {
    #[test]
    fn messages_round_trip(msg in message()) {
        prop_assert_eq!(round_trip(&msg)?, msg);
    }

    #[test]
    fn tool_specs_round_trip(
        name in tool_name(), description in ".{0,40}", parameters in json_value(), risk in risk()
    ) {
        let spec = ToolSpec { name, description, parameters, risk };
        prop_assert_eq!(round_trip(&spec)?, spec);
    }

    #[test]
    fn verifications_round_trip(v in verification()) {
        prop_assert_eq!(round_trip(&v)?, v);
    }
}
