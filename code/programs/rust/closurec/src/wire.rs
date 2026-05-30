//! `wire` — translate `cli-builder`'s [`ParseResult`] into a typed
//! [`CompilerConfig`].
//!
//! # Why this is one module, not inline
//!
//! Every CLOC11 implementation PR after CLOC11.01 will reach in
//! here and add lines that say "and here's how `--polymer_version`
//! becomes `cfg.special_passes.polymer_version`." Keeping that
//! mapping in one place — adjacent to its only purpose — means:
//!
//! - Reviewers can audit "did this PR map the new flag?" in one
//!   file.
//! - Tests of the mapping are colocated with the mapping.
//! - `main.rs` stays terse and focused on top-level orchestration.
//!
//! # Schema cohesion vs flexibility
//!
//! `cli-builder` exposes flags as `HashMap<String, serde_json::Value>`,
//! and the JSON values are typed per the `cli.spec.json` flag
//! declarations (string, integer, boolean, enum). Our wire helpers
//! below assume a flag is *the type the spec said it is*; mismatch
//! is a bug in `cli.spec.json` and is reported as `ConfigError::SpecMismatch`
//! so a corrupted spec doesn't silently produce wrong configs.
//!
//! # CLOC11.01 scope
//!
//! Only **I/O-relevant** flags get fully mapped this PR (so
//! identity-build works end-to-end). Every other flag is wired
//! into its config slot via the same `read_*` helpers so subsequent
//! PRs add behavior, not architecture.
//!
//! [`ParseResult`]: cli_builder::types::ParseResult
//! [`CompilerConfig`]: crate::config::CompilerConfig

use crate::config::*;
use cli_builder::types::ParseResult;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Things that can go wrong while turning a `ParseResult` into a
/// `CompilerConfig`.
///
/// Most of these "shouldn't happen" if `cli.spec.json` matches the
/// wiring code — but humans edit both, and silent mismatches between
/// "I declared this as an integer" and "I read it as a string" are
/// exactly the bug class we want to surface loudly.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigError {
    /// `cli.spec.json` declared a flag as one type and we tried to
    /// read it as a different one. The string is human-readable.
    SpecMismatch(String),
    /// `--define NAME=value` had a value that didn't parse as a JS
    /// literal. Closure accepts numbers, strings (quoted), bools,
    /// and `null`; everything else is an error.
    InvalidDefine { name: String, raw: String },
    /// A flag combination that Closure rejects.
    /// E.g. `--renaming false` together with `--compilation_level ADVANCED`.
    Conflict(String),
    /// `--source_map_input` was passed without the required
    /// `input|map` separator (CLOC11.40). CC errors on this;
    /// without the pipe we can't tell which side is the JS input
    /// and which is the map. Silently dropping the entry (the
    /// prior behavior) caused users to wonder why their source
    /// map chain didn't apply.
    InvalidSourceMapInput { raw: String },
    /// `--source_map_location_mapping` was passed without the
    /// required `filesystem|web` separator (CLOC11.41). Sibling
    /// of [`Self::InvalidSourceMapInput`] — same silent-drop bug
    /// in the sibling parser. Pre-CLOC11.41 typo'd `--source_map_
    /// location_mapping src/` would vanish, leaving the user
    /// puzzled why their map URLs didn't rewrite. Now errors.
    InvalidSourceMapLocationMapping { raw: String },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::SpecMismatch(s) => write!(f, "cli.spec.json mismatch: {s}"),
            ConfigError::InvalidDefine { name, raw } => write!(
                f,
                "--define {name}={raw}: value is not a valid JS literal (expected number, string, bool, or null)"
            ),
            ConfigError::Conflict(s) => write!(f, "incompatible flags: {s}"),
            ConfigError::InvalidSourceMapInput { raw } => write!(
                f,
                "--source_map_input {raw}: missing required `|` separator (expected `input-file-path|input-source-map`)"
            ),
            ConfigError::InvalidSourceMapLocationMapping { raw } => write!(
                f,
                "--source_map_location_mapping {raw}: missing required `|` separator (expected `filesystem-path|web-server-path`)"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Build a [`CompilerConfig`] from a parsed argv.
///
/// Every Closure Compiler flag is read here. Per [CLOC11.01], only
/// the I/O flags' *values* get used downstream in v1 — but every
/// flag is plumbed so adding behavior in a later PR is a one-line
/// `run_compiler` change.
///
/// [CLOC11.01]: ../../../../specs/CLOC11-drop-in-closure-compat.md#track-1--end-to-end-identity-build-foundation
pub fn config_from_parsed(parsed: &ParseResult) -> Result<CompilerConfig, ConfigError> {
    let cfg = CompilerConfig {
        io: read_io(parsed)?,
        compilation: read_compilation(parsed)?,
        language: read_language(parsed)?,
        formatting: read_formatting(parsed)?,
        source_map: read_source_map(parsed)?,
        diagnostics: read_diagnostics(parsed)?,
        defines: read_defines(parsed)?,
        dependencies: read_dependencies(parsed)?,
        chunks: read_chunks(parsed)?,
        polyfills: read_polyfills(parsed)?,
        renaming_reports: read_renaming_reports(parsed)?,
        exports: read_exports(parsed)?,
        conformance: read_conformance(parsed)?,
        instrumentation: read_instrumentation(parsed)?,
        special_modes: read_special_modes(parsed)?,
        special_passes: read_special_passes(parsed)?,
        translations: read_translations(parsed)?,
        json_streams: read_json_streams(parsed)?,
    };
    validate_combinations(&cfg)?;
    Ok(cfg)
}

// ---------------------------------------------------------------------------
// Per-feature readers
// ---------------------------------------------------------------------------

fn read_io(p: &ParseResult) -> Result<IoConfig, ConfigError> {
    Ok(IoConfig {
        js_patterns: get_str_list(p, "js")?,
        jszip_paths: get_str_list(p, "jszip")?.into_iter().map(PathBuf::from).collect(),
        js_output_file: get_str(p, "js_output_file")?
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
        // CLOC11.05: keep externs as raw pattern strings; they go
        // through `globs::expand_js_patterns` in `run_compiler`,
        // same as `--js`. Errors on glob/missing-file surface as
        // typed `CompilerError::ExternsGlobExpansion`.
        externs: get_str_list(p, "externs")?,
        env: match get_str(p, "env")?.as_deref() {
            Some("CUSTOM") => EnvKind::Custom,
            // BROWSER is the default; absent flag → default.
            _ => EnvKind::Browser,
        },
        charset: get_str(p, "charset")?.unwrap_or_default(),
    })
}

fn read_compilation(p: &ParseResult) -> Result<CompilationConfig, ConfigError> {
    Ok(CompilationConfig {
        level: match get_str(p, "compilation_level")?.as_deref() {
            Some("BUNDLE") => CompilationLevel::Bundle,
            Some("WHITESPACE_ONLY") => CompilationLevel::WhitespaceOnly,
            Some("TRANSPILE_ONLY") => CompilationLevel::TranspileOnly,
            Some("ADVANCED") => CompilationLevel::Advanced,
            // Default per spec: SIMPLE.
            _ => CompilationLevel::Simple,
        },
        debug: get_bool(p, "debug")?,
        // `--renaming` is a no-value boolean flag in cli-builder's
        // model (`--renaming` means "true"; there's no
        // `--renaming false` form). To disable renaming users use
        // `--compilation_level WHITESPACE_ONLY` instead. CLOC11.12
        // will revisit this once cli-builder grows
        // `--no-foo`/`--foo=value` support for default-true bools.
        renaming: true,
        assume_function_wrapper: get_bool(p, "assume_function_wrapper")?,
        use_types_for_optimization: get_bool_default(p, "use_types_for_optimization", true)?,
        continue_after_errors: get_bool(p, "continue_after_errors")?,
        checks_only: get_bool(p, "checks_only")?,
    })
}

fn read_language(p: &ParseResult) -> Result<LanguageConfig, ConfigError> {
    Ok(LanguageConfig {
        language_in: parse_language(get_str(p, "language_in")?.as_deref(), true),
        language_out: parse_language(get_str(p, "language_out")?.as_deref(), false),
        strict_mode_input: get_bool_default(p, "strict_mode_input", true)?,
        emit_use_strict: get_bool(p, "emit_use_strict")?,
        browser_featureset_year: get_int(p, "browser_featureset_year")?.unwrap_or(0),
    })
}

fn parse_language(s: Option<&str>, is_input: bool) -> LanguageVersion {
    // `s.is_none()` means flag absent → default. Defaults differ
    // between --language_in (STABLE) and --language_out (ECMASCRIPT_NEXT)
    // per the cli.spec.json declarations.
    match s {
        Some("ECMASCRIPT3") => LanguageVersion::Ecmascript3,
        Some("ECMASCRIPT5") => LanguageVersion::Ecmascript5,
        Some("ECMASCRIPT5_STRICT") => LanguageVersion::Ecmascript5Strict,
        Some("ECMASCRIPT_2015") => LanguageVersion::Ecmascript2015,
        Some("ECMASCRIPT_2016") => LanguageVersion::Ecmascript2016,
        Some("ECMASCRIPT_2017") => LanguageVersion::Ecmascript2017,
        Some("ECMASCRIPT_2018") => LanguageVersion::Ecmascript2018,
        Some("ECMASCRIPT_2019") => LanguageVersion::Ecmascript2019,
        Some("ECMASCRIPT_2020") => LanguageVersion::Ecmascript2020,
        Some("ECMASCRIPT_2021") => LanguageVersion::Ecmascript2021,
        Some("ECMASCRIPT_NEXT") => LanguageVersion::EcmascriptNext,
        Some("UNSTABLE") => LanguageVersion::Unstable,
        Some("NO_TRANSPILE") => LanguageVersion::NoTranspile,
        Some("STABLE") => LanguageVersion::Stable,
        None | Some(_) if is_input => LanguageVersion::Stable,
        None | Some(_) => LanguageVersion::EcmascriptNext,
    }
}

fn read_formatting(p: &ParseResult) -> Result<FormattingConfig, ConfigError> {
    let formatting = get_str_list(p, "formatting")?
        .into_iter()
        .filter_map(|s| match s.as_str() {
            "PRETTY_PRINT" => Some(FormattingMode::PrettyPrint),
            "PRINT_INPUT_DELIMITER" => Some(FormattingMode::PrintInputDelimiter),
            "SINGLE_QUOTES" => Some(FormattingMode::SingleQuotes),
            _ => None,
        })
        .collect();

    Ok(FormattingConfig {
        output_wrapper: get_str(p, "output_wrapper")?.unwrap_or_default(),
        output_wrapper_file: get_str(p, "output_wrapper_file")?
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
        isolation_mode: match get_str(p, "isolation_mode")?.as_deref() {
            Some("IIFE") => IsolationMode::Iife,
            _ => IsolationMode::None,
        },
        formatting,
        rename_variable_prefix: get_str(p, "rename_variable_prefix")?
            .filter(|s| !s.is_empty()),
        rename_prefix_namespace: get_str(p, "rename_prefix_namespace")?
            .filter(|s| !s.is_empty()),
    })
}

fn read_source_map(p: &ParseResult) -> Result<SourceMapConfig, ConfigError> {
    // CLOC11.41: error on malformed --source_map_location_mapping
    // entries rather than silently dropping them. Sibling fix to
    // CLOC11.40's --source_map_input handling — same silent-drop
    // bug, same shape, same fix. Empty halves (e.g. `|/static/`)
    // remain well-formed; only the *presence* of the pipe is
    // required.
    let mut location_mappings: Vec<LocationMapping> = Vec::new();
    for raw in get_str_list(p, "source_map_location_mapping")? {
        let Some((fs, web)) = raw.split_once('|') else {
            return Err(ConfigError::InvalidSourceMapLocationMapping { raw });
        };
        location_mappings.push(LocationMapping {
            filesystem_path: fs.to_string(),
            web_server_path: web.to_string(),
        });
    }

    // CLOC11.40: error on malformed --source_map_input entries
    // rather than silently dropping them via filter_map. The
    // prior behavior — typo'd separator → entry quietly vanishes
    // → user wonders why their source map chain didn't apply —
    // was a CC-incompat surprise. CC errors, we now match.
    let mut inputs: Vec<InputSourceMap> = Vec::new();
    for raw in get_str_list(p, "source_map_input")? {
        let Some((input, map)) = raw.split_once('|') else {
            return Err(ConfigError::InvalidSourceMapInput { raw });
        };
        inputs.push(InputSourceMap {
            input_file: PathBuf::from(input),
            map_file: PathBuf::from(map),
        });
    }

    Ok(SourceMapConfig {
        path_template: get_str(p, "create_source_map")?.unwrap_or_default(),
        format: match get_str(p, "source_map_format")?.as_deref() {
            Some("V3") => SourceMapFormat::V3,
            _ => SourceMapFormat::Default,
        },
        include_content: get_bool(p, "source_map_include_content")?,
        location_mappings,
        inputs,
        parse_inline_source_maps: get_bool_default(p, "parse_inline_source_maps", true)?,
        apply_input_source_maps: get_bool_default(p, "apply_input_source_maps", true)?,
    })
}

fn read_diagnostics(p: &ParseResult) -> Result<DiagnosticsConfig, ConfigError> {
    Ok(DiagnosticsConfig {
        warning_level: match get_str(p, "warning_level")?.as_deref() {
            Some("QUIET") => WarningLevel::Quiet,
            Some("VERBOSE") => WarningLevel::Verbose,
            _ => WarningLevel::Default,
        },
        jscomp_error: get_str_list(p, "jscomp_error")?,
        jscomp_warning: get_str_list(p, "jscomp_warning")?,
        jscomp_off: get_str_list(p, "jscomp_off")?,
        hide_warnings_for: get_str_list(p, "hide_warnings_for")?,
        warnings_allowlist_file: get_str(p, "warnings_allowlist_file")?
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
        error_format: ErrorFormat::Standard,
        summary_detail_level: get_int(p, "summary_detail_level")?.unwrap_or(1),
        third_party: get_bool(p, "third_party")?,
        extra_annotation_names: get_str_list(p, "extra_annotation_name")?,
    })
}

fn read_defines(p: &ParseResult) -> Result<DefinesConfig, ConfigError> {
    let mut defines = BTreeMap::new();
    for raw in get_str_list(p, "define")? {
        let (name, value) = match raw.split_once('=') {
            // `--define NAME=val` → parse val as a JS literal.
            Some((n, v)) => (n.to_string(), parse_define_value(n, v)?),
            // `--define NAME` (no =) → implicit true.
            None => (raw.clone(), DefineValue::Bool(true)),
        };
        defines.insert(name, value);
    }
    Ok(DefinesConfig { defines })
}

fn parse_define_value(name: &str, raw: &str) -> Result<DefineValue, ConfigError> {
    // Closure accepts: bare true/false, numbers, "quoted strings",
    // null. Bare bare strings (no quotes) are NOT accepted by the
    // upstream tool.
    let trimmed = raw.trim();
    if trimmed == "true" {
        return Ok(DefineValue::Bool(true));
    }
    if trimmed == "false" {
        return Ok(DefineValue::Bool(false));
    }
    if trimmed == "null" {
        return Ok(DefineValue::Null);
    }
    if let Ok(n) = trimmed.parse::<f64>() {
        // Reject `NaN` and `Infinity` strings the same way Closure
        // does — they parse as f64 in Rust but aren't valid in JS
        // literal position.
        if n.is_finite() {
            return Ok(DefineValue::Number(n));
        }
    }
    if (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2)
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2)
    {
        return Ok(DefineValue::String(
            trimmed[1..trimmed.len() - 1].to_string(),
        ));
    }
    Err(ConfigError::InvalidDefine {
        name: name.to_string(),
        raw: raw.to_string(),
    })
}

fn read_dependencies(p: &ParseResult) -> Result<DependenciesConfig, ConfigError> {
    Ok(DependenciesConfig {
        mode: match get_str(p, "dependency_mode")?.as_deref() {
            Some("SORT_ONLY") => DependencyMode::SortOnly,
            Some("PRUNE") => DependencyMode::Prune,
            Some("PRUNE_LEGACY") => DependencyMode::PruneLegacy,
            _ => DependencyMode::None,
        },
        entry_points: get_str_list(p, "entry_point")?,
        process_closure_primitives: get_bool_default(p, "process_closure_primitives", true)?,
        process_common_js_modules: get_bool(p, "process_common_js_modules")?,
        js_module_roots: get_str_list(p, "js_module_root")?
            .into_iter()
            .map(PathBuf::from)
            .collect(),
        module_resolution: match get_str(p, "module_resolution")?.as_deref() {
            Some("NODE") => ModuleResolution::Node,
            Some("WEBPACK") => ModuleResolution::Webpack,
            _ => ModuleResolution::Browser,
        },
        browser_resolver_prefix_replacements: get_str_list(
            p,
            "browser_resolver_prefix_replacements",
        )?,
        package_json_entry_names: get_str(p, "package_json_entry_names")?
            .filter(|s| !s.is_empty()),
    })
}

fn read_chunks(p: &ParseResult) -> Result<ChunksConfig, ConfigError> {
    Ok(ChunksConfig {
        chunk_specs: get_str_list(p, "chunk")?,
        chunk_wrappers: get_str_list(p, "chunk_wrapper")?,
        output_path_prefix: PathBuf::from(
            get_str(p, "chunk_output_path_prefix")?.unwrap_or_else(|| "./".to_string()),
        ),
        output_type: match get_str(p, "chunk_output_type")?.as_deref() {
            Some("ES_MODULES") => ChunkOutputType::EsModules,
            _ => ChunkOutputType::GlobalNamespace,
        },
        output_chunk_dependencies_file: get_str(p, "output_chunk_dependencies")?
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
        output_manifest_file: get_str(p, "output_manifest")?
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
    })
}

fn read_polyfills(p: &ParseResult) -> Result<PolyfillsConfig, ConfigError> {
    Ok(PolyfillsConfig {
        rewrite_polyfills: get_bool_default(p, "rewrite_polyfills", true)?,
        isolate_polyfills: get_bool(p, "isolate_polyfills")?,
        inject_libraries: get_bool_default(p, "inject_libraries", true)?,
        force_inject_libraries: get_str_list(p, "force_inject_library")?,
    })
}

fn read_renaming_reports(p: &ParseResult) -> Result<RenamingReportsConfig, ConfigError> {
    Ok(RenamingReportsConfig {
        variable_renaming_report: get_str(p, "variable_renaming_report")?
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
        property_renaming_report: get_str(p, "property_renaming_report")?
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
        create_renaming_reports: get_bool(p, "create_renaming_reports")?,
    })
}

fn read_exports(p: &ParseResult) -> Result<ExportsConfig, ConfigError> {
    Ok(ExportsConfig {
        generate_exports: get_bool_default(p, "generate_exports", true)?,
        export_local_property_definitions: get_bool_default(
            p,
            "export_local_property_definitions",
            true,
        )?,
    })
}

fn read_conformance(p: &ParseResult) -> Result<ConformanceConfig, ConfigError> {
    Ok(ConformanceConfig {
        configs: get_str_list(p, "conformance_configs")?
            .into_iter()
            .map(PathBuf::from)
            .collect(),
    })
}

fn read_instrumentation(p: &ParseResult) -> Result<InstrumentationConfig, ConfigError> {
    Ok(InstrumentationConfig {
        coverage: match get_str(p, "instrument_for_coverage_option")?.as_deref() {
            Some("LINE") => CoverageOption::Line,
            Some("BRANCH") => CoverageOption::Branch,
            Some("PRODUCTION") => CoverageOption::Production,
            _ => CoverageOption::None,
        },
        production_instrumentation_array_name: get_str(p, "production_instrumentation_array_name")?
            .unwrap_or_default(),
        instrument_mapping_report: get_str(p, "instrument_mapping_report")?
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
        tracer_mode: match get_str(p, "tracer_mode")?.as_deref() {
            Some("ALL") => TracerMode::All,
            Some("AST_SIZE") => TracerMode::AstSize,
            Some("RAW_SIZE") => TracerMode::RawSize,
            Some("TIMING_ONLY") => TracerMode::TimingOnly,
            _ => TracerMode::Off,
        },
    })
}

fn read_special_modes(p: &ParseResult) -> Result<SpecialModesConfig, ConfigError> {
    Ok(SpecialModesConfig {
        print_tree: get_bool(p, "print_tree")?,
        print_tree_json: get_bool(p, "print_tree_json")?,
        print_ast: get_bool(p, "print_ast")?,
        print_source_after_each_pass: get_bool(p, "print_source_after_each_pass")?,
        help_markdown: get_bool(p, "help_markdown")?,
        correlation_vector: get_bool(p, "correlation_vector")?,
        correlation_vector_output: get_str(p, "correlation_vector_output")?
            .filter(|s| !s.is_empty())
            .map(std::path::PathBuf::from),
        correlation_vector_pretty: get_bool(p, "correlation_vector_pretty")?,
        correlation_vector_format: match get_str(p, "correlation_vector_format")?
            .as_deref()
        {
            Some("NDJSON") => crate::config::CorrelationVectorFormat::Ndjson,
            Some("NONE") => crate::config::CorrelationVectorFormat::None,
            // JSON, empty, or absent → default
            _ => crate::config::CorrelationVectorFormat::Json,
        },
        correlation_vector_filter: get_str(p, "correlation_vector_filter")?
            .map(|s| {
                s.split(',')
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        correlation_vector_filter_includes_origin: get_bool(
            p,
            "correlation_vector_filter_includes_origin",
        )?,
        correlation_vector_filter_invert: get_bool(
            p,
            "correlation_vector_filter_invert",
        )?,
        correlation_vector_summary: get_bool(p, "correlation_vector_summary")?,
        correlation_vector_summary_format: match get_str(
            p,
            "correlation_vector_summary_format",
        )?
        .as_deref()
        {
            Some("JSON") => crate::config::CorrelationVectorSummaryFormat::Json,
            Some("KV") => crate::config::CorrelationVectorSummaryFormat::Kv,
            // TEXT, empty, or absent → default
            _ => crate::config::CorrelationVectorSummaryFormat::Text,
        },
        correlation_vector_summary_stderr: get_bool(
            p,
            "correlation_vector_summary_stderr",
        )?,
        correlation_vector_summary_only: get_bool(
            p,
            "correlation_vector_summary_only",
        )?,
    })
}

fn read_special_passes(p: &ParseResult) -> Result<SpecialPassesConfig, ConfigError> {
    Ok(SpecialPassesConfig {
        angular_pass: get_bool(p, "angular_pass")?,
        polymer_version: get_int(p, "polymer_version")?,
        chrome_pass: get_bool(p, "chrome_pass")?,
        j2cl_pass: match get_str(p, "j2cl_pass")?.as_deref() {
            Some("OFF") => J2clPassMode::Off,
            Some("ON") => J2clPassMode::On,
            _ => J2clPassMode::Auto,
        },
        remove_j2cl_asserts: get_bool_default(p, "remove_j2cl_asserts", true)?,
    })
}

fn read_translations(p: &ParseResult) -> Result<TranslationsConfig, ConfigError> {
    Ok(TranslationsConfig {
        translations_file: get_str(p, "translations_file")?
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
        translations_project: get_str(p, "translations_project")?
            .filter(|s| !s.is_empty()),
    })
}

fn read_json_streams(p: &ParseResult) -> Result<JsonStreamsMode, ConfigError> {
    Ok(match get_str(p, "json_streams")?.as_deref() {
        Some("IN") => JsonStreamsMode::In,
        Some("OUT") => JsonStreamsMode::Out,
        Some("BOTH") => JsonStreamsMode::Both,
        _ => JsonStreamsMode::None,
    })
}

// ---------------------------------------------------------------------------
// Cross-flag validation
// ---------------------------------------------------------------------------

fn validate_combinations(_cfg: &CompilerConfig) -> Result<(), ConfigError> {
    // Cross-flag validation. Closure rejects several flag
    // combinations (e.g. `--renaming false` with `--compilation_level
    // ADVANCED`). CLOC11.01 doesn't implement any of these checks
    // yet because cli-builder's current boolean-flag model doesn't
    // expose `--no-X` / `--foo=value` for default-true bools, which
    // is a prerequisite for distinguishing "user opted out" from
    // "absent → spec default." Later tracks revisit this.
    Ok(())
}

// ---------------------------------------------------------------------------
// JSON-value helpers
// ---------------------------------------------------------------------------
//
// cli-builder hands us flag values as `serde_json::Value`. These
// helpers do the unwrap-with-typed-error pattern uniformly. They
// also handle the "absent" case for every cardinality (single
// string, repeatable list, boolean, integer).

fn get(p: &ParseResult, key: &str) -> Option<JsonValue> {
    p.flags.get(key).cloned()
}

fn get_str(p: &ParseResult, key: &str) -> Result<Option<String>, ConfigError> {
    match get(p, key) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::String(s)) => Ok(Some(s)),
        Some(other) => Err(ConfigError::SpecMismatch(format!(
            "flag {key:?} declared as string but got {other:?}"
        ))),
    }
}

fn get_str_list(p: &ParseResult, key: &str) -> Result<Vec<String>, ConfigError> {
    match get(p, key) {
        None | Some(JsonValue::Null) => Ok(Vec::new()),
        Some(JsonValue::Array(arr)) => arr
            .into_iter()
            .map(|v| match v {
                JsonValue::String(s) => Ok(s),
                other => Err(ConfigError::SpecMismatch(format!(
                    "flag {key:?} array element not a string: {other:?}"
                ))),
            })
            .collect(),
        Some(JsonValue::String(s)) => Ok(vec![s]),
        Some(other) => Err(ConfigError::SpecMismatch(format!(
            "flag {key:?} declared as repeatable but got {other:?}"
        ))),
    }
}

fn get_bool(p: &ParseResult, key: &str) -> Result<bool, ConfigError> {
    match get(p, key) {
        None | Some(JsonValue::Null) => Ok(false),
        Some(JsonValue::Bool(b)) => Ok(b),
        Some(other) => Err(ConfigError::SpecMismatch(format!(
            "flag {key:?} declared as boolean but got {other:?}"
        ))),
    }
}

/// Like [`get_bool`] but returns `default` when the flag is absent.
/// Some Closure flags default to true (e.g. `--renaming`,
/// `--strict_mode_input`); use this for those.
fn get_bool_default(p: &ParseResult, key: &str, default: bool) -> Result<bool, ConfigError> {
    match get(p, key) {
        None | Some(JsonValue::Null) => Ok(default),
        Some(JsonValue::Bool(b)) => Ok(b),
        Some(other) => Err(ConfigError::SpecMismatch(format!(
            "flag {key:?} declared as boolean but got {other:?}"
        ))),
    }
}

fn get_int(p: &ParseResult, key: &str) -> Result<Option<i64>, ConfigError> {
    match get(p, key) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Number(n)) => Ok(n.as_i64()),
        Some(other) => Err(ConfigError::SpecMismatch(format!(
            "flag {key:?} declared as integer but got {other:?}"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use cli_builder::{load_spec_from_str, Parser};
    use cli_builder::types::ParserOutput;

    const CLI_SPEC_JSON: &str = include_str!("../cli.spec.json");

    /// Parse argv through the real cli.spec.json — keeps these
    /// tests honest about what the actual flag surface accepts.
    fn parse(argv: &[&str]) -> ParseResult {
        let spec = load_spec_from_str(CLI_SPEC_JSON).expect("spec loads");
        let mut full = vec!["closurec".to_string()];
        full.extend(argv.iter().map(|s| s.to_string()));
        match Parser::new(spec).parse(&full).expect("parses") {
            ParserOutput::Parse(r) => r,
            other => panic!("expected Parse, got {other:?}"),
        }
    }

    #[test]
    fn empty_argv_yields_default_config() {
        let cfg = config_from_parsed(&parse(&[])).expect("config builds");
        // Empty inputs, no output (stdout), Simple level.
        assert!(cfg.io.js_patterns.is_empty());
        assert!(cfg.io.js_output_file.is_none());
        assert_eq!(cfg.compilation.level, CompilationLevel::Simple);
    }

    #[test]
    fn canonical_invocation_maps_io_and_level() {
        let cfg = config_from_parsed(&parse(&[
            "--js",
            "a.js",
            "--js",
            "b.js",
            "--js_output_file",
            "out.js",
            "--compilation_level",
            "ADVANCED",
        ]))
        .expect("config builds");
        assert_eq!(cfg.io.js_patterns, vec!["a.js".to_string(), "b.js".to_string()]);
        assert_eq!(cfg.io.js_output_file, Some(PathBuf::from("out.js")));
        assert_eq!(cfg.compilation.level, CompilationLevel::Advanced);
    }

    #[test]
    fn js_output_file_absent_maps_to_none() {
        // cli-builder's string parser rejects an explicit empty
        // string, so we rely on flag *absence* meaning stdout.
        // Real CC accepts `""` as "write to stdout" — we'll
        // bring that ergonomic alignment in CLOC11.03 by making
        // cli-builder's string validator more permissive for
        // js_output_file. For now: absent flag = None.
        let cfg = config_from_parsed(&parse(&[])).expect("ok");
        assert!(cfg.io.js_output_file.is_none());
    }

    #[test]
    fn formatting_repeatable_collects() {
        let cfg = config_from_parsed(&parse(&[
            "--formatting",
            "PRETTY_PRINT",
            "--formatting",
            "SINGLE_QUOTES",
        ]))
        .expect("ok");
        assert_eq!(
            cfg.formatting.formatting,
            vec![FormattingMode::PrettyPrint, FormattingMode::SingleQuotes],
        );
    }

    #[test]
    fn define_bool_implicit_true() {
        let cfg = config_from_parsed(&parse(&["--define", "DEBUG"])).expect("ok");
        assert_eq!(cfg.defines.defines.get("DEBUG"), Some(&DefineValue::Bool(true)));
    }

    #[test]
    fn define_explicit_bool_number_string_null() {
        let cfg = config_from_parsed(&parse(&[
            "-D", "DEBUG=false",
            "-D", "VERSION=42",
            "-D", "NAME=\"hello\"",
            "-D", "EMPTY=null",
        ]))
        .expect("ok");
        assert_eq!(cfg.defines.defines.get("DEBUG"), Some(&DefineValue::Bool(false)));
        assert_eq!(cfg.defines.defines.get("VERSION"), Some(&DefineValue::Number(42.0)));
        assert_eq!(
            cfg.defines.defines.get("NAME"),
            Some(&DefineValue::String("hello".to_string())),
        );
        assert_eq!(cfg.defines.defines.get("EMPTY"), Some(&DefineValue::Null));
    }

    // ------------------------------------------------------------------
    // CLOC11.04 — --define numeric edge case coverage
    //
    // CC accepts the same number forms Java's Double.parseDouble
    // accepts: negative, fractional, scientific, leading-plus.
    // Rust's f64::parse covers all of these so the existing
    // parse_define_value branch already works — these tests just
    // pin the contract so a future refactor (e.g. switching to a
    // hand-rolled number parser) can't quietly regress it.
    // ------------------------------------------------------------------

    #[test]
    fn define_accepts_negative_integer() {
        let cfg = config_from_parsed(&parse(&["-D", "N=-42"])).expect("ok");
        assert_eq!(cfg.defines.defines.get("N"), Some(&DefineValue::Number(-42.0)));
    }

    #[test]
    fn define_accepts_negative_float() {
        let cfg = config_from_parsed(&parse(&["-D", "X=-1.5"])).expect("ok");
        assert_eq!(cfg.defines.defines.get("X"), Some(&DefineValue::Number(-1.5)));
    }

    #[test]
    fn define_accepts_fractional_only() {
        // `.5` (no leading digit) is a valid JS number literal
        // and a valid Rust f64::parse input.
        let cfg = config_from_parsed(&parse(&["-D", "HALF=.5"])).expect("ok");
        assert_eq!(cfg.defines.defines.get("HALF"), Some(&DefineValue::Number(0.5)));
    }

    #[test]
    fn define_accepts_scientific_notation() {
        let cfg = config_from_parsed(&parse(&["-D", "KILO=1e3"])).expect("ok");
        assert_eq!(cfg.defines.defines.get("KILO"), Some(&DefineValue::Number(1000.0)));
    }

    #[test]
    fn define_accepts_scientific_notation_with_negative_exponent() {
        let cfg = config_from_parsed(&parse(&["-D", "MICRO=1e-6"])).expect("ok");
        assert_eq!(
            cfg.defines.defines.get("MICRO"),
            Some(&DefineValue::Number(1e-6))
        );
    }

    #[test]
    fn define_accepts_leading_plus_sign() {
        // `+1` is uncommon but accepted by both CC's parser and
        // Rust's f64::parse. Some shell scripts emit it from
        // `echo "+$N"` patterns.
        let cfg = config_from_parsed(&parse(&["-D", "P=+1"])).expect("ok");
        assert_eq!(cfg.defines.defines.get("P"), Some(&DefineValue::Number(1.0)));
    }

    #[test]
    fn define_rejects_nan_string() {
        // `NaN` parses as f64 but is non-finite; we reject (CC
        // does too — NaN isn't a valid JS *literal*, you have to
        // write `0/0` to get it).
        let err = config_from_parsed(&parse(&["-D", "BAD=NaN"]))
            .expect_err("must reject");
        match err {
            ConfigError::InvalidDefine { name, .. } => assert_eq!(name, "BAD"),
            other => panic!("expected InvalidDefine, got {other:?}"),
        }
    }

    #[test]
    fn define_rejects_infinity_string() {
        // Same reasoning as NaN.
        let err = config_from_parsed(&parse(&["-D", "INF=Infinity"]))
            .expect_err("must reject");
        match err {
            ConfigError::InvalidDefine { name, .. } => assert_eq!(name, "INF"),
            other => panic!("expected InvalidDefine, got {other:?}"),
        }
    }

    #[test]
    fn define_accepts_hex_via_f64_parse_failure_path_fallthrough() {
        // Rust's f64::parse does NOT accept hex literals
        // (`0xFF`). Since CC's value parser uses
        // Double.parseDouble which also rejects hex, we error.
        // Pin so a refactor doesn't accidentally accept hex.
        let err = config_from_parsed(&parse(&["-D", "HEX=0xFF"]))
            .expect_err("must reject");
        match err {
            ConfigError::InvalidDefine { name, .. } => assert_eq!(name, "HEX"),
            other => panic!("expected InvalidDefine, got {other:?}"),
        }
    }

    #[test]
    fn define_zero_and_negative_zero_both_parse() {
        let cfg = config_from_parsed(&parse(&["-D", "Z=0", "-D", "NZ=-0"]))
            .expect("ok");
        assert_eq!(cfg.defines.defines.get("Z"), Some(&DefineValue::Number(0.0)));
        // -0.0 IS finite and parses; we accept it. The f64 -0.0
        // compares equal to +0.0 under ==, so the assertion still
        // passes.
        assert_eq!(cfg.defines.defines.get("NZ"), Some(&DefineValue::Number(-0.0)));
    }

    #[test]
    fn define_invalid_returns_error() {
        // Bare unquoted string is not a valid JS literal — CC
        // rejects, we reject too.
        let err = config_from_parsed(&parse(&["--define", "NAME=barewords"]))
            .expect_err("should reject");
        match err {
            ConfigError::InvalidDefine { name, .. } => assert_eq!(name, "NAME"),
            other => panic!("expected InvalidDefine, got {other:?}"),
        }
    }

    #[test]
    fn renaming_defaults_to_true_unconditionally_in_cloc11_01() {
        // CLOC11.01 hard-codes renaming = true (see comment in
        // read_compilation). The "--renaming false" / ADVANCED
        // conflict check that CC enforces will land in a later
        // CLOC11 PR once cli-builder grows --no-foo support.
        // We pin the current behavior so a regression here is
        // visible.
        let cfg_advanced = config_from_parsed(&parse(&[
            "--compilation_level",
            "ADVANCED",
        ]))
        .expect("ok");
        assert!(cfg_advanced.compilation.renaming);
        let cfg_default = config_from_parsed(&parse(&[])).expect("ok");
        assert!(cfg_default.compilation.renaming);
    }

    #[test]
    fn language_enum_round_trips() {
        // `ECMASCRIPT5_STRICT` is only valid for --language_in
        // per cli.spec.json's enum_values list; --language_out
        // omits it. So we pair `--language_in ECMASCRIPT5_STRICT`
        // with `--language_out ECMASCRIPT5`.
        let cfg = config_from_parsed(&parse(&[
            "--language_in",
            "ECMASCRIPT5_STRICT",
            "--language_out",
            "ECMASCRIPT5",
        ]))
        .expect("ok");
        assert_eq!(cfg.language.language_in, LanguageVersion::Ecmascript5Strict);
        assert_eq!(cfg.language.language_out, LanguageVersion::Ecmascript5);
    }

    #[test]
    fn language_in_modern_to_es5_output_round_trips() {
        let cfg = config_from_parsed(&parse(&[
            "--language_in",
            "ECMASCRIPT_2020",
            "--language_out",
            "ECMASCRIPT5",
        ]))
        .expect("ok");
        assert_eq!(cfg.language.language_in, LanguageVersion::Ecmascript2020);
        assert_eq!(cfg.language.language_out, LanguageVersion::Ecmascript5);
    }

    #[test]
    fn source_map_location_mapping_splits_on_pipe() {
        let cfg = config_from_parsed(&parse(&[
            "--source_map_location_mapping",
            "src/|/static/js/",
        ]))
        .expect("ok");
        assert_eq!(cfg.source_map.location_mappings.len(), 1);
        assert_eq!(cfg.source_map.location_mappings[0].filesystem_path, "src/");
        assert_eq!(
            cfg.source_map.location_mappings[0].web_server_path,
            "/static/js/",
        );
    }

    #[test]
    fn jscomp_groups_collect() {
        let cfg = config_from_parsed(&parse(&[
            "--jscomp_off", "checkTypes",
            "--jscomp_off", "uselessCode",
            "--jscomp_error", "missingProvide",
        ]))
        .expect("ok");
        assert_eq!(
            cfg.diagnostics.jscomp_off,
            vec!["checkTypes".to_string(), "uselessCode".to_string()],
        );
        assert_eq!(cfg.diagnostics.jscomp_error, vec!["missingProvide".to_string()]);
    }

    #[test]
    fn config_error_display() {
        assert!(ConfigError::SpecMismatch("x".into()).to_string().contains("spec.json"));
        assert!(ConfigError::Conflict("y".into()).to_string().contains("incompatible"));
        assert!(ConfigError::InvalidDefine {
            name: "N".into(),
            raw: "R".into(),
        }
        .to_string()
        .contains("N=R"));
        assert!(ConfigError::InvalidSourceMapInput { raw: "RAW".into() }
            .to_string()
            .contains("--source_map_input RAW"));
        assert!(ConfigError::InvalidSourceMapLocationMapping { raw: "BAD".into() }
            .to_string()
            .contains("--source_map_location_mapping BAD"));
        let _: &dyn std::error::Error = &ConfigError::SpecMismatch("x".into());
    }

    // ------------------------------------------------------------------
    // CLOC11.40 — --source_map_input validation
    // ------------------------------------------------------------------

    #[test]
    fn source_map_input_splits_on_pipe_into_two_paths() {
        let cfg = config_from_parsed(&parse(&[
            "--source_map_input",
            "src/a.js|src/a.js.map",
        ]))
        .expect("ok");
        assert_eq!(cfg.source_map.inputs.len(), 1);
        assert_eq!(
            cfg.source_map.inputs[0].input_file,
            std::path::PathBuf::from("src/a.js")
        );
        assert_eq!(
            cfg.source_map.inputs[0].map_file,
            std::path::PathBuf::from("src/a.js.map")
        );
    }

    #[test]
    fn source_map_input_missing_pipe_errors() {
        // Prior behavior: silently dropped. CLOC11.40: typed error.
        let err = config_from_parsed(&parse(&[
            "--source_map_input",
            "src/a.js-no-separator",
        ]))
        .expect_err("must error");
        match err {
            ConfigError::InvalidSourceMapInput { raw } => {
                assert_eq!(raw, "src/a.js-no-separator");
            }
            other => panic!("expected InvalidSourceMapInput, got {other:?}"),
        }
    }

    #[test]
    fn source_map_input_error_message_names_flag_and_value() {
        let err = ConfigError::InvalidSourceMapInput {
            raw: "bad-value".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("--source_map_input"));
        assert!(msg.contains("bad-value"));
        assert!(msg.contains("missing required `|`"));
    }

    #[test]
    fn source_map_input_one_good_one_bad_errors_on_the_bad_one() {
        // Process in argv order; the first bad entry surfaces.
        // Useful so users fix typos one at a time rather than
        // playing whack-a-mole after each retry.
        let err = config_from_parsed(&parse(&[
            "--source_map_input", "ok.js|ok.js.map",
            "--source_map_input", "broken-no-pipe",
        ]))
        .expect_err("must error");
        match err {
            ConfigError::InvalidSourceMapInput { raw } => {
                assert_eq!(raw, "broken-no-pipe");
            }
            other => panic!("expected InvalidSourceMapInput, got {other:?}"),
        }
    }

    #[test]
    fn source_map_input_empty_left_or_right_is_still_well_formed() {
        // `|map.map` and `input.js|` are both well-formed under
        // the pipe rule — only the presence of the pipe is
        // checked. Garbage-in-garbage-out for empty halves; the
        // file system step (CLOC11.40+) will catch missing files
        // later. CC treats them the same way.
        let cfg = config_from_parsed(&parse(&[
            "--source_map_input", "|map.map",
        ]))
        .expect("ok");
        assert_eq!(cfg.source_map.inputs.len(), 1);
        assert_eq!(
            cfg.source_map.inputs[0].input_file,
            std::path::PathBuf::from("")
        );
        assert_eq!(
            cfg.source_map.inputs[0].map_file,
            std::path::PathBuf::from("map.map")
        );
    }

    // ------------------------------------------------------------------
    // CLOC11.41 — --source_map_location_mapping validation
    // ------------------------------------------------------------------

    #[test]
    fn source_map_location_mapping_missing_pipe_errors() {
        // Prior behavior: silently dropped. CLOC11.41: typed error.
        let err = config_from_parsed(&parse(&[
            "--source_map_location_mapping",
            "src-without-separator",
        ]))
        .expect_err("must error");
        match err {
            ConfigError::InvalidSourceMapLocationMapping { raw } => {
                assert_eq!(raw, "src-without-separator");
            }
            other => panic!("expected InvalidSourceMapLocationMapping, got {other:?}"),
        }
    }

    #[test]
    fn source_map_location_mapping_error_message_names_flag_and_value() {
        let err = ConfigError::InvalidSourceMapLocationMapping {
            raw: "no-pipe-here".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("--source_map_location_mapping"));
        assert!(msg.contains("no-pipe-here"));
        assert!(msg.contains("missing required `|`"));
        assert!(msg.contains("filesystem-path|web-server-path"));
    }

    #[test]
    fn source_map_location_mapping_one_good_one_bad_errors_on_the_bad_one() {
        // Process in argv order. The first bad entry surfaces so
        // users fix typos one at a time. Matches the
        // --source_map_input policy from CLOC11.40.
        let err = config_from_parsed(&parse(&[
            "--source_map_location_mapping", "src/|/static/js/",
            "--source_map_location_mapping", "broken-no-pipe",
        ]))
        .expect_err("must error");
        match err {
            ConfigError::InvalidSourceMapLocationMapping { raw } => {
                assert_eq!(raw, "broken-no-pipe");
            }
            other => panic!("expected InvalidSourceMapLocationMapping, got {other:?}"),
        }
    }

    #[test]
    fn source_map_location_mapping_empty_halves_still_well_formed() {
        // `|web/` and `fs/|` are well-formed under the pipe rule
        // — only the presence of the pipe is required. Empty
        // halves are accepted by CC; the actual path-rewriting
        // step would catch any issues later when source maps
        // emit (post-CLOC11.07).
        let cfg = config_from_parsed(&parse(&[
            "--source_map_location_mapping", "|/static/js/",
        ]))
        .expect("ok");
        assert_eq!(cfg.source_map.location_mappings.len(), 1);
        assert_eq!(
            cfg.source_map.location_mappings[0].filesystem_path,
            ""
        );
        assert_eq!(
            cfg.source_map.location_mappings[0].web_server_path,
            "/static/js/"
        );
    }
}
