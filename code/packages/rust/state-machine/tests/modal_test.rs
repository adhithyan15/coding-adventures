//! Integration tests for the Modal State Machine implementation.

use std::collections::{HashMap, HashSet};

use state_machine::dfa::DFA;
use state_machine::modal::ModalStateMachine;

// ============================================================
// Helper constructors
// ============================================================

fn make_data_mode() -> DFA {
    DFA::new(
        HashSet::from(["text".into(), "tag_detected".into()]),
        HashSet::from(["char".into(), "open_angle".into()]),
        HashMap::from([
            (("text".into(), "char".into()), "text".into()),
            (("text".into(), "open_angle".into()), "tag_detected".into()),
            (("tag_detected".into(), "char".into()), "text".into()),
            (
                ("tag_detected".into(), "open_angle".into()),
                "tag_detected".into(),
            ),
        ]),
        "text".into(),
        HashSet::from(["text".into()]),
    )
    .unwrap()
}

fn make_tag_mode() -> DFA {
    DFA::new(
        HashSet::from(["reading_name".into(), "tag_done".into()]),
        HashSet::from(["char".into(), "close_angle".into()]),
        HashMap::from([
            (
                ("reading_name".into(), "char".into()),
                "reading_name".into(),
            ),
            (
                ("reading_name".into(), "close_angle".into()),
                "tag_done".into(),
            ),
            (("tag_done".into(), "char".into()), "reading_name".into()),
            (("tag_done".into(), "close_angle".into()), "tag_done".into()),
        ]),
        "reading_name".into(),
        HashSet::from(["tag_done".into()]),
    )
    .unwrap()
}

fn make_script_mode() -> DFA {
    DFA::new(
        HashSet::from(["raw".into()]),
        HashSet::from(["char".into(), "end_marker".into()]),
        HashMap::from([
            (("raw".into(), "char".into()), "raw".into()),
            (("raw".into(), "end_marker".into()), "raw".into()),
        ]),
        "raw".into(),
        HashSet::from(["raw".into()]),
    )
    .unwrap()
}

fn html_tokenizer() -> ModalStateMachine {
    ModalStateMachine::new(
        HashMap::from([
            ("data".into(), make_data_mode()),
            ("tag".into(), make_tag_mode()),
            ("script".into(), make_script_mode()),
        ]),
        HashMap::from([
            (("data".into(), "enter_tag".into()), "tag".into()),
            (("tag".into(), "exit_tag".into()), "data".into()),
            (("tag".into(), "enter_script".into()), "script".into()),
            (("script".into(), "exit_script".into()), "data".into()),
        ]),
        "data".into(),
    )
    .unwrap()
}

// ============================================================
// Construction Tests
// ============================================================

#[test]
fn test_valid_construction() {
    let modal = html_tokenizer();
    assert_eq!(modal.current_mode(), "data");
    assert_eq!(modal.modes().len(), 3);
}

#[test]
fn test_no_modes_rejected() {
    let result = ModalStateMachine::new(HashMap::new(), HashMap::new(), "data".into());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("one mode"));
}

#[test]
fn test_invalid_initial_mode() {
    let result = ModalStateMachine::new(
        HashMap::from([("data".into(), make_data_mode())]),
        HashMap::new(),
        "missing".into(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Initial mode"));
}

#[test]
fn test_invalid_transition_source() {
    let result = ModalStateMachine::new(
        HashMap::from([("data".into(), make_data_mode())]),
        HashMap::from([(("missing".into(), "trigger".into()), "data".into())]),
        "data".into(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("source"));
}

#[test]
fn test_invalid_transition_target() {
    let result = ModalStateMachine::new(
        HashMap::from([("data".into(), make_data_mode())]),
        HashMap::from([(("data".into(), "trigger".into()), "missing".into())]),
        "data".into(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("target"));
}

// ============================================================
// Mode Switching Tests
// ============================================================

#[test]
fn test_switch_mode() {
    let mut modal = html_tokenizer();
    assert_eq!(modal.current_mode(), "data");
    modal.switch_mode("enter_tag").unwrap();
    assert_eq!(modal.current_mode(), "tag");
}

#[test]
fn test_switch_mode_returns_new_mode() {
    let mut modal = html_tokenizer();
    let result = modal.switch_mode("enter_tag").unwrap();
    assert_eq!(result, "tag");
}

#[test]
fn test_switch_resets_target_dfa() {
    let mut modal = html_tokenizer();
    modal.switch_mode("enter_tag").unwrap();
    modal.process("char").unwrap();
    modal.process("close_angle").unwrap();
    assert_eq!(modal.active_machine().current_state(), "tag_done");

    modal.switch_mode("exit_tag").unwrap();
    modal.switch_mode("enter_tag").unwrap();
    assert_eq!(modal.active_machine().current_state(), "reading_name");
}

#[test]
fn test_switch_data_to_tag_to_data() {
    let mut modal = html_tokenizer();
    modal.switch_mode("enter_tag").unwrap();
    assert_eq!(modal.current_mode(), "tag");
    modal.switch_mode("exit_tag").unwrap();
    assert_eq!(modal.current_mode(), "data");
}

#[test]
fn test_switch_to_script_mode() {
    let mut modal = html_tokenizer();
    modal.switch_mode("enter_tag").unwrap();
    modal.switch_mode("enter_script").unwrap();
    assert_eq!(modal.current_mode(), "script");
    modal.switch_mode("exit_script").unwrap();
    assert_eq!(modal.current_mode(), "data");
}

#[test]
fn test_invalid_trigger() {
    let mut modal = html_tokenizer();
    let result = modal.switch_mode("nonexistent_trigger");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No mode transition"));
}

#[test]
fn test_mode_trace() {
    let mut modal = html_tokenizer();
    modal.switch_mode("enter_tag").unwrap();
    modal.switch_mode("exit_tag").unwrap();

    let trace = modal.mode_trace();
    assert_eq!(trace.len(), 2);
    assert_eq!(trace[0].from_mode, "data");
    assert_eq!(trace[0].trigger, "enter_tag");
    assert_eq!(trace[0].to_mode, "tag");
    assert_eq!(trace[1].from_mode, "tag");
    assert_eq!(trace[1].trigger, "exit_tag");
    assert_eq!(trace[1].to_mode, "data");
}

// ============================================================
// Processing Within Modes Tests
// ============================================================

#[test]
fn test_process_in_data_mode() {
    let mut modal = html_tokenizer();
    let result = modal.process("char").unwrap();
    assert_eq!(result, "text");
}

#[test]
fn test_process_in_tag_mode() {
    let mut modal = html_tokenizer();
    modal.switch_mode("enter_tag").unwrap();
    let result = modal.process("char").unwrap();
    assert_eq!(result, "reading_name");
    let result = modal.process("close_angle").unwrap();
    assert_eq!(result, "tag_done");
}

#[test]
fn test_process_in_script_mode() {
    let mut modal = html_tokenizer();
    modal.switch_mode("enter_tag").unwrap();
    modal.switch_mode("enter_script").unwrap();
    let result = modal.process("char").unwrap();
    assert_eq!(result, "raw");
}

#[test]
fn test_process_invalid_event_for_mode() {
    let mut modal = html_tokenizer();
    let result = modal.process("close_angle");
    assert!(result.is_err());
}

#[test]
fn test_active_machine_property() {
    let mut modal = html_tokenizer();
    assert_eq!(modal.active_machine().current_state(), "text");

    modal.switch_mode("enter_tag").unwrap();
    assert_eq!(modal.active_machine().current_state(), "reading_name");
}

// ============================================================
// Reset Tests
// ============================================================

#[test]
fn test_reset_mode() {
    let mut modal = html_tokenizer();
    modal.switch_mode("enter_tag").unwrap();
    modal.reset();
    assert_eq!(modal.current_mode(), "data");
}

#[test]
fn test_reset_clears_trace() {
    let mut modal = html_tokenizer();
    modal.switch_mode("enter_tag").unwrap();
    modal.switch_mode("exit_tag").unwrap();
    assert_eq!(modal.mode_trace().len(), 2);

    modal.reset();
    assert!(modal.mode_trace().is_empty());
}

#[test]
fn test_reset_resets_all_dfas() {
    let mut modal = html_tokenizer();
    modal.switch_mode("enter_tag").unwrap();
    modal.process("char").unwrap();
    modal.process("close_angle").unwrap();

    modal.reset();
    modal.switch_mode("enter_tag").unwrap();
    assert_eq!(modal.active_machine().current_state(), "reading_name");
}

// ============================================================
// Display Tests
// ============================================================

#[test]
fn test_display() {
    let modal = html_tokenizer();
    let s = format!("{}", modal);
    assert!(s.contains("ModalStateMachine"));
    assert!(s.contains("data"));
}

// ============================================================
// Additional edge case tests
// ============================================================

#[test]
fn test_single_mode_machine() {
    let modal = ModalStateMachine::new(
        HashMap::from([("only".into(), make_data_mode())]),
        HashMap::new(),
        "only".into(),
    )
    .unwrap();
    assert_eq!(modal.current_mode(), "only");
}

#[test]
fn test_self_mode_transition() {
    let mut modal = ModalStateMachine::new(
        HashMap::from([("data".into(), make_data_mode())]),
        HashMap::from([(("data".into(), "refresh".into()), "data".into())]),
        "data".into(),
    )
    .unwrap();
    modal.process("char").unwrap();
    assert_eq!(modal.active_machine().current_state(), "text");

    modal.process("open_angle").unwrap();
    assert_eq!(modal.active_machine().current_state(), "tag_detected");

    // Self-transition resets the DFA
    modal.switch_mode("refresh").unwrap();
    assert_eq!(modal.active_machine().current_state(), "text"); // reset to initial
    assert_eq!(modal.current_mode(), "data");
}
