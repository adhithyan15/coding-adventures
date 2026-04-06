# Changelog — Parrot (Elixir)

All notable changes to this project will be documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/).

---

## [1.0.0] — 2026-04-06

### Added

- **`lib/parrot/prompt.ex`** — `Parrot.Prompt` module implementing the
  `CodingAdventures.Repl.Prompt` behaviour with parrot-themed strings:
  - `global_prompt/0` returns a multi-line banner (`"🦜 Parrot REPL\nI repeat everything you say! Type :quit to exit.\n\n"`)
  - `line_prompt/0` returns `"🦜 > "`

- **`lib/parrot/main.ex`** — `Parrot.Main` module, the escript entry point.
  Wires `EchoLanguage`, `Parrot.Prompt`, `SilentWaiting`, and real
  `IO.gets`/`IO.write` through `Loop.run/6`.

- **`test/parrot_test.exs`** — 25 ExUnit tests covering:
  - Basic echo (single input, raw output presence)
  - Quit handling (`:quit` ends session, subsequent inputs not echoed)
  - Multiple inputs echoed in order (3-input and 5-input sequences)
  - Sync mode (echo, quit, ordering)
  - Global prompt content and placement
  - Line prompt content and type
  - EOF handling (nil from input_fn, inputs before EOF)
  - Empty string echo
  - Whitespace echo (spaces, tab)
  - Parrot.Prompt module (all attributes)
  - Error output format (`"ERROR: "` prefix, session continues after error)
  - Banner frequency (once per REPL cycle)

- **`mix.exs`** — Mix project config with escript target (`main_module: Parrot.Main`),
  path dependency on `coding_adventures_repl`, 80% coverage threshold.

- **`BUILD`** / **`BUILD_windows`** — Build scripts for the monorepo build tool:
  Unix runs `mix deps.get 2>/dev/null && mix test`;
  Windows runs `mix deps.get && mix test`.

- **`README.md`** — Documentation with usage examples and example session.

### Dependencies

- `coding_adventures_repl` (path: `../../../packages/elixir/repl`) — the REPL
  framework package providing `Loop`, `EchoLanguage`, `SilentWaiting`,
  `Prompt`, and `Language` behaviours.
