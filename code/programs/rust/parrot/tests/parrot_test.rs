//! Integration tests for the Parrot REPL program.
//!
//! # Testing strategy
//!
//! All tests drive the REPL through injected I/O instead of real stdin/stdout.
//! Inputs are provided as a `Vec<&str>` and outputs are collected into a
//! `Vec<String>`. This makes tests deterministic, fast, and side-effect-free:
//!
//! ```text
//! Vec<&str> ──► input_fn ──► run_with_options ──► output_fn ──► Vec<String>
//! ```
//!
//! No terminal emulation, no pipe setup, no thread leaks between tests.
//!
//! # Coverage
//!
//! 1.  Echo basic input
//! 2.  Quit ends the session
//! 3.  Multiple inputs echoed in order
//! 4.  Sync mode echoes correctly
//! 5.  Async mode echoes correctly
//! 6.  GlobalPrompt contains parrot emoji (tested directly on struct)
//! 7.  LinePrompt contains parrot emoji (tested directly on struct)
//! 8.  EOF exits gracefully (empty input)
//! 9.  Empty string is echoed
//! 10. Multiple inputs before quit — all appear
//! 11. GlobalPrompt contains ">" cursor character
//! 12. LinePrompt ends with a trailing space
//! 13. Session ends on :quit — no further output after quit line
//! 14. Output collected in correct order (sync mode)
//! 15. Sync and async modes produce the same echo output
//! 16. Input with spaces echoed verbatim
//! 17. Only :quit — no echoed lines in output

use std::sync::Arc;

use parrot::prompt::ParrotPrompt;
use repl::runner::run_with_options;
use repl::types::Mode;
use repl::{EchoLanguage, Prompt, SilentWaiting};

// =============================================================================
// Test helpers
// =============================================================================

/// Run the Parrot REPL with a fixed set of inputs in `Mode::Async` and collect
/// all strings passed to `output_fn` into a `Vec<String>`.
///
/// The REPL loop ends when the input iterator is exhausted (EOF) or when
/// `EchoLanguage` receives `":quit"`.
fn run_parrot(inputs: Vec<&str>) -> Vec<String> {
    run_parrot_mode(inputs, Mode::Async)
}

/// Like [`run_parrot`] but with an explicit [`Mode`], allowing both sync and
/// async paths to be exercised in separate tests.
fn run_parrot_mode(inputs: Vec<&str>, mode: Mode) -> Vec<String> {
    // Convert &str slices to owned Strings up front so the closure can move
    // the iterator without lifetime entanglement.
    let owned: Vec<String> = inputs.iter().map(|s| s.to_string()).collect();
    let mut iter = owned.into_iter();

    // `output` is written to by `output_fn` on every output call.
    // We collect into a plain Vec here — no Mutex needed because the runner
    // calls `output_fn` on the main thread (the fn is `FnMut`, not `Send`).
    let mut output: Vec<String> = Vec::new();

    let input_fn = move || -> Option<String> { iter.next() };

    // We need a mutable borrow of `output` inside the closure. Rust requires
    // that the closure not outlive `output`, which is satisfied here because
    // `run_with_options` is synchronous — it returns only after the loop ends.
    let output_fn = |text: &str| {
        output.push(text.to_string());
    };

    run_with_options(
        Arc::new(EchoLanguage),
        Arc::new(ParrotPrompt),
        // In sync mode, `waiting` is unused and may be `None`.
        // In async mode, we must provide `Some(...)`.
        Some(Arc::new(SilentWaiting)),
        mode,
        input_fn,
        output_fn,
    );

    output
}

/// Join all output strings into a single string for easier substring checks.
fn join(outputs: &[String]) -> String {
    outputs.join("")
}

// =============================================================================
// Tests
// =============================================================================

// ─────────────────────────────────────────────────────────────────────────────
// 1. Echo basic input
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_echo_basic_input() {
    let out = run_parrot(vec!["hello", ":quit"]);
    assert!(
        join(&out).contains("hello"),
        "expected 'hello' in output, got {out:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Quit ends the session
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_quit_ends_session() {
    // After :quit the loop stops. The sentinel must not appear in output.
    let out = run_parrot(vec![":quit", "should-not-appear"]);
    assert!(
        !join(&out).contains("should-not-appear"),
        "loop should stop at :quit, but got extra output: {out:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Multiple inputs echoed in order
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_multiple_inputs_echoed() {
    let out = run_parrot(vec!["alpha", "beta", "gamma", ":quit"]);
    let full = join(&out);
    for word in ["alpha", "beta", "gamma"] {
        assert!(full.contains(word), "expected {word:?} in output, got {out:?}");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Sync mode echoes correctly
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_sync_mode_echoes() {
    // Mode::Sync calls eval directly on the calling thread — no spawned threads.
    let out = run_parrot_mode(vec!["sync-test", ":quit"], Mode::Sync);
    assert!(
        join(&out).contains("sync-test"),
        "sync mode: expected 'sync-test' in output, got {out:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Async mode echoes correctly
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_async_mode_echoes() {
    // Mode::Async spawns a worker thread for each eval call.
    let out = run_parrot_mode(vec!["async-test", ":quit"], Mode::Async);
    assert!(
        join(&out).contains("async-test"),
        "async mode: expected 'async-test' in output, got {out:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. GlobalPrompt contains parrot emoji
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_global_prompt_contains_parrot_emoji() {
    let p = ParrotPrompt;
    let gp = p.global_prompt();
    assert!(
        gp.contains("🦜"),
        "GlobalPrompt should contain parrot emoji, got {gp:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. LinePrompt contains parrot emoji
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_line_prompt_contains_parrot_emoji() {
    let p = ParrotPrompt;
    let lp = p.line_prompt();
    assert!(
        lp.contains("🦜"),
        "LinePrompt should contain parrot emoji, got {lp:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. EOF exits gracefully
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_eof_exits_gracefully() {
    // An empty inputs vec means input_fn immediately returns None.
    // The loop should exit without panicking or blocking.
    let out = run_parrot(vec![]);
    // No assertion on content — the test passes if it returns at all.
    let _ = out;
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. Empty string is echoed
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_empty_string_echoed() {
    // EchoLanguage returns EvalResult::Ok(Some("")) for an empty string.
    // The runner will call output_fn with an empty string.
    let out = run_parrot(vec!["", ":quit"]);
    // We expect at least one output call (the echoed empty string).
    assert!(
        !out.is_empty(),
        "expected at least one output for empty-string input, got empty"
    );
    // At least one of the outputs should be an empty string (the echo of "").
    let has_empty_echo = out.iter().any(|s| s.is_empty());
    assert!(has_empty_echo, "expected echoed empty string in outputs: {out:?}");
}

// ─────────────────────────────────────────────────────────────────────────────
// 10. Multiple inputs before quit — all appear in output
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_multiple_inputs_before_quit() {
    let out = run_parrot(vec!["one", "two", "three", "four", "five", ":quit"]);
    let full = join(&out);
    for word in ["one", "two", "three", "four", "five"] {
        assert!(
            full.contains(word),
            "expected {word:?} in output, got {full:?}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 11. GlobalPrompt contains ">" cursor character
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_global_prompt_contains_cursor() {
    let p = ParrotPrompt;
    let gp = p.global_prompt();
    assert!(
        gp.contains('>'),
        "GlobalPrompt should contain '>' cursor, got {gp:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 12. LinePrompt ends with a trailing space
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_line_prompt_ends_with_space() {
    // Conventional prompts end with a space so the cursor is one space away
    // from the prompt character. Python's `>>> ` and `... ` follow this rule.
    let p = ParrotPrompt;
    let lp = p.line_prompt();
    assert!(
        !lp.is_empty(),
        "LinePrompt should not be empty"
    );
    assert!(
        lp.ends_with(' '),
        "LinePrompt should end with a space, got {lp:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 13. Session ends on :quit — no further output after quit line
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_session_ends_on_quit() {
    let out = run_parrot(vec!["before", ":quit", "after-sentinel"]);
    let full = join(&out);

    assert!(
        !full.contains("after-sentinel"),
        "loop should stop at :quit; 'after-sentinel' should not appear: {full:?}"
    );
    assert!(
        full.contains("before"),
        "'before' should appear before :quit: {full:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 14. Output collected in correct order (sync mode)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_output_collected_in_order() {
    // Use sync mode to eliminate any theoretical reordering from thread
    // scheduling. Each input must appear in the output in the same order.
    let out = run_parrot_mode(vec!["first", "second", "third", ":quit"], Mode::Sync);

    // The outputs for a 3-input run should contain exactly 3 echoed strings.
    assert_eq!(
        out.len(),
        3,
        "expected 3 output calls (one per echoed input), got {}: {out:?}",
        out.len()
    );

    let expected = ["first", "second", "third"];
    for (i, exp) in expected.iter().enumerate() {
        assert_eq!(
            out[i], *exp,
            "output[{i}]: expected {exp:?}, got {:?}",
            out[i]
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 15. Sync and async modes produce the same echo output
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_sync_and_async_produce_same_output() {
    let inputs = vec!["parrot", "squawk", ":quit"];

    let sync_out = run_parrot_mode(inputs.clone(), Mode::Sync);
    let async_out = run_parrot_mode(inputs, Mode::Async);

    assert_eq!(
        sync_out.len(),
        async_out.len(),
        "sync produced {} outputs, async produced {}",
        sync_out.len(),
        async_out.len()
    );

    for (i, (s, a)) in sync_out.iter().zip(async_out.iter()).enumerate() {
        assert_eq!(s, a, "output[{i}]: sync={s:?} async={a:?}");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 16. Input with spaces echoed verbatim
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_input_with_spaces_echoed_verbatim() {
    // EchoLanguage does not trim whitespace from normal input (only from the
    // :quit sentinel check, which uses trim()). Internal spaces are preserved.
    let input = "hello   world   with   spaces";
    let out = run_parrot(vec![input, ":quit"]);
    let full = join(&out);
    assert!(
        full.contains(input),
        "expected {input:?} verbatim in output, got {full:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 17. Only :quit — no echoed lines in output
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_quit_only_produces_no_echoed_lines() {
    // When the only input is :quit, the runner exits immediately with no
    // output (EvalResult::Quit suppresses any output_fn call).
    let out = run_parrot(vec![":quit"]);
    assert!(
        out.is_empty(),
        "expected no output when input is only ':quit', got {out:?}"
    );
}
