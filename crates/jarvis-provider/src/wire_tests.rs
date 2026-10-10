//! Eşleme testleri. Oracle: elle yazılmış `OpenAI` tel JSON'u (kodun kendi çıktısı değil).

use async_openai::types::chat::{ChatCompletionResponseMessage, CreateChatCompletionResponse};
use jarvis_types::{
    Content, Message, RiskLevel, Role, ToolCall, ToolCallId, ToolName, ToolSpec, Trust,
};
use proptest::prelude::*;
use serde_json::{Value, json};

use super::{from_wire_message, from_wire_response, to_wire_message, to_wire_request};
use crate::{ChatRequest, FinishReason, ProviderError, Usage};

fn call(id: &str, name: &str, arguments: Value) -> ToolCall {
    ToolCall {
        id: ToolCallId::new(id.to_owned()).unwrap(),
        name: ToolName::parse(name).unwrap(),
        arguments,
    }
}

fn assistant(text: Option<&str>, calls: Vec<ToolCall>) -> Message {
    let mut message = Message::text(Role::Assistant, text.unwrap_or_default());
    if text.is_none() {
        message.content.clear();
    }
    message.tool_calls = calls;
    message
}

fn wire(message: &Message) -> Value {
    serde_json::to_value(to_wire_message(message).unwrap()).unwrap()
}

fn response(body: &Value) -> CreateChatCompletionResponse {
    serde_json::from_value(body.clone()).unwrap()
}

fn completion(message: &Value, finish: &str) -> Value {
    json!({
        "id": "chatcmpl-1", "object": "chat.completion", "created": 1, "model": "m-1",
        "choices": [{"index": 0, "message": message, "finish_reason": finish}],
        "usage": {"prompt_tokens": 7, "completion_tokens": 3, "total_tokens": 10}
    })
}

#[test]
fn request_messages_map_to_the_openai_shape() {
    assert_eq!(
        wire(&Message::text(Role::System, "be brief")),
        json!({"role": "system", "content": "be brief"})
    );
    assert_eq!(
        wire(&Message::text(Role::User, "merhaba")),
        json!({"role": "user", "content": "merhaba"})
    );
    let mut tool = Message::text(Role::Tool, "ok");
    tool.tool_call_id = ToolCallId::new("call_9".to_owned()).ok();
    tool.trust = Trust::Untrusted {
        source: "read_file".to_owned(),
    };
    assert_eq!(
        wire(&tool),
        json!({"role": "tool", "content": "ok", "tool_call_id": "call_9"})
    );
}

#[test]
fn assistant_tool_calls_carry_json_text_arguments() {
    let message = assistant(
        Some("bakıyorum"),
        vec![call("call_1", "read_file", json!({"path": "/a"}))],
    );
    assert_eq!(
        wire(&message),
        json!({
            "role": "assistant", "content": "bakıyorum",
            "tool_calls": [{"type": "function", "id": "call_1",
                "function": {"name": "read_file", "arguments": "{\"path\":\"/a\"}"}}]
        })
    );
    let only_calls = assistant(None, vec![call("c", "noop", json!({}))]);
    let value = wire(&only_calls);
    assert_eq!(value.get("content"), None, "içerik yoksa alan yazılmaz");
    assert_eq!(
        value.pointer("/tool_calls/0/function/arguments"),
        Some(&json!("{}"))
    );
}

#[test]
fn unmappable_request_messages_are_explicit_errors() {
    let tool_without_id = Message::text(Role::Tool, "x");
    assert!(matches!(
        to_wire_message(&tool_without_id),
        Err(ProviderError::InvalidRequest(m)) if m.contains("tool_call_id")
    ));
    let mut image = Message::text(Role::User, "bak");
    image.content.push(Content::ImagePng(vec![1, 2, 3]));
    assert!(matches!(
        to_wire_message(&image),
        Err(ProviderError::InvalidRequest(m)) if m.contains("görüntü")
    ));
}

#[test]
fn request_carries_model_tools_and_max_tokens_only_when_set() {
    let spec = ToolSpec {
        name: ToolName::parse("read_file").unwrap(),
        description: "Reads a file.".to_owned(),
        parameters: json!({"type": "object", "properties": {"path": {"type": "string"}}}),
        risk: RiskLevel::Read,
    };
    let full = ChatRequest {
        messages: vec![Message::text(Role::User, "hi")],
        tools: vec![spec],
        max_tokens: Some(256),
    };
    let body = serde_json::to_value(to_wire_request("model-x", &full).unwrap()).unwrap();
    assert_eq!(
        body,
        json!({
            "model": "model-x",
            "messages": [{"role": "user", "content": "hi"}],
            "tools": [{"type": "function", "function": {
                "name": "read_file", "description": "Reads a file.",
                "parameters": {"type": "object", "properties": {"path": {"type": "string"}}}}}],
            "max_tokens": 256
        })
    );
    let bare = ChatRequest {
        messages: full.messages,
        tools: Vec::new(),
        max_tokens: None,
    };
    let body = serde_json::to_value(to_wire_request("model-x", &bare).unwrap()).unwrap();
    assert_eq!(
        body,
        json!({"model": "model-x", "messages": [{"role": "user", "content": "hi"}]})
    );
}

#[test]
fn request_without_messages_is_rejected() {
    let empty = ChatRequest::default();
    assert!(matches!(
        to_wire_request("m", &empty),
        Err(ProviderError::InvalidRequest(m)) if m.contains("mesaj")
    ));
}

#[test]
fn text_response_maps_message_finish_usage_and_model() {
    let body = completion(&json!({"role": "assistant", "content": "selam"}), "stop");
    let mapped = from_wire_response(response(&body)).unwrap();
    assert_eq!(mapped.message, Message::text(Role::Assistant, "selam"));
    assert_eq!(mapped.finish, FinishReason::Stop);
    assert_eq!(mapped.model, "m-1");
    assert_eq!(
        mapped.usage,
        Some(Usage {
            prompt_tokens: 7,
            completion_tokens: 3,
            total_tokens: 10
        })
    );
}

#[test]
fn finish_reasons_map_one_to_one() {
    for (wire_name, expected) in [
        ("stop", FinishReason::Stop),
        ("length", FinishReason::Length),
        ("tool_calls", FinishReason::ToolCalls),
        ("content_filter", FinishReason::ContentFilter),
        ("function_call", FinishReason::ToolCalls),
    ] {
        let body = completion(&json!({"role": "assistant", "content": "x"}), wire_name);
        assert_eq!(
            from_wire_response(response(&body)).unwrap().finish,
            expected,
            "{wire_name}"
        );
    }
}

#[test]
fn tool_call_arguments_are_parsed_and_unparsable_text_is_kept_as_a_string() {
    let message = json!({"role": "assistant", "content": null, "tool_calls": [
        {"type": "function", "id": "c1", "function": {
            "name": "read_file", "arguments": "{\"path\":\"/a\"}"}},
        {"type": "function", "id": "c2", "function": {"name": "read_file", "arguments": "{bozuk"}}
    ]});
    let mapped = from_wire_response(response(&completion(&message, "tool_calls"))).unwrap();
    let calls = &mapped.message.tool_calls;
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls.first().map(|c| &c.arguments),
        Some(&json!({"path": "/a"}))
    );
    assert_eq!(calls.get(1).map(|c| &c.arguments), Some(&json!("{bozuk")));
    assert!(mapped.message.content.is_empty());
}

#[test]
fn unmappable_responses_are_invalid_response_errors() {
    let cases = [
        (json!({"role": "assistant"}), "stop", "ne metin ne araç"),
        (
            json!({"role": "assistant", "content": ""}),
            "stop",
            "ne metin ne araç",
        ),
        (json!({"role": "user", "content": "x"}), "stop", "asistan"),
        (
            json!({"role": "assistant", "tool_calls": [{"type": "function", "id": "c",
                "function": {"name": "Read.File", "arguments": "{}"}}]}),
            "tool_calls",
            "araç adı",
        ),
        (
            json!({"role": "assistant", "tool_calls": [{"type": "function", "id": "",
                "function": {"name": "ok", "arguments": "{}"}}]}),
            "tool_calls",
            "kimliği",
        ),
        (
            json!({"role": "assistant", "tool_calls": [{"type": "custom", "id": "c",
                "custom_tool": {"name": "x", "input": "y"}}]}),
            "tool_calls",
            "özel araç",
        ),
    ];
    for (message, finish, needle) in cases {
        let result = from_wire_response(response(&completion(&message, finish)));
        assert!(
            matches!(&result, Err(ProviderError::InvalidResponse(m)) if m.contains(needle)),
            "{message}: {result:?}"
        );
    }
}

#[test]
fn response_without_choices_or_finish_reason_is_invalid() {
    let mut no_choices = completion(&json!({"role": "assistant", "content": "x"}), "stop");
    no_choices["choices"] = json!([]);
    assert!(matches!(
        from_wire_response(response(&no_choices)),
        Err(ProviderError::InvalidResponse(m)) if m.contains("seçenek")
    ));
    let mut no_finish = completion(&json!({"role": "assistant", "content": "x"}), "stop");
    no_finish["choices"][0]["finish_reason"] = Value::Null;
    assert!(matches!(
        from_wire_response(response(&no_finish)),
        Err(ProviderError::InvalidResponse(m)) if m.contains("finish_reason")
    ));
}

#[test]
fn refusal_text_is_surfaced_not_dropped() {
    let message = json!({"role": "assistant", "content": null, "refusal": "yapamam"});
    let mapped = from_wire_response(response(&completion(&message, "stop"))).unwrap();
    assert_eq!(mapped.message.joined_text(), "yapamam");
}

fn arb_call() -> impl Strategy<Value = ToolCall> {
    (
        "[a-zA-Z0-9_-]{1,40}",
        "[a-z][a-z0-9_]{0,20}",
        prop_oneof![
            Just(json!({})),
            Just(json!({"path": "/tmp/ş"})),
            Just(json!({"n": 3, "flag": true, "xs": [1, 2]})),
        ],
    )
        .prop_map(|(id, name, arguments)| call(&id, &name, arguments))
}

proptest! {
    #[test]
    fn assistant_messages_survive_domain_to_wire_to_domain(
        text in proptest::option::of("[^\u{0}]{1,60}"),
        calls in proptest::collection::vec(arb_call(), 0..4),
    ) {
        prop_assume!(text.is_some() || !calls.is_empty());
        let original = assistant(text.as_deref(), calls);
        let json = serde_json::to_value(to_wire_message(&original).unwrap()).unwrap();
        let parsed: ChatCompletionResponseMessage = serde_json::from_value(json).unwrap();
        prop_assert_eq!(from_wire_message(parsed).unwrap(), original);
    }
}
