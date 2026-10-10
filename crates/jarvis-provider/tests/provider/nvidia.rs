//! L3: NVIDIA biçimli kasetler (`tests/cassettes/`, elle yazılmış — bkz. kasetlerin README'si).

use std::path::Path;

use futures_util::StreamExt;
use jarvis_provider::{ChatModel, ChatRequest, FinishReason, NoObserver, RawChat, RawResponse};
use jarvis_testkit::CassetteServer;
use jarvis_types::{Message, Role, ToolName, ToolSpec};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::support::{fake_clock, model, within};

#[expect(
    clippy::unwrap_used,
    reason = "yardımcı: geçersiz sabit girdide test düşmeli"
)]
async fn replay(name: &str) -> CassetteServer {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/cassettes")
        .join(name);
    CassetteServer::replay(&path).await.unwrap()
}

#[expect(
    clippy::unwrap_used,
    reason = "yardımcı: geçersiz sabit girdide test düşmeli"
)]
fn read_file_spec() -> ToolSpec {
    ToolSpec {
        name: ToolName::parse("read_file").unwrap(),
        description: "Reads a UTF-8 text file and returns its content.".to_owned(),
        parameters: json!({
            "type": "object",
            "properties": {"path": {"type": "string"}},
            "required": ["path"]
        }),
        risk: jarvis_types::RiskLevel::Read,
    }
}

#[tokio::test]
async fn nvidia_shaped_text_reply_is_mapped() {
    let server = replay("nvidia-text.jsonl").await;
    let llm = model(&server.base_url(), &fake_clock());
    let request = ChatRequest {
        messages: vec![Message::text(Role::User, "Merhaba")],
        ..ChatRequest::default()
    };
    let reply = within(llm.complete(request, CancellationToken::new(), &NoObserver))
        .await
        .unwrap();
    assert_eq!(
        reply.message.joined_text(),
        "Merhaba! Size nasıl yardımcı olabilirim?"
    );
    assert!(
        reply.message.tool_calls.is_empty(),
        "boş tool_calls dizisi araç çağrısı değildir"
    );
    assert_eq!(reply.finish, FinishReason::Stop);
    assert_eq!(reply.model, "meta/llama-3.3-70b-instruct");
    assert_eq!(reply.usage.map(|u| u.total_tokens), Some(42));
    server.assert_exhausted().unwrap();
}

#[tokio::test]
async fn nvidia_shaped_tool_call_round_trip_replays_the_whole_conversation() {
    let server = replay("nvidia-tool-call.jsonl").await;
    let llm = model(&server.base_url(), &fake_clock());
    let mut messages = vec![Message::text(Role::User, "/tmp/a.txt dosyasını oku")];
    let request = ChatRequest {
        messages: messages.clone(),
        tools: vec![read_file_spec()],
        max_tokens: None,
    };
    let first = within(llm.complete(request, CancellationToken::new(), &NoObserver))
        .await
        .unwrap();
    assert_eq!(first.finish, FinishReason::ToolCalls);
    let call = first.message.tool_calls.first().unwrap().clone();
    assert_eq!(call.id.as_str(), "chatcmpl-tool-9f8e7d6c5b4a");
    assert_eq!(call.arguments, json!({"path": "/tmp/a.txt"}));

    messages.push(first.message);
    let mut result = Message::text(Role::Tool, "merhaba dünya");
    result.tool_call_id = Some(call.id);
    messages.push(result);
    let request = ChatRequest {
        messages,
        tools: vec![read_file_spec()],
        max_tokens: None,
    };
    let second = within(llm.complete(request, CancellationToken::new(), &NoObserver))
        .await
        .unwrap();
    assert_eq!(
        second.message.joined_text(),
        "Dosyada şu yazıyor: merhaba dünya"
    );
    server.assert_exhausted().unwrap();
}

#[tokio::test]
async fn nvidia_shaped_stream_is_forwarded_chunk_by_chunk() {
    let server = replay("nvidia-stream.jsonl").await;
    let llm = model(&server.base_url(), &fake_clock());
    let body = json!({"model": "fast", "stream": true,
                      "messages": [{"role": "user", "content": "Merhaba"}]});
    let response = within(llm.forward(body, CancellationToken::new(), &NoObserver))
        .await
        .unwrap();
    let RawResponse::Stream(stream) = response else {
        panic!("akış bekleniyordu");
    };
    let chunks: Vec<_> = stream.map(Result::unwrap).collect().await;
    let text: String = chunks
        .iter()
        .filter_map(|c| {
            c.pointer("/choices/0/delta/content")
                .and_then(|v| v.as_str())
        })
        .collect();
    assert_eq!(text, "Merhaba");
    assert_eq!(chunks.len(), 4, "[DONE] iletilmez");
    assert_eq!(
        chunks.last().and_then(|c| c.pointer("/usage/total_tokens")),
        Some(&json!(12))
    );
    server.assert_exhausted().unwrap();
}
