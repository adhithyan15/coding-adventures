use coding_adventures_sha256::sha256_hex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: u32 = 1;
pub const UPSTREAM_PROJECT: &str = "google/closure-compiler";
pub const UPSTREAM_RELEASE: &str = "v20260915";
pub const UPSTREAM_COMMIT: &str = "10ca677aff381d2c2e6e1b254ba32861e503173d";
pub const UPSTREAM_SOURCE_PATH: &str = "src/com/google/javascript/jscomp/CommandLineRunner.java";
pub const UPSTREAM_BLOB_SHA1: &str = "8f3967499d8793d4f7a79b414428366a4796df67";
pub const UPSTREAM_SOURCE_SHA256: &str =
    "153d1bf4d285f3a40ef032acb36b77b9798b87e53da26de212014f68641203c7";
pub const UPSTREAM_SOURCE_SIZE: usize = 87_726;
pub const EXPECTED_UPSTREAM_OPTIONS: usize = 102;
pub const UPSTREAM_SURFACE_SHA256: &str =
    "a7d2346f5ad4bbe77acfe8bc176065d08ea3a8fa9ceb4ee4d05ade68644c8947";

const GENERATED_FLAGS: &[&str] = &["help", "version"];
const LOCAL_EXTENSIONS: &[&str] = &[
    "correlation_vector",
    "correlation_vector_filter",
    "correlation_vector_filter_includes_origin",
    "correlation_vector_filter_invert",
    "correlation_vector_format",
    "correlation_vector_output",
    "correlation_vector_pretty",
    "correlation_vector_summary",
    "correlation_vector_summary_format",
    "correlation_vector_summary_only",
    "correlation_vector_summary_stderr",
];
const DEPRECATED_TYPED_AST_ALIAS: &str = "typed_ast_output_file__INTENRNAL_USE_ONLY";
const CANONICAL_TYPED_AST_FLAG: &str = "typed_ast_output_file";
const EXPECTED_UPSTREAM_ALIASES: &[(&str, &str)] = &[
    ("--D", "define"),
    ("--checks-only", "checks_only"),
    ("--dev_mode", "jscomp_dev_mode"),
    ("--warnings_whitelist_file", "warnings_allowlist_file"),
    ("-D", "define"),
    ("-O", "compilation_level"),
    ("-W", "warning_level"),
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OptionSurface {
    pub name: String,
    pub aliases: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePin {
    pub project: String,
    pub release: String,
    pub commit_sha: String,
    pub path: String,
    pub git_blob_sha1: String,
    pub size_bytes: usize,
    pub sha256: String,
    pub extracted_surface_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Counts {
    pub upstream_options: usize,
    pub upstream_aliases: usize,
    pub local_spec_flags: usize,
    pub direct_upstream_flags: usize,
    pub generated_flags: usize,
    pub local_extensions: usize,
    pub deprecated_aliases: usize,
    pub unsupported_upstream_flags: usize,
    pub unsupported_upstream_aliases: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AliasDisposition {
    pub name: String,
    pub canonical: String,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Classification {
    pub direct_upstream_flags: Vec<String>,
    pub generated_flags: Vec<String>,
    pub local_extensions: Vec<String>,
    pub deprecated_aliases: Vec<AliasDisposition>,
    pub unsupported_upstream_flags: Vec<AliasDisposition>,
    pub unsupported_upstream_aliases: Vec<AliasDisposition>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuditReport {
    pub schema_version: u32,
    pub upstream_source: SourcePin,
    pub local_spec_path: String,
    pub local_spec_sha256: String,
    pub counts: Counts,
    pub upstream_options: Vec<OptionSurface>,
    pub classification: Classification,
}

#[derive(Debug)]
struct LocalFlagSurface {
    long: BTreeSet<String>,
    long_aliases: BTreeMap<String, String>,
    short: BTreeSet<String>,
}

#[allow(dead_code)] // Used by the generator target; the verifier includes this shared module too.
fn quoted_assignment(body: &str, marker: &str) -> Result<String, String> {
    let start = body
        .find(marker)
        .ok_or_else(|| format!("option block is missing {marker:?}"))?
        + marker.len();
    let rest = &body[start..];
    let end = rest
        .find('"')
        .ok_or_else(|| format!("unterminated quoted assignment after {marker:?}"))?;
    Ok(rest[..end].to_string())
}

#[allow(dead_code)] // Used by the generator target; the verifier includes this shared module too.
fn quoted_values(body: &str) -> Result<Vec<String>, String> {
    let mut values = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find('"') {
        rest = &rest[start + 1..];
        let end = rest
            .find('"')
            .ok_or_else(|| "unterminated alias string".to_string())?;
        values.push(rest[..end].to_string());
        rest = &rest[end + 1..];
    }
    Ok(values)
}

#[allow(dead_code)] // Used by the generator target; the verifier includes this shared module too.
pub fn parse_upstream_options(source: &str) -> Result<Vec<OptionSurface>, String> {
    let mut options = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find("@Option(") {
        rest = &rest[start + "@Option(".len()..];
        let end = rest
            .find("\n    private ")
            .ok_or_else(|| "@Option block has no following private field".to_string())?;
        let body = &rest[..end];
        let raw_name = quoted_assignment(body, "name = \"")?;
        let name = raw_name
            .strip_prefix("--")
            .ok_or_else(|| format!("canonical option is not long-form: {raw_name}"))?
            .to_string();
        let aliases = if let Some(alias_start) = body.find("aliases = {") {
            let alias_body = &body[alias_start + "aliases = {".len()..];
            let alias_end = alias_body
                .find('}')
                .ok_or_else(|| format!("unterminated aliases for --{name}"))?;
            quoted_values(&alias_body[..alias_end])?
        } else {
            Vec::new()
        };
        options.push(OptionSurface { name, aliases });
        rest = &rest[end + 1..];
    }
    options.sort_by(|left, right| left.name.cmp(&right.name));
    let names: BTreeSet<_> = options.iter().map(|option| option.name.as_str()).collect();
    if names.len() != options.len() {
        return Err("upstream source contains duplicate canonical option names".to_string());
    }
    Ok(options)
}

fn parse_local_surface(spec: &[u8]) -> Result<LocalFlagSurface, String> {
    let root: Value = serde_json::from_slice(spec).map_err(|error| error.to_string())?;
    let flags = root
        .get("flags")
        .and_then(Value::as_array)
        .ok_or_else(|| "cli.spec.json has no flags array".to_string())?;
    let mut long = BTreeSet::new();
    let mut long_aliases = BTreeMap::new();
    let mut short = BTreeSet::new();
    for flag in flags {
        let id = flag
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "local flag has no string id".to_string())?;
        let canonical_long = flag.get("long").and_then(Value::as_str);
        if let Some(name) = canonical_long {
            if !long.insert(name.to_string()) {
                return Err(format!("duplicate local long flag --{name}"));
            }
        }
        if let Some(value) = flag.get("long_aliases") {
            let aliases = value
                .as_array()
                .ok_or_else(|| format!("local flag {id:?} has non-array long_aliases"))?;
            let canonical = canonical_long.ok_or_else(|| {
                format!("local flag {id:?} has long_aliases without a canonical long form")
            })?;
            for alias in aliases {
                let alias = alias.as_str().ok_or_else(|| {
                    format!("local flag {id:?} has a non-string long alias")
                })?;
                if alias.is_empty() || alias.starts_with('-') {
                    return Err(format!(
                        "local flag {id:?} has invalid long alias {alias:?}"
                    ));
                }
                if long.contains(alias) || long_aliases.contains_key(alias) {
                    return Err(format!("duplicate local long spelling --{alias}"));
                }
                long_aliases.insert(alias.to_string(), canonical.to_string());
            }
        }
        if let Some(name) = flag.get("short").and_then(Value::as_str) {
            if !short.insert(name.to_string()) {
                return Err(format!("duplicate local short flag -{name}"));
            }
        }
        if flag.get("long").is_none()
            && flag.get("short").is_none()
            && flag.get("single_dash_long").is_none()
        {
            return Err(format!("local flag {id:?} has no command-line form"));
        }
    }
    if let Some(collision) = long
        .iter()
        .find(|canonical| long_aliases.contains_key(canonical.as_str()))
    {
        return Err(format!("duplicate local long spelling --{collision}"));
    }
    Ok(LocalFlagSurface {
        long,
        long_aliases,
        short,
    })
}

fn deprecated_alias() -> AliasDisposition {
    AliasDisposition {
        name: DEPRECATED_TYPED_AST_ALIAS.to_string(),
        canonical: CANONICAL_TYPED_AST_FLAG.to_string(),
        reason: "pre-CCR-005 closurec misspelling retained for compatibility".to_string(),
    }
}

fn unsupported_aliases() -> Vec<AliasDisposition> {
    Vec::new()
}

fn source_pin() -> SourcePin {
    SourcePin {
        project: UPSTREAM_PROJECT.to_string(),
        release: UPSTREAM_RELEASE.to_string(),
        commit_sha: UPSTREAM_COMMIT.to_string(),
        path: UPSTREAM_SOURCE_PATH.to_string(),
        git_blob_sha1: UPSTREAM_BLOB_SHA1.to_string(),
        size_bytes: UPSTREAM_SOURCE_SIZE,
        sha256: UPSTREAM_SOURCE_SHA256.to_string(),
        extracted_surface_sha256: UPSTREAM_SURFACE_SHA256.to_string(),
    }
}

fn surface_hash(options: &[OptionSurface]) -> String {
    let mut bytes = Vec::new();
    for option in options {
        bytes.extend_from_slice(option.name.as_bytes());
        bytes.push(0);
        for alias in &option.aliases {
            bytes.extend_from_slice(alias.as_bytes());
            bytes.push(0);
        }
        bytes.push(b'\n');
    }
    sha256_hex(&bytes)
}

fn alias_map(options: &[OptionSurface]) -> BTreeSet<(String, String)> {
    options
        .iter()
        .flat_map(|option| {
            option
                .aliases
                .iter()
                .map(|alias| (alias.clone(), option.name.clone()))
        })
        .collect()
}

fn expected_alias_map() -> BTreeSet<(String, String)> {
    EXPECTED_UPSTREAM_ALIASES
        .iter()
        .map(|(alias, canonical)| ((*alias).to_string(), (*canonical).to_string()))
        .collect()
}

fn long_alias_map(options: &[OptionSurface]) -> BTreeMap<String, String> {
    options
        .iter()
        .flat_map(|option| {
            option.aliases.iter().filter_map(|alias| {
                alias
                    .strip_prefix("--")
                    .map(|alias| (alias.to_string(), option.name.clone()))
            })
        })
        .collect()
}

fn validate_local_long_aliases(
    actual: &BTreeMap<String, String>,
    expected: &BTreeMap<String, String>,
    errors: &mut Vec<String>,
) {
    if actual == expected {
        return;
    }
    let missing: Vec<_> = expected
        .iter()
        .filter(|(alias, canonical)| actual.get(*alias) != Some(*canonical))
        .map(|(alias, canonical)| format!("--{alias} -> --{canonical}"))
        .collect();
    let unclassified: Vec<_> = actual
        .iter()
        .filter(|(alias, canonical)| expected.get(*alias) != Some(*canonical))
        .map(|(alias, canonical)| format!("--{alias} -> --{canonical}"))
        .collect();
    if !missing.is_empty() {
        errors.push(format!("missing local long aliases: {missing:?}"));
    }
    if !unclassified.is_empty() {
        errors.push(format!(
            "unclassified local long aliases: {unclassified:?}"
        ));
    }
}

fn require_strict_order(label: &str, values: &[String], errors: &mut Vec<String>) {
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        errors.push(format!("{label} must be strictly sorted and unique"));
    }
}

#[allow(dead_code)] // Used by the generator target; the verifier includes this shared module too.
pub fn generate_report(source: &[u8], local_spec: &[u8]) -> Result<AuditReport, Vec<String>> {
    let mut errors = Vec::new();
    if source.len() != UPSTREAM_SOURCE_SIZE {
        errors.push(format!(
            "upstream source size mismatch: got {}, expected {UPSTREAM_SOURCE_SIZE}",
            source.len()
        ));
    }
    let source_hash = sha256_hex(source);
    if source_hash != UPSTREAM_SOURCE_SHA256 {
        errors.push(format!(
            "upstream source SHA-256 mismatch: got {source_hash}, expected {UPSTREAM_SOURCE_SHA256}"
        ));
    }
    let source_text = match std::str::from_utf8(source) {
        Ok(text) => text,
        Err(error) => {
            errors.push(format!("upstream source is not UTF-8: {error}"));
            ""
        }
    };
    let options = match parse_upstream_options(source_text) {
        Ok(options) => options,
        Err(error) => {
            errors.push(error);
            Vec::new()
        }
    };
    if options.len() != EXPECTED_UPSTREAM_OPTIONS {
        errors.push(format!(
            "upstream option count mismatch: got {}, expected {EXPECTED_UPSTREAM_OPTIONS}",
            options.len()
        ));
    }
    let extracted_hash = surface_hash(&options);
    if extracted_hash != UPSTREAM_SURFACE_SHA256 {
        errors.push(format!(
            "extracted upstream surface SHA-256 mismatch: got {extracted_hash}, expected {UPSTREAM_SURFACE_SHA256}"
        ));
    }
    if alias_map(&options) != expected_alias_map() {
        errors.push("extracted upstream alias map does not match the reviewed map".to_string());
    }
    let local = match parse_local_surface(local_spec) {
        Ok(local) => local,
        Err(error) => {
            errors.push(error);
            LocalFlagSurface {
                long: BTreeSet::new(),
                long_aliases: BTreeMap::new(),
                short: BTreeSet::new(),
            }
        }
    };
    if !errors.is_empty() {
        return Err(errors);
    }

    let generated: BTreeSet<_> = GENERATED_FLAGS
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    let extensions: BTreeSet<_> = LOCAL_EXTENSIONS
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    let upstream: BTreeSet<_> = options.iter().map(|option| option.name.clone()).collect();
    let direct: BTreeSet<_> = upstream.difference(&generated).cloned().collect();
    let mut expected_local = direct.clone();
    expected_local.extend(extensions.iter().cloned());
    expected_local.insert(DEPRECATED_TYPED_AST_ALIAS.to_string());
    if local.long != expected_local {
        let missing: Vec<_> = expected_local.difference(&local.long).cloned().collect();
        let unclassified: Vec<_> = local.long.difference(&expected_local).cloned().collect();
        if !missing.is_empty() {
            errors.push(format!(
                "local CLI is missing classified flags: {missing:?}"
            ));
        }
        if !unclassified.is_empty() {
            errors.push(format!(
                "local CLI has unclassified flags: {unclassified:?}"
            ));
        }
    }
    validate_local_long_aliases(&local.long_aliases, &long_alias_map(&options), &mut errors);

    let mut alias_count = 0;
    for option in &options {
        for alias in &option.aliases {
            alias_count += 1;
            if let Some(short) = alias
                .strip_prefix('-')
                .filter(|value| !value.starts_with('-'))
            {
                if !local.short.contains(short) {
                    errors.push(format!(
                        "upstream alias {alias} for --{} is not represented locally",
                        option.name
                    ));
                }
            } else if let Some(long_alias) = alias.strip_prefix("--") {
                if local.long_aliases.get(long_alias) != Some(&option.name) {
                    errors.push(format!(
                        "upstream alias {alias} for --{} is not represented locally",
                        option.name
                    ));
                }
            } else {
                errors.push(format!("upstream alias {alias} has an invalid spelling"));
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(AuditReport {
        schema_version: SCHEMA_VERSION,
        upstream_source: source_pin(),
        local_spec_path: "cli.spec.json".to_string(),
        local_spec_sha256: sha256_hex(local_spec),
        counts: Counts {
            upstream_options: options.len(),
            upstream_aliases: alias_count,
            local_spec_flags: local.long.len(),
            direct_upstream_flags: direct.len(),
            generated_flags: generated.len(),
            local_extensions: extensions.len(),
            deprecated_aliases: 1,
            unsupported_upstream_flags: 0,
            unsupported_upstream_aliases: 0,
        },
        upstream_options: options,
        classification: Classification {
            direct_upstream_flags: direct.into_iter().collect(),
            generated_flags: generated.into_iter().collect(),
            local_extensions: extensions.into_iter().collect(),
            deprecated_aliases: vec![deprecated_alias()],
            unsupported_upstream_flags: Vec::new(),
            unsupported_upstream_aliases: unsupported_aliases(),
        },
    })
}

pub fn verify_report(report: &AuditReport, local_spec: &[u8]) -> Vec<String> {
    let mut errors = Vec::new();
    if report.schema_version != SCHEMA_VERSION {
        errors.push(format!(
            "unsupported audit schema {}, expected {SCHEMA_VERSION}",
            report.schema_version
        ));
    }
    if report.upstream_source != source_pin() {
        errors.push("upstream source pin does not match the reviewed pin".to_string());
    }
    if report.local_spec_path != "cli.spec.json" {
        errors.push("local spec path must be cli.spec.json".to_string());
    }
    let local_hash = sha256_hex(local_spec);
    if report.local_spec_sha256 != local_hash {
        errors.push(format!(
            "local spec SHA-256 mismatch: report {}, actual {local_hash}",
            report.local_spec_sha256
        ));
    }

    let local = match parse_local_surface(local_spec) {
        Ok(local) => local,
        Err(error) => {
            errors.push(error);
            return errors;
        }
    };
    let upstream_names: BTreeSet<_> = report
        .upstream_options
        .iter()
        .map(|option| option.name.clone())
        .collect();
    if upstream_names.len() != report.upstream_options.len() {
        errors.push("report contains duplicate upstream option names".to_string());
    }
    if report
        .upstream_options
        .windows(2)
        .any(|pair| pair[0].name >= pair[1].name)
    {
        errors.push("upstream options are not strictly sorted".to_string());
    }
    if upstream_names.len() != EXPECTED_UPSTREAM_OPTIONS {
        errors.push(format!(
            "report has {} upstream options, expected {EXPECTED_UPSTREAM_OPTIONS}",
            upstream_names.len()
        ));
    }
    let extracted_hash = surface_hash(&report.upstream_options);
    if extracted_hash != UPSTREAM_SURFACE_SHA256 {
        errors.push(format!(
            "reported upstream surface SHA-256 mismatch: got {extracted_hash}, expected {UPSTREAM_SURFACE_SHA256}"
        ));
    }
    if alias_map(&report.upstream_options) != expected_alias_map() {
        errors.push("reported upstream alias map does not match the reviewed map".to_string());
    }

    let direct: BTreeSet<_> = report
        .classification
        .direct_upstream_flags
        .iter()
        .cloned()
        .collect();
    let generated: BTreeSet<_> = report
        .classification
        .generated_flags
        .iter()
        .cloned()
        .collect();
    let extensions: BTreeSet<_> = report
        .classification
        .local_extensions
        .iter()
        .cloned()
        .collect();
    let unsupported_flags: BTreeSet<_> = report
        .classification
        .unsupported_upstream_flags
        .iter()
        .map(|item| item.name.clone())
        .collect();
    require_strict_order(
        "direct upstream flag classification",
        &report.classification.direct_upstream_flags,
        &mut errors,
    );
    require_strict_order(
        "generated flag classification",
        &report.classification.generated_flags,
        &mut errors,
    );
    require_strict_order(
        "local extension classification",
        &report.classification.local_extensions,
        &mut errors,
    );
    let unsupported_flag_names: Vec<_> = report
        .classification
        .unsupported_upstream_flags
        .iter()
        .map(|item| item.name.clone())
        .collect();
    require_strict_order(
        "unsupported upstream flag classification",
        &unsupported_flag_names,
        &mut errors,
    );
    let expected_generated: BTreeSet<_> = GENERATED_FLAGS
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    let expected_extensions: BTreeSet<_> = LOCAL_EXTENSIONS
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    if generated != expected_generated {
        errors.push("generated flag classification drifted".to_string());
    }
    if extensions != expected_extensions {
        errors.push("local extension classification drifted".to_string());
    }

    let mut owners: BTreeMap<String, usize> = BTreeMap::new();
    for name in direct
        .iter()
        .chain(generated.iter())
        .chain(unsupported_flags.iter())
    {
        *owners.entry(name.clone()).or_default() += 1;
    }
    for (name, count) in &owners {
        if *count != 1 {
            errors.push(format!(
                "upstream flag --{name} has {count} classification owners"
            ));
        }
    }
    let classified_upstream: BTreeSet<_> = owners.keys().cloned().collect();
    if classified_upstream != upstream_names {
        let missing: Vec<_> = upstream_names
            .difference(&classified_upstream)
            .cloned()
            .collect();
        let stale: Vec<_> = classified_upstream
            .difference(&upstream_names)
            .cloned()
            .collect();
        errors.push(format!(
            "upstream classification mismatch; missing={missing:?}, stale={stale:?}"
        ));
    }

    let deprecated = &report.classification.deprecated_aliases;
    if deprecated != &vec![deprecated_alias()] {
        errors.push("deprecated alias classification drifted".to_string());
    }
    let mut expected_local = direct.clone();
    expected_local.extend(extensions.iter().cloned());
    expected_local.extend(deprecated.iter().map(|item| item.name.clone()));
    if local.long != expected_local {
        let missing: Vec<_> = expected_local.difference(&local.long).cloned().collect();
        let unclassified: Vec<_> = local.long.difference(&expected_local).cloned().collect();
        errors.push(format!(
            "local surface mismatch; missing={missing:?}, unclassified={unclassified:?}"
        ));
    }
    validate_local_long_aliases(
        &local.long_aliases,
        &long_alias_map(&report.upstream_options),
        &mut errors,
    );

    let expected_unsupported_aliases = unsupported_aliases();
    if report.classification.unsupported_upstream_aliases != expected_unsupported_aliases {
        errors.push("unsupported upstream alias classification drifted".to_string());
    }
    let mut alias_count = 0;
    for option in &report.upstream_options {
        for alias in &option.aliases {
            alias_count += 1;
            if let Some(short) = alias
                .strip_prefix('-')
                .filter(|value| !value.starts_with('-'))
            {
                if !local.short.contains(short) {
                    errors.push(format!(
                        "supported upstream alias {alias} is missing locally"
                    ));
                }
            } else if let Some(long_alias) = alias.strip_prefix("--") {
                if local.long_aliases.get(long_alias) != Some(&option.name) {
                    errors.push(format!(
                        "supported upstream alias {alias} for --{} is missing locally",
                        option.name
                    ));
                }
            } else {
                errors.push(format!("upstream alias {alias} has an invalid spelling"));
            }
        }
    }

    let expected_counts = Counts {
        upstream_options: report.upstream_options.len(),
        upstream_aliases: alias_count,
        local_spec_flags: local.long.len(),
        direct_upstream_flags: direct.len(),
        generated_flags: generated.len(),
        local_extensions: extensions.len(),
        deprecated_aliases: deprecated.len(),
        unsupported_upstream_flags: unsupported_flags.len(),
        unsupported_upstream_aliases: report.classification.unsupported_upstream_aliases.len(),
    };
    if report.counts != expected_counts {
        errors.push("audit counts do not match the classified surface".to_string());
    }
    errors
}
