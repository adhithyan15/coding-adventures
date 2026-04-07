# Changelog

All notable changes to this package will be documented in this file.

## [0.01] - 2026-04-06

### Added

- Initial implementation of the REPL framework
- `CodingAdventures::Repl` — main entry point with `run()` and `run_with_io()`
- `CodingAdventures::Repl::Language` — base class and interface documentation for the Language duck type
- `CodingAdventures::Repl::Prompt` — base class and interface documentation for the Prompt duck type
- `CodingAdventures::Repl::Waiting` — base class and interface documentation for the Waiting duck type
- `CodingAdventures::Repl::Loop` — the loop engine; reads from `input_fn`, writes to `output_fn`, handles exceptions via `eval {}`
- `CodingAdventures::Repl::EchoLanguage` — built-in demo language; `:quit` exits, everything else echoes
- `CodingAdventures::Repl::DefaultPrompt` — built-in prompt returning `"> "` and `"... "`
- `CodingAdventures::Repl::SilentWaiting` — built-in no-op waiting handler (Null Object pattern)
- Full I/O injection via `input_fn` / `output_fn` coderefs for testability
- Exception safety: all `eval()` calls wrapped in Perl `eval {}` blocks
- Synchronous evaluation model (no threads); rationale documented in `Waiting.pm`
- Literate programming style with extensive inline POD documentation
- 12 test cases in `t/01-basic.t` covering all major code paths
- `t/00-load.t` smoke test verifying all modules load
