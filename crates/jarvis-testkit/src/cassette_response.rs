//! Kaset turunun HTTP yanıtına çevrilmesi.

use axum::body::Body;
use axum::http::StatusCode;
use axum::response::Response;
use serde_json::json;

use crate::cassette::Turn;

/// Uyuşmazlık/tükenme yanıtı: 500 ve `OpenAI` hata gövdesi biçiminde ayrıntı.
pub fn failure(message: &str) -> Response {
    let body = json!({"error": {"message": message, "type": "cassette_error"}});
    build(
        StatusCode::INTERNAL_SERVER_ERROR,
        "application/json",
        &[],
        body.to_string(),
    )
}

pub fn replay_response(turn: &Turn) -> Response {
    let spec = &turn.response;
    let (content_type, body) = match (&spec.body, &spec.stream, &spec.text) {
        (Some(json), _, _) => ("application/json", json.to_string()),
        (_, Some(chunks), _) => (
            "text/event-stream",
            chunks.iter().fold(String::new(), |mut sse, chunk| {
                sse.push_str("data: ");
                sse.push_str(chunk);
                sse.push_str("\n\n");
                sse
            }),
        ),
        (_, _, Some(text)) => ("text/plain; charset=utf-8", text.clone()),
        _ => ("text/plain; charset=utf-8", String::new()),
    };
    let status = StatusCode::from_u16(spec.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let extra: Vec<(&str, &str)> = spec
        .headers
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    build(status, content_type, &extra, body)
}

fn build(status: StatusCode, content_type: &str, extra: &[(&str, &str)], body: String) -> Response {
    let overridden = extra
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("content-type"));
    let mut builder = Response::builder().status(status);
    if !overridden {
        builder = builder.header("content-type", content_type);
    }
    for (name, value) in extra {
        builder = builder.header(*name, *value);
    }
    builder.body(Body::from(body)).unwrap_or_else(|error| {
        let mut fallback = Response::new(Body::from(format!("kaset yanıtı kurulamadı: {error}")));
        *fallback.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        fallback
    })
}
