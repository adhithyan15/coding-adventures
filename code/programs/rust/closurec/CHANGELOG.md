# Changelog

All notable changes to the `coding-adventures-closurec` binary will be documented in this file.

## [0.8.0] - 2026-05-25

### Added — CLOC11.31: `--isolation_mode IIFE` wrapping

Companion to CLOC11.30 in Track 6. When `--isolation_mode IIFE` is passed, the compiled output is wrapped in `(function(){…}).call(this);` — matching CC's `CompilerOptions` IIFE behavior. `--isolation_mode NONE` (the default) is unchanged.

- **New `wrapper::apply_iife_wrap(compiled) -> String`.** Emits `(function(){<compiled>}).call(this);` — using `.call(this)` rather than the simpler `()` form to preserve outer `this` binding the same way CC has since the option was introduced. Pinned by a test (`iife_wrap_uses_call_this_not_bare_invocation`) so a future "simplification" can't silently regress.
- **Pipeline ordering.** IIFE wrapping runs *after* `--output_wrapper` template substitution but *before* writing to disk/stdout. So a `--output_wrapper '// banner%n%%output%'` + `--isolation_mode IIFE` produces `(function(){// banner\n<compiled>}).call(this);` — banner sits *inside* the IIFE, matching CC's layered behavior.
- **Diff fixture** `tests/diff/isolation-iife/` per CLOC11 §3.
- **New integration test** `tests/diff_isolation_iife.rs` drives the built binary against the fixture.
- **4 new unit tests** in `wrapper::tests`: basic wrap, empty body, content-preservation, `.call(this)` form is pinned.

### Pipeline matrix (cumulative across CLOC11)

After CLOC11.31 lands, `run_compiler` does:

1. Per-input `transform_source`:
   - 1a. `--compilation_level` (CLOC11.06: WHITESPACE_ONLY active)
   - 1b. `--define / -D` substitution (CLOC11.19)
2. Concatenate transformed inputs (`combined`)
3. `--output_wrapper` template substitution (CLOC11.30)
4. **`--isolation_mode IIFE` wrap (CLOC11.31, new)**
5. Write to `--js_output_file` (auto-create parents) or stdout

### Behavior changes (user-visible)

- `closurec --isolation_mode IIFE --js app.js` now actually wraps. Previously the flag was parsed but ignored.

### Tests

113 unit + 7 integration tests passing. Clippy clean.

### Version

Bumps closurec 0.7.0 → 0.8.0 (Cargo.toml + cli.spec.json kept in sync).

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.7.0] - 2026-05-25

### Added — CLOC11.30: `--output_wrapper` template substitution

Third behavioral slice of [CLOC11], landing the first piece of Track 6 (output formatting). `closurec` now honors `--output_wrapper <template>` (and the companion `--output_wrapper_file <path>`) end-to-end.

- **New `wrapper` module.** Single forward-scan template substituter recognizing two placeholders per CC's documented behavior:
  - `%output%` → the compiled JS (the result of all prior pipeline stages: transform_source + defines + concatenation).
  - `%n%` → a literal newline character.
  
  Unrecognized `%name%` placeholders (e.g. `%foo%`) pass through verbatim — CC's behavior. Lone `%` signs without a closing partner before a non-name character (e.g. `50% off`) also pass through unchanged.

- **`--output_wrapper_file` overrides `--output_wrapper`** when both are supplied. The file's contents become the wrapper template. Matches CC's documented behavior ("loads the specified file and passes its contents to `--output_wrapper`").

- **Pipeline ordering**: applied in `run_compiler` *after* the per-input transform and concatenation, *before* writing to disk or stdout. So the wrapper sees the final compiled JS — including everything WHITESPACE_ONLY, defines, and any future passes contributed.

- **Fast-path passthrough.** When neither `--output_wrapper` nor `--output_wrapper_file` is set, `apply_output_wrapper` returns the compiled string unchanged without allocating. The common case stays cheap.

- **New `CompilerError::Wrapper(WrapperError)`** variant. The single failure path today is `--output_wrapper_file` pointing at a non-readable path; we surface a typed `WrapperFileReadError` with the path, `io::ErrorKind`, and message.

- **Diff fixture** `tests/diff/output-wrapper/` per CLOC11 §3: a tiny input file, a flags file invoking `--output_wrapper '(function(){%output%})();'`, and the expected wrapped output.

- **New integration test** `tests/diff_output_wrapper.rs` drives the built binary against the fixture and asserts byte-equal stdout.

- **14 new unit tests in `wrapper::tests`** covering: no-wrapper passthrough, `%output%` substitution, `%n%` newline expansion, unrecognized placeholder passthrough, lone `%` passthrough (`50% off`), wrapper without `%output%`, multiple `%output%`s all substitute, `--output_wrapper_file` override, missing file → typed error, trailing `%n%`, empty compiled + wrapper, Unicode-in-template, error display, low-level scanner edge cases.

### Pipeline matrix (cumulative across CLOC11)

After CLOC11.30 lands, `transform_source` per input does:

1. `--compilation_level` transform (CLOC11.06: WHITESPACE_ONLY active; others identity until CLOC11.07+).
2. `--define / -D` substitution (CLOC11.19).

Then `run_compiler` does:

3. Concatenation of transformed inputs.
4. **`--output_wrapper` template substitution (CLOC11.30, new).**
5. Write to `--js_output_file` (auto-create parent dirs) or stdout.

### Behavior changes (user-visible)

- `closurec --output_wrapper '(function(){%output%})();' --js app.js` now actually wraps. Previously the flag was parsed but ignored.
- `closurec --output_wrapper_file banner.txt --js app.js` reads `banner.txt` as the wrapper template.

### Tests

109 unit + 6 integration tests passing. Clippy clean.

### Version

Bumps closurec 0.6.0 → 0.7.0 (Cargo.toml + cli.spec.json kept in sync).

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.6.0] - 2026-05-25

### Added — CLOC11.19: `--define / -D` value substitution

Second behavioral slice of [CLOC11]. Users can now pass `--define NAME=value` (or `-D NAME=value`) and closurec will substitute every reference to `NAME` with `value` in the output.

- **New `defines` module.** Token-level substitution: tokenize via `javascript-lexer`, walk tokens, replace any identifier-type token whose value matches a `--define` key with the typed value rendered as JS source. Keywords (`if`, `var`, etc.) are explicitly NOT eligible. String-literal content is NOT substituted — `"DEBUG"` stays a string even if `DEBUG` is defined.
- **`DefineValue` rendering** for each variant of [`crate::config::DefineValue`]:
  - `Bool(true)` → `true`
  - `Bool(false)` → `false`
  - `Null` → `null`
  - `Number(42.0)` → `42` (integer-valued doubles emit without trailing `.0`, matching CC)
  - `Number(3.14)` → `3.14`
  - `Number(NaN)` → `NaN` sentinel
  - `Number(Infinity)` → `Infinity` (or `-Infinity`)
  - `String("hi")` → `"hi"` (re-quoted with JS escapes for `"`, `\`, LF, CR, TAB)
- **`transform_source` now runs in two phases:**
  1. **Level transform** — WHITESPACE_ONLY / identity per the compilation level (CLOC11.06 behavior).
  2. **Define substitution** — applies `cfg.defines.defines` over the level's output.
  This ordering means `--define DEBUG=false` composes naturally with `--compilation_level WHITESPACE_ONLY` (or with any future level transform).
- **Fast path:** when `cfg.defines.defines` is empty, `apply_defines` is a string-copy no-op (skips tokenization entirely).
- **New `CompilerError::Define(defines::DefineError)`** variant for substitution failures (currently only "tokenizer rejected the source").
- **Diff fixture `tests/diff/define/`** per CLOC11 §3.
- **New integration test `tests/diff_define.rs`** drives the actual binary against the fixture.
- **17 new unit tests in `defines::tests`** covering: empty defines passthrough, every DefineValue variant (bool/integer/fractional/string/null), case-sensitive identifier matching (`DEBUG` doesn't match `debug`), string-literal content protection, word-boundary preservation (`return DEBUG` → `return false`, not `returnfalse`), no-space-around-punctuation, multiple defines, keyword non-substitution, NaN/Infinity sentinels, embedded-quote re-escape, error display.

### Looseness vs. real CC

This is v1: we substitute *every* reference to a `--define` name, not only references to `goog.define`-annotated variables. In practice this matches what users expect when they pass `--define FLAG_DEBUG=false` for a flag they own. The cases where CC would NOT substitute (e.g. a `var FLAG_DEBUG` shadowing the same name) are rare in real builds. CLOC11.21+ will tighten the rule once we have JSDoc `@define`-aware metadata.

### Behavior changes (user-visible)

- `closurec --define DEBUG=false --js app.js` now actually substitutes `DEBUG` references in the output. Previously the flag was parsed but ignored.
- As a side-effect of routing through the tokenizer, the substitution output is already minified (single-space gap between word-like tokens, no spaces elsewhere). CC's WHITESPACE_ONLY does the same thing; we just get it for free.

### Tests

95 unit + 5 integration tests passing. Clippy clean.

### Version

Bumps closurec 0.5.0 → 0.6.0 (Cargo.toml + cli.spec.json kept in sync).

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.5.0] - 2026-05-25

### Added — CLOC11.06: `--compilation_level WHITESPACE_ONLY` wired

First *behavioral* compilation-level slice of [CLOC11]. CLOC11.01–03 wired the I/O layer; CLOC11.06 starts actually transforming JavaScript. The closurec binary now honors `--compilation_level WHITESPACE_ONLY` end-to-end, matching Closure's documented behavior at this level.

- **New `whitespace_only` module.** Token-level minifier: tokenize via `javascript-lexer::tokenize_javascript_typed`, drop trivia (comments / whitespace / newlines), re-stitch survivors with the minimum-necessary inter-token whitespace. Conservative space-insertion rule: a single space goes between two adjacent *word-like* tokens (identifier, number, keyword, regex, template, BigInt, private name); other adjacencies emit back-to-back.
- **String-literal re-quoting.** The lexer's `Token.value` is *unescaped* content (escape sequences resolved), so emitting it raw would corrupt `var s = "a\"b"`. The minifier re-quotes string tokens with double quotes and re-escapes `"`, `\`, LF, CR, TAB. Matches CC's WHITESPACE_ONLY canonicalization.
- **`transform_source(source, config)` dispatch added to `run.rs`.** New per-level matrix:
  - `WhitespaceOnly` → call into `whitespace_only::whitespace_only_minify`.
  - `Simple` / `Advanced` / `Bundle` / `TranspileOnly` → identity for now; CLOC11.07–10 land their real bodies.
- **`map_language_in_to_es_version`** projects `LanguageVersion` enum → `EsVersion` for the lexer. `Stable` / `EcmascriptNext` / `Unstable` / `NoTranspile` shortcuts all resolve to `EsVersion::latest()` so modern JS isn't silently downgraded.
- **`CompilerError::Minify(MinifyError)`** new variant; carries the underlying tokenizer error message with the offending source context.
- **Two new crate dependencies**: `coding-adventures-javascript-lexer` (the `tokenize_javascript_typed` entry point) and `lexer` (the underlying `Token` / `TokenType` types from the grammar-driven lexer).
- **Diff fixture `tests/diff/whitespace-only/`** per CLOC11 §3: a JS input with line comments, block comments, mixed whitespace, function bodies — `expected.stdout` is the canonical compact emission.
- **New integration test `tests/diff_whitespace_only.rs`** drives the built binary against the fixture and asserts byte-equal stdout.
- **11 new unit tests in `whitespace_only::tests`**: empty input, line-comment stripping, block-comment stripping, whitespace collapsing around punctuation, space-between-keywords (`return typeof x`), space-between-keyword-and-number (`return 1`, must not become `return1`), no-space-around-punctuation, string-literal content preservation through re-quoting, multiline-to-single-line, mixed-comments-and-whitespace, error display.

### Behavior changes (user-visible)

- **`closurec --compilation_level WHITESPACE_ONLY --js foo.js`** now actually minifies. Previously this invocation ran the identity pipeline.
- All other levels still run identity (concatenation) — that changes in CLOC11.07+.

### Implementation note — operating at the token level (not the AST)

The CLOC09 typed AST (`javascript_ast::Program`) and the parser (which produces `GrammarASTNode`) don't yet have a bridge. WHITESPACE_ONLY doesn't need the AST — it operates on tokens — so this PR skips the bridge and uses the lexer directly. Building the AST bridge is on the critical path for CLOC11.07 (`--compilation_level SIMPLE`) and will land then.

### Tests

78 unit + 4 integration tests passing. Clippy clean.

### Version

Bumps closurec 0.4.0 → 0.5.0 (Cargo.toml + cli.spec.json kept in sync).

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.4.0] - 2026-05-25

### Added — CLOC11.03: `--js_output_file` write semantics

Third implementation slice of [CLOC11] (drop-in Closure Compiler compatibility). CLOC11.01 wired `--js_output_file` to a simple `fs::write` call; this release brings the disk-write side to behavioral parity with the upstream Java tool.

- **Auto-create parent directories.** A `--js_output_file build/dist/app.min.js` no longer requires a preceding `mkdir -p build/dist`. The upstream `closure-compiler.jar` creates the parent tree automatically; we now match. Implemented as `fs::create_dir_all` on the resolved parent path, gated on `path.parent().is_some()` && `parent.exists() == false` so a bare filename in CWD doesn't try to `create_dir_all("")`.
- **`write_output_file(path, contents)` extracted as its own pub function.** Mirrors the CLOC11.02 pattern of splitting concerns into independently-testable units. The full pipeline (`run_compiler`) now calls it; unit tests can also call it directly.
- **Typed error on parent-create failure.** When `fs::create_dir_all` fails (e.g. the path collides with an existing regular file), we surface `CompilerError::OutputWriteError { path: <parent>, kind, message }` — the path field points at the parent so the user can fix the right thing.
- **Diff fixture `tests/diff/js-output-file/`** per CLOC11 §3: two .js inputs + flags.txt + expected.stdout.
- **Two new integration tests in `tests/diff_output_file.rs`**:
  - `js_output_file_writes_to_disk_with_auto_create_parents` — invokes the real binary with `--js_output_file <fresh-nested-path>`, asserts the file lands with the expected content and stdout stays empty.
  - `omitting_js_output_file_falls_back_to_stdout` — same fixture without the flag, asserts content lands on stdout.
- **Five new unit tests in `run::tests`**:
  - `write_output_file_creates_missing_parent_directories`
  - `write_output_file_bare_filename_does_not_create_dot` (regression: `parent()` of bare filename is `Some("")`; we must skip the `create_dir_all` rather than ask the OS to create an empty path)
  - `write_output_file_reports_create_dir_failure_as_typed_error` (file-where-directory-expected)
  - `run_compiler_autocreates_output_parent_dirs` (end-to-end)
  - `run_compiler_stdout_fallback_when_output_file_absent` (regression pin on the CLOC11.01 behavior)

### Known gap deferred to a follow-up

- **Empty-string value (`--js_output_file ""`) still rejected** by cli-builder's string validator at parse time (per `positional_resolver.rs`). The upstream Closure tool accepts it as a synonym for stdout. Closing this gap requires either (a) a cli-builder change to support `allow_empty: true` per-flag, or (b) a closurec-side argv preprocessor that special-cases the empty value. Both are out of scope for CLOC11.03 — tracked for a separate small PR. Workaround today: simply omit the flag to get stdout.

### What's NOT new

- v0.4.0 does not lex, parse, optimise, or emit JavaScript yet — the pipeline body remains "concatenate inputs". That work begins with CLOC11.06 (`--compilation_level WHITESPACE_ONLY`). CLOC11.03's value is making the I/O layer trustworthy for every later PR to build on.

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.3.0] - 2026-05-25

### Added — CLOC11.02: `--js` glob expansion + `!` exclusion

Second implementation slice of [CLOC11] (drop-in Closure Compiler compatibility). CLOC11.01 read `--js` values as literal file paths; this release replaces that with a real glob expander matching Closure's documented semantics.

- **New module `globs`.** Hand-rolled (zero-dep) glob matcher supporting:
  - `*` — matches any sequence within a single path segment.
  - `**` — matches zero or more whole path segments. Only special as a full segment per CC's docs; `src/**.js` is literal.
  - `?` — exactly one character within a segment.
  - `[abc]` / `[a-z]` / `[!abc]` — character classes with range and negation.
  - Literal text otherwise.
- **`!` exclusion.** A `--js` value starting with `!` removes everything it matches from the running included set. Mirrors Closure's behavior: `--js 'src/**/*.js' --js '!src/legacy/**'` includes all `src/` JS then drops the legacy subtree.
- **Walk strategy.** For each inclusion pattern we identify the longest fixed (glob-free) prefix and walk under it only — same optimisation as upstream `CommandLineRunner.findJsFiles`. Directory entries are sorted lexicographically before recursion so expansion is deterministic.
- **`resolve_inputs(config)`** extracted as its own pub function so glob behavior is unit-testable without going through full `run_compiler`. Result: `run_compiler` calls `resolve_inputs` first, then reads the resolved paths.
- **New `CompilerError::GlobExpansion(globs::GlobError)` variant** carrying the typed glob failure (NoMatches / InvalidPattern / WalkError) with the offending pattern.
- **Diff fixture `tests/diff/js-glob/`** per CLOC11 §3:
  - `input/` directory tree with 4 .js files including one excluded subtree.
  - `flags.txt` invoking `--js 'tests/diff/js-glob/input/**/*.js' --js '!tests/diff/js-glob/input/excluded/**'`.
  - `expected.stdout` with the concatenated content of the surviving 3 files in lex order.
- **`tests/diff_glob.rs`** integration test that runs the actual built binary against the fixture and asserts byte-equal output.

### Behavior changes (potentially user-visible)

- **Missing literal paths now error with `GlobExpansion(NoMatches)` instead of `InputReadError(NotFound)`**. Matches Closure's behavior (it emits `JSC_NO_JS_FILES_FOUND_FOR_PATTERN` regardless of whether the input was a glob or a literal). The `missing_input_returns_typed_error` test was updated to assert the new variant.
- **A `--js` invocation that produces zero matches is now a hard error** (exit code 2), even for literal paths. Closure does the same.

### Tests

21 new unit tests in `globs::tests`:
- 6 pure-function tests: literal vs glob detection, fixed-prefix splitting (including absolute paths), segment-matcher behavior for literals, `*`, `**`, `?`, char classes (positive, range, negative), invalid char class, error display.
- 9 filesystem-backed tests: literal-path passthrough, missing literal errors, `*.js`, `**/*.js` recursion, exclusion, no-matches error, invalid-pattern error, dedupe across overlapping inclusions, order preservation across patterns, subtree exclusion via `**`.

Plus the integration diff test brings the binary's total to 60 tests passing.

### Architecture

`globs.rs` is a single self-contained module under `code/programs/rust/closurec/src/`. No new crate dependencies. Per the repo's zero-dep working principle, this implements just enough of POSIX glob to match Closure's documented surface. Brace expansion (`{a,b}`), capture groups, and other features beyond Closure's surface are not supported and are not part of the v1 scope.

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.2.0] - 2026-05-24

### Added — CLOC11.01: CompilerConfig + identity build wiring

First implementation slice of [CLOC11] (drop-in Closure Compiler compatibility). Previously `closurec` validated argv, then printed `"closurec v0.1.0 - identity pipeline\n"` and exited — flag values were dropped on the floor. This release threads them through.

- **New module `config`.** A typed `CompilerConfig` struct with 18 per-feature sub-structs (`IoConfig`, `CompilationConfig`, `LanguageConfig`, `FormattingConfig`, `SourceMapConfig`, `DiagnosticsConfig`, `DefinesConfig`, `DependenciesConfig`, `ChunksConfig`, `PolyfillsConfig`, `RenamingReportsConfig`, `ExportsConfig`, `ConformanceConfig`, `InstrumentationConfig`, `SpecialModesConfig`, `SpecialPassesConfig`, `TranslationsConfig`, `JsonStreamsMode`). One sub-struct per row in CLOC11 §4's flag inventory, so later CLOC11.* PRs add lines, never new architecture.
- **New module `wire`.** `pub fn config_from_parsed(parsed: &ParseResult) -> Result<CompilerConfig, ConfigError>` translates cli-builder's `HashMap<String, serde_json::Value>` into the typed config. Every one of the 100 declared Closure Compiler flags gets read here; v1 of this PR only *uses* the I/O fields downstream, but all 100 flag slots are populated and tested.
  - `ConfigError::SpecMismatch` for "cli.spec.json says string but runtime got integer" — catches spec/wire drift loudly.
  - `ConfigError::InvalidDefine` for `--define NAME=value` values that aren't valid JS literals. Closure-strict semantics: bare unquoted strings rejected.
  - `ConfigError::Conflict` reserved for incompatible flag combinations in later PRs.
- **New module `run`.** `pub fn run_compiler(config: &CompilerConfig) -> Result<CompilerOutput, CompilerError>` executes the compiler. v1 = identity pipeline: read every `--js` literal path, concatenate with newline separators in input order, write to `--js_output_file` or stdout. CLOC11.02 will replace literal-path reads with glob expansion.
  - `CompilerError::InputReadError` / `OutputWriteError` carry the `io::ErrorKind` so callers format meaningfully without losing the underlying cause.
- **`main::parse_and_run` rewired.** The `ParserOutput::Parse` branch now calls `wire::config_from_parsed` → `run::run_compiler` and surfaces their results. Exit codes:
  - `0` — success (clean parse + successful compile).
  - `1` — argv parse error (unchanged).
  - `2` — compilation error (new; covers I/O failures and config validation).
- **23 new tests** across the three modules (config: 3, wire: 12, run: 7) plus updated existing CLI tests.

### Changed

- The "identity pipeline" banner now appears only when `--js` is absent. With `--js` inputs the binary reads + writes them.
- Pre-existing CLI-surface tests that fed nonexistent `--js` paths and pinned the banner string now assert "parses cleanly" (no `unknown`/`invalid` markers) rather than pinning the banner. The CLI *surface* contract is unchanged.

### Architecture notes

Per [CLOC11 §5], the bridge between cli-builder's untyped flag map and the compiler pipeline is one typed `CompilerConfig` with per-feature sub-structs. Adding a flag in any later CLOC11.* PR follows a fixed recipe:

1. Add a field to the appropriate sub-struct in `config.rs`.
2. Map it in the corresponding `read_*` function in `wire.rs`.
3. Consume it in `run.rs`.
4. Add a diff test under `tests/diff/<feature>/` (CLOC11 §3).

No new architectural pieces are needed per flag.

[CLOC11]: ../../specs/CLOC11-drop-in-closure-compat.md

## [0.1.0] - 2026-05-23

### Added
- New program per CLOC08 — the CLI driver that ties together every crate in Stages 1–4 (lexer, parser, type sidecar, JSDoc extractor, type-checker, pass pipeline + every canonical pass per CLOC06, emitter, source-map generator).
- **Drop-in compatibility with the upstream Java Closure Compiler at the command-line surface.** A script written against `java -jar closure-compiler.jar --js foo.js --js_output_file out.js --compilation_level ADVANCED` works unchanged when the `java -jar …` invocation is swapped for `closurec`.
- All ~100 flags from `CommandLineRunner.java` declared in [`cli.spec.json`](./cli.spec.json), a [cli-builder](../../../packages/rust/cli-builder) JSON spec embedded into the binary via `include_str!`:
  - inputs/outputs: `--js`, `--externs`, `--js_output_file`, `--chunk`, `--chunk_output_path_prefix`, `--chunk_wrapper`;
  - compilation control: `--compilation_level` (`BUNDLE`/`WHITESPACE_ONLY`/`SIMPLE`/`TRANSPILE_ONLY`/`ADVANCED`), `--checks_only`, `--continue_after_errors`, `--use_types_for_optimization`;
  - language: `--language_in`/`--language_out` with the full ECMAScript-3-through-2021 + `STABLE`/`NEXT`/`UNSTABLE` enumeration;
  - source maps: `--create_source_map`, `--source_map_format`, `--source_map_location_mapping`, `--source_map_input`, `--apply_input_source_maps`, `--source_map_include_content`, `--parse_inline_source_maps`;
  - modules: `--module_resolution`, `--js_module_root`, `--process_common_js_modules`, `--rewrite_polyfills`, `--isolate_polyfills`, `--inject_libraries`, `--force_inject_library`;
  - warnings: `--warning_level`, `--jscomp_error`/`--jscomp_warning`/`--jscomp_off`, `--hide_warnings_for`, `--warnings_allowlist_file`, `--extra_annotation_name`;
  - renaming + reports: `--variable_renaming_report`, `--property_renaming_report`, `--rename_variable_prefix`, `--rename_prefix_namespace`, `--variable_map_input_file`, `--property_map_input_file`;
  - output shape: `--isolation_mode`, `--output_wrapper`/`--output_wrapper_file`, `--chunk_output_type`;
  - formatting: `--formatting` (repeatable enum: `PRETTY_PRINT`/`PRINT_INPUT_DELIMITER`/`SINGLE_QUOTES`), `--charset`, `--emit_use_strict`;
  - conformance + framework hooks: `--conformance_configs`, `--angular_pass`, `--polymer_version`, `--chrome_pass`, `--j2cl_pass`, `--remove_j2cl_asserts`;
  - defines: `--define name[=val]` (short `-D`);
  - coverage: `--instrument_for_coverage_option`, `--production_instrumentation_array_name`, `--instrument_mapping_report`;
  - dependency management: `--dependency_mode`, `--entry_point`;
  - tracing + debugging: `--debug`, `--print_tree`/`--print_tree_json`/`--print_ast`, `--print_source_after_each_pass`, `--tracer_mode`, `--logging_level`, `--summary_detail_level`, `--output_manifest`, `--output_chunk_dependencies`, `--help_markdown`;
  - dynamic imports: `--allow_dynamic_import`, `--dynamic_import_alias`;
  - JSON streams: `--json_streams` (`NONE`/`IN`/`OUT`/`BOTH`);
  - misc: `--browser_featureset_year`, `--env`, `--third_party`, `--flagfile`, `--num_parallel_threads`, `--continue_after_errors`, `--assume_function_wrapper`, `--assume_static_inheritance_is_not_used`, `--assume_no_prototype_method_enumeration`, `--renaming`, `--error_format`, `--expected_diagnostics`.
- Short aliases honored: `-O` → `--compilation_level`, `-W` → `--warning_level`, `-D` → `--define`.
- `--help` / `-h` and `--version` injected automatically by cli-builder; version sourced from `Cargo.toml`.
- `parse_and_run(&[String]) -> (String, ExitCode)` is a **pure function** with no I/O — tests drive it directly without spawning the binary.
- Exit codes: `0` success, `1` parse error, `70` internal error (`EX_SOFTWARE`).
- 15 tests covering: `cli.spec.json` loads cleanly (90+ flags), `--help` long + short produce help text, `--version` returns the crate version, canonical Closure invocations parse (`--js`/`--js_output_file`/`--compilation_level`/`--create_source_map`), `--js` is repeatable, unknown flag returns error mentioning the bad flag, invalid enum value returns error, short aliases (`-O`, `-W`, `-D`) work, `--formatting` is a repeatable enum, deprecated hyphenated alias `--checks-only` is rejected (known v0.1.0 gap — see notes), empty argv parses cleanly with defaults, `version_string_matches_crate_version` locks the Cargo.toml ↔ spec sync.

### Changed from the (unmerged) earlier draft
- The earlier `feat/scaffold-closurec` revision used a hand-rolled `std::env::args` parser and a custom flag surface (`--input`, `--output`, `--source-map BOOL`, `--ascii-only BOOL`, `--pretty BOOL`, `--disable NAME`). It was reworked **before merge** at user direction to (a) use `cli-builder` declaratively and (b) be drop-in compatible with the Java Closure Compiler. The custom flag surface is retired.

### Notes
- **Known compatibility gaps in v0.1.0**: cli-builder doesn't currently support multiple long-form aliases per flag, so a handful of deprecated upstream aliases are not implemented. Use the canonical name instead:
  - `--checks-only` → `--checks_only`
  - `--dev_mode` → `--jscomp_dev_mode`
  - `--warnings_whitelist_file` → `--warnings_allowlist_file`
  - `--D` (long form) → `--define` or `-D`
  Real-world Closure invocations use the canonical underscored names; these deprecated forms are rarely seen. Adding alias support to cli-builder is tracked as a v0.2 enhancement.
- v1 is scaffolding. The whole pipeline is identity today (`javascript-ast` ships only `Program` / `SourceType` per CLOC02 Phase 1), so a successful compile prints `closurec v0.1.0 - identity pipeline\n` and exits 0. Real wiring lands when the AST grows nodes. Pinning the Closure-compatible CLI surface now means scripts that invoke the Java tool today can target `closurec` with no flag changes when the body fills in.
- Dependencies: `cli-builder`; every crate scaffolded in Stages 1–4; `serde`/`serde_json`.
- Required capabilities: `fs.read` + `fs.write`. v1 doesn't actually touch the filesystem yet (identity body skips it) but the manifest declares the future surface.
- Source of truth: when upstream Closure Compiler adds a flag, `cli.spec.json` is updated and the binary picks it up via `include_str!`; no Rust code changes are required.
