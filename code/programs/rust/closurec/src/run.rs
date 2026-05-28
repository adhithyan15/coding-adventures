//! `run` — execute a compiled [`CompilerConfig`].
//!
//! # CLOC11.01 scope
//!
//! v1: **identity pipeline.** Read every `--js` input as a literal
//! file path, concatenate the contents in input order, write to
//! `--js_output_file` (or stdout if absent). No lexing, no
//! parsing, no optimisation, no source maps.
//!
//! This is enough to:
//!
//! 1. Prove the wiring works end-to-end — flags → config → behavior
//!    → bytes on disk.
//! 2. Give later CLOC11.* PRs a "before" they can diff against.
//! 3. Replicate the simplest real Closure Compiler invocation
//!    (`--js a.js --js_output_file b.js` as a copy).
//!
//! Subsequent PRs replace `read → concatenate → write` with
//! `read → lex → parse → typecheck → passes → emit → write`.
//! The function signature doesn't change; only the body grows.

use crate::config::{CompilationLevel, CompilerConfig, IsolationMode};
use crate::defines;
use crate::globs;
use crate::whitespace_only;
use crate::wrapper;
use coding_adventures_javascript_tokens::EsVersion;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Output type
// ---------------------------------------------------------------------------

/// The result of a successful `run_compiler` invocation.
///
/// `stdout_text` is what should be printed (empty if `--js_output_file`
/// captured all output). `wrote_files` lists every path actually
/// written, so tests can assert on disk effects without reading
/// each file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompilerOutput {
    pub stdout_text: String,
    pub wrote_files: Vec<PathBuf>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Reasons compilation can fail.
///
/// I/O errors keep the underlying [`io::ErrorKind`] so the caller
/// can report a useful message; we don't try to wrap the full
/// [`io::Error`] (it isn't `Clone`/`PartialEq`).
#[derive(Debug, Clone, PartialEq)]
pub enum CompilerError {
    /// Couldn't read an input file.
    InputReadError {
        path: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },
    /// Couldn't write the output file.
    OutputWriteError {
        path: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },
    /// `--js` pattern expansion failed (invalid glob, no matches,
    /// FS walk error). The inner [`globs::GlobError`] carries the
    /// specific reason and the offending pattern.
    GlobExpansion(globs::GlobError),
    /// `--externs` pattern expansion failed (invalid glob, no
    /// matches, FS walk error). Distinguished from
    /// [`Self::GlobExpansion`] so the user sees which flag's
    /// pattern was bad. The inner [`globs::GlobError`] carries
    /// the offending pattern. CC produces a similar
    /// "JSC_NO_JS_FILES_FOUND_FOR_PATTERN"-equivalent error for
    /// externs that match nothing.
    ExternsGlobExpansion(globs::GlobError),
    /// `--compilation_level WHITESPACE_ONLY` minification failed.
    /// Carries the underlying [`whitespace_only::MinifyError`].
    Minify(whitespace_only::MinifyError),
    /// `--define / -D` substitution failed (tokenizer rejected the
    /// source). Inner [`defines::DefineError`] carries the message.
    Define(defines::DefineError),
    /// `--output_wrapper_file` couldn't be read.
    Wrapper(wrapper::WrapperError),
    /// `--print_tree` token-stream dump failed (tokenizer rejected
    /// the source). Inner [`print_tree::PrintTreeError`] carries
    /// the message.
    PrintTree(crate::print_tree::PrintTreeError),
}

impl std::fmt::Display for CompilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilerError::InputReadError { path, message, .. } => {
                write!(f, "failed to read input {}: {message}", path.display())
            }
            CompilerError::OutputWriteError { path, message, .. } => {
                write!(f, "failed to write output {}: {message}", path.display())
            }
            CompilerError::GlobExpansion(e) => write!(f, "{e}"),
            CompilerError::ExternsGlobExpansion(e) => {
                // Prefix so the user sees which flag's pattern
                // was bad. The inner GlobError already names the
                // offending pattern.
                write!(f, "--externs: {e}")
            }
            CompilerError::Minify(e) => write!(f, "{e}"),
            CompilerError::Define(e) => write!(f, "{e}"),
            CompilerError::Wrapper(e) => write!(f, "{e}"),
            CompilerError::PrintTree(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CompilerError {}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Transform a single input source string per the config's
/// `--compilation_level`.
///
/// # Level matrix (CLOC11.06)
///
/// | Level             | Transform                                   |
/// |-------------------|---------------------------------------------|
/// | `WhitespaceOnly`  | strip comments + collapse whitespace        |
/// | `Simple`          | identity (CLOC11.07 lands real passes)      |
/// | `Advanced`        | identity (CLOC11.08)                        |
/// | `Bundle`          | identity (CLOC11.09)                        |
/// | `TranspileOnly`   | identity (CLOC11.10)                        |
///
/// Mapping `config.language.language_in` → `EsVersion`:
/// - `Stable` / `EcmascriptNext` / `Unstable` / anything missing →
///   `EsVersion::latest()`.
/// - A specific year → that EsVersion.
///
/// This mapping lives here (not in `wire.rs`) because it's
/// transform-policy, not flag-mapping.
pub fn transform_source(
    source: &str,
    config: &CompilerConfig,
) -> Result<String, CompilerError> {
    transform_source_with_cv(source, config, None)
}

/// CV-aware variant of [`transform_source`] (CLOC11.61).
///
/// When `cv` is `Some((log, id))`, each pipeline stage appends a
/// [`Contribution`](coding_adventures_correlation_vector::Contribution)
/// to the per-file CV entry identified by `id`. The pre-CLOC11.61
/// `run_compiler` recorded one summary `transform_source.applied`
/// contribution per file; this slice replaces it with per-stage
/// records so the trace shows which pass touched the bytes and by
/// how much.
///
/// When `cv` is `None`, this function is byte-identical in
/// behavior to the original `transform_source` — same Result,
/// same error mapping, zero CV overhead.
///
/// # Contribution shape
///
/// | Stage              | `source`           | `tag`            | `meta`                                   |
/// |--------------------|--------------------|------------------|------------------------------------------|
/// | WhitespaceOnly     | `compilation_level`| `whitespace_only`| `{input_byte_len, output_byte_len}`      |
/// | Simple / Advanced  | `compilation_level`| `identity`       | `{level: "SIMPLE" \| "ADVANCED" \| ...}` |
/// | Defines            | `defines`          | `applied`        | `{input_byte_len, output_byte_len, defines_count}` |
///
/// The `defines.applied` contribution lands for every input even
/// when `--define` is empty (`defines_count: 0`), because the
/// stage *ran* — it just didn't do any substitutions. That keeps
/// the trace symmetric across files and visualization tools
/// don't have to special-case zero-defines runs.
pub fn transform_source_with_cv(
    source: &str,
    config: &CompilerConfig,
    cv: Option<(
        &mut coding_adventures_correlation_vector::CVLog,
        &str,
    )>,
) -> Result<String, CompilerError> {
    let es_version = map_language_in_to_es_version(config);

    // Hoist `cv` into a local so we can borrow it twice inside
    // the function (once per stage). The Option<&mut> doesn't
    // implement Copy, but we can reborrow at each call site.
    let mut cv_pair = cv;

    // Step 1 — compilation-level transform.
    let after_level = match config.compilation.level {
        CompilationLevel::WhitespaceOnly => {
            whitespace_only::whitespace_only_minify(source, es_version)
                .map_err(CompilerError::Minify)?
        }
        // CLOC11.07+ will replace each of these with real passes.
        CompilationLevel::Simple
        | CompilationLevel::Advanced
        | CompilationLevel::Bundle
        | CompilationLevel::TranspileOnly => source.to_string(),
    };

    if let Some((log, cv_id)) = cv_pair.as_mut() {
        let mut meta = std::collections::HashMap::new();
        let (tag, extras): (&str, Vec<(&str, serde_json::Value)>) =
            match config.compilation.level {
                CompilationLevel::WhitespaceOnly => (
                    "whitespace_only",
                    vec![
                        (
                            "input_byte_len",
                            serde_json::Value::Number((source.len() as u64).into()),
                        ),
                        (
                            "output_byte_len",
                            serde_json::Value::Number((after_level.len() as u64).into()),
                        ),
                    ],
                ),
                CompilationLevel::Simple => (
                    "identity",
                    vec![("level", serde_json::Value::String("SIMPLE".into()))],
                ),
                CompilationLevel::Advanced => (
                    "identity",
                    vec![("level", serde_json::Value::String("ADVANCED".into()))],
                ),
                CompilationLevel::Bundle => (
                    "identity",
                    vec![("level", serde_json::Value::String("BUNDLE".into()))],
                ),
                CompilationLevel::TranspileOnly => (
                    "identity",
                    vec![(
                        "level",
                        serde_json::Value::String("TRANSPILE_ONLY".into()),
                    )],
                ),
            };
        for (k, v) in extras {
            meta.insert(k.to_string(), v);
        }
        let _ = log.contribute(cv_id, "compilation_level", tag, meta);
    }

    // Step 2 — `--define / -D` substitution (CLOC11.19). Runs
    // *after* the compilation-level transform so that
    // WHITESPACE_ONLY's output is the input to substitution. The
    // ordering matches CC's: defines are applied during
    // compilation, alongside the level's pass set. The
    // composition matters because some `@define`-tagged names
    // can survive comment-stripping (they shouldn't be in
    // comments) but the *whitespace* changes don't affect
    // identifier matching either way.
    //
    // Fast path: with no defines, this is a no-op string copy.
    let after_defines = defines::apply_defines(
        &after_level,
        &config.defines.defines,
        es_version,
    )
    .map_err(CompilerError::Define)?;

    if let Some((log, cv_id)) = cv_pair.as_mut() {
        let mut meta = std::collections::HashMap::new();
        meta.insert(
            "input_byte_len".to_string(),
            serde_json::Value::Number((after_level.len() as u64).into()),
        );
        meta.insert(
            "output_byte_len".to_string(),
            serde_json::Value::Number((after_defines.len() as u64).into()),
        );
        meta.insert(
            "defines_count".to_string(),
            serde_json::Value::Number((config.defines.defines.len() as u64).into()),
        );
        let _ = log.contribute(cv_id, "defines", "applied", meta);
    }

    Ok(after_defines)
}

/// Project the typed `LanguageVersion` enum into the
/// `EsVersion` the lexer understands. Defaults to the latest
/// known version for the shortcuts (`Stable`, `EcmascriptNext`,
/// `Unstable`) so user code targeting "modern JS" never gets
/// silently restricted.
fn map_language_in_to_es_version(config: &CompilerConfig) -> EsVersion {
    use crate::config::LanguageVersion as L;
    match config.language.language_in {
        L::Ecmascript3 => EsVersion::Es5, // closest available
        L::Ecmascript5 | L::Ecmascript5Strict => EsVersion::Es5,
        L::Ecmascript2015 => EsVersion::Es2015,
        L::Ecmascript2016 => EsVersion::Es2016,
        L::Ecmascript2017 => EsVersion::Es2017,
        L::Ecmascript2018 => EsVersion::Es2018,
        L::Ecmascript2019 => EsVersion::Es2019,
        L::Ecmascript2020 => EsVersion::Es2020,
        L::Ecmascript2021 => EsVersion::Es2021,
        L::Stable | L::EcmascriptNext | L::Unstable | L::NoTranspile => EsVersion::latest(),
    }
}

/// Resolve the list of input files from a `CompilerConfig`.
///
/// Extracted as its own function so glob behavior is unit-testable
/// without touching output writing. v0.2.0 treated `js_patterns`
/// as literal file paths; v0.3.0 (CLOC11.02) replaces that with
/// real glob expansion (see [`crate::globs`]).
pub fn resolve_inputs(config: &CompilerConfig) -> Result<Vec<PathBuf>, CompilerError> {
    globs::expand_js_patterns(&config.io.js_patterns).map_err(CompilerError::GlobExpansion)
}

/// Resolve the list of `--externs` files from a `CompilerConfig`.
///
/// Same glob rules as `resolve_inputs` (CLOC11.02): `*`/`**`
/// patterns, `!` exclusion, missing-pattern errors. Returns
/// `Ok(vec![])` when no `--externs` were provided (empty
/// patterns slice). Errors are tagged with
/// [`CompilerError::ExternsGlobExpansion`] so users see which
/// flag's glob failed.
///
/// # Why a separate function from `resolve_inputs`
///
/// 1. Different error variant lets us prefix `"--externs: "`
///    instead of leaving the GlobError naked, so the user
///    diagnoses without re-reading the command line.
/// 2. Externs are not "inputs to compile"; conflating them
///    today would invite mistakes when CLOC11.07+ starts
///    actually using the resolved externs list for type
///    checking. Two-function shape is the right separation.
///
/// # No-externs fast path
///
/// `expand_js_patterns` errors on empty input (treating "no
/// patterns" as a user mistake). For `--externs`, an empty list
/// is the normal case (most invocations don't pass it) — we
/// short-circuit to `Ok(vec![])` before calling into the glob
/// machinery so the user isn't punished for not opting in.
pub fn resolve_externs(config: &CompilerConfig) -> Result<Vec<PathBuf>, CompilerError> {
    if config.io.externs.is_empty() {
        return Ok(Vec::new());
    }
    globs::expand_js_patterns(&config.io.externs).map_err(CompilerError::ExternsGlobExpansion)
}

/// Run the compiler with `config`.
///
/// CLOC11.02: glob-expanded inputs, identity pipeline body. See the
/// module docstring for the future expansion plan.
pub fn run_compiler(config: &CompilerConfig) -> Result<CompilerOutput, CompilerError> {
    // Step 0: identity-banner fallback. Empty argv → friendly
    // banner so users running `closurec` with no flags get a
    // useful response rather than a glob error.
    if config.io.js_patterns.is_empty() {
        return Ok(CompilerOutput {
            stdout_text: "closurec v0.1.0 - identity pipeline\n".to_string(),
            wrote_files: Vec::new(),
        });
    }

    // Step 1: glob-expand --js patterns into a concrete list of
    // files (CLOC11.02). Exclusion patterns (leading `!`) remove
    // from the accumulator. Errors out if any inclusion pattern
    // produces zero matches.
    let inputs = resolve_inputs(config)?;

    // Step 1.25 (CLOC11.05): validate --externs patterns by
    // glob-expanding them. The expansion result is discarded
    // today; the goal is to surface a JSC_NO_JS_FILES_FOUND_FOR
    // _PATTERN-equivalent error if the user typo'd an externs
    // path. When the typechecker bridge lands (CLOC11.07+),
    // store the resolved list in `CompilerOutput` or pass it
    // into the typecheck stage.
    //
    // Runs *after* --js resolution so a single invocation that
    // bad-globs both produces the first error encountered (which
    // matches CC: it stops at the first JSC_NO_JS_FILES error,
    // not after collecting all of them).
    let _resolved_externs = resolve_externs(config)?;

    // Step 1.5 (CLOC11.52/.53): --print_tree / --print_tree_json
    // short-circuit.
    //
    // CC's --print_tree dumps the parsed AST to stdout and exits
    // without emitting JS. --print_tree_json does the same in
    // JSON form. Until our parser produces the typed AST
    // (CLOC11.07-ish bridges that), we emit the *token stream*
    // — one significant token per line for text mode, an array
    // of token objects for JSON mode — which is the closest
    // analogue the lexer can produce.
    //
    // No transform pipeline runs: both flags are purely a
    // diagnostic readout of what the lexer sees, not what the
    // compilation level would do.
    //
    // If both flags are set, --print_tree (the older, simpler
    // flag) wins. This is a defensive choice; CC errors on the
    // conflict but emitting *something* is more useful than
    // refusing.
    if config.special_modes.print_tree {
        let es_version = map_language_in_to_es_version(config);
        let mut dump = String::new();
        for path in &inputs {
            let contents =
                fs::read_to_string(path).map_err(|e| CompilerError::InputReadError {
                    path: path.clone(),
                    kind: e.kind(),
                    message: e.to_string(),
                })?;
            // Header so multi-file dumps are navigable. Single
            // line, prefixed with `===` to be visually distinct
            // from token lines.
            dump.push_str("=== ");
            dump.push_str(&path.to_string_lossy());
            dump.push_str(" ===\n");
            let one_file =
                crate::print_tree::format_token_dump(&contents, es_version)
                    .map_err(CompilerError::PrintTree)?;
            dump.push_str(&one_file);
        }
        return Ok(CompilerOutput {
            stdout_text: dump,
            wrote_files: Vec::new(),
        });
    }

    if config.special_modes.print_tree_json {
        let es_version = map_language_in_to_es_version(config);
        let dump = if inputs.len() == 1 {
            // Single-file: emit just the tokens array. This is
            // what JSON consumers expect for the common case.
            let path = &inputs[0];
            let contents = fs::read_to_string(path).map_err(|e| {
                CompilerError::InputReadError {
                    path: path.clone(),
                    kind: e.kind(),
                    message: e.to_string(),
                }
            })?;
            crate::print_tree::format_token_dump_json(&contents, es_version)
                .map_err(CompilerError::PrintTree)?
        } else {
            // Multi-file: emit an array of file-objects (each
            // carrying its path + tokens) so consumers can
            // disambiguate. Reads all inputs upfront so a partial
            // failure doesn't leave us with a half-formed JSON
            // document on stdout.
            let mut sources = Vec::with_capacity(inputs.len());
            for path in &inputs {
                let contents = fs::read_to_string(path).map_err(|e| {
                    CompilerError::InputReadError {
                        path: path.clone(),
                        kind: e.kind(),
                        message: e.to_string(),
                    }
                })?;
                sources.push((path.to_string_lossy().into_owned(), contents));
            }
            crate::print_tree::format_token_dump_json_multi(&sources, es_version)
                .map_err(CompilerError::PrintTree)?
        };
        return Ok(CompilerOutput {
            stdout_text: dump,
            wrote_files: Vec::new(),
        });
    }

    // Step 2: read every resolved input, then transform per
    // --compilation_level (CLOC11.06+).
    //
    // Today's level dispatch:
    //   - WHITESPACE_ONLY → strip comments + collapse whitespace
    //     via the token-level minifier (whitespace_only.rs).
    //   - All other levels → identity (concatenate verbatim).
    //
    // CLOC11.07+ replace the SIMPLE/ADVANCED arms with real
    // lex/parse/typecheck/passes/emit pipelines.
    let mut combined = String::new();
    // CLOC11.60: opt-in correlation-vector log. Constructed in
    // "enabled" mode iff `--correlation_vector` was passed; in
    // disabled mode every CV call is a no-op. We accumulate
    // contributions through the per-file loop and dump them at
    // the end of `run_compiler` to a side-channel file
    // (`closurec-cv.json` by default) when enabled.
    let mut cv_log = coding_adventures_correlation_vector::CVLog::new(
        config.special_modes.correlation_vector,
    );
    for path in &inputs {
        let contents = fs::read_to_string(path).map_err(|e| CompilerError::InputReadError {
            path: path.clone(),
            kind: e.kind(),
            message: e.to_string(),
        })?;

        // CLOC11.60 — assign a CV ID at ingestion (the per-file
        // root), so every downstream contribution can attach to
        // it. The `Origin` is the file path; granularity is
        // per-file for now. CLOC11.61+ deepen this to per-token
        // / per-byte as the transform stages get CV-aware.
        let cv_id = if config.special_modes.correlation_vector {
            let mut meta = std::collections::HashMap::new();
            meta.insert(
                "byte_len".to_string(),
                serde_json::Value::Number((contents.len() as u64).into()),
            );
            Some(cv_log.create(Some(
                coding_adventures_correlation_vector::Origin {
                    source: "input_file".to_string(),
                    location: path.to_string_lossy().into_owned(),
                    timestamp: None,
                    meta,
                },
            )))
        } else {
            None
        };

        // CLOC11.61: per-stage CV contributions. When CV is on,
        // pass the per-file cv_id through to `transform_source_with_cv`
        // so the transform records one record per stage
        // (compilation_level + defines) rather than the single
        // summary contribution from CLOC11.60.
        let transformed = match &cv_id {
            Some(id) => transform_source_with_cv(
                &contents,
                config,
                Some((&mut cv_log, id.as_str())),
            )?,
            None => transform_source(&contents, config)?,
        };

        combined.push_str(&transformed);
        // Closure separates concatenated inputs with a newline so
        // back-to-back files don't end up syntactically merged.
        if !transformed.ends_with('\n') {
            combined.push('\n');
        }
    }

    // Step 2.25 (CLOC11.51): --checks_only short-circuits emission.
    //
    // CC's --checks_only validates the inputs (so the same parse/
    // type/diagnostic errors a real compile would catch still
    // surface) but emits *no* JS. Concretely: we already ran
    // tokenize-driven transforms above for each input — any
    // tokenizer rejection produced a typed error before we got
    // here. With --checks_only set, the user wants validation
    // only, so we discard `combined` (and never run wrapping or
    // write), returning an empty CompilerOutput.
    //
    // No `wrote_files`, no `stdout_text`: a CI script invoking
    // `closurec --checks_only --js app.js` cares about exit code
    // (0 if validation passed, non-zero otherwise) and stderr,
    // not stdout. Matches CC.
    if config.compilation.checks_only {
        return Ok(CompilerOutput {
            stdout_text: String::new(),
            wrote_files: Vec::new(),
        });
    }

    // Step 2.5 (CLOC11.18): prepend `"use strict";` when
    // `--emit_use_strict` is set. The directive must land at the
    // top of whatever scope ultimately wraps the code, so we
    // attach it to `combined` *before* the output wrapper and IIFE
    // run — both of those build syntactic envelopes around the
    // body, and a "use strict" directive only takes effect when
    // it sits at the very top of the function body it's meant to
    // govern. CC has the same ordering.
    if config.language.emit_use_strict {
        // Use double quotes to match CC's emission. A trailing
        // newline keeps the directive on its own line, which is
        // visually clearer in --output_wrapper templates that
        // include their own newlines.
        combined.insert_str(0, "\"use strict\";\n");
    }

    // Step 3 (CLOC11.30): apply --output_wrapper / --output_wrapper_file.
    // When neither is set, `apply_output_wrapper` short-circuits to
    // a passthrough so the common no-wrapper case stays a simple
    // copy. When a wrapper is set, `%output%` is replaced by the
    // accumulated `combined` JS and `%n%` is replaced by a literal
    // newline. `--output_wrapper_file` (read at this point, not at
    // config-build time) overrides the inline `--output_wrapper`
    // string when both are present.
    let wrapped = wrapper::apply_output_wrapper(
        &combined,
        &config.formatting.output_wrapper,
        config.formatting.output_wrapper_file.as_deref(),
    )
    .map_err(CompilerError::Wrapper)?;

    // Step 3.5 (CLOC11.31): apply --isolation_mode IIFE if set.
    // Layered after the user wrapper, matching CC's pipeline:
    // user wrapper runs first, IIFE wraps the result. So the
    // user's banner sits *inside* the IIFE — same semantics as
    // CC, and the behavior users requesting IIFE expect.
    let isolated = match config.formatting.isolation_mode {
        IsolationMode::Iife => wrapper::apply_iife_wrap(&wrapped),
        IsolationMode::None => wrapped,
    };

    // Step 3.75 (CLOC11.16): apply --charset normalization.
    //
    // CC's documented default is "UTF-8 in, US_ASCII out": every
    // non-ASCII character in the final output is escaped as
    // `\uXXXX` (or a UTF-16 surrogate pair for astral
    // codepoints) so the emitted JS is pure 7-bit ASCII and
    // safe in any transport. We match that default.
    //
    // Runs last in the transform chain so any non-ASCII the
    // user injected via `--output_wrapper` (e.g. a `©` in a
    // banner) gets escaped alongside the body. See
    // `crate::charset` for the table of accepted values.
    let encoded = crate::charset::apply_charset(
        &isolated,
        crate::charset::OutputCharset::from_raw(&config.io.charset),
    );

    // Step 4: write the output. Two cases:
    //   a) --js_output_file set → write to disk via write_output_file.
    //   b) absent → stdout via the returned `stdout_text`.
    //
    // Step 5 (CLOC11.42): when --create_source_map=path is set,
    // write a minimal v3 source map at that path. Today the
    // mappings are empty (real position tracking lands with the
    // parser-bridge in CLOC11.07+); the goal of this slice is
    // that build pipelines expecting a file at the path see one.
    // The source-map write runs *after* the JS write so callers
    // get a consistent on-disk pair (or no source map at all if
    // the flag is unset).
    let result = match &config.io.js_output_file {
        Some(path) => {
            write_output_file(path, &encoded)?;
            CompilerOutput {
                stdout_text: String::new(),
                wrote_files: vec![path.clone()],
            }
        }
        None => CompilerOutput {
            stdout_text: encoded,
            wrote_files: Vec::new(),
        },
    };

    // Step 5 (CLOC11.42): source map write.
    let mut wrote_files = result.wrote_files;
    if !config.source_map.path_template.is_empty() {
        let map_path = std::path::PathBuf::from(&config.source_map.path_template);
        let map_body = crate::source_map::format_minimal_v3(
            config.io.js_output_file.as_deref(),
        );
        write_output_file(&map_path, &map_body)?;
        wrote_files.push(map_path);
    }

    // Step 6 (CLOC11.34): --output_manifest write.
    //
    // CC's --output_manifest=path writes a newline-separated list
    // of every input file the compilation considered. The flag is
    // commonly used by build systems (Bazel rules_closure) to
    // verify that the compiler saw the same input set the build
    // graph said it should.
    //
    // We write the resolved `--js` patterns (post-glob expansion),
    // one absolute or normalized path per line. CC writes paths
    // exactly as supplied to `--js`; for closurec we use the
    // glob-resolved form because that's what the compilation
    // *actually consumed* (the user can see exactly which files
    // their wildcards matched).
    //
    // Empty inputs (banner mode) produce an empty manifest file
    // — still valid, still useful as a "compilation ran" marker.
    if let Some(manifest_path) = &config.chunks.output_manifest_file {
        let body = format_manifest(&inputs);
        write_output_file(manifest_path, &body)?;
        wrote_files.push(manifest_path.clone());
    }

    // Step 7 (CLOC11.60): write the correlation-vector trace as
    // a JSON sidecar file when `--correlation_vector` was set.
    //
    // Path policy:
    //   - When `--js_output_file` is set, the sidecar sits
    //     beside it as `<output>.cv.json`. Build pipelines
    //     consuming the JS get the trace automatically without
    //     a separate flag for the path.
    //   - When `--js_output_file` is absent (stdout), the
    //     sidecar lands at `closurec-cv.json` in the working
    //     directory. Discoverable; user can rename / move.
    //
    // CLOC11.6N (a follow-up slice) will add an explicit
    // `--correlation_vector_output <path>` flag so callers who
    // want a custom location don't have to rely on the
    // sidecar-of-output convention.
    if config.special_modes.correlation_vector {
        let sidecar_path = match &config.io.js_output_file {
            Some(p) => {
                let mut s = p.as_os_str().to_owned();
                s.push(".cv.json");
                PathBuf::from(s)
            }
            None => PathBuf::from("closurec-cv.json"),
        };
        let body = format_cv_log_json(&cv_log);
        write_output_file(&sidecar_path, &body)?;
        wrote_files.push(sidecar_path);
    }

    Ok(CompilerOutput {
        stdout_text: result.stdout_text,
        wrote_files,
    })
}

/// Serialize a `CVLog` to a pretty-printed JSON string for the
/// `--correlation_vector` sidecar. We use `serde_json::to_string_pretty`
/// rather than hand-rolling the format because the CV crate's
/// `CVEntry` / `Contribution` types are already `Serialize`, and
/// the sidecar is consumed by tooling (not pinned in fixtures), so
/// serde's formatting choices don't bite us.
///
/// On serialization failure (vanishingly rare for the CV crate's
/// well-formed structs), we fall back to a minimal `{}` document so
/// the write still succeeds — a missing sidecar would be worse than
/// a stub one for the build-pipeline-consumes-the-file case.
fn format_cv_log_json(cv_log: &coding_adventures_correlation_vector::CVLog) -> String {
    cv_log
        .to_json_string()
        .unwrap_or_else(|_| "{}".to_string())
}

/// Format the contents of an `--output_manifest` file from a
/// resolved input list. One path per line, trailing newline so
/// `wc -l` reports the right count and concatenation is
/// well-formed. Returns an empty string when the input list is
/// empty (consumer still gets a 0-byte marker file).
fn format_manifest(inputs: &[PathBuf]) -> String {
    if inputs.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(inputs.len() * 64);
    for p in inputs {
        out.push_str(&p.to_string_lossy());
        out.push('\n');
    }
    out
}

/// Write `contents` to `path`, creating any missing parent
/// directories first.
///
/// # Why auto-create parents
///
/// The upstream Java Closure Compiler creates the
/// `--js_output_file`'s parent directory tree if it doesn't
/// exist. Mirroring that here means a script using
/// `closurec --js_output_file build/dist/app.min.js` works
/// without a preceding `mkdir -p build/dist`. Without this
/// behavior, `fs::write` would fail with
/// `io::ErrorKind::NotFound` on every fresh build.
///
/// # What we deliberately don't do
///
/// - We don't try to expand `~` or `$HOME` in the path; the
///   shell does that. A path the user passes through e.g.
///   `--js_output_file=~/out.js` reaches us as literally `~/out.js`
///   only if the shell didn't expand it (uncommon — usually
///   means the value was quoted), and CC has the same limitation.
/// - We don't `chmod` the created directories; default umask
///   applies (matches CC).
/// - We don't atomically write via a tempfile + rename. CC writes
///   directly too; the disk-half-full scenario is rare enough
///   and CC's behavior is already what users expect.
///
/// Extracted as its own function so the directory-creation
/// behavior is unit-testable independently of `run_compiler`'s
/// glob expansion + concatenation logic.
pub fn write_output_file(path: &Path, contents: &str) -> Result<(), CompilerError> {
    // Create the parent directory tree if needed. `parent()` of
    // a bare filename like `"out.js"` is `Some("")` (the empty
    // path), so we skip the create when the parent is empty —
    // `fs::create_dir_all("")` would error needlessly.
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| CompilerError::OutputWriteError {
                path: parent.to_path_buf(),
                kind: e.kind(),
                message: format!("failed to create parent directory: {e}"),
            })?;
        }
    }
    fs::write(path, contents.as_bytes()).map_err(|e| CompilerError::OutputWriteError {
        path: path.to_path_buf(),
        kind: e.kind(),
        message: e.to_string(),
    })
}

/// Convenience wrapper for `main` — runs the compiler and prints
/// `stdout_text` to actual stdout, returning whether the write
/// itself succeeded.
///
/// Test code uses `run_compiler` directly so it can assert on the
/// `CompilerOutput` without spawning a process or capturing
/// stdout.
pub fn run_and_print(config: &CompilerConfig) -> Result<(), CompilerError> {
    let out = run_compiler(config)?;
    let stdout = io::stdout();
    let mut h = stdout.lock();
    let _ = h.write_all(out.stdout_text.as_bytes());
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IoConfig;
    use std::env;

    /// Unique temp path under the system temp dir. We don't pull
    /// in the `tempfile` crate — the repo principle is zero-dep
    /// where reasonable, and a unique-named file is enough for
    /// these tests.
    fn temp_path(suffix: &str) -> PathBuf {
        let id = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        env::temp_dir().join(format!("closurec-cloc11-01-{id}-{nanos}-{suffix}"))
    }

    #[test]
    fn no_inputs_emits_identity_banner() {
        let cfg = CompilerConfig::default();
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.stdout_text.contains("identity pipeline"));
        assert!(out.wrote_files.is_empty());
    }

    #[test]
    fn single_input_to_stdout_round_trips() {
        let in_path = temp_path("in.js");
        fs::write(&in_path, "alert(1);").expect("write input");

        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: None,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.stdout_text.contains("alert(1);"));
        assert!(out.wrote_files.is_empty());
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn single_input_to_output_file_writes_disk() {
        let in_path = temp_path("in.js");
        let out_path = temp_path("out.js");
        fs::write(&in_path, "console.log('hi');").expect("write input");

        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.stdout_text.is_empty(), "nothing on stdout when output file is set");
        assert_eq!(out.wrote_files, vec![out_path.clone()]);

        let written = fs::read_to_string(&out_path).expect("read output");
        assert!(written.contains("console.log('hi');"));
        let _ = fs::remove_file(&in_path);
        let _ = fs::remove_file(&out_path);
    }

    #[test]
    fn multiple_inputs_concatenate_in_order_with_newlines() {
        let a = temp_path("a.js");
        let b = temp_path("b.js");
        fs::write(&a, "// a").expect("write a");
        fs::write(&b, "// b").expect("write b");

        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![
                    a.to_string_lossy().to_string(),
                    b.to_string_lossy().to_string(),
                ],
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        // Both files appear; the first one gets a trailing newline
        // injected because its content didn't end with one.
        let s = &out.stdout_text;
        let a_idx = s.find("// a").expect("a in output");
        let b_idx = s.find("// b").expect("b in output");
        assert!(a_idx < b_idx, "input order preserved");
        let _ = fs::remove_file(&a);
        let _ = fs::remove_file(&b);
    }

    #[test]
    fn missing_input_returns_typed_error() {
        // CLOC11.02: a missing literal --js path now flows through
        // glob expansion first, which produces a NoMatches error
        // before we ever try to read the file. This matches CC's
        // behavior (a literal `--js missing.js` errors with
        // JSC_NO_JS_FILES_FOUND_FOR_PATTERN, not a low-level "file
        // not found"). We assert the typed GlobExpansion wrapper.
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec!["/nonexistent/path/closurec/test/missing.js".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let err = run_compiler(&cfg).expect_err("missing input must error");
        match err {
            CompilerError::GlobExpansion(inner) => {
                // Inner GlobError must be NoMatches for the literal.
                assert!(format!("{inner}").contains("missing.js"));
            }
            other => panic!("expected GlobExpansion, got {other:?}"),
        }
    }

    #[test]
    fn input_already_ending_in_newline_does_not_get_doubled() {
        let in_path = temp_path("already-newline.js");
        fs::write(&in_path, "var x;\n").expect("write");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        // Should be exactly the original 7 bytes — no extra newline
        // appended because the input already had one.
        assert_eq!(out.stdout_text, "var x;\n");
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn compiler_error_display_includes_path() {
        let e = CompilerError::InputReadError {
            path: PathBuf::from("/x/y.js"),
            kind: io::ErrorKind::PermissionDenied,
            message: "permission denied".to_string(),
        };
        let s = e.to_string();
        assert!(s.contains("/x/y.js"));
        assert!(s.contains("permission denied"));
        let _: &dyn std::error::Error = &e;
    }

    // ------------------------------------------------------------------
    // CLOC11.03 — write_output_file behavior
    // ------------------------------------------------------------------

    #[test]
    fn write_output_file_creates_missing_parent_directories() {
        // The signature use case: a fresh build that targets
        // `build/dist/app.min.js` without a prior `mkdir -p`.
        let base = temp_path("autocreate");
        let nested = base.join("a").join("b").join("c");
        let out_path = nested.join("result.js");
        assert!(!nested.exists(), "parent dir must not exist pre-write");

        write_output_file(&out_path, "// generated\n").expect("write ok");

        assert!(nested.is_dir(), "parent dir should now exist");
        let written = fs::read_to_string(&out_path).expect("read written file");
        assert_eq!(written, "// generated\n");
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn write_output_file_bare_filename_does_not_create_dot() {
        // `parent()` of a bare filename is `Some("")`. We must
        // skip the create_dir_all call rather than asking the OS
        // to create the empty path.
        let dir = temp_path("bare");
        fs::create_dir_all(&dir).expect("setup");
        let bare = dir.join("only.js");
        write_output_file(&bare, "// bare\n").expect("write ok");
        let written = fs::read_to_string(&bare).expect("read");
        assert_eq!(written, "// bare\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_output_file_reports_create_dir_failure_as_typed_error() {
        // Attempting to create a directory underneath a regular
        // file should surface as an OutputWriteError pointing at
        // the *parent* — not silently truncate the user's file or
        // panic.
        let blocker = temp_path("blocker.txt");
        fs::write(&blocker, "i am not a directory").expect("setup");
        // Try to write a child under a path that's actually a file.
        let bad = blocker.join("child").join("output.js");
        let err = write_output_file(&bad, "// won't land").expect_err("must error");
        match err {
            CompilerError::OutputWriteError { path, .. } => {
                // Either the create_dir_all hits the blocking
                // file (parent ends in /child) or the write itself
                // does. Either is a meaningful, typed error.
                assert!(
                    path.to_string_lossy().contains("blocker"),
                    "error path should mention the blocker: {path:?}"
                );
            }
            other => panic!("expected OutputWriteError, got {other:?}"),
        }
        let _ = fs::remove_file(&blocker);
    }

    #[test]
    fn run_compiler_autocreates_output_parent_dirs() {
        // End-to-end variant of the auto-create test: drive the
        // whole pipeline (config -> run_compiler -> write) and
        // verify the parent dir got created.
        let base = temp_path("e2e");
        let in_path = base.join("src").join("a.js");
        fs::create_dir_all(in_path.parent().unwrap()).expect("setup in");
        fs::write(&in_path, "var x = 1;\n").expect("write in");

        let out_path = base.join("build").join("dist").join("app.js");
        assert!(!out_path.parent().unwrap().exists());

        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert_eq!(out.wrote_files, vec![out_path.clone()]);
        let written = fs::read_to_string(&out_path).expect("read out");
        assert_eq!(written, "var x = 1;\n");
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn run_compiler_stdout_fallback_when_output_file_absent() {
        // CLOC11.01 already supported this; pin it as a CLOC11.03
        // regression test so we can't accidentally break stdout
        // fallback when the output-file path grows new behavior.
        let in_path = temp_path("stdout-fallback.js");
        fs::write(&in_path, "// content\n").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: None,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.wrote_files.is_empty());
        assert!(out.stdout_text.contains("// content"));
        let _ = fs::remove_file(&in_path);
    }

    // ------------------------------------------------------------------
    // CLOC11.18 — --emit_use_strict behavior
    // ------------------------------------------------------------------

    #[test]
    fn emit_use_strict_prepends_directive() {
        let in_path = temp_path("strict.js");
        fs::write(&in_path, "var x=1;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            language: crate::config::LanguageConfig {
                emit_use_strict: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        // Directive lands at the very top.
        assert!(
            out.stdout_text.starts_with("\"use strict\";"),
            "expected leading 'use strict'; got: {:?}",
            out.stdout_text
        );
        // And the original content survives.
        assert!(out.stdout_text.contains("var x=1;"));
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn emit_use_strict_default_does_not_prepend() {
        // When the flag isn't set, no prelude.
        let in_path = temp_path("no-strict.js");
        fs::write(&in_path, "alert(1);").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(!out.stdout_text.contains("use strict"));
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn emit_use_strict_lands_inside_iife() {
        // The directive must sit inside the IIFE body so it
        // actually governs the wrapped code — i.e. *after* the
        // `(function(){` opener.
        let in_path = temp_path("strict-iife.js");
        fs::write(&in_path, "var x=1;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            language: crate::config::LanguageConfig {
                emit_use_strict: true,
                ..Default::default()
            },
            formatting: crate::config::FormattingConfig {
                isolation_mode: crate::config::IsolationMode::Iife,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        // Verify the directive is *between* the IIFE opener and
        // the user code — not before the opener.
        let s = &out.stdout_text;
        let iife_start = s.find("(function(){").expect("iife opener");
        let directive = s.find("\"use strict\";").expect("directive");
        let body = s.find("var x=1;").expect("body");
        assert!(iife_start < directive, "got: {s}");
        assert!(directive < body, "got: {s}");
        assert!(s.ends_with("}).call(this);"), "got: {s}");
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn emit_use_strict_lands_inside_output_wrapper() {
        // With a user --output_wrapper, the directive should
        // appear inside the wrapper's `%output%` slot.
        let in_path = temp_path("strict-wrapper.js");
        fs::write(&in_path, "var x=1;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            language: crate::config::LanguageConfig {
                emit_use_strict: true,
                ..Default::default()
            },
            formatting: crate::config::FormattingConfig {
                output_wrapper: "PRE %output% POST".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        let s = &out.stdout_text;
        assert!(s.starts_with("PRE "), "got: {s}");
        assert!(s.contains("\"use strict\";"), "got: {s}");
        assert!(s.ends_with(" POST"), "got: {s}");
        // Directive sits between PRE and the body.
        let pre = s.find("PRE ").expect("pre");
        let directive = s.find("\"use strict\";").expect("directive");
        let body = s.find("var x=1;").expect("body");
        assert!(pre < directive);
        assert!(directive < body);
        let _ = fs::remove_file(&in_path);
    }

    // ------------------------------------------------------------------
    // CLOC11.51 — --checks_only behavior
    // ------------------------------------------------------------------

    #[test]
    fn checks_only_returns_empty_output() {
        let in_path = temp_path("checks-only-empty.js");
        fs::write(&in_path, "var x=1;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            compilation: crate::config::CompilationConfig {
                checks_only: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.stdout_text.is_empty(), "stdout must be empty: {:?}", out.stdout_text);
        assert!(out.wrote_files.is_empty());
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn checks_only_does_not_write_output_file() {
        // Even when --js_output_file is set, --checks_only must
        // skip the disk write entirely.
        let in_path = temp_path("checks-only-in.js");
        let out_path = temp_path("checks-only-out.js");
        fs::write(&in_path, "var x=1;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            compilation: crate::config::CompilationConfig {
                checks_only: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.wrote_files.is_empty());
        assert!(!out_path.exists(), "file should NOT have been written");
        let _ = fs::remove_file(&in_path);
    }

    // ------------------------------------------------------------------
    // CLOC11.52 — --print_tree behavior
    // ------------------------------------------------------------------

    #[test]
    fn print_tree_dumps_tokens_with_file_banner() {
        let in_path = temp_path("print-tree.js");
        fs::write(&in_path, "var x=1;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                print_tree: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        // Banner line names the file, then one token per line.
        assert!(out.stdout_text.contains("=== "), "banner: {:?}", out.stdout_text);
        assert!(
            out.stdout_text.contains(&in_path.to_string_lossy().to_string()),
            "banner names the file: {:?}",
            out.stdout_text
        );
        assert!(out.stdout_text.contains("\tvar"));
        assert!(out.stdout_text.contains("\tx"));
        assert!(out.stdout_text.contains("\t1"));
        // No files written; --print_tree is stdout-only and skips
        // the rest of the pipeline.
        assert!(out.wrote_files.is_empty());
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn print_tree_multi_file_emits_one_banner_per_file() {
        let a = temp_path("pt-a.js");
        let b = temp_path("pt-b.js");
        fs::write(&a, "var a;").expect("a");
        fs::write(&b, "var b;").expect("b");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![
                    a.to_string_lossy().to_string(),
                    b.to_string_lossy().to_string(),
                ],
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                print_tree: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        // Two banner lines, in input order.
        let banner_count = out.stdout_text.matches("=== ").count();
        assert_eq!(banner_count, 2, "got: {}", out.stdout_text);
        let a_idx = out.stdout_text.find("pt-a.js").expect("a banner");
        let b_idx = out.stdout_text.find("pt-b.js").expect("b banner");
        assert!(a_idx < b_idx, "input order: {}", out.stdout_text);
        let _ = fs::remove_file(&a);
        let _ = fs::remove_file(&b);
    }

    #[test]
    fn print_tree_skips_output_file_write() {
        // Even when --js_output_file is set, --print_tree must
        // bypass writing to disk — the dump goes to stdout only.
        let in_path = temp_path("pt-skip.js");
        let out_path = temp_path("pt-skip-out.js");
        fs::write(&in_path, "var x;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                print_tree: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.wrote_files.is_empty());
        assert!(!out_path.exists(), "file should NOT have been written");
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn print_tree_surfaces_lex_errors() {
        let in_path = temp_path("pt-broken.js");
        fs::write(&in_path, "var s = \"unterminated").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                print_tree: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let err = run_compiler(&cfg).expect_err("lex error must surface");
        match err {
            CompilerError::PrintTree(_) => {}
            other => panic!("expected PrintTree, got {other:?}"),
        }
        let _ = fs::remove_file(&in_path);
    }

    // ------------------------------------------------------------------
    // CLOC11.53 — --print_tree_json behavior
    // ------------------------------------------------------------------

    #[test]
    fn print_tree_json_single_file_emits_token_array() {
        let in_path = temp_path("ptj-single.js");
        fs::write(&in_path, "var x=1;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                print_tree_json: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        // Single-file → just the array; no `path` wrapper.
        assert!(out.stdout_text.starts_with("[\n"), "got: {:?}", out.stdout_text);
        assert!(out.stdout_text.ends_with("]\n"));
        assert!(!out.stdout_text.contains("\"path\""));
        assert!(out.stdout_text.contains("\"type\""));
        assert!(out.stdout_text.contains("\"value\": \"var\""));
        assert!(out.wrote_files.is_empty());
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn print_tree_json_multi_file_wraps_in_file_objects() {
        let a = temp_path("ptj-a.js");
        let b = temp_path("ptj-b.js");
        fs::write(&a, "var a;").expect("a");
        fs::write(&b, "var b;").expect("b");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![
                    a.to_string_lossy().to_string(),
                    b.to_string_lossy().to_string(),
                ],
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                print_tree_json: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        // Multi-file → file-object wrappers with `path` + `tokens`.
        assert!(out.stdout_text.contains("\"path\""));
        assert!(out.stdout_text.contains("\"tokens\""));
        assert_eq!(out.stdout_text.matches("\"path\"").count(), 2);
        // Both files' identifiers appear.
        assert!(out.stdout_text.contains("\"value\": \"a\""));
        assert!(out.stdout_text.contains("\"value\": \"b\""));
        let _ = fs::remove_file(&a);
        let _ = fs::remove_file(&b);
    }

    #[test]
    fn print_tree_json_skips_output_file_write() {
        let in_path = temp_path("ptj-skip.js");
        let out_path = temp_path("ptj-skip-out.js");
        fs::write(&in_path, "var x;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                print_tree_json: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.wrote_files.is_empty());
        assert!(!out_path.exists());
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn print_tree_wins_when_both_flags_set() {
        // Defensive precedence: --print_tree (older flag) wins
        // when --print_tree_json is also set. Diagnoses by the
        // banner-line marker, which JSON doesn't emit.
        let in_path = temp_path("ptj-both.js");
        fs::write(&in_path, "var x;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                print_tree: true,
                print_tree_json: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.stdout_text.contains("=== "), "text mode wins: {:?}", out.stdout_text);
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn print_tree_json_surfaces_lex_errors() {
        let in_path = temp_path("ptj-broken.js");
        fs::write(&in_path, "var s = \"unterminated").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                print_tree_json: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let err = run_compiler(&cfg).expect_err("lex error must surface");
        match err {
            CompilerError::PrintTree(_) => {}
            other => panic!("expected PrintTree, got {other:?}"),
        }
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn checks_only_surfaces_lex_errors_in_inputs() {
        // The validation phase still runs — a tokenizer-rejecting
        // input under --checks_only still produces an error.
        // Use a deliberately broken source: an unterminated string
        // literal.
        let in_path = temp_path("checks-only-broken.js");
        fs::write(&in_path, "var s = \"unterminated").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            compilation: crate::config::CompilationConfig {
                checks_only: true,
                // Force a path that tokenizes (WHITESPACE_ONLY).
                level: crate::config::CompilationLevel::WhitespaceOnly,
                ..Default::default()
            },
            ..Default::default()
        };
        // The transform_source step must surface the lex error.
        let result = run_compiler(&cfg);
        assert!(result.is_err(), "expected lex error, got: {result:?}");
        let _ = fs::remove_file(&in_path);
    }

    // ------------------------------------------------------------------
    // CLOC11.05 — --externs glob resolution
    // ------------------------------------------------------------------

    #[test]
    fn resolve_externs_returns_empty_when_no_externs_passed() {
        let cfg = CompilerConfig::default();
        let out = resolve_externs(&cfg).expect("ok");
        assert!(out.is_empty());
    }

    #[test]
    fn resolve_externs_glob_expands_real_files() {
        let dir = temp_path("externs-real");
        fs::create_dir_all(&dir).expect("setup");
        let p = dir.join("e1.js");
        fs::write(&p, "/** @const */ var GLOBAL_X = 1;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                externs: vec![p.to_string_lossy().to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let out = resolve_externs(&cfg).expect("ok");
        assert_eq!(out, vec![p]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_externs_returns_typed_error_for_missing_pattern() {
        let cfg = CompilerConfig {
            io: IoConfig {
                externs: vec![
                    "/nonexistent/path/closurec/test/missing-externs.js".to_string(),
                ],
                ..Default::default()
            },
            ..Default::default()
        };
        let err = resolve_externs(&cfg).expect_err("must error");
        match err {
            CompilerError::ExternsGlobExpansion(inner) => {
                assert!(format!("{inner}").contains("missing-externs.js"));
            }
            other => panic!("expected ExternsGlobExpansion, got {other:?}"),
        }
    }

    #[test]
    fn run_compiler_surfaces_externs_glob_error_with_flag_prefix() {
        // End-to-end: a bad --externs pattern errors out of
        // run_compiler with a Display that names the --externs
        // flag, so the user sees which glob was bad without
        // re-reading argv.
        let in_path = temp_path("ok-input.js");
        fs::write(&in_path, "var x;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                externs: vec!["/nonexistent/cloc11-05-externs.js".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let err = run_compiler(&cfg).expect_err("must error");
        assert!(
            err.to_string().contains("--externs:"),
            "error must mention --externs flag: {err}"
        );
        match err {
            CompilerError::ExternsGlobExpansion(_) => {}
            other => panic!("expected ExternsGlobExpansion, got {other:?}"),
        }
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn run_compiler_succeeds_when_externs_resolve() {
        // Happy-path: a real --externs file alongside a real
        // --js input compiles cleanly.
        let in_path = temp_path("with-externs.js");
        let ext_path = temp_path("the-extern.js");
        fs::write(&in_path, "var x;").expect("setup");
        fs::write(&ext_path, "/** @const */ var EXT;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                externs: vec![ext_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.stdout_text.contains("var x"));
        let _ = fs::remove_file(&in_path);
        let _ = fs::remove_file(&ext_path);
    }

    // ------------------------------------------------------------------
    // CLOC11.42 — --create_source_map minimal v3 emission
    // ------------------------------------------------------------------

    #[test]
    fn create_source_map_writes_file_when_path_set() {
        let in_path = temp_path("smap-input.js");
        let out_path = temp_path("smap-out.js");
        let map_path = temp_path("smap-out.js.map");
        fs::write(&in_path, "var x;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            source_map: crate::config::SourceMapConfig {
                path_template: map_path.to_string_lossy().to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.wrote_files.contains(&out_path), "JS written");
        assert!(out.wrote_files.contains(&map_path), "map written");
        let map_body = fs::read_to_string(&map_path).expect("read map");
        assert!(map_body.contains("\"version\": 3"));
        assert!(map_body.contains("\"file\": \""));
        assert!(map_body.contains("\"mappings\": \"\""));
        let _ = fs::remove_file(&in_path);
        let _ = fs::remove_file(&out_path);
        let _ = fs::remove_file(&map_path);
    }

    #[test]
    fn create_source_map_empty_path_writes_no_map() {
        // Default behavior: --create_source_map unset → no map
        // file written. Pin this regression so a future refactor
        // of the gate doesn't accidentally emit one always.
        let in_path = temp_path("no-smap.js");
        let out_path = temp_path("no-smap-out.js");
        fs::write(&in_path, "var x;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        // Only the JS output file is written.
        assert_eq!(out.wrote_files, vec![out_path.clone()]);
        let _ = fs::remove_file(&in_path);
        let _ = fs::remove_file(&out_path);
    }

    #[test]
    fn create_source_map_with_stdout_output_writes_map_with_empty_file_key() {
        // Source-map writing works even when there's no
        // --js_output_file (compiled JS goes to stdout). In that
        // case the map's `file` field is empty per the
        // source_map module's contract.
        let in_path = temp_path("stdout-smap.js");
        let map_path = temp_path("stdout-smap.js.map");
        fs::write(&in_path, "var x;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: None,
                ..Default::default()
            },
            source_map: crate::config::SourceMapConfig {
                path_template: map_path.to_string_lossy().to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.stdout_text.contains("var x"));
        assert!(out.wrote_files.contains(&map_path));
        let map_body = fs::read_to_string(&map_path).expect("read map");
        assert!(map_body.contains("\"file\": \"\""));
        let _ = fs::remove_file(&in_path);
        let _ = fs::remove_file(&map_path);
    }

    #[test]
    fn create_source_map_uses_basename_of_output_file() {
        // The map's `file` field should be the basename of the
        // compiled-output path, not the full path. Lets the map
        // be served from a different directory than the JS.
        let dir = temp_path("smap-basename-dir");
        fs::create_dir_all(&dir).expect("setup");
        let in_path = dir.join("in.js");
        let out_path = dir.join("nested").join("out.min.js");
        let map_path = dir.join("nested").join("out.min.js.map");
        fs::write(&in_path, "var x;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            source_map: crate::config::SourceMapConfig {
                path_template: map_path.to_string_lossy().to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let map_body = fs::read_to_string(&map_path).expect("read map");
        // Basename only — no directory components.
        assert!(map_body.contains("\"file\": \"out.min.js\""), "got: {map_body}");
        assert!(!map_body.contains("\"file\": \"nested/"));
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.34 — --output_manifest behavior
    // ------------------------------------------------------------------

    #[test]
    fn format_manifest_empty_inputs_yields_empty_string() {
        assert_eq!(format_manifest(&[]), "");
    }

    #[test]
    fn format_manifest_one_path_per_line_with_trailing_newline() {
        let inputs = vec![
            PathBuf::from("src/a.js"),
            PathBuf::from("src/b.js"),
            PathBuf::from("src/c.js"),
        ];
        let out = format_manifest(&inputs);
        assert_eq!(out, "src/a.js\nsrc/b.js\nsrc/c.js\n");
        // Pin the line count to match `wc -l`'s reading.
        assert_eq!(out.matches('\n').count(), 3);
    }

    #[test]
    fn output_manifest_writes_resolved_input_paths() {
        let dir = temp_path("manifest-dir");
        fs::create_dir_all(&dir).expect("setup");
        let in_path = dir.join("in.js");
        let manifest_path = dir.join("manifest.txt");
        let out_path = dir.join("out.js");
        fs::write(&in_path, "var x;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            chunks: crate::config::ChunksConfig {
                output_manifest_file: Some(manifest_path.clone()),
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.wrote_files.contains(&manifest_path), "manifest written");
        let body = fs::read_to_string(&manifest_path).expect("read manifest");
        // Manifest contains the resolved input path (the user-supplied
        // pattern after glob expansion).
        assert!(
            body.contains(&in_path.to_string_lossy().to_string()),
            "manifest missing input path. got: {body}"
        );
        assert!(body.ends_with('\n'));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn output_manifest_unset_writes_no_manifest() {
        // Default behavior: --output_manifest unset → no manifest
        // file written. Pin so a future refactor doesn't
        // accidentally emit one always.
        let in_path = temp_path("no-manifest-in.js");
        fs::write(&in_path, "var x;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.wrote_files.is_empty(), "no files written (stdout only)");
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn output_manifest_combines_with_js_output_file_and_source_map() {
        // All three writes coexist: JS output, source map, and
        // manifest. Order in `wrote_files`: JS, then source map,
        // then manifest (matches pipeline ordering).
        let dir = temp_path("manifest-trifecta");
        fs::create_dir_all(&dir).expect("setup");
        let in_path = dir.join("in.js");
        let out_path = dir.join("out.js");
        let map_path = dir.join("out.js.map");
        let manifest_path = dir.join("manifest.txt");
        fs::write(&in_path, "var x;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            source_map: crate::config::SourceMapConfig {
                path_template: map_path.to_string_lossy().to_string(),
                ..Default::default()
            },
            chunks: crate::config::ChunksConfig {
                output_manifest_file: Some(manifest_path.clone()),
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert_eq!(
            out.wrote_files,
            vec![out_path, map_path, manifest_path.clone()],
            "wrote_files in pipeline order: JS, source map, manifest"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.60 — opt-in correlation-vector plumbing
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_off_writes_no_sidecar() {
        // Default behavior: --correlation_vector unset → no CV
        // sidecar file. Pin so a refactor doesn't accidentally
        // always-on the CV trace (which would be visible
        // user-perf regression and disk-usage surprise).
        let in_path = temp_path("cv-off-in.js");
        fs::write(&in_path, "var x=1;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        // Stdout-only output, no sidecar.
        assert!(out.wrote_files.is_empty(), "no files written; got {:?}", out.wrote_files);
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn correlation_vector_on_writes_sidecar_next_to_output_file() {
        // With --js_output_file set, the sidecar lands at
        // `<output>.cv.json` next to the JS.
        let dir = temp_path("cv-on-dir");
        fs::create_dir_all(&dir).expect("setup");
        let in_path = dir.join("in.js");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        fs::write(&in_path, "var x=1;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.wrote_files.contains(&sidecar_path), "sidecar written");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        // Body must be JSON. We don't pin the exact format
        // (CVLog::to_json_string controls it) but assert the
        // shape: starts with `{`, contains an entries section.
        assert!(body.trim_start().starts_with('{'), "got: {body}");
        // The Origin we recorded names "input_file" — pin that
        // marker so the file→entry connection is verifiable
        // without re-parsing the JSON.
        assert!(body.contains("input_file"), "got: {body}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_on_writes_default_sidecar_when_stdout_output() {
        // When --js_output_file is absent, the sidecar lands at
        // `closurec-cv.json` in CWD. Test from a temp dir we
        // chdir into so we don't litter the repo root.
        let prev_cwd = std::env::current_dir().expect("cwd");
        let dir = temp_path("cv-stdout-dir");
        fs::create_dir_all(&dir).expect("setup");
        std::env::set_current_dir(&dir).expect("chdir");

        let in_path = dir.join("in.js");
        fs::write(&in_path, "var x=1;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        let expected_sidecar = PathBuf::from("closurec-cv.json");
        assert!(out.wrote_files.contains(&expected_sidecar), "got: {:?}", out.wrote_files);
        assert!(dir.join("closurec-cv.json").exists());

        // Restore cwd before the temp dir goes away.
        std::env::set_current_dir(prev_cwd).expect("restore cwd");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_records_one_entry_per_input_file() {
        // Two --js inputs → CV log gains two entries (one per
        // input). Verifies the per-file `create()` call lands.
        let dir = temp_path("cv-multi-dir");
        fs::create_dir_all(&dir).expect("setup");
        let a = dir.join("a.js");
        let b = dir.join("b.js");
        let out_path = dir.join("combined.js");
        let sidecar_path = dir.join("combined.js.cv.json");
        fs::write(&a, "var a=1;").expect("a");
        fs::write(&b, "var b=2;").expect("b");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![
                    a.to_string_lossy().to_string(),
                    b.to_string_lossy().to_string(),
                ],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        // Both file paths should appear as origins. Granular
        // assertion via substring count: each `a.js` and `b.js`
        // path appears at least once as the Origin.location.
        assert!(body.contains("a.js"), "missing a.js: {body}");
        assert!(body.contains("b.js"), "missing b.js: {body}");
        // CLOC11.61 superseded the CLOC11.60 single
        // "transform_source.applied" contribution with per-stage
        // records: `compilation_level` + `defines` per file. So
        // two files yields 2 × 2 = 4 contributions total, with
        // each stage's source string appearing twice (once per
        // file).
        let cl_marker = "\"source\":\"compilation_level\"";
        let def_marker = "\"source\":\"defines\"";
        assert_eq!(
            body.matches(cl_marker).count(),
            2,
            "expected 2 compilation_level contributions, got: {body}"
        );
        assert_eq!(
            body.matches(def_marker).count(),
            2,
            "expected 2 defines contributions, got: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.61 — per-stage CV contribution tests
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_records_compilation_level_stage_per_file() {
        // With WHITESPACE_ONLY level set, the per-file CV entry
        // should gain a `compilation_level.whitespace_only`
        // contribution with input/output byte_len in meta.
        let dir = temp_path("cv-cl-ws-dir");
        fs::create_dir_all(&dir).expect("setup");
        let in_path = dir.join("in.js");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        fs::write(&in_path, "var x  =  1; // trim me").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::WhitespaceOnly,
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        // The compilation_level tag is "whitespace_only" — pin
        // both the source-name and the tag verbatim.
        assert!(
            body.contains("\"source\":\"compilation_level\""),
            "missing compilation_level source: {body}"
        );
        assert!(
            body.contains("\"tag\":\"whitespace_only\""),
            "missing whitespace_only tag: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_records_identity_with_level_name_for_simple() {
        // Default level is SIMPLE; the identity-path
        // contribution should record the level name as meta.
        let dir = temp_path("cv-cl-simple-dir");
        fs::create_dir_all(&dir).expect("setup");
        let in_path = dir.join("in.js");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        fs::write(&in_path, "var x = 1;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            // Default compilation level is SIMPLE.
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        assert!(body.contains("\"tag\":\"identity\""), "got: {body}");
        assert!(body.contains("\"level\":\"SIMPLE\""), "got: {body}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_records_defines_stage_with_defines_count() {
        // The `defines.applied` contribution lands per file
        // regardless of whether the user passed any `--define`
        // entries. Meta includes a `defines_count` so the trace
        // shows whether substitutions were possible.
        let dir = temp_path("cv-defines-dir");
        fs::create_dir_all(&dir).expect("setup");
        let in_path = dir.join("in.js");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        fs::write(&in_path, "var DEBUG = false;").expect("setup");
        let mut defines = std::collections::BTreeMap::new();
        defines.insert(
            "DEBUG".to_string(),
            crate::config::DefineValue::Bool(true),
        );
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            defines: crate::config::DefinesConfig { defines },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        assert!(
            body.contains("\"source\":\"defines\""),
            "missing defines source: {body}"
        );
        assert!(
            body.contains("\"tag\":\"applied\""),
            "missing applied tag: {body}"
        );
        assert!(
            body.contains("\"defines_count\":1"),
            "missing defines_count: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_records_both_stages_in_pipeline_order() {
        // Both `compilation_level` and `defines` contributions
        // must appear, and the CVLog's pass_order should reflect
        // the call order (compilation_level → defines).
        let dir = temp_path("cv-both-stages-dir");
        fs::create_dir_all(&dir).expect("setup");
        let in_path = dir.join("in.js");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        fs::write(&in_path, "var x = 1;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        // Both source-names present.
        assert!(body.contains("\"source\":\"compilation_level\""));
        assert!(body.contains("\"source\":\"defines\""));
        // pass_order in the CVLog dump should show
        // compilation_level FIRST. We pin the substring `["compilation_level"` to catch the leading element.
        assert!(
            body.contains("\"pass_order\":[\"compilation_level\",\"defines\"]"),
            "expected pass_order [compilation_level, defines], got: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn transform_source_facade_is_byte_identical_to_with_cv_none() {
        // Pin the pre-CLOC11.61 contract: `transform_source(s, c)`
        // and `transform_source_with_cv(s, c, None)` produce the
        // same Result for the same inputs. Lets the facade stay
        // valid for callers that don't care about CV.
        let cfg = CompilerConfig::default();
        let source = "var x = 1;";
        let a = transform_source(source, &cfg).expect("a");
        let b = transform_source_with_cv(source, &cfg, None).expect("b");
        assert_eq!(a, b);
    }
}
