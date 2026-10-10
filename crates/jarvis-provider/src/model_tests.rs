use serde_json::json;

use super::{RawResponse, RoleName};

#[test]
fn role_names_round_trip_through_their_wire_names() {
    let names: Vec<&str> = RoleName::ALL.iter().map(|r| r.as_str()).collect();
    assert_eq!(names, ["planner", "vision", "fast"]);
    for role in RoleName::ALL {
        assert_eq!(RoleName::parse(role.as_str()), Some(role));
        assert_eq!(role.to_string(), role.as_str());
    }
    assert_eq!(RoleName::parse("jarvis"), None);
    assert_eq!(RoleName::parse(""), None);
    assert_eq!(RoleName::parse("Planner"), None, "büyük/küçük harf duyarlı");
}

#[test]
fn raw_response_debug_never_dumps_the_stream() {
    let json = format!("{:?}", RawResponse::Json(json!({"a": 1})));
    assert_eq!(json, r#"Json(Object {"a": Number(1)})"#);
    let stream = format!(
        "{:?}",
        RawResponse::Stream(Box::pin(futures_util::stream::empty()))
    );
    assert_eq!(stream, "Stream(..)");
}
