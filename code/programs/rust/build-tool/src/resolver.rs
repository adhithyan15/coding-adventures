// Dependency resolver — reads package metadata files and builds a directed graph.
//
// # Why dependency resolution matters
//
// In a monorepo, packages often depend on each other. If package B depends
// on package A, we must build A before B. The resolver reads each package's
// metadata file to discover these relationships, then encodes them as edges
// in a directed graph.
//
// # Dependency naming conventions
//
// Each language ecosystem uses a different naming convention for packages:
//
//   - Python: pyproject.toml uses "coding-adventures-" prefix with hyphens.
//     "coding-adventures-logic-gates" maps to "python/logic-gates".
//
//   - Ruby: .gemspec uses "coding_adventures_" prefix with underscores.
//     "coding_adventures_logic_gates" maps to "ruby/logic_gates".
//
//   - Go: go.mod uses full module paths. We map based on the last path
//     component: "go/directed-graph".
//
//   - Rust: Cargo.toml uses `[dependencies]` with `path = "..."` for
//     workspace-local deps. We match by crate name convention.
//
// External dependencies (those not matching the monorepo prefix) are
// silently skipped — we only care about internal build ordering.
//
// # The directed graph
//
// Edges go FROM dependency TO dependent: if B depends on A, the edge is
// A -> B. This convention means "A must be built before B", and
// independent_groups() naturally produces the correct build order.

use crate::discovery::Package;
use crate::graph::Graph;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

const METADATA_INVALID_UTF8: &str = "METADATA_INVALID_UTF8";
const DEPENDENCY_SELF_EDGE: &str = "DEPENDENCY_SELF_EDGE";

/// Text metadata that violates the repository's strict UTF-8 contract.
///
/// `manifest` is always repository-relative so diagnostics cannot disclose a
/// checkout root or temporary-directory path.
#[derive(Debug, Eq, PartialEq)]
pub struct MetadataEncodingError {
    pub code: String,
    pub package: String,
    pub manifest: String,
    pub encoding: String,
}

impl fmt::Display for MetadataEncodingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: package={} manifest={} encoding={}",
            self.code, self.package, self.manifest, self.encoding
        )
    }
}

impl std::error::Error for MetadataEncodingError {}

/// A package manifest that resolves the package itself as a dependency.
#[derive(Debug, Eq, PartialEq)]
pub struct DependencySelfEdgeError {
    pub code: String,
    pub package: String,
    pub manifest: String,
    pub dependency: String,
}

impl fmt::Display for DependencySelfEdgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: package={} manifest={} dependency={}",
            self.code, self.package, self.manifest, self.dependency
        )
    }
}

impl std::error::Error for DependencySelfEdgeError {}

/// Stable dependency-resolution failures returned to the CLI.
#[derive(Debug, Eq, PartialEq)]
pub enum ResolutionError {
    MetadataEncoding(MetadataEncodingError),
    DependencySelfEdge(DependencySelfEdgeError),
}

impl fmt::Display for ResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolutionError::MetadataEncoding(error) => error.fmt(formatter),
            ResolutionError::DependencySelfEdge(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ResolutionError {}

impl From<MetadataEncodingError> for ResolutionError {
    fn from(error: MetadataEncodingError) -> Self {
        ResolutionError::MetadataEncoding(error)
    }
}

fn repository_manifest_path(path: &Path) -> String {
    let parts: Vec<String> = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    let mut canonical_start = None;
    for index in 0..parts.len().saturating_sub(1) {
        if parts[index] == "code"
            && (parts[index + 1] == "packages" || parts[index + 1] == "programs")
        {
            canonical_start = Some(index);
        }
    }
    match canonical_start {
        Some(index) => parts[index..].join("/"),
        None => path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
    }
}

fn first_manifest_with_suffix(pkg: &Package, suffixes: &[&str]) -> Option<std::path::PathBuf> {
    let mut matches: Vec<std::path::PathBuf> = fs::read_dir(&pkg.path)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .map(|name| {
                    let name = name.to_string_lossy();
                    suffixes.iter().any(|suffix| name.ends_with(suffix))
                })
                .unwrap_or(false)
        })
        .collect();
    matches.sort();
    matches.into_iter().next()
}

fn dependency_manifest_path(pkg: &Package) -> String {
    let manifest = match pkg.language.as_str() {
        "python" => pkg.path.join("pyproject.toml"),
        "go" => pkg.path.join("go.mod"),
        "rust" | "wasm" => pkg.path.join("Cargo.toml"),
        "elixir" => pkg.path.join("mix.exs"),
        "perl" => pkg.path.join("Makefile.PL"),
        "ruby" => {
            first_manifest_with_suffix(pkg, &[".gemspec"]).unwrap_or_else(|| pkg.path.join("BUILD"))
        }
        "lua" => first_manifest_with_suffix(pkg, &[".rockspec"])
            .unwrap_or_else(|| pkg.path.join("BUILD")),
        "haskell" => {
            first_manifest_with_suffix(pkg, &[".cabal"]).unwrap_or_else(|| pkg.path.join("BUILD"))
        }
        "csharp" | "fsharp" | "dotnet" => first_manifest_with_suffix(pkg, &[".csproj", ".fsproj"])
            .unwrap_or_else(|| pkg.path.join("BUILD")),
        _ => pkg.path.join("BUILD"),
    };
    repository_manifest_path(&manifest)
}

// ---------------------------------------------------------------------------
// Known-name mapping
// ---------------------------------------------------------------------------

/// Creates a mapping from ecosystem-specific dependency names to our
/// internal package names.
///
/// This mapping is the "Rosetta Stone" of our build system. Each language
/// ecosystem uses its own naming convention for packages:
///
///   - Python: "coding-adventures-logic-gates" -> "python/logic-gates"
///   - Ruby:   "coding_adventures_logic_gates" -> "ruby/logic_gates"
///   - Go:     full module path -> "go/module-name"
///   - Rust:   crate name -> "rust/crate-name"
///
/// By building this mapping upfront, we can resolve dependencies across
/// languages without hard-coding specific package names.
///
/// Only the tests build the unscoped map directly; production code always
/// resolves within a language scope via [`build_known_names_for_scope`].
#[cfg(test)]
pub fn build_known_names(packages: &[Package]) -> HashMap<String, String> {
    build_known_names_for_scope(packages, "")
}

fn dependency_scope(language: &str) -> &str {
    match language {
        "csharp" | "fsharp" | "dotnet" => "dotnet",
        "wasm" => "wasm",
        _ => language,
    }
}

fn in_dependency_scope(package_language: &str, scope: &str) -> bool {
    match scope {
        "dotnet" => matches!(package_language, "csharp" | "fsharp" | "dotnet"),
        "wasm" => matches!(package_language, "wasm" | "rust"),
        _ => package_language == scope,
    }
}

fn read_cargo_package_name(pkg: &Package) -> Option<String> {
    let cargo_toml = pkg.path.join("Cargo.toml");
    let data = fs::read_to_string(cargo_toml).ok()?;
    let parsed = data.parse::<toml::Table>().ok()?;
    let package = parsed.get("package")?;
    let name = package.get("name")?.as_str()?;
    Some(name.to_lowercase())
}

fn build_known_names_for_scope(packages: &[Package], scope: &str) -> HashMap<String, String> {
    let mut known = HashMap::new();

    let set_known = |known: &mut HashMap<String, String>, key: String, pkg: &Package| {
        let current_is_program = pkg.name.split('/').nth(1) == Some("programs");
        if let Some(existing) = known.get(&key) {
            let existing_is_program = existing.split('/').nth(1) == Some("programs");
            if !existing_is_program && current_is_program {
                return;
            }
        }
        known.insert(key, pkg.name.clone());
    };

    for pkg in packages {
        if !scope.is_empty() && !in_dependency_scope(&pkg.language, scope) {
            continue;
        }
        let dir_name = pkg
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        match pkg.language.as_str() {
            "python" => {
                // Convert dir name to PyPI name: "logic-gates" -> "coding-adventures-logic-gates"
                let pypi_name = format!("coding-adventures-{}", dir_name);
                set_known(&mut known, pypi_name, pkg);
            }
            "ruby" => {
                // Convert dir name to gem name: "logic_gates" -> "coding_adventures_logic_gates"
                let gem_name = format!("coding_adventures_{}", dir_name);
                set_known(&mut known, gem_name, pkg);
            }
            "go" => {
                // For Go, read the module path from go.mod.
                let go_mod = pkg.path.join("go.mod");
                if let Ok(data) = fs::read_to_string(&go_mod) {
                    for line in data.lines() {
                        if line.starts_with("module ") {
                            let module_path =
                                line.trim_start_matches("module ").trim().to_lowercase();
                            set_known(&mut known, module_path, pkg);
                            break;
                        }
                    }
                }
            }
            "rust" => {
                // A Rust crate is known by its directory name ("logic-gates")
                // AND by its declared `[package]` `name`, which Cargo
                // dependency keys and `package` renames normally spell.
                set_known(&mut known, dir_name.clone(), pkg);
                if let Some(cargo_name) = read_cargo_package_name(pkg) {
                    set_known(&mut known, cargo_name, pkg);
                }
            }
            "wasm" => {
                // WASM wrappers resolve only through their declared Cargo
                // name ("graph-wasm"): the bare directory name ("graph")
                // belongs to the canonical Rust crate the wrapper wraps.
                if let Some(cargo_name) = read_cargo_package_name(pkg) {
                    set_known(&mut known, cargo_name, pkg);
                }
            }
            "elixir" => {
                let app_name = format!("coding_adventures_{}", dir_name.replace('-', "_"));
                set_known(&mut known, app_name, pkg);
            }
            "lua" => {
                // Convert dir name to rockspec name: "logic_gates" -> "coding-adventures-logic-gates"
                // Lua rockspecs use hyphens with a "coding-adventures-" prefix.
                let rock_name = format!("coding-adventures-{}", dir_name.replace('_', "-"));
                set_known(&mut known, rock_name, pkg);
            }
            "perl" => {
                // Perl CPAN dist names use hyphens: "logic-gates" -> "coding-adventures-logic-gates"
                // This matches the Python convention exactly.
                let cpan_name = format!("coding-adventures-{}", dir_name);
                set_known(&mut known, cpan_name, pkg);
            }
            "haskell" => {
                // Haskell Cabal package names use hyphens.
                let cabal_name = format!("coding-adventures-{}", dir_name);
                set_known(&mut known, cabal_name, pkg);
            }
            "csharp" | "fsharp" | "dotnet" => {
                set_known(&mut known, dir_name, pkg);
            }
            _ => {}
        }
    }

    known
}

// ---------------------------------------------------------------------------
// Python dependency parsing
// ---------------------------------------------------------------------------

/// Extracts internal dependencies from a Python pyproject.toml.
///
/// We use simple string scanning rather than a full TOML parser for the
/// dependencies array. This avoids complexity since we only need to extract
/// a single array of strings from the [project] section.
///
/// The parsing strategy:
///  1. Find the "dependencies = [" line
///  2. Collect lines until we hit "]"
///  3. Extract quoted strings and strip version specifiers
fn parse_python_deps(pkg: &Package, known_names: &HashMap<String, String>) -> Vec<String> {
    let pyproject = pkg.path.join("pyproject.toml");
    let data = match fs::read_to_string(&pyproject) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut internal_deps = Vec::new();
    let mut in_deps = false;

    for line in data.lines() {
        let trimmed = line.trim();

        if !in_deps {
            // Look for the start of the dependencies array.
            if trimmed.starts_with("dependencies") && trimmed.contains('=') {
                let after_eq = trimmed.split_once('=').map_or("", |(_, v)| v).trim();

                if after_eq.starts_with('[') {
                    if after_eq.contains(']') {
                        // Single-line array: dependencies = ["foo", "bar"]
                        extract_deps(after_eq, known_names, &mut internal_deps);
                        break;
                    }
                    // Multi-line array starts here.
                    in_deps = true;
                    extract_deps(after_eq, known_names, &mut internal_deps);
                }
            }
            continue;
        }

        // We're inside a multi-line dependencies array.
        if trimmed.contains(']') {
            extract_deps(trimmed, known_names, &mut internal_deps);
            break;
        }
        extract_deps(trimmed, known_names, &mut internal_deps);
    }

    internal_deps
}

/// Finds quoted dependency names in a line and maps them to internal
/// package names. Version specifiers (>=, <, etc.) are stripped.
fn extract_deps(
    line: &str,
    known_names: &HashMap<String, String>,
    deps: &mut Vec<String>,
) {
    // Simple state machine to extract quoted strings.
    // We don't use regex to avoid adding a dependency.
    let mut i = 0;
    let chars: Vec<char> = line.chars().collect();

    while i < chars.len() {
        if chars[i] == '"' || chars[i] == '\'' {
            let quote = chars[i];
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != quote {
                i += 1;
            }
            if i < chars.len() {
                let content: String = chars[start..i].iter().collect();
                // Strip version specifiers: split on >=, <=, >, <, ==, !=, ~=, ;, spaces
                let dep_name = content
                    .split(|c: char| ">=<!~; ".contains(c))
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_lowercase();

                if let Some(pkg_name) = known_names.get(&dep_name) {
                    deps.push(pkg_name.clone());
                }
            }
        }
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Ruby dependency parsing
// ---------------------------------------------------------------------------

/// Extracts internal dependencies from a Ruby .gemspec file.
///
/// Ruby gemspecs declare dependencies with:
///   spec.add_dependency "coding_adventures_logic_gates"
///
/// We scan for these lines and map gem names to internal package names.
fn parse_ruby_deps(pkg: &Package, known_names: &HashMap<String, String>) -> Vec<String> {
    // Find .gemspec files in the package directory.
    let entries = match fs::read_dir(&pkg.path) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut gemspec_path = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".gemspec") {
            gemspec_path = Some(entry.path());
            break;
        }
    }

    let gemspec_path = match gemspec_path {
        Some(p) => p,
        None => return Vec::new(),
    };

    let data = match fs::read_to_string(&gemspec_path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut internal_deps = Vec::new();

    for line in data.lines() {
        // Match: spec.add_dependency "coding_adventures_something"
        if let Some(rest) = line.trim().strip_prefix("spec.add_dependency") {
            // Extract the quoted gem name.
            if let Some(start) = rest.find('"') {
                let after_quote = &rest[start + 1..];
                if let Some(end) = after_quote.find('"') {
                    let gem_name = after_quote[..end].trim().to_lowercase();
                    if let Some(pkg_name) = known_names.get(&gem_name) {
                        internal_deps.push(pkg_name.clone());
                    }
                }
            }
        }
    }

    internal_deps
}

// ---------------------------------------------------------------------------
// Go dependency parsing
// ---------------------------------------------------------------------------

/// Extracts internal dependencies from a Go go.mod file.
///
/// Go modules declare dependencies in go.mod with:
///   require github.com/user/repo/pkg v1.0.0
///
/// or in a block:
///   require (
///       github.com/user/repo/pkg v1.0.0
///   )
///
/// We parse both forms and map module paths to our internal package names.
fn parse_go_deps(pkg: &Package, known_names: &HashMap<String, String>) -> Vec<String> {
    let go_mod = pkg.path.join("go.mod");
    let data = match fs::read_to_string(&go_mod) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut internal_deps = Vec::new();
    let mut in_require_block = false;

    for line in data.lines() {
        let stripped = line.trim();

        if stripped == "require (" {
            in_require_block = true;
            continue;
        }
        if stripped == ")" {
            in_require_block = false;
            continue;
        }

        if in_require_block || stripped.starts_with("require ") {
            // Extract the module path (first whitespace-separated token).
            let clean = stripped.trim_start_matches("require ").trim();
            let module_path = clean
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_lowercase();

            if let Some(pkg_name) = known_names.get(&module_path) {
                internal_deps.push(pkg_name.clone());
            }
        }
    }

    internal_deps
}

// ---------------------------------------------------------------------------
// Rust dependency parsing
// ---------------------------------------------------------------------------

/// Extracts internal dependencies from a Rust Cargo.toml file.
///
/// # Which tables are build-graph edges
///
/// A graph edge means "rebuild and re-test the dependent when this changes",
/// so it must cover everything Cargo compiles INTO the crate:
///
/// ```text
/// table                                  edge?  why
/// -------------------------------------  -----  ------------------------------
/// [dependencies]                         yes    linked into the library
/// [build-dependencies]                   yes    compiled into build.rs, which
///                                               generates or configures it
/// [target.<cfg>.dependencies]            yes    linked on that target; CI
/// [target.<cfg>.build-dependencies]      yes    builds on several hosts, so
///                                               the union over every <cfg>
/// [dev-dependencies] (every form)        no     tests only; Cargo allows dev
///                                               cycles, a graph must not
/// [patch], [replace], [workspace], ...   no     not a dependency of the crate
/// ```
///
/// `build_dependencies` is Cargo's legacy spelling of `build-dependencies`.
///
/// # Which entry shapes
///
/// Both TOML spellings of a path dependency are read:
///
/// ```text
/// [dependencies]
/// logic-gates = { path = "../logic-gates" }          <- inline table
/// gates = { package = "logic-gates", path = "..." }  <- renamed: "logic-gates"
///
/// [dependencies.logic-gates]                         <- sub-table
/// path = "../logic-gates"
/// ```
///
/// The crate name (the `package` rename if present, else the key) is then
/// looked up in the known names, and each internal package is returned once.
///
/// # Why not the `toml` crate?
///
/// This tool does parse TOML elsewhere (the `[package]` `name`), but the
/// dependency edges must be the SAME edges every build-tool front computes,
/// and most fronts have no TOML library. So the cross-implementation contract
/// in `code/specs/build-tool-conformance.md` is a line procedure, and a full
/// parser would disagree with it at the edges: it would read the out-of-scope
/// dotted key `a.path = "../a"` as a path dependency, and it would drop every
/// edge of a manifest with one malformed line. [`cargo_link_dependency_names`]
/// implements that procedure; the shared fixture
/// `resolution-rust-field-aware.json` pins it.
fn parse_rust_deps(pkg: &Package, known_names: &HashMap<String, String>) -> Vec<String> {
    let cargo_toml = pkg.path.join("Cargo.toml");
    let data = match fs::read_to_string(&cargo_toml) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut internal_deps: Vec<String> = Vec::new();
    for crate_name in cargo_link_dependency_names(&data) {
        if let Some(pkg_name) = known_names.get(&crate_name) {
            // A crate may appear in several link tables at once (say in
            // `[dependencies]` and `[build-dependencies]`): one edge.
            if !internal_deps.contains(pkg_name) {
                internal_deps.push(pkg_name.clone());
            }
        }
    }

    internal_deps
}

/// The table names whose path entries link into, or build, the crate. They
/// are compared exactly: TOML keys are case-sensitive, so `[Dependencies]`
/// is some other table.
const CARGO_LINK_TABLE_NAMES: [&str; 3] =
    ["dependencies", "build-dependencies", "build_dependencies"];

/// How the lines under the current header are read.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CargoLinkTable {
    /// Not a link table: its lines are skipped.
    Ignored,
    /// `[D]` or `[target.C.D]`: one `name = { ... }` entry per line.
    Inline,
    /// `[D.N]` or `[target.C.D.N]`: the keys of the one entry `N`.
    Sub(String),
}

/// Returns the ASCII-lowercased crate names of every path dependency in a
/// manifest's link tables, in manifest order, duplicates included (the
/// caller collapses them).
///
/// It is one pass over the lines with a three-state machine
/// ([`CargoLinkTable`]). Every header line — even a malformed one, or an
/// array of tables — ends the current table, and so does the end of the
/// file; a sub-table's entry is complete only then, because its `path` and
/// `package` keys may come in any order.
///
/// One inline entry may cover several lines. TOML 1.0 lets an ARRAY inside
/// an inline table span lines, and the repository writes exactly that:
///
/// ```text
/// coding_adventures_vault_pm_cli = { path = "../vault-pm-cli", features = [
///     "crash-injection",
/// ] }
/// ```
///
/// So while the entry's value still has an unclosed `{` or `[`, the next
/// lines are joined onto it with one space (and are not looked at as headers
/// or entries). If the file ends first, the entry contributes nothing.
fn cargo_link_dependency_names(text: &str) -> Vec<String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    // Step 1 of the contract: split at LF, then drop ONE trailing CR.
    let lines: Vec<&str> = text.split('\n').collect();
    let clean_line = |raw: &str| -> String {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        trim_toml_blank(strip_toml_line_comment(raw)).to_string()
    };

    let mut names = Vec::new();
    let mut table = CargoLinkTable::Ignored;
    let mut sub_path: Option<String> = None;
    let mut sub_package: Option<String> = None;

    // A sub-table is a candidate only once it ends, when `path` was set; it
    // is named by its `package` rename, else by its key `N`.
    let finish_table = |table: &CargoLinkTable,
                        sub_path: &Option<String>,
                        sub_package: &Option<String>,
                        names: &mut Vec<String>| {
        if let (CargoLinkTable::Sub(key), Some(_)) = (table, sub_path) {
            let name = sub_package.as_deref().unwrap_or(key);
            names.push(name.to_ascii_lowercase());
        }
    };

    let mut index = 0;
    while index < lines.len() {
        let line = clean_line(lines[index]);
        index += 1;

        if line.starts_with('[') && line.ends_with(']') {
            finish_table(&table, &sub_path, &sub_package, &mut names);
            table = classify_cargo_link_header(&line);
            sub_path = None;
            sub_package = None;
            continue;
        }

        let Some(equals) = index_toml_outside_strings(&line, b'=') else {
            continue;
        };
        match &table {
            CargoLinkTable::Ignored => {}
            CargoLinkTable::Inline => {
                let mut value = trim_toml_blank(&line[equals + 1..]).to_string();
                let mut complete = true;
                while value.starts_with('{') && toml_open_bracket_depth(&value) > 0 {
                    if index >= lines.len() {
                        complete = false; // the file ended inside the value
                        break;
                    }
                    value.push(' ');
                    value.push_str(&clean_line(lines[index]));
                    index += 1;
                }
                if complete {
                    if let Some(name) = cargo_path_dependency_name(&line[..equals], &value) {
                        names.push(name);
                    }
                }
            }
            CargoLinkTable::Sub(_) => {
                // Only the two bare, lowercase keys matter; the first
                // occurrence of each wins. `version`, `features`, `optional`
                // ... are ignored.
                let Some(value) = toml_single_string(&line[equals + 1..]) else {
                    continue;
                };
                let slot = match trim_toml_blank(&line[..equals]) {
                    "path" => &mut sub_path,
                    "package" => &mut sub_package,
                    _ => continue,
                };
                if slot.is_none() {
                    *slot = Some(value.to_string());
                }
            }
        }
    }
    finish_table(&table, &sub_path, &sub_package, &mut names);
    names
}

/// Decides how to read the lines under a header line (already
/// comment-stripped and trimmed, starting with `[` and ending with `]`).
///
/// The header is split into dotted-key segments at every `.` OUTSIDE a
/// string, and each segment is decoded to its value. That is what makes a
/// cfg header one segment however much punctuation it quotes:
///
/// ```text
/// [target.'cfg(any(target_os = "linux", windows))'.dependencies]
///  └─┬──┘ └──────────────────┬───────────────────┘ └────┬─────┘
///  target            C (one quoted segment)               D
///
/// [target."thumbv8m.main-none-eabihf".dependencies.pi]  -> sub-table "pi"
/// [target.x86_64-unknown-linux-gnu.build-dependencies]  -> bare triple
/// ["dependencies"]                                      -> same as [dependencies]
/// ```
///
/// Segment lists that are not exactly one of `[D]`, `[D.N]`, `[target.C.D]`
/// or `[target.C.D.N]` — and malformed headers — are ignored tables.
fn classify_cargo_link_header(line: &str) -> CargoLinkTable {
    if line.starts_with("[[") {
        return CargoLinkTable::Ignored; // array of tables, e.g. [[bin]]
    }
    let mut segments = Vec::new();
    for raw in split_toml_outside_strings(&line[1..line.len() - 1], b'.', false) {
        match toml_key_segment(raw) {
            Some(segment) => segments.push(segment),
            None => return CargoLinkTable::Ignored,
        }
    }
    let mut segments = segments.as_slice();
    if segments.len() >= 3 && segments[0] == "target" {
        segments = &segments[2..]; // drop `target` and the cfg, whatever it is
    }
    match segments {
        [table] if CARGO_LINK_TABLE_NAMES.contains(table) => CargoLinkTable::Inline,
        [table, key] if CARGO_LINK_TABLE_NAMES.contains(table) => {
            CargoLinkTable::Sub(key.to_string())
        }
        _ => CargoLinkTable::Ignored,
    }
}

/// Returns the crate a completed inline-table entry `key = value` names when
/// it is a path dependency — the only kind that can point at a repository
/// package. The key is the crate name unless the inline table renames it
/// with `package = "..."`:
///
/// ```text
/// logic-gates = { path = "../logic-gates" }                    -> logic-gates
/// gates = { package = "logic-gates", path = "../logic-gates" } -> logic-gates
/// serde = "1"                                                  -> not a path dep
/// x = { version = "1", features = ["path"] }                   -> not a path dep
/// foo.path = "../foo"                                          -> dotted key: no
/// ```
///
/// Fields are split at commas outside strings AND outside nested `[...]` or
/// `{...}`, so a `features` array cannot leak a fake field and a nested
/// table's `path` is not the entry's. Field keys must be exactly `path` and
/// `package` (Cargo ignores `Package`, so it must not count as a rename).
fn cargo_path_dependency_name(key: &str, value: &str) -> Option<String> {
    let crate_name = toml_key_segment(key)?;
    let value = trim_toml_blank(value);
    if value.len() < 2 || !value.starts_with('{') || !value.ends_with('}') {
        return None;
    }
    let mut path: Option<&str> = None;
    let mut package_name: Option<&str> = None;
    for field in split_toml_outside_strings(&value[1..value.len() - 1], b',', true) {
        let Some(equals) = index_toml_outside_strings(field, b'=') else {
            continue;
        };
        let Some(field_value) = toml_single_string(&field[equals + 1..]) else {
            continue;
        };
        let slot = match trim_toml_blank(&field[..equals]) {
            "path" => &mut path,
            "package" => &mut package_name,
            _ => continue,
        };
        if slot.is_none() {
            *slot = Some(field_value);
        }
    }
    path?;
    Some(package_name.unwrap_or(crate_name).to_ascii_lowercase())
}

/// Trims spaces and tabs only — the contract's notion of blank. (`str::trim`
/// would also strip other Unicode whitespace, which TOML does not allow.)
fn trim_toml_blank(s: &str) -> &str {
    s.trim_matches(|character| character == ' ' || character == '\t')
}

/// Returns the index just past the string that opens at `s[start]` (a `"`
/// or `'`), or `None` if it is not closed on this line. In a basic `"..."`
/// string a backslash makes the next byte part of the string, so `\"` does
/// not end it; a literal `'...'` string has no escapes.
///
/// Every byte compared here is ASCII, and in UTF-8 an ASCII byte is never
/// part of a multi-byte character, so scanning bytes is exact and every
/// index returned is a character boundary that is safe to slice at.
fn toml_string_end(s: &[u8], start: usize) -> Option<usize> {
    let quote = s[start];
    let mut index = start + 1;
    while index < s.len() {
        if quote == b'"' && s[index] == b'\\' {
            index += 1;
        } else if s[index] == quote {
            return Some(index + 1);
        }
        index += 1;
    }
    None
}

/// Removes a TOML comment: everything from the first `#` outside a string.
/// An unclosed string runs to the end of the line.
fn strip_toml_line_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'"' | b'\'' => match toml_string_end(bytes, index) {
                Some(end) => index = end,
                None => return line,
            },
            b'#' => return &line[..index],
            _ => index += 1,
        }
    }
    line
}

/// Returns the index of the first `target` byte that is outside every
/// string, or `None` (also when a string is left unclosed before it).
fn index_toml_outside_strings(s: &str, target: u8) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'"' | b'\'' => index = toml_string_end(bytes, index)?,
            byte if byte == target => return Some(index),
            _ => index += 1,
        }
    }
    None
}

/// Counts the `{` and `[` outside strings in `s` that are not closed again
/// within `s`. Zero (or less) means the value is complete; an unclosed
/// string stops the count where it opens.
fn toml_open_bracket_depth(s: &str) -> i64 {
    let bytes = s.as_bytes();
    let mut depth = 0;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'"' | b'\'' => match toml_string_end(bytes, index) {
                Some(end) => {
                    index = end;
                    continue;
                }
                None => return depth,
            },
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth -= 1,
            _ => {}
        }
        index += 1;
    }
    depth
}

/// Splits `s` at every `separator` outside strings. With `nested` set, a
/// separator inside `[...]` or `{...}` does not split either. An unclosed
/// string makes the rest of `s` the last part.
fn split_toml_outside_strings(s: &str, separator: u8, nested: bool) -> Vec<&str> {
    let bytes = s.as_bytes();
    let mut parts = Vec::new();
    let (mut start, mut depth, mut index) = (0, 0i64, 0);
    while index < bytes.len() {
        match bytes[index] {
            b'"' | b'\'' => match toml_string_end(bytes, index) {
                Some(end) => {
                    index = end;
                    continue;
                }
                None => {
                    parts.push(&s[start..]);
                    return parts;
                }
            },
            b'[' | b'{' if nested => depth += 1,
            b']' | b'}' if nested => depth -= 1,
            byte if byte == separator && depth == 0 => {
                parts.push(&s[start..index]);
                start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }
    parts.push(&s[start..]);
    parts
}

/// Decodes one key or header segment: a bare key (ASCII letters, digits,
/// `_`, `-`), or exactly one complete string, whose value is the text
/// between its quotes taken verbatim (escapes are not decoded). Anything
/// else — empty, dotted, or text around a string — is not a key.
fn toml_key_segment(raw: &str) -> Option<&str> {
    let segment = trim_toml_blank(raw);
    let bytes = segment.as_bytes();
    match bytes.first()? {
        b'"' | b'\'' => {
            if toml_string_end(bytes, 0)? != bytes.len() {
                return None;
            }
            Some(&segment[1..segment.len() - 1])
        }
        _ => bytes
            .iter()
            .all(|&byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
            .then_some(segment),
    }
}

/// Returns the content of a value that is exactly one complete, non-empty
/// string (after trimming), as `path` and `package` require. `"1"` is one;
/// `"a" "b"`, `1`, `""` and `["x"]` are not.
fn toml_single_string(raw: &str) -> Option<&str> {
    let value = trim_toml_blank(raw);
    let bytes = value.as_bytes();
    if !matches!(bytes.first(), Some(b'"' | b'\'')) || toml_string_end(bytes, 0)? != bytes.len() {
        return None;
    }
    let content = &value[1..value.len() - 1];
    (!content.is_empty()).then_some(content)
}

fn parse_dotnet_deps(pkg: &Package, known_names: &HashMap<String, String>) -> Vec<String> {
    let entries = match fs::read_dir(&pkg.path) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut internal_deps = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".csproj") && !name.ends_with(".fsproj") {
            continue;
        }

        let data = match fs::read_to_string(entry.path()) {
            Ok(s) => s,
            Err(_) => continue,
        };

        for line in data.lines() {
            let Some(include_pos) = line.find("<ProjectReference") else {
                continue;
            };
            let line = &line[include_pos..];
            let Some(attr_pos) = line.find("Include=\"") else {
                continue;
            };
            let include = &line[attr_pos + "Include=\"".len()..];
            let Some(end_quote) = include.find('"') else {
                continue;
            };
            let project_path = &include[..end_quote];
            let normalized = project_path.replace('\\', "/");
            let Some(remainder) = normalized.strip_prefix("../") else {
                continue;
            };
            let Some(dep_dir) = remainder.split('/').next() else {
                continue;
            };
            let dep_dir = dep_dir.to_lowercase();
            if dep_dir.contains('/') || dep_dir.contains('\\') || dep_dir == ".." {
                continue;
            }
            if let Some(pkg_name) = known_names.get(&dep_dir) {
                internal_deps.push(pkg_name.clone());
            }
        }
    }

    internal_deps
}

// ---------------------------------------------------------------------------
// Elixir dependency parsing
// ---------------------------------------------------------------------------

fn parse_elixir_deps(pkg: &Package, known_names: &HashMap<String, String>) -> Vec<String> {
    let mix_exs = pkg.path.join("mix.exs");
    let data = match fs::read_to_string(&mix_exs) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut internal_deps = Vec::new();
    for line in data.lines() {
        if let Some(idx) = line.find("{:coding_adventures_") {
            let start = idx + 2; // skip {:
            let rest = &line[start..];
            let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());
            let app_name = rest[..end].to_lowercase();
            if let Some(pkg_name) = known_names.get(&app_name) {
                internal_deps.push(pkg_name.clone());
            }
        }
    }
    internal_deps
}

// ---------------------------------------------------------------------------
// Lua dependency parsing
// ---------------------------------------------------------------------------

/// Extracts internal dependencies from a Lua .rockspec file.
///
/// Lua rockspecs declare dependencies inside a `dependencies` table:
///
///   dependencies = {
///       "lua >= 5.4",
///       "coding-adventures-logic-gates >= 0.1.0",
///   }
///
/// We scan for quoted strings inside the `dependencies = { ... }` block,
/// strip version specifiers (>=, <=, ==, etc.), and look them up in
/// known_names. Only internal monorepo dependencies are returned.
fn parse_lua_deps(
    pkg: &Package,
    known_names: &HashMap<String, String>,
) -> Result<Vec<String>, MetadataEncodingError> {
    // Find .rockspec files in the package directory.
    let entries = match fs::read_dir(&pkg.path) {
        Ok(e) => e,
        Err(_) => return Ok(Vec::new()),
    };

    let mut rockspec_path = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".rockspec") {
            rockspec_path = Some(entry.path());
            break;
        }
    }

    let rockspec_path = match rockspec_path {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };

    let raw = match fs::read(&rockspec_path) {
        Ok(data) => data,
        Err(_) => return Ok(Vec::new()),
    };
    let data = match String::from_utf8(raw) {
        Ok(text) => text,
        Err(_) => {
            return Err(MetadataEncodingError {
                code: METADATA_INVALID_UTF8.to_string(),
                package: pkg.name.clone(),
                manifest: repository_manifest_path(&rockspec_path),
                encoding: "UTF-8".to_string(),
            });
        }
    };

    let mut internal_deps = Vec::new();
    let mut in_deps = false;

    for line in data.lines() {
        let trimmed = line.trim();

        if !in_deps {
            // Look for the start of the dependencies block.
            if trimmed.starts_with("dependencies") && trimmed.contains('{') {
                in_deps = true;
                // Extract any deps on the same line as the opening brace.
                extract_lua_dep(trimmed, known_names, &mut internal_deps);
                if trimmed.contains('}') {
                    break; // Single-line block.
                }
                continue;
            }
            continue;
        }

        // We're inside the dependencies block.
        if trimmed.contains('}') {
            extract_lua_dep(trimmed, known_names, &mut internal_deps);
            break;
        }
        extract_lua_dep(trimmed, known_names, &mut internal_deps);
    }

    Ok(internal_deps)
}

/// Extracts a single dependency name from a quoted string in a Lua
/// rockspec line. Version specifiers are stripped by splitting on
/// whitespace and taking only the first token (the package name).
fn extract_lua_dep(
    line: &str,
    known_names: &HashMap<String, String>,
    deps: &mut Vec<String>,
) {
    // Find quoted strings and extract the dependency name.
    let mut i = 0;
    let chars: Vec<char> = line.chars().collect();

    while i < chars.len() {
        if chars[i] == '"' || chars[i] == '\'' {
            let quote = chars[i];
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != quote {
                i += 1;
            }
            if i < chars.len() {
                let content: String = chars[start..i].iter().collect();
                // Strip version specifiers: take only the package name
                // (everything before the first space or version operator).
                let dep_name = content
                    .split(|c: char| ">=<!~ ".contains(c))
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_lowercase();

                if let Some(pkg_name) = known_names.get(&dep_name) {
                    deps.push(pkg_name.clone());
                }
            }
        }
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Perl dependency parsing
// ---------------------------------------------------------------------------

/// Extracts internal dependencies from a Perl cpanfile.
///
/// A cpanfile declares dependencies with one `requires` per line:
///
///     requires 'coding-adventures-logic-gates';
///     requires 'coding-adventures-bitset', '>= 0.01';
///
/// We scan for lines containing `requires` followed by a quoted string
/// starting with `coding-adventures-` and map them to internal package
/// names. External deps are silently skipped.
fn parse_perl_deps(pkg: &Package, known_names: &HashMap<String, String>) -> Vec<String> {
    let cpanfile = pkg.path.join("cpanfile");
    let data = match fs::read_to_string(&cpanfile) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut internal_deps = Vec::new();
    let prefix = "coding-adventures-";

    for line in data.lines() {
        let trimmed = line.trim();

        // Skip blank lines and comments.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Look for: requires 'coding-adventures-...' or requires "coding-adventures-..."
        if !trimmed.contains("requires") {
            continue;
        }

        // Find the quoted dependency name after "requires".
        // Try both single and double quotes.
        for quote in &['\'', '"'] {
            if let Some(start) = trimmed.find(&format!("{}{}", quote, prefix)) {
                let after_quote = start + 1; // skip the opening quote
                if let Some(end) = trimmed[after_quote..].find(*quote) {
                    let dep_name = trimmed[after_quote..after_quote + end].to_lowercase();
                    if let Some(pkg_name) = known_names.get(&dep_name) {
                        internal_deps.push(pkg_name.clone());
                    }
                }
                break;
            }
        }
    }

    internal_deps
}

// ---------------------------------------------------------------------------
// Haskell dependency parsing
// ---------------------------------------------------------------------------

/// Extracts internal dependencies from a Haskell Cabal file.
fn parse_haskell_deps(pkg: &Package, known_names: &HashMap<String, String>) -> Vec<String> {
    let entries = match fs::read_dir(&pkg.path) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut cabal_path = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".cabal") {
            cabal_path = Some(entry.path());
            break;
        }
    }

    let cabal_path = match cabal_path {
        Some(p) => p,
        None => return Vec::new(),
    };

    let data = match fs::read_to_string(&cabal_path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut internal_deps = Vec::new();
    let prefix = "coding-adventures-";

    for line in data.lines() {
        let mut i = 0;
        let chars: Vec<char> = line.chars().collect();
        while i < chars.len() {
            if chars[i..].starts_with(&prefix.chars().collect::<Vec<_>>()) {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '-') {
                    i += 1;
                }
                let dep_name: String = chars[start..i].iter().collect();
                if let Some(pkg_name) = known_names.get(&dep_name.to_lowercase()) {
                    if pkg_name == &pkg.name {
                        continue;
                    }
                    internal_deps.push(pkg_name.clone());
                }
            } else {
                i += 1;
            }
        }
    }

    internal_deps
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parses package metadata to discover dependencies and builds a directed graph.
///
/// The graph contains all discovered packages as nodes. Edges represent
/// build ordering: an edge from A to B means "A must be built before B"
/// (because B depends on A). External dependencies — those not found
/// among the discovered packages — are silently skipped.
///
/// This function is the main entry point for dependency resolution.
pub fn resolve_dependencies(packages: &[Package]) -> Result<Graph, ResolutionError> {
    let mut graph = Graph::new();

    // First, add all packages as nodes. Even packages with no dependencies
    // need to be in the graph so they appear in independent_groups().
    for pkg in packages {
        graph.add_node(&pkg.name);
    }

    // Build the ecosystem-specific name mapping table.
    let mut known_names_by_scope: HashMap<String, HashMap<String, String>> = HashMap::new();
    for pkg in packages {
        let scope = dependency_scope(&pkg.language).to_string();
        known_names_by_scope
            .entry(scope.clone())
            .or_insert_with(|| build_known_names_for_scope(packages, &scope));
    }

    // Parse dependencies for each package and add edges.
    for pkg in packages {
        let known_names = known_names_by_scope
            .get(dependency_scope(&pkg.language))
            .expect("known name scope must exist");
        let deps = match pkg.language.as_str() {
            "python" => parse_python_deps(pkg, known_names),
            "ruby" => parse_ruby_deps(pkg, known_names),
            "go" => parse_go_deps(pkg, known_names),
            "rust" | "wasm" => parse_rust_deps(pkg, known_names),
            "elixir" => parse_elixir_deps(pkg, known_names),
            "lua" => parse_lua_deps(pkg, known_names).map_err(ResolutionError::from)?,
            "perl" => parse_perl_deps(pkg, known_names),
            "haskell" => parse_haskell_deps(pkg, known_names),
            "csharp" | "fsharp" | "dotnet" => parse_dotnet_deps(pkg, known_names),
            _ => Vec::new(),
        };

        for dep_name in deps {
            // Edge direction: dep -> pkg means "dep must be built before pkg".
            // This convention makes independent_groups() produce the correct
            // build order: nodes with zero in-degree (no deps) come first.
            if dep_name == pkg.name {
                return Err(ResolutionError::DependencySelfEdge(
                    DependencySelfEdgeError {
                        code: DEPENDENCY_SELF_EDGE.to_string(),
                        package: pkg.name.clone(),
                        manifest: dependency_manifest_path(pkg),
                        dependency: dep_name,
                    },
                ));
            }
            graph.add_edge(&dep_name, &pkg.name);
        }
    }

    Ok(graph)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::path::{Path, PathBuf};

    #[derive(Debug, Deserialize)]
    struct ResolutionFixture {
        workspace: FixtureWorkspace,
        expected: FixtureExpected,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureWorkspace {
        files: Vec<FixtureFile>,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureFile {
        path: String,
        #[serde(default)]
        content_utf8: String,
        #[serde(default)]
        content_base64: String,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureExpected {
        outcome: String,
        #[serde(default)]
        result: FixtureResult,
        #[serde(default)]
        diagnostics: Vec<FixtureDiagnostic>,
    }

    #[derive(Debug, Default, Deserialize)]
    struct FixtureResult {
        #[serde(default)]
        edges: Vec<Vec<String>>,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureDiagnostic {
        code: String,
        path: String,
        package: String,
        details: FixtureDiagnosticDetails,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureDiagnosticDetails {
        #[serde(default)]
        encoding: Option<String>,
        #[serde(default)]
        dependency: Option<String>,
    }

    fn decode_base64(input: &str) -> Vec<u8> {
        let mut output = Vec::new();
        let mut accumulator = 0u32;
        let mut bits = 0u8;

        for byte in input.bytes() {
            let value = match byte {
                b'A'..=b'Z' => byte - b'A',
                b'a'..=b'z' => byte - b'a' + 26,
                b'0'..=b'9' => byte - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                b'=' => break,
                b'\r' | b'\n' | b'\t' | b' ' => continue,
                _ => panic!("invalid base64 fixture byte: {byte}"),
            };
            accumulator = (accumulator << 6) | u32::from(value);
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                output.push((accumulator >> bits) as u8);
                accumulator &= (1 << bits) - 1;
            }
        }

        output
    }

    fn load_resolution_fixture(name: &str) -> ResolutionFixture {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../specs/fixtures/build-tool-v1/cases")
            .join(name);
        let data = fs::read(&path)
            .unwrap_or_else(|error| panic!("read shared fixture {}: {error}", path.display()));
        serde_json::from_slice(&data)
            .unwrap_or_else(|error| panic!("decode shared fixture {}: {error}", path.display()))
    }

    fn materialize_resolution_fixture(
        fixture: &ResolutionFixture,
        case_name: &str,
    ) -> (PathBuf, Vec<Package>) {
        let root = std::env::temp_dir().join(format!(
            "build_tool_rust_resolution_{case_name}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        for file in &fixture.workspace.files {
            let path = root.join(Path::new(&file.path));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let data = if file.content_base64.is_empty() {
                file.content_utf8.as_bytes().to_vec()
            } else {
                decode_base64(&file.content_base64)
            };
            fs::write(&path, data).unwrap();
        }

        let packages = crate::discovery::discover_packages(&root.join("code"))
            .expect("resolution fixture identities must be unique");
        (root, packages)
    }

    #[test]
    fn test_lua_resolution_conformance_fixtures() {
        for name in [
            "resolution-lua-utf8.json",
            "resolution-lua-invalid-utf8.json",
        ] {
            let fixture = load_resolution_fixture(name);
            let (root, packages) = materialize_resolution_fixture(&fixture, name);
            let result = resolve_dependencies(&packages);

            if fixture.expected.outcome == "ok" {
                let graph = result.expect("valid UTF-8 fixture must resolve");
                let mut actual_edges = Vec::new();
                for package in &packages {
                    for dependency in graph.predecessors(&package.name).unwrap() {
                        actual_edges.push(vec![dependency, package.name.clone()]);
                    }
                }
                actual_edges.sort();
                let mut expected_edges = fixture.expected.result.edges.clone();
                expected_edges.sort();
                assert_eq!(actual_edges, expected_edges);
            } else {
                let error = match result {
                    Ok(_) => panic!("invalid UTF-8 fixture must fail closed"),
                    Err(error) => error,
                };
                let encoding_error = match error {
                    ResolutionError::MetadataEncoding(error) => error,
                    other => panic!("expected metadata encoding error, got {other}"),
                };
                let diagnostic = &fixture.expected.diagnostics[0];
                assert_eq!(encoding_error.code, diagnostic.code);
                assert_eq!(encoding_error.package, diagnostic.package);
                assert_eq!(encoding_error.manifest, diagnostic.path);
                assert_eq!(
                    Some(encoding_error.encoding.as_str()),
                    diagnostic.details.encoding.as_deref()
                );
                assert!(!encoding_error
                    .to_string()
                    .contains(&root.to_string_lossy().to_string()));
            }

            let _ = fs::remove_dir_all(root);
        }
    }

    #[test]
    fn test_elixir_resolution_conformance_fixtures() {
        for name in [
            "resolution-elixir-program-package.json",
            "resolution-elixir-self-edge.json",
        ] {
            let fixture = load_resolution_fixture(name);
            let (root, packages) = materialize_resolution_fixture(&fixture, name);
            let result = resolve_dependencies(&packages);

            if fixture.expected.outcome == "ok" {
                let graph = result.expect("distinct package and program identities must resolve");
                let mut actual_edges = Vec::new();
                for package in &packages {
                    for dependency in graph.predecessors(&package.name).unwrap() {
                        actual_edges.push(vec![dependency, package.name.clone()]);
                    }
                }
                actual_edges.sort();
                let mut expected_edges = fixture.expected.result.edges.clone();
                expected_edges.sort();
                assert_eq!(actual_edges, expected_edges);
            } else {
                let error = match result {
                    Ok(_) => panic!("dependency self-edge fixture must fail closed"),
                    Err(error) => error,
                };
                let diagnostic = &fixture.expected.diagnostics[0];
                let expected = format!(
                    "{}: package={} manifest={} dependency={}",
                    diagnostic.code,
                    diagnostic.package,
                    diagnostic.path,
                    diagnostic.details.dependency.as_deref().unwrap()
                );
                assert_eq!(error.to_string(), expected);
                assert!(!error
                    .to_string()
                    .contains(&root.to_string_lossy().to_string()));
            }

            let _ = fs::remove_dir_all(root);
        }
    }

    /// Runs the shared case every build-tool front implements: inline and
    /// sub-table entries in `[dependencies]`, `[build-dependencies]` and
    /// their target forms are edges; dev-dependencies (every form),
    /// `[patch]`, registry entries, features and comments are not.
    #[test]
    fn test_rust_resolution_conformance_fixture() {
        let name = "resolution-rust-field-aware.json";
        let fixture = load_resolution_fixture(name);
        assert_eq!(fixture.expected.outcome, "ok");
        let (root, packages) = materialize_resolution_fixture(&fixture, name);
        let graph = resolve_dependencies(&packages).expect("rust fixture must resolve");

        let mut actual_edges = Vec::new();
        for package in &packages {
            for dependency in graph.predecessors(&package.name).unwrap() {
                actual_edges.push(vec![dependency, package.name.clone()]);
            }
        }
        actual_edges.sort();
        let mut expected_edges = fixture.expected.result.edges.clone();
        expected_edges.sort();
        assert_eq!(actual_edges, expected_edges);

        let _ = fs::remove_dir_all(root);
    }

    /// Pins each line rule of the Cargo contract in
    /// `code/specs/build-tool-conformance.md`, one manifest shape per case.
    /// The table is the Go reference's `TestCargoLinkDependencyNames`,
    /// case for case, so the two readers are held to the same shapes.
    #[test]
    fn test_cargo_link_dependency_names() {
        let cases: &[(&str, &str, &[&str])] = &[
            (
                "top-level inline",
                "[dependencies]\na = { path = \"../a\" }\n",
                &["a"],
            ),
            (
                "build-dependencies",
                "[build-dependencies]\na = { path = \"../a\" }\n",
                &["a"],
            ),
            (
                "legacy build_dependencies",
                "[build_dependencies]\na = { path = \"../a\" }\n",
                &["a"],
            ),
            (
                "quoted cfg target",
                "[target.'cfg(any(target_os = \"linux\", windows))'.dependencies]\na = { path = \"../a\" }\n",
                &["a"],
            ),
            (
                "bare triple target build",
                "[target.x86_64-pc-windows-msvc.build-dependencies]\na = { path = \"../a\" }\n",
                &["a"],
            ),
            (
                "legacy target build",
                "[target.'cfg(unix)'.build_dependencies]\na = { path = \"../a\" }\n",
                &["a"],
            ),
            (
                "spaced header segments",
                "[ target . 'cfg(unix)' . dependencies ]\na = { path = \"../a\" }\n",
                &["a"],
            ),
            (
                "quoted table segment",
                "[\"dependencies\"]\na = { path = \"../a\" }\n",
                &["a"],
            ),
            (
                "sub-table",
                "[dependencies.a]\nversion = \"1\"\npath = \"../a\"\n",
                &["a"],
            ),
            (
                "sub-table rename before path",
                "[build-dependencies.x]\npackage = \"A\"\npath = '../a'\n",
                &["a"],
            ),
            (
                "target sub-table, dotted quoted triple",
                "[target.\"thumbv8m.main-none-eabihf\".dependencies.a]\npath = \"../a\"\n",
                &["a"],
            ),
            (
                "quoted sub-table key",
                "[dependencies.\"a\"]\npath = \"../a\"\n",
                &["a"],
            ),
            (
                "sub-table without path",
                "[dependencies.a]\nversion = \"1\"\n",
                &[],
            ),
            (
                "sub-table first path wins, ended by header",
                "[dependencies.a]\npath = \"../a\"\npath = \"../b\"\n[package]\nname = \"z\"\n",
                &["a"],
            ),
            (
                "sub-table non-string path",
                "[dependencies.a]\npath = 1\n",
                &[],
            ),
            (
                "sub-table empty path",
                "[dependencies.a]\npath = \"\"\n",
                &[],
            ),
            (
                "sub-table capitalised Path key",
                "[dependencies.a]\nPath = \"../a\"\n",
                &[],
            ),
            (
                "rename",
                "[dependencies]\nx = { package = \"Logic-Gates\", path = \"../g\" }\n",
                &["logic-gates"],
            ),
            (
                "capitalised Package is not a rename",
                "[dependencies]\nx = { Package = \"y\", path = \"../g\" }\n",
                &["x"],
            ),
            (
                "capitalised Path is not a path",
                "[dependencies]\nx = { Path = \"../g\" }\n",
                &[],
            ),
            (
                "version plus path with trailing comment",
                "[dependencies]\na = { version = \"1\", path = \"../a\" } # b = { path = \"../b\" }\n",
                &["a"],
            ),
            (
                "hash inside a string is not a comment",
                "[dependencies]\na = { path = \"../a#1\" }\n",
                &["a"],
            ),
            (
                "quoted entry key",
                "[dependencies]\n\"a\" = { path = \"../a\" }\n",
                &["a"],
            ),
            (
                "registry entry with path feature",
                "[dependencies]\na = { version = \"1\", features = [\"path\", \"x\"] }\nb = \"1\"\n",
                &[],
            ),
            (
                "nested table path is not the entry's",
                "[dependencies]\na = { x = { path = \"../a\" } }\n",
                &[],
            ),
            (
                "features array commas do not split fields",
                "[dependencies]\na = { features = [\"p\", \"q\"], path = \"../a\" }\n",
                &["a"],
            ),
            (
                "empty path",
                "[dependencies]\na = { path = \"\" }\n",
                &[],
            ),
            (
                "array continued onto later lines",
                "[dependencies]\na = { path = \"../a\", features = [\n    \"x\", # [not-a-header]\n] }\nb = { path = \"../b\" }\n",
                &["a", "b"],
            ),
            (
                "continuation lines are not headers",
                "[dependencies]\na = { path = \"../a\", features = [\n[\"x\"]\n] }\nb = { path = \"../b\" }\n",
                &["a", "b"],
            ),
            (
                "braces continued onto later lines",
                "[dependencies]\na = { version = \"1\",\n  path = \"../a\" }\n",
                &["a"],
            ),
            (
                "file ends inside the value",
                "[dependencies]\na = { path = \"../a\", features = [\n  \"x\",\n",
                &[],
            ),
            (
                "continued value must end with a brace",
                "[dependencies]\na = { path = \"../a\", features = [\n] } x\n",
                &[],
            ),
            (
                "dotted key is out of scope",
                "[dependencies]\na.path = \"../a\"\n",
                &[],
            ),
            (
                "commented-out entry",
                "[dependencies]\n# a = { path = \"../a\" }\n",
                &[],
            ),
            (
                "dev-dependencies",
                "[dev-dependencies]\na = { path = \"../a\" }\n[dev_dependencies]\nb = { path = \"../b\" }\n",
                &[],
            ),
            (
                "target dev-dependencies",
                "[target.'cfg(unix)'.dev-dependencies]\na = { path = \"../a\" }\n",
                &[],
            ),
            (
                "dev-dependencies sub-table",
                "[dev-dependencies.a]\npath = \"../a\"\n",
                &[],
            ),
            (
                "patch, replace, workspace",
                "[patch.crates-io]\na = { path = \"../a\" }\n[workspace.dependencies]\nb = { path = \"../b\" }\n[replace]\n\"c:1.0.0\" = { path = \"../c\" }\n",
                &[],
            ),
            (
                "capitalised table name",
                "[Dependencies]\na = { path = \"../a\" }\n",
                &[],
            ),
            (
                "features sub-table of a dependency",
                "[dependencies.a.features]\npath = \"../a\"\n",
                &[],
            ),
            (
                "target table itself",
                "[target.'cfg(unix)']\na = { path = \"../a\" }\n",
                &[],
            ),
            (
                "array of tables ends the table",
                "[dependencies]\n[[bin]]\na = { path = \"../a\" }\n",
                &[],
            ),
            (
                "malformed header ends the table",
                "[dependencies]\n[a b]\nc = { path = \"../c\" }\n",
                &[],
            ),
            (
                "unterminated quoted header ends the table",
                "[dependencies]\n[target.'cfg(unix).dependencies]\nc = { path = \"../c\" }\n",
                &[],
            ),
            (
                "CRLF and BOM",
                "\u{feff}[dependencies]\r\na = { path = \"../a\" }\r\n",
                &["a"],
            ),
            (
                "union across tables keeps duplicates for the caller",
                "[dependencies]\na = { path = \"../a\" }\n[build-dependencies]\na = { path = \"../a\" }\n",
                &["a", "a"],
            ),
        ];
        assert_eq!(cases.len(), 46);
        for (name, manifest, want) in cases {
            assert_eq!(
                cargo_link_dependency_names(manifest),
                want.to_vec(),
                "case {name:?}: {manifest:?}"
            );
        }
    }

    /// Writes `files` (relative path -> contents) under a fresh temporary
    /// directory and returns it.
    fn write_rust_fixture(case: &str, files: &[(&str, &str)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "build_tool_rust_deps_{case}_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        for (path, contents) in files {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
        root
    }

    fn rust_package(name: &str, path: PathBuf) -> Package {
        Package {
            name: name.to_string(),
            path,
            build_commands: vec![],
            language: "rust".to_string(),
        }
    }

    /// A crate named by three link tables yields one dependency, and an
    /// unknown path dependency (`serde`) is dropped.
    #[test]
    fn test_parse_rust_deps_collapses_duplicate_links() {
        let root = write_rust_fixture(
            "dedupe",
            &[
                (
                    "app/Cargo.toml",
                    "[package]\nname = \"app\"\n\n[dependencies]\nlib = { path = \"../lib\" }\n\
                     serde = { path = \"../vendored-serde\" }\n\n[build-dependencies]\n\
                     lib = { path = \"../lib\" }\n\n[target.'cfg(windows)'.dependencies.lib]\n\
                     path = \"../lib\"\n",
                ),
                ("lib/Cargo.toml", "[package]\nname = \"lib\"\n"),
            ],
        );
        let packages = vec![
            rust_package("rust/app", root.join("app")),
            rust_package("rust/lib", root.join("lib")),
        ];
        let deps = parse_rust_deps(&packages[0], &build_known_names(&packages));
        assert_eq!(deps, vec!["rust/lib"]);

        let _ = fs::remove_dir_all(root);
    }

    /// Step 5 of the contract: a Rust crate is known by its directory name
    /// AND by its declared `[package]` `name`, ASCII-lowercased.
    #[test]
    fn test_build_known_names_rust_directory_and_declared_name() {
        let root = write_rust_fixture(
            "known",
            &[(
                "logic-gates/Cargo.toml",
                "[package]\nname = \"Coding-Adventures-Logic-Gates\"\n",
            )],
        );
        let packages = vec![rust_package("rust/logic-gates", root.join("logic-gates"))];
        let known = build_known_names(&packages);
        assert_eq!(
            known.get("logic-gates"),
            Some(&"rust/logic-gates".to_string())
        );
        assert_eq!(
            known.get("coding-adventures-logic-gates"),
            Some(&"rust/logic-gates".to_string())
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_repository_manifest_path_uses_final_canonical_boundary() {
        let host_path = PathBuf::from("private")
            .join("code")
            .join("checkout")
            .join("code")
            .join("packages")
            .join("lua")
            .join("pkg")
            .join("pkg.rockspec");
        assert_eq!(
            repository_manifest_path(&host_path),
            "code/packages/lua/pkg/pkg.rockspec"
        );
    }

    #[test]
    fn test_build_known_names_python() {
        let packages = vec![Package {
            name: "python/logic-gates".to_string(),
            path: PathBuf::from("/repo/code/packages/python/logic-gates"),
            build_commands: vec![],
            language: "python".to_string(),
        }];

        let known = build_known_names(&packages);
        assert_eq!(
            known.get("coding-adventures-logic-gates"),
            Some(&"python/logic-gates".to_string())
        );
    }

    #[test]
    fn test_build_known_names_ruby() {
        let packages = vec![Package {
            name: "ruby/logic_gates".to_string(),
            path: PathBuf::from("/repo/code/packages/ruby/logic_gates"),
            build_commands: vec![],
            language: "ruby".to_string(),
        }];

        let known = build_known_names(&packages);
        assert_eq!(
            known.get("coding_adventures_logic_gates"),
            Some(&"ruby/logic_gates".to_string())
        );
    }

    #[test]
    fn test_extract_deps_single_line() {
        let mut known = HashMap::new();
        known.insert(
            "coding-adventures-logic-gates".to_string(),
            "python/logic-gates".to_string(),
        );
        known.insert(
            "coding-adventures-arithmetic".to_string(),
            "python/arithmetic".to_string(),
        );

        let mut deps = Vec::new();
        extract_deps(
            r#"["coding-adventures-logic-gates>=1.0", "coding-adventures-arithmetic"]"#,
            &known,
            &mut deps,
        );
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"python/logic-gates".to_string()));
        assert!(deps.contains(&"python/arithmetic".to_string()));
    }

    #[test]
    fn test_extract_deps_strips_version() {
        let mut known = HashMap::new();
        known.insert(
            "coding-adventures-logic-gates".to_string(),
            "python/logic-gates".to_string(),
        );

        let mut deps = Vec::new();
        extract_deps(
            r#""coding-adventures-logic-gates>=2.0,<3.0""#,
            &known,
            &mut deps,
        );
        assert_eq!(deps, vec!["python/logic-gates"]);
    }

    #[test]
    fn test_parse_python_deps_with_temp_file() {
        let dir = std::env::temp_dir().join(format!(
            "build_tool_pydeps_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(
            dir.join("pyproject.toml"),
            r#"
[project]
name = "coding-adventures-arithmetic"
dependencies = [
    "coding-adventures-logic-gates>=1.0",
]
"#,
        )
        .unwrap();

        let pkg = Package {
            name: "python/arithmetic".to_string(),
            path: dir.clone(),
            build_commands: vec![],
            language: "python".to_string(),
        };

        let mut known = HashMap::new();
        known.insert(
            "coding-adventures-logic-gates".to_string(),
            "python/logic-gates".to_string(),
        );

        let deps = parse_python_deps(&pkg, &known);
        assert_eq!(deps, vec!["python/logic-gates"]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_dependencies_creates_graph() {
        let dir = std::env::temp_dir().join(format!(
            "build_tool_resolve_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);

        // Package A: no deps
        let dir_a = dir.join("python/logic-gates");
        fs::create_dir_all(&dir_a).unwrap();
        fs::write(
            dir_a.join("pyproject.toml"),
            r#"
[project]
name = "coding-adventures-logic-gates"
dependencies = []
"#,
        )
        .unwrap();

        // Package B: depends on A
        let dir_b = dir.join("python/arithmetic");
        fs::create_dir_all(&dir_b).unwrap();
        fs::write(
            dir_b.join("pyproject.toml"),
            r#"
[project]
name = "coding-adventures-arithmetic"
dependencies = [
    "coding-adventures-logic-gates",
]
"#,
        )
        .unwrap();

        let packages = vec![
            Package {
                name: "python/logic-gates".to_string(),
                path: dir_a,
                build_commands: vec![],
                language: "python".to_string(),
            },
            Package {
                name: "python/arithmetic".to_string(),
                path: dir_b,
                build_commands: vec![],
                language: "python".to_string(),
            },
        ];

        let graph = resolve_dependencies(&packages).unwrap();

        // Verify the graph has both nodes and the correct edge.
        assert!(graph.has_node("python/logic-gates"));
        assert!(graph.has_node("python/arithmetic"));

        // logic-gates should be a predecessor of arithmetic
        // (arithmetic depends on logic-gates).
        let preds = graph.predecessors("python/arithmetic").unwrap();
        assert!(preds.contains(&"python/logic-gates".to_string()));

        let _ = fs::remove_dir_all(&dir);
    }
}
