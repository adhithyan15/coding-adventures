# Changelog

All notable changes to the `coding-adventures-closurec` binary will be documented in this file.

## [0.1.0] - 2026-05-23

### Added
- New program per CLOC08 — the CLI driver that ties together every crate in Stages 1–4 (lexer, parser, type sidecar, JSDoc extractor, type-checker, pass pipeline + every canonical pass per CLOC06, emitter, source-map generator).
- CLI surface frozen:
  - `--input PATH` (required) — input JavaScript file.
  - `--output PATH` (defaults to stdout) — compiled output target.
  - `--source-map BOOL` (default `true`) — emit companion source-map blob.
  - `--ascii-only BOOL` (default `false`) — escape non-ASCII characters in output.
  - `--pretty BOOL` (default `false`) — emit human-readable whitespace.
  - `--disable NAME` (repeatable) — disable a canonical pass by name.
  - `--help` / `-h` — print usage and exit 0.
  - `--version` / `-V` — print version and exit 0.
- BOOL synonyms: `true|false|1|0|yes|no|on|off` (case-insensitive).
- Exit codes: 0 success, 2 usage error (POSIX convention). 1 reserved for future compilation failure.
- `parse_args(&[String]) -> ParseResult` is a pure function with no I/O — tests drive it directly without spawning the binary.
- `run(result) -> (text, ExitCode)` is similarly pure: converts a parse result into print-text + exit code. `main` is a thin wrapper that just prints + exits.
- 17 tests covering: short and long forms of `--help` and `--version`, `--help` short-circuits past other args, missing `--input` is a usage error, unknown flag is a usage error, `--input`-only sets the documented defaults, every option round-trips through `parse_args` correctly, all BOOL synonyms parse (forward and reverse cases), invalid BOOL value is a usage error mentioning both the flag and the bad value, flag-without-value is a usage error, empty argv is a usage error, help text mentions every known pass (catches additions to `KNOWN_PASSES` that miss the help text), `version_string()` includes `CARGO_PKG_VERSION`, `run()` produces the correct text for each variant including the v1 identity-pipeline banner for `Compile`.

### Notes
- v1 is scaffolding. The whole pipeline is identity today (`javascript-ast` ships only `Program` / `SourceType` per CLOC02 Phase 1), so a successful compile prints `closurec v0.1.0 - identity pipeline\n` and exits 0. The real lex → parse → typecheck → pipeline → emit wiring lands when the AST grows nodes. Pinning the CLI surface now means scripts and CI configs that invoke `closurec` won't need to change when the body fills in.
- Dependencies: every crate scaffolded in Stages 1–4. Listed exhaustively in `Cargo.toml`. No third-party CLI parser — `std::env::args` plus a small in-file parser covers v1.
- Required capabilities: `fs.read` + `fs.write` (CLOC08 binary reads input file, writes output + source-map). v1 doesn't actually touch the filesystem yet (identity body skips it) but the manifest declares the future surface.
