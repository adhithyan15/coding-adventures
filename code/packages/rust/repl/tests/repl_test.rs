//! Integration tests for the repl crate.
//!
//! These tests exercise the full loop via `run_with_io`, using `EchoLanguage`,
//! `DefaultPrompt`, and `SilentWaiting` as the built-in implementations.
//! I/O is injected through closures so no terminal or stdin is required.
//!
//! Test coverage:
//!
//! 1. `test_echo_single_line` — one input line echoed, then `:quit`
//! 2. `test_quit_immediately` — `:quit` on first line produces no output
//! 3. `test_eof_without_quit` — iterator exhausted without `:quit`
//! 4. `test_multiple_lines` — several lines echoed before `:quit`
//! 5. `test_error_result_prefixed` — custom language that returns Error
//! 6. `test_ok_none_produces_no_output` — Ok(None) is silent

use std::sync::Arc;
use repl::runner::{run_with_io, run_with_options};
use repl::echo_language::EchoLanguage;
use repl::default_prompt::DefaultPrompt;
use repl::silent_waiting::SilentWaiting;
use repl::language::Language;
use repl::types::{EvalResult, Mode};

// ===========================================================================
// Helper — run a script through EchoLanguage and collect outputs
// ===========================================================================

/// Feed a list of lines through `run_with_io` with the built-in impls and
/// collect everything passed to `output_fn`.
fn run_echo(inputs: Vec<&str>) -> Vec<String> {
    let inputs: Vec<String> = inputs.iter().map(|s| s.to_string()).collect();
    let mut iter = inputs.into_iter();
    let mut outputs: Vec<String> = Vec::new();

    run_with_io(
        Arc::new(EchoLanguage),
        Arc::new(DefaultPrompt),
        Arc::new(SilentWaiting),
        || iter.next(),
        |s| outputs.push(s.to_string()),
    );

    outputs
}

// ===========================================================================
// Test 1 — single line echoed, then quit
// ===========================================================================

#[test]
fn test_echo_single_line() {
    // One line of content followed by `:quit`.
    // Expected: the content line is echoed; the `:quit` itself is not printed.
    let outputs = run_echo(vec!["hello", ":quit"]);
    assert_eq!(outputs, vec!["hello"]);
}

// ===========================================================================
// Test 2 — :quit on first line produces no output
// ===========================================================================

#[test]
fn test_quit_immediately() {
    // `:quit` is the very first line.
    // Expected: the loop ends with no output at all.
    let outputs = run_echo(vec![":quit"]);
    assert!(
        outputs.is_empty(),
        "expected no output, got: {outputs:?}"
    );
}

// ===========================================================================
// Test 3 — EOF (iterator exhausted) without an explicit :quit
// ===========================================================================

#[test]
fn test_eof_without_quit() {
    // The input iterator ends without ever sending `:quit`.
    // Expected: the loop ends cleanly; all lines before EOF are echoed.
    let outputs = run_echo(vec!["alpha", "beta", "gamma"]);
    assert_eq!(outputs, vec!["alpha", "beta", "gamma"]);
}

// ===========================================================================
// Test 4 — multiple lines echoed in order before :quit
// ===========================================================================

#[test]
fn test_multiple_lines() {
    let outputs = run_echo(vec!["one", "two", "three", ":quit"]);
    assert_eq!(outputs, vec!["one", "two", "three"]);
}

// ===========================================================================
// Test — sync mode: echo works without spawning threads
// ===========================================================================

/// Feed inputs through `run_with_options` with `Mode::Sync` and collect output.
fn run_sync(inputs: Vec<&str>) -> Vec<String> {
    let inputs: Vec<String> = inputs.iter().map(|s| s.to_string()).collect();
    let mut iter = inputs.into_iter();
    let mut outputs: Vec<String> = Vec::new();

    run_with_options(
        Arc::new(EchoLanguage),
        Arc::new(DefaultPrompt),
        None::<Arc<SilentWaiting>>, // waiting unused in sync mode
        Mode::Sync,
        || iter.next(),
        |s| outputs.push(s.to_string()),
    );
    outputs
}

/// `test_sync_mode_echo` — sync mode echoes input lines correctly.
///
/// Uses `Mode::Sync` with `None` for waiting (the sync path never touches it).
/// The output should match what async mode produces.
#[test]
fn test_sync_mode_echo() {
    let outputs = run_sync(vec!["hello", "world", ":quit"]);
    assert_eq!(outputs, vec!["hello", "world"]);
}

/// `test_sync_mode_quit` — sync mode terminates on `:quit`.
///
/// Lines sent *after* `:quit` must not appear in output.
#[test]
fn test_sync_mode_quit() {
    let outputs = run_sync(vec!["hello", ":quit", "after-quit"]);
    // "hello" must appear; "after-quit" must not.
    assert!(
        outputs.contains(&"hello".to_string()),
        "expected 'hello' in output; got: {outputs:?}"
    );
    assert!(
        !outputs.iter().any(|s| s.contains("after-quit")),
        "loop continued past :quit; output: {outputs:?}"
    );
}

/// `test_sync_mode_none_waiting` — explicitly passes `None` for waiting with
/// `Mode::Sync` to confirm the framework never dereferences the `None`.
///
/// This is the most direct test of the nil-waiting contract: if the sync path
/// accidentally touches `waiting`, it will panic here.
#[test]
fn test_sync_mode_none_waiting() {
    let inputs: Vec<String> = vec!["foo".to_string(), "bar".to_string()];
    let mut iter = inputs.into_iter();
    let mut outputs: Vec<String> = Vec::new();

    // None::<Arc<SilentWaiting>> makes the type checker happy while
    // exercising the code path where waiting is genuinely absent.
    run_with_options(
        Arc::new(EchoLanguage),
        Arc::new(DefaultPrompt),
        None::<Arc<SilentWaiting>>,
        Mode::Sync,
        || iter.next(),
        |s| outputs.push(s.to_string()),
    );

    assert_eq!(outputs, vec!["foo", "bar"]);
}

// ===========================================================================
// Test 5 — Error result is prefixed with "Error: "
// ===========================================================================

/// A language that always returns an error for any input.
struct AlwaysErrorLanguage;

impl Language for AlwaysErrorLanguage {
    fn eval(&self, input: &str) -> EvalResult {
        if input.trim() == ":quit" {
            return EvalResult::Quit;
        }
        EvalResult::Error(format!("cannot evaluate: {input}"))
    }
}

#[test]
fn test_error_result_prefixed() {
    // The runner prepends "Error: " to any Error variant.
    let inputs: Vec<String> = vec!["bad input".to_string(), ":quit".to_string()];
    let mut iter = inputs.into_iter();
    let mut outputs: Vec<String> = Vec::new();

    run_with_io(
        Arc::new(AlwaysErrorLanguage),
        Arc::new(DefaultPrompt),
        Arc::new(SilentWaiting),
        || iter.next(),
        |s| outputs.push(s.to_string()),
    );

    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0], "Error: cannot evaluate: bad input");
}

// ===========================================================================
// Test 6 — Ok(None) produces no output
// ===========================================================================

/// A language that returns Ok(None) — silent success — for every line except
/// `:quit`.
struct SilentOkLanguage;

impl Language for SilentOkLanguage {
    fn eval(&self, input: &str) -> EvalResult {
        if input.trim() == ":quit" {
            return EvalResult::Quit;
        }
        EvalResult::Ok(None)
    }
}

#[test]
fn test_ok_none_produces_no_output() {
    // Three lines of "silent success" should produce no output at all.
    let inputs: Vec<String> = vec![
        "stmt1".to_string(),
        "stmt2".to_string(),
        "stmt3".to_string(),
        ":quit".to_string(),
    ];
    let mut iter = inputs.into_iter();
    let mut outputs: Vec<String> = Vec::new();

    run_with_io(
        Arc::new(SilentOkLanguage),
        Arc::new(DefaultPrompt),
        Arc::new(SilentWaiting),
        || iter.next(),
        |s| outputs.push(s.to_string()),
    );

    assert!(
        outputs.is_empty(),
        "expected no output for Ok(None), got: {outputs:?}"
    );
}
