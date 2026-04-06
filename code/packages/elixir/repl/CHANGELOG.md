# Changelog

## 0.1.0 — 2026-04-06

### Added

- `CodingAdventures.Repl.run/4` — start an interactive session on the real terminal
- `CodingAdventures.Repl.run_with_io/5` — run with injected I/O for testing and programmatic use
- `CodingAdventures.Repl.step/6` — execute a single REPL iteration in isolation
- `CodingAdventures.Repl.Language` behaviour — `eval/1` callback contract for language plugins
- `CodingAdventures.Repl.Prompt` behaviour — `global_prompt/0` and `line_prompt/0` callbacks
- `CodingAdventures.Repl.Waiting` behaviour — `start/0`, `tick/1`, `tick_ms/0`, `stop/1` callbacks
- `CodingAdventures.Repl.Loop` — async eval loop using `Task.async` + `Task.yield` with exception safety
- `CodingAdventures.Repl.EchoLanguage` — built-in Language that echoes input; `:quit` exits
- `CodingAdventures.Repl.DefaultPrompt` — built-in Prompt with `"> "` and `"... "`
- `CodingAdventures.Repl.SilentWaiting` — built-in no-op Waiting with 100 ms poll interval
- 30+ tests covering echo, quit, multiple turns, nil output suppression, error formatting, exception recovery, EOF handling, and all built-in plugin contracts
