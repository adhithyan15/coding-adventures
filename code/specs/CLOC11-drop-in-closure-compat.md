# CLOC11 — Drop-in Closure Compiler compatibility

> Status: spec. Implementation lands across many follow-up PRs
> per the slicing in §6.

## 1. Purpose

Make `closurec` a **true drop-in replacement** for Google's Java
Closure Compiler — same flag surface, same default behavior, same
output shape, same exit codes. A user with an existing build that
runs:

```bash
java -jar closure-compiler.jar \
  --js src/**/*.js \
  --js_output_file dist/app.min.js \
  --compilation_level ADVANCED \
  --language_in ECMASCRIPT_2020 \
  --language_out ECMASCRIPT5_STRICT \
  --create_source_map dist/app.min.js.map \
  --define DEBUG=false \
  --externs externs/global.js \
  --jscomp_off checkTypes \
  --warning_level VERBOSE
```

should be able to substitute `closurec` for `java -jar closure-compiler.jar`
and get a working build with the same output, the same diagnostics,
and the same source map. No script changes. No "this flag isn't
supported yet". No silent no-ops.

That is the bar. Every spec in CLOC11.* exists to drag us closer to
it.

## 2. Where we are today

The good news is that **the CLI surface is already complete**.
`code/programs/rust/closurec/cli.spec.json` declares 100 flags,
covering every option `CommandLineRunner.java` exposes. `closurec
--help` is byte-for-byte recognisable to a Closure user.

```text
$ closurec --js a.js --compilation_level ADVANCED --define DEBUG=false
closurec v0.1.0 - identity pipeline
```

The flags *parse*. cli-builder's "did you mean?" suggestions work.
Unknown flags get rejected. The exit codes are right.

The bad news is that **nothing past parsing is wired**:

```text
$ closurec --js a.js --js_output_file out.js --compilation_level ADVANCED
closurec v0.1.0 - identity pipeline    ← Always. No matter the flags.
```

`closurec`'s `parse_and_run` validates argv, prints
`"closurec v0.1.0 - identity pipeline\n"`, and exits. No file
reads, no compilation, no output, no source map. The flag values
are dropped on the floor.

The gap, then, is not the surface — it's the *body*. Every flag
needs:

1. **Semantics** — what the real CC does when this flag is set.
2. **Wiring** — turn the parsed flag value into compiler state
   (`PassPipeline` config, `Sidecar` config, `Emitter` config,
   `SourceMap` config, …).
3. **Tests** — drive `parse_and_run` with the flag and assert the
   observed output matches `java -jar closure-compiler.jar` for
   the same input.

This is the gap. The rest of this spec enumerates it.

## 3. Strategy

### 3.1 Differential testing is the bar

For every behavioral flag we wire, we add a **differential test**:
a small JS input plus an exact `closurec --flag=...` invocation,
and we assert `closurec`'s output is byte-equal (or "behaviorally
equal" per §3.3) to `java -jar closure-compiler.jar --flag=...`
output for the same input.

Diff tests live in `code/programs/rust/closurec/tests/diff/`,
each as a directory:

```text
tests/diff/define-substitution/
├── input.js
├── flags.txt           # one flag per line
├── expected.stdout     # captured by running real CC once, checked in
└── expected.exitcode   # if not 0
```

The runner compares `closurec`'s output to `expected.stdout`. The
fixtures are captured once, by hand, from a real CC binary; CI
doesn't need Java installed. A separate **fixture-refresh script**
(opt-in, requires Java) re-captures all fixtures so we can verify
nothing drifted on real-CC upgrades.

### 3.2 Why differential, not "spec-driven"

Closure's own docs are notoriously incomplete. `--jscomp_off` is
documented as "turn off the named warning class," but which named
warning classes exist isn't listed anywhere except
`DiagnosticGroups.java`. `--define` accepts implicit-true bools
that aren't in the docs. `--compilation_level ADVANCED` enables
exactly 47 passes in a specific order — that order isn't in the
docs either. The only ground truth is the actual binary. Diffing
against it is therefore the only credible way to claim
compatibility.

### 3.3 "Behavioral equality"

Byte-exact output is the goal but it's too strict in some cases.
Our renaming will produce different names than CC's — we just need
the *renamed program* to be equivalent. Source map paths differ.
Bundle ordering for `--dependency_mode PRUNE` is stable but
different. We define an **equivalence relation**:

- **Type A — byte-equal.** `--print_tree`, `--help`, `--version`,
  identity transforms (e.g. `--compilation_level WHITESPACE_ONLY`).
- **Type B — α-equivalent.** Renamed identifiers may differ but
  the program semantics must match. Verified by running the
  output through V8 (or `node`, since `v8c` isn't shipping yet)
  and comparing observable behavior.
- **Type C — structurally equal.** JSON-shaped outputs (source
  maps, manifests, chunk-dependency JSON) — compare parsed JSON,
  not byte streams.

Each diff test declares its type up front.

### 3.4 Buckets, not a monolithic implementation

Wiring 100 flags as one PR is infeasible. We slice along
**user-visible compiler features**, each its own CLOC11.*
sub-spec and its own (small) PR. The slicing is in §6.

## 4. Flag inventory

Every flag declared in `cli.spec.json`, grouped by which
compiler feature it controls. Status legend:

- ✅ **Wired** — value flows into compiler state, has diff test.
- 🟡 **Partial** — wired but with caveats (documented per row).
- ❌ **Unwired** — declared, parsed, dropped. (Most current rows.)
- ⏸ **Deferred-v2** — intentionally not implementing v1; usually
  because the underlying compiler feature isn't built yet (e.g.
  Polymer pass).
- 🚫 **Won't-implement** — hidden in CC, internal-only, or
  meaningless in our context (e.g. `--filename_to_save_to`,
  which depends on CC's serialisable AST format we don't share).

### 4.1 I/O — required to compile anything

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--js`                            | ❌     | Resolve globs (incl. `!` excludes); read each file. |
| `--js_output_file`                | ❌     | If empty, write to stdout. |
| `--jszip`                         | ❌     | ZIP archive of .js entries; iterate. |
| `--externs`                       | ❌     | Externs files seed the type sidecar with declared globals. |
| `--charset`                       | ❌     | Default UTF-8 in / US-ASCII out per CC. |
| `--json_streams`                  | ❌     | Stdin/stdout as a JSON array of `{ src, path, ... }`. |

### 4.2 Compilation level

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--compilation_level`             | ❌     | `BUNDLE` / `WHITESPACE_ONLY` / `SIMPLE` / `ADVANCED` / `TRANSPILE_ONLY`. Each enables a known pass set. |
| `--debug`                         | ❌     | Renames produce long readable names. Counter-pass to `--compilation_level ADVANCED`'s default short renames. |
| `--renaming`                      | ❌     | Disable variable renaming. Errors with `ADVANCED`. |
| `--assume_function_wrapper`       | ❌     | Allow globals-leak optimisation; useful only when output is wrapped. |
| `--use_types_for_optimization`    | ❌     | Type-aware optimisations on/off. Off implies no `disambiguateProperties`. |

### 4.3 Language level

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--language_in`                   | 🟡     | Parser accepts ES2025 always; this flag should narrow what's *accepted* (e.g. error on optional chaining when set to ES2018). |
| `--language_out`                  | ❌     | Drives transpilation level — what features the emitter must lower for compatibility. |
| `--strict_mode_input`             | ❌     | Treat inputs as strict-mode even without `"use strict"` directive. |
| `--emit_use_strict`               | ❌     | Prepend `"use strict";` to output. |
| `--browser_featureset_year`       | ❌     | Implies `language_out` based on browser support year (2012 onward). |

### 4.4 Output formatting

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--output_wrapper`                | ❌     | Interpolate output into a string at the `%output%` marker. |
| `--output_wrapper_file`           | ❌     | Same, but read the wrapper from a file. |
| `--isolation_mode`                | ❌     | `NONE` / `IIFE` — wrap the whole output in `(function(){…})()`. |
| `--formatting`                    | ❌     | Repeatable: `PRETTY_PRINT`, `PRINT_INPUT_DELIMITER`, `SINGLE_QUOTES`. |
| `--rename_variable_prefix`        | ❌     | All renamed vars get this prefix. |
| `--rename_prefix_namespace`       | ❌     | Globals stored on a named object instead of polluting the global scope. |

### 4.5 Source maps

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--create_source_map`             | ❌     | Path template — `%outname%` token substituted. |
| `--source_map_format`             | ❌     | `V3` / `DEFAULT` — V3 is the spec; DEFAULT means V3 today. |
| `--source_map_include_content`    | ❌     | Embed `sourcesContent` in the map. |
| `--source_map_location_mapping`   | ❌     | Filesystem path → web-server path rewrites. |
| `--source_map_input`              | ❌     | Pre-existing input source maps (chained from upstream transpilation). |
| `--apply_input_source_maps`       | ❌     | Apply input source maps to the output map (chained source-map composition). |
| `--parse_inline_source_maps`      | ❌     | Honor `//# sourceMappingURL=data:...` comments. |

### 4.6 Diagnostics

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--warning_level`                 | ❌     | `QUIET` / `DEFAULT` / `VERBOSE` — affects default `jscomp_*` levels. |
| `--jscomp_error`                  | ❌     | Promote named diagnostic group to error. |
| `--jscomp_warning`                | ❌     | Demote named diagnostic group to warning. |
| `--jscomp_off`                    | ❌     | Silence named diagnostic group. |
| `--hide_warnings_for`             | ❌     | Substring match on file paths; hide warnings from matching files. |
| `--warnings_allowlist_file`       | ❌     | File listing warning sites to suppress (file:line:type). |
| `--error_format`                  | ❌     | Only `STANDARD` supported by CC today; future could add JSON/SARIF. |
| `--summary_detail_level`          | ❌     | 0-3, controls end-of-run summary verbosity. |
| `--continue_after_errors`         | ❌     | Don't bail on first error; collect and report. |
| `--extra_annotation_name`         | ❌     | Add tag names to JSDoc parser's allowlist (suppresses "unknown @tag" warnings). |
| `--third_party`                   | ❌     | Skip Closure style-convention enforcement (e.g. permitting `var` use). |

### 4.7 `@define` substitution

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--define` / `-D`                 | ❌     | `--define NAME=val` overrides `@define`-annotated constants. Implicit `true` if `=val` omitted. Values are JS literals (number/string/bool/null). |

### 4.8 Dependencies & modules

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--dependency_mode`               | ❌     | `NONE` / `SORT_ONLY` / `PRUNE` / `PRUNE_LEGACY`. Drives Closure's dependency graph. |
| `--entry_point`                   | ❌     | Root(s) for `PRUNE` mode. File path or `goog.module(...)`-style namespace. |
| `--process_closure_primitives`    | ❌     | Recognise `goog.provide` / `goog.require` / `goog.module` / `goog.scope`. |
| `--process_common_js_modules`     | ❌     | Treat inputs as CommonJS; rewrite `require`/`module.exports`. |
| `--js_module_root`                | ❌     | Path prefixes to strip when computing module IDs. |
| `--module_resolution`             | ❌     | `BROWSER` / `NODE` / `WEBPACK` — algorithm for resolving relative imports. |
| `--browser_resolver_prefix_replacements` | ❌ | Prefix substitution table for `BROWSER` resolver. |
| `--package_json_entry_names`      | ❌     | Field order for `package.json` "main"/"module"/"browser" lookup. |

### 4.9 Chunks (multi-output bundling)

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--chunk`                         | ❌     | `<name>:<numfiles>[:<dep>[,<dep>]*]` — declares a chunk. Compiler bundles files into chunks following the declared dep graph. |
| `--chunk_wrapper`                 | ❌     | Per-chunk `--output_wrapper` analogue: `<name>:<wrapper>`. |
| `--chunk_output_path_prefix`      | ❌     | Directory prefix for chunk output files. |
| `--chunk_output_type`             | ❌     | `GLOBAL_NAMESPACE` (default) vs `ES_MODULES` (emit `import`/`export`). |
| `--output_chunk_dependencies`     | ❌     | Emit a JSON file describing chunk → chunk edges. |
| `--output_manifest`               | ❌     | Emit a text file listing every input file (in compilation order). |

### 4.10 Special-case passes

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--angular_pass`                  | ⏸     | Generates `$inject` arrays for `@ngInject` functions. AngularJS-specific. Defer until we have a concrete user. |
| `--polymer_version`               | ⏸     | Polymer-specific rewrites. Defer. |
| `--chrome_pass`                   | ⏸     | `cr.*` rewrites for Chromium-internal JS. Defer. |
| `--j2cl_pass`                     | ⏸     | J2CL (Java-to-Closure) output cleanup. Defer. |
| `--remove_j2cl_asserts`           | ⏸     | Strip J2CL `Asserts.*` calls. Defer with `--j2cl_pass`. |
| `--dart_pass`                     | ⏸     | Not present in cli.spec.json — CC has removed it. |

### 4.11 Polyfills

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--rewrite_polyfills`             | ❌     | Inject ES6+ polyfills as needed (Promise, Array.from, …). |
| `--isolate_polyfills`             | ❌     | Hide polyfills from the global scope. |
| `--inject_libraries`              | ❌     | Allow injection at all. `--rewrite_polyfills` implies on. |
| `--force_inject_library`          | ❌     | Force-inject named libraries even if unreferenced. |

### 4.12 Renaming reports

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--variable_renaming_report`      | ❌     | Path to JSON dump of `original → renamed` map for variables. |
| `--property_renaming_report`      | ❌     | Same for property names. |
| `--create_renaming_reports`       | ❌     | Generate both reports (boolean toggle for the above). |
| `--instrument_mapping_report`     | ❌     | Production-instrumentation map output. |

### 4.13 Exports & generation

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--generate_exports`              | ❌     | Honor `@export` JSDoc tags. |
| `--export_local_property_definitions` | ❌ | Honor `@export` on local properties. |

### 4.14 Conformance

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--conformance_configs`           | ❌     | Repeatable: paths to JS Conformance proto-text configs. Each declares forbidden patterns. |

### 4.15 Instrumentation

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--instrument_for_coverage_option` | ❌    | `NONE` / `LINE` / `BRANCH` / `PRODUCTION`. Inject coverage probes. |
| `--production_instrumentation_array_name` | ❌ | Name of the global array probes write into in `PRODUCTION` mode. |
| `--tracer_mode`                   | ❌     | `OFF` / `ALL` / `RAW_SIZE` / `AST_SIZE` / `TIMING_ONLY`. Pass-by-pass timing output. |

### 4.16 Special modes

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--checks_only`                   | ❌     | Run typecheck + lint, skip code gen. |
| `--print_tree`                    | ❌     | Print AST and exit. |
| `--print_tree_json`               | ❌     | Same, as JSON. |
| `--print_ast`                     | ❌     | Dot-graph output of internal AST. |
| `--print_source_after_each_pass`  | ❌     | After each pass, re-emit JS — diagnostic for pass authors. |
| `--help_markdown`                 | ❌     | Markdown-formatted `--help`. |

### 4.17 Translation

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--translations_file`             | ❌     | XTB-format translated message bundle. |
| `--translations_project`          | ❌     | Scope translations to a project. |

### 4.18 Internal/hidden CC flags we accept-but-ignore

| Flag                              | Status | Notes |
|-----------------------------------|--------|-------|
| `--jscomp_dev_mode`               | 🚫     | CC dev-only; accept and no-op. |
| `--logging_level`                 | 🚫     | Maps to JUL levels in CC; we use diagnostics, no-op. |
| `--filename_to_save_to`           | 🚫     | Serializes CC's internal AST. Our IR differs; would mislead users. Always reject with a clear message. |
| `--filename_to_restore_from`      | 🚫     | Same. Reject. |
| `--segment_of_compilation_to_run` | 🚫     | Tied to save/restore. Reject. |
| `--num_parallel_threads`          | 🚫     | We schedule passes ourselves; accept and ignore. |
| `--incremental_check_mode`        | ⏸     | Generates `.i.js` files. Defer. |
| `--typed_ast_output_file_internal` | 🚫    | Internal CC format; reject. |
| `--preserve_type_annotations`     | ⏸     | Only meaningful when we emit `@type` JSDoc in the output. Defer. |
| `--flagfile`                      | ❌     | Expand `@flagfile.txt` recursively into argv. Generally useful, wire early. |
| `--env`                           | ❌     | Selects which built-in externs to load. |

### 4.19 Counts

```text
Wired (✅):           0 / 100
Partial (🟡):         1 / 100  (--language_in narrowing not done)
Unwired (❌):        76 / 100   ← the work
Deferred-v2 (⏸):     14 / 100
Won't-implement (🚫): 9 / 100
```

The 76-flag "unwired" bucket is the heart of CLOC11.

## 5. Architecture for wiring flags

Currently `parse_and_run` does nothing with the parsed flags.
The wiring layer we need:

```rust
// In closurec::wire:
pub struct CompilerConfig {
    pub inputs: Vec<InputSource>,
    pub output: OutputTarget,
    pub externs: Vec<PathBuf>,
    pub charset: Charset,
    pub compilation_level: CompilationLevel,
    pub language_in: LanguageVersion,
    pub language_out: LanguageVersion,
    pub formatting: FormattingOptions,
    pub source_map: Option<SourceMapConfig>,
    pub diagnostics: DiagnosticsConfig,
    pub defines: BTreeMap<String, DefineValue>,
    pub dependencies: DependencyConfig,
    pub chunks: Vec<ChunkSpec>,
    pub polyfills: PolyfillConfig,
    pub renaming_reports: RenamingReportConfig,
    pub conformance: Vec<PathBuf>,
    pub instrumentation: InstrumentationConfig,
    pub special_modes: SpecialModes,
    // …
}

pub fn config_from_parsed(parsed: &cli_builder::ParseResult)
    -> Result<CompilerConfig, ConfigError>;

pub fn run_compiler(config: &CompilerConfig)
    -> Result<CompilerOutput, CompilerError>;
```

Each `*Config` sub-struct lives in its own module
(`closurec::wire::source_map`, etc.) and owns the per-feature
mapping from cli flag to compiler behavior. Adding a flag means
adding to its config struct, mapping it in
`config_from_parsed`, and consuming it in `run_compiler`.

Once the wiring layer exists, every PR in §6 just adds rows to
existing structs + tests. No new architectural pieces per flag.

## 6. PR slicing

The flag inventory is large; the PR list has to be small per
unit to land cleanly. Each item below is one PR. Ordering is
by user-value (top of every Closure build) × wiring-prereq.

### Track 1 — End-to-end identity build (foundation)

The goal of Track 1 is to make `closurec --js foo.js
--js_output_file out.js` actually read, parse, emit, and write
out — even if every optimisation is off. Once this lands, every
later track plugs into an already-working pipeline.

- **CLOC11.01** — `CompilerConfig` skeleton + `config_from_parsed`
  scaffold. No semantics yet; just plumb every flag into the
  config struct. Wire one round-trip test: `--js a.js
  --js_output_file b.js` → identity copy.
- **CLOC11.02** — `--js` glob resolution + file reads. Support
  `!` exclude. Diff test against CC for the resulting file list
  via `--output_manifest`.
- **CLOC11.03** — `--js_output_file` writing (including stdout
  fallback when empty). Diff: byte-equal for the identity
  pipeline.
- **CLOC11.04** — `--charset`. UTF-8 in / US-ASCII out default;
  alternate charsets via `encoding_rs` (zero-dep alternative
  TBD).
- **CLOC11.05** — `--flagfile`. Recursive `@file.txt` expansion
  into argv before `cli-builder` sees it.

### Track 2 — Compilation levels

The single biggest source of "this flag does nothing":
`--compilation_level` controls the pass set, and right now no
passes run no matter what's set. Each level is its own PR
because each is a specific set of canonical passes from CLOC06
plus level-specific configuration.

- **CLOC11.06** — `--compilation_level WHITESPACE_ONLY`. Run only
  whitespace removal; closure-emitter emits compact JS. Diff:
  byte-equal-ish (we may differ in newlines but token sequence
  matches).
- **CLOC11.07** — `--compilation_level SIMPLE`. Default. Run
  constant-fold, fold-control-flow, dce, local-variable
  renaming. Diff: α-equivalent (renames may differ).
- **CLOC11.08** — `--compilation_level ADVANCED`. Plus
  global-rename, treeshake, collapse-properties, inline,
  remove-unused-vars, type-based optimisations. Behaviorally
  equivalent on a battery of fixtures.
- **CLOC11.09** — `--compilation_level BUNDLE`. Concatenate
  inputs with minimal munging; for development.
- **CLOC11.10** — `--compilation_level TRANSPILE_ONLY`. Drive
  emitter for `language_out` lowering only.
- **CLOC11.11** — `--debug`. Long readable renames; pass
  `debug: true` into rename pass config.
- **CLOC11.12** — `--renaming false`. Force rename pass off;
  error in combination with ADVANCED.
- **CLOC11.13** — `--assume_function_wrapper`. Allow leaky
  optimisations.

### Track 3 — Language level

Closure's lingua franca is "input in modern ES, output in
older ES." `--language_in/--language_out` define the source
and target.

- **CLOC11.14** — `--language_in` enforcement. Already
  parse-tolerant; here, after parsing, walk the AST and
  diagnose features above the declared level (e.g. emit a
  warning for optional chaining when `--language_in
  ECMASCRIPT_2018`).
- **CLOC11.15** — `--language_out` baseline. Emitter
  drops/lowers features above the target (e.g. arrow → function,
  `const` → `var` when targeting ES5).
- **CLOC11.16** — `--browser_featureset_year`. Map year → level
  per CC's published table; override `language_out`.
- **CLOC11.17** — `--strict_mode_input`.
- **CLOC11.18** — `--emit_use_strict`.

### Track 4 — Defines

The most-used non-input flag in real builds.

- **CLOC11.19** — `--define / -D`. Constants get substituted
  before optimisation. Recognise `@define`-tagged consts in
  the AST; substitute; fold. Diff against CC for a tiny
  fixture: `goog.DEBUG = false` → DCE picks up the dead branch.

### Track 5 — Diagnostics

Closure's diagnostic ergonomics are a huge feature. Cover them
in two passes: the level-style flags, then the named-group
flags.

- **CLOC11.20** — `--warning_level`. Default `jscomp_*` levels
  per warning class.
- **CLOC11.21** — Diagnostic group registry. Closure's
  `DiagnosticGroups` enumerates ~80 named groups; mirror them.
  No-op v1; needed for the next 4 PRs.
- **CLOC11.22** — `--jscomp_error / --jscomp_warning / --jscomp_off`.
  Per-group severity override.
- **CLOC11.23** — `--hide_warnings_for`. Substring match on path.
- **CLOC11.24** — `--warnings_allowlist_file`.
- **CLOC11.25** — `--error_format STANDARD`. Stable line/column
  reporting format.
- **CLOC11.26** — `--summary_detail_level` 0-3.
- **CLOC11.27** — `--continue_after_errors`.
- **CLOC11.28** — `--third_party`. Skip Closure style
  enforcement.
- **CLOC11.29** — `--extra_annotation_name`. JSDoc tag allowlist.

### Track 6 — Output formatting

- **CLOC11.30** — `--output_wrapper` / `--output_wrapper_file`
  with `%output%` token. Plus `%n%` newline expansion.
- **CLOC11.31** — `--isolation_mode IIFE`. Wrap output in
  `(function(){…}).call(this);` per CC.
- **CLOC11.32** — `--formatting PRETTY_PRINT`.
- **CLOC11.33** — `--formatting PRINT_INPUT_DELIMITER`.
- **CLOC11.34** — `--formatting SINGLE_QUOTES`.
- **CLOC11.35** — `--rename_variable_prefix`.
- **CLOC11.36** — `--rename_prefix_namespace`.

### Track 7 — Source maps

`closure-source-map` already exists. Wire it up.

- **CLOC11.37** — `--create_source_map`. With `%outname%`
  substitution.
- **CLOC11.38** — `--source_map_include_content`.
- **CLOC11.39** — `--source_map_location_mapping`. Path rewrites.
- **CLOC11.40** — `--source_map_input` + `--apply_input_source_maps`.
  Chained source-map composition.
- **CLOC11.41** — `--parse_inline_source_maps`.
- **CLOC11.42** — `--source_map_format`. Only V3 today; flag is
  for forward-compat.

### Track 8 — Renaming reports

- **CLOC11.43** — `--variable_renaming_report`.
- **CLOC11.44** — `--property_renaming_report`.
- **CLOC11.45** — `--create_renaming_reports` (toggle for both).

### Track 9 — Externs + types

- **CLOC11.46** — `--externs`. Read files, parse JSDoc, seed
  Sidecar with declared globals.
- **CLOC11.47** — `--env BROWSER` / `--env CUSTOM`. Built-in
  externs (window, document, navigator, …) for `BROWSER`.
- **CLOC11.48** — `--use_types_for_optimization`. Gate
  type-aware passes on this flag's value.

### Track 10 — Generate exports

- **CLOC11.49** — `--generate_exports`. Honor `@export` tags.
- **CLOC11.50** — `--export_local_property_definitions`.

### Track 11 — Special modes (low-risk wins)

- **CLOC11.51** — `--checks_only`. Skip emit phase.
- **CLOC11.52** — `--print_tree`. Walk the AST and pretty-print.
- **CLOC11.53** — `--print_tree_json`. Same, as JSON via
  serde_json.
- **CLOC11.54** — `--print_ast`. Dot-graph; reuses
  print-tree-json data.
- **CLOC11.55** — `--print_source_after_each_pass`. Pass-author
  diagnostic; runs emit after every pass and prints to stderr.
- **CLOC11.56** — `--help_markdown`. Markdown-formatted help.
- **CLOC11.57** — `--tracer_mode`. Already partly emerged via
  `PassStats`; add wall-clock timing per pass.

### Track 12 — Dependencies & modules

Closure's dependency story is intricate. We pay it off slowly.

- **CLOC11.58** — `--dependency_mode NONE | SORT_ONLY`. Simple
  topo-sort.
- **CLOC11.59** — `--process_closure_primitives`. Recognise
  `goog.provide` / `goog.require` / `goog.module`. Build a
  goog-namespace graph.
- **CLOC11.60** — `--dependency_mode PRUNE | PRUNE_LEGACY`.
  Drop unreferenced files.
- **CLOC11.61** — `--entry_point`. Root the prune.
- **CLOC11.62** — `--process_common_js_modules`. CJS rewrite.
- **CLOC11.63** — `--js_module_root`.
- **CLOC11.64** — `--module_resolution NODE`.
- **CLOC11.65** — `--module_resolution BROWSER` +
  `--browser_resolver_prefix_replacements`.
- **CLOC11.66** — `--module_resolution WEBPACK`.
- **CLOC11.67** — `--package_json_entry_names`.

### Track 13 — Chunks

- **CLOC11.68** — `--chunk` spec parsing + chunk graph
  construction.
- **CLOC11.69** — `--chunk_output_path_prefix` writing.
- **CLOC11.70** — `--chunk_wrapper`.
- **CLOC11.71** — `--chunk_output_type GLOBAL_NAMESPACE`.
- **CLOC11.72** — `--chunk_output_type ES_MODULES`.
- **CLOC11.73** — `--output_chunk_dependencies` JSON output.
- **CLOC11.74** — `--output_manifest`.

### Track 14 — Polyfills

- **CLOC11.75** — Polyfill catalog (which features map to
  which runtime libraries — matches CC's `Es6ToEs3Util` table).
- **CLOC11.76** — `--rewrite_polyfills`. Insert polyfill calls.
- **CLOC11.77** — `--inject_libraries`. Gate.
- **CLOC11.78** — `--force_inject_library`.
- **CLOC11.79** — `--isolate_polyfills`.

### Track 15 — Conformance

- **CLOC11.80** — Conformance proto-text parser (subset that
  matches CC's `requirement.proto`).
- **CLOC11.81** — `--conformance_configs`. Apply requirements
  to the AST; emit per-requirement diagnostics.

### Track 16 — Instrumentation

- **CLOC11.82** — `--instrument_for_coverage_option LINE`.
- **CLOC11.83** — `--instrument_for_coverage_option BRANCH`.
- **CLOC11.84** — `--instrument_for_coverage_option PRODUCTION` +
  `--production_instrumentation_array_name` +
  `--instrument_mapping_report`.

### Track 17 — Translations

- **CLOC11.85** — XTB parser.
- **CLOC11.86** — `--translations_file` substitution at
  `goog.getMsg` sites.
- **CLOC11.87** — `--translations_project` scoping.

### Track 18 — JSON streams

- **CLOC11.88** — `--json_streams IN`.
- **CLOC11.89** — `--json_streams OUT`.
- **CLOC11.90** — `--json_streams BOTH`.

### Track 19 — Special passes (deferred to "real users")

These come as users actually ask for them.

- **CLOC11.91** — `--angular_pass`.
- **CLOC11.92** — `--polymer_version 1`.
- **CLOC11.93** — `--polymer_version 2`.
- **CLOC11.94** — `--chrome_pass`.
- **CLOC11.95** — `--j2cl_pass` + `--remove_j2cl_asserts`.
- **CLOC11.96** — `--incremental_check_mode`.
- **CLOC11.97** — `--preserve_type_annotations`.

### Track 20 — Reject-with-friendly-message

- **CLOC11.98** — One PR rejecting the won't-implement flags
  (`--filename_to_save_to` et al.) with a clear "not supported
  by closurec" message and an exit code distinct from
  invalid-arg.

### Tracks at a glance

| Track | Theme                       | PRs | Cumulative |
|-------|-----------------------------|-----|------------|
| 1     | End-to-end identity build   | 5   | 5          |
| 2     | Compilation levels          | 8   | 13         |
| 3     | Language level              | 5   | 18         |
| 4     | Defines                     | 1   | 19         |
| 5     | Diagnostics                 | 10  | 29         |
| 6     | Output formatting           | 7   | 36         |
| 7     | Source maps                 | 6   | 42         |
| 8     | Renaming reports            | 3   | 45         |
| 9     | Externs + types             | 3   | 48         |
| 10    | Generate exports            | 2   | 50         |
| 11    | Special modes               | 7   | 57         |
| 12    | Dependencies & modules      | 10  | 67         |
| 13    | Chunks                      | 7   | 74         |
| 14    | Polyfills                   | 5   | 79         |
| 15    | Conformance                 | 2   | 81         |
| 16    | Instrumentation             | 3   | 84         |
| 17    | Translations                | 3   | 87         |
| 18    | JSON streams                | 3   | 90         |
| 19    | Special passes              | 7   | 97         |
| 20    | Reject won't-implement      | 1   | 98         |

98 PRs sounds enormous, but each is small: add a config field,
wire it in `config_from_parsed`, consume in `run_compiler`, add
1-3 diff tests. Most PRs land in 20-40 minutes of focused work
plus CI. We have parallel-runnable tracks (5 and 6 are
independent of 2 and 3, for example), so multi-track autonomous
chains keep throughput high.

## 7. Parallel-runnable tracks

Tracks below this list block on others; tracks above run in
parallel from day one:

- **Track 1** is the only sequential prereq for everything else
  (you need to read inputs to do anything). Once 11.01-11.03 land,
  the world opens.
- After 11.03: Tracks **2, 3, 4, 6, 11, 18** are mutually
  independent — each operates on a different config slice.
- After 11.07 (SIMPLE compilation): Tracks **5, 7, 8, 9, 10**
  can each begin (they each need a real pass pipeline running).
- After 11.21 (Diagnostic group registry): Track **15** unblocks.
- Track **12** is mostly self-contained but needs externs (9)
  for its `--process_closure_primitives` semantics.
- Track **13** blocks on 12 (chunks reference modules).
- Track **14** blocks on Track 3 (polyfills depend on
  `language_out`).

A reasonable target is **5 parallel chains** at steady state:

| Chain | First few PRs                                  |
|-------|-------------------------------------------------|
| A     | 11.01 → 11.02 → 11.03 → 11.06 → 11.07 → 11.08  |
| B     | 11.04, 11.05, 11.17, 11.18, 11.19              |
| C     | 11.14 → 11.15 → 11.16 (language)               |
| D     | 11.30 → 11.31 → 11.32-34 → 11.35-36 (formatting) |
| E     | 11.21 → 11.22 → 11.23 → 11.24 → 11.25 (diags) |

## 8. What this PR locks down

This is the umbrella spec. It commits us to:

- The differential-testing methodology (§3.1, §3.3).
- The `CompilerConfig` wiring architecture (§5).
- The PR slicing (§6) — 98 numbered slices, each ≤ 1 PR.
- Tracking each slice's status in this doc — when a PR lands,
  its row in §4 flips from ❌ to ✅, and the §4.19 counter
  ticks up. The doc is therefore the project's drop-in-readiness
  dashboard.

Reviewers should challenge:

- Is the §3.3 equivalence trichotomy (byte / α / structural)
  sound, or do we need a fourth bucket?
- Is the PR slicing in §6 honest? (Each row claims "this is one
  PR" — is any row obviously two?)
- Is the won't-implement list (§4.18) defensible, or should we
  bite the bullet on save/restore (CC's `--filename_to_save_to`
  family) so we're truly drop-in?
- Should we drop a track entirely (e.g. translations) if no v1
  user needs it, rather than scope-creeping?

Once this lands, **CLOC11.01** starts immediately as the first
implementation PR in the new autonomous chain.

## 9. Non-goals

- **No new compiler features.** CLOC11 is about wiring the
  features we've already implemented (the canonical passes from
  CLOC06, the emitter from CLOC07, the source map from CLOC07
  Phase 2) into the flag surface. Genuinely new functionality
  (Polymer pass, J2CL, etc.) is explicitly deferred — see §4.10.
- **No CLI redesign.** `cli.spec.json` is final.
- **No bytecode / runtime work.** That's the V8C series, paused
  in favor of this. We resume V8 when `closurec` actually
  drop-in-replaces `closure-compiler.jar` for at least one real
  user build.
- **No Java JAR shipping.** We don't aim to *be* CC, we aim to
  *behave* like CC's command-line surface. Internals diverge.
