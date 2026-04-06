# Changelog — parrot (TypeScript)

All notable changes to this program are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [1.0.0] — 2026-04-06

### Added

- Initial release of the Parrot REPL program.
- `src/prompt.ts` — `ParrotPrompt` class implementing the `Prompt` interface
  from `@coding-adventures/repl` with parrot-themed banner and line prompts.
- `src/main.ts` — entry point that wires `EchoLanguage`, `ParrotPrompt`, and
  `SilentWaiting` together with readline-based stdin/stdout I/O.
- `tests/parrot.test.ts` — 15 unit tests covering echo behaviour, quit
  handling, sync/async modes, prompt content, EOF handling, empty input, error
  output format, and output collection accuracy.
- `BUILD` and `BUILD_windows` — build scripts that install the repl dependency
  then run vitest.
- `package.json` with `@coding-adventures/repl` as a `file:` path dependency.
- `tsconfig.json` with strict mode and ESNext module resolution.
- `vitest.config.ts` with v8 coverage thresholds (80% lines/functions/branches).
