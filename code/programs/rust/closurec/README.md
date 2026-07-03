# closurec

`closurec` is the CLI driver for the Closure Compiler clone.
**Drop-in compatible with the upstream Java Closure Compiler at
the command-line surface** — a script written against
`java -jar closure-compiler.jar --js foo.js --js_output_file
out.js --compilation_level ADVANCED` works unchanged when the
`java -jar …` invocation is swapped for `closurec`. Per
[CLOC08](../../../specs/CLOC08-closurec-cli-surface.md).

The binary ties together every crate in Stages 1–4: lexer,
parser, type sidecar, JSDoc extractor, type-checker, pass
pipeline + every canonical pass per CLOC06, emitter, and
source-map generator. This crate is just the glue.

## How the CLI is built

`closurec` uses [`cli-builder`](../../../packages/rust/cli-builder)
for argument parsing. The flag surface is declared **declaratively**
in [`cli.spec.json`](./cli.spec.json), a cli-builder JSON spec
that mirrors `CommandLineRunner.java` upstream. The spec is
embedded into the binary via `include_str!` at compile time —
no runtime file lookup is required for the parser to come up.

```rust
const CLI_SPEC_JSON: &str = include_str!("../cli.spec.json");
let spec = load_spec_from_str(CLI_SPEC_JSON)?;
let parser = Parser::new(spec);
let output = parser.parse(&argv)?;
```

cli-builder handles:

- multi-token flags (`--js file.js`),
- enum-value validation (`--compilation_level ADVANCED`),
- repeatable flags (`--js a.js --js b.js --js c.js`),
- short aliases (`-O ADVANCED` ↔ `--compilation_level ADVANCED`),
- type validation (integers, booleans, paths, enums),
- conflict checking,
- fuzzy "did you mean?" suggestions on unknown flags,
- `--help` and `--version` auto-injection.

When upstream Closure Compiler adds a flag, we update
`cli.spec.json` and the binary picks it up — no code changes.

## CLI surface

The full ~100-flag surface is in `cli.spec.json`. Highlights:

```
closurec --js src/foo.js --js src/bar.js \
         --js_output_file out/bundle.js \
         --compilation_level ADVANCED \
         --language_in ECMASCRIPT_2021 \
         --language_out ECMASCRIPT_2015 \
         --create_source_map out/bundle.js.map \
         --warning_level VERBOSE \
         --jscomp_off lintChecks \
         --define DEBUG=false \
         --formatting PRETTY_PRINT \
         --formatting SINGLE_QUOTES
```

Short aliases the Java tool ships:

| Short | Long |
|-------|------|
| `-O`  | `--compilation_level` |
| `-W`  | `--warning_level` |
| `-D`  | `--define` |

## Exit codes

| Code | Meaning |
|------|---------|
| 0    | Success. |
| 1    | Parse error (unknown flag, invalid value, missing required flag, conflicting flags). |
| 70   | Internal error (`cli.spec.json` malformed — bug in *us*). `EX_SOFTWARE` per `sysexits.h`. |

Future v2 codes:
- 2 reserved for usage error.
- 3 reserved for compilation error.
- 4 reserved for I/O error.

## Known compatibility gaps in v0.1.0

cli-builder doesn't currently support multiple long-form aliases
per flag. These deprecated upstream aliases are **not
implemented** in v0.1.0 — use the canonical name instead:

| Deprecated alias    | Use instead              |
|---------------------|--------------------------|
| `--checks-only`     | `--checks_only`          |
| `--dev_mode`        | `--jscomp_dev_mode`      |
| `--warnings_whitelist_file` | `--warnings_allowlist_file` |
| `--D` (long form)   | `--define` or `-D`       |

Real-world Closure Compiler invocations use the canonical
underscored names; these deprecated forms are rarely seen.

## Scope (v1)

The whole pipeline is **identity** today — `javascript-ast`
ships only `Program` / `SourceType` per CLOC02 Phase 1. v1 of
`closurec`:

- parses every Closure Compiler flag (validation, type
  checking, repeatable handling, enum values),
- returns clear errors on misuse (cli-builder collects every
  error in a single pass and offers "did you mean?" suggestions),
- on a valid invocation, prints
  `closurec v0.1.0 - identity pipeline\n` and exits 0.

The actual lex/parse/typecheck/passes/emit wiring lands when
the AST grows nodes. **Pinning the Closure-compatible CLI
surface now means scripts and CI configs that invoke the Java
tool today can target `closurec` with no flag changes when the
body fills in.**

## Compilation levels

`--compilation_level` (`-O`) selects how hard the compiler works:

| Level | What it does |
|-------|--------------|
| `WHITESPACE_ONLY` | Strips comments and inter-token whitespace only. Token-level; never parses to a typed AST. |
| `SIMPLE` | Runs the typed-AST optimization pipeline — parse → bridge → passes → emit. Today the pipeline is `constant-fold → fold-control-flow → dce → inline → remove-unused-vars → treeshake → rename` (e.g. `1 + 2` ⇒ `3`; `if (2 > 3) {a} else {b}` ⇒ `{b}`; code after a `return` is dropped; unused top-level `var`s with pure initializers and unused top-level `function`s are deleted; leaf-function parameters are shortened to `a`, `b`, …); more passes land one PR at a time. Falls back to `WHITESPACE_ONLY` if the source uses a not-yet-supported construct, so it never errors on valid input. |
| `ADVANCED` | Runs the same typed optimization pipeline as `SIMPLE` (it is specified to be at least as aggressive). Advanced-only passes — aggressive property/global renaming, cross-module tree-shaking — layer on as they are implemented. |
| `BUNDLE` / `TRANSPILE_ONLY` | Identity passthrough for now — module bundling and language down-levelling are orthogonal to the optimization pipeline and land separately. |

The SIMPLE pipeline:

```text
source ──parse──▶ grammar AST ──bridge──▶ typed Program
       ──passes──▶ optimized Program ──emit──▶ JS text
```

```sh
# SIMPLE evaluates constant expressions at compile time:
closurec --compilation_level SIMPLE --js in.js
#   var x = 1 + 2;   ⇒   var x=3;
```

## Architecture

```text
  args ─► cli_builder::Parser::parse ──► ParserOutput
                                            │
                                            ├─ Parse(r)   ──► run_pipeline(r)   (v1: banner)
                                            ├─ Help(h)    ──► h.text
                                            └─ Version(v) ──► v.version
```

`parse_and_run(args)` is a **pure function** returning
`(text, ExitCode)` so tests can exercise the whole pipeline
without spawning the binary. `main` is a thin wrapper.

## What's coming

- v2: real lex → parse → typecheck → pipeline → emit wiring
  once the AST grows variants. `--js` actually reads files;
  `--js_output_file` actually writes one. Compilation failures
  surface as exit code 3.
- v2: route specific flags to pass configuration —
  `--jscomp_off`/`--jscomp_warning`/`--jscomp_error` populate
  the warning level map; `--define` populates a value map the
  passes consult; `--compilation_level` selects a canonical
  pass preset.
- v0.2 alias enhancement: hyphenated long-form aliases when
  cli-builder grows alias support.

## Dependency whitelist

- `cli-builder` — declarative CLI parsing.
- Front end: `javascript-tokens`, `javascript-ast`.
- Type checker: `closure-typechecker`.
- Pass pipeline: `closure-pass-pipeline` + every pass crate
  (`constant-fold`, `fold-control-flow`, `dce`, `inline`,
  `rename`, `treeshake`, `collapse-properties`,
  `remove-unused-vars`).
- Back end: `closure-emitter`, `closure-source-map`.
- Shared: `correlation-vector`, `type-sidecar`, `serde`,
  `serde_json`.
