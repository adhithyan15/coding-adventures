# Changelog

All notable changes to the `coding-adventures-closurec` binary will be documented in this file.

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
