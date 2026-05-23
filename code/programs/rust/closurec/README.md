# closurec

`closurec` is the CLI driver for the Closure Compiler clone.
Per [CLOC08](../../../specs/CLOC08-closurec-cli.md).

```text
closurec [OPTIONS] --input PATH
```

The binary ties together every crate in Stages 1–4: lexer,
parser, type sidecar, JSDoc extractor, type-checker, pass
pipeline + every canonical pass per CLOC06, emitter, and
source-map generator. This crate is just the glue.

## CLI surface (frozen in v0.1.0)

| Flag | Value | Default | Meaning |
|---|---|---|---|
| `--input PATH` | path | *(required)* | Input JavaScript file. |
| `--output PATH` | path | stdout | Where to write compiled output. |
| `--source-map BOOL` | bool | `true` | Emit companion source-map blob. |
| `--ascii-only BOOL` | bool | `false` | Escape non-ASCII characters in output. |
| `--pretty BOOL` | bool | `false` | Emit human-readable whitespace. |
| `--disable NAME` | pass name | — | Disable a pass. Repeatable. |
| `--help`, `-h` | — | — | Print usage and exit 0. |
| `--version`, `-V` | — | — | Print version and exit 0. |

BOOL values accept `true | false | 1 | 0 | yes | no | on | off`
(case-insensitive).

Known pass names (`--disable`):

```
constant-fold, fold-control-flow, dce, inline, rename, treeshake,
collapse-properties, remove-unused-vars
```

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success. |
| `2` | Usage error (POSIX convention). |

(Future `1` will mean compilation failure, once compilation
actually does anything.)

## Scope (v1)

The whole pipeline is **identity** today — `javascript-ast`
ships only `Program` / `SourceType` (CLOC02 Phase 1), so every
pass is a no-op and the emitter returns empty output. v1 of
`closurec` therefore:

- parses the full CLI surface (so users can already script
  against it),
- validates argument combinations and exits 2 on misuse,
- prints `closurec v0.1.0 - identity pipeline\n` on the happy
  path,
- exits 0 on success.

The point of locking the CLI surface this early is that build
systems and CI configs can wire up `closurec invocations` now,
and they won't need to change when the body fills in.

## Architecture

```text
  args ─► parse_args ──► ParseResult
                            │
                            ├─ Run(Action::PrintHelp)    ──► help_text()
                            ├─ Run(Action::PrintVersion) ──► version_string()
                            ├─ Run(Action::Compile(args)) ──► (v2+) pipeline
                            └─ UsageError(msg)            ──► "error: …" + exit 2
```

`parse_args` is a **pure function** of `&[String]` — no I/O, no
global state. Tests drive it directly without spawning the
binary, which keeps the test suite fast and deterministic.

`run(result)` is similarly pure: it converts a `ParseResult`
into `(text_to_print, ExitCode)`. `main` is a thin wrapper that
just prints + exits.

## What's coming

- v2: real lex → parse → typecheck → pipeline → emit wiring
  once the AST grows variants. `--input` actually reads a
  file; `--output` actually writes one. Compilation failures
  surface as exit code 1.
- A `--debug-cv` flag that dumps the CV log for tracing how
  each output byte came from which input byte.
- `--config FILE` for project-level defaults.

## Dependency whitelist

Front end: `javascript-tokens`, `javascript-ast`.
Type checker: `closure-typechecker`.
Pass pipeline: `closure-pass-pipeline` + every pass crate
(`constant-fold`, `fold-control-flow`, `dce`, `inline`, `rename`,
`treeshake`, `collapse-properties`, `remove-unused-vars`).
Back end: `closure-emitter`, `closure-source-map`.
Shared: `correlation-vector`, `type-sidecar`, `serde`,
`serde_json`.

No third-party CLI parser — `std::env::args` plus the tiny
parser in `parse_args` covers v1's surface and keeps cold-start
cheap.
