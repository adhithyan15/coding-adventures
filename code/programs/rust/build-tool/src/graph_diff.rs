//! Pure, bounded graph and diff-selection operations.
//!
//! Callers provide already-decoded inert values. This module does not inspect a
//! checkout, invoke Git, read an environment, launch a process, consult a
//! clock, use randomness, or access a network. Every public evaluation returns
//! either one complete deterministic result or one stable error code.

use crate::glob_match::{compile_patterns, match_compiled_path, CompiledPattern};
use oxixml_unicode::{case, norm};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const MAX_GRAPH_PACKAGES: usize = 4_096;
pub const MAX_GRAPH_EDGES: usize = 16_384;
pub const MAX_SOURCE_GLOBS: usize = 256;
pub const MAX_DIFF_SELECTION_MATCH_WORK: u64 = 50_000_000;

const MAX_PATH_SCALARS: usize = 512;
const MAX_PACKAGE_NAME_SCALARS: usize = 240;
const MAX_BOUNDARY_DEPTH: usize = 16;
const MAX_BOUNDARY_NODES: usize = 65_536;
const MAX_BOUNDARY_CONTAINERS: usize = 4_096;
const MAX_BOUNDARY_STRING_BYTES: usize = 16_777_216;
const BOUNDARY_DOMAIN: &[u8] =
    b"coding-adventures/build-tool-repository-source-input-boundary/v1\0";
const BUILD_FRONTS: &[&str] = &[
    "BUILD",
    "BUILD_windows",
    "BUILD_mac",
    "BUILD_linux",
    "BUILD_mac_and_linux",
];
const BOUNDARY_ORIGINS: &[&str] = &[
    "c",
    "cpp",
    "csharp",
    "dart",
    "dotnet",
    "elixir",
    "fsharp",
    "go",
    "haskell",
    "java",
    "kotlin",
    "lua",
    "mosaic",
    "ocaml",
    "perl",
    "python",
    "repository",
    "ruby",
    "rust",
    "starlark",
    "swift",
    "twig",
    "typescript",
    "wasm",
];

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Edge {
    pub prerequisite: String,
    pub dependent: String,
}

impl Edge {
    pub fn new(prerequisite: String, dependent: String) -> Self {
        Self {
            prerequisite,
            dependent,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphInput {
    pub packages: Vec<String>,
    pub edges: Vec<Edge>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphResult {
    pub edges: Vec<Edge>,
    pub levels: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PackageSpec {
    pub name: String,
    pub rel_path: String,
    pub source_mode: String,
    #[serde(default)]
    pub source_globs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DiffSelectionInput {
    pub packages: Vec<PackageSpec>,
    pub edges: Vec<Edge>,
    pub forced_packages: Vec<String>,
    pub unknown_path_policy: String,
    pub changed_paths: Vec<String>,
    pub boundary_sha256: String,
    pub boundary: Option<Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffSelectionResult {
    pub changed_packages: Vec<String>,
    pub affected_packages: Vec<String>,
    pub prerequisite_packages: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Evaluation<T> {
    Success(T),
    Failure(&'static str),
}

#[derive(Debug)]
struct ValidatedGraph {
    names: BTreeSet<String>,
    edges: Vec<Edge>,
    dependents: BTreeMap<String, Vec<String>>,
    prerequisites: BTreeMap<String, Vec<String>>,
}

#[derive(Debug)]
struct ValidatedPackage<'a> {
    spec: &'a PackageSpec,
    compiled_globs: Vec<CompiledPattern>,
}

/// Validate and evaluate canonical graph edges and prerequisite-first levels.
pub fn evaluate_graph(input: &GraphInput) -> Evaluation<GraphResult> {
    let graph = match validate_graph(&input.packages, &input.edges) {
        Ok(graph) => graph,
        Err(code) => return Evaluation::Failure(code),
    };
    let Some(levels) = graph_levels(&graph) else {
        return Evaluation::Failure("GRAPH_CYCLE");
    };
    Evaluation::Success(GraphResult {
        edges: graph.edges,
        levels,
    })
}

/// Select changed packages and their deterministic graph closures.
pub fn evaluate_diff_selection(input: &DiffSelectionInput) -> Evaluation<DiffSelectionResult> {
    match try_evaluate_diff_selection(input) {
        Ok(result) => Evaluation::Success(result),
        Err(code) => Evaluation::Failure(code),
    }
}

fn try_evaluate_diff_selection(
    input: &DiffSelectionInput,
) -> Result<DiffSelectionResult, &'static str> {
    if input.packages.len() > MAX_GRAPH_PACKAGES {
        return Err("DIFF_PACKAGE_INVALID");
    }
    let names = input
        .packages
        .iter()
        .map(|package| package.name.clone())
        .collect::<Vec<_>>();
    let graph = validate_graph(&names, &input.edges)?;
    if graph_levels(&graph).is_none() {
        return Err("DIFF_EDGE_CYCLE");
    }
    let packages = validate_diff_input(input, &graph)?;
    let boundary_consumers = boundary_reverse_index(input, &packages)?;
    preflight_match_work(&packages, &input.changed_paths)?;

    let mut changed = input
        .forced_packages
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut unknown = false;
    for path in &input.changed_paths {
        let consumers = boundary_consumers.get(path);
        if let Some(consumers) = consumers {
            changed.extend(consumers.iter().cloned());
        }
        let mut known = consumers.is_some_and(|values| !values.is_empty());
        for package in &packages {
            if !inside(path, &package.spec.rel_path) {
                continue;
            }
            known = true;
            let relative = relative_path(path, &package.spec.rel_path);
            let selected = package.spec.source_mode == "package_prefix"
                || is_build_front(relative)
                || package
                    .compiled_globs
                    .iter()
                    .any(|pattern| match_compiled_path(pattern, relative));
            if selected {
                changed.insert(package.spec.name.clone());
            }
        }
        if !known {
            unknown = true;
        }
    }

    if unknown {
        if input.unknown_path_policy == "error" {
            return Err("DIFF_UNKNOWN_PATH");
        }
        changed = graph.names.clone();
    }

    let affected = closure(&changed, &graph.dependents);
    let mut prerequisites = closure(&affected, &graph.prerequisites);
    for name in &affected {
        prerequisites.remove(name);
    }
    Ok(DiffSelectionResult {
        changed_packages: changed.into_iter().collect(),
        affected_packages: affected.into_iter().collect(),
        prerequisite_packages: prerequisites.into_iter().collect(),
    })
}

fn validate_graph(packages: &[String], edges: &[Edge]) -> Result<ValidatedGraph, &'static str> {
    if packages.len() > MAX_GRAPH_PACKAGES {
        return Err("GRAPH_PACKAGE_LIMIT_EXCEEDED");
    }
    if edges.len() > MAX_GRAPH_EDGES {
        return Err("GRAPH_EDGE_LIMIT_EXCEEDED");
    }

    let mut names = BTreeSet::new();
    let mut dependents = BTreeMap::new();
    let mut prerequisites = BTreeMap::new();
    for name in packages {
        if !valid_package_name(name) {
            return Err("GRAPH_PACKAGE_INVALID");
        }
        if !names.insert(name.clone()) {
            return Err("GRAPH_PACKAGE_DUPLICATE");
        }
        dependents.insert(name.clone(), Vec::new());
        prerequisites.insert(name.clone(), Vec::new());
    }

    let mut seen = BTreeSet::new();
    let mut canonical = Vec::with_capacity(edges.len());
    for edge in edges {
        if !names.contains(&edge.prerequisite) || !names.contains(&edge.dependent) {
            return Err("GRAPH_EDGE_UNKNOWN");
        }
        if edge.prerequisite == edge.dependent {
            return Err("GRAPH_EDGE_SELF");
        }
        if !seen.insert(edge.clone()) {
            return Err("GRAPH_EDGE_DUPLICATE");
        }
        dependents
            .get_mut(&edge.prerequisite)
            .expect("validated prerequisite")
            .push(edge.dependent.clone());
        prerequisites
            .get_mut(&edge.dependent)
            .expect("validated dependent")
            .push(edge.prerequisite.clone());
        canonical.push(edge.clone());
    }
    canonical.sort();
    for values in dependents.values_mut() {
        values.sort();
    }
    for values in prerequisites.values_mut() {
        values.sort();
    }
    Ok(ValidatedGraph {
        names,
        edges: canonical,
        dependents,
        prerequisites,
    })
}

fn graph_levels(graph: &ValidatedGraph) -> Option<Vec<Vec<String>>> {
    let mut indegree = graph
        .names
        .iter()
        .map(|name| {
            (
                name.clone(),
                graph.prerequisites.get(name).expect("complete graph").len(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut ready = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let mut levels = Vec::new();
    let mut visited = 0;
    while !ready.is_empty() {
        let level = ready;
        visited += level.len();
        let mut next = Vec::new();
        for name in &level {
            for dependent in graph.dependents.get(name).expect("complete graph") {
                let degree = indegree.get_mut(dependent).expect("complete graph");
                *degree -= 1;
                if *degree == 0 {
                    next.push(dependent.clone());
                }
            }
        }
        next.sort();
        levels.push(level);
        ready = next;
    }
    (visited == graph.names.len()).then_some(levels)
}

fn validate_diff_input<'a>(
    input: &'a DiffSelectionInput,
    graph: &ValidatedGraph,
) -> Result<Vec<ValidatedPackage<'a>>, &'static str> {
    let mut packages = Vec::with_capacity(input.packages.len());
    let mut names = BTreeSet::new();
    let mut root_identities = Vec::new();
    for package in &input.packages {
        if !valid_package_name(&package.name) {
            return Err("DIFF_PACKAGE_INVALID");
        }
        if !valid_portable_path(&package.rel_path) {
            return Err("DIFF_PATH_INVALID");
        }
        let identity = case::fold_str(&norm::nfc(&package.rel_path));
        if root_identities.iter().any(|prior: &String| {
            identity == *prior
                || identity.starts_with(&format!("{prior}/"))
                || prior.starts_with(&format!("{identity}/"))
        }) {
            return Err("DIFF_PATH_INVALID");
        }
        root_identities.push(identity);

        if package.source_mode != "package_prefix" && package.source_mode != "strict_globs" {
            return Err("DIFF_SOURCE_MODE_INVALID");
        }
        if package.source_globs.len() > MAX_SOURCE_GLOBS
            || has_duplicates(&package.source_globs)
            || package
                .source_globs
                .iter()
                .any(|pattern| !valid_portable_glob(pattern))
            || (package.source_mode != "strict_globs" && !package.source_globs.is_empty())
        {
            return Err("DIFF_GLOB_INVALID");
        }
        let compiled_globs =
            compile_patterns(&package.source_globs).map_err(|_| "DIFF_GLOB_INVALID")?;
        if !names.insert(package.name.clone()) {
            return Err("DIFF_PACKAGE_DUPLICATE");
        }
        packages.push(ValidatedPackage {
            spec: package,
            compiled_globs,
        });
    }
    if names != graph.names {
        return Err("DIFF_PACKAGE_INVALID");
    }
    if input.unknown_path_policy != "all" && input.unknown_path_policy != "error" {
        return Err("DIFF_POLICY_INVALID");
    }
    if input.forced_packages.len() > MAX_GRAPH_PACKAGES
        || has_duplicates(&input.forced_packages)
        || input
            .forced_packages
            .iter()
            .any(|name| !valid_package_name(name))
    {
        return Err("DIFF_FORCED_PACKAGE_INVALID");
    }
    if input.changed_paths.len() > MAX_GRAPH_PACKAGES
        || has_duplicates(&input.changed_paths)
        || input
            .changed_paths
            .iter()
            .any(|path| !valid_portable_path(path))
    {
        return Err("DIFF_PATH_INVALID");
    }
    if input
        .forced_packages
        .iter()
        .any(|name| !names.contains(name))
    {
        return Err("DIFF_FORCED_PACKAGE_UNKNOWN");
    }
    Ok(packages)
}

fn preflight_match_work(
    packages: &[ValidatedPackage<'_>],
    paths: &[String],
) -> Result<(), &'static str> {
    let mut remaining = MAX_DIFF_SELECTION_MATCH_WORK;
    for package in packages {
        if package.spec.source_mode != "strict_globs" {
            continue;
        }
        let pattern_factor = package
            .spec
            .source_globs
            .iter()
            .map(|pattern| pattern.chars().count() as u64 + 1)
            .sum::<u64>();
        for path in paths {
            if !inside(path, &package.spec.rel_path) {
                continue;
            }
            let relative = relative_path(path, &package.spec.rel_path);
            if is_build_front(relative) {
                continue;
            }
            let path_factor = relative.chars().count() as u64 + 1;
            if pattern_factor != 0 && pattern_factor > remaining / path_factor {
                return Err("DIFF_MATCH_LIMIT_EXCEEDED");
            }
            remaining -= pattern_factor * path_factor;
        }
    }
    Ok(())
}

fn boundary_reverse_index(
    input: &DiffSelectionInput,
    packages: &[ValidatedPackage<'_>],
) -> Result<BTreeMap<String, BTreeSet<String>>, &'static str> {
    if input.boundary_sha256.is_empty() {
        if input.boundary.is_some() {
            return Err("DIFF_BOUNDARY_DIGEST_MISMATCH");
        }
        return Ok(BTreeMap::new());
    }
    if !valid_digest(&input.boundary_sha256) {
        return Err("DIFF_BOUNDARY_DIGEST_MISMATCH");
    }
    let boundary_value = input
        .boundary
        .as_ref()
        .ok_or("DIFF_BOUNDARY_DIGEST_MISMATCH")?;
    validate_boundary_value_bounds(boundary_value)?;
    let encoded = canonical_json(boundary_value)?;
    let mut hasher = Sha256::new();
    hasher.update(BOUNDARY_DOMAIN);
    hasher.update((encoded.len() as u64).to_be_bytes());
    hasher.update(encoded.as_bytes());
    let actual = format!("{:x}", hasher.finalize());
    if actual != input.boundary_sha256 {
        return Err("DIFF_BOUNDARY_DIGEST_MISMATCH");
    }
    let boundary = serde_json::from_value::<RepositoryBoundary>(boundary_value.clone())
        .map_err(|_| "DIFF_BOUNDARY_INVALID")?;
    validate_boundary(&boundary)?;

    let mut result = BTreeMap::<String, BTreeSet<String>>::new();
    for package in packages {
        for rule in &boundary.boundaries {
            if !boundary_applies(&rule.applies_to, &package.spec.rel_path) {
                continue;
            }
            for boundary_input in &rule.inputs {
                result
                    .entry(boundary_input.path.clone())
                    .or_default()
                    .insert(package.spec.name.clone());
            }
        }
    }
    Ok(result)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepositoryBoundary {
    schema_version: u64,
    language_source_input_registry_sha256: String,
    boundaries: Vec<BoundaryRule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundaryRule {
    id: String,
    input_origin: String,
    applies_to: BoundaryApplicability,
    inputs: Vec<BoundaryInput>,
    reason: String,
    owner: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundaryApplicability {
    exact_roots: Vec<String>,
    descendant_roots: Vec<String>,
    excluded_roots: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundaryInput {
    path: String,
    role: String,
    #[serde(default)]
    generated_component: Option<String>,
}

fn validate_boundary(boundary: &RepositoryBoundary) -> Result<(), &'static str> {
    if boundary.schema_version != 1
        || !valid_digest(&boundary.language_source_input_registry_sha256)
        || boundary.boundaries.is_empty()
        || boundary.boundaries.len() > 256
    {
        return Err("DIFF_BOUNDARY_INVALID");
    }
    let mut ids = BTreeSet::new();
    let mut scopes = 0usize;
    let mut authorizations = 0usize;
    for rule in &boundary.boundaries {
        if !valid_slug(&rule.id, 120)
            || !BOUNDARY_ORIGINS.contains(&rule.input_origin.as_str())
            || !valid_scalar_string(&rule.reason, 1, 512)
            || !valid_slug(&rule.owner, 120)
            || !ids.insert(rule.id.clone())
            || rule.inputs.is_empty()
            || rule.inputs.len() > 64
        {
            return Err("DIFF_BOUNDARY_INVALID");
        }
        validate_boundary_paths(&rule.applies_to.exact_roots, 4_096)?;
        validate_boundary_paths(&rule.applies_to.descendant_roots, 64)?;
        validate_boundary_paths(&rule.applies_to.excluded_roots, 4_096)?;
        if rule.applies_to.exact_roots.is_empty() && rule.applies_to.descendant_roots.is_empty() {
            return Err("DIFF_BOUNDARY_INVALID");
        }
        let rule_scopes =
            rule.applies_to.exact_roots.len() + rule.applies_to.descendant_roots.len();
        scopes = scopes.saturating_add(rule_scopes);
        authorizations = authorizations.saturating_add(rule.inputs.len() * rule_scopes);
        if scopes > 8_192 || authorizations > 32_768 {
            return Err("DIFF_BOUNDARY_INVALID");
        }
        for input in &rule.inputs {
            if !valid_portable_path(&input.path)
                || !matches!(
                    input.role.as_str(),
                    "cross_package_exact" | "generated_pruning_exception" | "shared_ancestor"
                )
            {
                return Err("DIFF_BOUNDARY_INVALID");
            }
            if input.role == "generated_pruning_exception" {
                if !input
                    .generated_component
                    .as_deref()
                    .is_some_and(valid_generated_component)
                {
                    return Err("DIFF_BOUNDARY_INVALID");
                }
            } else if input.generated_component.is_some() {
                return Err("DIFF_BOUNDARY_INVALID");
            }
        }
    }
    Ok(())
}

fn validate_boundary_paths(values: &[String], maximum: usize) -> Result<(), &'static str> {
    if values.len() > maximum
        || has_duplicates(values)
        || values.iter().any(|value| !valid_portable_path(value))
    {
        return Err("DIFF_BOUNDARY_INVALID");
    }
    Ok(())
}

fn validate_boundary_value_bounds(value: &Value) -> Result<(), &'static str> {
    struct Counters {
        nodes: usize,
        containers: usize,
        string_bytes: usize,
    }
    fn visit(value: &Value, depth: usize, counters: &mut Counters) -> Result<(), &'static str> {
        counters.nodes += 1;
        if counters.nodes > MAX_BOUNDARY_NODES || depth > MAX_BOUNDARY_DEPTH {
            return Err("DIFF_BOUNDARY_INVALID");
        }
        match value {
            Value::String(value) => {
                counters.string_bytes = counters.string_bytes.saturating_add(value.len())
            }
            Value::Array(values) => {
                counters.containers += 1;
                if !values.is_empty() && depth == MAX_BOUNDARY_DEPTH {
                    return Err("DIFF_BOUNDARY_INVALID");
                }
                for value in values {
                    visit(value, depth + 1, counters)?;
                }
            }
            Value::Object(values) => {
                counters.containers += 1;
                if !values.is_empty() && depth == MAX_BOUNDARY_DEPTH {
                    return Err("DIFF_BOUNDARY_INVALID");
                }
                for (key, value) in values {
                    counters.string_bytes = counters.string_bytes.saturating_add(key.len());
                    visit(value, depth + 1, counters)?;
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
        if counters.containers > MAX_BOUNDARY_CONTAINERS
            || counters.string_bytes > MAX_BOUNDARY_STRING_BYTES
        {
            return Err("DIFF_BOUNDARY_INVALID");
        }
        Ok(())
    }

    visit(
        value,
        0,
        &mut Counters {
            nodes: 0,
            containers: 0,
            string_bytes: 0,
        },
    )
}

fn canonical_json(value: &Value) -> Result<String, &'static str> {
    match value {
        Value::Null => Ok("null".to_owned()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(value.to_string()),
        Value::String(value) => serde_json::to_string(value).map_err(|_| "DIFF_BOUNDARY_INVALID"),
        Value::Array(values) => {
            let values = values
                .iter()
                .map(canonical_json)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(format!("[{}]", values.join(",")))
        }
        Value::Object(values) => {
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
            let mut members = Vec::with_capacity(keys.len());
            for key in keys {
                let encoded_key =
                    serde_json::to_string(key).map_err(|_| "DIFF_BOUNDARY_INVALID")?;
                members.push(format!("{encoded_key}:{}", canonical_json(&values[key])?));
            }
            Ok(format!("{{{}}}", members.join(",")))
        }
    }
}

fn valid_package_name(value: &str) -> bool {
    if value.chars().count() > MAX_PACKAGE_NAME_SCALARS || !value.contains('/') {
        return false;
    }
    value.split('/').all(|segment| {
        let mut chars = segment.chars();
        chars
            .next()
            .is_some_and(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
            && chars.all(|character| {
                character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || matches!(character, '.' | '_' | '-')
            })
    })
}

fn valid_portable_path(value: &str) -> bool {
    valid_portable_prefix(value, "<>:\"|?*")
        && value
            .split('/')
            .all(|segment| !invalid_segment(segment) && !reserved_segment(segment))
}

fn valid_portable_glob(value: &str) -> bool {
    valid_portable_prefix(value, "<>:\"|?")
        && value.split('/').all(|segment| {
            !invalid_segment(segment)
                && (segment.chars().any(|character| "*[]{}".contains(character))
                    || !reserved_segment(segment))
        })
}

fn valid_portable_prefix(value: &str, forbidden: &str) -> bool {
    !value.is_empty()
        && value.chars().count() <= MAX_PATH_SCALARS
        && norm::nfc(value) == value
        && !value.starts_with('/')
        && !has_drive_prefix(value)
        && !value.contains('\\')
        && !value.contains("//")
        && !value
            .chars()
            .any(|character| character < '\u{20}' || forbidden.contains(character))
}

fn invalid_segment(segment: &str) -> bool {
    segment.is_empty()
        || matches!(segment, "." | "..")
        || segment.ends_with(' ')
        || segment.ends_with('.')
}

fn reserved_segment(segment: &str) -> bool {
    let base = segment.split('.').next().unwrap_or_default();
    let uppercase = case::full_uppercase(base);
    matches!(
        uppercase.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$" | "CLOCK$"
    ) || uppercase
        .strip_prefix("COM")
        .or_else(|| uppercase.strip_prefix("LPT"))
        .is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
}

fn has_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_slug(value: &str, maximum: usize) -> bool {
    valid_scalar_string(value, 1, maximum)
        && value.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

fn valid_scalar_string(value: &str, minimum: usize, maximum: usize) -> bool {
    let length = value.chars().count();
    (minimum..=maximum).contains(&length)
}

fn valid_generated_component(value: &str) -> bool {
    valid_scalar_string(value, 1, 128)
        && !value
            .chars()
            .any(|character| character < '\u{20}' || "\\/<>:\"|?*".contains(character))
}

fn has_duplicates(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    values.iter().any(|value| !seen.insert(value))
}

fn boundary_applies(value: &BoundaryApplicability, root: &str) -> bool {
    value.exact_roots.iter().any(|candidate| candidate == root)
        || (!value
            .excluded_roots
            .iter()
            .any(|candidate| candidate == root)
            && value
                .descendant_roots
                .iter()
                .any(|parent| root.starts_with(&format!("{parent}/"))))
}

fn inside(path: &str, root: &str) -> bool {
    path == root || path.starts_with(&format!("{root}/"))
}

fn relative_path<'a>(path: &'a str, root: &str) -> &'a str {
    if path == root {
        ""
    } else {
        &path[root.len() + 1..]
    }
}

fn is_build_front(path: &str) -> bool {
    BUILD_FRONTS.contains(&path.rsplit('/').next().unwrap_or(path))
}

fn closure(
    seeds: &BTreeSet<String>,
    adjacency: &BTreeMap<String, Vec<String>>,
) -> BTreeSet<String> {
    let mut result = seeds.clone();
    let mut pending = seeds.iter().cloned().collect::<VecDeque<_>>();
    while let Some(name) = pending.pop_front() {
        for next in adjacency.get(&name).into_iter().flatten() {
            if result.insert(next.clone()) {
                pending.push_back(next.clone());
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(name: &str, path: &str, mode: &str, globs: &[&str]) -> PackageSpec {
        PackageSpec {
            name: name.to_owned(),
            rel_path: path.to_owned(),
            source_mode: mode.to_owned(),
            source_globs: globs.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    fn base_diff_input(packages: Vec<PackageSpec>) -> DiffSelectionInput {
        DiffSelectionInput {
            packages,
            edges: Vec::new(),
            forced_packages: Vec::new(),
            unknown_path_policy: "error".to_owned(),
            changed_paths: Vec::new(),
            boundary_sha256: String::new(),
            boundary: None,
        }
    }

    #[test]
    fn graph_validation_is_all_or_error() {
        let duplicate = GraphInput {
            packages: vec!["fixture/a".to_owned(), "fixture/a".to_owned()],
            edges: Vec::new(),
        };
        assert_eq!(
            evaluate_graph(&duplicate),
            Evaluation::Failure("GRAPH_PACKAGE_DUPLICATE")
        );

        let partial_cycle = GraphInput {
            packages: vec![
                "fixture/a".to_owned(),
                "fixture/b".to_owned(),
                "fixture/free".to_owned(),
            ],
            edges: vec![
                Edge::new("fixture/a".to_owned(), "fixture/b".to_owned()),
                Edge::new("fixture/b".to_owned(), "fixture/a".to_owned()),
            ],
        };
        assert_eq!(
            evaluate_graph(&partial_cycle),
            Evaluation::Failure("GRAPH_CYCLE")
        );
    }

    #[test]
    fn diff_validation_precedes_matching() {
        let mut input = base_diff_input(vec![package(
            "fixture/a",
            "p",
            "strict_globs",
            &["src/**/*.rs", "src/[z-a].rs"],
        )]);
        input.changed_paths.push("p/src/main.rs".to_owned());
        assert_eq!(
            evaluate_diff_selection(&input),
            Evaluation::Failure("DIFF_GLOB_INVALID")
        );
    }

    #[test]
    fn root_identity_uses_nfc_full_casefold() {
        let input = base_diff_input(vec![
            package("fixture/a", "code/P", "package_prefix", &[]),
            package("fixture/b", "code/p/child", "package_prefix", &[]),
        ]);
        assert_eq!(
            evaluate_diff_selection(&input),
            Evaluation::Failure("DIFF_PATH_INVALID")
        );
    }

    #[test]
    fn exact_match_work_ceiling_uses_unicode_scalars() {
        let package = ValidatedPackage {
            spec: &PackageSpec {
                name: "fixture/a".to_owned(),
                rel_path: "p".to_owned(),
                source_mode: "strict_globs".to_owned(),
                source_globs: vec![format!("😀{}", "a".repeat(98))],
            },
            compiled_globs: Vec::new(),
        };
        let paths = (0..1_000)
            .map(|index| format!("p/{index:0499}"))
            .collect::<Vec<_>>();
        assert_eq!(preflight_match_work(&[package], &paths), Ok(()));
    }
}
