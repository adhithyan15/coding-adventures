# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-06

### Added

- `run_with_io(language, prompt, waiting, input_fn, output_fn)` — the core REPL loop with fully injected I/O, enabling testing without a terminal
- `run(language, prompt, waiting)` — convenience wrapper using `io.read` / `io.write` for interactive use
- **EchoLanguage** built-in Language plug-in: echoes any input back as `{tag="ok", output=input}`; maps `":quit"` to `{tag="quit"}`
- **DefaultPrompt** built-in Prompt plug-in: `global_prompt()` returns `"> "`, `line_prompt()` returns `"... "`
- **SilentWaiting** built-in Waiting plug-in: all methods are no-ops; `tick_ms()` returns 100
- Three documented plug-in conventions (Language, Prompt, Waiting) with tagged-union result type for Language eval
- `pcall()` wrapping of all `language.eval()` calls: panicking language plug-ins produce `{tag="error"}` rather than crashing the REPL
- Newline stripping: trailing `\n` is removed from input lines before passing to eval, normalising across different `input_fn` implementations
- Argument validation at `run_with_io` entry with clear error messages for missing or wrong-typed plug-ins
- Comprehensive busted test suite covering: EchoLanguage contract (echo, quit, case sensitivity, non-string guard), DefaultPrompt idempotency, SilentWaiting round-trip, end-to-end loop (echo, multi-line, quit, EOF, error display, pcall safety, prompt appearance, newline stripping, custom plug-ins, void output), and full module API surface
- Literate programming style with inline explanations of the REPL algorithm, the Lua synchronous-eval limitation, the tagged-union pattern, and the cooperative-coroutine constraint
