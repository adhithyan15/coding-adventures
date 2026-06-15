# Changelog

## 0.1.0 — initial release

First release of the SIR shell runtime for Python, per
`code/specs/sir-runtime.md`. Provides the runtime landing point for the SIR
`backtick` builtin — the Ruby→SIR frontend lowers a backtick literal `` `cmd` ``
to `BuiltinCall("backtick", [StrLit cmd])`, which previously had no Python
runtime, so emitted code using a backtick failed. Python mirror of
`@coding-adventures/sir-runtime-shell` (TypeScript).

### Added

- `backtick(command) -> str` — run `command` via the system shell and return its
  captured stdout as a `str`, modelling Ruby's `` `cmd` `` expression. Implemented
  with `subprocess.run(command, shell=True, capture_output=True, text=True,
  check=False)`. The child's exit status is ignored (`check=False`), so a non-zero
  exit still returns whatever stdout was produced; stderr is captured by the call
  but not returned (Ruby's backtick value is stdout only).
- `Val` — the universal SIR value type alias at this boundary.
- Full standard layout: `pyproject.toml` (src layout, hatchling), `BUILD`,
  `BUILD_windows`, `required_capabilities.json` (one `process`/`spawn`
  capability), `py.typed`, README. pytest suite at 100% coverage of `shell.py`;
  `mypy --strict` + `ruff` clean.

### Design note — `shell=True` is intentional

`shell=True` is load-bearing, not an oversight: Ruby backtick is *defined* as
"run via the system shell" (`/bin/sh -c`), so shell features (pipes,
redirections, globbing, `$VAR`) are part of the builtin's contract. The
`command` is author-supplied — the string literal from the compiled program's
own Ruby source — and this package interpolates no external/untrusted runtime
input, so it introduces no new injection surface beyond what Ruby itself grants.
The package therefore declares a single `process`/`spawn` capability.
