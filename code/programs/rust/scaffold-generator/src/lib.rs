// =========================================================================
// scaffold-generator -- Generate CI-ready package scaffolding
// =========================================================================
//
// This program generates correctly-structured, CI-ready package directories
// for the coding-adventures monorepo. It supports all six languages:
// Python, Go, Ruby, TypeScript, Rust, and Elixir.
//
// # Why this tool exists
//
// The lessons.md file documents 12+ recurring categories of CI failures
// caused by agents hand-crafting packages inconsistently:
//
//   - Missing BUILD files
//   - TypeScript "main" pointing to dist/ instead of src/
//   - Missing transitive dependency installs in BUILD files
//   - Ruby require ordering (deps before own modules)
//   - Rust workspace Cargo.toml not updated
//   - Missing README.md or CHANGELOG.md
//
// This tool eliminates those failures. Run it, get a package that compiles,
// lints, and passes tests. Then fill in the business logic.
//
// # Architecture
//
//   1. Parse argv manually (no external crate dependencies)
//   2. Resolve dependencies (read sibling package metadata, BFS, topo sort)
//   3. Generate files for each requested language
//
// # Argument parsing
//
// Unlike the Go implementation which uses cli-builder, this Rust version
// parses arguments manually from std::env::args. The scaffold-generator
// only has a handful of flags so hand-parsing is straightforward:
//
//   --type, -t        Package type: "library" or "program" (default: "library")
//   --language, -l    Comma-separated languages or "all" (default: "all")
//   --depends-on, -d  Comma-separated sibling package dependencies
//   --layer           Layer number for README context
//   --description     One-line package description
//   --dry-run         Print what would be generated, don't write
//   --help, -h        Show usage
//   --version, -V     Show version
//   PACKAGE_NAME      Positional: the kebab-case package name (required)

use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

// =========================================================================
// Constants
// =========================================================================

/// The version string, matching the spec and Go implementation.
const VERSION: &str = "1.0.0";

/// All six supported target languages, in canonical order.
const VALID_LANGUAGES: &[&str] = &["python", "go", "ruby", "typescript", "rust", "elixir"];

// =========================================================================
// Name normalization
// =========================================================================
//
// The input package name is always kebab-case (e.g., "my-package"). Each
// language has different naming conventions. These functions convert between
// them.
//
// Examples:
//   "my-package" -> "my_package"  (snake_case, for Python/Ruby/Elixir)
//   "my-package" -> "MyPackage"   (CamelCase, for Ruby modules/Elixir modules)
//   "my-package" -> "mypackage"   (joined lower, for Go package names)

/// Converts "my-package" to "my_package".
///
/// Snake case is the standard naming convention for Python modules, Ruby gems,
/// and Elixir applications. We simply replace every hyphen with an underscore.
pub fn to_snake_case(kebab: &str) -> String {
    kebab.replace('-', "_")
}

/// Converts "my-package" to "MyPackage".
///
/// CamelCase (also called PascalCase) is used for Ruby module names and
/// Elixir module names. We split on hyphens and capitalize the first letter
/// of each segment.
pub fn to_camel_case(kebab: &str) -> String {
    kebab
        .split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + chars.as_str()
                }
            }
        })
        .collect()
}

/// Converts "my-package" to "mypackage".
///
/// Go package names must be a single lowercase word -- no hyphens, no
/// underscores. We simply strip all hyphens.
pub fn to_joined_lower(kebab: &str) -> String {
    kebab.replace('-', "")
}

/// Returns the directory name for a package in a given language.
///
/// Ruby and Elixir use snake_case directories (e.g., "my_package").
/// All other languages use the original kebab-case (e.g., "my-package").
pub fn dir_name(kebab: &str, lang: &str) -> String {
    match lang {
        "ruby" | "elixir" => to_snake_case(kebab),
        _ => kebab.to_string(),
    }
}

/// Validates that a string is valid kebab-case.
///
/// Rules:
///   - Starts with a lowercase letter
///   - Contains only lowercase letters, digits, and hyphens
///   - Segments (between hyphens) must not be empty
///   - Must not start or end with a hyphen
///
/// Examples:
///   "logic-gates"  -> true
///   "my-package-2" -> true
///   "MyPackage"    -> false (uppercase)
///   "-bad"         -> false (leading hyphen)
///   "also--bad"    -> false (double hyphen)
pub fn is_kebab_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    // Must start with a lowercase letter.
    let first = s.chars().next().unwrap();
    if !first.is_ascii_lowercase() {
        return false;
    }
    // Split on hyphens; each segment must be non-empty and contain only
    // lowercase letters and digits.
    for segment in s.split('-') {
        if segment.is_empty() {
            return false;
        }
        if !segment.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
            return false;
        }
    }
    true
}

// =========================================================================
// Argument parsing
// =========================================================================
//
// We parse arguments by iterating through std::env::args and matching on
// known flag names. This is a simple state machine:
//
//   1. If we see --flag or -f that takes a value, consume the next arg
//   2. If we see --flag for a boolean flag, set it to true
//   3. If we see a bare argument (no leading --), treat it as the positional
//      package name (only one is expected)
//
// This approach avoids any external dependencies while being clear and
// easy to understand.

/// Holds the parsed command-line arguments.
///
/// Each field corresponds to one of the CLI flags or the positional argument.
/// Fields use Option<T> where a flag might not be provided, and have sensible
/// defaults applied after parsing.
#[derive(Debug)]
pub struct Args {
    /// The package name (positional argument). Must be kebab-case.
    pub package_name: Option<String>,
    /// Package type: "library" or "program". Default: "library".
    pub pkg_type: String,
    /// Comma-separated language list, or "all". Default: "all".
    pub language: String,
    /// Comma-separated dependency names. Default: empty.
    pub depends_on: String,
    /// Layer number for README context. Default: 0 (no layer).
    pub layer: i32,
    /// One-line description of the package.
    pub description: String,
    /// If true, print what would be generated without writing files.
    pub dry_run: bool,
    /// If true, print help and exit.
    pub help: bool,
    /// If true, print version and exit.
    pub version: bool,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            package_name: None,
            pkg_type: "library".to_string(),
            language: "all".to_string(),
            depends_on: String::new(),
            layer: 0,
            description: String::new(),
            dry_run: false,
            help: false,
            version: false,
        }
    }
}

/// Usage text shown when --help is passed.
const USAGE: &str = "\
scaffold-generator -- Generate CI-ready package scaffolding

USAGE:
    scaffold-generator [OPTIONS] PACKAGE_NAME

ARGUMENTS:
    PACKAGE_NAME    Name of the package in kebab-case (e.g., 'my-package')

OPTIONS:
    -t, --type <TYPE>          Package type: 'library' or 'program' (default: library)
    -l, --language <LANGS>     Comma-separated languages or 'all' (default: all)
    -d, --depends-on <DEPS>    Comma-separated sibling package dependencies
        --layer <N>            Layer number for README context
        --description <TEXT>   One-line description of the package
        --dry-run              Print what would be generated without writing
    -h, --help                 Show this help
    -V, --version              Show version

SUPPORTED LANGUAGES:
    python, go, ruby, typescript, rust, elixir

EXAMPLES:
    scaffold-generator my-package
    scaffold-generator -l rust,go -d logic-gates --layer 2 my-alu
    scaffold-generator --dry-run -t program my-tool";

/// Parses command-line arguments from an iterator of strings.
///
/// This function accepts an iterator so it can be tested without depending
/// on the actual process arguments. The first element should be the program
/// name (it is skipped).
pub fn parse_args<I>(args: I) -> Result<Args, String>
where
    I: IntoIterator<Item = String>,
{
    let mut parsed = Args::default();
    let mut iter = args.into_iter();

    // Skip the program name (argv[0]).
    iter.next();

    // We collect remaining args into a Vec so we can index into them.
    // This makes it easier to handle --flag value pairs.
    let tokens: Vec<String> = iter.collect();
    let mut i = 0;

    while i < tokens.len() {
        let token = &tokens[i];
        match token.as_str() {
            // --- Boolean flags ---
            "--help" | "-h" => {
                parsed.help = true;
            }
            "--version" | "-V" => {
                parsed.version = true;
            }
            "--dry-run" => {
                parsed.dry_run = true;
            }

            // --- Flags that consume the next token as a value ---
            "--type" | "-t" => {
                i += 1;
                if i >= tokens.len() {
                    return Err("--type requires a value".to_string());
                }
                parsed.pkg_type = tokens[i].clone();
            }
            "--language" | "-l" => {
                i += 1;
                if i >= tokens.len() {
                    return Err("--language requires a value".to_string());
                }
                parsed.language = tokens[i].clone();
            }
            "--depends-on" | "-d" => {
                i += 1;
                if i >= tokens.len() {
                    return Err("--depends-on requires a value".to_string());
                }
                parsed.depends_on = tokens[i].clone();
            }
            "--layer" => {
                i += 1;
                if i >= tokens.len() {
                    return Err("--layer requires a value".to_string());
                }
                parsed.layer = tokens[i]
                    .parse::<i32>()
                    .map_err(|_| format!("--layer value must be an integer, got {:?}", tokens[i]))?;
            }
            "--description" => {
                i += 1;
                if i >= tokens.len() {
                    return Err("--description requires a value".to_string());
                }
                parsed.description = tokens[i].clone();
            }

            // --- Positional argument ---
            other => {
                if other.starts_with('-') {
                    return Err(format!("unknown flag: {}", other));
                }
                if parsed.package_name.is_some() {
                    return Err(format!("unexpected extra argument: {}", other));
                }
                parsed.package_name = Some(other.to_string());
            }
        }
        i += 1;
    }

    Ok(parsed)
}

// =========================================================================
// Dependency resolution
// =========================================================================
//
// The scaffold generator reads existing packages' metadata to discover their
// dependencies, then computes the transitive closure and topological sort.
// This is the most critical feature -- missing transitive deps in BUILD files
// is the #1 CI failure category.
//
// Each language stores dependency information differently:
//
//   Python:     BUILD file, "-e ../" entries
//   Go:         go.mod, "=> ../" replace directives
//   Ruby:       Gemfile, path: "../" gem entries
//   TypeScript: package.json, "file:../" dependency values
//   Rust:       Cargo.toml, path = "../" dependency entries
//   Elixir:     mix.exs, path: "../" dependency entries

/// Reads the direct local dependencies of a package by examining its
/// language-specific metadata files.
///
/// Returns dependency names in kebab-case. If the metadata file does not
/// exist or cannot be parsed, returns an empty Vec (not an error), because
/// a package with no metadata simply has no local dependencies.
pub fn read_deps(pkg_dir: &Path, lang: &str) -> Vec<String> {
    match lang {
        "python" => read_python_deps(pkg_dir),
        "go" => read_go_deps(pkg_dir),
        "ruby" => read_ruby_deps(pkg_dir),
        "typescript" => read_typescript_deps(pkg_dir),
        "rust" => read_rust_deps(pkg_dir),
        "elixir" => read_elixir_deps(pkg_dir),
        _ => Vec::new(),
    }
}

/// Reads Python dependencies from the BUILD file.
///
/// Looks for `-e ../package-name` entries, which indicate editable installs
/// of sibling packages. This is the convention used by the coding-adventures
/// monorepo for Python BUILD files.
fn read_python_deps(pkg_dir: &Path) -> Vec<String> {
    let build_path = pkg_dir.join("BUILD");
    let content = match fs::read_to_string(&build_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut deps = Vec::new();
    for line in content.lines() {
        // Look for: -e ../package-name or -e "../package-name"
        for prefix in &["-e ../", "-e \"../"] {
            if let Some(idx) = line.find(prefix) {
                let rest = &line[idx + prefix.len()..];
                let dep: String = rest
                    .chars()
                    .take_while(|c| *c != ' ' && *c != '"' && *c != '\'')
                    .collect();
                if !dep.is_empty() && dep != "." {
                    deps.push(dep);
                }
            }
        }
    }
    deps
}

/// Reads Go dependencies from go.mod replace directives.
///
/// Go modules in this monorepo use `replace` directives to point to sibling
/// packages via relative paths. For example:
///
///   replace github.com/.../logic-gates => ../logic-gates
///
/// We extract the directory name after `=> ../`.
fn read_go_deps(pkg_dir: &Path) -> Vec<String> {
    let mod_path = pkg_dir.join("go.mod");
    let content = match fs::read_to_string(&mod_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut deps = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(idx) = trimmed.find("=> ../") {
            let rest = &trimmed[idx + 6..];
            let dep: String = rest
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            if !dep.is_empty() {
                deps.push(dep);
            }
        }
    }
    deps
}

/// Reads Ruby dependencies from the Gemfile.
///
/// Ruby packages in this monorepo list path dependencies like:
///
///   gem "coding_adventures_logic_gates", path: "../logic_gates"
///
/// We extract the directory name after `"../` and convert it back from
/// snake_case to kebab-case (since Ruby uses snake_case directories).
fn read_ruby_deps(pkg_dir: &Path) -> Vec<String> {
    let gemfile_path = pkg_dir.join("Gemfile");
    let content = match fs::read_to_string(&gemfile_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut deps = Vec::new();
    for line in content.lines() {
        if line.contains("path:") {
            if let Some(idx) = line.find("\"../") {
                let rest = &line[idx + 4..]; // skip past "../
                let dep: String = rest.chars().take_while(|c| *c != '"').collect();
                // Convert snake_case dir back to kebab-case.
                let dep = dep.replace('_', "-");
                if !dep.is_empty() {
                    deps.push(dep);
                }
            }
        }
    }
    deps
}

/// Reads TypeScript dependencies from package.json.
///
/// TypeScript packages use `"file:../"` values in their dependencies:
///
///   "@coding-adventures/logic-gates": "file:../logic-gates"
///
/// We parse the JSON minimally (looking for the pattern in raw text)
/// rather than pulling in a JSON library, since we only need this one
/// specific pattern.
fn read_typescript_deps(pkg_dir: &Path) -> Vec<String> {
    let pkg_json_path = pkg_dir.join("package.json");
    let content = match fs::read_to_string(&pkg_json_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut deps = Vec::new();
    // Look for lines containing "file:../" which is the monorepo convention
    // for TypeScript sibling dependencies.
    for line in content.lines() {
        if let Some(idx) = line.find("\"file:../") {
            let rest = &line[idx + 9..]; // skip past "file:../
            let dep: String = rest.chars().take_while(|c| *c != '"').collect();
            if !dep.is_empty() {
                deps.push(dep);
            }
        }
    }
    deps
}

/// Reads Rust dependencies from Cargo.toml path entries.
///
/// Rust crates in this monorepo use path dependencies:
///
///   logic-gates = { path = "../logic-gates" }
///
/// We look for `path = "../` and extract the directory name.
fn read_rust_deps(pkg_dir: &Path) -> Vec<String> {
    let cargo_path = pkg_dir.join("Cargo.toml");
    let content = match fs::read_to_string(&cargo_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut deps = Vec::new();
    for line in content.lines() {
        if let Some(idx) = line.find("path = \"../") {
            let rest = &line[idx + 11..];
            let dep: String = rest.chars().take_while(|c| *c != '"').collect();
            if !dep.is_empty() {
                deps.push(dep);
            }
        }
    }
    deps
}

/// Reads Elixir dependencies from mix.exs path entries.
///
/// Elixir packages in this monorepo use path dependencies:
///
///   {:coding_adventures_logic_gates, path: "../logic_gates"}
///
/// We extract the directory name after `path: "../` and convert it back
/// from snake_case to kebab-case.
fn read_elixir_deps(pkg_dir: &Path) -> Vec<String> {
    let mix_path = pkg_dir.join("mix.exs");
    let content = match fs::read_to_string(&mix_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut deps = Vec::new();
    for line in content.lines() {
        if let Some(idx) = line.find("path: \"../") {
            let rest = &line[idx + 10..];
            let dep: String = rest.chars().take_while(|c| *c != '"').collect();
            // Convert snake_case dir back to kebab-case.
            let dep = dep.replace('_', "-");
            if !dep.is_empty() {
                deps.push(dep);
            }
        }
    }
    deps
}

// =========================================================================
// Transitive closure (BFS)
// =========================================================================
//
// Starting from the direct dependencies, we do a breadth-first search to
// discover all transitive dependencies. For each dependency, we read its
// metadata to find its own dependencies, and add any new ones to the queue.
//
// Example: if A depends on B, and B depends on C, then transitive_closure(A)
// returns [B, C].

/// Computes the full set of transitive dependencies starting from the
/// given direct dependencies.
///
/// Uses breadth-first search: start with the direct deps in a queue, and
/// for each dep, read its own deps and add any unseen ones to the queue.
/// Returns the complete set sorted alphabetically.
pub fn transitive_closure(direct_deps: &[String], lang: &str, base_dir: &Path) -> Vec<String> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = direct_deps.iter().cloned().collect();

    while let Some(dep) = queue.pop_front() {
        if visited.contains(&dep) {
            continue;
        }
        visited.insert(dep.clone());

        let dep_dir = base_dir.join(dir_name(&dep, lang));
        let dep_deps = read_deps(&dep_dir, lang);
        for dd in dep_deps {
            if !visited.contains(&dd) {
                queue.push_back(dd);
            }
        }
    }

    let mut result: Vec<String> = visited.into_iter().collect();
    result.sort();
    result
}

// =========================================================================
// Topological sort (Kahn's algorithm)
// =========================================================================
//
// After computing the transitive closure, we need to determine the install
// order. Dependencies that have no dependencies of their own (leaves) must
// be installed first, then packages that depend only on those leaves, and
// so on up the tree.
//
// Kahn's algorithm works by:
//   1. Computing the in-degree (number of dependencies within the set)
//      for each node
//   2. Starting with all nodes that have in-degree 0 (leaves)
//   3. Processing each leaf, reducing the in-degree of its dependents
//   4. When a node's in-degree reaches 0, it joins the queue
//
// This produces a topological ordering where leaves come first -- exactly
// what we need for BUILD file install ordering.

/// Returns dependencies in leaf-first order using Kahn's algorithm.
///
/// The resulting order ensures that when we install dependencies one by
/// one, each dependency's own dependencies are already installed by the
/// time we get to it. This is critical for BUILD files that must chain
/// installs from leaves to root.
pub fn topological_sort(all_deps: &[String], lang: &str, base_dir: &Path) -> Result<Vec<String>, String> {
    let dep_set: HashSet<&str> = all_deps.iter().map(|s| s.as_str()).collect();

    // Build the dependency graph: for each dep, which other deps (within our set)
    // does it depend on?
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut in_degree: HashMap<&str, usize> = HashMap::new();

    for dep in all_deps {
        graph.insert(dep.as_str(), Vec::new());
        in_degree.insert(dep.as_str(), 0);
    }

    for dep in all_deps {
        let dep_dir = base_dir.join(dir_name(dep, lang));
        let dep_deps = read_deps(&dep_dir, lang);
        for dd in &dep_deps {
            if dep_set.contains(dd.as_str()) {
                graph.get_mut(dep.as_str()).unwrap().push(
                    all_deps
                        .iter()
                        .find(|d| d.as_str() == dd.as_str())
                        .unwrap()
                        .as_str(),
                );
            }
        }
    }

    // In-degree = how many deps (within the set) each node depends on.
    // A leaf (no dependencies within the set) has in-degree 0.
    for dep in all_deps {
        let count = graph[dep.as_str()].len();
        in_degree.insert(dep.as_str(), count);
    }

    // Start with all leaves (in-degree 0).
    let mut queue: Vec<&str> = all_deps
        .iter()
        .filter(|d| in_degree[d.as_str()] == 0)
        .map(|d| d.as_str())
        .collect();
    queue.sort(); // deterministic output

    let mut result: Vec<String> = Vec::new();

    while let Some(node) = queue.first().cloned() {
        queue.remove(0);
        result.push(node.to_string());

        // Find all nodes that depend on `node` and decrease their in-degree.
        for dep in all_deps {
            if graph[dep.as_str()].contains(&node) {
                let deg = in_degree.get_mut(dep.as_str()).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push(dep.as_str());
                    queue.sort();
                }
            }
        }
    }

    if result.len() != all_deps.len() {
        return Err(format!(
            "circular dependency detected: resolved {} of {} deps",
            result.len(),
            all_deps.len()
        ));
    }

    Ok(result)
}

// =========================================================================
// File generation -- Python
// =========================================================================
//
// Python packages in this monorepo follow a standard structure:
//
//   my-package/
//     pyproject.toml          -- metadata, build system, tool config
//     src/my_package/
//       __init__.py            -- module entry point
//     tests/
//       __init__.py            -- makes tests a package
//       test_my_package.py     -- test file
//     BUILD                    -- CI build script

/// Generates a Python package scaffold.
///
/// Creates pyproject.toml (hatchling-based), src layout with __init__.py,
/// a test file using pytest, and a BUILD file that installs transitive deps
/// in the correct order before running tests.
fn generate_python(
    target_dir: &Path,
    pkg_name: &str,
    description: &str,
    layer_ctx: &str,
    _direct_deps: &[String],
    ordered_deps: &[String],
) -> io::Result<()> {
    let snake = to_snake_case(pkg_name);

    // --- pyproject.toml ---
    let pyproject = format!(
        r#"[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "coding-adventures-{pkg_name}"
version = "0.1.0"
description = "{description}"
requires-python = ">=3.12"
license = "MIT"
authors = [{{ name = "Adhithya Rajasekaran" }}]
readme = "README.md"

[project.optional-dependencies]
dev = ["pytest>=8.0", "pytest-cov>=5.0", "ruff>=0.4", "mypy>=1.10"]

[tool.hatch.build.targets.wheel]
packages = ["src/{snake}"]

[tool.ruff]
target-version = "py312"
line-length = 88

[tool.ruff.lint]
select = ["E", "W", "F", "I", "UP", "B", "SIM", "ANN"]

[tool.pytest.ini_options]
testpaths = ["tests"]
addopts = "--cov={snake} --cov-report=term-missing --cov-fail-under=80"

[tool.coverage.run]
source = ["src/{snake}"]

[tool.coverage.report]
fail_under = 80
show_missing = true
"#,
        pkg_name = pkg_name,
        description = description,
        snake = snake,
    );

    // --- src/__init__.py ---
    let init_py = format!(
        r#""""{pkg_name} -- {description}

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.
{layer_ctx}"""

__version__ = "0.1.0"
"#,
        pkg_name = pkg_name,
        description = description,
        layer_ctx = layer_ctx,
    );

    // --- tests/test_*.py ---
    let test_py = format!(
        r#""""Tests for {pkg_name}."""

from {snake} import __version__


class TestVersion:
    """Verify the package is importable and has a version."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"
"#,
        pkg_name = pkg_name,
        snake = snake,
    );

    // --- BUILD ---
    let mut build_lines: Vec<String> = Vec::new();
    build_lines.push("uv venv .venv --quiet --no-project".to_string());
    for dep in ordered_deps {
        build_lines.push(format!("uv pip install --python .venv -e ../{} --quiet", dep));
    }
    build_lines.push("uv pip install --python .venv -e .[dev] --quiet".to_string());
    build_lines.push("uv run --no-project python -m pytest tests/ -v".to_string());
    let build = build_lines.join("\n") + "\n";

    // Create directories
    let src_dir = target_dir.join("src").join(&snake);
    let test_dir = target_dir.join("tests");
    fs::create_dir_all(&src_dir)?;
    fs::create_dir_all(&test_dir)?;

    // Write files
    fs::write(target_dir.join("pyproject.toml"), &pyproject)?;
    fs::write(src_dir.join("__init__.py"), &init_py)?;
    fs::write(test_dir.join("__init__.py"), "")?;
    fs::write(
        test_dir.join(format!("test_{}.py", snake)),
        &test_py,
    )?;
    fs::write(target_dir.join("BUILD"), &build)?;

    Ok(())
}

// =========================================================================
// File generation -- Go
// =========================================================================
//
// Go packages in this monorepo follow a standard structure:
//
//   my-package/
//     go.mod                 -- module definition with replace directives
//     my_package.go          -- source file
//     my_package_test.go     -- test file
//     BUILD                  -- CI build script

/// Generates a Go package scaffold.
///
/// Creates go.mod with require/replace directives for dependencies, a source
/// file, a test file, and a BUILD file.
fn generate_go(
    target_dir: &Path,
    pkg_name: &str,
    description: &str,
    layer_ctx: &str,
    direct_deps: &[String],
    all_transitive_deps: &[String],
) -> io::Result<()> {
    let go_pkg = to_joined_lower(pkg_name);
    let snake = to_snake_case(pkg_name);

    // --- go.mod ---
    let mut go_mod = format!(
        "module github.com/adhithyan15/coding-adventures/code/packages/go/{}\n\ngo 1.26\n",
        pkg_name
    );

    if !direct_deps.is_empty() {
        go_mod.push_str("\nrequire (\n");
        for dep in direct_deps {
            go_mod.push_str(&format!(
                "\tgithub.com/adhithyan15/coding-adventures/code/packages/go/{} v0.0.0\n",
                dep
            ));
        }
        go_mod.push_str(")\n");

        go_mod.push_str("\nreplace (\n");
        for dep in all_transitive_deps {
            go_mod.push_str(&format!(
                "\tgithub.com/adhithyan15/coding-adventures/code/packages/go/{dep} => ../{dep}\n",
                dep = dep
            ));
        }
        go_mod.push_str(")\n");
    }

    // --- source file ---
    let src_file = format!(
        "// Package {go_pkg} provides {description}.\n\
         //\n\
         // This package is part of the coding-adventures monorepo, a ground-up\n\
         // implementation of the computing stack from transistors to operating systems.\n\
         // {layer_ctx}\n\
         package {go_pkg}\n",
        go_pkg = go_pkg,
        description = description,
        layer_ctx = layer_ctx,
    );

    // --- test file ---
    let test_file = format!(
        "package {go_pkg}\n\n\
         import \"testing\"\n\n\
         func TestPackageLoads(t *testing.T) {{\n\
         \tt.Log(\"{pkg_name} package loaded successfully\")\n\
         }}\n",
        go_pkg = go_pkg,
        pkg_name = pkg_name,
    );

    let build = "go test ./... -v -cover\n".to_string();

    // Write files
    fs::write(target_dir.join("go.mod"), &go_mod)?;
    fs::write(target_dir.join(format!("{}.go", snake)), &src_file)?;
    fs::write(target_dir.join(format!("{}_test.go", snake)), &test_file)?;
    fs::write(target_dir.join("BUILD"), &build)?;

    Ok(())
}

// =========================================================================
// File generation -- Ruby
// =========================================================================
//
// Ruby packages in this monorepo follow a standard structure:
//
//   my_package/
//     coding_adventures_my_package.gemspec
//     Gemfile
//     Rakefile
//     lib/coding_adventures_my_package.rb        -- entry point
//     lib/coding_adventures/my_package/version.rb
//     test/test_my_package.rb
//     BUILD

/// Generates a Ruby package scaffold.
fn generate_ruby(
    target_dir: &Path,
    pkg_name: &str,
    description: &str,
    _layer_ctx: &str,
    direct_deps: &[String],
    all_transitive_deps: &[String],
) -> io::Result<()> {
    let snake = to_snake_case(pkg_name);
    let camel = to_camel_case(pkg_name);

    // --- gemspec ---
    let mut gemspec = format!(
        r#"# frozen_string_literal: true

require_relative "lib/coding_adventures/{snake}/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_{snake}"
  spec.version       = CodingAdventures::{camel}::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "{description}"
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {{
    "source_code_uri"        => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required"  => "true"
  }}

"#,
        snake = snake,
        camel = camel,
        description = description,
    );

    for dep in direct_deps {
        let dep_snake = to_snake_case(dep);
        gemspec.push_str(&format!(
            "  spec.add_dependency \"coding_adventures_{}\", \"~> 0.1\"\n",
            dep_snake
        ));
    }
    gemspec.push_str(
        "  spec.add_development_dependency \"minitest\", \"~> 5.0\"\n\
         \x20 spec.add_development_dependency \"rake\", \"~> 13.0\"\n\
         end\n",
    );

    // --- Gemfile ---
    let mut gemfile = String::from(
        "# frozen_string_literal: true\n\nsource \"https://rubygems.org\"\ngemspec\n",
    );
    if !all_transitive_deps.is_empty() {
        gemfile.push_str("\n# All transitive path dependencies must be listed here.\n");
        gemfile.push_str("# Bundler needs to know where to find each gem locally.\n");
        for dep in all_transitive_deps {
            let dep_snake = to_snake_case(dep);
            gemfile.push_str(&format!(
                "gem \"coding_adventures_{}\", path: \"../{}\"\n",
                dep_snake, dep_snake
            ));
        }
    }

    // --- Rakefile ---
    let rakefile = r#"# frozen_string_literal: true

require "rake/testtask"

Rake::TestTask.new(:test) do |t|
  t.libs << "test"
  t.libs << "lib"
  t.test_files = FileList["test/**/test_*.rb"]
end

task default: :test
"#;

    // --- Entry point (lib/coding_adventures_*.rb) ---
    // IMPORTANT: Require dependencies FIRST, before own modules (Ruby lesson).
    let mut entry_point = String::from("# frozen_string_literal: true\n\n");
    if !direct_deps.is_empty() {
        entry_point.push_str("# IMPORTANT: Require dependencies FIRST, before own modules.\n");
        entry_point
            .push_str("# Ruby loads files in require order. If our modules reference\n");
        entry_point
            .push_str("# constants from dependencies, those gems must be loaded first.\n");
        for dep in direct_deps {
            let dep_snake = to_snake_case(dep);
            entry_point.push_str(&format!("require \"coding_adventures_{}\"\n", dep_snake));
        }
        entry_point.push('\n');
    }
    entry_point.push_str(&format!(
        "require_relative \"coding_adventures/{}/version\"\n\n",
        snake
    ));
    entry_point.push_str(&format!(
        "module CodingAdventures\n  # {}\n  module {}\n  end\nend\n",
        description, camel
    ));

    // --- version.rb ---
    let version_rb = format!(
        "# frozen_string_literal: true\n\n\
         module CodingAdventures\n\
         \x20 module {}\n\
         \x20   VERSION = \"0.1.0\"\n\
         \x20 end\n\
         end\n",
        camel
    );

    // --- test file ---
    let test_rb = format!(
        "# frozen_string_literal: true\n\n\
         require \"minitest/autorun\"\n\
         require \"coding_adventures_{snake}\"\n\n\
         class Test{camel} < Minitest::Test\n\
         \x20 def test_version_exists\n\
         \x20   refute_nil CodingAdventures::{camel}::VERSION\n\
         \x20 end\n\
         end\n",
        snake = snake,
        camel = camel,
    );

    let build = "bundle install --quiet\nbundle exec rake test\n".to_string();

    // Create directories
    let lib_dir = target_dir
        .join("lib")
        .join("coding_adventures")
        .join(&snake);
    let test_dir = target_dir.join("test");
    fs::create_dir_all(&lib_dir)?;
    fs::create_dir_all(&test_dir)?;

    // Write files
    fs::write(
        target_dir.join(format!("coding_adventures_{}.gemspec", snake)),
        &gemspec,
    )?;
    fs::write(target_dir.join("Gemfile"), &gemfile)?;
    fs::write(target_dir.join("Rakefile"), rakefile)?;
    fs::write(
        target_dir.join("lib").join(format!("coding_adventures_{}.rb", snake)),
        &entry_point,
    )?;
    fs::write(lib_dir.join("version.rb"), &version_rb)?;
    fs::write(test_dir.join(format!("test_{}.rb", snake)), &test_rb)?;
    fs::write(target_dir.join("BUILD"), &build)?;

    Ok(())
}

// =========================================================================
// File generation -- TypeScript
// =========================================================================
//
// TypeScript packages in this monorepo follow a standard structure:
//
//   my-package/
//     package.json           -- IMPORTANT: "main": "src/index.ts" (not dist/)
//     tsconfig.json
//     vitest.config.ts
//     src/index.ts
//     tests/my-package.test.ts
//     BUILD

/// Generates a TypeScript package scaffold.
///
/// Key lesson: The "main" field in package.json MUST point to "src/index.ts",
/// not "dist/index.js". Vitest resolves file: dependencies using the main
/// field, and since we don't pre-compile, it must point to the TypeScript
/// source.
fn generate_typescript(
    target_dir: &Path,
    pkg_name: &str,
    description: &str,
    layer_ctx: &str,
    direct_deps: &[String],
    ordered_deps: &[String],
) -> io::Result<()> {
    // --- package.json ---
    let deps_json = if direct_deps.is_empty() {
        String::new()
    } else {
        let entries: Vec<String> = direct_deps
            .iter()
            .map(|dep| {
                format!(
                    "    \"@coding-adventures/{}\": \"file:../{}\"",
                    dep, dep
                )
            })
            .collect();
        entries.join(",\n")
    };

    let package_json = format!(
        r#"{{
  "name": "@coding-adventures/{pkg_name}",
  "version": "0.1.0",
  "description": "{description}",
  "type": "module",
  "main": "src/index.ts",
  "scripts": {{
    "build": "tsc",
    "test": "vitest run",
    "test:coverage": "vitest run --coverage"
  }},
  "author": "Adhithya Rajasekaran",
  "license": "MIT",
  "dependencies": {{
{deps_json}
  }},
  "devDependencies": {{
    "typescript": "^5.0.0",
    "vitest": "^3.0.0",
    "@vitest/coverage-v8": "^3.0.0"
  }}
}}
"#,
        pkg_name = pkg_name,
        description = description,
        deps_json = deps_json,
    );

    let tsconfig = r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "outDir": "dist",
    "rootDir": "src",
    "declaration": true
  },
  "include": ["src"]
}
"#;

    let vitest_config = r#"import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      thresholds: {
        lines: 80,
      },
    },
  },
});
"#;

    let index_ts = format!(
        "/**\n\
         \x20* @coding-adventures/{pkg_name}\n\
         \x20*\n\
         \x20* {description}\n\
         \x20*\n\
         \x20* This package is part of the coding-adventures monorepo, a ground-up\n\
         \x20* implementation of the computing stack from transistors to operating systems.\n\
         \x20* {layer_ctx}\n\
         \x20*/\n\n\
         export const VERSION = \"0.1.0\";\n",
        pkg_name = pkg_name,
        description = description,
        layer_ctx = layer_ctx,
    );

    let test_ts = format!(
        "import {{ describe, it, expect }} from \"vitest\";\n\
         import {{ VERSION }} from \"../src/index.js\";\n\n\
         describe(\"{pkg_name}\", () => {{\n\
         \x20 it(\"has a version\", () => {{\n\
         \x20   expect(VERSION).toBe(\"0.1.0\");\n\
         \x20 }});\n\
         }});\n",
        pkg_name = pkg_name,
    );

    // --- BUILD --- npm ci resolves file: deps transitively
    let build = "npm ci --quiet\nnpx vitest run --coverage\n".to_string();

    // Create directories
    let src_dir = target_dir.join("src");
    let tests_dir = target_dir.join("tests");
    fs::create_dir_all(&src_dir)?;
    fs::create_dir_all(&tests_dir)?;

    // Write files
    fs::write(target_dir.join("package.json"), &package_json)?;
    fs::write(target_dir.join("tsconfig.json"), tsconfig)?;
    fs::write(target_dir.join("vitest.config.ts"), vitest_config)?;
    fs::write(src_dir.join("index.ts"), &index_ts)?;
    fs::write(
        tests_dir.join(format!("{}.test.ts", pkg_name)),
        &test_ts,
    )?;
    fs::write(target_dir.join("BUILD"), &build)?;

    Ok(())
}

// =========================================================================
// File generation -- Rust
// =========================================================================
//
// Rust packages (crates) in this monorepo follow a standard structure:
//
//   my-package/
//     Cargo.toml             -- crate metadata, path dependencies
//     src/lib.rs             -- library entry point
//     BUILD                  -- CI build script
//
// IMPORTANT: After generating a Rust crate, the workspace Cargo.toml
// at code/packages/rust/Cargo.toml must be updated to include the new
// crate in its members list (see update_rust_workspace).

/// Generates a Rust crate scaffold.
fn generate_rust(
    target_dir: &Path,
    pkg_name: &str,
    description: &str,
    layer_ctx: &str,
    direct_deps: &[String],
) -> io::Result<()> {
    // --- Cargo.toml ---
    let mut cargo = format!(
        "[package]\n\
         name = \"{}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2021\"\n\
         description = \"{}\"\n\n\
         [dependencies]\n",
        pkg_name, description
    );

    for dep in direct_deps {
        cargo.push_str(&format!(
            "{} = {{ path = \"../{}\" }}\n",
            dep, dep
        ));
    }

    // --- src/lib.rs ---
    let lib_rs = format!(
        "//! # {pkg_name}\n\
         //!\n\
         //! {description}\n\
         //!\n\
         //! This crate is part of the coding-adventures monorepo, a ground-up\n\
         //! implementation of the computing stack from transistors to operating systems.\n\
         //! {layer_ctx}\n\n\
         #[cfg(test)]\n\
         mod tests {{\n\
         \x20   #[test]\n\
         \x20   fn it_loads() {{\n\
         \x20       assert!(true, \"{pkg_name} crate loaded successfully\");\n\
         \x20   }}\n\
         }}\n",
        pkg_name = pkg_name,
        description = description,
        layer_ctx = layer_ctx,
    );

    let build = format!("cargo test -p {} -- --nocapture\n", pkg_name);

    // Create directories
    let src_dir = target_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    // Write files
    fs::write(target_dir.join("Cargo.toml"), &cargo)?;
    fs::write(src_dir.join("lib.rs"), &lib_rs)?;
    fs::write(target_dir.join("BUILD"), &build)?;

    Ok(())
}

// =========================================================================
// File generation -- Elixir
// =========================================================================
//
// Elixir packages in this monorepo follow a standard structure:
//
//   my_package/
//     mix.exs                -- project definition and dependencies
//     lib/coding_adventures/my_package.ex
//     test/my_package_test.exs
//     test/test_helper.exs
//     BUILD

/// Generates an Elixir package scaffold.
fn generate_elixir(
    target_dir: &Path,
    pkg_name: &str,
    description: &str,
    layer_ctx: &str,
    direct_deps: &[String],
    ordered_deps: &[String],
) -> io::Result<()> {
    let snake = to_snake_case(pkg_name);
    let camel = to_camel_case(pkg_name);

    // --- mix.exs ---
    let mut mix_exs = format!(
        "defmodule CodingAdventures.{camel}.MixProject do\n\
         \x20 use Mix.Project\n\n\
         \x20 def project do\n\
         \x20   [\n\
         \x20     app: :coding_adventures_{snake},\n\
         \x20     version: \"0.1.0\",\n\
         \x20     elixir: \"~> 1.14\",\n\
         \x20     start_permanent: Mix.env() == :prod,\n\
         \x20     deps: deps(),\n\
         \x20     test_coverage: [\n\
         \x20       summary: [threshold: 80]\n\
         \x20     ]\n\
         \x20   ]\n\
         \x20 end\n\n\
         \x20 def application do\n\
         \x20   [\n\
         \x20     extra_applications: [:logger]\n\
         \x20   ]\n\
         \x20 end\n\n\
         \x20 defp deps do\n\
         \x20   [\n",
        camel = camel,
        snake = snake,
    );

    for (i, dep) in direct_deps.iter().enumerate() {
        let dep_snake = to_snake_case(dep);
        let comma = if i == direct_deps.len() - 1 { "" } else { "," };
        mix_exs.push_str(&format!(
            "      {{:coding_adventures_{}, path: \"../{}\"}}{}\n",
            dep_snake, dep_snake, comma
        ));
    }
    mix_exs.push_str("    ]\n  end\nend\n");

    // --- lib module ---
    let lib_ex = format!(
        "defmodule CodingAdventures.{camel} do\n\
         \x20 @moduledoc \"\"\"\n\
         \x20 {description}\n\n\
         \x20 This module is part of the coding-adventures monorepo, a ground-up\n\
         \x20 implementation of the computing stack from transistors to operating systems.\n\
         \x20 {layer_ctx}\n\
         \x20 \"\"\"\n\
         end\n",
        camel = camel,
        description = description,
        layer_ctx = layer_ctx,
    );

    // --- test ---
    let test_exs = format!(
        "defmodule CodingAdventures.{camel}Test do\n\
         \x20 use ExUnit.Case\n\n\
         \x20 test \"module loads\" do\n\
         \x20   assert Code.ensure_loaded?(CodingAdventures.{camel})\n\
         \x20 end\n\
         end\n",
        camel = camel,
    );

    let test_helper = "ExUnit.start()\n";

    // --- BUILD --- chain install transitive deps
    let build = if !ordered_deps.is_empty() {
        let mut parts: Vec<String> = ordered_deps
            .iter()
            .map(|dep| {
                let dep_snake = to_snake_case(dep);
                format!(
                    "cd ../{} && mix deps.get --quiet && mix compile --quiet",
                    dep_snake
                )
            })
            .collect();
        parts.push(format!(
            "cd ../{} && mix deps.get --quiet && mix test --cover",
            snake
        ));
        parts.join(" && \\\n") + "\n"
    } else {
        "mix deps.get --quiet && mix test --cover\n".to_string()
    };

    // Create directories
    let lib_dir = target_dir.join("lib").join("coding_adventures");
    let test_dir = target_dir.join("test");
    fs::create_dir_all(&lib_dir)?;
    fs::create_dir_all(&test_dir)?;

    // Write files
    fs::write(target_dir.join("mix.exs"), &mix_exs)?;
    fs::write(
        lib_dir.join(format!("{}.ex", snake)),
        &lib_ex,
    )?;
    fs::write(
        test_dir.join(format!("{}_test.exs", snake)),
        &test_exs,
    )?;
    fs::write(test_dir.join("test_helper.exs"), test_helper)?;
    fs::write(target_dir.join("BUILD"), &build)?;

    Ok(())
}

// =========================================================================
// Common files (README, CHANGELOG)
// =========================================================================
//
// Every package, regardless of language, must have:
//   - README.md  -- what it does, dependencies, development instructions
//   - CHANGELOG.md -- version history starting from 0.1.0

/// Generates README.md and CHANGELOG.md for any language.
pub fn generate_common_files(
    target_dir: &Path,
    pkg_name: &str,
    description: &str,
    layer: i32,
    direct_deps: &[String],
) -> io::Result<()> {
    // We use a fixed date format. In a real tool we'd use chrono, but since
    // we're std-only, we'll use a placeholder that matches the generation date.
    let today = {
        // Read the system time and format as YYYY-MM-DD.
        // std::time doesn't have calendar formatting, so we use a simple approach.
        // For correctness we'd need a date library, but for scaffolding the exact
        // date isn't critical -- we'll use a reasonable approximation.
        use std::time::SystemTime;
        let _now = SystemTime::now();
        // Since we can't easily format dates with std alone, use a fixed approach.
        // The Go implementation uses time.Now().Format("2006-01-02").
        // We'll compute it manually from the UNIX timestamp.
        format_date_today()
    };

    // --- CHANGELOG.md ---
    let changelog = format!(
        "# Changelog\n\n\
         All notable changes to this package will be documented in this file.\n\n\
         ## [0.1.0] - {}\n\n\
         ### Added\n\n\
         - Initial package scaffolding generated by scaffold-generator\n",
        today
    );

    // --- README.md ---
    let mut readme = format!("# {}\n\n{}\n", pkg_name, description);
    if layer > 0 {
        readme.push_str(&format!(
            "\n## Layer {}\n\nThis package is part of Layer {} of the coding-adventures computing stack.\n",
            layer, layer
        ));
    }
    if !direct_deps.is_empty() {
        readme.push_str("\n## Dependencies\n\n");
        for dep in direct_deps {
            readme.push_str(&format!("- {}\n", dep));
        }
    }
    readme.push_str("\n## Development\n\n```bash\n# Run tests\nbash BUILD\n```\n");

    fs::write(target_dir.join("README.md"), &readme)?;
    fs::write(target_dir.join("CHANGELOG.md"), &changelog)?;

    Ok(())
}

/// Formats today's date as YYYY-MM-DD using only std.
///
/// Since Rust's std library doesn't have calendar date formatting, we
/// compute the date from the UNIX timestamp using basic arithmetic.
/// This handles leap years correctly via the standard algorithm.
fn format_date_today() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Convert UNIX timestamp to calendar date.
    // Algorithm: compute days since epoch, then convert to year/month/day.
    let days = (secs / 86400) as i64;

    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}", y, m, d)
}

// =========================================================================
// Rust workspace integration
// =========================================================================
//
// When generating a Rust library crate, we must also update the workspace
// Cargo.toml at code/packages/rust/Cargo.toml to include the new crate
// in its members list. Without this, the crate won't be part of
// `cargo build --workspace` and will be invisible to CI.

/// Adds a new crate to the workspace Cargo.toml members list.
///
/// Reads the workspace Cargo.toml, finds the `members = [...]` array,
/// and inserts the new crate name if it's not already present.
pub fn update_rust_workspace(repo_root: &Path, pkg_name: &str) -> Result<(), String> {
    let workspace_path = repo_root
        .join("code")
        .join("packages")
        .join("rust")
        .join("Cargo.toml");

    let content = fs::read_to_string(&workspace_path)
        .map_err(|e| format!("cannot read workspace Cargo.toml: {}", e))?;

    // Check if already present.
    if content.contains(&format!("\"{}\"", pkg_name)) {
        return Ok(());
    }

    // Find the members = [...] array and add the new crate.
    let members_idx = content
        .find("members = [")
        .ok_or_else(|| "cannot find members = [ in workspace Cargo.toml".to_string())?;

    let closing_idx = content[members_idx..]
        .find(']')
        .ok_or_else(|| "cannot find closing ] for members array".to_string())?
        + members_idx;

    let new_entry = format!("  \"{}\",\n", pkg_name);
    let new_content = format!(
        "{}{}{}",
        &content[..closing_idx],
        new_entry,
        &content[closing_idx..]
    );

    fs::write(&workspace_path, &new_content)
        .map_err(|e| format!("cannot write workspace Cargo.toml: {}", e))?;

    Ok(())
}

// =========================================================================
// Find repo root
// =========================================================================

/// Walks up from the current directory to find the git repository root.
///
/// The repo root is identified by the presence of a `.git` directory.
/// Returns an error if we reach the filesystem root without finding one.
fn find_repo_root() -> Result<PathBuf, String> {
    let mut dir = env::current_dir().map_err(|e| format!("cannot get current directory: {}", e))?;
    loop {
        if dir.join(".git").exists() {
            return Ok(dir);
        }
        let parent = dir.parent().map(|p| p.to_path_buf());
        match parent {
            Some(p) if p != dir => dir = p,
            _ => return Err("not inside a git repository".to_string()),
        }
    }
}

// =========================================================================
// Scaffold configuration
// =========================================================================

/// Holds the parsed and validated configuration for scaffolding.
struct ScaffoldConfig {
    package_name: String,
    pkg_type: String,
    languages: Vec<String>,
    direct_deps: Vec<String>,
    layer: i32,
    description: String,
    dry_run: bool,
    repo_root: PathBuf,
}

// =========================================================================
// Main scaffolding logic
// =========================================================================

/// Generates the package scaffold for a single language.
///
/// This function orchestrates the entire generation process:
///   1. Determine the target directory
///   2. Validate that dependencies exist
///   3. Compute transitive closure and topological sort
///   4. Generate language-specific files
///   5. Generate common files (README, CHANGELOG)
///   6. Perform post-generation steps (e.g., Rust workspace update)
fn scaffold(
    cfg: &ScaffoldConfig,
    lang: &str,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<(), String> {
    // Determine base directory.
    let base_category = if cfg.pkg_type == "library" {
        "packages"
    } else {
        "programs"
    };
    let base_dir = cfg.repo_root.join("code").join(base_category).join(lang);
    let d_name = dir_name(&cfg.package_name, lang);
    let target_dir = base_dir.join(&d_name);

    // Check target doesn't already exist.
    if target_dir.exists() {
        return Err(format!("directory already exists: {}", target_dir.display()));
    }

    // Validate dependencies exist.
    for dep in &cfg.direct_deps {
        let dep_dir = base_dir.join(dir_name(dep, lang));
        if !dep_dir.exists() {
            return Err(format!(
                "dependency {:?} not found for {} at {}",
                dep,
                lang,
                dep_dir.display()
            ));
        }
    }

    // Compute transitive closure and topological sort.
    let all_deps = transitive_closure(&cfg.direct_deps, lang, &base_dir);
    let ordered_deps = topological_sort(&all_deps, lang, &base_dir)?;

    let layer_ctx = if cfg.layer > 0 {
        format!("Layer {} in the computing stack.", cfg.layer)
    } else {
        String::new()
    };

    // Dry run: just print what would happen.
    if cfg.dry_run {
        writeln!(
            stdout,
            "[dry-run] Would create {} package at: {}",
            lang,
            target_dir.display()
        )
        .ok();
        writeln!(stdout, "  Direct deps: {:?}", cfg.direct_deps).ok();
        writeln!(stdout, "  All transitive deps: {:?}", all_deps).ok();
        writeln!(stdout, "  Install order: {:?}", ordered_deps).ok();
        return Ok(());
    }

    // Create target directory.
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("cannot create directory: {}", e))?;

    // Generate language-specific files.
    let io_err = |e: io::Error| format!("file generation error: {}", e);
    match lang {
        "python" => generate_python(
            &target_dir,
            &cfg.package_name,
            &cfg.description,
            &layer_ctx,
            &cfg.direct_deps,
            &ordered_deps,
        )
        .map_err(io_err)?,
        "go" => generate_go(
            &target_dir,
            &cfg.package_name,
            &cfg.description,
            &layer_ctx,
            &cfg.direct_deps,
            &all_deps,
        )
        .map_err(io_err)?,
        "ruby" => generate_ruby(
            &target_dir,
            &cfg.package_name,
            &cfg.description,
            &layer_ctx,
            &cfg.direct_deps,
            &all_deps,
        )
        .map_err(io_err)?,
        "typescript" => generate_typescript(
            &target_dir,
            &cfg.package_name,
            &cfg.description,
            &layer_ctx,
            &cfg.direct_deps,
            &ordered_deps,
        )
        .map_err(io_err)?,
        "rust" => generate_rust(
            &target_dir,
            &cfg.package_name,
            &cfg.description,
            &layer_ctx,
            &cfg.direct_deps,
        )
        .map_err(io_err)?,
        "elixir" => generate_elixir(
            &target_dir,
            &cfg.package_name,
            &cfg.description,
            &layer_ctx,
            &cfg.direct_deps,
            &ordered_deps,
        )
        .map_err(io_err)?,
        _ => return Err(format!("unknown language: {}", lang)),
    }

    // Generate common files (README, CHANGELOG).
    generate_common_files(
        &target_dir,
        &cfg.package_name,
        &cfg.description,
        cfg.layer,
        &cfg.direct_deps,
    )
    .map_err(io_err)?;

    writeln!(
        stdout,
        "Created {} package at: {}",
        lang,
        target_dir.display()
    )
    .ok();

    // Language-specific post-generation steps.
    match lang {
        "rust" => {
            match update_rust_workspace(&cfg.repo_root, &cfg.package_name) {
                Ok(()) => {
                    writeln!(
                        stdout,
                        "  Updated code/packages/rust/Cargo.toml workspace members"
                    )
                    .ok();
                }
                Err(e) => {
                    writeln!(stderr, "  WARNING: Could not update Rust workspace: {}", e).ok();
                    writeln!(
                        stderr,
                        "  You must manually add \"{}\" to code/packages/rust/Cargo.toml members",
                        cfg.package_name
                    )
                    .ok();
                }
            }
            writeln!(stdout, "  Run: cargo build --workspace (to verify)").ok();
        }
        "typescript" => {
            writeln!(
                stdout,
                "  Run: cd {} && npm install (to generate package-lock.json)",
                target_dir.display()
            )
            .ok();
        }
        "go" => {
            writeln!(
                stdout,
                "  Run: cd {} && go mod tidy",
                target_dir.display()
            )
            .ok();
            writeln!(
                stdout,
                "  After other packages depend on this, run go mod tidy in those too"
            )
            .ok();
        }
        _ => {}
    }

    Ok(())
}

// =========================================================================
// run -- the testable core
// =========================================================================
//
// The `run` function contains all the logic. It accepts argv as a parameter
// so it can be tested without process-level side effects. It returns an
// exit code (0 for success, 1 for errors).

/// The main entry point logic, separated from `main()` for testability.
///
/// Parses arguments, validates inputs, and dispatches scaffolding for each
/// requested language. Returns 0 on success, 1 on error.
pub fn run(argv: Vec<String>, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let args = match parse_args(argv) {
        Ok(a) => a,
        Err(e) => {
            writeln!(stderr, "scaffold-generator: {}", e).ok();
            return 1;
        }
    };

    // Handle --help
    if args.help {
        writeln!(stdout, "{}", USAGE).ok();
        return 0;
    }

    // Handle --version
    if args.version {
        writeln!(stdout, "{}", VERSION).ok();
        return 0;
    }

    // Validate package name is present.
    let pkg_name = match &args.package_name {
        Some(name) => name.clone(),
        None => {
            writeln!(
                stderr,
                "scaffold-generator: missing required argument PACKAGE_NAME"
            )
            .ok();
            return 1;
        }
    };

    // Validate package name is kebab-case.
    if !is_kebab_case(&pkg_name) {
        writeln!(
            stderr,
            "scaffold-generator: invalid package name {:?} (must be kebab-case: lowercase, digits, hyphens)",
            pkg_name
        )
        .ok();
        return 1;
    }

    // Validate package type.
    if args.pkg_type != "library" && args.pkg_type != "program" {
        writeln!(
            stderr,
            "scaffold-generator: invalid type {:?} (must be 'library' or 'program')",
            args.pkg_type
        )
        .ok();
        return 1;
    }

    // Parse languages.
    let languages: Vec<String> = if args.language == "all" {
        VALID_LANGUAGES.iter().map(|s| s.to_string()).collect()
    } else {
        let mut langs = Vec::new();
        for l in args.language.split(',') {
            let l = l.trim();
            if !VALID_LANGUAGES.contains(&l) {
                writeln!(
                    stderr,
                    "scaffold-generator: unknown language {:?} (valid: {})",
                    l,
                    VALID_LANGUAGES.join(", ")
                )
                .ok();
                return 1;
            }
            langs.push(l.to_string());
        }
        langs
    };

    // Parse dependencies.
    let mut direct_deps: Vec<String> = Vec::new();
    if !args.depends_on.is_empty() {
        for d in args.depends_on.split(',') {
            let d = d.trim();
            if !d.is_empty() {
                if !is_kebab_case(d) {
                    writeln!(
                        stderr,
                        "scaffold-generator: invalid dependency name {:?} (must be kebab-case)",
                        d
                    )
                    .ok();
                    return 1;
                }
                direct_deps.push(d.to_string());
            }
        }
    }

    // Find repo root.
    let repo_root = match find_repo_root() {
        Ok(r) => r,
        Err(e) => {
            writeln!(stderr, "scaffold-generator: {}", e).ok();
            return 1;
        }
    };

    let cfg = ScaffoldConfig {
        package_name: pkg_name,
        pkg_type: args.pkg_type,
        languages,
        direct_deps,
        layer: args.layer,
        description: args.description,
        dry_run: args.dry_run,
        repo_root,
    };

    // Scaffold for each language.
    let mut had_error = false;
    for lang in &cfg.languages {
        if let Err(e) = scaffold(&cfg, lang, stdout, stderr) {
            writeln!(stderr, "scaffold-generator [{}]: {}", lang, e).ok();
            had_error = true;
        }
    }

    if had_error {
        1
    } else {
        0
    }
}
