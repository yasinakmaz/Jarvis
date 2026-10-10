//! `RawChat::forward` (passthrough, Tasarım 0006/0009). Oracle: kaset sunucusunun aldığı
//! gövde ve elle yazılmış beklenen değerler.

use std::time::Duration;

use futures_util::StreamExt;
use jarvis_provider::{NoObserver, ProviderError, RawChat, RawResponse, RetryReason};
use jarvis_testkit::CassetteServer;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::support::{MODEL, Notices, drive, fake_clock, model, within};

#[expect(
    clippy::unwrap_used,
    reason = "yardımcı: geçersiz sabit girdide test düşmeli"
)]
async fn serve(turns: &[Value]) -> CassetteServer {
    let lines: Vec<String> = turns.iter().map(Value::to_string).collect();
    CassetteServer::replay_str(&lines.join("\n")).await.unwrap()
}

fn turn(n: u32, request: &Value, response: &Value) -> Value {
    json!({"turn": n,
        "request": {"method": "POST", "path": "/v1/chat/completions", "body": request},
        "response": response})
}

async fn forward(llm: &impl RawChat, body: Value) -> Result<RawResponse, ProviderError> {
    within(llm.forward(body, CancellationToken::new(), &NoObserver)).await
}

fn user_body(model_name: &str) -> Value {
    json!({"model": model_name, "temperature": 0.2,
           "messages": [{"role": "user", "content": "selam"}]})
}

#[tokio::test]
async fn json_passthrough_swaps_only_the_model_and_returns_the_body_untouched() {
    let reply = json!({"id": "x", "object": "chat.completion", "anything": [1, 2, {"k": null}]});
    let server = serve(&[turn(
        1,
        &user_body(MODEL),
        &json!({"status": 200, "body": reply}),
    )])
    .await;
    let llm = model(&server.base_url(), &fake_clock());
    let RawResponse::Json(got) = forward(&llm, user_body("planner")).await.unwrap() else {
        panic!("JSON bekleniyordu");
    };
    assert_eq!(got, reply, "yanıt alan modeline çevrilmeden iletilir");
    server.assert_exhausted().unwrap();
}

#[tokio::test]
async fn streaming_passthrough_yields_raw_chunks_without_the_done_marker() {
    let mut request = user_body(MODEL);
    request["stream"] = json!(true);
    let server = serve(&[turn(
        1,
        &request,
        &json!({"status": 200, "stream": [
            "{\"i\":1}", "{\"i\":2,\"extra\":{\"a\":true}}", "[DONE]"]}),
    )])
    .await;
    let llm = model(&server.base_url(), &fake_clock());
    let mut body = user_body("fast");
    body["stream"] = json!(true);
    let RawResponse::Stream(stream) = forward(&llm, body).await.unwrap() else {
        panic!("akış bekleniyordu");
    };
    let items: Vec<_> = stream.collect().await;
    assert_eq!(
        items,
        [
            Ok(json!({"i": 1})),
            Ok(json!({"i": 2, "extra": {"a": true}}))
        ]
    );
    server.assert_exhausted().unwrap();
}

#[tokio::test]
async fn a_garbled_chunk_ends_the_stream_with_an_invalid_response_error() {
    let mut request = user_body(MODEL);
    request["stream"] = json!(true);
    let server = serve(&[turn(
        1,
        &request,
        &json!({"status": 200, "stream": ["{\"i\":1}", "bozuk", "{\"i\":3}"]}),
    )])
    .await;
    let llm = model(&server.base_url(), &fake_clock());
    let mut body = user_body("fast");
    body["stream"] = json!(true);
    let RawResponse::Stream(stream) = forward(&llm, body).await.unwrap() else {
        panic!("akış bekleniyordu");
    };
    let items: Vec<_> = stream.collect().await;
    assert_eq!(items.len(), 2, "{items:?}");
    assert_eq!(items.first(), Some(&Ok(json!({"i": 1}))));
    assert!(
        matches!(items.get(1), Some(Err(ProviderError::InvalidResponse(_)))),
        "{items:?}"
    );
}

#[tokio::test]
async fn explicit_stream_false_is_a_plain_json_call() {
    let mut request = user_body(MODEL);
    request["stream"] = json!(false);
    let server = serve(&[turn(
        1,
        &request,
        &json!({"status": 200, "body": {"ok": 1}}),
    )])
    .await;
    let llm = model(&server.base_url(), &fake_clock());
    let mut body = user_body("fast");
    body["stream"] = json!(false);
    assert!(matches!(
        forward(&llm, body).await,
        Ok(RawResponse::Json(_))
    ));
}

#[tokio::test]
async fn a_body_that_is_not_an_object_is_refused_before_any_network_call() {
    let server = serve(&[turn(
        1,
        &user_body(MODEL),
        &json!({"status": 200, "body": {}}),
    )])
    .await;
    let llm = model(&server.base_url(), &fake_clock());
    for body in [json!([1, 2]), json!("metin"), Value::Null] {
        let error = forward(&llm, body).await.unwrap_err();
        assert!(matches!(error, ProviderError::InvalidRequest(_)), "{error}");
    }
    assert!(server.requests().is_empty());
}

#[tokio::test]
async fn setup_failures_are_retried_with_the_same_backoff_and_reported() {
    let mut request = user_body(MODEL);
    request["stream"] = json!(true);
    let rate_limited = json!({"status": 429, "body": {"error": {"message": "x", "type": "t"}}});
    let server = serve(&[
        turn(1, &request, &rate_limited),
        turn(
            2,
            &request,
            &json!({"status": 200, "stream": ["{\"i\":1}", "[DONE]"]}),
        ),
    ])
    .await;
    let clock = fake_clock();
    let llm = model(&server.base_url(), &clock);
    let notices = Notices::default();
    let mut body = user_body("fast");
    body["stream"] = json!(true);
    let response = drive(
        &clock,
        llm.forward(body, CancellationToken::new(), &notices),
    )
    .await
    .unwrap();
    assert!(matches!(response, RawResponse::Stream(_)));
    assert_eq!(clock.sleeps(), [Duration::from_millis(500)]);
    assert_eq!(
        notices.all().first().map(|n| n.reason),
        Some(RetryReason::RateLimited)
    );
}

#[tokio::test]
async fn upstream_rejections_pass_through_as_rejected_errors() {
    let denied = json!({"status": 401, "body": {"error": {"message": "no", "type": "auth"}}});
    let server = serve(&[turn(1, &user_body(MODEL), &denied)]).await;
    let llm = model(&server.base_url(), &fake_clock());
    let error = forward(&llm, user_body("planner")).await.unwrap_err();
    assert!(
        matches!(error, ProviderError::Rejected { status: 401, .. }),
        "{error}"
    );
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn cancelling_a_slow_passthrough_returns_cancelled() {
    let slow = json!({"status": 200, "body": {}, "delay_ms": 5000});
    let server = serve(&[turn(1, &user_body(MODEL), &slow)]).await;
    let llm = std::sync::Arc::new(model(&server.base_url(), &fake_clock()));
    let cancel = CancellationToken::new();
    let task = {
        let (llm, cancel) = (llm.clone(), cancel.clone());
        tokio::spawn(async move { llm.forward(user_body("planner"), cancel, &NoObserver).await })
    };
    crate::support::until(|| !server.requests().is_empty()).await;
    cancel.cancel();
    let outcome = task.await.unwrap();
    assert!(
        matches!(outcome, Err(ProviderError::Cancelled)),
        "{outcome:?}"
    );
}
