use forme_plugin_runner_rs::{CancellationToken, StageError};

#[test]
fn stage_errors_have_stable_redacted_wire_fields() {
    let error = StageError::new("FIXTURE", "fixture failed")
        .with_field("line", 3.into())
        .recoverable(true);
    let data = error.wire_data();
    assert_eq!(data["stageErrorCode"], "FIXTURE");
    assert_eq!(data["recoverable"], true);
    assert_eq!(data["fields"]["line"], 3);
}

#[test]
fn cancellation_is_idempotent_and_preserves_the_first_reason() {
    let token = CancellationToken::new();
    token.cancel("first");
    token.cancel("second");
    let error = token.check().unwrap_err();
    assert_eq!(error.to_string(), "first");
}
