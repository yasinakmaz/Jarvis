//! `OpenAiModel::complete` (Tasarım 0006 test planı, L2). Oracle: kaset sunucusunun aldığı
//! gövdeler ve başlıklar, `FakeClock` bekleme kayıtları, elle yazılmış beklenen değerler.

use std::sync::Arc;
use std::time::Duration;

use crate::support::{KEY, MODEL, Notices, drive, fake_clock, model, model_with, until, within};
use jarvis_provider::{
    ChatModel, ChatRequest, ChatResponse, FinishReason, Jitter, NoObserver, ProviderError,
    RetryNotice, RetryReason, Usage,
};
use jarvis_testkit::CassetteServer;
use jarvis_types::{Message, Role, ToolName};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

fn ask() -> ChatRequest {
    ChatRequest {
        messages: vec![Message::text(Role::User, "merhaba")],
        ..ChatRequest::default()
    }
}

fn wire_request() -> Value {
    json!({"model": MODEL, "messages": [{"role": "user", "content": "merhaba"}]})
}

fn turn(n: u32, response: &Value) -> String {
    json!({"turn": n,
        "request": {"method": "POST", "path": "/v1/chat/completions", "body": wire_request()},
        "response": response})
    .to_string()
}

fn ok_body(text: &str) -> Value {
    json!({
        "id": "c1", "object": "chat.completion", "created": 1, "model": "served-model",
        "choices": [{"index": 0, "message": {"role": "assistant", "content": text},
                     "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 5, "completion_tokens": 2, "total_tokens": 7}
    })
}

fn ok(n: u32, text: &str) -> String {
    turn(n, &json!({"status": 200, "body": ok_body(text)}))
}

fn failure(n: u32, status: u16, message: &str) -> String {
    turn(
        n,
        &json!({"status": status, "body": {"error": {"message": message, "type": "t"}}}),
    )
}

#[expect(
    clippy::unwrap_used,
    reason = "yardımcı: geçersiz sabit girdide test düşmeli"
)]
async fn cassette(lines: &[String]) -> CassetteServer {
    CassetteServer::replay_str(&lines.join("\n")).await.unwrap()
}

async fn complete(
    llm: &impl ChatModel,
    request: ChatRequest,
) -> Result<ChatResponse, ProviderError> {
    within(llm.complete(request, CancellationToken::new(), &NoObserver)).await
}

#[tokio::test]
async fn a_text_completion_sends_the_documented_wire_request_with_the_bearer_key() {
    let server = cassette(&[ok(1, "selam")]).await;
    let clock = fake_clock();
    let llm = model(&server.base_url(), &clock);
    let response = complete(&llm, ask()).await.unwrap();

    assert_eq!(response.message, Message::text(Role::Assistant, "selam"));
    assert_eq!(response.finish, FinishReason::Stop);
    assert_eq!(response.model, "served-model");
    assert_eq!(
        response.usage,
        Some(Usage {
            prompt_tokens: 5,
            completion_tokens: 2,
            total_tokens: 7
        })
    );
    server.assert_exhausted().unwrap();
    let recorded = server.recorded();
    let sent = recorded.first().unwrap();
    assert!(
        sent.headers
            .contains(&("authorization".to_owned(), format!("Bearer {KEY}")))
    );
    let names: Vec<&str> = sent.headers.iter().map(|(name, _)| name.as_str()).collect();
    assert!(!names.contains(&"openai-organization"), "{names:?}");
    assert!(!names.contains(&"openai-project"), "{names:?}");
    assert!(clock.sleeps().is_empty());
}

#[tokio::test]
async fn tools_and_max_tokens_reach_the_provider_and_tool_calls_come_back() {
    let spec = jarvis_types::ToolSpec {
        name: ToolName::parse("read_file").unwrap(),
        description: "Reads a file.".to_owned(),
        parameters: json!({"type": "object"}),
        risk: jarvis_types::RiskLevel::Read,
    };
    let expected = json!({
        "model": MODEL, "max_tokens": 64,
        "messages": [{"role": "user", "content": "merhaba"}],
        "tools": [{"type": "function", "function": {
            "name": "read_file", "description": "Reads a file.", "parameters": {"type": "object"}}}]
    });
    let reply = json!({
        "id": "c2", "object": "chat.completion", "created": 1, "model": "m",
        "choices": [{"index": 0, "finish_reason": "tool_calls", "message": {
            "role": "assistant", "content": null,
            "tool_calls": [{"type": "function", "id": "call_7",
                "function": {"name": "read_file", "arguments": "{\"path\":\"/a\"}"}}]}}]
    });
    let line = json!({"turn": 1,
        "request": {"method": "POST", "path": "/v1/chat/completions", "body": expected},
        "response": {"status": 200, "body": reply}})
    .to_string();
    let server = cassette(&[line]).await;
    let llm = model(&server.base_url(), &fake_clock());
    let request = ChatRequest {
        tools: vec![spec],
        max_tokens: Some(64),
        ..ask()
    };
    let response = complete(&llm, request).await.unwrap();
    assert_eq!(response.finish, FinishReason::ToolCalls);
    let call = response.message.tool_calls.first().unwrap();
    assert_eq!(
        (call.id.as_str(), call.name.as_str()),
        ("call_7", "read_file")
    );
    assert_eq!(call.arguments, json!({"path": "/a"}));
    assert_eq!(response.usage, None);
    server.assert_exhausted().unwrap();
}

#[tokio::test]
async fn rate_limited_twice_then_success_waits_half_a_second_then_one() {
    let server = cassette(&[
        failure(1, 429, "slow down"),
        failure(2, 429, "slow down"),
        ok(3, "tamam"),
    ])
    .await;
    let clock = fake_clock();
    let llm = model(&server.base_url(), &clock);
    let notices = Notices::default();
    let response = drive(
        &clock,
        llm.complete(ask(), CancellationToken::new(), &notices),
    )
    .await
    .unwrap();
    assert_eq!(response.message.joined_text(), "tamam");
    assert_eq!(
        clock.sleeps(),
        [Duration::from_millis(500), Duration::from_secs(1)]
    );
    assert_eq!(
        notices.all(),
        [
            RetryNotice {
                attempt: 1,
                delay: Duration::from_millis(500),
                reason: RetryReason::RateLimited
            },
            RetryNotice {
                attempt: 2,
                delay: Duration::from_secs(1),
                reason: RetryReason::RateLimited
            },
        ]
    );
    server.assert_exhausted().unwrap();
}

#[tokio::test]
async fn five_server_errors_exhaust_the_attempts_with_doubling_waits() {
    let lines: Vec<String> = (1..=5).map(|n| failure(n, 503, "bakımda")).collect();
    let server = cassette(&lines).await;
    let clock = fake_clock();
    let llm = model(&server.base_url(), &clock);
    let error = drive(&clock, complete(&llm, ask())).await.unwrap_err();
    let ProviderError::Unavailable {
        status: Some(503),
        message,
    } = &error
    else {
        panic!("Unavailable(503) bekleniyordu: {error:?}");
    };
    assert!(message.contains("bakımda"), "{message}");
    let waits: Vec<u64> = clock
        .sleeps()
        .iter()
        .map(|d| d.as_millis().try_into().unwrap())
        .collect();
    assert_eq!(waits, [500, 1000, 2000, 4000]);
    server.assert_exhausted().unwrap();
}

#[tokio::test]
async fn five_rate_limits_end_as_rate_limited() {
    let lines: Vec<String> = (1..=5).map(|n| failure(n, 429, "yavaş")).collect();
    let server = cassette(&lines).await;
    let clock = fake_clock();
    let llm = model(&server.base_url(), &clock);
    let error = drive(&clock, complete(&llm, ask())).await.unwrap_err();
    assert_eq!(error, ProviderError::RateLimited { retry_after: None });
    server.assert_exhausted().unwrap();
}

#[tokio::test]
async fn a_retry_after_header_is_not_honoured_yet_the_own_backoff_applies() {
    let first = turn(
        1,
        &json!({"status": 429, "headers": {"retry-after": "7"},
                "body": {"error": {"message": "x", "type": "t"}}}),
    );
    let server = cassette(&[first, ok(2, "tamam")]).await;
    let clock = fake_clock();
    let llm = model(&server.base_url(), &clock);
    drive(&clock, complete(&llm, ask())).await.unwrap();
    assert_eq!(clock.sleeps(), [Duration::from_millis(500)]);
}

#[tokio::test]
async fn unauthorized_is_rejected_after_one_request_and_the_message_hides_the_key() {
    let message = format!("Incorrect API key provided: {KEY}");
    let server = cassette(&[failure(1, 401, &message)]).await;
    let clock = fake_clock();
    let llm = model(&server.base_url(), &clock);
    let error = complete(&llm, ask()).await.unwrap_err();
    let ProviderError::Rejected { status, message } = &error else {
        panic!("Rejected bekleniyordu: {error:?}");
    };
    assert_eq!(*status, 401);
    assert!(
        !message.contains(KEY) && message.contains("***"),
        "{message}"
    );
    assert!(!error.to_string().contains(KEY));
    assert_eq!(server.requests().len(), 1);
    assert!(clock.sleeps().is_empty());
    server.assert_exhausted().unwrap();
}

#[tokio::test]
async fn other_client_errors_are_rejected_without_retry() {
    for status in [400_u16, 403, 404, 422] {
        let server = cassette(&[failure(1, status, "kötü")]).await;
        let llm = model(&server.base_url(), &fake_clock());
        let error = complete(&llm, ask()).await.unwrap_err();
        assert!(
            matches!(&error, ProviderError::Rejected { status: s, .. } if *s == status),
            "{error}"
        );
        assert_eq!(server.requests().len(), 1, "{status}");
    }
}

#[tokio::test]
async fn cancelling_during_backoff_returns_cancelled_and_sends_no_further_request() {
    let server = cassette(&[failure(1, 503, "x"), ok(2, "asla")]).await;
    let clock = fake_clock();
    let llm = Arc::new(model(&server.base_url(), &clock));
    let cancel = CancellationToken::new();
    let task = {
        let (llm, cancel) = (Arc::clone(&llm), cancel.clone());
        tokio::spawn(async move { llm.complete(ask(), cancel, &NoObserver).await })
    };
    until(|| !clock.sleeps().is_empty()).await;
    cancel.cancel();
    assert_eq!(task.await.unwrap(), Err(ProviderError::Cancelled));
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn cancelling_while_the_request_is_in_flight_returns_cancelled() {
    let slow = turn(
        1,
        &json!({"status": 200, "body": ok_body("geç"), "delay_ms": 5000}),
    );
    let server = Arc::new(cassette(&[slow]).await);
    let llm = Arc::new(model(&server.base_url(), &fake_clock()));
    let cancel = CancellationToken::new();
    let task = {
        let (llm, cancel) = (Arc::clone(&llm), cancel.clone());
        tokio::spawn(async move { llm.complete(ask(), cancel, &NoObserver).await })
    };
    until(|| !server.requests().is_empty()).await;
    let started = std::time::Instant::now();
    cancel.cancel();
    assert_eq!(task.await.unwrap(), Err(ProviderError::Cancelled));
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[tokio::test]
async fn a_timeout_is_retried_as_unavailable() {
    let slow = turn(
        1,
        &json!({"status": 200, "body": ok_body("geç"), "delay_ms": 1500}),
    );
    let server = cassette(&[slow, ok(2, "hızlı")]).await;
    let clock = fake_clock();
    let llm = model(&server.base_url(), &clock).with_timeout(Duration::from_millis(150));
    let notices = Notices::default();
    let response = drive(
        &clock,
        llm.complete(ask(), CancellationToken::new(), &notices),
    )
    .await
    .unwrap();
    assert_eq!(response.message.joined_text(), "hızlı");
    assert_eq!(
        notices.all().first().map(|n| n.reason),
        Some(RetryReason::Unavailable)
    );
}

#[tokio::test]
async fn an_unreachable_server_ends_as_unavailable_without_a_status() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    drop(listener);
    let clock = fake_clock();
    let llm = model(&url, &clock);
    let error = drive(&clock, complete(&llm, ask())).await.unwrap_err();
    let ProviderError::Unavailable {
        status: None,
        message,
    } = &error
    else {
        panic!("durumsuz Unavailable bekleniyordu: {error:?}");
    };
    assert!(message.contains("bağlantı"), "{message}");
    assert_eq!(clock.sleeps().len(), 4);
}

#[tokio::test]
async fn unparsable_or_empty_success_bodies_are_invalid_responses_and_never_retried() {
    let html = turn(1, &json!({"status": 200, "text": "<html>proxy</html>"}));
    let empty = turn(
        1,
        &json!({"status": 200, "body": {"id": "c", "object": "chat.completion", "created": 1,
                 "model": "m", "choices": []}}),
    );
    let not_found = turn(1, &json!({"status": 404, "text": "<html>404</html>"}));
    for line in [html, empty, not_found] {
        let server = cassette(&[line]).await;
        let llm = model(&server.base_url(), &fake_clock());
        let error = complete(&llm, ask()).await.unwrap_err();
        assert!(
            matches!(error, ProviderError::InvalidResponse(_)),
            "{error}"
        );
        assert_eq!(server.requests().len(), 1);
    }
}

#[tokio::test]
async fn the_rate_limit_spans_calls_and_retries() {
    let server = cassette(&[ok(1, "a"), ok(2, "b"), ok(3, "c")]).await;
    let clock = fake_clock();
    let llm = model_with(
        &server.base_url(),
        2,
        &clock,
        Arc::new(jarvis_provider::FixedJitter::NONE),
    );
    drive(&clock, async {
        for _ in 0..3 {
            complete(&llm, ask()).await.unwrap();
        }
    })
    .await;
    assert_eq!(
        clock.sleeps(),
        [Duration::from_secs(30)],
        "dakikada 2 istek → 3. istek 30 sn bekler"
    );
}

struct Max;

impl Jitter for Max {
    fn jitter(&self, max: Duration) -> Duration {
        max
    }
}

#[tokio::test]
async fn jitter_is_added_to_the_backoff() {
    let server = cassette(&[failure(1, 503, "x"), failure(2, 503, "x"), ok(3, "ok")]).await;
    let clock = fake_clock();
    let llm = model_with(&server.base_url(), 600, &clock, Arc::new(Max));
    drive(&clock, complete(&llm, ask())).await.unwrap();
    assert_eq!(
        clock.sleeps(),
        [Duration::from_millis(625), Duration::from_millis(1250)]
    );
}

#[tokio::test]
async fn the_debug_output_never_contains_the_key() {
    let llm = model("http://127.0.0.1:1/v1", &fake_clock());
    let text = format!("{llm:?}");
    assert!(
        !text.contains(KEY) && text.contains("OpenAiModel"),
        "{text}"
    );
}

#[tokio::test]
async fn an_unmappable_request_fails_before_any_network_call() {
    let server = cassette(&[ok(1, "asla")]).await;
    let llm = model(&server.base_url(), &fake_clock());
    let error = complete(&llm, ChatRequest::default()).await.unwrap_err();
    assert!(matches!(error, ProviderError::InvalidRequest(_)), "{error}");
    assert!(server.requests().is_empty());
}
