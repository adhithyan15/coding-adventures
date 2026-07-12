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
use coding_adventures_javascript_lexer::tokenize_javascript_typed;
use coding_adventures_javascript_parser::{bridge, bridge::BridgeError, parse_javascript_typed};
use coding_adventures_javascript_tokens::EsVersion;
use lexer::token::TokenType;
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
/// each file. `stderr_text` (CLOC11.75) is content the caller
/// should print to stderr — used today for the CV summary
/// when `--correlation_vector_summary_stderr` is set so the
/// summary doesn't corrupt a stdout-bound JS payload. Default
/// empty; tests can assert on routing without grepping fds.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompilerOutput {
    pub stdout_text: String,
    pub stderr_text: String,
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
    /// `--compilation_level SIMPLE` bridge parse produced an
    /// `InternalError` (a bug in the bridge, not unsupported
    /// syntax). Graceful degrade to whitespace_only is NOT applied
    /// for internal errors — they indicate a bridge invariant
    /// violation and must surface to the caller.
    ///
    /// `BridgeError::UnsupportedSyntax` (Phase 2+ constructs) is
    /// NOT mapped here; it causes a silent degrade to
    /// `whitespace_only` output instead.
    Bridge(String),
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
            CompilerError::Bridge(msg) => write!(f, "bridge internal error: {msg}"),
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
/// | Level             | Transform                                         |
/// |-------------------|---------------------------------------------------|
/// | `WhitespaceOnly`  | strip comments + collapse whitespace              |
/// | `Simple`          | bridge → typed optimization pipeline → emit       |
/// | `Advanced`        | same typed pipeline as `Simple` (≥ SIMPLE;        |
/// |                   | advanced-only passes land here as implemented)    |
/// | `Bundle`          | identity                                          |
/// | `TranspileOnly`   | identity                                          |
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
/// | Simple             | `compilation_level`| `simple_v2`      | `{level, bridge_status, passes, input_byte_len, output_byte_len}` |
/// | Advanced           | `compilation_level`| `advanced_v1`    | same shape as `simple_v2`; `passes` adds `rename-globals` (aggressive top-level renaming) |
/// | Bundle / Transpile | `compilation_level`| `identity`       | `{level: "BUNDLE" \| "TRANSPILE_ONLY"}` |
/// | Defines            | `defines`          | `applied`        | `{input_byte_len, output_byte_len, defines_count}` |
///
/// **`bridge_status`** (Simple only): `"ok"` if the full
/// parse→bridge→passes→emit chain succeeded, otherwise the degrade
/// reason — `"parse_error:<e>"`, `"unsupported_syntax:<rule>@<loc>"`
/// (Phase 2+ constructs), `"pass_error:<e>"`, or `"emit_error:<e>"` —
/// in all of which cases the output falls back to whitespace_only.
/// `"n/a"` when the level is not Simple.
///
/// The `defines.applied` contribution lands for every input even
/// when `--define` is empty (`defines_count: 0`), because the
/// stage *ran* — it just didn't do any substitutions. That keeps
/// the trace symmetric across files and visualization tools
/// don't have to special-case zero-defines runs.
/// The ordered list of optimization passes the SIMPLE level runs.
///
/// Each PR appends one more SIMPLE-appropriate pass here (and to the
/// pipeline below) so the growth is auditable one pass at a time. The
/// names are the passes' own canonical `Pass::name()` values; they also
/// become the `passes` field in the correlation-vector trace.
///
/// The list is in execution order. Ordering is enforced two ways: every
/// pass that needs a predecessor declares it via `Pass::depends_on`, and
/// where two passes are mutually independent the pipeline falls back to
/// *registration order* as the tie-breaker.
///
/// - constant-fold turns `2 > 3` into the literal `false`;
/// - fold-control-flow then prunes `if (false) {A}` to an empty `;`;
/// - dce then sweeps that empty statement (and any code after a `return`)
///   away;
/// - inline is in the chain because `remove-unused-vars` declares
///   `depends_on = ["dce", "inline"]` — the scheduler will not run
///   remove-unused-vars unless `inline` is registered. (inline is
///   currently an identity pass; it earns its slot once function
///   inlining lands, and registering it now pins the canonical order.)
/// - remove-unused-vars deletes top-level `var/let/const` bindings that
///   nothing references, when their initializer is side-effect-free;
/// - treeshake runs last and deletes top-level `function`/`class`
///   declarations that nothing references. It is the function-shaped
///   complement to remove-unused-vars (which deliberately skips
///   functions). Running it after remove-unused-vars means a function
///   that only a now-removed var referenced is itself swept this pass.
///   Removing an unused function declaration is unconditionally safe —
///   declaring a function has no side effect, so no purity gate is
///   needed (unlike a `var` initializer).
const SIMPLE_PASS_NAMES: &[&str] = &[
    "constant-fold",
    "fold-control-flow",
    "dce",
    "inline",
    "inline-variables",
    "remove-unused-vars",
    "treeshake",
    "rename",
];

/// The ADVANCED pipeline = every SIMPLE pass, then `rename-globals` —
/// aggressive renaming of program-private top-level names (the canonical
/// ADVANCED-over-SIMPLE win). It runs *after* `rename` (which shortens
/// leaf-function locals) so the two renamers shorten disjoint name
/// layers. `rename-globals` is gated by the `--externs` do-not-rename
/// boundary; see [`externs_do_not_rename`].
///
/// A further `rename-properties` pass runs after `rename-globals` **only
/// when the user supplied `--externs`** (it is appended dynamically at the
/// trace site, not listed here, because it is conditional). Property
/// renaming is unsafe without an externs property boundary — the bundled
/// built-in list omits the DOM — so it is opt-in via the externs contract.
/// See [`AdvancedConfig`] and [`collect_externs_property_names`].
const ADVANCED_PASS_NAMES: &[&str] = &[
    "constant-fold",
    "fold-control-flow",
    "dce",
    "inline",
    "inline-variables",
    "remove-unused-vars",
    "treeshake",
    "rename",
    "rename-globals",
];

/// ADVANCED-only pipeline configuration. Passing `Some` to
/// [`run_typed_pipeline`] selects ADVANCED; `None` is SIMPLE.
///
/// The two fields encode the two halves of the externs contract — the
/// value namespace (which top-level identifiers are external) and the
/// property namespace (which member/key names are external). They are
/// independent boundaries: `rename-globals` always runs under ADVANCED
/// (with an empty keep-set when no externs were given, since top-level
/// renaming is sound under closurec's whole-program contract), whereas
/// `rename-properties` is **gated on the user supplying `--externs`** —
/// without an externs file we have no DOM/host property boundary and the
/// bundled built-in list omits the DOM, so renaming properties by default
/// would miscompile browser code.
struct AdvancedConfig {
    /// Top-level names the `rename-globals` pass must keep (the externs
    /// value-namespace boundary; empty when no `--externs` were given).
    do_not_rename_globals: std::collections::HashSet<String>,
    /// When `Some`, run `rename-properties` gated on this externs
    /// property-namespace boundary. `None` means property renaming stays
    /// OFF — either the user passed no `--externs`, or (fail-closed) an
    /// externs source could not be loaded, so the boundary is untrustworthy
    /// and renaming would risk a miscompile. See
    /// [`collect_externs_property_names`].
    rename_properties_externs: Option<std::collections::HashSet<String>>,
}

/// Run the typed-AST optimization pipeline over a bridged `Program` and
/// emit the result as JavaScript text.
///
/// The caller has already turned source text into a typed [`Program`] via
/// the grammar parser and the bridge; this runs the pass pipeline over it
/// and serialises the optimized tree back to JS with
/// [`closure_emitter::emit`]. Returns `Some(code)` on success and `None`
/// if a pass or the emitter fails — the caller then degrades to
/// `whitespace_only`. Either way it records the outcome in `*status`
/// (`"ok"`, `"pass_error:<e>"`, or `"emit_error:<e>"`) so the
/// correlation-vector trace can distinguish a true optimized emit from a
/// degrade. SIMPLE v2 has no type-inference stage yet, so an empty
/// [`Sidecar`] is passed; the pass-internal [`CVLog`] is disabled because
/// the per-byte trace is emitted by the caller's stage block.
///
/// `advanced` distinguishes the two levels: `None` is SIMPLE; `Some(cfg)`
/// is ADVANCED, which appends `rename-globals` (always) and
/// `rename-properties` (only when `cfg.rename_properties_externs` is
/// `Some`) after the SIMPLE passes. See [`SIMPLE_PASS_NAMES`] /
/// [`ADVANCED_PASS_NAMES`] / [`AdvancedConfig`].
fn run_typed_pipeline(
    program: coding_adventures_javascript_ast::Program,
    status: &mut Option<String>,
    advanced: Option<AdvancedConfig>,
    // CLOC27 P4 (D5): the run's real (enabled) CV log when
    // `--correlation_vector` is on. The constant-fold pass `derive`s each
    // folded literal from its leaf's source CvId against this log, so the
    // sidecar records real per-token provenance for folds. `None` (the
    // default / non-CV path) uses an internal disabled log — unchanged
    // behaviour, and output bytes are identical either way since CV ids
    // never influence folding or emission.
    cv: Option<&mut coding_adventures_correlation_vector::CVLog>,
) -> Option<String> {
    use coding_adventures_closure_emitter::{emit, EmitOptions};
    use coding_adventures_closure_pass_constant_fold::ConstantFoldPass;
    use coding_adventures_closure_pass_dce::DcePass;
    use coding_adventures_closure_pass_fold_control_flow::FoldControlFlowPass;
    use coding_adventures_closure_pass_inline::InlinePass;
    use coding_adventures_closure_pass_inline_variables::InlineVariablesPass;
    use coding_adventures_closure_pass_pipeline::PassPipeline;
    use coding_adventures_closure_pass_remove_unused_vars::RemoveUnusedVarsPass;
    use coding_adventures_closure_pass_rename::RenamePass;
    use coding_adventures_closure_pass_rename_globals::RenameGlobalsPass;
    use coding_adventures_closure_pass_rename_properties::RenamePropertiesPass;
    use coding_adventures_closure_pass_treeshake::TreeshakePass;
    use coding_adventures_correlation_vector::CVLog;
    use coding_adventures_type_sidecar::Sidecar;

    let sidecar = Sidecar::new();
    // CLOC27 P4 (D5): the passes and emitter take a CV log. When CV is on the
    // caller threads the run's REAL (enabled) log here, so the constant-fold
    // pass `derive`s folded literals from their leaf source CvIds and the
    // sidecar gains real per-token fold provenance. When CV is off we fall
    // back to an internal disabled log that accepts contributions and drops
    // them — zero overhead, byte-identical output.
    let mut owned_disabled_cv = CVLog::new(false);
    let pass_cv: &mut CVLog = match cv {
        Some(log) => log,
        None => &mut owned_disabled_cv,
    };

    // The pipeline topo-sorts on each pass's `depends_on`, with
    // registration order as the tie-breaker between independent passes.
    // `remove-unused-vars` declares `depends_on = ["dce", "inline"]`, so
    // `inline` MUST be registered or the scheduler refuses to run
    // remove-unused-vars. `inline` is an identity pass today; it holds
    // the canonical slot until real function inlining lands.
    let mut pipeline = PassPipeline::new();
    pipeline.add(Box::new(ConstantFoldPass::new()));
    pipeline.add(Box::new(FoldControlFlowPass::new()));
    pipeline.add(Box::new(DcePass::new()));
    pipeline.add(Box::new(InlinePass::new()));
    // inline-variables propagates a top-level `const = literal` to its
    // use sites; it runs before remove-unused-vars, which then deletes
    // the now-unreferenced `const` declaration.
    pipeline.add(Box::new(InlineVariablesPass::new()));
    pipeline.add(Box::new(RemoveUnusedVarsPass::new()));
    pipeline.add(Box::new(TreeshakePass::new()));
    // rename runs last among the SIMPLE passes: it shortens leaf-function
    // parameter names after every structural pass has finished
    // removing/rewriting code. It has no dependencies (correct
    // standalone), so registration order places it at the end.
    pipeline.add(Box::new(RenamePass::new()));

    // ADVANCED only. Registered after `rename` so the renamers shorten
    // disjoint name layers (locals → globals → properties).
    if let Some(adv) = advanced {
        // `rename-globals` shortens program-private TOP-LEVEL names, gated
        // by the externs value-namespace boundary (empty keep-set is sound
        // here — top-level renaming holds under the whole-program contract).
        pipeline.add(Box::new(RenameGlobalsPass::new(adv.do_not_rename_globals)));
        // `rename-properties` shortens program-private PROPERTY names, but
        // only when the user opted into the externs contract by supplying
        // `--externs` (the property-namespace boundary). Without it the pass
        // does not run at all — see [`AdvancedConfig`] for why.
        if let Some(prop_externs) = adv.rename_properties_externs {
            pipeline.add(Box::new(RenamePropertiesPass::new(prop_externs)));
        }
    }

    let optimized = match pipeline.run(program, &sidecar, &mut *pass_cv) {
        Ok(out) => out.program,
        Err(e) => {
            *status = Some(format!("pass_error:{e}"));
            return None;
        }
    };

    // Emit minified JS (pretty=false). No source map at this layer —
    // SIMPLE source maps land with the dedicated source-map work.
    let opts = EmitOptions {
        source_map: false,
        ..Default::default()
    };
    match emit(&optimized, &sidecar, &mut *pass_cv, &opts) {
        Ok(out) => {
            *status = Some("ok".to_string());
            Some(out.code)
        }
        Err(e) => {
            *status = Some(format!("emit_error:{e}"));
            None
        }
    }
}

pub fn transform_source_with_cv(
    source: &str,
    config: &CompilerConfig,
    cv: Option<(
        &mut coding_adventures_correlation_vector::CVLog,
        &str,
        // CLOC12.132: per-token CV IDs parallel to the full token
        // array from `tokenize_javascript_typed`. Passed through to
        // `whitespace_only_minify` so gap-rule drops get tombstones.
        // Empty slice is safe: out-of-bounds accesses are silently
        // skipped inside `whitespace_only_minify`.
        &[String],
        // CLOC27 P4 (D5): the source-file display name (input path). On the
        // SIMPLE/ADVANCED typed path with CV on, this is passed to
        // `parse_javascript_typed_with_cv` as the per-token `Origin.source`,
        // so a constant-folded literal can be traced back through the CV log
        // to the source bytes it derived from. Unused on the WHITESPACE_ONLY
        // / degrade paths.
        &str,
    )>,
) -> Result<String, CompilerError> {
    let es_version = map_language_in_to_es_version(config);

    // Hoist `cv` into a local so we can borrow it twice inside
    // the function (once per stage). The Option<&mut> doesn't
    // implement Copy, but we can reborrow at each call site.
    let mut cv_pair = cv;

    // bridge_status is set for the typed-pipeline levels (Simple and
    // Advanced, which share that pipeline) and threaded into the CV
    // contribution below. Other levels leave it None, where the CV block
    // substitutes "n/a".
    let mut simple_bridge_status: Option<String> = None;

    // Decide ADVANCED property renaming ONCE, fail-closed, so the pipeline
    // and the CV trace below agree (they must never disagree — one running
    // the pass while the other omits it from the trace). `Some` means
    // `rename-properties` will run against this externs property boundary;
    // `None` means it will not (SIMPLE, no `--externs`, or — critically — an
    // externs source that failed to load, which DISABLES the pass rather
    // than running it against an empty/partial boundary). See
    // [`collect_externs_property_names`].
    let rename_properties_externs: Option<std::collections::HashSet<String>> =
        if matches!(config.compilation.level, CompilationLevel::Advanced)
            && !config.io.externs.is_empty()
        {
            collect_externs_property_names(config)
        } else {
            None
        };
    let will_rename_properties = rename_properties_externs.is_some();

    // Step 1 — compilation-level transform.
    let after_level = match config.compilation.level {
        CompilationLevel::WhitespaceOnly => {
            // CLOC12.132: thread token_cv_ids into whitespace_only_minify
            // as a re-borrow so the log and file CV ID remain available
            // for the per-stage contribution block below.
            let wo_cv = cv_pair.as_mut().map(|(log, id, ids, _file)| {
                (
                    *log as &mut coding_adventures_correlation_vector::CVLog,
                    *id,
                    *ids,
                )
            });
            whitespace_only::whitespace_only_minify(source, es_version, wo_cv)
                .map_err(CompilerError::Minify)?
        }
        // CLOC12.155: SIMPLE runs the typed-AST optimization pipeline (v2).
        //
        // The pipeline is:
        //
        //     source ──parse──▶ grammar AST ──bridge──▶ typed Program
        //            ──passes──▶ optimized Program ──emit──▶ JS text
        //
        // In v2 the pass pipeline holds a single pass — `constant-fold`
        // (e.g. `1 + 2` ⇒ `3`). Follow-up PRs append the remaining
        // SIMPLE-appropriate passes (fold-control-flow, dce,
        // remove-unused-vars, local inline/rename), one pass per PR.
        //
        // Degrade policy — the typed path is best-effort. If ANY stage
        // fails (the grammar parser rejects the source, the bridge hits
        // a Phase-2+ construct, a pass errors, or the emitter cannot
        // serialise a node), we fall back to `whitespace_only` so the
        // compiler never errors on valid-but-not-yet-supported input.
        // The one exception is `BridgeError::InternalError`, which
        // signals a broken bridge invariant rather than an unsupported
        // construct — that propagates as `CompilerError::Bridge`.
        //
        // `simple_bridge_status` records which branch we took so the
        // correlation-vector trace can show whether the run was a true
        // optimized emit (`"ok"`) or a degrade (`"parse_error:…"`,
        // `"unsupported_syntax:…"`, `"pass_error:…"`, `"emit_error:…"`).
        // ADVANCED currently runs the *same* typed optimization pipeline
        // as SIMPLE. It is specified to be at least as aggressive as
        // SIMPLE, so reusing the SIMPLE pipeline is a correct lower bound
        // (and removes the former literal no-op, where ADVANCED returned
        // the source verbatim). Advanced-only passes — aggressive
        // property/global renaming, cross-module tree-shaking — layer on
        // here as they are implemented.
        CompilationLevel::Simple | CompilationLevel::Advanced => {
            // CLOC27 P4 (D5): when --correlation_vector is on (the `cv` tuple
            // is present), parse via `parse_javascript_typed_with_cv` so every
            // token carries its source CvId into the bridge, where the leaf
            // factory stamps it onto each literal (CLOC27 P2/P3). The 4th tuple
            // element is the input file's display name, used as the per-token
            // `Origin.source`. The non-CV path keeps the zero-overhead
            // `parse_javascript_typed` and is byte-identical to before.
            let parse_result = match cv_pair.as_mut() {
                Some((log, _id, _ids, file)) => {
                    coding_adventures_javascript_parser::parse_javascript_typed_with_cv(
                        source, file, es_version, log,
                    )
                }
                None => parse_javascript_typed(source, es_version),
            };
            // Attempt the typed optimization path. `Some(code)` means
            // the full parse→bridge→passes→emit chain succeeded;
            // `None` means we should degrade to whitespace_only.
            let optimized: Option<String> = match parse_result {
                Err(parse_err) => {
                    // Malformed JS — grammar parser rejected it.
                    // Degrade; whitespace_only surfaces the real error.
                    simple_bridge_status = Some(format!("parse_error:{parse_err}"));
                    None
                }
                Ok(node) => match bridge::grammar_to_program(&node, es_version) {
                    Ok(program) => {
                        // ADVANCED adds aggressive renaming on top of the
                        // SIMPLE pipeline. `rename-globals` runs always
                        // (gated by the externs value boundary), and
                        // `rename-properties` runs only with the (fail-closed)
                        // property boundary decided above — SIMPLE runs the
                        // typed pipeline unchanged.
                        let advanced = match config.compilation.level {
                            CompilationLevel::Advanced => Some(AdvancedConfig {
                                do_not_rename_globals: externs_do_not_rename(config),
                                rename_properties_externs,
                            }),
                            _ => None,
                        };
                        // CLOC27 P4 (D5): run the pass pipeline against the
                        // run's REAL (enabled) CV log when CV is on, so the
                        // constant-fold pass `derive`s each folded literal from
                        // its leaf's source CvId — landing real per-token
                        // provenance in the sidecar. With CV off this is `None`
                        // and the pipeline uses an internal disabled log
                        // (unchanged; output bytes are identical either way,
                        // since CV ids never affect folding or emission).
                        let pipe_cv = cv_pair.as_mut().map(|(log, _id, _ids, _file)| {
                            *log as &mut coding_adventures_correlation_vector::CVLog
                        });
                        run_typed_pipeline(
                            program,
                            &mut simple_bridge_status,
                            advanced,
                            pipe_cv,
                        )
                    }
                    Err(BridgeError::UnsupportedSyntax { rule, location }) => {
                        simple_bridge_status =
                            Some(format!("unsupported_syntax:{rule}@{location}"));
                        None
                    }
                    Err(BridgeError::InternalError { msg, rule }) => {
                        return Err(CompilerError::Bridge(format!("{rule}: {msg}")));
                    }
                },
            };

            match optimized {
                Some(code) => code,
                None => {
                    // Degrade path: emit via whitespace_only.
                    let wo_cv = cv_pair.as_mut().map(|(log, id, ids, _file)| {
                        (
                            *log as &mut coding_adventures_correlation_vector::CVLog,
                            *id,
                            *ids,
                        )
                    });
                    whitespace_only::whitespace_only_minify(source, es_version, wo_cv)
                        .map_err(CompilerError::Minify)?
                }
            }
        }
        // Bundle / TranspileOnly: identity for now (module bundling and
        // language down-levelling are orthogonal to the optimization
        // pipeline and land separately).
        CompilationLevel::Bundle | CompilationLevel::TranspileOnly => source.to_string(),
    };

    if let Some((log, cv_id, _token_ids, _file)) = cv_pair.as_mut() {
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
                // SIMPLE and ADVANCED share the typed optimization
                // pipeline, so they share this contribution shape. The
                // tag and `level` distinguish them; ADVANCED runs the same
                // passes today (it is ≥ SIMPLE) and gains advanced-only
                // passes here as they land.
                CompilationLevel::Simple | CompilationLevel::Advanced => {
                    // The pass list is dynamic for ADVANCED: `rename-properties`
                    // appears in the trace exactly when it actually ran —
                    // `will_rename_properties` is the SAME fail-closed decision
                    // that gated the pipeline above, so the trace can never
                    // disagree with what executed.
                    let (tag, level, passes_list): (&str, &str, Vec<&str>) =
                        match config.compilation.level {
                            CompilationLevel::Advanced => {
                                let mut passes: Vec<&str> = ADVANCED_PASS_NAMES.to_vec();
                                if will_rename_properties {
                                    passes.push("rename-properties");
                                }
                                ("advanced_v1", "ADVANCED", passes)
                            }
                            _ => ("simple_v2", "SIMPLE", SIMPLE_PASS_NAMES.to_vec()),
                        };
                    (
                        tag,
                        vec![
                            ("level", serde_json::Value::String(level.into())),
                            (
                                "bridge_status",
                                serde_json::Value::String(
                                    simple_bridge_status.as_deref().unwrap_or("n/a").into(),
                                ),
                            ),
                            // The optimization passes that ran (in order).
                            // A degrade (`bridge_status != "ok"`) still
                            // lists them — they were the *intended*
                            // pipeline even when the run fell back to
                            // whitespace_only.
                            (
                                "passes",
                                serde_json::Value::Array(
                                    passes_list
                                        .iter()
                                        .map(|p| serde_json::Value::String((*p).into()))
                                        .collect(),
                                ),
                            ),
                            (
                                "input_byte_len",
                                serde_json::Value::Number((source.len() as u64).into()),
                            ),
                            (
                                "output_byte_len",
                                serde_json::Value::Number((after_level.len() as u64).into()),
                            ),
                        ],
                    )
                }
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

    if let Some((log, cv_id, _token_ids, _file)) = cv_pair.as_mut() {
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

/// Collect the externs **do-not-rename** set — the top-level names
/// declared in the `--externs` files. These are the program's external
/// boundary: ADVANCED's `rename-globals` pass renames every other
/// top-level name but must keep these (they're referenced from outside
/// this compilation). Returns the union of every externs file's top-level
/// `function` ids and `var`/`let`/`const` target names.
///
/// **Degrade-safe.** A glob failure, an unreadable file, a parse error,
/// or a bridge rejection on any externs file simply contributes no names
/// from that file rather than failing the compile — the same best-effort
/// posture the typed pipeline uses on the main input. (A genuine
/// bad-glob is surfaced earlier, by `resolve_externs` in `run_compiler`.)
fn externs_do_not_rename(config: &CompilerConfig) -> std::collections::HashSet<String> {
    use coding_adventures_javascript_ast::{BindingTarget, Declaration, ProgramItem, Statement};

    let mut names = std::collections::HashSet::new();
    let paths = match resolve_externs(config) {
        Ok(p) => p,
        Err(_) => return names,
    };
    let es = map_language_in_to_es_version(config);
    for path in paths {
        let src = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let node = match parse_javascript_typed(&src, es) {
            Ok(n) => n,
            Err(_) => continue,
        };
        let program = match bridge::grammar_to_program(&node, es) {
            Ok(p) => p,
            Err(_) => continue,
        };
        for item in &program.body {
            match item {
                ProgramItem::Declaration(Declaration::FunctionDeclaration(fd))
                | ProgramItem::Statement(Statement::Declaration(
                    Declaration::FunctionDeclaration(fd),
                )) => {
                    names.insert(fd.id.name.clone());
                }
                ProgramItem::Declaration(Declaration::VariableDeclaration(vd))
                | ProgramItem::Statement(Statement::Declaration(
                    Declaration::VariableDeclaration(vd),
                )) => {
                    for d in &vd.declarations {
                        let BindingTarget::Identifier(id) = &d.id;
                        names.insert(id.name.clone());
                    }
                }
                _ => {}
            }
        }
    }
    names
}

/// Collect the externs **property** do-not-rename set — every property
/// name mentioned in the `--externs` files. This is the *property*
/// namespace twin of [`externs_do_not_rename`] (which collects top-level
/// *variable* names). ADVANCED's `rename-properties` pass renames every
/// other program-private property but must keep these (they are the
/// host/library surface — `innerHTML`, `addEventListener`, …).
///
/// The actual walk lives in the rename-properties crate
/// (`collect_property_names`) so there is one whole-program property
/// traversal to keep in sync, reused here and by the pass itself.
///
/// # Fail-closed, NOT degrade-safe
///
/// Unlike [`externs_do_not_rename`] (whose pass, `rename-globals`, runs at
/// ADVANCED regardless and is sound with an empty keep-set under the
/// whole-program contract), `rename-properties` is sound **only because the
/// user declared the external property boundary via `--externs`**. So the
/// boundary's *contents* — not the mere presence of the flag — are what
/// make renaming safe. If any externs source fails to resolve, read,
/// parse, or bridge, we return [`None`], and the caller **disables property
/// renaming entirely** for this run. Silently contributing a partial (or
/// empty) boundary would let an externally-observable property be renamed —
/// a miscompile of valid input, in precisely the case the user opted into
/// safety. A typo'd path, a permission error, or an externs file using
/// syntax the Phase-1 parser rejects must turn the pass OFF, never run it
/// against a shrunken boundary.
///
/// Returns `Some(set)` only when EVERY resolved externs file successfully
/// contributed its property names (the set may legitimately be empty if the
/// externs files mention no properties). Callers only invoke this when
/// `config.io.externs` is non-empty.
fn collect_externs_property_names(
    config: &CompilerConfig,
) -> Option<std::collections::HashSet<String>> {
    use coding_adventures_closure_pass_rename_properties::collect_property_names;

    // A glob failure here would already have been surfaced by
    // `resolve_externs` in `run_compiler`; treat it as fail-closed anyway.
    let paths = resolve_externs(config).ok()?;
    let es = map_language_in_to_es_version(config);
    let mut names = std::collections::HashSet::new();
    for path in paths {
        // Any read/parse/bridge failure disables the pass (`?` → None) — a
        // partial boundary is unsound, so we refuse to optimize rather than
        // risk renaming an external property.
        let src = fs::read_to_string(&path).ok()?;
        let node = parse_javascript_typed(&src, es).ok()?;
        let program = bridge::grammar_to_program(&node, es).ok()?;
        names.extend(collect_property_names(&program));
    }
    Some(names)
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
            stderr_text: String::new(),
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
            stderr_text: String::new(),
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
            stderr_text: String::new(),
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
    // CLOC11.62: per-file CV IDs accumulate so the post-loop
    // stages (wrapper / IIFE / charset / etc.) can derive a
    // single "combined" CV entry with all of them as parents.
    // That combined entry is the substrate the rest of the
    // pipeline contributes against — every byte from any input
    // gets its post-combine provenance recorded there.
    let mut per_file_cv_ids: Vec<String> = Vec::new();
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

        // CLOC11.64: per-token CV entries. With CV enabled we
        // tokenize the file and create a CV entry per token that
        // is a *child* (via `derive`) of the per-file CV entry.
        // This lays the substrate later slices need to migrate
        // token-level contributions (defines.applied,
        // whitespace_only drops, rename mappings…) off the
        // per-file CV and onto the precise token CV they touched.
        //
        // The per-file CV is left in place: it remains the
        // attach-point for stage-level summaries
        // (compilation_level, defines) and the merge-parent for
        // the combined post-concat CV. Tokens add detail; they
        // don't replace the file root.
        //
        // Granularity (intentional this slice):
        //   - one child entry per token from
        //     `tokenize_javascript_typed` — comments and pure
        //     whitespace are not emitted as tokens by the
        //     lexer, so we get exactly the bytes that survive
        //     into the token stream.
        //   - Origin.source = "lexer_token", location =
        //     "<path>:<line>:<col>" (1-based, matching the
        //     lexer's own line/column).
        //   - meta = {kind, lexeme_byte_len, token_index} —
        //     kind is the TokenType as a stable lowercase
        //     string; lexeme_byte_len is value.len() (post
        //     escape resolution for strings — matches what the
        //     emitter would write); token_index is the 0-based
        //     position in the stream so later passes can refer
        //     back without keeping the token vec around.
        //
        // Errors: a lex failure (malformed JS) does not abort
        // the build. The string-only transform pipeline can
        // still handle WHITESPACE_ONLY style copies; we just
        // record a `lex.failed` contribution on the per-file
        // CV and skip token-CV creation. Later slices that need
        // tokens may treat absence as "lex didn't reach this
        // file" — a recoverable state.
        //
        // CLOC12.132: declare token_cv_ids outside the lex block so
        // it can be passed to transform_source_with_cv (which threads
        // it into whitespace_only_minify for gap-drop tombstones).
        // Stays empty on lex failure or when CV is off — both callers
        // guard on cv_id / token_cv_ids being populated before using
        // the indices.
        let mut token_cv_ids: Vec<String> = Vec::new();
        if let Some(id) = &cv_id {
            let es = map_language_in_to_es_version(config);
            match tokenize_javascript_typed(&contents, es) {
                Ok(tokens) => {
                    let token_count = tokens.len();
                    // CLOC11.65: capture each derived token CV ID
                    // so that *after* the lex loop we can attach
                    // token-level contributions (defines.applied
                    // on the specific Name token a --define
                    // substituted) onto the precise child CV
                    // rather than smearing them on the per-file
                    // root.
                    token_cv_ids = Vec::with_capacity(token_count);
                    for (idx, tok) in tokens.iter().enumerate() {
                        let mut tmeta = std::collections::HashMap::new();
                        tmeta.insert(
                            "kind".to_string(),
                            serde_json::Value::String(
                                format!("{:?}", tok.type_).to_lowercase(),
                            ),
                        );
                        tmeta.insert(
                            "lexeme_byte_len".to_string(),
                            serde_json::Value::Number(
                                (tok.value.len() as u64).into(),
                            ),
                        );
                        tmeta.insert(
                            "token_index".to_string(),
                            serde_json::Value::Number((idx as u64).into()),
                        );
                        let tok_cv = cv_log.derive(
                            id,
                            Some(coding_adventures_correlation_vector::Origin {
                                source: "lexer_token".to_string(),
                                location: format!(
                                    "{}:{}:{}",
                                    path.to_string_lossy(),
                                    tok.line,
                                    tok.column,
                                ),
                                timestamp: None,
                                meta: tmeta,
                            }),
                        );
                        token_cv_ids.push(tok_cv);
                    }
                    let mut cmeta = std::collections::HashMap::new();
                    cmeta.insert(
                        "token_count".to_string(),
                        serde_json::Value::Number((token_count as u64).into()),
                    );
                    let _ = cv_log.contribute(
                        id, "lex", "tokens_emitted", cmeta,
                    );

                    // CLOC11.65: per-token `defines.applied`.
                    // Walk the token stream; whenever a Name
                    // token's lexeme matches a key in
                    // `config.defines.defines`, contribute
                    // `defines.applied` on THAT token's CV with
                    // {define_name, define_value, define_value_kind}.
                    //
                    // Why on the token CV and not the file CV:
                    // upstream Closure substitutes per occurrence,
                    // so the trace should record per occurrence.
                    // The per-file `defines.applied` summary
                    // contribution still fires later (preserves
                    // the "stage ran" signal for zero-defines
                    // runs) — the per-token records are *in
                    // addition*, giving precise byte-level
                    // provenance.
                    //
                    // Caveats this slice does not handle:
                    //   - defines inside string literals (the
                    //     current string-level apply_defines
                    //     happens to skip them too, but only by
                    //     accident of regex; we just look at
                    //     Name tokens which already excludes
                    //     strings).
                    //   - shorthand object syntax (`{ FOO }`),
                    //     where renaming would change semantics
                    //     — same caveat as the existing
                    //     string-level pass.
                    if !config.defines.defines.is_empty() {
                        for (idx, tok) in tokens.iter().enumerate() {
                            if tok.type_ != TokenType::Name {
                                continue;
                            }
                            let Some(def_value) =
                                config.defines.defines.get(&tok.value)
                            else {
                                continue;
                            };
                            let (val_json, val_kind) = match def_value {
                                crate::config::DefineValue::Bool(b) => (
                                    serde_json::Value::Bool(*b),
                                    "bool",
                                ),
                                crate::config::DefineValue::Number(n) => (
                                    serde_json::Number::from_f64(*n)
                                        .map(serde_json::Value::Number)
                                        .unwrap_or(serde_json::Value::Null),
                                    "number",
                                ),
                                crate::config::DefineValue::String(s) => (
                                    serde_json::Value::String(s.clone()),
                                    "string",
                                ),
                                crate::config::DefineValue::Null => (
                                    serde_json::Value::Null,
                                    "null",
                                ),
                            };
                            let mut dmeta = std::collections::HashMap::new();
                            dmeta.insert(
                                "define_name".to_string(),
                                serde_json::Value::String(tok.value.clone()),
                            );
                            dmeta.insert(
                                "define_value".to_string(),
                                val_json,
                            );
                            dmeta.insert(
                                "define_value_kind".to_string(),
                                serde_json::Value::String(val_kind.into()),
                            );
                            dmeta.insert(
                                "token_index".to_string(),
                                serde_json::Value::Number((idx as u64).into()),
                            );
                            // token_cv_ids[idx] is valid: same
                            // length and order as tokens (we
                            // built it in lock-step above).
                            let _ = cv_log.contribute(
                                &token_cv_ids[idx],
                                "defines",
                                "applied",
                                dmeta,
                            );
                        }
                    }

                    // CLOC11.66 — WHITESPACE_ONLY token
                    // tombstones. Only the WHITESPACE_ONLY
                    // compilation level filters tokens at this
                    // stage (`whitespace_only_minify` drops
                    // trivia and EOF). For every token CV that
                    // matches the same trivia / EOF predicate
                    // the minifier uses, record a
                    // DeletionRecord (tombstone) on the token
                    // CV so the trace shows precisely which
                    // bytes the pass killed.
                    //
                    // Why not lex twice (pre + post) and diff
                    // the streams? The trivia / EOF predicate
                    // *is* the contract — `is_trivia` and
                    // `is_eof` in `whitespace_only.rs` are
                    // what the minifier itself uses. Reusing
                    // them keeps the tombstone set guaranteed
                    // identical to the dropped set without a
                    // second lex pass.
                    //
                    // Other compilation levels (SIMPLE,
                    // ADVANCED, BUNDLE, TRANSPILE_ONLY) are
                    // currently identity on the string —
                    // they don't drop any tokens — so no
                    // tombstones land for those. As those
                    // levels grow real bodies in later
                    // CLOC11.* slices, each will need its own
                    // tombstone block.
                    if matches!(
                        config.compilation.level,
                        crate::config::CompilationLevel::WhitespaceOnly,
                    ) {
                        for (idx, tok) in tokens.iter().enumerate() {
                            let is_trivia_tok = crate::whitespace_only::is_trivia(tok);
                            let is_eof_tok = crate::whitespace_only::is_eof(tok);
                            if !is_trivia_tok && !is_eof_tok {
                                continue;
                            }
                            let mut wmeta = std::collections::HashMap::new();
                            wmeta.insert(
                                "kind".to_string(),
                                serde_json::Value::String(
                                    if is_eof_tok {
                                        "eof".into()
                                    } else {
                                        "trivia".into()
                                    },
                                ),
                            );
                            wmeta.insert(
                                "token_index".to_string(),
                                serde_json::Value::Number((idx as u64).into()),
                            );
                            wmeta.insert(
                                "token_lexeme_byte_len".to_string(),
                                serde_json::Value::Number(
                                    (tok.value.len() as u64).into(),
                                ),
                            );
                            cv_log.delete(
                                &token_cv_ids[idx],
                                "compilation_level",
                                "whitespace_only_dropped",
                                wmeta,
                            );
                        }
                    }
                }
                Err(err) => {
                    let mut emeta = std::collections::HashMap::new();
                    emeta.insert(
                        "message".to_string(),
                        serde_json::Value::String(err),
                    );
                    let _ = cv_log.contribute(
                        id, "lex", "failed", emeta,
                    );
                }
            }
        }

        // CLOC11.61: per-stage CV contributions. When CV is on,
        // pass the per-file cv_id through to `transform_source_with_cv`
        // so the transform records one record per stage
        // (compilation_level + defines) rather than the single
        // summary contribution from CLOC11.60.
        //
        // CLOC12.132: also pass token_cv_ids so whitespace_only_minify
        // can tombstone gap-rule-dropped tokens (hoisted above the lex
        // block; empty if lex failed, which is safe — whitespace_only
        // skips out-of-bounds indices).
        // CLOC27 P4 (D5): the input file's display name becomes the per-token
        // `Origin.source` when the SIMPLE/ADVANCED typed path parses with CV on,
        // so a folded literal traces back to the file (and line:col) it derived
        // from. Bound here so it outlives the borrow inside the call.
        let path_display = path.to_string_lossy();
        let transformed = match &cv_id {
            Some(id) => transform_source_with_cv(
                &contents,
                config,
                Some((&mut cv_log, id.as_str(), &token_cv_ids, path_display.as_ref())),
            )?,
            None => transform_source(&contents, config)?,
        };

        if let Some(id) = cv_id {
            per_file_cv_ids.push(id);
        }

        combined.push_str(&transformed);
        // Closure separates concatenated inputs with a newline so
        // back-to-back files don't end up syntactically merged.
        if !transformed.ends_with('\n') {
            combined.push('\n');
        }
    }

    // CLOC11.62: derive the post-concat "combined" CV entry.
    //
    // After the per-file loop, every subsequent stage operates on
    // the concatenated `combined` string, not on individual
    // files. The CV trace needs an entity that represents that
    // post-concat substrate — otherwise contributions from
    // `--emit_use_strict`, `--output_wrapper`, IIFE, `--charset`
    // would have nowhere to attach.
    //
    // We use `CVLog::merge()` with the per-file CV IDs as parents,
    // so a downstream consumer following an output-byte's
    // provenance walks: combined-entry → all source files. The
    // origin is synthetic ("concatenated_combined_source"); no
    // location since it's not a file on disk.
    let combined_cv_id = if config.special_modes.correlation_vector
        && !per_file_cv_ids.is_empty()
    {
        let parent_refs: Vec<&str> =
            per_file_cv_ids.iter().map(|s| s.as_str()).collect();
        let mut meta = std::collections::HashMap::new();
        meta.insert(
            "file_count".to_string(),
            serde_json::Value::Number((per_file_cv_ids.len() as u64).into()),
        );
        meta.insert(
            "byte_len".to_string(),
            serde_json::Value::Number((combined.len() as u64).into()),
        );
        Some(cv_log.merge(
            &parent_refs,
            Some(coding_adventures_correlation_vector::Origin {
                source: "concatenated_combined_source".to_string(),
                location: String::new(),
                timestamp: None,
                meta,
            }),
        ))
    } else {
        None
    };

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
            stderr_text: String::new(),
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
        // CLOC11.62 — record the emit_use_strict contribution
        // on the combined entry. The directive is a fixed-size
        // 16-byte prepend (`"use strict";\n`), so meta carries
        // the input/output byte lengths.
        if let Some(id) = &combined_cv_id {
            let mut meta = std::collections::HashMap::new();
            meta.insert(
                "input_byte_len".to_string(),
                serde_json::Value::Number((combined.len() as u64).into()),
            );
            meta.insert(
                "output_byte_len".to_string(),
                serde_json::Value::Number(
                    ((combined.len() + 16) as u64).into(),
                ),
            );
            let _ = cv_log.contribute(
                id,
                "emit_use_strict",
                "prepended",
                meta,
            );
        }
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

    // CLOC11.62 — record the output_wrapper contribution.
    //
    // We only emit a contribution when the wrapper actually
    // changed the bytes (`wrapped != combined`). Skipping the
    // contribution for the no-wrapper passthrough avoids
    // spurious entries that say "this pass ran and did
    // nothing" — the CV trace stays focused on bytes that
    // moved.
    if let Some(id) = &combined_cv_id {
        if wrapped != combined {
            let mut meta = std::collections::HashMap::new();
            meta.insert(
                "input_byte_len".to_string(),
                serde_json::Value::Number((combined.len() as u64).into()),
            );
            meta.insert(
                "output_byte_len".to_string(),
                serde_json::Value::Number((wrapped.len() as u64).into()),
            );
            let _ = cv_log.contribute(
                id,
                "output_wrapper",
                "substituted",
                meta,
            );
        }
    }

    // Step 3.5 (CLOC11.31): apply --isolation_mode IIFE if set.
    // Layered after the user wrapper, matching CC's pipeline:
    // user wrapper runs first, IIFE wraps the result. So the
    // user's banner sits *inside* the IIFE — same semantics as
    // CC, and the behavior users requesting IIFE expect.
    let isolated = match config.formatting.isolation_mode {
        IsolationMode::Iife => wrapper::apply_iife_wrap(&wrapped),
        IsolationMode::None => wrapped.clone(),
    };

    // CLOC11.62 — record the isolation_mode contribution.
    // Only fires when IIFE is on; the None case is a pass-through
    // that doesn't change bytes.
    if let Some(id) = &combined_cv_id {
        if config.formatting.isolation_mode == IsolationMode::Iife {
            let mut meta = std::collections::HashMap::new();
            meta.insert(
                "input_byte_len".to_string(),
                serde_json::Value::Number((wrapped.len() as u64).into()),
            );
            meta.insert(
                "output_byte_len".to_string(),
                serde_json::Value::Number((isolated.len() as u64).into()),
            );
            let _ = cv_log.contribute(
                id,
                "isolation_mode",
                "iife_wrapped",
                meta,
            );
        }
    }

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
    let charset_mode = crate::charset::OutputCharset::from_raw(&config.io.charset);
    let encoded = crate::charset::apply_charset(&isolated, charset_mode);

    // CLOC11.62 — record the charset contribution.
    //
    // The contribution lands even when the mode is UTF-8 (pass-
    // through) because the stage RAN — same symmetry argument as
    // CLOC11.61's defines.applied. Meta carries the resolved
    // mode name and byte deltas so a viewer can show whether
    // escapes were emitted.
    if let Some(id) = &combined_cv_id {
        let mut meta = std::collections::HashMap::new();
        meta.insert(
            "mode".to_string(),
            serde_json::Value::String(
                match charset_mode {
                    crate::charset::OutputCharset::UsAscii => "US_ASCII",
                    crate::charset::OutputCharset::Utf8 => "UTF-8",
                }
                .to_string(),
            ),
        );
        meta.insert(
            "input_byte_len".to_string(),
            serde_json::Value::Number((isolated.len() as u64).into()),
        );
        meta.insert(
            "output_byte_len".to_string(),
            serde_json::Value::Number((encoded.len() as u64).into()),
        );
        let _ = cv_log.contribute(id, "charset", "normalized", meta);
    }

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
    //
    // CLOC11.63: capture `encoded.len()` BEFORE the match so the
    // downstream CV `js_output_file` record can include it. The
    // None arm moves `encoded` into stdout_text; without the
    // pre-capture we'd see a borrow-of-moved error.
    let encoded_byte_len = encoded.len();
    // CLOC11.76 — when --correlation_vector_summary_only is
    // set, skip the JS write entirely. The CV log still
    // accumulates per-file / combined / post-combine
    // records; we just don't produce any build artifacts.
    // The js_output_file CV record below is also skipped
    // (it would describe a write that never happened).
    let mut result = if config.special_modes.correlation_vector_summary_only {
        CompilerOutput::default()
    } else {
        match &config.io.js_output_file {
            Some(path) => {
                write_output_file(path, &encoded)?;
                CompilerOutput {
                    stdout_text: String::new(),
                    stderr_text: String::new(),
                    wrote_files: vec![path.clone()],
                }
            }
            None => CompilerOutput {
                stdout_text: encoded,
                stderr_text: String::new(),
                wrote_files: Vec::new(),
            },
        }
    };

    // Step 5 (CLOC11.42): source map write.
    let mut wrote_files = result.wrote_files;

    // CLOC11.63: when CV is on AND the JS write actually went to
    // disk, derive a `js_output_file` CV entry with the combined
    // entry as parent. Contributes a `wrote` record with the
    // path + byte_len so a CV trace consumer can match an output
    // file back to its substrate.
    //
    // CLOC11.76: skip this CV record under summary_only — the
    // write never happened, so the trace should not pretend
    // there's a file on disk.
    if !config.special_modes.correlation_vector_summary_only {
    if let Some(parent_id) = &combined_cv_id {
        if let Some(js_path) = &config.io.js_output_file {
            let mut origin_meta = std::collections::HashMap::new();
            origin_meta.insert(
                "path".to_string(),
                serde_json::Value::String(
                    js_path.to_string_lossy().into_owned(),
                ),
            );
            let js_cv_id = cv_log.derive(
                parent_id,
                Some(coding_adventures_correlation_vector::Origin {
                    source: "js_output_file".to_string(),
                    location: js_path.to_string_lossy().into_owned(),
                    timestamp: None,
                    meta: origin_meta,
                }),
            );
            let mut contrib_meta = std::collections::HashMap::new();
            contrib_meta.insert(
                "byte_len".to_string(),
                serde_json::Value::Number((encoded_byte_len as u64).into()),
            );
            let _ = cv_log.contribute(
                &js_cv_id,
                "write_output_file",
                "wrote",
                contrib_meta,
            );
        }
    }
    } // end CLOC11.76 summary_only gate around js CV record

    // CLOC11.76: source map write also skipped under
    // summary_only.
    if !config.special_modes.correlation_vector_summary_only
        && !config.source_map.path_template.is_empty() {
        let map_path = std::path::PathBuf::from(&config.source_map.path_template);
        let map_body = crate::source_map::format_minimal_v3(
            config.io.js_output_file.as_deref(),
        );
        write_output_file(&map_path, &map_body)?;
        wrote_files.push(map_path.clone());

        // CLOC11.63: source map derives from the combined entry
        // (the JS it maps points back to combined). Contribute
        // a `wrote` record with byte_len of the v3 JSON.
        if let Some(parent_id) = &combined_cv_id {
            let mut origin_meta = std::collections::HashMap::new();
            origin_meta.insert(
                "path".to_string(),
                serde_json::Value::String(
                    map_path.to_string_lossy().into_owned(),
                ),
            );
            let map_cv_id = cv_log.derive(
                parent_id,
                Some(coding_adventures_correlation_vector::Origin {
                    source: "source_map_output".to_string(),
                    location: map_path.to_string_lossy().into_owned(),
                    timestamp: None,
                    meta: origin_meta,
                }),
            );
            let mut contrib_meta = std::collections::HashMap::new();
            contrib_meta.insert(
                "byte_len".to_string(),
                serde_json::Value::Number((map_body.len() as u64).into()),
            );
            let _ = cv_log.contribute(
                &map_cv_id,
                "write_output_file",
                "wrote",
                contrib_meta,
            );
        }
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
    // CLOC11.76: manifest write also skipped under summary_only.
    if !config.special_modes.correlation_vector_summary_only {
    if let Some(manifest_path) = &config.chunks.output_manifest_file {
        let body = format_manifest(&inputs);
        write_output_file(manifest_path, &body)?;
        wrote_files.push(manifest_path.clone());

        // CLOC11.63: manifest derives from the *per-file* CVs,
        // not the combined entry — the manifest enumerates input
        // files, not the merged output. Use `merge()` with the
        // per-file IDs as parents.
        if config.special_modes.correlation_vector
            && !per_file_cv_ids.is_empty()
        {
            let parent_refs: Vec<&str> =
                per_file_cv_ids.iter().map(|s| s.as_str()).collect();
            let mut origin_meta = std::collections::HashMap::new();
            origin_meta.insert(
                "path".to_string(),
                serde_json::Value::String(
                    manifest_path.to_string_lossy().into_owned(),
                ),
            );
            origin_meta.insert(
                "file_count".to_string(),
                serde_json::Value::Number(
                    (per_file_cv_ids.len() as u64).into(),
                ),
            );
            let manifest_cv_id = cv_log.merge(
                &parent_refs,
                Some(coding_adventures_correlation_vector::Origin {
                    source: "manifest_output".to_string(),
                    location: manifest_path.to_string_lossy().into_owned(),
                    timestamp: None,
                    meta: origin_meta,
                }),
            );
            let mut contrib_meta = std::collections::HashMap::new();
            contrib_meta.insert(
                "byte_len".to_string(),
                serde_json::Value::Number((body.len() as u64).into()),
            );
            let _ = cv_log.contribute(
                &manifest_cv_id,
                "write_output_file",
                "wrote",
                contrib_meta,
            );
        }
    }
    } // end CLOC11.76 summary_only gate around manifest write

    // Step 7 (CLOC11.60): write the correlation-vector trace as
    // a JSON sidecar file when `--correlation_vector` was set.
    //
    // Path policy (CLOC11.67):
    //   1. If `--correlation_vector_output <path>` is set,
    //      honor it verbatim. Highest precedence — callers
    //      who want a custom location (CI artifact dir,
    //      tmpfs, /dev/null for benchmarks) get exactly that
    //      path with no decoration.
    //   2. Else if `--js_output_file` is set, the sidecar sits
    //      beside it as `<output>.cv.json`. Build pipelines
    //      consuming the JS get the trace automatically
    //      without an extra flag.
    //   3. Else (stdout output), the sidecar lands at
    //      `closurec-cv.json` in the working directory.
    //      Discoverable; user can rename / move.
    // Track whether we actually wrote a sidecar (vs. NONE
    // format that skipped) and where, so CLOC11.73 can name
    // the file in the summary line.
    let mut cv_sidecar_written: Option<PathBuf> = None;
    if config.special_modes.correlation_vector {
        // CLOC11.69 — format selection. `None` short-circuits
        // the whole write step: the CV log is still computed
        // (the rest of the function ran) but nothing hits disk.
        // Useful for benchmarks measuring CV compute overhead
        // vs CV write overhead in isolation.
        use crate::config::CorrelationVectorFormat;
        match config.special_modes.correlation_vector_format {
            CorrelationVectorFormat::None => {
                // explicit no-op
            }
            fmt @ (CorrelationVectorFormat::Json
            | CorrelationVectorFormat::Ndjson) => {
                let sidecar_path = match &config.special_modes.correlation_vector_output {
                    // CLOC11.67 — explicit override wins.
                    Some(p) => p.clone(),
                    None => match &config.io.js_output_file {
                        Some(p) => {
                            let mut s = p.as_os_str().to_owned();
                            s.push(".cv.json");
                            PathBuf::from(s)
                        }
                        None => PathBuf::from("closurec-cv.json"),
                    },
                };
                let body = match fmt {
                    CorrelationVectorFormat::Ndjson => format_cv_log_ndjson(
                        &cv_log,
                        &config.special_modes.correlation_vector_filter,
                        config.special_modes.correlation_vector_filter_includes_origin,
                        config.special_modes.correlation_vector_filter_invert,
                    ),
                    _ => format_cv_log_json(
                        &cv_log,
                        config.special_modes.correlation_vector_pretty,
                        &config.special_modes.correlation_vector_filter,
                        config.special_modes.correlation_vector_filter_includes_origin,
                        config.special_modes.correlation_vector_filter_invert,
                    ),
                };
                write_output_file(&sidecar_path, &body)?;
                wrote_files.push(sidecar_path.clone());
                cv_sidecar_written = Some(sidecar_path);
            }
        }

        // CLOC11.73 — opt-in stdout summary. Computed
        // post-filter so the line describes what's actually
        // on disk (or would have been, under NONE format).
        if config.special_modes.correlation_vector_summary {
            let summary = compute_cv_summary(
                &cv_log,
                &config.special_modes.correlation_vector_filter,
                config.special_modes.correlation_vector_filter_includes_origin,
                config.special_modes.correlation_vector_filter_invert,
                cv_sidecar_written.as_deref(),
                config.special_modes.correlation_vector_summary_format,
            );
            // CLOC11.75 — route to stderr_text when the
            // stderr flag is on; default stays on stdout.
            if config.special_modes.correlation_vector_summary_stderr {
                result.stderr_text.push_str(&summary);
            } else {
                result.stdout_text.push_str(&summary);
            }
        }
    }

    Ok(CompilerOutput {
        stdout_text: result.stdout_text,
        stderr_text: result.stderr_text,
        wrote_files,
    })
}

/// CLOC11.73 — produce a one-line stdout summary of the CV
/// log, *after* the same filter would have been applied for
/// the sidecar. The output ends with a newline so it composes
/// cleanly with the rest of `stdout_text`.
///
/// Two flavors:
///   - `wrote_path = Some(p)` (Json / Ndjson format):
///     "cv sidecar: <p>: N entries, M contributions, T
///      tombstones, pass_order=[...]"
///   - `wrote_path = None` (NONE format / write skipped):
///     "cv sidecar: skipped (format=NONE): N entries, M
///      contributions, T tombstones, pass_order=[...]"
///
/// Counts reflect the post-filter view; under filter=[] the
/// summary describes the unfiltered log.
fn compute_cv_summary(
    cv_log: &coding_adventures_correlation_vector::CVLog,
    filter: &[String],
    filter_includes_origin: bool,
    filter_invert: bool,
    wrote_path: Option<&std::path::Path>,
    summary_format: crate::config::CorrelationVectorSummaryFormat,
) -> String {
    // Build the same parsed-Value form the formatters use,
    // apply the same filter, then count.
    let compact = cv_log
        .to_json_string()
        .unwrap_or_else(|_| "{}".to_string());
    let mut root: serde_json::Value = match serde_json::from_str(&compact) {
        Ok(v) => v,
        Err(_) => {
            // Fallback to a stub summary; pass_order is
            // unknown but we still print zeros so the line
            // is well-formed.
            return summary_line(wrote_path, 0, 0, 0, &[], summary_format);
        }
    };
    if !filter.is_empty() {
        prune_entries_by_source(
            &mut root,
            filter,
            filter_includes_origin,
            filter_invert,
        );
    }
    let entries = root
        .get("entries")
        .and_then(|v| v.as_object())
        .map(|o| o.values().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let entry_count = entries.len();
    let contribution_count: usize = entries
        .iter()
        .map(|e| {
            e.get("contributions")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0)
        })
        .sum();
    let tombstone_count: usize = entries
        .iter()
        .filter(|e| {
            e.get("deleted")
                .map(|d| !d.is_null())
                .unwrap_or(false)
        })
        .count();
    let pass_order: Vec<String> = root
        .get("pass_order")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    summary_line(
        wrote_path,
        entry_count,
        contribution_count,
        tombstone_count,
        &pass_order,
        summary_format,
    )
}

/// Render the CLOC11.73 summary as a single line ending in
/// `\n`. Kept separate from `compute_cv_summary` so the
/// formatting can be tested / changed without re-running the
/// CV count walk.
///
/// CLOC11.74: dispatch on `summary_format` to produce one of
/// three shapes:
///   - `Text` (default): the CLOC11.73 human-readable form.
///   - `Json`: single-line JSON object `{"cv_sidecar": {...}}`.
///     `path` is `null` under `skipped=true`.
///   - `Kv`: space-separated `key=value` pairs prefixed
///     with `cv_sidecar.`. Values that may contain spaces
///     (path; pass_order joined by `,`) are quoted on the
///     RHS so shell tooling can `awk '{...}'`/`cut` safely.
fn summary_line(
    wrote_path: Option<&std::path::Path>,
    entries: usize,
    contributions: usize,
    tombstones: usize,
    pass_order: &[String],
    summary_format: crate::config::CorrelationVectorSummaryFormat,
) -> String {
    use crate::config::CorrelationVectorSummaryFormat as F;
    match summary_format {
        F::Text => {
            let prefix = match wrote_path {
                Some(p) => format!("cv sidecar: {}", p.display()),
                None => "cv sidecar: skipped (format=NONE)".to_string(),
            };
            format!(
                "{}: {} entries, {} contributions, {} tombstones, pass_order=[{}]\n",
                prefix,
                entries,
                contributions,
                tombstones,
                pass_order.join(","),
            )
        }
        F::Json => {
            // Build a serde_json::Map for the cv_sidecar
            // payload so we don't have to hand-escape paths
            // or pass_order entries. serde_json handles
            // strings, quotes, nested objects safely.
            let mut payload = serde_json::Map::new();
            match wrote_path {
                Some(p) => {
                    payload.insert(
                        "path".to_string(),
                        serde_json::Value::String(p.display().to_string()),
                    );
                    payload.insert(
                        "skipped".to_string(),
                        serde_json::Value::Bool(false),
                    );
                }
                None => {
                    payload.insert(
                        "path".to_string(),
                        serde_json::Value::Null,
                    );
                    payload.insert(
                        "skipped".to_string(),
                        serde_json::Value::Bool(true),
                    );
                }
            }
            payload.insert(
                "entries".to_string(),
                serde_json::Value::Number((entries as u64).into()),
            );
            payload.insert(
                "contributions".to_string(),
                serde_json::Value::Number((contributions as u64).into()),
            );
            payload.insert(
                "tombstones".to_string(),
                serde_json::Value::Number((tombstones as u64).into()),
            );
            payload.insert(
                "pass_order".to_string(),
                serde_json::Value::Array(
                    pass_order
                        .iter()
                        .map(|s| serde_json::Value::String(s.clone()))
                        .collect(),
                ),
            );
            let mut root = serde_json::Map::new();
            root.insert(
                "cv_sidecar".to_string(),
                serde_json::Value::Object(payload),
            );
            let mut line = serde_json::to_string(&serde_json::Value::Object(root))
                .unwrap_or_else(|_| "{\"cv_sidecar\":{}}".to_string());
            line.push('\n');
            line
        }
        F::Kv => {
            // Quote path + pass_order on the RHS so callers
            // can split on whitespace; the values themselves
            // never contain a literal `"` in our pipeline,
            // but we still let serde escape them defensively.
            let path_val = match wrote_path {
                Some(p) => p.display().to_string(),
                None => String::new(),
            };
            let skipped_val = wrote_path.is_none();
            let pass_order_joined = pass_order.join(",");
            let path_quoted = serde_json::to_string(&path_val)
                .unwrap_or_else(|_| "\"\"".to_string());
            let pass_order_quoted = serde_json::to_string(&pass_order_joined)
                .unwrap_or_else(|_| "\"\"".to_string());
            format!(
                "cv_sidecar.path={} cv_sidecar.skipped={} cv_sidecar.entries={} cv_sidecar.contributions={} cv_sidecar.tombstones={} cv_sidecar.pass_order={}\n",
                path_quoted,
                skipped_val,
                entries,
                contributions,
                tombstones,
                pass_order_quoted,
            )
        }
    }
}

/// Serialize a `CVLog` to a JSON string for the
/// `--correlation_vector` sidecar.
///
/// `pretty` (CLOC11.68): when true, pretty-prints the JSON
/// (multi-line, 2-space indent) by round-tripping through
/// `serde_json::Value` and `to_string_pretty`. When false
/// (the default, and what CI / build pipelines want), emits
/// compact single-line JSON via the CV crate's native
/// `to_json_string`.
///
/// Why the round-trip for pretty mode: the CV crate's
/// `to_json_string` is hard-coded to compact output and it's
/// the only path that knows the `LogSnapshot` shape (the
/// fields aren't pub). Parsing back to a `serde_json::Value`
/// and re-emitting via `to_string_pretty` is wasteful but
/// correct, and only happens on the opt-in slow path. The
/// performance hit is irrelevant — humans-eyes mode is
/// already off the critical path of a build.
///
/// On any serialization failure (vanishingly rare for the
/// CV crate's well-formed structs, and additionally
/// surviving a round-trip in pretty mode), we fall back to
/// a minimal `{}` document so the write still succeeds —
/// a missing sidecar would be worse than a stub one for the
/// build-pipeline-consumes-the-file case.
fn format_cv_log_json(
    cv_log: &coding_adventures_correlation_vector::CVLog,
    pretty: bool,
    filter: &[String],
    filter_includes_origin: bool,
    filter_invert: bool,
) -> String {
    let compact = cv_log
        .to_json_string()
        .unwrap_or_else(|_| "{}".to_string());
    let need_filter = !filter.is_empty();
    if !pretty && !need_filter {
        // Fast path: default-default — no parse, no transform.
        return compact;
    }
    // Round-trip: compact text → serde_json::Value → (filter?) → text.
    let mut value: serde_json::Value =
        match serde_json::from_str(&compact) {
            Ok(v) => v,
            Err(_) => return compact,
        };
    if need_filter {
        prune_entries_by_source(
            &mut value,
            filter,
            filter_includes_origin,
            filter_invert,
        );
    }
    if pretty {
        serde_json::to_string_pretty(&value).unwrap_or(compact)
    } else {
        serde_json::to_string(&value).unwrap_or(compact)
    }
}

/// CLOC11.69 — serialize a `CVLog` as newline-delimited JSON.
///
/// Output shape:
///   - one line per CV entry, formatted as the JSON for the
///     entry's `CVEntry` struct,
///   - followed by one final line with the metadata object:
///     `{"_meta": {"pass_order": [...], "enabled": <bool>}}`.
///
/// Why a final `_meta` line: streaming consumers reading the
/// sidecar with `tail -f` or line-by-line want the entries
/// available as they arrive without waiting for a closing
/// brace. Trailing the metadata keeps `pass_order` parseable
/// once the producer is done without polluting any individual
/// entry line.
///
/// We reuse `to_json_string` + parse-as-Value rather than
/// touching CV crate internals. The compact JSON has the shape
/// `{"entries":{"id":{...}}, "pass_order":[...], "enabled":...}`
/// so we walk the `entries` map and re-emit each value as a
/// single-line JSON document. On any parse / serialize hiccup
/// we fall through to the compact JSON document (a valid
/// fallback the consumer's `tail` will still see).
fn format_cv_log_ndjson(
    cv_log: &coding_adventures_correlation_vector::CVLog,
    filter: &[String],
    filter_includes_origin: bool,
    filter_invert: bool,
) -> String {
    let compact = cv_log
        .to_json_string()
        .unwrap_or_else(|_| "{}".to_string());
    let mut root: serde_json::Value = match serde_json::from_str(&compact) {
        Ok(v) => v,
        Err(_) => return compact,
    };
    if !filter.is_empty() {
        prune_entries_by_source(
            &mut root,
            filter,
            filter_includes_origin,
            filter_invert,
        );
    }
    let mut out = String::new();
    if let Some(entries) = root.get("entries").and_then(|v| v.as_object()) {
        for entry in entries.values() {
            if let Ok(line) = serde_json::to_string(entry) {
                out.push_str(&line);
                out.push('\n');
            }
        }
    }
    // Append the metadata footer line.
    let mut meta = serde_json::Map::new();
    if let Some(po) = root.get("pass_order") {
        meta.insert("pass_order".to_string(), po.clone());
    }
    if let Some(en) = root.get("enabled") {
        meta.insert("enabled".to_string(), en.clone());
    }
    let mut footer = serde_json::Map::new();
    footer.insert("_meta".to_string(), serde_json::Value::Object(meta));
    if let Ok(line) = serde_json::to_string(&serde_json::Value::Object(footer)) {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// CLOC11.70 + CLOC11.71 — in-place prune of the `entries`
/// object on a parsed CVLog JSON `Value` by a source allowlist.
///
/// Walks `root["entries"]` (an object map id → entry). An
/// entry is kept iff at least one of the following matches a
/// string in `allowlist`:
///
///   1. any element of `contributions` whose `source` is in
///      the allowlist (the CLOC11.70 rule, always applied);
///   2. **if** `include_origin` is true (CLOC11.71): the
///      entry's `origin.source` string.
///
/// `include_origin=false` preserves CLOC11.70 strict
/// semantics byte-for-byte. `include_origin=true` lets
/// `--correlation_vector_filter lex` also keep per-token CV
/// entries whose `Origin.source == "lexer_token"` even
/// though they have zero contributions.
///
/// `allowlist` is treated as a closed set of exact-match
/// strings — no wildcards, no prefix matching. The empty
/// allowlist case is the caller's responsibility (caller
/// should skip this function entirely; we'd otherwise prune
/// everything).
///
/// Why a separate helper rather than inlining at each
/// formatter: the json and ndjson paths both round-trip
/// through `serde_json::Value` (the former for pretty mode,
/// the latter unconditionally), so one helper handling both
/// is cheaper than two near-identical loops and keeps the
/// "entry is kept iff …" rule in one auditable place.
fn prune_entries_by_source(
    root: &mut serde_json::Value,
    allowlist: &[String],
    include_origin: bool,
    invert: bool,
) {
    let Some(entries) = root
        .get_mut("entries")
        .and_then(|v| v.as_object_mut())
    else {
        return;
    };
    // Build a HashSet for O(1) lookup; the loop runs over
    // every entry × every contribution otherwise.
    let allow: std::collections::HashSet<&str> =
        allowlist.iter().map(String::as_str).collect();
    entries.retain(|_id, entry| {
        // Compute "does this entry match the source list?"
        // (1) any contribution.source in the allowlist, OR
        // (2) (CLOC11.71 opt-in) origin.source in the allowlist.
        let contrib_match = entry
            .get("contributions")
            .and_then(|v| v.as_array())
            .map(|contribs| {
                contribs.iter().any(|c| {
                    c.get("source")
                        .and_then(|s| s.as_str())
                        .map(|s| allow.contains(s))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        let origin_match = include_origin
            && entry
                .get("origin")
                .and_then(|o| o.get("source"))
                .and_then(|s| s.as_str())
                .map(|s| allow.contains(s))
                .unwrap_or(false);
        let matches = contrib_match || origin_match;
        // CLOC11.72 — invert flips the keep rule. Default
        // (invert=false): keep matches, drop non-matches
        // (allowlist). invert=true: keep non-matches, drop
        // matches (blocklist).
        if invert { !matches } else { matches }
    });
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
        // SIMPLE v1 produces whitespace_only output; use compact input
        // so the assertion holds regardless of whitespace normalisation.
        assert!(written.contains("console.log(\"hi\");") || written.contains("console.log('hi');"));
        let _ = fs::remove_file(&in_path);
        let _ = fs::remove_file(&out_path);
    }

    #[test]
    fn multiple_inputs_concatenate_in_order_with_newlines() {
        let a = temp_path("a.js");
        let b = temp_path("b.js");
        // Use compact non-comment content — SIMPLE v1 runs whitespace_only
        // which strips comments, so comments are not reliable markers.
        fs::write(&a, "var a=1;").expect("write a");
        fs::write(&b, "var b=2;").expect("write b");

        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![
                    a.to_string_lossy().to_string(),
                    b.to_string_lossy().to_string(),
                ],
                ..Default::default()
            },
            // Pinned to WHITESPACE_ONLY: this test is about input
            // concatenation order, not optimization (at SIMPLE the
            // unreferenced `var a`/`var b` would be removed).
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::WhitespaceOnly,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        // Both files appear; the first one gets a trailing newline
        // injected because its content didn't end with one.
        let s = &out.stdout_text;
        let a_idx = s.find("var a=1;").expect("a in output");
        let b_idx = s.find("var b=2;").expect("b in output");
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
            // Pinned to WHITESPACE_ONLY: this test is about newline
            // handling, not optimization. At the default level (SIMPLE)
            // remove-unused-vars would delete the unreferenced `var x`.
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::WhitespaceOnly,
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
            // Pinned to WHITESPACE_ONLY: this test is about parent-dir
            // auto-creation, not optimization (at SIMPLE the unreferenced
            // `var x` would be removed).
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::WhitespaceOnly,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert_eq!(out.wrote_files, vec![out_path.clone()]);
        let written = fs::read_to_string(&out_path).expect("read out");
        assert_eq!(written, "var x=1;\n");
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn run_compiler_stdout_fallback_when_output_file_absent() {
        // CLOC11.01 already supported this; pin it as a CLOC11.03
        // regression test so we can't accidentally break stdout
        // fallback when the output-file path grows new behavior.
        let in_path = temp_path("stdout-fallback.js");
        // Use compact non-comment content — SIMPLE v1 runs whitespace_only
        // which strips comments. This test verifies the stdout fallback
        // plumbing, not the compilation output content.
        fs::write(&in_path, "var x=1;\n").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: None,
                ..Default::default()
            },
            // Pinned to WHITESPACE_ONLY: this test is about the stdout
            // fallback plumbing, not optimization (at SIMPLE the
            // unreferenced `var x` would be removed).
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::WhitespaceOnly,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.wrote_files.is_empty());
        assert!(out.stdout_text.contains("var x=1;"));
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
            // Pinned to WHITESPACE_ONLY: this test is about the
            // use-strict directive, not optimization (at SIMPLE the
            // unreferenced `var x` would be removed).
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::WhitespaceOnly,
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
            // Pinned to WHITESPACE_ONLY: tests directive placement
            // inside the IIFE, not optimization.
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::WhitespaceOnly,
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
            // Pinned to WHITESPACE_ONLY: tests directive placement
            // inside the output wrapper, not optimization.
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::WhitespaceOnly,
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
            // Pinned to WHITESPACE_ONLY: this test is about externs
            // resolution, not optimization (at SIMPLE the unreferenced
            // `var x` would be removed).
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::WhitespaceOnly,
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
            // Pinned to WHITESPACE_ONLY: this test is about source-map
            // emission, not optimization (at SIMPLE the unreferenced
            // `var x` would be removed).
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::WhitespaceOnly,
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

    /// Process-global mutex shared by every test that calls
    /// `std::env::set_current_dir`. CWD is process state, not
    /// thread state — without serialization, two tests chdir'ing
    /// in parallel race and one reads from the other's temp dir
    /// (CLOC11.63 hit this when CLOC11.60's chdir test and
    /// CLOC11.63's chdir test ran concurrently).
    ///
    /// We use a plain `std::sync::Mutex<()>` rather than a
    /// crate like `serial_test`: the repo principle is
    /// zero-deps where reasonable and the lock pattern here is
    /// trivial.
    static CWD_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn correlation_vector_on_writes_default_sidecar_when_stdout_output() {
        // When --js_output_file is absent, the sidecar lands at
        // `closurec-cv.json` in CWD. Test from a temp dir we
        // chdir into so we don't litter the repo root.
        let _cwd_guard = CWD_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
        // CLOC12.155: SIMPLE now records "simple_v2" (constant-fold pipeline).
        assert!(body.contains("\"tag\":\"simple_v2\""), "got: {body}");
        assert!(body.contains("\"level\":\"SIMPLE\""), "got: {body}");
        assert!(body.contains("\"bridge_status\":\"ok\""), "got: {body}");
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
        // pass_order in the CVLog dump should show lex FIRST
        // (CLOC11.64 added per-token CV emission ahead of
        // transform_source_with_cv), then compilation_level,
        // then defines. Other passes follow (CLOC11.62 adds
        // charset; later slices add more). Pin the prefix to
        // keep the per-file ordering invariant while staying
        // tolerant of new passes.
        assert!(
            body.contains(
                "\"pass_order\":[\"lex\",\"compilation_level\",\"defines\""
            ),
            "expected pass_order to start with [lex, compilation_level, defines, ...], got: {body}"
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

    // ------------------------------------------------------------------
    // CLOC11.64 — per-token CV entries (children of per-file CV)
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_emits_per_token_cv_entries() {
        // With --correlation_vector on, every token from the
        // lexer should appear in the sidecar as a derived CV
        // entry with source="lexer_token" and parent_ids
        // pointing at the per-file CV root. We also expect a
        // `lex.tokens_emitted` contribution on the per-file CV
        // with a token_count matching the lexer's output.
        let dir = std::env::temp_dir().join("closurec_cloc11_64_tokens");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write input");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
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
        // Source name appears.
        assert!(
            body.contains("\"source\":\"lexer_token\""),
            "expected lexer_token source in CV sidecar, got: {body}"
        );
        // lex.tokens_emitted contribution lands on the per-file CV.
        assert!(
            body.contains("\"source\":\"lex\"")
                && body.contains("\"tag\":\"tokens_emitted\""),
            "expected lex.tokens_emitted contribution, got: {body}"
        );
        // token_index keys appear (proves we wrote the per-token meta).
        assert!(
            body.contains("\"token_index\""),
            "expected token_index in token meta, got: {body}"
        );
        // For "var x = 1;" the JS lexer emits at least 5 tokens
        // (var, x, =, 1, ;) — assert lower-bound, not exact,
        // because the lexer may emit additional tokens (EOF,
        // implicit semis) and we don't want to brittle-pin
        // grammar internals.
        let token_count = body.matches("\"source\":\"lexer_token\"").count();
        assert!(
            token_count >= 5,
            "expected at least 5 token CV entries for 'var x = 1;', got {token_count}: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_disabled_emits_no_token_entries() {
        // With CV disabled (default), no sidecar should be
        // written and no per-token work should happen — same
        // byte-identical behavior as pre-CLOC11.64.
        let dir = std::env::temp_dir().join("closurec_cloc11_64_off");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write input");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            // correlation_vector NOT set (default false)
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        assert!(
            !sidecar_path.exists(),
            "no sidecar should exist when CV is disabled"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.65 — per-token `defines.applied` contributions
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_records_defines_applied_on_token_cv() {
        // With --correlation_vector on and --define FOO=true,
        // a Name token whose lexeme is "FOO" should get a
        // `defines.applied` contribution recorded on its
        // token CV (parent = per-file CV). The per-file
        // summary contribution from transform_source_with_cv
        // still fires; this checks the *new* per-token one.
        use crate::config::{DefineValue, DefinesConfig};
        let dir = std::env::temp_dir().join("closurec_cloc11_65_defines");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        // FOO appears as a Name token twice; defines.applied
        // contribution should fire for each occurrence.
        fs::write(&in_path, "var x = FOO; FOO + 1;").expect("write input");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        let mut defines = std::collections::BTreeMap::new();
        defines.insert("FOO".to_string(), DefineValue::Bool(true));
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            defines: DefinesConfig { defines },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        // The token-level defines.applied tag appears alongside
        // the per-file summary. Distinguish the two by checking
        // for the token-only meta key `define_name` (the per-file
        // summary uses `defines_count` instead).
        assert!(
            body.contains("\"define_name\":\"FOO\""),
            "expected token-level defines.applied with define_name=FOO, got: {body}"
        );
        assert!(
            body.contains("\"define_value_kind\":\"bool\""),
            "expected define_value_kind=bool meta, got: {body}"
        );
        // FOO appears twice in the input → two per-token records.
        let occurrences = body.matches("\"define_name\":\"FOO\"").count();
        assert!(
            occurrences >= 2,
            "expected ≥2 per-token defines.applied for FOO×2, got {occurrences}: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_no_per_token_defines_when_unmatched() {
        // A Name token whose lexeme doesn't match any --define
        // key should NOT get a token-level defines.applied
        // contribution. The per-file summary still fires
        // (it always does, with defines_count: 0).
        use crate::config::DefinesConfig;
        let dir = std::env::temp_dir().join("closurec_cloc11_65_nomatch");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write input");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            defines: DefinesConfig { defines: std::collections::BTreeMap::new() },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        // No define_name meta key (token-level) — only the
        // per-file summary which uses defines_count.
        assert!(
            !body.contains("\"define_name\""),
            "expected NO token-level defines.applied, got: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.66 — WHITESPACE_ONLY token tombstones
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_tombstones_trivia_tokens_under_whitespace_only() {
        // With --correlation_vector + --compilation_level
        // WHITESPACE_ONLY, the dropped trivia + EOF tokens
        // should appear in the sidecar as deleted CV entries
        // (DeletionRecord present, source=compilation_level,
        // reason=whitespace_only_dropped). The surviving Name
        // tokens (var, x, etc.) should NOT be tombstoned.
        let dir = std::env::temp_dir().join("closurec_cloc11_66_ws_drop");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        // Comments + whitespace generate trivia tokens; EOF
        // is always emitted at end.
        fs::write(
            &in_path,
            "// a comment\nvar x = 1; /* block */ var y = 2;",
        )
        .expect("write input");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
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
        // The tombstone reason string lands in the sidecar.
        assert!(
            body.contains("\"reason\":\"whitespace_only_dropped\""),
            "expected whitespace_only_dropped tombstone, got: {body}"
        );
        // EOF kind always lands (every file ends with EOF
        // sentinel; the JS grammar happens not to emit COMMENT
        // tokens — comments are skipped at lex time — so trivia
        // kind never fires for this grammar today, but the code
        // path covers both cases against future grammar
        // evolution).
        assert!(
            body.contains("\"kind\":\"eof\""),
            "expected eof kind tombstone, got: {body}"
        );
        // The tombstone landed via the DeletionRecord field.
        assert!(
            body.contains("\"source\":\"compilation_level\",\"reason\":\"whitespace_only_dropped\""),
            "expected DeletionRecord with compilation_level source, got: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_no_tombstones_under_simple_level() {
        // Non-WHITESPACE_ONLY levels currently don't drop
        // tokens at the compilation_level stage, so no
        // tombstones should appear. (As later slices add
        // real bodies to SIMPLE/ADVANCED/etc., each will
        // need its own block + test; this pins the current
        // contract.)
        let dir = std::env::temp_dir().join("closurec_cloc11_66_simple");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "// a comment\nvar x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            // SIMPLE is the default; spelled out for clarity.
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
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
        assert!(
            !body.contains("\"reason\":\"whitespace_only_dropped\""),
            "expected NO whitespace_only_dropped tombstones under SIMPLE, got: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC12.132 — whitespace_only gap-drop tombstones
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_tombstones_gap_dropped_tokens_under_whitespace_only() {
        // With --correlation_vector + WHITESPACE_ONLY, the gap
        // pre-passes that remove non-trivia tokens should appear
        // in the sidecar as deleted CV entries with
        //   source="whitespace_only", reason="gap_drop".
        //
        // Input: `var x=(1);` — gap-053 (paren elision around
        // var-init RHS) drops the redundant `(` and `)` in its
        // pre-pass, producing `var x=1;`. Both paren tokens are
        // non-trivia and should be tombstoned.
        let dir = std::env::temp_dir().join("closurec_cloc12_132_gap_drop");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x=(1);").expect("write input");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
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
        // gap-drop tombstone from whitespace_only_minify.
        assert!(
            body.contains("\"reason\":\"gap_drop\""),
            "expected gap_drop tombstone for new Foo() → new Foo, got: {body}"
        );
        assert!(
            body.contains("\"source\":\"whitespace_only\""),
            "expected source=whitespace_only on gap_drop tombstone, got: {body}"
        );
        // The lexeme of the dropped paren should appear in the meta.
        assert!(
            body.contains("\"lexeme\":\"(\"") || body.contains("\"lexeme\":\")\""),
            "expected dropped paren lexeme in tombstone meta, got: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_no_gap_drop_tombstones_when_no_gaps_fire() {
        // A source with no gap-rule targets should produce NO
        // gap_drop tombstones. `var x=1;` has no redundant
        // parens, empty new-args, etc. — zero gap pre-pass drops.
        let dir = std::env::temp_dir().join("closurec_cloc12_132_no_drops");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x=1;").expect("write input");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
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
        assert!(
            !body.contains("\"reason\":\"gap_drop\""),
            "expected NO gap_drop tombstones for var x=1;, got: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC12.133 — whitespace_only emit-loop skip tombstones
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_emit_skip_gap050_new_empty_args() {
        // gap-050 runs in the EMIT LOOP: `new Foo()` → `new Foo`.
        // The `(` and `)` survive pre-passes (they are in `kept`) but
        // are suppressed during emission. With CV enabled, they should
        // appear as DeletionRecords with
        //   reason="emit_skip", meta.gap="gap-050".
        let dir = std::env::temp_dir().join("closurec_cloc12_133_gap050");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x=new Foo();").expect("write input");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
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
        assert!(
            body.contains("\"reason\":\"emit_skip\""),
            "expected emit_skip tombstone for gap-050, got: {body}"
        );
        assert!(
            body.contains("\"gap\":\"gap-050\""),
            "expected meta.gap=gap-050 on emit_skip tombstone, got: {body}"
        );
        // Both `(` and `)` should be tombstoned.
        assert!(
            body.contains("\"lexeme\":\"(\"") || body.contains("\"lexeme\":\")\""),
            "expected ( or ) lexeme in emit_skip tombstone meta, got: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_emit_skip_gap030_rule_a_semi_before_brace() {
        // gap-030 rule-A runs in the EMIT LOOP: a `;` immediately before
        // `}` is dropped (the `}` itself acts as the statement terminator).
        // `(function(){var x=1;})()` — the `;` before `}` should be
        // tombstoned with reason="emit_skip", meta.gap="gap-030-rule-a".
        let dir = std::env::temp_dir().join("closurec_cloc12_133_gap030");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "(function(){var x=1;})();").expect("write input");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
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
        assert!(
            body.contains("\"reason\":\"emit_skip\""),
            "expected emit_skip tombstone for gap-030 rule-A, got: {body}"
        );
        assert!(
            body.contains("\"gap\":\"gap-030-rule-a\""),
            "expected meta.gap=gap-030-rule-a, got: {body}"
        );
        // The dropped `;` should appear as the lexeme.
        assert!(
            body.contains("\"lexeme\":\";\""),
            "expected ';' lexeme in emit_skip tombstone meta, got: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.67 — --correlation_vector_output <path> flag
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_output_flag_overrides_sidecar_path() {
        // With --correlation_vector_output set to an explicit
        // path, the sidecar should land THERE (verbatim, no
        // .cv.json decoration) and NOT at the default location.
        let dir = std::env::temp_dir().join("closurec_cloc11_67_explicit");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write input");
        let out_path = dir.join("out.js");
        let default_sidecar = dir.join("out.js.cv.json"); // would-be default
        let explicit_sidecar = dir.join("custom-trace.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_output: Some(explicit_sidecar.clone()),
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        assert!(
            explicit_sidecar.exists(),
            "expected sidecar at explicit path {:?}",
            explicit_sidecar
        );
        assert!(
            !default_sidecar.exists(),
            "expected NO sidecar at default path {:?}",
            default_sidecar
        );
        // Sanity: file contains the CV JSON structure.
        let body = fs::read_to_string(&explicit_sidecar).expect("read");
        assert!(
            body.contains("\"entries\""),
            "explicit-path sidecar missing CVLog structure: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_output_flag_ignored_when_cv_disabled() {
        // With --correlation_vector off, --correlation_vector_output
        // is silently ignored — no sidecar of any kind is written.
        // Pins the contract that the path flag does not
        // accidentally enable the trace.
        let dir = std::env::temp_dir().join("closurec_cloc11_67_off");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write input");
        let out_path = dir.join("out.js");
        let explicit_sidecar = dir.join("should-not-exist.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                // correlation_vector remains false
                correlation_vector_output: Some(explicit_sidecar.clone()),
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        assert!(
            !explicit_sidecar.exists(),
            "expected NO sidecar when correlation_vector is off, got file at {:?}",
            explicit_sidecar
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.68 — --correlation_vector_pretty flag
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_pretty_off_emits_compact_json() {
        // Default behavior: compact single-line JSON. We
        // detect compact form by absence of "\n  " (indented
        // newlines). The body has at least one entry so
        // pretty would definitely insert newlines.
        let dir = std::env::temp_dir().join("closurec_cloc11_68_compact");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                // correlation_vector_pretty defaults to false
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read");
        assert!(
            !body.contains("\n  "),
            "expected compact JSON (no indented newlines), got: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_pretty_on_emits_indented_json() {
        // With --correlation_vector_pretty, the sidecar JSON
        // should have indented newlines. We check for "\n  "
        // (newline + 2 spaces) which serde_json::to_string_pretty
        // emits at every nested key.
        let dir = std::env::temp_dir().join("closurec_cloc11_68_pretty");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_pretty: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read");
        assert!(
            body.contains("\n  "),
            "expected pretty JSON with indented newlines, got: {body}"
        );
        // Same data content as compact: still has entries key
        // and pass_order. Pretty mode is just whitespace.
        assert!(
            body.contains("\"entries\""),
            "pretty JSON missing entries key: {body}"
        );
        assert!(
            body.contains("\"pass_order\""),
            "pretty JSON missing pass_order key: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.69 — --correlation_vector_format (JSON | NDJSON | NONE)
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_format_ndjson_writes_one_entry_per_line() {
        // NDJSON: every line should parse as JSON; the count
        // of lines should be `entries + 1` (one per CV entry,
        // one footer `_meta` line). We don't pin the exact
        // count to avoid brittleness as the pipeline grows; we
        // just assert ≥2 lines and every line is valid JSON.
        let dir = std::env::temp_dir().join("closurec_cloc11_69_ndjson");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_format:
                    crate::config::CorrelationVectorFormat::Ndjson,
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        let lines: Vec<&str> =
            body.lines().filter(|l| !l.is_empty()).collect();
        assert!(
            lines.len() >= 2,
            "expected ≥2 NDJSON lines (entries + meta), got {}: {body}",
            lines.len()
        );
        // Every line must parse as JSON on its own.
        for (i, line) in lines.iter().enumerate() {
            serde_json::from_str::<serde_json::Value>(line)
                .unwrap_or_else(|_| panic!(
                    "NDJSON line {i} did not parse as JSON: {line}"
                ));
        }
        // The last line should be the _meta footer.
        assert!(
            lines.last().unwrap().contains("\"_meta\""),
            "expected last line to be _meta footer, got: {}",
            lines.last().unwrap()
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_format_none_skips_sidecar_write() {
        // NONE format: CV is enabled, the trace is computed,
        // but NO sidecar file is created. The would-be default
        // path must not exist after the run.
        let dir = std::env::temp_dir().join("closurec_cloc11_69_none");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let default_sidecar = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_format:
                    crate::config::CorrelationVectorFormat::None,
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        assert!(
            !default_sidecar.exists(),
            "NONE format should skip sidecar write, found file at {:?}",
            default_sidecar
        );
        // JS output still produced — CV gating doesn't affect
        // the normal pipeline.
        assert!(
            out_path.exists(),
            "JS output should still be written under NONE: {:?}",
            out_path
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_format_json_default_unchanged() {
        // Default (JSON) format: existing behavior preserved.
        // Sidecar exists and is a single JSON document.
        let dir = std::env::temp_dir().join("closurec_cloc11_69_json");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                // correlation_vector_format defaults to Json
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        // Whole body must parse as a single JSON document.
        serde_json::from_str::<serde_json::Value>(&body)
            .expect("JSON format should produce a single JSON doc");
        // And NOT contain a _meta footer (that's NDJSON-only).
        assert!(
            !body.contains("\"_meta\""),
            "JSON format should not include the NDJSON _meta footer"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.70 — --correlation_vector_filter allowlist
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_filter_keeps_only_allowlisted_sources() {
        // With filter=["lex"], only entries containing a
        // contribution with source="lex" should survive.
        // Empty-contribution entries (token CVs) are dropped.
        // Entries with allowlisted contributions (per-file
        // CV root has lex.tokens_emitted) are kept.
        let dir = std::env::temp_dir().join("closurec_cloc11_70_filter_lex");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_filter: vec!["lex".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        let root: serde_json::Value =
            serde_json::from_str(&body).expect("parse");
        let entries = root
            .get("entries")
            .and_then(|v| v.as_object())
            .expect("entries object");
        // Every surviving entry must have at least one
        // contribution with source="lex".
        for (id, entry) in entries.iter() {
            let contribs = entry
                .get("contributions")
                .and_then(|v| v.as_array())
                .expect("contributions array");
            let has_lex = contribs.iter().any(|c| {
                c.get("source").and_then(|s| s.as_str()) == Some("lex")
            });
            assert!(
                has_lex,
                "entry {id} survived but has no lex contribution: {entry}"
            );
        }
        // No lexer_token entries (no contributions) should
        // appear under filter=lex.
        for (id, entry) in entries.iter() {
            let origin_source = entry
                .get("origin")
                .and_then(|o| o.get("source"))
                .and_then(|s| s.as_str());
            assert_ne!(
                origin_source,
                Some("lexer_token"),
                "lexer_token entry {id} should have been pruned: {entry}"
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_filter_empty_keeps_everything() {
        // Empty filter = no pruning. Sanity check that the
        // default config behaves identically to pre-CLOC11.70.
        let dir = std::env::temp_dir().join("closurec_cloc11_70_filter_empty");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                // correlation_vector_filter defaults to empty Vec
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        // Confirm at least one lexer_token entry exists (it
        // would be pruned under filter=lex per the test above).
        assert!(
            body.contains("\"source\":\"lexer_token\""),
            "expected lexer_token entries in unfiltered sidecar: {body}"
        );
    }

    #[test]
    fn correlation_vector_filter_unmatched_prunes_all() {
        // Filter with a source name that never appears →
        // entries object becomes empty. The skeleton
        // (entries / pass_order / enabled) is still valid
        // JSON.
        let dir = std::env::temp_dir().join("closurec_cloc11_70_filter_none");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_filter: vec![
                    "nonexistent_source_xyz".to_string()
                ],
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        let root: serde_json::Value =
            serde_json::from_str(&body).expect("parse");
        let entries = root
            .get("entries")
            .and_then(|v| v.as_object())
            .expect("entries object");
        assert!(
            entries.is_empty(),
            "expected empty entries under unmatched filter, got {}: {body}",
            entries.len()
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.71 — --correlation_vector_filter_includes_origin
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_filter_origin_off_drops_lexer_token_origin() {
        // Default semantics (sub-flag off): filter=lexer_token
        // alone prunes every entry that doesn't have a
        // contribution with source="lexer_token". Token CV
        // entries have NO contributions; only their Origin
        // says "lexer_token". So they should all be dropped.
        let dir =
            std::env::temp_dir().join("closurec_cloc11_71_origin_off");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_filter: vec!["lexer_token".to_string()],
                // correlation_vector_filter_includes_origin defaults to false
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        // No entry has a contribution with source="lexer_token",
        // so under strict semantics every entry is pruned.
        assert!(
            !body.contains("\"source\":\"lexer_token\""),
            "filter=lexer_token without include_origin should drop all token entries, got: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_filter_origin_on_keeps_lexer_token_origin() {
        // With include_origin = true, filter=lexer_token now
        // keeps the entries whose Origin.source is
        // "lexer_token" (i.e. the per-token CV entries) even
        // though they have zero contributions. The per-file
        // CV root (Origin.source="input_file") is dropped
        // because neither its Origin nor its contributions
        // mention "lexer_token".
        let dir = std::env::temp_dir().join("closurec_cloc11_71_origin_on");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_filter: vec!["lexer_token".to_string()],
                correlation_vector_filter_includes_origin: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        // Origin.source=lexer_token entries survived.
        assert!(
            body.contains("\"source\":\"lexer_token\""),
            "filter=lexer_token + include_origin=true should keep token entries, got: {body}"
        );
        // input_file root entry was pruned.
        assert!(
            !body.contains("\"source\":\"input_file\""),
            "input_file Origin should not match filter=lexer_token, got: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.72 — --correlation_vector_filter_invert (blocklist)
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_filter_invert_drops_matched_keeps_rest() {
        // With filter=lex AND invert=true, entries that have
        // a lex contribution are DROPPED and entries without
        // are kept. So the per-file CV root (which has the
        // lex.tokens_emitted contribution) should be gone,
        // but token CVs (no lex contribution) should remain
        // alongside the combined/output entries.
        let dir =
            std::env::temp_dir().join("closurec_cloc11_72_invert_basic");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_filter: vec!["lex".to_string()],
                correlation_vector_filter_invert: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        let root: serde_json::Value =
            serde_json::from_str(&body).expect("parse");
        let entries = root
            .get("entries")
            .and_then(|v| v.as_object())
            .expect("entries object");
        // No surviving entry has a lex contribution.
        for (id, entry) in entries.iter() {
            let contribs = entry
                .get("contributions")
                .and_then(|v| v.as_array())
                .expect("contributions array");
            let has_lex = contribs.iter().any(|c| {
                c.get("source").and_then(|s| s.as_str()) == Some("lex")
            });
            assert!(
                !has_lex,
                "entry {id} survived but has a lex contribution under invert: {entry}"
            );
        }
        // Entries DO still exist — invert is not "drop
        // everything". The non-input_file entries (combined,
        // js_output_file, manifest, etc.) live on.
        assert!(
            !entries.is_empty(),
            "invert should keep non-matching entries, got empty: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_filter_invert_empty_filter_is_noop() {
        // An empty filter with invert=true is still a
        // no-op (nothing can match an empty allowlist, so
        // nothing is blocked). Same byte-identical output
        // as no-filter-set.
        let dir = std::env::temp_dir().join("closurec_cloc11_72_invert_empty");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                // correlation_vector_filter left empty
                correlation_vector_filter_invert: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        // Sanity: input_file root + lexer_token entries both
        // appear (no pruning happened).
        assert!(
            body.contains("\"source\":\"input_file\""),
            "expected input_file Origin under empty inverted filter: {body}"
        );
        assert!(
            body.contains("\"source\":\"lexer_token\""),
            "expected lexer_token Origins under empty inverted filter: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.73 — --correlation_vector_summary
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_summary_writes_one_line_after_sidecar() {
        // With --correlation_vector + --correlation_vector_summary,
        // stdout_text should end with the summary line and
        // the sidecar should still land on disk.
        let dir =
            std::env::temp_dir().join("closurec_cloc11_73_summary_default");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_summary: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(
            sidecar_path.exists(),
            "expected sidecar at {:?}",
            sidecar_path
        );
        // stdout summary present.
        assert!(
            out.stdout_text.contains("cv sidecar:"),
            "expected `cv sidecar:` prefix, got: {:?}",
            out.stdout_text
        );
        assert!(
            out.stdout_text.contains(" entries, "),
            "expected entries field, got: {:?}",
            out.stdout_text
        );
        assert!(
            out.stdout_text.contains("pass_order=["),
            "expected pass_order field, got: {:?}",
            out.stdout_text
        );
        // The summary line ends in `\n`.
        assert!(
            out.stdout_text.ends_with('\n'),
            "expected trailing newline, got: {:?}",
            out.stdout_text
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_summary_reports_skipped_under_format_none() {
        // Format=NONE: sidecar is not written. Summary line
        // should say "skipped (format=NONE)" but still count
        // entries / contributions / tombstones from the
        // (in-memory) cv_log.
        let dir =
            std::env::temp_dir().join("closurec_cloc11_73_summary_none");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let default_sidecar = dir.join("out.js.cv.json");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_format:
                    crate::config::CorrelationVectorFormat::None,
                correlation_vector_summary: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(
            !default_sidecar.exists(),
            "NONE format should skip sidecar write"
        );
        assert!(
            out.stdout_text.contains("skipped (format=NONE)"),
            "expected `skipped (format=NONE)` marker, got: {:?}",
            out.stdout_text
        );
        // Still reports counts.
        assert!(
            out.stdout_text.contains(" entries, "),
            "expected entries field even under NONE: {:?}",
            out.stdout_text
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_summary_off_emits_no_summary() {
        // Default flag off → no summary line in stdout_text
        // even when CV is enabled.
        let dir = std::env::temp_dir().join("closurec_cloc11_73_summary_off");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                // correlation_vector_summary defaults to false
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(
            !out.stdout_text.contains("cv sidecar:"),
            "expected no summary line under default flag, got: {:?}",
            out.stdout_text
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.74 — --correlation_vector_summary_format
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_summary_format_json_emits_parseable_object() {
        // JSON summary: stdout_text contains one line that
        // parses as JSON and has the cv_sidecar object with
        // expected fields.
        let dir =
            std::env::temp_dir().join("closurec_cloc11_74_summary_json");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_summary: true,
                correlation_vector_summary_format:
                    crate::config::CorrelationVectorSummaryFormat::Json,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        let line = out.stdout_text.trim();
        let parsed: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|_| panic!("JSON summary did not parse: {line}"));
        let cv = parsed
            .get("cv_sidecar")
            .and_then(|v| v.as_object())
            .expect("cv_sidecar object missing");
        assert_eq!(cv.get("skipped"), Some(&serde_json::Value::Bool(false)));
        assert!(cv.get("path").and_then(|v| v.as_str()).is_some());
        assert!(cv.get("entries").and_then(|v| v.as_u64()).is_some());
        assert!(cv.get("contributions").and_then(|v| v.as_u64()).is_some());
        assert!(cv.get("tombstones").and_then(|v| v.as_u64()).is_some());
        assert!(cv.get("pass_order").and_then(|v| v.as_array()).is_some());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_summary_format_kv_emits_key_value_pairs() {
        // KV summary: stdout contains the cv_sidecar.* keys
        // with `=` separators; path and pass_order are quoted.
        let dir =
            std::env::temp_dir().join("closurec_cloc11_74_summary_kv");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_summary: true,
                correlation_vector_summary_format:
                    crate::config::CorrelationVectorSummaryFormat::Kv,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        let line = out.stdout_text.trim();
        assert!(
            line.contains("cv_sidecar.path=\""),
            "expected quoted cv_sidecar.path=\"...\" in KV summary, got: {line}"
        );
        assert!(
            line.contains("cv_sidecar.skipped=false"),
            "expected cv_sidecar.skipped=false in KV summary, got: {line}"
        );
        assert!(
            line.contains("cv_sidecar.entries="),
            "expected cv_sidecar.entries= in KV summary, got: {line}"
        );
        assert!(
            line.contains("cv_sidecar.pass_order=\""),
            "expected quoted cv_sidecar.pass_order=\"...\" in KV summary, got: {line}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_summary_format_text_remains_default() {
        // No format flag set → CLOC11.73 text line, byte-
        // for-byte unchanged.
        let dir =
            std::env::temp_dir().join("closurec_cloc11_74_summary_default");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_summary: true,
                // summary_format defaults to Text
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(
            out.stdout_text.starts_with("cv sidecar:"),
            "expected text prefix `cv sidecar:`, got: {:?}",
            out.stdout_text
        );
        assert!(
            !out.stdout_text.contains("{\"cv_sidecar\""),
            "text default should not emit JSON, got: {:?}",
            out.stdout_text
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.75 — --correlation_vector_summary_stderr
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_summary_stderr_routes_to_stderr_text() {
        // With the stderr flag on, the summary line should
        // appear in stderr_text and NOT in stdout_text.
        let dir =
            std::env::temp_dir().join("closurec_cloc11_75_stderr_on");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_summary: true,
                correlation_vector_summary_stderr: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(
            out.stderr_text.contains("cv sidecar:"),
            "expected summary on stderr_text, got: {:?}",
            out.stderr_text
        );
        assert!(
            !out.stdout_text.contains("cv sidecar:"),
            "expected NO summary on stdout_text under stderr flag, got: {:?}",
            out.stdout_text
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_summary_stderr_off_remains_on_stdout() {
        // Default: stderr flag off → summary stays on stdout
        // (CLOC11.73 behaviour). stderr_text is empty.
        let dir =
            std::env::temp_dir().join("closurec_cloc11_75_stderr_off");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_summary: true,
                // correlation_vector_summary_stderr defaults to false
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(
            out.stdout_text.contains("cv sidecar:"),
            "expected summary on stdout_text by default, got: {:?}",
            out.stdout_text
        );
        assert!(
            out.stderr_text.is_empty(),
            "expected empty stderr_text by default, got: {:?}",
            out.stderr_text
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.76 — --correlation_vector_summary_only
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_summary_only_skips_js_and_map_and_manifest() {
        // summary_only=true + format=NONE: no JS file on disk,
        // no source map file, no manifest file, no CV sidecar.
        // wrote_files should be empty. Summary still emitted.
        let dir =
            std::env::temp_dir().join("closurec_cloc11_76_summary_only");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let map_path = dir.join("out.js.map");
        let manifest_path = dir.join("manifest.txt");
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
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_summary: true,
                correlation_vector_summary_only: true,
                correlation_vector_format:
                    crate::config::CorrelationVectorFormat::None,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        // No files written anywhere.
        assert!(
            !out_path.exists(),
            "summary_only should skip JS write, found file at {:?}",
            out_path
        );
        assert!(
            !map_path.exists(),
            "summary_only should skip source map, found file at {:?}",
            map_path
        );
        assert!(
            !manifest_path.exists(),
            "summary_only should skip manifest, found file at {:?}",
            manifest_path
        );
        assert!(
            out.wrote_files.is_empty(),
            "summary_only should produce empty wrote_files, got: {:?}",
            out.wrote_files
        );
        // Summary line still appears.
        assert!(
            out.stdout_text.contains("cv sidecar:"),
            "expected summary line under summary_only, got: {:?}",
            out.stdout_text
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_summary_only_off_writes_outputs_normally() {
        // Default (summary_only=false): JS file is written
        // even when summary is on. Tests the
        // byte-for-byte-unchanged guarantee.
        let dir = std::env::temp_dir().join("closurec_cloc11_76_only_off");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create dir");
        let in_path = dir.join("a.js");
        fs::write(&in_path, "var x = 1;").expect("write");
        let out_path = dir.join("out.js");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                correlation_vector_summary: true,
                // correlation_vector_summary_only defaults to false
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        assert!(
            out_path.exists(),
            "default summary_only=false should still write JS, got nothing at {:?}",
            out_path
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.62 — CV records for post-combine stages (emit_use_strict,
    // output_wrapper, isolation_mode, charset)
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_creates_combined_entry_after_per_file_loop() {
        // The "concatenated_combined_source" CV entry should
        // appear in the sidecar, with the per-file entries as
        // parents. Lets us walk a downstream byte's provenance
        // back to its source file(s).
        let dir = temp_path("cv-combined-dir");
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
        assert!(
            body.contains("\"source\":\"concatenated_combined_source\""),
            "missing combined entry: {body}"
        );
        // file_count meta should show 1.
        assert!(body.contains("\"file_count\":1"), "got: {body}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_records_emit_use_strict_contribution() {
        let dir = temp_path("cv-strict-dir");
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
            language: crate::config::LanguageConfig {
                emit_use_strict: true,
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
        assert!(
            body.contains("\"source\":\"emit_use_strict\""),
            "missing emit_use_strict source: {body}"
        );
        assert!(
            body.contains("\"tag\":\"prepended\""),
            "missing prepended tag: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_does_not_record_emit_use_strict_when_unset() {
        let dir = temp_path("cv-no-strict-dir");
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
            // emit_use_strict NOT set → no contribution.
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        assert!(
            !body.contains("emit_use_strict"),
            "should NOT contain emit_use_strict when flag is off: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_records_output_wrapper_when_bytes_changed() {
        let dir = temp_path("cv-wrapper-dir");
        fs::create_dir_all(&dir).expect("setup");
        let in_path = dir.join("in.js");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        fs::write(&in_path, "var x;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            formatting: crate::config::FormattingConfig {
                output_wrapper: "PRE %output% POST".to_string(),
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
        assert!(body.contains("\"source\":\"output_wrapper\""), "got: {body}");
        assert!(body.contains("\"tag\":\"substituted\""), "got: {body}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_records_iife_when_isolation_mode_set() {
        let dir = temp_path("cv-iife-dir");
        fs::create_dir_all(&dir).expect("setup");
        let in_path = dir.join("in.js");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        fs::write(&in_path, "var x;").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            formatting: crate::config::FormattingConfig {
                isolation_mode: IsolationMode::Iife,
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
        assert!(body.contains("\"source\":\"isolation_mode\""), "got: {body}");
        assert!(body.contains("\"tag\":\"iife_wrapped\""), "got: {body}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_always_records_charset_with_resolved_mode() {
        // The charset contribution lands even at the default
        // (US_ASCII out, no flag passed) because the stage RAN.
        let dir = temp_path("cv-charset-dir");
        fs::create_dir_all(&dir).expect("setup");
        let in_path = dir.join("in.js");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        fs::write(&in_path, "var x;").expect("setup");
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
        assert!(body.contains("\"source\":\"charset\""), "got: {body}");
        assert!(body.contains("\"tag\":\"normalized\""), "got: {body}");
        // The default mode is US_ASCII; should appear in meta.
        assert!(body.contains("\"mode\":\"US_ASCII\""), "got: {body}");
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC11.63 — CV records for output writes (JS, source map, manifest)
    // ------------------------------------------------------------------

    #[test]
    fn correlation_vector_records_js_output_file_entry() {
        // When --js_output_file is set and CV is on, a
        // `js_output_file` derived entry should appear in the
        // sidecar with combined as parent.
        let dir = temp_path("cv-js-write-dir");
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
        assert!(
            body.contains("\"source\":\"js_output_file\""),
            "missing js_output_file entry: {body}"
        );
        assert!(
            body.contains("\"source\":\"write_output_file\""),
            "missing write_output_file contribution: {body}"
        );
        assert!(body.contains("\"tag\":\"wrote\""), "got: {body}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_no_js_output_file_entry_when_stdout() {
        // When --js_output_file is absent (stdout), no
        // `js_output_file` entry — the JS didn't write to disk.
        let _cwd_guard = CWD_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prev_cwd = std::env::current_dir().expect("cwd");
        let dir = temp_path("cv-js-stdout-dir");
        fs::create_dir_all(&dir).expect("setup");
        std::env::set_current_dir(&dir).expect("chdir");

        let in_path = dir.join("in.js");
        fs::write(&in_path, "var x = 1;").expect("setup");
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
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string("closurec-cv.json").expect("read sidecar");
        assert!(
            !body.contains("\"source\":\"js_output_file\""),
            "should NOT contain js_output_file when stdout: {body}"
        );

        std::env::set_current_dir(prev_cwd).expect("restore cwd");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_records_source_map_output_entry() {
        let dir = temp_path("cv-smap-write-dir");
        fs::create_dir_all(&dir).expect("setup");
        let in_path = dir.join("in.js");
        let out_path = dir.join("out.js");
        let map_path = dir.join("out.js.map");
        let sidecar_path = dir.join("out.js.cv.json");
        fs::write(&in_path, "var x = 1;").expect("setup");
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
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let _ = run_compiler(&cfg).expect("ok");
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        assert!(
            body.contains("\"source\":\"source_map_output\""),
            "missing source_map_output entry: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correlation_vector_records_manifest_output_entry_with_per_file_parents() {
        // Manifest CV entry should be created via merge() with
        // per-file IDs as parents (NOT combined as parent — the
        // manifest enumerates input files, not the merged output).
        let dir = temp_path("cv-manifest-write-dir");
        fs::create_dir_all(&dir).expect("setup");
        let a = dir.join("a.js");
        let b = dir.join("b.js");
        let out_path = dir.join("out.js");
        let manifest_path = dir.join("manifest.txt");
        let sidecar_path = dir.join("out.js.cv.json");
        fs::write(&a, "var a;").expect("a");
        fs::write(&b, "var b;").expect("b");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![
                    a.to_string_lossy().to_string(),
                    b.to_string_lossy().to_string(),
                ],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            chunks: crate::config::ChunksConfig {
                output_manifest_file: Some(manifest_path.clone()),
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
        assert!(
            body.contains("\"source\":\"manifest_output\""),
            "missing manifest_output entry: {body}"
        );
        // file_count should be 2.
        assert!(body.contains("\"file_count\":2"), "got: {body}");
        let _ = fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------------------
    // CLOC12.137 — SIMPLE level routes through the typed-AST bridge
    // ------------------------------------------------------------------

    #[test]
    fn simple_level_strips_whitespace_not_identity() {
        // SIMPLE output must be whitespace-stripped (not the raw
        // source). With no foldable expression the typed pipeline
        // emits the same compact form whitespace_only would.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        // `x` is referenced so remove-unused-vars keeps it; the point
        // here is the whitespace normalisation, not removal.
        let source = "var  x   =   1 ; use(x);";
        let out = transform_source(source, &cfg).expect("ok");
        // The typed emitter produces the compact `var x=1;use(x);`.
        assert_ne!(out, source, "SIMPLE must not return the raw source");
        assert_eq!(out, "var x=1;use(x);");
    }

    #[test]
    fn simple_level_constant_folds_arithmetic() {
        // CLOC12.155: the SIMPLE pipeline runs constant-fold, so a
        // literal arithmetic expression is evaluated at compile time.
        // This is the load-bearing difference from whitespace_only —
        // `1 + 2` becomes `3`, not `1+2`. If the bridge/pipeline/emit
        // chain ever silently degraded, this would regress to `1+2`.
        //
        // `x` is referenced (`use(x)`) so remove-unused-vars — now last
        // in the SIMPLE pipeline — keeps the declaration; otherwise the
        // whole `var x` would be removed and the fold wouldn't be visible.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = transform_source("var x = 1 + 2; use(x);", &cfg).expect("ok");
        assert_eq!(out, "var x=3;use(x);", "constant-fold must evaluate 1 + 2");
    }

    #[test]
    fn simple_level_whitespace_only_leaves_arithmetic_unfolded() {
        // Companion to the test above — the SAME input under
        // WHITESPACE_ONLY keeps `1+2` literally, proving the fold is
        // the SIMPLE pipeline's doing and not the lexer/emitter.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::WhitespaceOnly,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = transform_source("var x = 1 + 2;", &cfg).expect("ok");
        assert_eq!(out, "var x=1+2;", "whitespace_only must NOT fold");
    }

    #[test]
    fn simple_fold_control_flow_prunes_dead_branch() {
        // CLOC12.156: the SIMPLE pipeline now runs fold-control-flow
        // after constant-fold, so an `if` with a statically-false
        // condition keeps only its `else` branch. The `2 > 3` form is
        // the load-bearing case: constant-fold turns `2 > 3` into
        // `false`, and only then can fold-control-flow decide the
        // branch — so this exercises the two passes composing.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = transform_source("if (2 > 3) { a(); } else { b(); }", &cfg).expect("ok");
        assert_eq!(out, "{b()}", "fold-control-flow must keep only the else branch");
    }

    #[test]
    fn simple_fold_control_flow_whitespace_only_keeps_if() {
        // Companion — the SAME input under WHITESPACE_ONLY keeps the
        // whole `if`/`else`, proving the pruning is the SIMPLE
        // pipeline's doing and not the lexer/emitter.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::WhitespaceOnly,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = transform_source("if (2 > 3) { a(); } else { b(); }", &cfg).expect("ok");
        assert_eq!(
            out, "if(2>3)a();else b();",
            "whitespace_only must NOT prune the branch"
        );
    }

    #[test]
    fn simple_dce_drops_dead_after_return() {
        // CLOC12.157: the SIMPLE pipeline now runs dce after
        // fold-control-flow. Code after a `return` in a block is
        // unreachable, so dce removes it.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        // `f` is called so treeshake (now last in the SIMPLE pipeline)
        // keeps the declaration; otherwise the unused function would be
        // removed wholesale and the dce-inside-the-body effect wouldn't
        // be observable. It is called TWICE so the single-use void
        // statement-inliner (CLOC15) declines it — otherwise `f` would be
        // spliced away entirely and the dce-after-return effect (the point
        // of this test) would no longer be observable in the output.
        let out = transform_source("function f() { g(); return 1; dead(); } f(); f();", &cfg)
            .expect("ok");
        assert_eq!(
            out, "function f(){g();return 1};f();f();",
            "dce must drop the statement after the return"
        );
    }

    #[test]
    fn simple_dce_sweeps_folded_if_empty_statement() {
        // All three passes compose: constant-fold turns `4 > 5` into
        // `false`, fold-control-flow turns `if (false) {…}` into an
        // empty `;`, and dce sweeps that empty statement out of the
        // block — leaving just the `return`.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        // `f` is referenced so treeshake keeps the declaration. The
        // extra value use `sink(f)` (not a call) makes the inliner
        // decline `f` — uses != inlinable-calls — so this test stays
        // focused on DCE's empty-statement sweep rather than inlining.
        let out = transform_source(
            "function f() { if (4 > 5) { x(); } return 2; } f(); sink(f);",
            &cfg,
        )
        .expect("ok");
        assert_eq!(
            out, "function f(){return 2};f();sink(f);",
            "dce must sweep the empty statement left by fold-control-flow"
        );
    }

    #[test]
    fn simple_dce_whitespace_only_keeps_dead_code() {
        // Companion — the SAME input under WHITESPACE_ONLY keeps the
        // dead statement, proving the elimination is the SIMPLE
        // pipeline's doing.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::WhitespaceOnly,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = transform_source("function f() { g(); return 1; dead(); }", &cfg)
            .expect("ok");
        assert_eq!(
            out, "function f(){g();return 1;dead()};",
            "whitespace_only must NOT drop dead code"
        );
    }

    #[test]
    fn simple_remove_unused_drops_dead_top_level_var() {
        // CLOC12.158: the SIMPLE pipeline now ends with
        // remove-unused-vars. An unreferenced top-level `var` with a
        // pure (literal) initializer is deleted entirely.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = transform_source("var unused = 1; used();", &cfg).expect("ok");
        assert_eq!(out, "used();", "the unused var must be removed");
    }

    #[test]
    fn simple_remove_unused_composes_with_constant_fold() {
        // `var x = 1 + 2; sideEffect();` — constant-fold turns the
        // initializer into the literal `3`, and only then does
        // remove-unused-vars see a pure init it can drop. Proves the
        // two passes compose end to end.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = transform_source("var x = 1 + 2; sideEffect();", &cfg).expect("ok");
        assert_eq!(out, "sideEffect();", "folded-then-dead var must be removed");
    }

    #[test]
    fn simple_remove_unused_keeps_impure_initializer() {
        // `var impure = run();` — unreferenced, but the call initializer
        // may have a side effect, so the purity gate keeps it.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = transform_source("var impure = run();", &cfg).expect("ok");
        assert_eq!(
            out, "var impure=run();",
            "a dead var with a side-effecting initializer must be kept"
        );
    }

    #[test]
    fn simple_remove_unused_whitespace_only_keeps_var() {
        // Companion — the SAME unused var under WHITESPACE_ONLY survives,
        // proving the removal is the SIMPLE pipeline's doing.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::WhitespaceOnly,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = transform_source("var unused = 1; used();", &cfg).expect("ok");
        assert_eq!(out, "var unused=1;used();", "whitespace_only must NOT remove");
    }

    #[test]
    fn simple_treeshake_drops_unused_function() {
        // CLOC12.159: the SIMPLE pipeline now ends with treeshake, which
        // deletes a top-level function declaration nothing references.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        let out =
            transform_source("function dead() { return 1; } used();", &cfg).expect("ok");
        assert_eq!(out, "used();", "the unused function must be removed");
    }

    #[test]
    fn simple_treeshake_keeps_called_function() {
        // A function that IS referenced (called) survives.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        // The value use `sink(f)` (not a call) makes the inliner decline
        // `f`, so this test isolates treeshake's keep-referenced
        // behaviour rather than exercising inlining.
        let out = transform_source("function f() { return 2; } log(f()); sink(f);", &cfg)
            .expect("ok");
        assert_eq!(
            out, "function f(){return 2};log(f());sink(f);",
            "a called function must be kept"
        );
    }

    #[test]
    fn simple_treeshake_whitespace_only_keeps_function() {
        // Companion — the SAME unused function under WHITESPACE_ONLY
        // survives, proving the removal is the SIMPLE pipeline's doing.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::WhitespaceOnly,
                ..Default::default()
            },
            ..Default::default()
        };
        let out =
            transform_source("function dead() { return 1; } used();", &cfg).expect("ok");
        assert_eq!(
            out, "function dead(){return 1};used();",
            "whitespace_only must NOT remove the function"
        );
    }

    #[test]
    fn simple_rename_shortens_leaf_function_params() {
        // CLOC12.160: the SIMPLE pipeline now ends with rename, which
        // shortens a leaf function's parameter names. The function is
        // called so treeshake keeps it; the top-level name `f` is kept,
        // its parameter `longName` becomes `a`.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        // The value use `sink(f)` (not a call) makes the inliner decline
        // `f`, so this test isolates rename's parameter-shortening.
        let out = transform_source(
            "function f(longName) { return longName + 1; } f(5); sink(f);",
            &cfg,
        )
        .expect("ok");
        assert_eq!(out, "function f(a){return a+1};f(5);sink(f);");
    }

    #[test]
    fn simple_rename_keeps_property_names() {
        // Rename must not touch property names: `obj.longName` keeps its
        // `.longName`; only the parameter `obj` is shortened.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        // The value use `sink(f)` (not a call) makes the inliner decline
        // `f`, so this test isolates rename's property-name preservation.
        let out = transform_source(
            "function f(obj) { return obj.longName; } f(x); sink(f);",
            &cfg,
        )
        .expect("ok");
        assert_eq!(out, "function f(a){return a.longName};f(x);sink(f);");
    }

    #[test]
    fn simple_binary_operator_spacing() {
        // Symbolic binary/logical operators emit tight in compact mode (matching
        // upstream Closure), word operators keep their spaces, and the additive
        // sign hazard keeps the one space it needs to avoid an `++`/`--` merge.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        let simple = |src: &str| transform_source(src, &cfg).expect("ok");

        assert_eq!(simple("x=a+b*c;"), "x=a+b*c;");
        assert_eq!(simple("x=a&&b;"), "x=a&&b;");
        assert_eq!(simple("x=a<<b;"), "x=a<<b;");
        assert_eq!(simple("x=a===b;"), "x=a===b;");
        // Word operators MUST keep spaces.
        assert_eq!(simple("x=a instanceof b;"), "x=a instanceof b;");
        assert_eq!(simple("x=a in b;"), "x=a in b;");
        // Sign hazard: `a+ +b` must NOT become `a++b`.
        assert_eq!(simple("x=a+ +b;"), "x=a+ +b;");
        assert_eq!(simple("x=a- -b;"), "x=a- -b;");
        // Parenthesisation is unaffected (precedence still correct).
        assert_eq!(simple("x=a-(b-c);"), "x=a-(b-c);");
        assert_eq!(simple("x=(a||b)&&c;"), "x=(a||b)&&c;");
    }

    #[test]
    fn simple_array_elisions_preserved() {
        // End-to-end regression: array holes used to be dropped at the bridge,
        // changing the array's length and index membership (a miscompile). They
        // must survive SIMPLE minification byte-for-byte.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        let simple = |src: &str| transform_source(src, &cfg).expect("ok");

        assert_eq!(simple("f([1,,3]);"), "f([1,,3]);"); // internal hole, len 3
        assert_eq!(simple("f([,,]);"), "f([,,]);"); // two holes, len 2
        assert_eq!(simple("f([1,,]);"), "f([1,,]);"); // trailing hole, len 2
        assert_eq!(simple("f([,1]);"), "f([,1]);"); // leading hole, len 2
        assert_eq!(simple("f([1,,,2]);"), "f([1,,,2]);"); // two holes, len 4
        // A trailing comma after an element is NOT a hole — it is dropped.
        assert_eq!(simple("f([1,2,3,]);"), "f([1,2,3]);");
        assert_eq!(simple("f([1,2,3]);"), "f([1,2,3]);");
    }

    #[test]
    fn simple_exponentiation_operand_precedence() {
        // `**` is right-associative and its base must bind tighter than unary
        // (the grammar base is an UpdateExpression). A unary base therefore needs
        // parens — `-a**2` is a SyntaxError; the correct form is `(-a)**2`. The
        // emitter previously emitted the invalid `-a**2`. The right operand, by
        // contrast, accepts the same precedence, so `a**b**c` needs no parens.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        let simple = |src: &str| transform_source(src, &cfg).expect("ok");

        // Unary base MUST be parenthesised (else invalid JS).
        assert_eq!(simple("x=(-a)**2;"), "x=(-a)**2;");
        assert_eq!(simple("x=(~a)**2;"), "x=(~a)**2;");
        assert_eq!(simple("x=(!a)**2;"), "x=(!a)**2;");
        // Lower-precedence base stays parenthesised.
        assert_eq!(simple("x=(a||b)**2;"), "x=(a||b)**2;");
        // Right-associative: same-precedence right needs NO parens (was
        // over-parenthesised as `a**(b**c)`).
        assert_eq!(simple("x=a**b**c;"), "x=a**b**c;");
        // A unary RIGHT operand is legal without parens.
        assert_eq!(simple("x=a**-b;"), "x=a**-b;");
        assert_eq!(simple("x=a**b;"), "x=a**b;");
    }

    #[test]
    fn simple_member_and_call_object_keeps_required_parens() {
        // Regression: the emitter wrote a member-expression's object (and a
        // call's callee) at parent precedence 0, dropping the parentheses that
        // make a lower-precedence object a unit — `(a||b).c` became `a||b.c`
        // (`a||(b.c)`), a miscompile. The object/callee is now emitted at
        // PREC_PRIMARY, so the parens survive while plain `a.b.c` stays bare.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        let simple = |src: &str| transform_source(src, &cfg).expect("ok");

        // Member object: lower-precedence objects must keep their parens.
        assert_eq!(simple("x=(a||b).c;"), "x=(a||b).c;");
        assert_eq!(simple("x=(a+b).c;"), "x=(a+b).c;");
        assert_eq!(simple("x=(a?b:c).d;"), "x=(a?b:c).d;");
        assert_eq!(simple("x=(-a).b;"), "x=(-a).b;");
        assert_eq!(simple("x=(a||b)[c];"), "x=(a||b)[c];"); // computed member too
        // Call callee: same requirement.
        assert_eq!(simple("x=(a||b)();"), "x=(a||b)();");
        assert_eq!(simple("x=(-a).b();"), "x=(-a).b();");
        // High-precedence objects/callees stay paren-free.
        assert_eq!(simple("a.b.c;"), "a.b.c;");
        assert_eq!(simple("a.b();"), "a.b();");
        assert_eq!(simple("x=a.b;"), "x=a.b;");
    }

    #[test]
    fn simple_object_string_keys_quote_handling() {
        // End-to-end regression for the property-key miscompile: the bridge used
        // to emit EVERY quoted object key as a bare identifier from un-decoded
        // text. Now a quoted key drops its quotes ONLY when its decoded value is a
        // valid identifier and is not `__proto__`; otherwise it stays a string.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        let simple = |src: &str| transform_source(src, &cfg).expect("ok");

        // Valid identifier → quotes dropped (the minification we keep).
        assert_eq!(simple("f({\"abc\":1});"), "f({abc:1});");
        // Reserved words are legal property names → also shortened.
        assert_eq!(simple("f({\"if\":1});"), "f({if:1});");
        // Non-identifier keys MUST stay quoted (bare would be invalid JS / a
        // different key).
        assert_eq!(simple("f({\"a-b\":1});"), "f({\"a-b\":1});");
        assert_eq!(simple("f({\"a b\":1});"), "f({\"a b\":1});");
        assert_eq!(simple("f({\"123\":1});"), "f({\"123\":1});");
        // Escapes decode then re-escape correctly (single, not double, backslash).
        assert_eq!(simple("f({\"x\\ty\":1});"), "f({\"x\\ty\":1});");
        // `__proto__` stays quoted: the bare form is the prototype setter — a
        // DIFFERENT object — so dropping the quotes would change semantics.
        assert_eq!(simple("f({\"__proto__\":1});"), "f({\"__proto__\":1});");
    }

    #[test]
    fn simple_rename_whitespace_only_keeps_param_names() {
        // Companion — the SAME input under WHITESPACE_ONLY keeps the full
        // parameter name, proving the renaming is the SIMPLE pipeline's.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::WhitespaceOnly,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = transform_source("function f(longName) { return longName + 1; } f(5);", &cfg)
            .expect("ok");
        assert_eq!(out, "function f(longName){return longName+1};f(5);");
    }

    #[test]
    fn advanced_optimizes_like_simple() {
        // CLOC12.161: ADVANCED was a literal no-op (returned the source
        // verbatim). It now runs the typed pipeline; this input is folded
        // and renamed instead of passed through. The value use `sink(f)`
        // (not a call) makes the inliner decline `f`, keeping the assert
        // on rename's parameter-shortening.
        let src = "function f(longName) { return longName + 1; } f(5); sink(f);";
        let advanced = transform_source(
            src,
            &CompilerConfig {
                compilation: crate::config::CompilationConfig {
                    level: crate::config::CompilationLevel::Advanced,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("ok");
        assert_eq!(advanced, "function f(a){return a+1};f(5);sink(f);");
        assert_ne!(advanced, src, "ADVANCED must no longer be an identity no-op");
    }

    #[test]
    fn advanced_matches_simple_output() {
        // ADVANCED adds `rename-globals` on top of the SIMPLE pipeline, but
        // it only differs when a top-level name SURVIVES to the end. Here
        // `g` is a single-use leaf function, so `inline`/`treeshake`
        // delete it entirely — nothing top-level remains, so ADVANCED and
        // SIMPLE produce identical output for THIS input. See
        // `advanced_renames_surviving_top_level_function` for the divergent
        // case.
        let src = "var dead = 1 + 2; function g(value) { return value * 2; } use(g(4));";
        let mk = |level| {
            transform_source(
                src,
                &CompilerConfig {
                    compilation: crate::config::CompilationConfig {
                        level,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .expect("ok")
        };
        assert_eq!(
            mk(crate::config::CompilationLevel::Advanced),
            mk(crate::config::CompilationLevel::Simple),
        );
    }

    #[test]
    fn advanced_renames_surviving_top_level_function() {
        // CLOC13.I: a top-level function that SURVIVES SIMPLE (multi-
        // statement body, and called MORE THAN ONCE so neither the
        // expression inliner nor the single-use void statement-inliner
        // (CLOC15) splices it away; `treeshake` keeps it) is shortened by
        // ADVANCED's `rename-globals` pass — the first point where ADVANCED
        // produces smaller output than SIMPLE. SIMPLE keeps the top-level
        // name (it may be external).
        let src = "function helper() { sideEffect(); return value; } helper(); helper();";
        let mk = |level| {
            transform_source(
                src,
                &CompilerConfig {
                    compilation: crate::config::CompilationConfig {
                        level,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .expect("ok")
        };
        let simple = mk(crate::config::CompilationLevel::Simple);
        let advanced = mk(crate::config::CompilationLevel::Advanced);
        assert_eq!(
            simple,
            "function helper(){sideEffect();return value};helper();helper();"
        );
        assert_eq!(
            advanced,
            "function a(){sideEffect();return value};a();a();"
        );
        assert!(
            advanced.len() < simple.len(),
            "ADVANCED must be smaller than SIMPLE here"
        );
    }

    // ------------------------------------------------------------------
    // CLOC13.K — ADVANCED property renaming, gated on --externs
    //
    // `rename-properties` shortens program-private property names. It is
    // unsafe to run unconditionally (the bundled built-in list omits the
    // DOM, so `el.innerHTML`/`node.onload` would be renamed and break
    // browser code), so closurec runs it ONLY when the user supplies at
    // least one `--externs` file — opting into the externs contract AND
    // providing the host/DOM property boundary. These three tests pin the
    // policy: SIMPLE and no-externs ADVANCED leave properties alone;
    // ADVANCED + externs renames a private property while keeping an
    // externs-declared one.
    // ------------------------------------------------------------------

    /// Top-level program whose property accesses survive every structural
    /// pass: `secretField` is program-private (renameable), `innerHTML` is
    /// declared in the externs file below (must be kept). `read`/`obj` are
    /// free globals, untouched by `rename-globals`.
    const PROP_INPUT: &str =
        "read(obj.innerHTML);\nread(obj.secretField);\nread(obj.secretField);\n";
    /// An externs file declaring `innerHTML` as part of the external
    /// surface (and nothing about `secretField`).
    const PROP_EXTERNS: &str = "var node;\nread(node.innerHTML);\n";
    /// An externs file that fails to PARSE (a `function` keyword with no
    /// name, parameter list, or body — a hard syntax error). Loading it
    /// fails, so the property boundary cannot be established — property
    /// renaming must fail closed (stay OFF).
    ///
    /// NOTE: this used to be `"node.innerHTML = 1;"`, which was "unparseable"
    /// only because of the CLOC17 assignment-expression grammar gap. Once
    /// that gap was fixed (the `assignment_expression` PEG alternatives were
    /// reordered so a member-assignment statement parses), that string became
    /// a *valid* externs file — `innerHTML` is then a legitimate boundary
    /// property and the fail-closed path was no longer exercised. We now use a
    /// genuinely malformed snippet so the fail-closed safety property is still
    /// tested independent of which expression forms parse.
    const BAD_EXTERNS: &str = "function {{{\n";

    /// Compile `PROP_INPUT` at `level`, optionally with an externs file
    /// whose body is `externs`, and return stdout. Writes real temp files
    /// because the externs boundary is collected from `--externs` paths on
    /// disk. `label` makes the temp paths unique per call so
    /// concurrently-running tests (cargo runs them on multiple threads)
    /// never share — and delete — a file.
    fn compile_prop(
        level: crate::config::CompilationLevel,
        externs: Option<&str>,
        label: &str,
    ) -> String {
        let in_path = temp_path(&format!("prop-{label}-in.js"));
        fs::write(&in_path, PROP_INPUT).expect("setup");
        let mut io = IoConfig {
            js_patterns: vec![in_path.to_string_lossy().to_string()],
            ..Default::default()
        };
        let ext_path = temp_path(&format!("prop-{label}-ext.js"));
        if let Some(body) = externs {
            fs::write(&ext_path, body).expect("setup");
            io.externs = vec![ext_path.to_string_lossy().to_string()];
        }
        let cfg = CompilerConfig {
            io,
            compilation: crate::config::CompilationConfig {
                level,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok").stdout_text;
        let _ = fs::remove_file(&in_path);
        let _ = fs::remove_file(&ext_path);
        out
    }

    #[test]
    fn simple_does_not_rename_properties() {
        // SIMPLE never renames properties — both names survive verbatim.
        let out = compile_prop(crate::config::CompilationLevel::Simple, None, "simple");
        assert!(out.contains(".innerHTML"), "got: {out}");
        assert!(out.contains(".secretField"), "got: {out}");
    }

    #[test]
    fn advanced_without_externs_does_not_rename_properties() {
        // ADVANCED with NO --externs leaves properties untouched: there is
        // no host/DOM property boundary, so renaming would be unsound.
        let out = compile_prop(crate::config::CompilationLevel::Advanced, None, "adv-noext");
        assert!(out.contains(".innerHTML"), "got: {out}");
        assert!(out.contains(".secretField"), "got: {out}");
    }

    #[test]
    fn advanced_with_externs_renames_private_property_only() {
        // ADVANCED + --externs: `secretField` (program-private) is renamed
        // away; `innerHTML` (declared in the externs file) is kept.
        let out = compile_prop(
            crate::config::CompilationLevel::Advanced,
            Some(PROP_EXTERNS),
            "adv-ext",
        );
        assert!(
            out.contains(".innerHTML"),
            "externs-declared property must be kept; got: {out}"
        );
        assert!(
            !out.contains("secretField"),
            "private property must be renamed away; got: {out}"
        );
        // And it actually shrank versus the no-externs ADVANCED output.
        let baseline = compile_prop(crate::config::CompilationLevel::Advanced, None, "adv-base");
        assert!(
            out.len() < baseline.len(),
            "property renaming must shrink output: {} vs {}",
            out.len(),
            baseline.len()
        );
    }

    #[test]
    fn advanced_with_unparseable_externs_disables_property_renaming() {
        // FAIL-CLOSED: the user supplied `--externs`, but the file fails to
        // parse, so the property boundary can't be established. Property
        // renaming must NOT run against an empty/partial boundary — doing so
        // would rename an externally-observable property (a miscompile of
        // valid input). Both names must survive untouched, exactly as if no
        // externs had been supplied.
        let out = compile_prop(
            crate::config::CompilationLevel::Advanced,
            Some(BAD_EXTERNS),
            "adv-bad",
        );
        assert!(
            out.contains(".innerHTML"),
            "must not rename when externs failed to load; got: {out}"
        );
        assert!(
            out.contains(".secretField"),
            "must not rename when externs failed to load; got: {out}"
        );
    }

    #[test]
    fn simple_pipeline_iterates_to_a_fixed_point() {
        // CLOC13.F: the pipeline now runs to a fixed point. `inline`
        // turns the single-use `double(7)` into `7 * 2` (sweep 1), and
        // `constant-fold` — which ran *before* inline in sweep 1 — folds
        // it to `14` on sweep 2. Before fixed-point iteration the
        // pipeline ran each pass once and stopped at `log(7 * 2);`.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = transform_source("function double(x) { return x * 2; } log(double(7));", &cfg)
            .expect("ok");
        assert_eq!(
            out, "log(14);",
            "inline → fold cascade must converge to the folded constant"
        );
    }

    #[test]
    fn simple_inlines_small_function_at_multiple_sites() {
        // CLOC13.G: a small pure function (`x * x`, within the size
        // budget) is inlined at BOTH call sites, treeshake removes it,
        // and constant-fold folds the literal results.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = transform_source("function sq(x) { return x * x; } a(sq(3)); b(sq(4));", &cfg)
            .expect("ok");
        assert_eq!(out, "a(9);b(16);");
    }

    #[test]
    fn simple_propagates_const_literal_and_removes_binding() {
        // CLOC13.H: a top-level `const` bound to a literal is propagated
        // to its use sites, remove-unused-vars deletes the binding, and
        // constant-fold folds the now-concrete `2 + 1`.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        let out =
            transform_source("const RATE = 2; total(base * RATE); margin(RATE + 1);", &cfg)
                .expect("ok");
        assert_eq!(out, "total(base*2);margin(3);");
    }

    #[test]
    fn simple_level_bridge_ok_status_in_cv() {
        // When the source parses cleanly, the CV contribution for the
        // `compilation_level` stage must carry bridge_status = "ok".
        let dir = temp_path("simple-cv-ok-dir");
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
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
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
        assert!(
            body.contains("\"tag\":\"simple_v2\""),
            "expected simple_v2 tag in CV sidecar: {body}"
        );
        assert!(
            body.contains("\"bridge_status\":\"ok\""),
            "expected bridge_status=ok in CV sidecar: {body}"
        );
        // The pass pipeline must be recorded in the trace, in order.
        assert!(
            body.contains(
                "\"passes\":[\"constant-fold\",\"fold-control-flow\",\"dce\",\"inline\",\"inline-variables\",\"remove-unused-vars\",\"treeshake\",\"rename\"]"
            ),
            "expected passes list in CV sidecar: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn simple_level_unsupported_syntax_degrades_gracefully() {
        // A class declaration hits BridgeError::UnsupportedSyntax.
        // The output must still be whitespace-only (degrade, not error)
        // and the CV must record bridge_status starting with
        // "unsupported_syntax:".
        let dir = temp_path("simple-cv-unsupported-dir");
        fs::create_dir_all(&dir).expect("setup");
        let in_path = dir.join("in.js");
        let out_path = dir.join("out.js");
        let sidecar_path = dir.join("out.js.cv.json");
        // with-statement: the grammar parses it, but bridge::grammar_to_program
        // returns UnsupportedSyntax (with_statement is still Phase 2+ — and
        // renaming-unsafe, so it is a deliberate non-target). do-while (CLOC20),
        // for-in (CLOC22), and for-of (CLOC23) are all supported now and no
        // longer degrade; class declarations fail at the grammar parser level,
        // not the bridge.
        fs::write(&in_path, "with (obj) { x(); }").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            special_modes: crate::config::SpecialModesConfig {
                correlation_vector: true,
                ..Default::default()
            },
            ..Default::default()
        };
        // Must not return Err — UnsupportedSyntax is a graceful degrade.
        let out = run_compiler(&cfg).expect("simple must not error on unsupported syntax");
        // Output must be non-empty (whitespace_only was applied).
        assert!(
            !out.stdout_text.is_empty() || out.wrote_files.contains(&out_path),
            "expected output file written"
        );
        let body = fs::read_to_string(&sidecar_path).expect("read sidecar");
        // Even on a degrade the stage is still tagged simple_v2 — the
        // tag names the level's pipeline version, not the outcome.
        assert!(
            body.contains("\"tag\":\"simple_v2\""),
            "expected simple_v2 tag: {body}"
        );
        assert!(
            body.contains("\"bridge_status\":\"unsupported_syntax:"),
            "expected unsupported_syntax in bridge_status: {body}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn simple_level_bridge_status_n_a_without_cv() {
        // Without CV enabled, simple_bridge_status is computed but not
        // written anywhere. Verify the pipeline still runs correctly.
        let cfg = CompilerConfig {
            compilation: crate::config::CompilationConfig {
                level: crate::config::CompilationLevel::Simple,
                ..Default::default()
            },
            ..Default::default()
        };
        // `x` is referenced so remove-unused-vars keeps it.
        let out = transform_source("var x = 1; use(x);", &cfg).expect("ok");
        assert_eq!(out, "var x=1;use(x);");
    }
}
