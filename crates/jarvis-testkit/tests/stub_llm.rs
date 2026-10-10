//! `StubLlm` (Tasarım 0011 test planı). Oracle: kaydedilen istekler ve dönen değerler.

use std::time::Duration;

use jarvis_provider::{
    ChatModel, ChatRequest, ChatResponse, FinishReason, NoObserver, ProviderError,
};
use jarvis_testkit::{Scripted, StubLlm};
use jarvis_types::{Message, Role};
use tokio_util::sync::CancellationToken;

fn reply(text: &str) -> ChatResponse {
    ChatResponse {
        message: Message::text(Role::Assistant, text),
        finish: FinishReason::Stop,
        usage: None,
        model: "stub".to_owned(),
    }
}

fn ask(text: &str) -> ChatRequest {
    ChatRequest {
        messages: vec![Message::text(Role::User, text)],
        ..ChatRequest::default()
    }
}

async fn call(llm: &StubLlm, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
    llm.complete(request, CancellationToken::new(), &NoObserver)
        .await
}

#[tokio::test]
async fn replies_follow_the_script_and_every_request_is_recorded() {
    let llm = StubLlm::new([Scripted::Reply(reply("bir")), Scripted::Reply(reply("iki"))]);
    assert_eq!(llm.remaining(), 2);
    assert_eq!(call(&llm, ask("a")).await, Ok(reply("bir")));
    assert_eq!(llm.remaining(), 1);
    assert_eq!(call(&llm, ask("b")).await, Ok(reply("iki")));
    assert_eq!(llm.requests(), [ask("a"), ask("b")]);
    assert_eq!(llm.remaining(), 0);
}

#[tokio::test]
async fn a_call_beyond_the_script_is_an_explicit_error_naming_the_call_number() {
    let llm = StubLlm::new([Scripted::Reply(reply("tek"))]);
    call(&llm, ask("a")).await.unwrap();
    let error = call(&llm, ask("b")).await.unwrap_err();
    assert_eq!(
        error,
        ProviderError::InvalidRequest("StubLlm: beklenmeyen çağrı #2".to_owned())
    );
    assert_eq!(llm.requests().len(), 2, "beklenmeyen istek de kaydedilir");
}

#[tokio::test]
async fn scripted_failures_are_returned_as_is() {
    let failure = ProviderError::Unavailable {
        status: Some(503),
        message: "bakım".to_owned(),
    };
    let llm = StubLlm::new([Scripted::Fail(failure.clone())]);
    assert_eq!(call(&llm, ask("a")).await, Err(failure));
}

#[tokio::test]
async fn wait_for_cancel_pends_until_the_token_fires() {
    let llm = StubLlm::new([Scripted::WaitForCancel]);
    let cancel = CancellationToken::new();
    let pending = tokio::time::timeout(
        Duration::from_millis(50),
        llm.complete(ask("a"), cancel.clone(), &NoObserver),
    )
    .await;
    assert!(pending.is_err(), "iptal gelene kadar tamamlanmamalı");

    let llm = StubLlm::new([Scripted::WaitForCancel]);
    let waiting = llm.complete(ask("a"), cancel.clone(), &NoObserver);
    cancel.cancel();
    assert_eq!(waiting.await, Err(ProviderError::Cancelled));
}
