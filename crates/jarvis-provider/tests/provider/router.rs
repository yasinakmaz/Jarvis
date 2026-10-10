//! `Router` (Tasarım 0006): rol çözümleme, anahtar okuma, paylaşılan hız sınırı.

use std::collections::BTreeMap;
use std::time::Duration;

use jarvis_provider::{ChatRequest, NoObserver, ProviderError, RoleName, Router};
use jarvis_testkit::CassetteServer;
use jarvis_types::{Message, Role};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::support::{
    KEY, MODEL, config, config_with_vision, drive, env_with_keys, fake_clock, within,
};

fn ask() -> ChatRequest {
    ChatRequest {
        messages: vec![Message::text(Role::User, "merhaba")],
        ..ChatRequest::default()
    }
}

fn exchange(n: u32, model: &str, text: &str) -> String {
    json!({"turn": n,
        "request": {"method": "POST", "path": "/v1/chat/completions",
            "body": {"model": model, "messages": [{"role": "user", "content": "merhaba"}]}},
        "response": {"status": 200, "body": {
            "id": "c", "object": "chat.completion", "created": 1, "model": model,
            "choices": [{"index": 0, "message": {"role": "assistant", "content": text},
                         "finish_reason": "stop"}]}}})
    .to_string()
}

#[expect(
    clippy::unwrap_used,
    reason = "yardımcı: geçersiz sabit girdide test düşmeli"
)]
async fn serve(lines: &[String]) -> CassetteServer {
    CassetteServer::replay_str(&lines.join("\n")).await.unwrap()
}

#[test]
fn roles_resolve_to_models_and_unassigned_roles_to_none() {
    let router = Router::from_config(
        &config("http://127.0.0.1:9/v1", 40),
        &env_with_keys(),
        fake_clock(),
    )
    .unwrap();
    assert!(router.chat(RoleName::Planner).is_some());
    assert!(router.chat(RoleName::Fast).is_some());
    assert!(router.raw(RoleName::Planner).is_some());
    assert!(router.chat(RoleName::Vision).is_none());
    assert!(router.raw(RoleName::Vision).is_none());
    let described = router.describe();
    let roles: Vec<RoleName> = described.iter().map(|i| i.role).collect();
    assert_eq!(roles, [RoleName::Planner, RoleName::Fast]);
    let first = described.first().unwrap();
    assert_eq!(
        (
            first.provider.as_str(),
            first.model.as_str(),
            first.base_url.as_str()
        ),
        ("test", MODEL, "http://127.0.0.1:9/v1")
    );
    assert_eq!(first.capabilities, ["tools", "streaming"]);
}

#[test]
fn a_missing_or_empty_key_fails_construction_naming_the_variable() {
    let config = config("http://127.0.0.1:9/v1", 40);
    for env in [
        BTreeMap::new(),
        BTreeMap::from([("TEST_KEY".to_owned(), String::new())]),
    ] {
        let error = Router::from_config(&config, &env, fake_clock()).unwrap_err();
        assert_eq!(
            error,
            ProviderError::MissingKey {
                provider: "test".to_owned(),
                env_var: "TEST_KEY".to_owned()
            }
        );
        assert!(error.to_string().contains("TEST_KEY"));
    }
}

#[test]
fn describe_never_contains_a_key() {
    let router = Router::from_config(
        &config("http://127.0.0.1:9/v1", 40),
        &env_with_keys(),
        fake_clock(),
    )
    .unwrap();
    let text = format!("{:?} {:?}", router.describe(), router);
    assert!(!text.contains(KEY), "{text}");
}

#[tokio::test]
async fn roles_sharing_a_provider_share_its_rate_limit() {
    let server = serve(&[
        exchange(1, MODEL, "bir"),
        exchange(2, MODEL, "iki"),
        exchange(3, MODEL, "üç"),
    ])
    .await;
    let clock = fake_clock();
    let router = Router::from_config(
        &config_with_vision(&server.base_url(), "http://127.0.0.1:9/v1"),
        &env_with_keys(),
        clock.clone(),
    )
    .unwrap();
    let planner = router.chat(RoleName::Planner).unwrap();
    let fast = router.chat(RoleName::Fast).unwrap();
    drive(&clock, async {
        for llm in [&planner, &fast, &planner] {
            llm.complete(ask(), CancellationToken::new(), &NoObserver)
                .await
                .unwrap();
        }
    })
    .await;
    assert_eq!(
        clock.sleeps(),
        [Duration::from_secs(30)],
        "dakikada 2 istek, iki rol için ortak"
    );
}

#[tokio::test]
async fn the_vision_role_talks_to_its_own_provider_with_its_own_key_and_model() {
    let eye = serve(&[exchange(1, "vision/eye-model", "gördüm")]).await;
    let router = Router::from_config(
        &config_with_vision("http://127.0.0.1:9/v1", &eye.base_url()),
        &env_with_keys(),
        fake_clock(),
    )
    .unwrap();
    let vision = router.chat(RoleName::Vision).unwrap();
    let reply = within(vision.complete(ask(), CancellationToken::new(), &NoObserver))
        .await
        .unwrap();
    assert_eq!(reply.message.joined_text(), "gördüm");
    let sent = eye.recorded();
    let auth = sent
        .first()
        .unwrap()
        .headers
        .iter()
        .find(|(k, _)| k == "authorization");
    assert_eq!(auth.map(|(_, v)| v.as_str()), Some("Bearer sk-eye-KEY-987"));
    eye.assert_exhausted().unwrap();
    let described = router.describe();
    assert_eq!(described.len(), 3);
}
