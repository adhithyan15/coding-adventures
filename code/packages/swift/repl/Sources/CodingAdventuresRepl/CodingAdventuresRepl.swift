// CodingAdventuresRepl.swift — Module entry point and documentation hub.
//
// This file serves two purposes:
//
//   1. It is the canonical "where do I start?" file for new readers.
//   2. It documents the framework's architecture and philosophy in one place.
//
// All symbols are defined in their own files (Types.swift, Language.swift,
// Prompt.swift, Waiting.swift, Runner.swift, EchoLanguage.swift,
// DefaultPrompt.swift, SilentWaiting.swift). This file imports nothing extra
// and re-exports nothing — Swift modules automatically export all public
// symbols from all files in the target.
//
// ─────────────────────────────────────────────────────────────────────────────
// Quick Start
// ─────────────────────────────────────────────────────────────────────────────
//
//   import CodingAdventuresRepl
//
//   runWithIO(
//       language: EchoLanguage(),
//       prompt: DefaultPrompt(),
//       waiting: SilentWaiting(),
//       inputFn: { readLine() },
//       outputFn: { print($0, terminator: "") }
//   )
//
// ─────────────────────────────────────────────────────────────────────────────
// Architecture
// ─────────────────────────────────────────────────────────────────────────────
//
// Three pluggable interfaces:
//
//   ┌────────────────────────────────────────────────────────────┐
//   │  Language  — eval(_ input: String) -> EvalResult           │
//   │  Prompt    — globalPrompt() -> String                      │
//   │              linePrompt() -> String                        │
//   │  Waiting   — start() -> State                              │
//   │              tick(_ state: State) -> State                 │
//   │              tickMs() -> Int                               │
//   │              stop(_ state: State)                          │
//   └────────────────────────────────────────────────────────────┘
//
// The runner (`runWithIO`) wires the three interfaces together in a
// read-eval-print loop. I/O is fully injected via `inputFn` / `outputFn`
// closures for deterministic testing.
//
// ─────────────────────────────────────────────────────────────────────────────
// Where It Fits in the Computing Stack
// ─────────────────────────────────────────────────────────────────────────────
//
//   Logic Gates → Arithmetic → CPU → ARM/RISC-V → Assembler
//     → Lexer → Parser → Compiler → VM → [REPL]
//
// The REPL framework sits at the very top. It wraps any evaluator — bytecode
// VM, interpreter, expression evaluator, or even a remote service — in a
// polished interactive shell.
//
// ─────────────────────────────────────────────────────────────────────────────
// Concurrency Model
// ─────────────────────────────────────────────────────────────────────────────
//
// In `.async_mode` (the default), eval runs on `DispatchQueue.global()`.
// The calling thread polls with `DispatchGroup.wait(timeout:)` every
// `tickMs()` milliseconds. This pattern is identical to Python's
// `thread.join(timeout=tick_ms/1000)` and Ruby's `thread.join(timeout)`.
//
// In `.sync` mode, eval blocks the calling thread directly with no
// concurrency overhead.
//
// ─────────────────────────────────────────────────────────────────────────────
// No External Dependencies
// ─────────────────────────────────────────────────────────────────────────────
//
// This package imports only `Foundation` (for `DispatchGroup` and
// `DispatchQueue`). No third-party packages are required.
