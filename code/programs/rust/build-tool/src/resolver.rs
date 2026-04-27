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

use std::collections::HashMap;
use std::fs;
use crate::discovery::Package;
use crate::graph::Graph;

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
                known.insert(pypi_name, pkg.name.clone());
            }
            "ruby" => {
                // Convert dir name to gem name: "logic_gates" -> "coding_adventures_logic_gates"
                let gem_name = format!("coding_adventures_{}", dir_name);
                known.insert(gem_name, pkg.name.clone());
            }
            "go" => {
                // For Go, read the module path from go.mod.
                let go_mod = pkg.path.join("go.mod");
                if let Ok(data) = fs::read_to_string(&go_mod) {
                    for line in data.lines() {
                        if line.starts_with("module ") {
                            let module_path = line
                                .trim_start_matches("module ")
                                .trim()
                                .to_lowercase();
                            known.insert(module_path, pkg.name.clone());
                            break;
                        }
                    }
                }
            }
            "rust" | "wasm" => {
                // For Rust, read the package name from Cargo.toml.
                // The crate name in dependencies should match this.
                let cargo_toml = pkg.path.join("Cargo.toml");
                if let Ok(data) = fs::read_to_string(&cargo_toml) {
                    if let Ok(parsed) = data.parse::<toml::Table>() {
                        if let Some(package) = parsed.get("package") {
                            if let Some(name) = package.get("name") {
                                if let Some(name_str) = name.as_str() {
                                    known.insert(name_str.to_lowercase(), pkg.name.clone());
                                }
                            }
                        }
                    }
                }
                if let Some(cargo_name) = read_cargo_package_name(pkg) {
                    known.insert(cargo_name, pkg.name.clone());
                }
            }
            "elixir" => {
                let app_name = format!("coding_adventures_{}", dir_name.replace('-', "_"));
                known.insert(app_name, pkg.name.clone());
            }
            "lua" => {
                // Convert dir name to rockspec name: "logic_gates" -> "coding-adventures-logic-gates"
                // Lua rockspecs use hyphens with a "coding-adventures-" prefix.
                let rock_name = format!("coding-adventures-{}", dir_name.replace('_', "-"));
                known.insert(rock_name, pkg.name.clone());
            }
            "perl" => {
                // Perl CPAN dist names use hyphens: "logic-gates" -> "coding-adventures-logic-gates"
                // This matches the Python convention exactly.
                let cpan_name = format!("coding-adventures-{}", dir_name);
                known.insert(cpan_name, pkg.name.clone());
            }
            "haskell" => {
                // Haskell Cabal package names use hyphens.
                let cabal_name = format!("coding-adventures-{}", dir_name);
                known.insert(cabal_name, pkg.name.clone());
            }
            "csharp" | "fsharp" | "dotnet" => {
                known.insert(dir_name, pkg.name.clone());
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
                let after_eq = trimmed.splitn(2, '=').nth(1).unwrap_or("").trim();

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
/// Rust crates declare local dependencies in Cargo.toml with:
///   [dependencies]
///   my-crate = { path = "../my-crate" }
///
/// We use the `toml` crate to parse the file properly and look for
/// dependencies that have a `path` key (indicating a local dependency)
/// or whose name matches a known internal package.
fn parse_rust_deps(pkg: &Package, known_names: &HashMap<String, String>) -> Vec<String> {
    let cargo_toml = pkg.path.join("Cargo.toml");
    let data = match fs::read_to_string(&cargo_toml) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let parsed: toml::Table = match data.parse() {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };

    let mut internal_deps = Vec::new();

    // Check [dependencies] section.
    if let Some(deps) = parsed.get("dependencies").and_then(|d| d.as_table()) {
        for (name, _value) in deps {
            let lower_name = name.to_lowercase();
            if let Some(pkg_name) = known_names.get(&lower_name) {
                internal_deps.push(pkg_name.clone());
            }
        }
    }

    internal_deps
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
fn parse_lua_deps(pkg: &Package, known_names: &HashMap<String, String>) -> Vec<String> {
    // Find .rockspec files in the package directory.
    let entries = match fs::read_dir(&pkg.path) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
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
        None => return Vec::new(),
    };

    let data = match fs::read_to_string(&rockspec_path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
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

    internal_deps
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
pub fn resolve_dependencies(packages: &[Package]) -> Graph {
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
            "lua" => parse_lua_deps(pkg, known_names),
            "perl" => parse_perl_deps(pkg, known_names),
            "haskell" => parse_haskell_deps(pkg, known_names),
            "csharp" | "fsharp" | "dotnet" => parse_dotnet_deps(pkg, known_names),
            _ => Vec::new(),
        };

        for dep_name in deps {
            // Edge direction: dep -> pkg means "dep must be built before pkg".
            // This convention makes independent_groups() produce the correct
            // build order: nodes with zero in-degree (no deps) come first.
            graph.add_edge(&dep_name, &pkg.name);
        }
    }

    graph
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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

        let graph = resolve_dependencies(&packages);

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
