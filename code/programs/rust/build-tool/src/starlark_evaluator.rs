// Starlark BUILD file evaluator for the Rust build tool.
//
// ==========================================================================
// Chapter 1: Why Starlark BUILD files?
// ==========================================================================
//
// Traditional BUILD files in this monorepo are shell scripts -- each line is
// a command executed sequentially. This works but has limitations:
//
//   - No change detection metadata: the build tool guesses which files
//     matter based on file extensions, not explicit declarations.
//   - No dependency declarations: deps are parsed from language-specific
//     config files (pyproject.toml, go.mod, etc.) with heuristic matching.
//   - No validation: a typo in a BUILD file only surfaces at build time.
//
// Starlark BUILD files solve all three. They are real programs that declare
// targets with explicit srcs, deps, and build metadata. The build tool
// evaluates them using the Rust starlark-interpreter crate and extracts the
// declared targets.
//
// ==========================================================================
// Chapter 2: How Evaluation Works
// ==========================================================================
//
// The evaluation flow mirrors the Go implementation:
//
//  1. Read the BUILD file contents from disk.
//  2. Create a Starlark interpreter with:
//     - An FsResolver rooted at the repo root (for load() statements).
//     - The full lexer -> parser -> compiler -> VM pipeline.
//  3. Execute the BUILD file through the interpreter.
//  4. Extract the `_targets` list from the result's variables.
//  5. Convert each target dict into a `Target` struct.
//
// The `_targets` variable is a convention: rule functions like `py_library()`
// append to a global `_targets` list so the build tool can discover all
// declared targets after evaluation.
//
// ==========================================================================
// Chapter 3: Detecting Starlark vs Shell BUILD Files
// ==========================================================================
//
// The monorepo supports both shell and Starlark BUILD files. We use a simple
// heuristic to distinguish them: examine the first non-comment, non-blank
// line. If it starts with a known Starlark pattern, it is Starlark.
// Otherwise, it is shell.
//
// Starlark indicators:
//   - `load("...")` statements (importing rule definitions)
//   - Known rule function calls: `py_library(`, `go_library(`, etc.
//   - `def ` statements (function definitions)
//
// This heuristic works because shell BUILD files start with commands like
// `pip install`, `go build`, `bundle install`, etc. -- none of which look
// like Starlark function calls.
//
// ==========================================================================
// Chapter 4: Generating Shell Commands from Targets
// ==========================================================================
//
// Once we have a `Target` struct, we need to convert it back into shell
// commands that the executor can run. Each rule type maps to a standard
// set of commands:
//
// | Rule            | Commands                                       |
// |-----------------|------------------------------------------------|
// | py_library      | uv pip install + pytest                        |
// | go_library      | go build + go test + go vet                    |
// | ruby_library    | bundle install + rake test                     |
// | ts_library      | npm install + vitest                           |
// | rust_library    | cargo build + cargo test                       |
// | elixir_library  | mix deps.get + mix test                        |
//
// This table is the same for `_binary` variants of each rule.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

use starlark_interpreter::{
    FsResolver, InterpreterResult, StarlarkInterpreter, StarlarkValue,
};

/// Schema version for the _ctx build context dict.
const CTX_SCHEMA_VERSION: i64 = 1;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single build target declared in a Starlark BUILD file.
///
/// Each call to `py_library()`, `go_library()`, etc. produces one `Target`.
/// The build tool uses targets to determine:
///   - **What to watch**: `srcs` lists source file patterns for change detection.
///   - **What to build first**: `deps` declares dependency ordering.
///   - **How to build**: `rule` determines the shell commands to generate.
///
/// ## Example
///
/// A Starlark BUILD file containing:
///
/// ```text
/// py_library(
///     name = "logic-gates",
///     srcs = ["src/**/*.py"],
///     deps = ["python/boolean-algebra"],
///     test_runner = "pytest",
/// )
/// ```
///
/// Produces a `Target` with:
///   - `rule = "py_library"`
///   - `name = "logic-gates"`
///   - `srcs = ["src/**/*.py"]`
///   - `deps = ["python/boolean-algebra"]`
///   - `test_runner = "pytest"`
///   - `entry_point = ""`
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    /// Rule type: "py_library", "go_binary", "rust_library", etc.
    pub rule: String,
    /// Target name: "logic-gates", "build-tool", etc.
    pub name: String,
    /// Declared source file patterns for change detection.
    pub srcs: Vec<String>,
    /// Dependencies as "language/package-name" strings.
    pub deps: Vec<String>,
    /// Test framework: "pytest", "vitest", "minitest", etc.
    pub test_runner: String,
    /// Binary entry point: "main.py", "src/index.ts", etc.
    pub entry_point: String,
    /// Structured command dicts from cmd.star.
    pub commands: Vec<StarlarkValue>,
}

/// Holds all targets extracted from evaluating a single Starlark BUILD file.
///
/// A BUILD file may declare zero, one, or many targets. For example, a
/// package might declare both a library and a binary target:
///
/// ```text
/// py_library(name = "mylib", ...)
/// py_binary(name = "myapp", deps = ["mylib"], ...)
/// ```
#[derive(Debug, Clone)]
pub struct BuildResult {
    /// The targets declared in the BUILD file, in order of appearance.
    pub targets: Vec<Target>,
}

// ---------------------------------------------------------------------------
// Starlark detection
// ---------------------------------------------------------------------------

/// Known Starlark rule function prefixes.
///
/// These are the rule functions defined in the monorepo's Starlark library
/// rules. If a BUILD file's first significant line starts with one of these,
/// it is a Starlark file. The list covers all six supported languages, each
/// with library and binary variants.
const KNOWN_RULES: &[&str] = &[
    "py_library(",
    "py_binary(",
    "go_library(",
    "go_binary(",
    "ruby_library(",
    "ruby_binary(",
    "ts_library(",
    "ts_binary(",
    "rust_library(",
    "rust_binary(",
    "elixir_library(",
    "elixir_binary(",
];

/// Detect whether a BUILD file contains Starlark code (as opposed to shell).
///
/// We examine the first non-comment, non-blank line and check for Starlark
/// patterns. If none are found, we treat it as a shell BUILD file.
///
/// ## Algorithm
///
/// ```text
/// for each line in content:
///   skip blank lines
///   skip lines starting with '#'
///   if line starts with "load("  -> Starlark
///   if line starts with "def "   -> Starlark
///   if line starts with a known rule name -> Starlark
///   otherwise -> not Starlark (stop checking)
/// ```
///
/// We only check the first significant line because that is sufficient:
/// Starlark files always start with either a `load()` import or a rule call.
/// Shell files start with commands like `pip install` or `go build`.
///
/// ## Examples
///
/// ```
/// use build_tool::starlark_evaluator::is_starlark_build;
///
/// assert!(is_starlark_build("load(\"//rules/python.star\", \"py_library\")\n"));
/// assert!(is_starlark_build("py_library(name = \"foo\")\n"));
/// assert!(is_starlark_build("def my_rule():\n    pass\n"));
/// assert!(!is_starlark_build("python -m pip install .\npytest\n"));
/// assert!(!is_starlark_build("go build ./...\n"));
/// assert!(!is_starlark_build("# comment\n\necho hello\n"));
/// ```
pub fn is_starlark_build(content: &str) -> bool {
    for line in content.lines() {
        let trimmed = line.trim();

        // Skip blank lines and comments -- they don't tell us anything
        // about the file's format.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Check for Starlark-specific patterns.
        if trimmed.starts_with("load(") {
            return true;
        }
        if trimmed.starts_with("def ") {
            return true;
        }
        for rule in KNOWN_RULES {
            if trimmed.starts_with(rule) {
                return true;
            }
        }

        // If we have seen a non-comment, non-blank line that does not match
        // any Starlark pattern, it is probably shell. Stop checking.
        break;
    }

    false
}

// ---------------------------------------------------------------------------
// BUILD file evaluation
// ---------------------------------------------------------------------------

/// Evaluate a Starlark BUILD file and extract its declared targets.
///
/// This is the main entry point for Starlark BUILD file processing. It:
///
///  1. Reads the BUILD file from `build_file_path`.
///  2. Creates a `StarlarkInterpreter` with an `FsResolver` rooted at
///     `repo_root` (so `load()` paths resolve correctly).
///  3. Executes the file through the full interpreter pipeline.
///  4. Extracts the `_targets` variable from the result.
///  5. Converts each target dict into a `Target` struct.
///
/// ## Parameters
///
/// - `build_file_path`: Absolute path to the BUILD file.
/// - `_pkg_dir`: The package directory (reserved for future glob() support).
/// - `repo_root`: The monorepo root, used to resolve `load()` paths.
///
/// ## Returns
///
/// - `Ok(BuildResult)` with the extracted targets on success.
/// - `Err(String)` with a descriptive error message on failure.
///
/// ## Error cases
///
/// - The BUILD file cannot be read (permissions, missing file).
/// - The Starlark code has syntax errors (lexer/parser failure).
/// - The Starlark code has runtime errors (type errors, undefined names).
/// - The `_targets` variable is not a list of dicts.
pub fn evaluate_build_file(
    build_file_path: &Path,
    _pkg_dir: &Path,
    repo_root: &Path,
) -> Result<BuildResult, String> {
    // Step 1: Read the BUILD file contents.
    let content = fs::read_to_string(build_file_path).map_err(|e| {
        format!(
            "reading BUILD file {}: {}",
            build_file_path.display(),
            e
        )
    })?;

    // Ensure the source ends with a newline (parser requirement).
    let source = if content.ends_with('\n') {
        content
    } else {
        format!("{}\n", content)
    };

    // Step 2: Create a file resolver rooted at the repo root.
    //
    // When a BUILD file calls:
    //   load("code/packages/starlark/library-rules/python_library.star", "py_library")
    //
    // The FsResolver joins the label with repo_root to get:
    //   <repo_root>/code/packages/starlark/library-rules/python_library.star
    let resolver = FsResolver::new(repo_root.to_string_lossy().to_string());

    // Step 3: Create the interpreter and execute the BUILD file.
    //
    // We use a max recursion depth of 200 (matching the default). BUILD files
    // rarely have deep call stacks, but loaded .star files might define helper
    // functions that call each other.
    // Build the _ctx dict — the build context injected into every Starlark
    // scope.  See spec 15 for the full schema.
    //
    // OS normalization: Rust's std::env::consts::OS returns "macos" on macOS,
    // but we normalize to "darwin" to match Go's runtime.GOOS convention.
    let os_name = match env::consts::OS {
        "macos" => "darwin",
        other => other,
    };

    use virtual_machine::Value;

    let ctx_value = Value::Dict(vec![
        (Value::Str("version".to_string()), Value::Int(CTX_SCHEMA_VERSION)),
        (Value::Str("os".to_string()), Value::Str(os_name.to_string())),
        (Value::Str("arch".to_string()), Value::Str(env::consts::ARCH.to_string())),
        (Value::Str("cpu_count".to_string()), Value::Int(num_cpus::get() as i64)),
        (Value::Str("ci".to_string()), Value::Bool(!env::var("CI").unwrap_or_default().is_empty())),
        (Value::Str("repo_root".to_string()), Value::Str(repo_root.to_string_lossy().to_string())),
    ]);

    let mut globals = HashMap::new();
    globals.insert("_ctx".to_string(), ctx_value);

    let mut interp = StarlarkInterpreter::new(Some(&resolver), 200)
        .with_globals(globals);
    let result = interp.interpret_source(&source).map_err(|e| {
        format!(
            "evaluating BUILD file {}: {}",
            build_file_path.display(),
            e
        )
    })?;

    // Step 4-5: Extract targets from the result.
    let targets = extract_targets(&result)?;

    Ok(BuildResult { targets })
}

// ---------------------------------------------------------------------------
// Target extraction helpers
// ---------------------------------------------------------------------------

/// Extract `Target` structs from the interpreter result's `_targets` variable.
///
/// The convention is that rule functions (py_library, go_binary, etc.) append
/// target dicts to a global `_targets` list. Each dict has keys:
///
/// | Key          | Type       | Required | Description                    |
/// |--------------|------------|----------|--------------------------------|
/// | rule         | string     | yes      | Rule type (e.g. "py_library")  |
/// | name         | string     | yes      | Target name                    |
/// | srcs         | list[str]  | no       | Source file patterns           |
/// | deps         | list[str]  | no       | Dependencies                   |
/// | test_runner  | string     | no       | Test framework                 |
/// | entry_point  | string     | no       | Binary entry point             |
///
/// If `_targets` is absent, we return an empty list (the BUILD file declared
/// no targets, which is valid for helper-only files).
fn extract_targets(result: &InterpreterResult) -> Result<Vec<Target>, String> {
    let raw_targets = match result.get("_targets") {
        Some(val) => val,
        None => {
            // No _targets variable -- the BUILD file did not declare any
            // targets. This is valid (e.g., a file that only defines helpers).
            return Ok(Vec::new());
        }
    };

    // _targets must be a list. Each element must be a dict.
    let target_list = match raw_targets {
        StarlarkValue::List(items) => items,
        other => {
            return Err(format!(
                "_targets is not a list (got {:?})",
                other
            ));
        }
    };

    let mut targets = Vec::new();
    for (i, item) in target_list.iter().enumerate() {
        let dict = match item {
            StarlarkValue::Dict(pairs) => pairs,
            other => {
                return Err(format!(
                    "_targets[{}] is not a dict (got {:?})",
                    i, other
                ));
            }
        };

        // Convert the list of (key, value) pairs into a HashMap for easy lookup.
        let map = dict_to_hashmap(dict);

        targets.push(Target {
            rule: get_string_field(&map, "rule"),
            name: get_string_field(&map, "name"),
            srcs: get_string_list_field(&map, "srcs"),
            deps: get_string_list_field(&map, "deps"),
            test_runner: get_string_field(&map, "test_runner"),
            entry_point: get_string_field(&map, "entry_point"),
            commands: get_dict_list_field(&map, "commands"),
        });
    }

    Ok(targets)
}

/// Convert a Starlark dict (list of key-value pairs) into a HashMap.
///
/// Starlark dicts are represented as `Vec<(StarlarkValue, StarlarkValue)>`
/// in our interpreter. For target extraction, we only care about string keys,
/// so we convert to `HashMap<String, StarlarkValue>` and skip non-string keys.
fn dict_to_hashmap(pairs: &[(StarlarkValue, StarlarkValue)]) -> HashMap<String, StarlarkValue> {
    pairs
        .iter()
        .filter_map(|(k, v)| {
            if let StarlarkValue::String(key) = k {
                Some((key.clone(), v.clone()))
            } else {
                None // Skip non-string keys (shouldn't happen for targets).
            }
        })
        .collect()
}

/// Safely extract a string value from a dict.
///
/// Returns an empty string if the key is missing or the value is not a string.
/// This is intentionally lenient -- missing optional fields like `test_runner`
/// simply default to empty.
fn get_string_field(map: &HashMap<String, StarlarkValue>, key: &str) -> String {
    match map.get(key) {
        Some(StarlarkValue::String(s)) => s.clone(),
        _ => String::new(),
    }
}

/// Safely extract a list of strings from a dict.
///
/// Returns an empty vec if the key is missing or the value is not a list.
/// Non-string elements within the list are silently skipped.
fn get_string_list_field(map: &HashMap<String, StarlarkValue>, key: &str) -> Vec<String> {
    match map.get(key) {
        Some(StarlarkValue::List(items)) => items
            .iter()
            .filter_map(|item| {
                if let StarlarkValue::String(s) = item {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Safely extract a list of dicts from a dict.
///
/// Returns an empty vec if the key is missing or the value is not a list.
/// Non-dict elements within the list are silently skipped.
fn get_dict_list_field(map: &HashMap<String, StarlarkValue>, key: &str) -> Vec<StarlarkValue> {
    match map.get(key) {
        Some(StarlarkValue::List(items)) => items
            .iter()
            .filter(|item| matches!(item, StarlarkValue::Dict(_)))
            .cloned()
            .collect(),
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Command rendering
// ---------------------------------------------------------------------------

/// Characters that require shell quoting.
const SHELL_META: &str = " \t\"'$`\\|&;()<>!#*?[]{}";

/// Check whether a string needs shell quoting.
fn needs_quoting(s: &str) -> bool {
    s.is_empty() || s.chars().any(|c| SHELL_META.contains(c))
}

/// Quote a single argument for safe shell interpolation.
///
/// Returns the argument unchanged if it contains no special characters.
/// Empty strings become `""`. Strings with special characters are wrapped
/// in double quotes with internal backslashes and double quotes escaped.
fn quote_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }
    if !needs_quoting(arg) {
        return arg.to_string();
    }
    let escaped = arg.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

/// Render a single command dict to a shell-safe string.
///
/// A command dict has the form:
/// ```text
/// {"program": "cargo", "args": ["build", "--release"]}
/// ```
///
/// Returns `Ok("cargo build --release")` or an error if `program` is missing.
pub fn render_command(cmd: &StarlarkValue) -> Result<String, String> {
    let pairs = match cmd {
        StarlarkValue::Dict(pairs) => pairs,
        _ => return Err("command is not a dict".to_string()),
    };

    let map = dict_to_hashmap(pairs);

    let program = match map.get("program") {
        Some(StarlarkValue::String(s)) if !s.is_empty() => s.clone(),
        _ => return Err("command dict missing 'program' key".to_string()),
    };

    let mut parts = vec![quote_arg(&program)];

    if let Some(StarlarkValue::List(args)) = map.get("args") {
        for arg in args {
            if let StarlarkValue::String(s) = arg {
                parts.push(quote_arg(s));
            }
        }
    }

    Ok(parts.join(" "))
}

/// Render a list of command dicts to shell-safe strings.
///
/// Skips entries that are not dicts or that fail to render.
pub fn render_commands(cmds: &[StarlarkValue]) -> Vec<String> {
    cmds.iter()
        .filter_map(|cmd| render_command(cmd).ok())
        .collect()
}

// ---------------------------------------------------------------------------
// Command generation
// ---------------------------------------------------------------------------

/// Convert a `Target` into shell commands that the executor can run.
///
/// This bridges Starlark declarations to actual build/test commands. Each
/// rule type maps to a standard set of commands for its language ecosystem:
///
/// ## Python rules
///
/// Python packages use `uv` for dependency installation (fast, modern pip
/// replacement) and `pytest` for testing (with coverage reporting).
///
/// ```text
/// py_library / py_binary:
///   1. uv pip install --system -e ".[dev]"
///   2. python -m pytest --cov --cov-report=term-missing
/// ```
///
/// If `test_runner` is set to something other than "pytest", we fall back
/// to `unittest`.
///
/// ## Go rules
///
/// Go packages use the standard toolchain: build, test with coverage, vet.
///
/// ```text
/// go_library / go_binary:
///   1. go build ./...
///   2. go test ./... -v -cover
///   3. go vet ./...
/// ```
///
/// ## Ruby rules
///
/// Ruby packages use Bundler for dependency management and Rake for testing.
///
/// ```text
/// ruby_library / ruby_binary:
///   1. bundle install --quiet
///   2. bundle exec rake test
/// ```
///
/// ## TypeScript rules
///
/// TypeScript packages use npm for dependencies and Vitest for testing.
///
/// ```text
/// ts_library / ts_binary:
///   1. npm install --silent
///   2. npx vitest run --coverage
/// ```
///
/// ## Rust rules
///
/// Rust packages use Cargo for everything.
///
/// ```text
/// rust_library / rust_binary:
///   1. cargo build
///   2. cargo test
/// ```
///
/// ## Elixir rules
///
/// Elixir packages use Mix for everything.
///
/// ```text
/// elixir_library / elixir_binary:
///   1. mix deps.get
///   2. mix test --cover
/// ```
pub fn generate_commands(target: &Target) -> Vec<String> {
    match target.rule.as_str() {
        // -- Python --
        "py_library" | "py_binary" => {
            let runner = if target.test_runner.is_empty() {
                "pytest"
            } else {
                &target.test_runner
            };
            if runner == "pytest" {
                vec![
                    r#"uv pip install --system -e ".[dev]""#.to_string(),
                    "python -m pytest --cov --cov-report=term-missing".to_string(),
                ]
            } else {
                vec![
                    r#"uv pip install --system -e ".[dev]""#.to_string(),
                    "python -m unittest discover tests/".to_string(),
                ]
            }
        }

        // -- Go --
        "go_library" | "go_binary" => vec![
            "go build ./...".to_string(),
            "go test ./... -v -cover".to_string(),
            "go vet ./...".to_string(),
        ],

        // -- Ruby --
        "ruby_library" | "ruby_binary" => vec![
            "bundle install --quiet".to_string(),
            "bundle exec rake test".to_string(),
        ],

        // -- TypeScript --
        "ts_library" | "ts_binary" => vec![
            "npm install --silent".to_string(),
            "npx vitest run --coverage".to_string(),
        ],

        // -- Rust --
        "rust_library" | "rust_binary" => vec![
            "cargo build".to_string(),
            "cargo test".to_string(),
        ],

        // -- Elixir --
        "elixir_library" | "elixir_binary" => vec![
            "mix deps.get".to_string(),
            "mix test --cover".to_string(),
        ],

        // -- Unknown rule --
        unknown => vec![format!("echo 'Unknown rule: {}'", unknown)],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
//
// These tests verify:
//   1. Starlark detection heuristic (is_starlark_build)
//   2. Target extraction from InterpreterResult
//   3. Command generation for each rule type
//   4. Edge cases: empty files, missing fields, unknown rules

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // is_starlark_build tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_detects_load_statement() {
        assert!(is_starlark_build(
            r#"load("//rules/python.star", "py_library")"#
        ));
    }

    #[test]
    fn test_detects_load_with_leading_comment() {
        // Comments and blank lines before the load() should not prevent
        // detection.
        let content = "# Build file for logic-gates\n\nload(\"//rules/python.star\", \"py_library\")\n";
        assert!(is_starlark_build(content));
    }

    #[test]
    fn test_detects_known_rule_calls() {
        assert!(is_starlark_build("py_library(name = \"foo\")\n"));
        assert!(is_starlark_build("go_binary(name = \"bar\")\n"));
        assert!(is_starlark_build("rust_library(name = \"baz\")\n"));
        assert!(is_starlark_build("ts_binary(name = \"qux\")\n"));
        assert!(is_starlark_build("ruby_library(name = \"gem\")\n"));
        assert!(is_starlark_build("elixir_binary(name = \"app\")\n"));
    }

    #[test]
    fn test_detects_def_statement() {
        assert!(is_starlark_build("def my_rule(name):\n    pass\n"));
    }

    #[test]
    fn test_rejects_shell_commands() {
        assert!(!is_starlark_build("python -m pip install .\npytest\n"));
        assert!(!is_starlark_build("go build ./...\n"));
        assert!(!is_starlark_build("bundle install\n"));
        assert!(!is_starlark_build("npm install\n"));
        assert!(!is_starlark_build("cargo build\n"));
        assert!(!is_starlark_build("mix deps.get\n"));
    }

    #[test]
    fn test_empty_file_is_not_starlark() {
        assert!(!is_starlark_build(""));
        assert!(!is_starlark_build("   \n\n   \n"));
    }

    #[test]
    fn test_comment_only_file_is_not_starlark() {
        assert!(!is_starlark_build("# just a comment\n# another comment\n"));
    }

    // -----------------------------------------------------------------------
    // extract_targets tests
    // -----------------------------------------------------------------------

    /// Helper to build an InterpreterResult with a _targets list for testing.
    fn make_result_with_targets(targets: Vec<StarlarkValue>) -> InterpreterResult {
        let mut variables = HashMap::new();
        variables.insert(
            "_targets".to_string(),
            StarlarkValue::List(targets),
        );
        InterpreterResult {
            variables,
            output: Vec::new(),
        }
    }

    /// Helper to build a single target dict as a StarlarkValue::Dict.
    fn make_target_dict(
        rule: &str,
        name: &str,
        srcs: Vec<&str>,
        deps: Vec<&str>,
    ) -> StarlarkValue {
        let mut pairs = vec![
            (
                StarlarkValue::String("rule".to_string()),
                StarlarkValue::String(rule.to_string()),
            ),
            (
                StarlarkValue::String("name".to_string()),
                StarlarkValue::String(name.to_string()),
            ),
        ];

        let srcs_list: Vec<StarlarkValue> = srcs
            .into_iter()
            .map(|s| StarlarkValue::String(s.to_string()))
            .collect();
        pairs.push((
            StarlarkValue::String("srcs".to_string()),
            StarlarkValue::List(srcs_list),
        ));

        let deps_list: Vec<StarlarkValue> = deps
            .into_iter()
            .map(|s| StarlarkValue::String(s.to_string()))
            .collect();
        pairs.push((
            StarlarkValue::String("deps".to_string()),
            StarlarkValue::List(deps_list),
        ));

        StarlarkValue::Dict(pairs)
    }

    #[test]
    fn test_extract_single_target() {
        let dict = make_target_dict(
            "py_library",
            "logic-gates",
            vec!["src/**/*.py"],
            vec!["python/boolean-algebra"],
        );
        let result = make_result_with_targets(vec![dict]);
        let targets = extract_targets(&result).unwrap();

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].rule, "py_library");
        assert_eq!(targets[0].name, "logic-gates");
        assert_eq!(targets[0].srcs, vec!["src/**/*.py"]);
        assert_eq!(targets[0].deps, vec!["python/boolean-algebra"]);
        assert!(targets[0].test_runner.is_empty());
        assert!(targets[0].entry_point.is_empty());
    }

    #[test]
    fn test_extract_multiple_targets() {
        let t1 = make_target_dict("py_library", "lib", vec!["src/*.py"], vec![]);
        let t2 = make_target_dict("py_binary", "app", vec!["main.py"], vec!["python/lib"]);
        let result = make_result_with_targets(vec![t1, t2]);
        let targets = extract_targets(&result).unwrap();

        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].name, "lib");
        assert_eq!(targets[1].name, "app");
        assert_eq!(targets[1].deps, vec!["python/lib"]);
    }

    #[test]
    fn test_extract_no_targets_variable() {
        // No _targets variable at all -- valid for helper-only files.
        let result = InterpreterResult {
            variables: HashMap::new(),
            output: Vec::new(),
        };
        let targets = extract_targets(&result).unwrap();
        assert!(targets.is_empty());
    }

    #[test]
    fn test_extract_targets_not_a_list() {
        let mut variables = HashMap::new();
        variables.insert("_targets".to_string(), StarlarkValue::Int(42));
        let result = InterpreterResult {
            variables,
            output: Vec::new(),
        };
        let err = extract_targets(&result).unwrap_err();
        assert!(err.contains("not a list"));
    }

    #[test]
    fn test_extract_target_with_optional_fields() {
        // Create a target dict that includes test_runner and entry_point.
        let pairs = vec![
            (
                StarlarkValue::String("rule".to_string()),
                StarlarkValue::String("py_binary".to_string()),
            ),
            (
                StarlarkValue::String("name".to_string()),
                StarlarkValue::String("myapp".to_string()),
            ),
            (
                StarlarkValue::String("srcs".to_string()),
                StarlarkValue::List(vec![
                    StarlarkValue::String("main.py".to_string()),
                ]),
            ),
            (
                StarlarkValue::String("deps".to_string()),
                StarlarkValue::List(Vec::new()),
            ),
            (
                StarlarkValue::String("test_runner".to_string()),
                StarlarkValue::String("pytest".to_string()),
            ),
            (
                StarlarkValue::String("entry_point".to_string()),
                StarlarkValue::String("main.py".to_string()),
            ),
        ];
        let dict = StarlarkValue::Dict(pairs);
        let result = make_result_with_targets(vec![dict]);
        let targets = extract_targets(&result).unwrap();

        assert_eq!(targets[0].test_runner, "pytest");
        assert_eq!(targets[0].entry_point, "main.py");
    }

    // -----------------------------------------------------------------------
    // generate_commands tests
    // -----------------------------------------------------------------------

    fn make_target(rule: &str) -> Target {
        Target {
            rule: rule.to_string(),
            name: "test-pkg".to_string(),
            srcs: Vec::new(),
            deps: Vec::new(),
            test_runner: String::new(),
            entry_point: String::new(),
            commands: Vec::new(),
        }
    }

    #[test]
    fn test_python_library_commands() {
        let cmds = generate_commands(&make_target("py_library"));
        assert_eq!(cmds.len(), 2);
        assert!(cmds[0].contains("uv pip install"));
        assert!(cmds[1].contains("pytest"));
    }

    #[test]
    fn test_python_binary_commands() {
        let cmds = generate_commands(&make_target("py_binary"));
        assert_eq!(cmds.len(), 2);
        assert!(cmds[0].contains("uv pip install"));
    }

    #[test]
    fn test_python_unittest_runner() {
        let mut target = make_target("py_library");
        target.test_runner = "unittest".to_string();
        let cmds = generate_commands(&target);
        assert!(cmds[1].contains("unittest"));
    }

    #[test]
    fn test_go_commands() {
        let cmds = generate_commands(&make_target("go_library"));
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0], "go build ./...");
        assert!(cmds[1].contains("go test"));
        assert_eq!(cmds[2], "go vet ./...");
    }

    #[test]
    fn test_go_binary_commands() {
        let cmds = generate_commands(&make_target("go_binary"));
        assert_eq!(cmds.len(), 3);
    }

    #[test]
    fn test_ruby_commands() {
        let cmds = generate_commands(&make_target("ruby_library"));
        assert_eq!(cmds.len(), 2);
        assert!(cmds[0].contains("bundle install"));
        assert!(cmds[1].contains("rake test"));
    }

    #[test]
    fn test_typescript_commands() {
        let cmds = generate_commands(&make_target("ts_library"));
        assert_eq!(cmds.len(), 2);
        assert!(cmds[0].contains("npm install"));
        assert!(cmds[1].contains("vitest"));
    }

    #[test]
    fn test_rust_commands() {
        let cmds = generate_commands(&make_target("rust_library"));
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0], "cargo build");
        assert_eq!(cmds[1], "cargo test");
    }

    #[test]
    fn test_elixir_commands() {
        let cmds = generate_commands(&make_target("elixir_library"));
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0], "mix deps.get");
        assert!(cmds[1].contains("mix test"));
    }

    #[test]
    fn test_unknown_rule_echo() {
        let cmds = generate_commands(&make_target("java_library"));
        assert_eq!(cmds.len(), 1);
        assert!(cmds[0].contains("Unknown rule: java_library"));
    }

    // -----------------------------------------------------------------------
    // dict_to_hashmap tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_dict_to_hashmap_basic() {
        let pairs = vec![
            (
                StarlarkValue::String("key1".to_string()),
                StarlarkValue::Int(42),
            ),
            (
                StarlarkValue::String("key2".to_string()),
                StarlarkValue::String("hello".to_string()),
            ),
        ];
        let map = dict_to_hashmap(&pairs);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("key1"), Some(&StarlarkValue::Int(42)));
    }

    #[test]
    fn test_dict_to_hashmap_skips_non_string_keys() {
        let pairs = vec![
            (StarlarkValue::Int(1), StarlarkValue::String("val".to_string())),
            (
                StarlarkValue::String("key".to_string()),
                StarlarkValue::Int(2),
            ),
        ];
        let map = dict_to_hashmap(&pairs);
        // Only the string-keyed entry survives.
        assert_eq!(map.len(), 1);
        assert!(map.contains_key("key"));
    }

    // -----------------------------------------------------------------------
    // get_string_field / get_string_list_field tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_string_field_present() {
        let mut map = HashMap::new();
        map.insert("name".to_string(), StarlarkValue::String("foo".to_string()));
        assert_eq!(get_string_field(&map, "name"), "foo");
    }

    #[test]
    fn test_get_string_field_missing() {
        let map: HashMap<String, StarlarkValue> = HashMap::new();
        assert_eq!(get_string_field(&map, "name"), "");
    }

    #[test]
    fn test_get_string_field_wrong_type() {
        let mut map = HashMap::new();
        map.insert("name".to_string(), StarlarkValue::Int(42));
        assert_eq!(get_string_field(&map, "name"), "");
    }

    #[test]
    fn test_get_string_list_field_present() {
        let mut map = HashMap::new();
        map.insert(
            "srcs".to_string(),
            StarlarkValue::List(vec![
                StarlarkValue::String("a.py".to_string()),
                StarlarkValue::String("b.py".to_string()),
            ]),
        );
        assert_eq!(get_string_list_field(&map, "srcs"), vec!["a.py", "b.py"]);
    }

    #[test]
    fn test_get_string_list_field_empty() {
        let mut map = HashMap::new();
        map.insert("srcs".to_string(), StarlarkValue::List(Vec::new()));
        let result = get_string_list_field(&map, "srcs");
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_string_list_field_missing() {
        let map: HashMap<String, StarlarkValue> = HashMap::new();
        let result = get_string_list_field(&map, "srcs");
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_string_list_field_skips_non_strings() {
        let mut map = HashMap::new();
        map.insert(
            "srcs".to_string(),
            StarlarkValue::List(vec![
                StarlarkValue::String("a.py".to_string()),
                StarlarkValue::Int(42), // This should be skipped.
                StarlarkValue::String("b.py".to_string()),
            ]),
        );
        assert_eq!(
            get_string_list_field(&map, "srcs"),
            vec!["a.py", "b.py"]
        );
    }
}
