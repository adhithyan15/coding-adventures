# CLOC08 — `closurec` CLI Surface

## What this spec locks down

CLOC08 defines `closurec`, the binary that ties everything else
together: argument parsing, file I/O, exit codes, and integration
with the upstream Java Closure Compiler's CLI.

**Design principle (revised 2026-05-23):** `closurec` is a
**drop-in replacement** for the Java Closure Compiler at the
command-line surface. A script written against
`java -jar closure-compiler.jar --js foo.js --js_output_file out.js
--compilation_level ADVANCED` must work unchanged when the
`java -jar …` invocation is swapped for `closurec`.

> **Note**: an earlier draft of this spec proposed a friendlier
> custom CLI (`--out-dir`, `--mode=simple|advanced|custom`,
> `--enabled-pass`, `--disabled-pass`). It was retired in favor of
> drop-in compatibility — users coming from the Java tool
> shouldn't have to relearn the flag set. The friendlier names
> may return as v2+ aliases once cli-builder grows alias
> support.

## Source of truth for the flag list

`closurec`'s flag surface mirrors
[`CommandLineRunner.java`](https://github.com/google/closure-compiler/blob/master/src/com/google/javascript/jscomp/CommandLineRunner.java)
in the upstream Closure Compiler project, flag for flag.

The **canonical declaration** of the flag list lives in
[`code/programs/rust/closurec/cli.spec.json`](../programs/rust/closurec/cli.spec.json),
a [`cli-builder`](../packages/rust/cli-builder) spec that declares
each flag's name, type, defaults, enum values, conflicts, and
short aliases. This spec is the single source of truth — when
upstream Closure Compiler adds a flag, we mirror it in
`cli.spec.json` and the binary picks it up via `include_str!`.

The spec covers all ~100 flags from upstream, including:

- **Inputs / outputs**: `--js`, `--externs`, `--js_output_file`,
  `--chunk`, `--chunk_output_path_prefix`, `--chunk_wrapper`.
- **Compilation control**: `--compilation_level`
  (`BUNDLE` | `WHITESPACE_ONLY` | `SIMPLE` | `TRANSPILE_ONLY` |
  `ADVANCED`), `--checks_only`, `--continue_after_errors`,
  `--use_types_for_optimization`.
- **Language**: `--language_in` / `--language_out` with the
  full ECMAScript-3-through-2021 + STABLE/NEXT/UNSTABLE
  enumeration. `--strict_mode_input`, `--emit_use_strict`.
- **Source maps**: `--create_source_map`,
  `--source_map_format` (`V3` | `DEFAULT`),
  `--source_map_location_mapping`, `--source_map_input`,
  `--apply_input_source_maps`, `--source_map_include_content`,
  `--parse_inline_source_maps`.
- **Modules**: `--module_resolution`
  (`BROWSER` | `NODE` | `WEBPACK`),
  `--browser_resolver_prefix_replacements`,
  `--package_json_entry_names`, `--js_module_root`,
  `--process_common_js_modules`, `--rewrite_polyfills`,
  `--isolate_polyfills`, `--inject_libraries`,
  `--force_inject_library`.
- **Warnings**: `--warning_level` (`QUIET` | `DEFAULT` |
  `VERBOSE`), `--jscomp_error`, `--jscomp_warning`,
  `--jscomp_off`, `--hide_warnings_for`,
  `--warnings_allowlist_file`, `--extra_annotation_name`.
- **Renaming + reports**: `--variable_renaming_report`,
  `--property_renaming_report`, `--rename_variable_prefix`,
  `--rename_prefix_namespace`, `--variable_map_input_file`,
  `--property_map_input_file`.
- **Output shape**: `--isolation_mode` (`NONE` | `IIFE`),
  `--output_wrapper`, `--output_wrapper_file`,
  `--chunk_output_type` (`GLOBAL_NAMESPACE` | `ES_MODULES`).
- **Formatting**: `--formatting` (`PRETTY_PRINT` |
  `PRINT_INPUT_DELIMITER` | `SINGLE_QUOTES` — repeatable),
  `--charset`, `--emit_use_strict`.
- **Polyfills**: `--rewrite_polyfills`, `--isolate_polyfills`,
  `--inject_libraries`, `--force_inject_library`.
- **Conformance + custom annotations**:
  `--conformance_configs`, `--extra_annotation_name`,
  `--angular_pass`, `--polymer_version`, `--chrome_pass`,
  `--j2cl_pass`, `--remove_j2cl_asserts`.
- **Defines**: `--define name[=val]` (short `-D`).
- **Coverage**: `--instrument_for_coverage_option`
  (`NONE` | `LINE` | `BRANCH` | `PRODUCTION`),
  `--production_instrumentation_array_name`,
  `--instrument_mapping_report`.
- **Dependency management**: `--dependency_mode`
  (`NONE` | `SORT_ONLY` | `PRUNE` | `PRUNE_LEGACY`),
  `--entry_point`.
- **Tracing + debugging**: `--debug`, `--print_tree`,
  `--print_tree_json`, `--print_ast`,
  `--print_source_after_each_pass`, `--tracer_mode`,
  `--logging_level`, `--summary_detail_level`,
  `--output_manifest`, `--output_chunk_dependencies`,
  `--help_markdown`.
- **Dynamic imports**: `--allow_dynamic_import`,
  `--dynamic_import_alias`.
- **JSON streams**: `--json_streams`
  (`NONE` | `IN` | `OUT` | `BOTH`).
- **Misc**: `--browser_featureset_year`, `--env`
  (`BROWSER` | `CUSTOM`), `--third_party`,
  `--flagfile` (repeatable), `--num_parallel_threads`,
  `--continue_after_errors`,
  `--assume_function_wrapper`,
  `--assume_static_inheritance_is_not_used`,
  `--assume_no_prototype_method_enumeration`,
  `--allow_dynamic_import`, `--dynamic_import_alias`,
  `--renaming`, `--error_format`, `--expected_diagnostics`.

The cli-builder spec also gets `--help` and `--version` for free
via cli-builder's built-in flag injection. The version string is
sourced from Cargo.toml.

### Short aliases

The Java tool ships three single-letter aliases. `closurec`
honors them:

| Short | Long |
|-------|------|
| `-O`  | `--compilation_level` |
| `-W`  | `--warning_level` |
| `-D`  | `--define` |

### Known compatibility gaps in v0.1.0

cli-builder doesn't currently support multiple long-form aliases
per flag, so a handful of deprecated upstream aliases are **not
implemented** in v0.1.0. Users should pass the canonical name
instead:

| Deprecated alias    | Use instead              |
|---------------------|--------------------------|
| `--checks-only`     | `--checks_only`          |
| `--dev_mode`        | `--jscomp_dev_mode`      |
| `--warnings_whitelist_file` | `--warnings_allowlist_file` |
| `--D` (long form)   | `--define` or `-D`       |

Passing a deprecated alias yields a clear "unknown flag" error
with no "did you mean?" suggestion pointing at the canonical
name. (Improving the suggestion to point at the canonical when
the deprecated name is recognized is tracked as a v0.2
enhancement.)

## Crate location and layout

`closurec` is a **program**, not a library — it lives under
`code/programs/rust/closurec/` (parallel to `cowsay`,
`mini-redis`, etc.). It is a standalone Cargo crate, not part of
the `code/packages/rust/Cargo.toml` workspace.

```
code/programs/rust/closurec/
├── Cargo.toml                  # binary target + deps on every Stage 1-4 crate
├── cli.spec.json               # canonical flag declaration (cli-builder spec)
├── src/
│   └── main.rs                 # parse_and_run() + main()
├── README.md
├── CHANGELOG.md
├── BUILD / BUILD_windows
└── required_capabilities.json  # fs.read + fs.write
```

### Dependency whitelist

- `cli-builder` — declarative CLI parsing.
- `coding-adventures-javascript-tokens`, `-javascript-ast`,
  `-type-sidecar`, `-correlation-vector`.
- `coding-adventures-closure-typechecker`.
- `coding-adventures-closure-pass-pipeline` + all 8 pass crates.
- `coding-adventures-closure-emitter`, `-closure-source-map`.
- `serde`, `serde_json`.

No third-party dependency beyond the workspace.

## Architecture

```text
  std::env::args  ─►  parse_and_run(&[String])  ─►  (String, ExitCode)
                                │
                                ▼
                        cli_builder::Parser
                                │
                  ┌─────────────┼─────────────┐
                  ▼             ▼             ▼
           ParserOutput::  ParserOutput:: ParserOutput::
              Parse(r)      Help(h)        Version(v)
                  │             │             │
                  ▼             ▼             ▼
              v1: banner     h.text       v.version
              v2+: pipeline
```

`parse_and_run` is a pure function over `&[String]` — no I/O, no
global state. Tests drive it directly without spawning the
binary. `main` is a thin wrapper.

## Exit codes

| Code | Meaning |
|------|---------|
| 0    | Success (Parse, Help, Version). |
| 1    | Parse error (unknown flag, invalid value, missing required flag, conflicting flags). |
| 70   | Internal error (`cli.spec.json` malformed — bug in *us*, not user error). Matches `EX_SOFTWARE` per `sysexits.h`. |

Future v2 codes:
- 2 reserved for usage error (when we add subcommands that fail
  validation in a non-flag-parser way).
- 3 reserved for compilation error.
- 4 reserved for I/O error.

This is more conservative than the Java tool's exit-code scheme
(which uses 0 vs 1 only) but lines up with Unix tradition
(`sysexits.h`).

## Scope (v1)

The whole pipeline is **identity** today — `javascript-ast`
ships only `Program` / `SourceType` per CLOC02 Phase 1, every
pass is a no-op, and the emitter returns empty output. v1 of
`closurec`:

- parses every Closure Compiler flag (validation, type
  checking, repeatable handling, enum values),
- returns clear errors on misuse,
- on a valid invocation, prints
  `closurec v0.1.0 - identity pipeline\n` and exits 0,
- exits with status 1 on parse error.

The actual lex/parse/typecheck/passes/emit wiring lands when
the AST grows nodes. **Pinning the Closure-compatible CLI
surface now means scripts and CI configs that invoke the Java
tool today can target `closurec` with no flag changes when the
body fills in.**

## What this PR locks down even as identity

1. The flag surface — every name, type, enum value, default,
   short alias matches `CommandLineRunner.java`.
2. The `parse_and_run` API — pure function, returns
   `(String, ExitCode)`. Tests don't need to spawn the binary.
3. The exit-code convention.
4. The `cli.spec.json` location as the single source of truth.

## What's coming

- v2: real lex → parse → typecheck → pipeline → emit wiring
  once the AST grows variants. `--js` actually reads files;
  `--js_output_file` actually writes one. Compilation failures
  surface as exit code 3.
- v2: route specific flags to pass configuration —
  `--jscomp_off` / `--jscomp_warning` / `--jscomp_error` populate
  the warning level map; `--define` populates a value map the
  passes consult; `--compilation_level` selects a canonical pass
  preset; `--use_types_for_optimization` toggles type-driven
  pass behavior.
- v0.2 alias enhancement: hyphenated long-form aliases per
  flag, when cli-builder supports them.
- A `--debug-cv` extension flag that dumps the CV log for
  tracing how each output byte came from which input byte.
- `--config FILE` for project-level defaults (likely
  Closure-Compiler-style `flagfile` — already in the spec).
