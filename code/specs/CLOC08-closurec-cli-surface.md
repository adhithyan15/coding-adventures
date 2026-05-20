# CLOC08 — `closurec` CLI Surface

## What this spec locks down

CLOC08 is the last foundational spec in the series. It defines the
public surface of `closurec`, the binary that ties everything else
together: argument parsing, file I/O, config files, env vars, exit codes,
diagnostic formatting, and integration with the existing repo build tool.

Once a user has source files and (optionally) sidecars, `closurec` is
how they actually run the Closure Compiler clone. Locking the surface
now lets every downstream pass and emitter PR test against a stable
caller.

## Crate location and layout

```text
code/packages/rust/closure-cli/
  BUILD
  BUILD_windows
  CHANGELOG.md
  Cargo.toml
  README.md
  required_capabilities.json
  src/
    main.rs               # entry point; argparse + dispatch
    args.rs               # clap-derive struct + validation
    config.rs             # closurec.config.json loader + merge with CLI
    env.rs                # CLOSUREC_* env var reader
    diagnostics.rs        # CV-resolved Origin printer, severity model
    input.rs              # file/glob/stdin discovery
    output.rs             # file/dir/stdout writer
    subcommands/
      compile.rs          # default subcommand
      check.rs            # --check (typecheck only, no emit)
      print_ast.rs        # --print-ast (debug helper)
      dump_cv.rs          # --dump-cv (debug helper)
```

Crate name: `coding-adventures-closure-cli`. Binary name: `closurec`.

### Dependency whitelist

- `coding-adventures-javascript-parser` — to produce `Program`.
- `coding-adventures-javascript-ast` — `Program` type.
- `coding-adventures-type-sidecar` + `coding-adventures-type-sidecar-merger`.
- `coding-adventures-jsdoc-types-extractor` (when `--jsdoc`).
- `coding-adventures-closure-typechecker`.
- `coding-adventures-closure-pass-pipeline`.
- All the `coding-adventures-closure-pass-*` crates (linked statically).
- `coding-adventures-closure-emitter`.
- `coding-adventures-closure-source-map`.
- `coding-adventures-correlation-vector`.
- `clap` (with `derive`) for argparse.
- `serde` + `serde_json` for config / sidecar I/O.
- `glob` for file discovery.

No async runtime. `closurec` is synchronous; the pass pipeline is already
single-threaded by default per CLOC06.

## Subcommand structure

`closurec` is *technically* a single binary but acts like it has
subcommands selected by mode flags:

| Invocation | Behavior |
| --- | --- |
| `closurec INPUT...` | **Default**: full compile pipeline → optimized JS + optional source map. |
| `closurec --check INPUT...` | Run typechecker only. No emitter, no source map, no output file. Exit 0 if no errors. |
| `closurec --print-ast INPUT` | Parse + print the AST as JSON. Skip typechecking and optimization. Useful for debugging the frontend. |
| `closurec --dump-cv INPUT` | Run the full pipeline but emit the CV log as a JSON document to stdout (or `--cv-out PATH`). For debugging "where did this byte come from." |
| `closurec --version` | Print version + linked-pass list. |
| `closurec --help` | Standard help. |

`--check`, `--print-ast`, `--dump-cv` are mutually exclusive. The CLI
errors clearly if more than one is set.

These aren't `clap` subcommands in the strict sense — they're top-level
flags, because `closurec`'s primary mode is compile, and we want to
keep the invocation `closurec INPUT.js -o OUT.js` short.

## Input modes

```text
closurec INPUT.js                      # single file
closurec INPUT1.js INPUT2.js INPUT3.js # multiple files (concatenated for module-style; or grouped per --separate)
closurec 'src/**/*.js'                 # glob — quoted to bypass shell expansion when needed
closurec --files-from list.txt         # one path per line
closurec -                             # stdin (single file mode only)
```

Resolution rules:

- Multiple positional args + at least one being a glob → all globs
  resolved, results sorted lexicographically, deduplicated.
- `--files-from` is additive with positional args.
- `-` (stdin) is its own mode; mutually exclusive with positional args
  and `--files-from`.
- An empty input set is an error.

Each input file's name is recorded as the CV `Origin.source` field per
CLOC03. The CLI passes the *absolute* path by default; `--relative-paths`
makes them relative to a `--base-dir` (default: current working
directory).

## Output modes

```text
closurec INPUT.js -o OUT.js                  # explicit output file
closurec INPUT.js --out-dir out/             # directory; output named like input
closurec INPUT.js                            # stdout (no -o)
closurec --print-ast INPUT.js > tree.json    # debug subcommands also flow through stdout
```

Multi-input compile modes:

- **Single output**: `-o OUT.js` requires the inputs to be a single
  logical module. Concatenation is **not** done by `closurec`; users use
  the `--bundle` flag (deferred to a later spec) for bundling.
- **Output dir**: `--out-dir out/` writes one output file per input,
  preserving the input filename. Used for batch mode (linting workflows).

If both `-o` and `--out-dir` are present, the CLI errors.

If no output flag is given and there's exactly one input, output goes to
stdout. For multi-input invocations with no output flag, the CLI errors
("specify --out-dir or use --check for typecheck-only").

## Mode flag (compilation level)

```text
closurec INPUT.js --mode=simple              # SIMPLE
closurec INPUT.js --mode=advanced            # ADVANCED  (default in CI; opt-in elsewhere)
closurec INPUT.js --mode=custom              # CUSTOM (requires --enabled-pass or config)
closurec INPUT.js --mode=advanced --disabled-pass=collapse-properties
closurec INPUT.js --mode=custom \
    --enabled-pass=constant-fold,dce \
    --enabled-pass=remove-unused-vars
```

`--enabled-pass` and `--disabled-pass` are comma-separated AND repeatable
— both forms compose. SIMPLE/ADVANCED ignore them by default unless
`--mode=custom`.

Default mode: **SIMPLE**. ADVANCED requires the user to opt in because it
performs name-changing optimizations that need explicit consent (the
classic Closure Compiler footgun is users running ADVANCED on code that
isn't annotated and breaking it). Closure-style.

## Sidecar flags

```text
closurec INPUT.js --jsdoc                       # extract JSDoc from comments → sidecar
closurec INPUT.js --sidecar=types.sidecar.json  # external sidecar file
closurec INPUT.js --sidecar=a.json --sidecar=b.json --merge-policy=strict
closurec INPUT.js --ts-types=src/lib.ts         # invoke TS extractor (when implemented)
closurec INPUT.js --no-sidecar                  # force-disable all sidecars (override config)
```

Composition order (top wins on conflict, before merger policy applies):

1. `--no-sidecar` (kills all sidecars).
2. Multiple `--sidecar` files in argv order.
3. `--jsdoc` (extracted from input comments).
4. `--ts-types` (extracted from TS).
5. Sidecars from `closurec.config.json`.

The merger from CLOC04 combines them with the policy from
`--merge-policy={default,strict,ts-wins}`. Default is `default`.

## Trace and source-map flags

Cross-references CLOC07 §"Behavior when cv.enabled == false." The
canonical table:

| Flags | Trace state | Source map output |
| --- | --- | --- |
| (default) | `enabled=true` | written next to output |
| `--source-map=PATH` | forced `enabled=true` | written to PATH |
| `--source-map-inline` | forced `enabled=true` | inlined as `data:` URL |
| `--source-map=PATH --source-map-inline` | — | error: choose one |
| `--no-trace` | `enabled=false` | none; warn if any sm flag set |
| `--no-trace --source-map=PATH` | — | error: contradiction |
| `--source-map-multi` | forced `enabled=true` | adds `x_closure_multi_origins` |
| `--no-source-map` | `enabled=true` (still useful for debug) | no map written |

Additional knobs:
- `--source-map-include-sources` — embed `sourcesContent`.
- `--source-root=ROOT` — sets the `sourceRoot` field.
- `--consume-input-map=PATH` — feeds an input source map per CLOC07.

## Diagnostic severity model (CLOC06 OQ5 resolved)

```rust
pub enum Severity { Error, Warning, Note }

pub struct DiagnosticGroup(&'static str);
// e.g. "missing-types", "unreachable-code", "ambiguous-jsdoc"

pub struct Diagnostic {
    pub severity: Severity,
    pub group: DiagnosticGroup,
    pub message: String,
    pub cv: CvId,                       // resolves to Origin for printing
    pub notes: Vec<Diagnostic>,         // attached "note:" lines
    pub suppressed_by: Option<String>,  // matches a @suppress {group} JSDoc tag
}
```

User controls:

```text
--strict                              # promote all warnings to errors
--strict-group=missing-types          # promote only that group
--allow-group=ambiguous-jsdoc         # demote errors of that group to warnings
--quiet-group=unreachable-code        # suppress that group entirely
--list-groups                         # print all known diagnostic groups
```

Interaction with JSDoc `@suppress`:

- A `@suppress {group1|group2}` JSDoc tag on a declaration suppresses
  diagnostics whose `group ∈ {group1, group2}` for nodes anchored to that
  declaration.
- `--strict` does **not** override `@suppress`. Users opt in to ignoring
  source-level suppressions with `--ignore-suppress`.

### Diagnostic output format

Default: clang/rustc-style — file:line:col, severity, group, message,
caret-pointing snippet. Resolved from `Diagnostic.cv` via
`cv.resolve_root` per CLOC07.

```text
warning [missing-types]: implicit `any` in parameter `id`
  --> src/api.js:42:18
   |
42 | function getUser(id) {
   |                  ^^
   = note: add `@param {number} id` or `--allow-group=missing-types` to silence
```

Alternative formats:
- `--diagnostic-format=json` — emits NDJSON, one diagnostic per line.
  Tooling-friendly. Each record includes the CV chain for navigation.
- `--diagnostic-format=brief` — single-line per diagnostic, no carets.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Success — no errors emitted, output produced (if requested). |
| `1` | One or more `Severity::Error` diagnostics. Output **not** written. |
| `2` | `--strict` mode and at least one warning was emitted (which got promoted to error). |
| `3` | Invalid CLI arguments (bad combo, missing required, unknown flag). |
| `4` | Invalid config file (`closurec.config.json` parse error or schema violation). |
| `5` | I/O error (input not readable, output not writable, etc.). |
| `64–127` | Internal compiler errors (`PassError`, ICE). The 6x range signals "file a bug." |
| `>= 128` | Reserved for signal-induced exits per POSIX. |

`closurec --check` follows the same codes — exit `0` on type-clean,
exit `1` on type errors.

## Config file: `closurec.config.json`

Loaded by default from `./closurec.config.json` (CWD) and the nearest
ancestor up to the user's home directory. Overridden by
`--config=PATH` or `--no-config`.

```json
{
  "$schema": "https://schemas.coding-adventures.dev/closurec.v1.json",
  "mode": "advanced",
  "inputs": ["src/**/*.js"],
  "out_dir": "dist/",
  "source_map": true,
  "source_map_include_sources": false,
  "ts_types": ["src/types/*.d.ts"],
  "sidecars": ["meta/extra-types.sidecar.json"],
  "merge_policy": "default",
  "passes": {
    "enabled": ["constant-fold", "dce"],
    "disabled": ["rename"],
    "extension": {
      "inline": { "max_inline_size": 80 }
    }
  },
  "diagnostics": {
    "strict": false,
    "strict_groups": ["missing-types"],
    "quiet_groups": ["jsdoc-mismatch"]
  }
}
```

CLI flags override config. Multiple `closurec.config.json` files up the
ancestor chain are merged: nearer files override farther ones. The
schema is published so editors can autocomplete.

## Env vars

All env vars prefixed `CLOSUREC_*`. Read at startup, lower priority than
both config file and CLI:

| Variable | Equivalent flag |
| --- | --- |
| `CLOSUREC_CONFIG` | `--config=PATH` |
| `CLOSUREC_MODE` | `--mode=...` |
| `CLOSUREC_OUT_DIR` | `--out-dir=...` |
| `CLOSUREC_TRACE` | `--no-trace` when `0` / `false` |
| `CLOSUREC_DIAGNOSTIC_FORMAT` | `--diagnostic-format=...` |
| `CLOSUREC_LOG` | `--log=info` etc. (separate from diagnostics — controls compiler tracing) |
| `CLOSUREC_NO_COLOR` | disables ANSI in diagnostics (also respected via `NO_COLOR=`) |

Precedence (highest first): CLI > env > nearest config file > parent
config files > built-in defaults.

## Stable vs unstable flag tiers

The CLI surface is split into two tiers:

- **Stable** flags (this spec): `--mode`, `--source-map`, `--check`,
  `--jsdoc`, `--strict`, `-o`, `--out-dir`, the trace/source-map family,
  exit-code table, env vars listed above. Once shipped, removed only
  with a major version bump.
- **Unstable** flags (prefix `-Z`): experimental knobs that may change
  freely between minor versions. Examples: `-Z dump-pass-cost`,
  `-Z dump-pipeline-graph`, `-Z deterministic-only`. Hidden from
  `--help` unless `-Z help` is passed.

This mirrors `rustc`'s convention. It lets us iterate aggressively
without locking in half-baked surfaces.

## Integration with the existing build tool

The repo's primary build tool is the Go implementation at
`code/programs/go/build-tool/`. `closurec` participates in the build-tool
graph as just another command-line consumer: it has a `BUILD` file
declaring dependencies on the upstream crates, and the build-tool
recompiles `closurec` whenever any dependency changes (the directed-
graph `affected_nodes()` mechanism per CLAUDE.md).

`closurec` does **not** invoke the build tool. Users invoke `closurec`
directly. The build tool's role is limited to (re)building the binary.

For Bazel-style consumers (if added later), `closurec` will gain a
JSON output mode that emits machine-readable build records. Out of
scope for MVP.

## Testing strategy

| Layer | Tests |
| --- | --- |
| `args` | Per-flag parse tests; mutually-exclusive flag pairs error cleanly. |
| `config` | Config + CLI merge with each field; ancestor-chain resolution; bad JSON exits `4`. |
| `env` | Each `CLOSUREC_*` honored when CLI absent. |
| `input/output` | Glob expansion, stdin, missing input, unwritable output. |
| `diagnostics` | Each `Severity`/`group` formats correctly; `--strict-group`, `--allow-group`, `--quiet-group`, `@suppress`, `--ignore-suppress`. |
| End-to-end | Golden test corpus: input directory + flags → expected output JS, source map, exit code, stderr. ~50 fixtures cover the primary flag combinations. |
| Exit codes | Each row in the exit-code table has a fixture. |
| `--print-ast`, `--dump-cv` | Round-trip JSON, snapshot tested. |
| Stability test | Stable flags must not change names across versions; a CI check parses `--help` against a checked-in spec. |
| Unstable flags | Allowed to change; the stability test ignores `-Z` flags. |

Coverage target per `feedback_repo_standards`: **80%+** since this is a
program (not a library); the typing of CLI surfaces makes 95% hard
without testing every clap-derive corner.

## What this spec does **not** cover

- `--bundle` mode (multi-input → single output with module resolution).
  Deferred; it's its own design problem.
- Watch mode (`--watch`). Useful for IDEs; out of scope for MVP — IDEs
  use the LSP, not `closurec`.
- Cache / incremental compilation. The first version always does a full
  compile.
- Cross-compilation targets (different ES versions). The grammar's
  `--target-es` flag is a separate concern; deferred to the version-
  selection follow-up.
- Plugin loading. CLOC06 OQ4 already settled this: no dynamic plugins.

## Open questions

1. **Stdout vs stderr separation.** Diagnostics → stderr; compiled JS →
   stdout (when no `-o`). What about `--print-ast` JSON? Default: stdout
   for the artifact, stderr for any compiler messages. This is the
   classic unix split.
2. **Color handling in CI.** Respect `NO_COLOR`, `CLICOLOR_FORCE`, and
   isatty checks. The diagnostics formatter detects all three.
3. **Long-running progress.** For large bundles, do we print pass-by-
   pass progress? Default: silent on a TTY for under 1 s, periodic
   "still working" message after that. `--quiet` and `--verbose`
   override.
4. **Error recovery in parse.** When the parser hits a syntax error,
   should `closurec --check` continue scanning later code for more
   errors, or stop? Default: stop on first error in v1; an
   `--error-recovery` flag may follow.
5. **Sidecar emission.** `closurec --emit-sidecar=PATH` could let users
   capture the merged sidecar (useful for debugging the JSDoc
   extractor). Marked as a future addition under `-Z` first.

## Closing: where the foundational specs end

CLOC08 is the last foundational spec. With CLOC01-08 merged, the
implementation queue per CLOC01's staged plan is fully unblocked:

- **Stage 1** — JS frontend fix-up (stub-grammar retirement, `javascript-
  tokens`, `javascript-ast`, CV plumbing, default `EsVersion::Es2025`).
- **Stage 2** — `type-sidecar`, JSDoc grammar + crates.
- **Stage 3** — `closure-typechecker` + the `closure-pass-*` family.
- **Stage 4** — `closure-emitter`, `closure-source-map`, `closure-cli`.

Each PR in those stages will reference the spec it implements.
