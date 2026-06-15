# Changelog

## 0.1.0 — initial release

First release of the SIR regex runtime for Python, per
`code/specs/sir-runtime.md`. Provides the runtime landing point for the SIR
`regex` builtin — the Ruby→SIR frontend lowers `/pat/flags` to
`BuiltinCall("regex", [StrLit pattern, StrLit flags])`, which previously had no
Python runtime, so emitted code using a regex failed. Python mirror of
`@coding-adventures/sir-runtime-regex` (TypeScript).

### Added

- `compile(pattern, flags="") -> re.Pattern[str]` — compile a Ruby-dialect
  pattern, translating Ruby inline flags to Python `re` flags (`i`→IGNORECASE,
  `m`→DOTALL since Ruby `/m` is dot-matches-newline, `x`→VERBOSE; unknown chars
  ignored) and **always** OR-ing in `re.MULTILINE` because Ruby's `^`/`$` are
  always line anchors.
- `is_match(pattern, string) -> bool` — unanchored search (Ruby `=~`/`match?`
  semantics); accepts a compiled pattern or a raw string.
- `match_data(pattern, string) -> str | None` — matched substring (group 0) or
  `None`; minimal model of Ruby `String#match`.
- `_compiled(pattern)` helper — pass through a compiled pattern, else compile a
  raw value.
- `Val` — the universal SIR value type alias at this boundary.
- Full standard layout: `pyproject.toml` (src layout), `BUILD`, `BUILD_windows`,
  `required_capabilities.json` (no capabilities), `py.typed`, README. pytest
  suite at 100% coverage of `regex.py`; `mypy --strict` + `ruff` clean.

### Design note

`compile` intentionally shadows `builtins.compile` / `re.compile` because
`regex` is the SIR builtin's name and emitted code addresses this package's
`compile` by qualified name. The Ruby→Python divergence is concentrated in the
flag mapping and the unconditional `re.MULTILINE` (Ruby line anchors); the
shared Perl-compatible syntax passes straight through to the `re` engine.
