//! Tel biçimi (Tasarım 0005 test planı, L1). Oracle: Mimari §3 olay tablosu ve elle yazılmış
//! JSON/SSE metinleri.

use jarvis_events::{
    Event, EventKind, LAGGED_EVENT, StepPhase, ToolCallSummary, sse_frame, sse_lagged_frame,
};
use jarvis_types::{
    ErrorCode, EventSeq, RiskLevel, RunId, SessionId, Timestamp, ToolCall, ToolCallId, ToolName,
    TraceId, Verification,
};
use serde_json::json;

const RUN: &str = "01900000-0000-7000-8000-000000000001";
const TRACE: &str = "01900000-0000-7000-8000-000000000002";

#[expect(clippy::unwrap_used, reason = "test sabiti; geçersizse test düşmeli")]
fn call_id() -> ToolCallId {
    ToolCallId::new("call_1".to_owned()).unwrap()
}

#[expect(clippy::unwrap_used, reason = "test sabiti; geçersizse test düşmeli")]
fn summary(arguments: &str) -> ToolCallSummary {
    ToolCallSummary {
        id: call_id(),
        name: ToolName::parse("read_file").unwrap(),
        arguments: arguments.to_owned(),
    }
}

/// Mimari §3 tablosundaki M1 olayları (onay olayları M2'de).
fn samples() -> Vec<(EventKind, &'static str)> {
    vec![
        (EventKind::RunStarted { session_id: None }, "run.started"),
        (
            EventKind::RunStep {
                step: 1,
                phase: StepPhase::Plan,
                detail: String::new(),
            },
            "run.step",
        ),
        (EventKind::RunFinished { steps: 3 }, "run.finished"),
        (
            EventKind::RunFailed {
                code: ErrorCode::Internal,
                message: String::new(),
            },
            "run.failed",
        ),
        (
            EventKind::ToolRequested {
                call: summary("{}"),
                risk: RiskLevel::Read,
            },
            "tool.requested",
        ),
        (
            EventKind::ToolCompleted {
                call_id: call_id(),
                verification: Verification::Verified,
            },
            "tool.completed",
        ),
        (
            EventKind::DesktopUnavailable {
                backend: "portal".to_owned(),
                reason: String::new(),
            },
            "desktop.unavailable",
        ),
        (EventKind::Halt { cancelled_runs: 0 }, "halt"),
    ]
}

#[test]
fn every_kind_uses_the_architecture_wire_name() {
    for (kind, name) in samples() {
        assert_eq!(kind.wire_name(), name);
        assert_eq!(serde_json::to_value(&kind).unwrap()["type"], name);
    }
}

#[expect(clippy::unwrap_used, reason = "test sabiti; geçersizse test düşmeli")]
fn sample_event() -> Event {
    Event {
        seq: EventSeq::new(7),
        at: Timestamp::parse_rfc3339("2026-10-09T12:00:00Z").unwrap(),
        run_id: Some(RunId::parse(RUN).unwrap()),
        trace_id: TraceId::parse(TRACE).unwrap(),
        kind: EventKind::RunStep {
            step: 2,
            phase: StepPhase::Act,
            detail: "ls".to_owned(),
        },
    }
}

#[test]
fn event_json_is_flat_and_round_trips() {
    let event = sample_event();
    let expected = json!({
        "seq": 7, "at": "2026-10-09T12:00:00Z", "run_id": RUN, "trace_id": TRACE,
        "type": "run.step", "step": 2, "phase": "act", "detail": "ls",
    });
    assert_eq!(serde_json::to_value(&event).unwrap(), expected);
    assert_eq!(serde_json::from_value::<Event>(expected).unwrap(), event);
    for (kind, _) in samples() {
        let event = Event {
            kind,
            ..sample_event()
        };
        let text = serde_json::to_string(&event).unwrap();
        assert_eq!(serde_json::from_str::<Event>(&text).unwrap(), event);
    }
}

#[test]
fn phases_have_snake_case_wire_names() {
    let phases = [
        (StepPhase::Plan, "plan"),
        (StepPhase::Act, "act"),
        (StepPhase::Verify, "verify"),
        (StepPhase::Retry, "retry"),
    ];
    for (phase, name) in phases {
        assert_eq!(serde_json::to_value(phase).unwrap(), name);
    }
}

#[test]
fn sse_frame_matches_the_hand_written_frame() {
    let frame = sse_frame(&sample_event()).unwrap();
    let data = format!(
        concat!(
            r#"{{"seq":7,"at":"2026-10-09T12:00:00Z","run_id":"{}","trace_id":"{}","#,
            r#""type":"run.step","step":2,"phase":"act","detail":"ls"}}"#,
        ),
        RUN, TRACE
    );
    assert_eq!(frame, format!("id: 7\nevent: run.step\ndata: {data}\n\n"));
}

#[test]
fn sse_frame_keeps_newlines_inside_data_escaped() {
    let mut event = sample_event();
    event.kind = EventKind::RunStep {
        step: 1,
        phase: StepPhase::Plan,
        detail: "a\nb".to_owned(),
    };
    let frame = sse_frame(&event).unwrap();
    assert_eq!(frame.lines().filter(|l| l.starts_with("data: ")).count(), 1);
    assert!(frame.contains(r#""detail":"a\nb""#));
    assert!(frame.ends_with("\n\n"));
}

#[test]
fn lagged_frame_reports_the_missed_count_without_an_id() {
    assert_eq!(LAGGED_EVENT, "events.lagged");
    assert_eq!(
        sse_lagged_frame(6),
        "event: events.lagged\ndata: {\"missed\":6}\n\n"
    );
}

#[test]
fn tool_call_summary_keeps_short_arguments_verbatim() {
    let call = ToolCall {
        id: call_id(),
        name: ToolName::parse("read_file").unwrap(),
        arguments: json!({"path": "/tmp/a"}),
    };
    assert_eq!(
        ToolCallSummary::from_call(&call),
        summary(r#"{"path":"/tmp/a"}"#)
    );
}

#[test]
fn tool_call_summary_truncates_long_arguments_on_char_boundaries() {
    let max = ToolCallSummary::MAX_ARGUMENTS;
    assert_eq!(max, 512);
    // `["şşş…"]`: JSON metni 2 + 2 + n karakter; çok baytlı karakterle bayt değil karakter sayılır.
    for (n, truncated) in [(max - 4, false), (max - 3, true), (2 * max, true)] {
        let call = ToolCall {
            id: call_id(),
            name: ToolName::parse("read_file").unwrap(),
            arguments: json!(["ş".repeat(n)]),
        };
        let full = format!("[\"{}\"]", "ş".repeat(n));
        let preview = ToolCallSummary::from_call(&call).arguments;
        if truncated {
            let kept: String = full.chars().take(max).collect();
            assert_eq!(preview, format!("{kept}…"), "n = {n}");
        } else {
            assert_eq!(preview, full, "n = {n}");
        }
    }
}

/// Serbest metin alanı olan her tür; tüm metin alanları `text`.
fn with_text(text: &str) -> Vec<EventKind> {
    let owned = || text.to_owned();
    vec![
        EventKind::RunStep {
            step: 1,
            phase: StepPhase::Plan,
            detail: owned(),
        },
        EventKind::RunFailed {
            code: ErrorCode::Internal,
            message: owned(),
        },
        EventKind::ToolRequested {
            call: summary(text),
            risk: RiskLevel::Act,
        },
        EventKind::ToolCompleted {
            call_id: call_id(),
            verification: Verification::Unverifiable { reason: owned() },
        },
        EventKind::ToolCompleted {
            call_id: call_id(),
            verification: Verification::Failed { reason: owned() },
        },
        EventKind::DesktopUnavailable {
            backend: owned(),
            reason: owned(),
        },
    ]
}

#[test]
fn redaction_masks_every_free_text_field() {
    let leaky = with_text("x s3cret Bearer tok y");
    let clean = with_text("x *** Bearer *** y");
    assert_eq!(leaky.len(), clean.len());
    for (input, expected) in leaky.into_iter().zip(clean) {
        assert_eq!(input.redacted(&["s3cret"]), expected);
    }
}

#[test]
fn redaction_leaves_kinds_without_free_text_untouched() {
    let untouched = [
        EventKind::RunStarted {
            session_id: Some(SessionId::new()),
        },
        EventKind::RunFinished { steps: 1 },
        EventKind::Halt { cancelled_runs: 1 },
        EventKind::ToolCompleted {
            call_id: call_id(),
            verification: Verification::Verified,
        },
    ];
    for kind in untouched {
        assert_eq!(kind.clone().redacted(&["s3cret"]), kind);
    }
}
