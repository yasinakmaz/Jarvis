//! Kaset sunucusu (Tasarım 0011 test planı). Oracle: elle yazılmış HTTP istemcisi ve kaset
//! metinleri; sunucunun kendi kaydına güvenilmez.

use std::time::{Duration, Instant};

use jarvis_testkit::{CassetteError, CassetteServer};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

struct Raw {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Raw {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

fn host_of(base_url: &str) -> String {
    base_url
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or_default()
        .to_owned()
}

#[expect(clippy::unwrap_used, reason = "yardımcı: G/Ç hatasında test düşmeli")]
async fn send(base_url: &str, method: &str, path: &str, extra: &str, body: &str) -> Raw {
    let host = host_of(base_url);
    let mut stream = TcpStream::connect(&host).await.unwrap();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n{extra}\
         Content-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).await.unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let (head, body) = text.split_once("\r\n\r\n").unwrap();
    let mut lines = head.lines();
    let status = lines
        .next()
        .unwrap()
        .split(' ')
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let headers = lines
        .filter_map(|line| line.split_once(": "))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    Raw {
        status,
        headers,
        body: body.to_owned(),
    }
}

async fn post(base_url: &str, body: &Value) -> Raw {
    send(
        base_url,
        "POST",
        "/v1/chat/completions",
        "",
        &body.to_string(),
    )
    .await
}

fn turn(n: u32, request_body: &Value, response: &Value) -> String {
    json!({"turn": n,
        "request": {"method": "POST", "path": "/v1/chat/completions", "body": request_body},
        "response": response})
    .to_string()
}

#[expect(
    clippy::unwrap_used,
    reason = "yardımcı: geçersiz kasette test düşmeli"
)]
async fn server(lines: &[String]) -> CassetteServer {
    CassetteServer::replay_str(&lines.join("\n")).await.unwrap()
}

#[tokio::test]
async fn a_matching_request_gets_the_recorded_json_response() {
    let cassette = server(&[turn(
        1,
        &json!({"model": "m", "a": 1}),
        &json!({"status": 200, "headers": {"x-extra": "1"}, "body": {"ok": true}}),
    )])
    .await;
    let reply = post(&cassette.base_url(), &json!({"a": 1, "model": "m"})).await;
    assert_eq!(reply.status, 200);
    assert_eq!(
        serde_json::from_str::<Value>(&reply.body).unwrap(),
        json!({"ok": true})
    );
    assert_eq!(reply.header("content-type"), Some("application/json"));
    assert_eq!(reply.header("x-extra"), Some("1"));
    cassette.assert_exhausted().unwrap();
    assert_eq!(cassette.requests(), [json!({"a": 1, "model": "m"})]);
}

#[tokio::test]
async fn stream_and_text_turns_are_framed_as_sse_and_plain_text() {
    let cassette = server(&[
        turn(
            1,
            &json!({"s": 1}),
            &json!({"status": 200, "stream": ["{\"x\":1}", "[DONE]"]}),
        ),
        turn(
            2,
            &json!({"s": 2}),
            &json!({"status": 502, "text": "<html>kötü geçit</html>",
                    "headers": {"content-type": "text/html"}}),
        ),
    ])
    .await;
    let sse = post(&cassette.base_url(), &json!({"s": 1})).await;
    assert_eq!(sse.header("content-type"), Some("text/event-stream"));
    assert_eq!(sse.body, "data: {\"x\":1}\n\ndata: [DONE]\n\n");
    let html = post(&cassette.base_url(), &json!({"s": 2})).await;
    assert_eq!(
        (html.status, html.body.as_str()),
        (502, "<html>kötü geçit</html>")
    );
    assert_eq!(html.header("content-type"), Some("text/html"));
    cassette.assert_exhausted().unwrap();
}

#[tokio::test]
async fn a_body_mismatch_is_a_500_naming_the_first_difference_and_fails_the_cassette() {
    let cassette = server(&[turn(
        1,
        &json!({"messages": [{"content": "a"}]}),
        &json!({"status": 200, "body": {}}),
    )])
    .await;
    let reply = post(
        &cassette.base_url(),
        &json!({"messages": [{"content": "b"}]}),
    )
    .await;
    assert_eq!(reply.status, 500);
    assert!(reply.body.contains("/messages/0/content"), "{}", reply.body);
    let error = cassette.assert_exhausted().unwrap_err();
    assert!(error.to_string().contains("tur 1"), "{error}");
    assert!(error.to_string().contains("/messages/0/content"), "{error}");
}

#[tokio::test]
async fn method_path_and_non_json_bodies_must_match_too() {
    let cassette = server(&[
        turn(1, &json!({}), &json!({"status": 200, "body": {}})),
        turn(2, &json!({}), &json!({"status": 200, "body": {}})),
        turn(3, &json!({}), &json!({"status": 200, "body": {}})),
    ])
    .await;
    let url = cassette.base_url();
    let wrong_path = send(&url, "POST", "/v1/models", "", "{}").await;
    assert!(
        wrong_path.status == 500 && wrong_path.body.contains("yol"),
        "{}",
        wrong_path.body
    );
    let wrong_method = send(&url, "GET", "/v1/chat/completions", "", "{}").await;
    assert!(wrong_method.status == 500 && wrong_method.body.contains("yöntem"));
    let not_json = send(&url, "POST", "/v1/chat/completions", "", "düz metin").await;
    assert!(
        not_json.status == 500 && not_json.body.contains("JSON"),
        "{}",
        not_json.body
    );
    assert!(cassette.assert_exhausted().is_err());
}

#[tokio::test]
async fn requests_beyond_the_cassette_and_unused_turns_are_errors() {
    let cassette = server(&[turn(1, &json!({}), &json!({"status": 200, "body": {}}))]).await;
    let unused = cassette.assert_exhausted().unwrap_err();
    assert!(
        unused.to_string().contains("kullanılmayan 1 tur"),
        "{unused}"
    );

    assert_eq!(post(&cassette.base_url(), &json!({})).await.status, 200);
    cassette.assert_exhausted().unwrap();
    let extra = post(&cassette.base_url(), &json!({})).await;
    assert_eq!(extra.status, 500);
    assert!(extra.body.contains("kaset tükendi"), "{}", extra.body);
    let error = cassette.assert_exhausted().unwrap_err();
    assert!(error.to_string().contains("kaset tükendi"), "{error}");
}

#[tokio::test]
async fn request_headers_are_recorded_lowercase_for_inspection() {
    let cassette = server(&[turn(1, &json!({}), &json!({"status": 200, "body": {}}))]).await;
    send(
        &cassette.base_url(),
        "POST",
        "/v1/chat/completions",
        "Authorization: Bearer abc\r\n",
        "{}",
    )
    .await;
    let recorded = cassette.recorded();
    let first = recorded.first().unwrap();
    assert_eq!(
        (first.method.as_str(), first.path.as_str()),
        ("POST", "/v1/chat/completions")
    );
    assert!(
        first
            .headers
            .contains(&("authorization".to_owned(), "Bearer abc".to_owned()))
    );
}

#[tokio::test]
async fn delay_ms_holds_the_response_back_in_real_time() {
    let cassette = server(&[turn(
        1,
        &json!({}),
        &json!({"status": 200, "body": {}, "delay_ms": 120}),
    )])
    .await;
    let started = Instant::now();
    post(&cassette.base_url(), &json!({})).await;
    assert!(
        started.elapsed() >= Duration::from_millis(120),
        "{:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn cassette_files_load_from_disk_and_bad_inputs_fail_loudly() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("k.jsonl");
    std::fs::write(
        &path,
        turn(1, &json!({}), &json!({"status": 200, "body": {}})),
    )
    .unwrap();
    assert!(CassetteServer::replay(&path).await.is_ok());

    let missing = CassetteServer::replay(&dir.path().join("yok.jsonl"))
        .await
        .err()
        .unwrap();
    assert!(matches!(&missing, CassetteError::Io { path, .. } if path.ends_with("yok.jsonl")));
    let bad_status = turn(1, &json!({}), &json!({"status": 42, "body": {}}));
    let error = CassetteServer::replay_str(&bad_status).await.err().unwrap();
    assert!(error.to_string().contains("HTTP durumu 42"), "{error}");
    assert!(CassetteServer::replay_str("").await.is_err());
}

#[tokio::test]
async fn dropping_the_server_closes_the_port() {
    let cassette = server(&[turn(1, &json!({}), &json!({"status": 200, "body": {}}))]).await;
    let host = host_of(&cassette.base_url());
    drop(cassette);
    for _ in 0..200 {
        if TcpStream::connect(&host).await.is_err() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("sunucu kapanmadı: {host}");
}
