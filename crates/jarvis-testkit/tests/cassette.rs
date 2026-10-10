//! Kaset ayrıştırma (Tasarım 0011 test planı). Oracle: elle yazılmış kaset metinleri.

use jarvis_testkit::CassetteError;
use jarvis_testkit::cassette::{first_difference, parse};
use serde_json::{Value, json};

fn turn(n: u32, body: &Value, response: &Value) -> String {
    json!({"turn": n,
        "request": {"method": "POST", "path": "/v1/chat/completions", "body": body},
        "response": response})
    .to_string()
}

fn turn_1() -> String {
    turn(
        1,
        &json!({"a": 1}),
        &json!({"status": 200, "body": {"ok": true}}),
    )
}

fn turn_2() -> String {
    turn(
        2,
        &json!({"a": 2}),
        &json!({"status": 200, "stream": ["{\"x\":1}", "[DONE]"]}),
    )
}

#[test]
fn well_formed_cassettes_parse_in_order_ignoring_blank_lines() {
    let turns = parse(&format!("{}\n\n{}\n", turn_1(), turn_2())).unwrap();
    assert_eq!(turns.len(), 2);
    assert_eq!(turns.first().map(|t| t.turn), Some(1));
    assert_eq!(
        turns.first().and_then(|t| t.response.body.clone()),
        Some(json!({"ok": true}))
    );
    assert_eq!(
        turns.get(1).and_then(|t| t.response.stream.clone()),
        Some(vec!["{\"x\":1}".to_owned(), "[DONE]".to_owned()])
    );
}

fn with_response(response: &Value) -> String {
    turn(1, &json!({}), response)
}

#[test]
fn malformed_cassettes_fail_with_the_line_number() {
    let cases: [(String, usize, &str); 5] = [
        (format!("{}\n{{bozuk", turn_1()), 2, "JSON"),
        (turn_2(), 1, "tur numarası 1 olmalı"),
        (
            format!("{}\n{}", turn_1(), turn_1()),
            2,
            "tur numarası 2 olmalı",
        ),
        (with_response(&json!({"status": 200})), 1, "tam biri"),
        (
            with_response(&json!({"status": 200, "body": {}, "text": "x"})),
            1,
            "tam biri",
        ),
    ];
    for (text, line, needle) in cases {
        let error = parse(&text).unwrap_err();
        let CassetteError::Parse {
            line: found,
            detail,
        } = &error
        else {
            panic!("Parse hatası bekleniyordu: {error}");
        };
        assert_eq!(*found, line, "{text}");
        assert!(detail.contains(needle), "{detail}");
    }
}

#[test]
fn an_empty_cassette_is_an_error() {
    assert_eq!(parse("  \n\n").unwrap_err(), CassetteError::Empty);
}

#[test]
fn first_difference_names_the_json_pointer_of_the_first_mismatch() {
    let expected = json!({"model": "m", "messages": [{"role": "user", "content": "a"}]});
    assert_eq!(first_difference(&expected, &expected), None);
    let actual = json!({"model": "m", "messages": [{"role": "user", "content": "b"}]});
    let diff = first_difference(&expected, &actual).unwrap();
    assert!(diff.starts_with("/messages/0/content"), "{diff}");
    assert!(diff.contains("\"a\"") && diff.contains("\"b\""), "{diff}");

    let missing = json!({"model": "m"});
    let diff = first_difference(&expected, &missing).unwrap();
    assert!(
        diff.starts_with("/messages") && diff.contains("eksik"),
        "{diff}"
    );
    let extra = json!({"model": "m", "messages": [], "tools": []});
    let diff = first_difference(&json!({"model": "m", "messages": []}), &extra).unwrap();
    assert!(
        diff.starts_with("/tools") && diff.contains("fazladan"),
        "{diff}"
    );
    let length = first_difference(&json!([1, 2]), &json!([1])).unwrap();
    assert!(length.contains("uzunluğu"), "{length}");
}
