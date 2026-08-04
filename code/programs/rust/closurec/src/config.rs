//! `CompilerConfig` — the typed bridge between `cli-builder`'s
//! parsed flags and the compiler pipeline.
//!
//! # Why this exists
//!
//! `cli-builder` hands us a [`ParseResult`] with a
//! `HashMap<String, serde_json::Value>` of every parsed flag value.
//! That map is the *raw* surface, fine for parsing, terrible for
//! running a compiler:
//!
//! - Every consumer of "what's `--compilation_level` set to?" would
//!   have to re-derive an enum from a string, re-validate, re-handle
//!   defaults. One bug per consumer.
//! - The map doesn't *speak the compiler's language*. The pipeline
//!   doesn't think in JSON; it thinks in `CompilationLevel::Advanced`,
//!   `LanguageVersion::Es2020`, `PathBuf`s, and pass-config structs.
//! - There's no central place to enforce flag *combinations* — e.g.
//!   "you can't use `--renaming false` with `--compilation_level
//!   ADVANCED`."
//!
//! `CompilerConfig` is the typed home for parsed-and-validated
//! configuration. Every later track of [CLOC11] adds a row to one
//! of these sub-structs. The architecture is fixed; the wiring grows.
//!
//! # How it's organised
//!
//! One sub-struct per *user-visible compiler feature*, matching the
//! flag groupings in [CLOC11 §4]. Each sub-struct lives next to the
//! flag IDs it owns, so a contributor adding `--rename_variable_prefix`
//! knows exactly which struct to extend (`FormattingConfig`) without
//! grepping `main.rs`.
//!
//! ```text
//!     CompilerConfig
//!     ├── io                  — --js / --js_output_file / --externs / --charset / --jszip
//!     ├── compilation         — --compilation_level / --debug / --renaming / …
//!     ├── language            — --language_in / --language_out / --strict_mode_input / …
//!     ├── formatting          — --output_wrapper / --formatting / --isolation_mode / …
//!     ├── source_map          — --create_source_map / --source_map_* / --apply_input_source_maps
//!     ├── diagnostics         — --warning_level / --jscomp_* / --hide_warnings_for / …
//!     ├── defines             — --define / -D
//!     ├── dependencies        — --dependency_mode / --entry_point / --module_resolution / …
//!     ├── chunks              — --chunk / --chunk_wrapper / --chunk_output_type / …
//!     ├── polyfills           — --rewrite_polyfills / --inject_libraries / …
//!     ├── renaming_reports    — --variable_renaming_report / --property_renaming_report
//!     ├── exports             — --generate_exports / --export_local_property_definitions
//!     ├── conformance         — --conformance_configs
//!     ├── instrumentation     — --instrument_for_coverage_option / --tracer_mode / …
//!     ├── special_modes       — --checks_only / --print_tree / --print_ast / --help_markdown
//!     ├── special_passes      — --angular_pass / --polymer_version / --chrome_pass / --j2cl_pass
//!     ├── translations        — --translations_file / --translations_project
//!     └── json_streams        — --json_streams
//! ```
//!
//! # CLOC11.01 scope
//!
//! This PR lays down **all** the sub-structs (so later PRs only
//! ever *extend* — they never have to introduce new architecture)
//! but only the I/O fields are *consumed* by [`crate::run`]. Every
//! other field is plumbed-through-to-storage so subsequent CLOC11.*
//! PRs can flip behavior on without a parser-layer change.
//!
//! [CLOC11]: ../../../../specs/CLOC11-drop-in-closure-compat.md
//! [CLOC11 §4]: ../../../../specs/CLOC11-drop-in-closure-compat.md#4-flag-inventory
//! [`ParseResult`]: cli_builder::types::ParseResult
//! [`crate::run`]: crate::run

use std::collections::BTreeMap;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Top-level
// ---------------------------------------------------------------------------

/// The complete parsed-and-validated configuration for one
/// `closurec` invocation.
///
/// Constructed by [`crate::wire::config_from_parsed`] from the
/// raw `cli-builder` [`ParseResult`](cli_builder::types::ParseResult);
/// consumed by [`crate::run::run_compiler`].
///
/// Fields are public so consumers can pattern-match. The struct is
/// intentionally large — there are 100 Closure Compiler flags, and
/// hiding them behind methods would obscure rather than illuminate.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompilerConfig {
    pub io: IoConfig,
    pub compilation: CompilationConfig,
    pub language: LanguageConfig,
    pub formatting: FormattingConfig,
    pub source_map: SourceMapConfig,
    pub diagnostics: DiagnosticsConfig,
    pub defines: DefinesConfig,
    pub dependencies: DependenciesConfig,
    pub chunks: ChunksConfig,
    pub polyfills: PolyfillsConfig,
    pub renaming_reports: RenamingReportsConfig,
    pub exports: ExportsConfig,
    pub conformance: ConformanceConfig,
    pub instrumentation: InstrumentationConfig,
    pub special_modes: SpecialModesConfig,
    pub special_passes: SpecialPassesConfig,
    pub translations: TranslationsConfig,
    pub json_streams: JsonStreamsMode,
}

// ---------------------------------------------------------------------------
// I/O — the only sub-struct that's actually consumed in CLOC11.01
// ---------------------------------------------------------------------------

/// Input/output configuration.
///
/// `inputs` is the post-glob-resolved list of source files to read;
/// `output` says where to write. Glob resolution itself is
/// [CLOC11.02]'s territory — in CLOC11.01 we just store the raw
/// `--js` strings here and the runner uses them as literal paths.
///
/// [CLOC11.02]: ../../../../specs/CLOC11-drop-in-closure-compat.md#track-1--end-to-end-identity-build-foundation
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IoConfig {
    /// Raw `--js` values (glob patterns; not yet expanded in
    /// CLOC11.01). Order is preserved — Closure uses input order
    /// as the default dependency order.
    pub js_patterns: Vec<String>,

    /// `--jszip` archive paths. Honored in a later PR.
    pub jszip_paths: Vec<PathBuf>,

    /// `--js_output_file`. `None` means stdout (Closure's default
    /// when the flag is empty or absent).
    pub js_output_file: Option<PathBuf>,

    /// `--externs` file *patterns* (glob-expandable strings).
    /// Held as strings rather than `PathBuf`s because they go
    /// through `globs::expand_js_patterns` in `run_compiler`
    /// (CLOC11.05) — same resolution rules as `--js`. CC's
    /// `--externs` is documented as accepting glob patterns, so
    /// storing the raw strings here preserves user intent until
    /// resolution time.
    pub externs: Vec<String>,

    /// `--env`. Selects which built-in externs to load.
    pub env: EnvKind,

    /// `--charset`. Empty string = use default (UTF-8 in, US-ASCII
    /// out per Closure).
    pub charset: String,
}

/// `--env BROWSER | CUSTOM`. Selects which set of built-in externs
/// to load (browser globals like `window` and `document` for
/// `BROWSER`; nothing for `CUSTOM`, leaving the user to provide
/// all externs).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EnvKind {
    #[default]
    Browser,
    Custom,
}

// ---------------------------------------------------------------------------
// Compilation level
// ---------------------------------------------------------------------------

/// Configuration covering `--compilation_level` and the modifier
/// flags that change how compilation runs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompilationConfig {
    pub level: CompilationLevel,
    pub debug: bool,
    pub renaming: bool,
    pub assume_function_wrapper: bool,
    pub use_types_for_optimization: bool,
    pub continue_after_errors: bool,
    pub checks_only: bool,
}

/// `--compilation_level`. Mirrors Closure's enum exactly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CompilationLevel {
    Bundle,
    WhitespaceOnly,
    #[default]
    Simple,
    TranspileOnly,
    Advanced,
}

// ---------------------------------------------------------------------------
// Language level
// ---------------------------------------------------------------------------

/// `--language_in` / `--language_out` / strict-mode toggles.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LanguageConfig {
    pub language_in: LanguageVersion,
    pub language_out: LanguageVersion,
    pub strict_mode_input: bool,
    pub emit_use_strict: bool,
    /// `--browser_featureset_year`. `0` means "not set"; otherwise
    /// implies `language_out`.
    pub browser_featureset_year: i64,
}

/// `--language_in` and `--language_out` accepted values. The
/// `Stable` and `EcmascriptNext` shortcuts are present because
/// Closure exposes them; they resolve to a specific year at
/// compile time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LanguageVersion {
    Ecmascript3,
    Ecmascript5,
    Ecmascript5Strict,
    Ecmascript2015,
    Ecmascript2016,
    Ecmascript2017,
    Ecmascript2018,
    Ecmascript2019,
    Ecmascript2020,
    Ecmascript2021,
    #[default]
    Stable,
    EcmascriptNext,
    /// Only valid for `--language_in`. Closure uses this when
    /// staging proposals.
    Unstable,
    /// Only valid for `--language_out`.
    NoTranspile,
}

// ---------------------------------------------------------------------------
// Output formatting
// ---------------------------------------------------------------------------

/// Output-shape config — wrappers, IIFE isolation, pretty-printing,
/// rename prefixes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FormattingConfig {
    pub output_wrapper: String,
    pub output_wrapper_file: Option<PathBuf>,
    pub isolation_mode: IsolationMode,
    /// Repeatable `--formatting` enum; multiple flags accumulate.
    pub formatting: Vec<FormattingMode>,
    pub rename_variable_prefix: Option<String>,
    pub rename_prefix_namespace: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IsolationMode {
    #[default]
    None,
    Iife,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormattingMode {
    PrettyPrint,
    PrintInputDelimiter,
    SingleQuotes,
}

// ---------------------------------------------------------------------------
// Source maps
// ---------------------------------------------------------------------------

/// Source-map config. `path_template` empty = no source-map output.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SourceMapConfig {
    /// `--create_source_map`. Empty means "don't write a source map."
    /// Supports `%outname%` substitution.
    pub path_template: String,
    pub format: SourceMapFormat,
    pub include_content: bool,
    /// `--source_map_location_mapping`: repeatable `fs|web` pairs.
    pub location_mappings: Vec<LocationMapping>,
    /// `--source_map_input`: repeatable `input|map` pairs.
    pub inputs: Vec<InputSourceMap>,
    pub parse_inline_source_maps: bool,
    pub apply_input_source_maps: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SourceMapFormat {
    #[default]
    V3,
    Default,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationMapping {
    pub filesystem_path: String,
    pub web_server_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSourceMap {
    pub input_file: PathBuf,
    pub map_file: PathBuf,
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

/// Diagnostic-handling config.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiagnosticsConfig {
    pub warning_level: WarningLevel,
    pub jscomp_error: Vec<String>,
    pub jscomp_warning: Vec<String>,
    pub jscomp_off: Vec<String>,
    pub hide_warnings_for: Vec<String>,
    pub warnings_allowlist_file: Option<PathBuf>,
    pub error_format: ErrorFormat,
    pub summary_detail_level: i64,
    pub third_party: bool,
    pub extra_annotation_names: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WarningLevel {
    Quiet,
    #[default]
    Default,
    Verbose,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ErrorFormat {
    #[default]
    Standard,
}

// ---------------------------------------------------------------------------
// Defines (--define / -D)
// ---------------------------------------------------------------------------

/// `--define NAME=value` and `-D NAME=value` mappings.
///
/// The map preserves insertion order via [`BTreeMap`] (lexical
/// order) — Closure resolves later `--define`s with the same name
/// to "last value wins," but we sort by name to make config-eq
/// comparison meaningful in tests. The runtime applies definitions
/// in the order they appear in the parsed JSON array regardless.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DefinesConfig {
    pub defines: BTreeMap<String, DefineValue>,
}

/// The right-hand side of a `--define NAME=value` flag.
///
/// Closure accepts JS literal forms: numbers, strings, booleans,
/// `null`. Bare `--define NAME` (no `=value`) is shorthand for
/// `--define NAME=true`.
#[derive(Debug, Clone, PartialEq)]
pub enum DefineValue {
    Bool(bool),
    Number(f64),
    String(String),
    Null,
}

// ---------------------------------------------------------------------------
// Dependencies & modules
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DependenciesConfig {
    pub mode: DependencyMode,
    pub entry_points: Vec<String>,
    pub process_closure_primitives: bool,
    pub process_common_js_modules: bool,
    pub js_module_roots: Vec<PathBuf>,
    pub module_resolution: ModuleResolution,
    pub browser_resolver_prefix_replacements: Vec<String>,
    pub package_json_entry_names: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DependencyMode {
    #[default]
    None,
    SortOnly,
    Prune,
    PruneLegacy,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ModuleResolution {
    #[default]
    Browser,
    Node,
    Webpack,
}

// ---------------------------------------------------------------------------
// Chunks
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChunksConfig {
    /// Raw `--chunk <name>:<n>[:<deps>]` strings; parsing into a
    /// chunk graph is [CLOC11.68] work.
    pub chunk_specs: Vec<String>,
    /// `--chunk_wrapper <name>:<wrapper>` strings.
    pub chunk_wrappers: Vec<String>,
    pub output_path_prefix: PathBuf,
    pub output_type: ChunkOutputType,
    pub output_chunk_dependencies_file: Option<PathBuf>,
    pub output_manifest_file: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ChunkOutputType {
    #[default]
    GlobalNamespace,
    EsModules,
}

// ---------------------------------------------------------------------------
// Polyfills
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PolyfillsConfig {
    pub rewrite_polyfills: bool,
    pub isolate_polyfills: bool,
    pub inject_libraries: bool,
    pub force_inject_libraries: Vec<String>,
}

// ---------------------------------------------------------------------------
// Renaming reports
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenamingReportsConfig {
    pub variable_renaming_report: Option<PathBuf>,
    pub property_renaming_report: Option<PathBuf>,
    pub create_renaming_reports: bool,
}

// ---------------------------------------------------------------------------
// Exports
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExportsConfig {
    pub generate_exports: bool,
    pub export_local_property_definitions: bool,
}

// ---------------------------------------------------------------------------
// Conformance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConformanceConfig {
    pub configs: Vec<PathBuf>,
}

// ---------------------------------------------------------------------------
// Instrumentation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InstrumentationConfig {
    pub coverage: CoverageOption,
    pub production_instrumentation_array_name: String,
    pub instrument_mapping_report: Option<PathBuf>,
    pub tracer_mode: TracerMode,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CoverageOption {
    #[default]
    None,
    Line,
    Branch,
    Production,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TracerMode {
    #[default]
    Off,
    All,
    AstSize,
    RawSize,
    TimingOnly,
}

// ---------------------------------------------------------------------------
// Special modes
// ---------------------------------------------------------------------------

/// Modes that *replace* normal compilation rather than running
/// alongside it (e.g. `--print_tree` prints the parse tree and exits;
/// no JS is emitted).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpecialModesConfig {
    pub print_tree: bool,
    pub print_tree_json: bool,
    pub print_ast: bool,
    pub print_source_after_each_pass: bool,
    pub help_markdown: bool,
    /// CLOC11.60: when true, the pipeline threads a
    /// [`coding_adventures_correlation_vector::CVLog`] through
    /// every transform stage so the user (or audit tooling) can
    /// trace any output byte back to its input provenance. When
    /// false (default), the CV log is constructed in disabled
    /// mode (zero overhead — the crate short-circuits every
    /// `create` / `contribute` call).
    pub correlation_vector: bool,
    /// CLOC11.67: explicit path for the correlation-vector
    /// sidecar JSON. When `None` (the default), the writer
    /// falls back to placing the sidecar next to
    /// `--js_output_file` as `<output>.cv.json`, or to
    /// `closurec-cv.json` in the current working directory
    /// when output is stdout. Only consulted when
    /// `correlation_vector` is also true; ignored otherwise.
    pub correlation_vector_output: Option<PathBuf>,
    /// CLOC11.68: when true, the CV sidecar is written as
    /// pretty-printed JSON (multi-line, 2-space indent) rather
    /// than the default compact single-line form. Pretty mode
    /// inflates byte size 3–5× for typical traces but is much
    /// easier to read by hand. CI / build pipelines should
    /// leave this off; humans inspecting a trace should turn
    /// it on.
    ///
    /// Only consulted when `correlation_vector` is true; the
    /// formatter never runs when the trace is disabled.
    pub correlation_vector_pretty: bool,
    /// CLOC11.69: how to format and persist the CV sidecar.
    ///   - `Json` (default): single JSON document, same shape
    ///     as CLOC11.60 → 11.68. The `pretty` toggle still
    ///     applies.
    ///   - `Ndjson`: one CV entry per line, no enclosing
    ///     object. Streaming consumers (jq, log aggregators)
    ///     can `tail -f` mid-build. `pretty` is ignored under
    ///     Ndjson — line-delimited JSON is inherently
    ///     single-line per record.
    ///   - `None`: compute the CV log but do NOT write a
    ///     sidecar. Useful for benchmarking how much the CV
    ///     trace itself costs versus the write.
    pub correlation_vector_format: CorrelationVectorFormat,
    /// CLOC11.70: allowlist of CV `contribution.source`
    /// values. When non-empty, the sidecar serializer prunes
    /// any entry whose `contributions` does NOT include at
    /// least one record whose `source` field appears in this
    /// vector.
    ///
    /// Caveat (documented and tested):
    ///   - Filtering matches against `contribution.source`,
    ///     **not** `origin.source`. So a per-token CV entry
    ///     created with `Origin{source: "lexer_token", ...}`
    ///     but with zero contributions will be pruned by
    ///     `--correlation_vector_filter lex` — its "lex"
    ///     association lives in the Origin, not in a
    ///     contribution. The per-file CV root (which holds
    ///     the `lex.tokens_emitted` contribution) is kept.
    ///     Later slices may extend the filter to also match
    ///     `origin.source` if a use-case demands.
    ///
    /// Empty (the default) → no pruning; every entry is
    /// written. Only consulted when `correlation_vector` is
    /// also true.
    pub correlation_vector_filter: Vec<String>,
    /// CLOC11.71: when true, the filter also matches against
    /// each entry's `origin.source`, not just
    /// `contribution.source` (the CLOC11.70 default).
    ///
    /// Default `false` preserves the CLOC11.70 strict
    /// semantics: per-token CV entries created with
    /// `Origin{source: "lexer_token", ...}` and zero
    /// contributions are *still* pruned under
    /// `--correlation_vector_filter lex` — because their
    /// "lex" association lives in the Origin, not in a
    /// contribution. Setting this flag to true keeps them.
    ///
    /// Only consulted when `correlation_vector` is true AND
    /// `correlation_vector_filter` is non-empty.
    pub correlation_vector_filter_includes_origin: bool,
    /// CLOC11.72: flip the allowlist sense. When `false`
    /// (default), entries are kept iff they match (CLOC11.70
    /// + CLOC11.71 semantics). When `true`, entries are kept
    /// iff they do NOT match — the allowlist becomes a
    /// blocklist.
    ///
    /// Composes orthogonally with `include_origin`:
    /// `include_origin` selects WHICH sources count as a
    /// "match" for an entry (contribution.source only, or
    /// contribution.source ∪ origin.source); `invert` then
    /// decides whether matches are kept or dropped.
    ///
    /// Empty allowlist with `invert=true` keeps everything
    /// (no entry can match → none are blocked). Empty
    /// allowlist with `invert=false` is the
    /// no-filter-configured fast path that pre-CLOC11.70
    /// callers used; both branches preserve byte-for-byte
    /// default behavior.
    ///
    /// Only consulted when `correlation_vector` is true AND
    /// `correlation_vector_filter` is non-empty (an inverted
    /// empty filter is a no-op, so we keep the empty-filter
    /// short-circuit).
    pub correlation_vector_filter_invert: bool,
    /// CLOC11.73: when true, after the CV sidecar is
    /// written (or skipped under
    /// `correlation_vector_format = None`), print a one-line
    /// summary of the trace to `stdout_text`. Lets a build
    /// pipeline see how many entries / contributions /
    /// tombstones the run produced without parsing the JSON
    /// itself. Counts reflect the post-filter state, so
    /// the summary describes what was actually written to
    /// disk.
    ///
    /// Only consulted when `correlation_vector` is true.
    pub correlation_vector_summary: bool,
    /// CLOC11.74: rendering style for the
    /// `--correlation_vector_summary` line.
    ///   - `Text` (default): human-readable one-line form
    ///     introduced by CLOC11.73.
    ///   - `Json`: single-line JSON object — `{"cv_sidecar":
    ///     {"path": "...", "skipped": false, "entries": N,
    ///     "contributions": M, "tombstones": T,
    ///     "pass_order": [...]}}`. Machine consumers can
    ///     parse without regex-matching the human line.
    ///   - `Kv`: space-separated key=value pairs prefixed
    ///     with `cv_sidecar.` — `cv_sidecar.path=... cv_sidecar.entries=N
    ///     cv_sidecar.contributions=M ...`. For shell-tooling
    ///     pipelines that grep/cut single fields.
    ///
    /// Only consulted when `correlation_vector_summary` is
    /// also true. With summary off the format flag is dead.
    pub correlation_vector_summary_format: CorrelationVectorSummaryFormat,
    /// CLOC11.75: route the summary line to `stderr_text`
    /// instead of `stdout_text`. Useful when stdout carries
    /// the actual JS payload (no `--js_output_file`) and you
    /// don't want a `cv sidecar: ...` line corrupting it.
    ///
    /// Default `false` preserves CLOC11.73's stdout-bound
    /// behaviour byte-for-byte. Only consulted when
    /// `correlation_vector_summary` is also true.
    pub correlation_vector_summary_stderr: bool,
    /// CLOC11.76: skip all output writes (JS, source map,
    /// manifest) and only emit the CV summary. Useful for
    /// `closurec --print-cv-summary` style invocations that
    /// just want the trace counts without producing build
    /// artifacts.
    ///
    /// Pairs naturally with
    /// `--correlation_vector_format NONE` to skip the
    /// sidecar too — pure analysis mode, no disk writes at
    /// all. The CV log is still computed in memory (so the
    /// summary counts are real).
    ///
    /// Default `false` preserves normal compile output.
    /// Independent of `correlation_vector`: if you set this
    /// without `--correlation_vector`, you get no writes
    /// and no summary — effectively a no-op pipeline run.
    pub correlation_vector_summary_only: bool,
}

/// CLOC11.74 — render style for the `--correlation_vector_summary`
/// line. Default `Text` (the CLOC11.73 form).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CorrelationVectorSummaryFormat {
    /// Human-readable one-line summary (CLOC11.73 default).
    #[default]
    Text,
    /// Single-line JSON object: `{"cv_sidecar": {...}}`.
    Json,
    /// Space-separated `key=value` pairs prefixed with
    /// `cv_sidecar.`.
    Kv,
}

/// CLOC11.69 — sidecar persistence format. Default is
/// `Json` (the historical CLOC11.60+ behaviour).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CorrelationVectorFormat {
    /// Single JSON document. The `correlation_vector_pretty`
    /// flag toggles compact vs indented.
    #[default]
    Json,
    /// Newline-delimited JSON: one entry per line. `pretty`
    /// is ignored.
    Ndjson,
    /// No sidecar written; CV log is still computed in
    /// memory and discarded. For benchmarks.
    None,
}

// ---------------------------------------------------------------------------
// Special passes
// ---------------------------------------------------------------------------

/// Framework-specific or vendor-specific passes. All deferred to
/// CLOC11 Track 19. Storing the flag values here means later PRs
/// can flip them on without re-touching the wiring layer.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpecialPassesConfig {
    pub angular_pass: bool,
    pub polymer_version: Option<i64>,
    pub chrome_pass: bool,
    pub j2cl_pass: J2clPassMode,
    pub remove_j2cl_asserts: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum J2clPassMode {
    Off,
    On,
    #[default]
    Auto,
}

// ---------------------------------------------------------------------------
// Translations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TranslationsConfig {
    pub translations_file: Option<PathBuf>,
    pub translations_project: Option<String>,
}

// ---------------------------------------------------------------------------
// JSON streams
// ---------------------------------------------------------------------------

/// `--json_streams`. Selects which streams (stdin / stdout) use the
/// JSON-array form `[{ "src": "...", "path": "..." }, ...]`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum JsonStreamsMode {
    #[default]
    None,
    In,
    Out,
    Both,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_sensible_zero_state() {
        // Default::default() must give us a config that says
        // "no inputs, write to stdout, identity pipeline."
        let cfg = CompilerConfig::default();
        assert!(cfg.io.js_patterns.is_empty());
        assert!(cfg.io.js_output_file.is_none());
        assert_eq!(cfg.io.env, EnvKind::Browser);
        assert_eq!(cfg.compilation.level, CompilationLevel::Simple);
        assert_eq!(cfg.language.language_in, LanguageVersion::Stable);
        assert_eq!(cfg.diagnostics.warning_level, WarningLevel::Default);
        assert_eq!(cfg.json_streams, JsonStreamsMode::None);
    }

    #[test]
    fn config_round_trips_through_clone() {
        // Clone + PartialEq cover the whole architecture; if any
        // sub-struct forgot to derive these, this test catches it.
        let mut cfg = CompilerConfig::default();
        cfg.io.js_patterns.push("src/**/*.js".to_string());
        cfg.io.js_output_file = Some(PathBuf::from("out.js"));
        cfg.compilation.level = CompilationLevel::Advanced;
        cfg.defines.defines.insert(
            "DEBUG".to_string(),
            DefineValue::Bool(false),
        );
        let cloned = cfg.clone();
        assert_eq!(cfg, cloned);
    }

    // `3.14` below is deliberate test data (a representative fractional define
    // value), not an approximation of std::f64::consts::PI.
    #[allow(clippy::approx_constant)]
    #[test]
    fn define_value_variants_round_trip() {
        // Each variant covers a JS literal form Closure accepts on
        // --define. Cover them all so adding a new one (e.g. BigInt)
        // is a visible diff to this test.
        let values = vec![
            DefineValue::Bool(true),
            DefineValue::Bool(false),
            DefineValue::Number(3.14),
            DefineValue::Number(-42.0),
            DefineValue::String("hello".to_string()),
            DefineValue::Null,
        ];
        for v in values {
            assert_eq!(v.clone(), v);
        }
    }
}
