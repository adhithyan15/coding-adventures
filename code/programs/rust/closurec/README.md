# closurec

`closurec` is the CLI driver for the Closure Compiler clone.
It tracks the upstream Java Closure Compiler's canonical command-line surface,
so a script written against
`java -jar closure-compiler.jar --js foo.js --js_output_file
out.js --compilation_level ADVANCED` works unchanged when the
`java -jar …` invocation is swapped for `closurec`. Per
[CLOC08](../../../specs/CLOC08-closurec-cli-surface.md).
Four deprecated upstream long aliases remain explicitly unsupported below;
semantic support also varies by flag and is tracked in the parity backlog.

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

When upstream Closure Compiler adds a flag, we update `cli.spec.json` and the
binary picks it up. The pinned surface audit below makes an unclassified local
or upstream change fail CI rather than relying on this prose.

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

`--js` and `--externs` accept literal files, `*`, `**`, `?`, character
classes, and leading-`!` exclusions. Paths use the host filesystem's native
component rules: Windows callers may use backslashes, forward slashes, or a
mixture after a drive or UNC root, while Unix keeps backslash as an ordinary
filename character. Matches are sorted deterministically within each inclusion
and duplicate files retain their first command-line position.

Short aliases the Java tool ships:

| Short | Long |
|-------|------|
| `-O`  | `--compilation_level` |
| `-W`  | `--warning_level` |
| `-D`  | `--define` |

`--typed_ast_output_file` is the current canonical upstream spelling.
`closurec` also accepts its historical misspelling
`--typed_ast_output_file__INTENRNAL_USE_ONLY` as a deprecated compatibility
alias; both spellings populate the same runtime configuration field, and
conflicting values fail closed.

### Machine-audited surface

[`tests/cli-surface/v20260915-audit.json`](./tests/cli-surface/v20260915-audit.json)
pins the 87,726-byte `CommandLineRunner.java` blob at upstream commit
`10ca677aff381d2c2e6e1b254ba32861e503173d`, including its Git blob ID and
SHA-256. The deterministic report classifies 102 upstream options, seven
upstream aliases, 112 local spec flags, two cli-builder-generated flags,
eleven local correlation-vector extensions, one deprecated compatibility
alias, and the four unsupported long aliases listed below.

The offline `cli_surface` integration test verifies that every upstream and
local name has exactly one reviewed disposition, the report still hashes the
current `cli.spec.json`, supported single-dash aliases are present, and no
unsupported alias disposition has gone stale. To regenerate from an
independently obtained pinned source file:

```sh
cargo run --example cli_surface_audit -- \
  --upstream-source /path/to/CommandLineRunner.java \
  --output tests/cli-surface/v20260915-audit.json
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0    | Success. |
| 1    | CLI parse error, or SIMPLE/ADVANCED compilation failed during parse, typed-AST bridging, an optimization pass, or emission. |
| 2    | Execution failure outside the typed pipeline, including input/output and glob errors. |
| 70   | Internal error (`cli.spec.json` malformed — bug in *us*). `EX_SOFTWARE` per `sysexits.h`. |

Compiler diagnostics are written to stderr. A failed SIMPLE/ADVANCED compile
writes no JavaScript output and does not create the requested output file.

## Known compatibility gaps

cli-builder doesn't currently support long-form aliases per flag. These exact
upstream aliases are classified as **unsupported** by the audit — use the
canonical name instead:

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
| `SIMPLE` | Runs `parse → bridge → constant-fold → fold-control-flow → dce → inline-variables → rename → emit`. It preserves top-level declarations under SIMPLE's open-world contract. |
| `ADVANCED` | Runs the SIMPLE passes plus closed-world `inline`, `remove-unused-vars`, `treeshake`, and `rename-globals`; `rename-properties` runs only with an explicit externs boundary. More advanced-only passes remain planned. |
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

SIMPLE and ADVANCED are fail-closed. If closurec's typed AST cannot represent a
valid construct yet, the command reports the `typed AST bridge` stage and the
unsupported grammar rule instead of silently returning WHITESPACE_ONLY bytes.
Malformed JavaScript likewise reports the `parse` stage and exits 1. This makes
an exit-0 result trustworthy: the requested typed pipeline actually completed.

## Architecture

```text
  args ─► cli_builder::Parser::parse ──► ParserOutput
                                            │
                                            ├─ Parse(r)   ──► wire config ──► run_compiler
                                            ├─ Help(h)    ──► h.text
                                            └─ Version(v) ──► v.version
```

`parse_and_run_with_streams(args)` returns `(stdout, stderr, ExitCode)` so tests
can exercise routing without spawning the binary. `parse_and_run(args)` is the
combined-stream compatibility helper; `main` only writes the returned streams.

## Reproducible upstream oracle

[`tests/oracle/manifest.json`](./tests/oracle/manifest.json) is the
machine-readable provenance registry for all 626 differential fixture
directories. It pins Google Closure Compiler `v20260915`, its annotated tag and
release commit, the exact Maven JAR URL, byte length and SHA-256, Java 21.0.12,
licensing, deterministic capture settings, and normalized command templates.

The 462 `minify_*` goldens have now been re-executed against that exact artifact
and Java 21.0.12. All 462 stdout captures are byte-identical to the checked-in
goldens, with zero changed and zero declined. The strict
[`minify-v20260915-report.json`](./tests/oracle/minify-v20260915-report.json)
preserves hashes, exit status, and complete stdout/stderr bytes for every case.
It also records the three successful legacy-octal compilations where upstream
emits warnings even though stdout matches. Local correlation-vector,
help/version, and transitional print-tree contracts still do not pretend to be
upstream bytes.

Oracle execution is an explicit maintainer action. After independently
obtaining and verifying the pinned artifact, run:

```sh
cargo run --example oracle_refresh -- \
  --oracle-jar /absolute/path/to/closure-compiler-v20260915.jar \
  --java /absolute/path/to/java-21.0.12
```

The tool never downloads Java or the JAR. It requires an absolute Java path,
enforces independent hard-coded trust pins and exact JVM argv, fully preflights
the cohort, and executes private snapshots of the verified JAR/input bytes with
JVM injection variables removed. Streamed size and aggregate budgets,
per-process timeout/output caps, child cleanup, before/after Java identity
checks, and atomic non-symlink report replacement keep regeneration fail-closed.
It fails before execution on artifact hash/size, Java version, cohort,
flag-shape, or path-containment drift. See
[`tests/oracle/README.md`](./tests/oracle/README.md) for the complete review
procedure.

The verifier is offline and does not execute Java:

```sh
cargo test --test oracle_manifest
cargo test --test oracle_refresh_report
```

It rejects unclassified or multiply classified fixtures, unknown schema data,
bad pins, unsafe/stale paths, incomplete harness mappings, false upstream
claims, malformed hashes/review links, altered capture bytes, and unreviewed oracle drift. A new fixture
therefore cannot inherit provenance merely because its directory happens to
match a glob. Normal tests and CI never download or execute the JAR.

## What's coming

- Complete typed-AST coverage so every JavaScript construct accepted upstream
  can remain on the SIMPLE/ADVANCED pipeline.
- Route remaining flags to pass configuration —
  `--jscomp_off`/`--jscomp_warning`/`--jscomp_error` populate
  the warning level map; `--define` populates a value map the
  passes consult; `--compilation_level` selects a canonical
  pass preset.
- Add hyphenated long-form aliases when
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
  `serde_json`; the explicit oracle maintainer target uses the repository's
  `sha256` crate as a development-only dependency.
