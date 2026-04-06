# Changelog — Parrot (Python)

All notable changes to this project will be documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/).

---

## [1.0.0] — 2026-04-06

### Added

- **`src/parrot/prompt.py`** — `ParrotPrompt` class implementing
  `coding_adventures_repl.Prompt` with parrot-themed strings:
  - `global_prompt()` returns `"🦜 Parrot REPL\nI repeat everything you say! Type :quit to exit.\n\n"`
  - `line_prompt()` returns `"🦜 > "`

- **`src/parrot/main.py`** — `main()` entry point and `_read_line()` helper.
  Wires `EchoLanguage`, `ParrotPrompt`, `SilentWaiting`, and real
  `sys.stdin.readline` / `sys.stdout.write` through `run_with_io`.
  Handles EOF correctly: `readline()` returns `""` on EOF which is
  converted to `None` (end-of-input signal) by `_read_line`.

- **`src/parrot/__init__.py`** — Package docstring explaining Parrot's
  purpose and providing usage examples.

- **`tests/test_parrot.py`** — 30+ pytest tests organised into 14 classes:
  - `TestBasicEcho` — single input echoed, raw output presence
  - `TestQuit` — `:quit` stops loop, no result output, queued inputs ignored
  - `TestMultipleInputs` — ordering and completeness of multiple echoes
  - `TestSyncMode` — echo, quit, ordering in `mode="sync"`
  - `TestAsyncMode` — echo with default and explicit `mode="async"`
  - `TestBannerContent` — "Parrot", emoji, `:quit` instruction, appears in output
  - `TestLinePrompt` — emoji, string type, format, differs from global
  - `TestEOF` — immediate EOF, inputs before EOF, explicit None in queue
  - `TestEmptyString` — empty string echoed, not suppressed
  - `TestErrorOutput` — error format, session continues after error
  - `TestGlobalPromptFrequency` — once per cycle, first item in output
  - `TestOutputCollection` — list type, minimum content
  - `TestLinePromptDetails` — starts with emoji, ends with space, short length
  - `TestGlobalPromptContent` — string type, multiline, double newline ending,
    repeat instruction

- **`pyproject.toml`** — Hatchling build config; `parrot` console script;
  `coding-adventures-repl` runtime dependency; `dev` extras (pytest, ruff).

- **`BUILD`** / **`BUILD_windows`** — Build scripts for the monorepo:
  Unix: `uv venv + uv pip install + pytest`;
  Windows: `uv venv + uv pip install --no-deps + uv run pytest`.

- **`README.md`** — Documentation with architecture diagram, example session,
  and usage instructions.

- **`tests/__init__.py`** — Empty file making `tests/` a Python package.

### Dependencies

- `coding-adventures-repl` (path: `../../../packages/python/repl`) — the REPL
  framework providing `EchoLanguage`, `SilentWaiting`, `Prompt`, and
  `run_with_io`.
