//! SSE çerçeveleri (saf). Biçim: `id: <seq>\nevent: <type>\ndata: <json>\n\n`.

use crate::Event;

/// Kayıp bildiriminin SSE olay adı.
pub const LAGGED_EVENT: &str = "events.lagged";

/// Olayın SSE çerçevesi.
///
/// # Errors
///
/// Olay JSON'a çevrilemezse (pratikte olmaz; tipler serileştirilebilir).
pub fn sse_frame(event: &Event) -> Result<String, serde_json::Error> {
    let data = serde_json::to_string(event)?;
    Ok(format!(
        "id: {}\nevent: {}\n{}\n",
        event.seq,
        event.kind.wire_name(),
        data_lines(&data)
    ))
}

/// Kayıp bildirimi çerçevesi. `id:` taşımaz; istemcinin `Last-Event-ID`'si ilerlemez.
#[must_use]
pub fn sse_lagged_frame(missed: u64) -> String {
    let data = format!("{{\"missed\":{missed}}}");
    format!("event: {LAGGED_EVENT}\n{}\n", data_lines(&data))
}

/// Her satıra `data: ` öneki koyar (SSE çok satırlı veri kuralı).
fn data_lines(payload: &str) -> String {
    payload
        .split('\n')
        .flat_map(|line| ["data: ", line, "\n"])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_payload_line_gets_its_own_data_prefix() {
        assert_eq!(data_lines("a"), "data: a\n");
        assert_eq!(data_lines("a\nb"), "data: a\ndata: b\n");
        assert_eq!(data_lines(""), "data: \n");
    }
}
