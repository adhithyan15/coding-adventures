# Changelog

All notable changes to the `coding-adventures-closurec` binary will be documented in this file.

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
