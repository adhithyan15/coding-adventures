//! Parrot — the world's simplest REPL.
//!
//! Whatever you type, the parrot repeats it back. Type `:quit` to exit.
//!
//! # What this program demonstrates
//!
//! This binary demonstrates the coding-adventures REPL framework by wiring
//! three components together:
//!
//! | Component | Role |
//! |-----------|------|
//! | [`repl::EchoLanguage`] | Evaluates input by echoing it back unchanged |
//! | [`ParrotPrompt`] | Provides parrot-themed prompts with the 🦜 emoji |
//! | [`repl::SilentWaiting`] | Shows nothing while the evaluator "runs" |
//!
//! The framework handles the async eval loop (each evaluation runs on a
//! dedicated OS thread), panic recovery (a crashing evaluator surfaces as an
//! error, not a crash), and I/O injection (the same loop works for terminals,
//! tests, and pipes).
//!
//! The program only supplies the personality.
//!
//! # Architecture
//!
//! ```text
//! stdin ──► input_fn ──► run_with_io ──► EchoLanguage::eval
//!                            │
//!                            ▼
//!                       ParrotPrompt (prompt strings injected via input_fn)
//!                            │
//!                            ▼
//!                       output_fn ──► stdout
//! ```
//!
//! Note: in the Rust REPL framework, the prompt string is printed inside the
//! `input_fn` closure (before reading from stdin), rather than by the runner
//! itself. This keeps the runner generic and lets the program control exactly
//! when and how the prompt appears.

use std::io::{self, BufRead, Write};
use std::sync::Arc;

use repl::runner::run_with_io;
use repl::{EchoLanguage, Prompt, SilentWaiting};

// The prompt module lives in src/prompt.rs and is also exported from lib.rs
// so that integration tests can import it. We include it here in main.rs as
// well so the binary has access to it.
mod prompt;
use prompt::ParrotPrompt;

fn main() {
    // ── I/O setup ─────────────────────────────────────────────────────────────
    //
    // We lock stdin and stdout once per session rather than on each line.
    // Locking upfront avoids repeated lock acquisitions in the hot path and
    // ensures the prompt and the subsequent read appear as a single atomic
    // operation (no interleaving from other threads).
    let stdin = io::stdin();
    let stdout = io::stdout();

    // ── Welcome banner ────────────────────────────────────────────────────────
    //
    // The banner is printed once before the loop starts. It is separate from
    // the per-line prompt (🦜 >) so the user sees it exactly once.
    {
        let mut out = stdout.lock();
        writeln!(out, "🦜 Parrot REPL").ok();
        writeln!(out, "I repeat everything you say! Type :quit to exit.").ok();
        writeln!(out).ok();
        out.flush().ok();
    }

    // ── Build the input function ──────────────────────────────────────────────
    //
    // `input_fn` is called once per iteration by the REPL runner. It must:
    //   1. Print the prompt (🦜 >) so the user knows where to type.
    //   2. Flush stdout so the prompt is visible before blocking on stdin.
    //   3. Read one line and return Some(line) or None on EOF.
    //
    // Using `BufRead::lines()` gives us owned Strings and skips the trailing
    // newline. `and_then(|r| r.ok())` collapses both the outer Option and the
    // inner Result into a single Option<String>.
    let prompt = Arc::new(ParrotPrompt);
    let prompt_for_input = Arc::clone(&prompt);

    let mut lines = stdin.lock().lines();

    let input_fn = move || -> Option<String> {
        // Print and flush the prompt before blocking on stdin.
        {
            let mut out = io::stdout();
            write!(out, "{}", prompt_for_input.global_prompt()).ok();
            out.flush().ok();
        }
        // Read one line; return None on EOF or read error.
        lines.next().and_then(|r| r.ok())
    };

    // ── Build the output function ─────────────────────────────────────────────
    //
    // `output_fn` is called by the runner when the evaluator produces output.
    // We print the string followed by a newline (the framework passes the
    // raw output; newline handling is our responsibility).
    let output_fn = |text: &str| {
        println!("{text}");
    };

    // ── REPL loop ─────────────────────────────────────────────────────────────
    //
    // `run_with_io` wraps `run_with_options` with Mode::Async. The loop runs
    // until EchoLanguage returns EvalResult::Quit (user typed ":quit") or
    // input_fn returns None (EOF / Ctrl-D).
    run_with_io(
        Arc::new(EchoLanguage),
        prompt,
        Arc::new(SilentWaiting),
        input_fn,
        output_fn,
    );

    // ── Goodbye ───────────────────────────────────────────────────────────────
    //
    // Printed after the loop exits, regardless of quit vs EOF.
    println!("Goodbye! 🦜");
}
