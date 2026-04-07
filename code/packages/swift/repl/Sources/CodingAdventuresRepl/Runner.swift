import Foundation

// Runner.swift — The core read-eval-print loop.
//
// This file contains the single public function that drives the REPL:
// `runWithIO`. Everything else in the framework (Language, Prompt, Waiting)
// is a plugin that runWithIO calls.
//
// ─────────────────────────────────────────────────────────────────────────────
// High-level flow diagram
// ─────────────────────────────────────────────────────────────────────────────
//
//   startup: print globalPrompt()
//
//   ┌──────────────────────────────────────────────────────────────────┐
//   │  LOOP                                                            │
//   │                                                                  │
//   │  1. print linePrompt()                                           │
//   │  2. call inputFn() → String?                                     │
//   │     └─ nil  ──► break (EOF)                                      │
//   │  3. dispatch language.eval(input):                               │
//   │     ├─ .sync      → call on calling thread                       │
//   │     └─ .async_mode → DispatchQueue.global(), poll with tickMs()  │
//   │  4. handle EvalResult:                                           │
//   │     ├─ .ok(nil)   → (nothing)                                    │
//   │     ├─ .ok(text)  → outputFn(text)                               │
//   │     ├─ .error(msg)→ outputFn("Error: \(msg)")                    │
//   │     └─ .quit      → outputFn("Goodbye!"), return                 │
//   └──────────────────────────────────────────────────────────────────┘
//
// ─────────────────────────────────────────────────────────────────────────────
// Async dispatch detail
// ─────────────────────────────────────────────────────────────────────────────
//
// In .async_mode the runner uses DispatchGroup + DispatchQueue.global():
//
//   Main thread                Background thread
//   ───────────────────────    ────────────────────────────
//   group.enter()
//   DispatchQueue.global()  →  language.eval(input)
//   .async { … group.leave() }         ↓
//   loop:                       evalResult = result
//     wait(timeout: tickMs)             ↓
//     if .timedOut: tick()        group.leave()
//   end loop
//   w.stop(state)
//
// This is the same polling pattern used by Python (thread.join(timeout))
// and Ruby (thread.join(timeout)). It lets the calling thread drive the
// Waiting animation while the evaluator runs.
//
// ─────────────────────────────────────────────────────────────────────────────
// I/O injection rationale
// ─────────────────────────────────────────────────────────────────────────────
//
// inputFn and outputFn are plain closures instead of direct stdin/stdout calls.
// This makes the function 100% testable without file descriptors, process
// substitution, or mocking frameworks:
//
//   var lines = ["hello", ":quit"]
//   var output: [String] = []
//   runWithIO(
//       language: EchoLanguage(),
//       prompt: DefaultPrompt(),
//       waiting: SilentWaiting(),
//       inputFn: { lines.isEmpty ? nil : lines.removeFirst() },
//       outputFn: { output.append($0) }
//   )
//   assert(output.contains("hello"))
//
// The same function powers both the interactive terminal and tests.

// ─────────────────────────────────────────────────────────────────────────────
// runWithIO
// ─────────────────────────────────────────────────────────────────────────────

/// The main entry point for the REPL framework.
///
/// Generic parameters:
/// - `L: Language` — evaluates input strings.
/// - `P: Prompt`   — provides prompt strings.
/// - `W: Waiting`  — animates while eval runs (async mode only).
///
/// Parameters:
/// - `language`: The evaluator plugin.
/// - `prompt`: The prompt-string provider.
/// - `waiting`: The busy-indicator plugin. Pass `nil` to skip animation
///   entirely (the runner blocks on eval silently).
/// - `inputFn`: Called to read each line. Return `nil` to signal EOF and end
///   the session cleanly (same as pressing Ctrl-D on a real terminal).
/// - `outputFn`: Called to emit each piece of output. Receives prompts,
///   eval results, error messages, and the "Goodbye!" farewell.
/// - `mode`: `.sync` or `.async_mode` (default). See `Mode` for details.
public func runWithIO<L: Language, P: Prompt, W: Waiting>(
    language: L,
    prompt: P,
    waiting: W?,
    inputFn: () -> String?,
    outputFn: (String) -> Void,
    mode: Mode = .async_mode
) {
    // ── Step 1: Print the global banner once at startup ────────────────────
    // An empty banner is valid (some REPLs are silent on startup), so we
    // only emit it when non-empty. This avoids a spurious blank line at the
    // top of captured test output.
    let banner = prompt.globalPrompt()
    if !banner.isEmpty {
        outputFn(banner)
    }

    // ── Main read-eval-print loop ──────────────────────────────────────────
    while true {

        // ── Step 2: Print the line prompt ─────────────────────────────────
        // We call outputFn (not print) so callers can intercept prompt strings
        // in tests. The prompt typically ends with a space, not a newline, so
        // the user's cursor appears right after it.
        outputFn(prompt.linePrompt())

        // ── Step 3: Read one line of input ────────────────────────────────
        // inputFn returns nil on EOF (Ctrl-D, end of pipe, or test exhaustion).
        guard let input = inputFn() else {
            // EOF — exit the loop cleanly without printing anything.
            // This mirrors the behaviour of Python's REPL on Ctrl-D.
            break
        }

        // Strip trailing newlines that may arrive from stdin or test inputs.
        // We use .newlines character set rather than trimming all whitespace
        // to preserve intentional leading/trailing spaces in the input.
        let trimmed = input.trimmingCharacters(in: .newlines)

        // ── Step 4: Evaluate the input ────────────────────────────────────
        let result: EvalResult

        switch mode {

        case .sync:
            // Synchronous mode: call eval on this thread.
            // Simple, deterministic, no DispatchGroup overhead.
            // The Waiting plugin is NOT invoked in sync mode because there is
            // no opportunity to interleave tick() calls with a blocking eval.
            result = language.eval(trimmed)

        case .async_mode:
            // Asynchronous mode: dispatch eval to a background thread and
            // tick the Waiting plugin on this thread while we wait.

            // We use a local variable captured by the async closure.
            // `nonisolated(unsafe)` is not needed here — we only read
            // evalResult AFTER the DispatchGroup confirms the closure finished.
            var evalResult: EvalResult = .error("eval did not complete")

            let group = DispatchGroup()
            group.enter()

            DispatchQueue.global().async {
                // Run the (potentially slow) evaluator on a background thread.
                evalResult = language.eval(trimmed)
                // Signal the DispatchGroup that we are done.
                group.leave()
            }

            if let w = waiting {
                // Waiting plugin is present: tick it every tickMs() ms.
                var state = w.start()

                // Poll: try to wait for `tickMs` milliseconds.
                // .timedOut → eval is still running, tick and try again.
                // .success  → eval finished, stop the animation.
                while group.wait(timeout: .now() + .milliseconds(w.tickMs())) == .timedOut {
                    state = w.tick(state)
                }

                // Eval finished — clean up the animation.
                w.stop(state)
            } else {
                // No Waiting plugin — block unconditionally until eval is done.
                group.wait()
            }

            result = evalResult
        }

        // ── Step 5: Handle the result ─────────────────────────────────────
        switch result {

        case .ok(let output):
            // Success. Print the output if there is one; do nothing otherwise.
            // Many REPL expressions (assignments, side-effect-only statements)
            // produce no displayable result.
            if let text = output {
                outputFn(text)
            }

        case .error(let message):
            // Failure. Prefix with "Error: " so the user can distinguish error
            // output from normal output even in monochrome terminals.
            outputFn("Error: \(message)")

        case .quit:
            // The user (or the language) asked to end the session.
            // Print the farewell and return immediately — do not loop again.
            outputFn("Goodbye!")
            return
        }
    }
    // Loop exited via EOF (inputFn returned nil). No farewell is printed — the
    // user's shell already handles "end of input" feedback.
}
