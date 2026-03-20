//! # AST-Based Capability Detector for Rust Source Code
//!
//! This module walks a Rust abstract syntax tree (AST) to find patterns
//! that indicate OS-level capability usage. Each detected pattern maps
//! to a capability in the format `category:action:target`.
//!
//! ## How AST Walking Works in Rust
//!
//! The [`syn`] crate parses Rust source code into a typed AST. For example,
//! the code:
//!
//! ```rust,ignore
//! use std::fs::File;
//! let f = File::open("data.txt");
//! ```
//!
//! Produces a tree roughly like:
//!
//! ```text
//! File
//! ├── ItemUse { tree: Path { segments: [std, fs, File] } }
//! └── Local { init: ExprCall {
//!         func: ExprPath { segments: [File, open] },
//!         args: [ExprLit { lit: "data.txt" }]
//!     }}
//! ```
//!
//! We implement [`syn::visit::Visit`] to walk this tree. When we encounter
//! an `ItemUse` with path `std::fs::File`, we record `fs:*:*`. When we
//! encounter `File::open("data.txt")`, we record `fs:read:data.txt`.
//!
//! ## Detection Categories
//!
//! | Category | What it covers              | Example patterns                  |
//! |----------|-----------------------------|-----------------------------------|
//! | `fs`     | Filesystem access           | `std::fs`, `File::open`           |
//! | `net`    | Network access              | `std::net`, `TcpStream::connect`  |
//! | `proc`   | Process execution           | `std::process`, `Command::new`    |
//! | `env`    | Environment variables       | `std::env`, `env::var`            |
//! | `ffi`    | Foreign function interface  | `unsafe`, `extern "C"`, `libc`    |
//!
//! ## Detection Rules
//!
//! There are three kinds of detection:
//!
//! 1. **Use statement detection** — `use std::fs` implies broad filesystem
//!    access. We map well-known module paths to capability categories.
//!
//! 2. **Function/method call detection** — `File::open("x")` tells us
//!    exactly what file is being opened and whether it's a read or write.
//!
//! 3. **Banned construct detection** — `unsafe` blocks and `extern "C"`
//!    declarations indicate potential FFI usage.

use serde::{Deserialize, Serialize};
use syn::spanned::Spanned;
use syn::visit::Visit;

// ── Data Structures ──────────────────────────────────────────────────
//
// A `DetectedCapability` records one instance of OS capability usage
// found in the source code. It carries enough information for both
// human-readable reports and machine-readable JSON output.

/// A single OS capability detected in source code.
///
/// Each detection records *what* was detected (category/action/target),
/// *where* it was detected (file/line), and *how* (evidence string).
///
/// # Fields
///
/// - `category` — The kind of resource: `fs`, `net`, `proc`, `env`, `ffi`
/// - `action` — The operation: `read`, `write`, `connect`, `exec`, `*`, etc.
/// - `target` — The specific resource: `"data.txt"`, `"HOME"`, `"*"`
/// - `file` — The source file path where detection occurred
/// - `line` — The line number (1-based) in the source file
/// - `evidence` — The code pattern that triggered detection (for humans)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedCapability {
    pub category: String,
    pub action: String,
    pub target: String,
    pub file: String,
    pub line: usize,
    pub evidence: String,
}

impl DetectedCapability {
    /// Format as a capability triple string: `category:action:target`
    pub fn as_triple(&self) -> String {
        format!("{}:{}:{}", self.category, self.action, self.target)
    }
}

impl std::fmt::Display for DetectedCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.category, self.action, self.target)
    }
}

// ── Use Statement Mapping ────────────────────────────────────────────
//
// When Rust code imports a module with `use`, the module path tells us
// what kind of OS capability the code *might* use. This mapping is
// conservative: `use std::fs` doesn't mean the code reads files, but
// it *could*. We flag it and let manifest comparison decide.
//
// The function takes the joined path of a use statement (e.g., "std::fs")
// and returns an optional (category, action) pair.

/// Map a use-statement path to a capability category and action.
///
/// # Examples
///
/// ```text
/// "std::fs"           → Some(("fs", "*"))
/// "std::fs::File"     → Some(("fs", "*"))
/// "std::net"          → Some(("net", "*"))
/// "std::process"      → Some(("proc", "exec"))
/// "std::env"          → Some(("env", "*"))
/// "std::os"           → Some(("fs", "*"))
/// "libc"              → Some(("ffi", "*"))
/// "std::collections"  → None  (not a capability)
/// ```
fn use_path_to_capability(path: &str) -> Option<(&'static str, &'static str)> {
    // We check prefixes so that `std::fs::File` matches the `std::fs` rule.
    // Order matters: more specific prefixes should come first if they have
    // different mappings, but in our case all sub-paths of `std::fs` map
    // to `fs:*`.

    // Filesystem access
    if path.starts_with("std::fs") {
        return Some(("fs", "*"));
    }
    // std::io is the I/O foundation — it implies filesystem access
    // because types like BufReader, Write, Read are used with files
    if path.starts_with("std::io") {
        return Some(("fs", "*"));
    }
    // std::os contains OS-specific filesystem extensions
    if path.starts_with("std::os") {
        return Some(("fs", "*"));
    }

    // Network access
    if path.starts_with("std::net") {
        return Some(("net", "*"));
    }

    // Process execution
    if path.starts_with("std::process") {
        return Some(("proc", "exec"));
    }

    // Environment variables
    if path.starts_with("std::env") {
        return Some(("env", "*"));
    }

    // FFI / native code
    if path == "libc" || path.starts_with("libc::") {
        return Some(("ffi", "*"));
    }

    None
}

// ── Function Call Mapping ────────────────────────────────────────────
//
// Beyond use statements, specific function and method calls indicate
// capability usage with more precision. For example, `File::open("x")`
// tells us the code reads a specific file, while `use std::fs` only
// tells us the code *might* access the filesystem.
//
// We match on the last two segments of a call path (e.g., `File::open`)
// because Rust code can use fully-qualified paths (`std::fs::File::open`)
// or short paths (`File::open`) depending on imports.

/// Map a function call path (last two segments) to a capability.
///
/// Returns `Some((category, action))` if the call is capability-bearing.
///
/// # Examples
///
/// ```text
/// ("File", "open")           → Some(("fs", "read"))
/// ("File", "create")         → Some(("fs", "write"))
/// ("fs", "read_to_string")   → Some(("fs", "read"))
/// ("TcpStream", "connect")   → Some(("net", "connect"))
/// ("Command", "new")         → Some(("proc", "exec"))
/// ("env", "var")             → Some(("env", "read"))
/// ("String", "from")         → None
/// ```
fn call_path_to_capability(type_name: &str, method_name: &str) -> Option<(&'static str, &'static str)> {
    match (type_name, method_name) {
        // ── Filesystem: File operations ──
        //
        // `File::open` opens a file for reading.
        // `File::create` creates (or truncates) a file for writing.
        ("File", "open") => Some(("fs", "read")),
        ("File", "create") => Some(("fs", "write")),

        // ── Filesystem: fs module free functions ──
        //
        // These are convenience functions in `std::fs` that combine
        // open + read/write into a single call.
        ("fs", "read_to_string") => Some(("fs", "read")),
        ("fs", "read") => Some(("fs", "read")),
        ("fs", "write") => Some(("fs", "write")),
        ("fs", "remove_file") => Some(("fs", "delete")),
        ("fs", "remove_dir") => Some(("fs", "delete")),
        ("fs", "remove_dir_all") => Some(("fs", "delete")),
        ("fs", "create_dir") => Some(("fs", "create")),
        ("fs", "create_dir_all") => Some(("fs", "create")),
        ("fs", "read_dir") => Some(("fs", "list")),
        ("fs", "rename") => Some(("fs", "write")),
        ("fs", "copy") => Some(("fs", "write")),
        ("fs", "metadata") => Some(("fs", "read")),
        ("fs", "symlink_metadata") => Some(("fs", "read")),
        ("fs", "canonicalize") => Some(("fs", "read")),

        // ── Network: TCP ──
        //
        // `TcpStream::connect` initiates an outbound TCP connection.
        // `TcpListener::bind` listens for inbound TCP connections.
        ("TcpStream", "connect") => Some(("net", "connect")),
        ("TcpListener", "bind") => Some(("net", "listen")),

        // ── Network: UDP ──
        //
        // `UdpSocket::bind` binds a UDP socket to a local address.
        // `UdpSocket::send_to` sends data to a remote address.
        ("UdpSocket", "bind") => Some(("net", "listen")),
        ("UdpSocket", "connect") => Some(("net", "connect")),

        // ── Process execution ──
        //
        // `Command::new("ls")` creates a new process builder.
        // The first argument is the program to execute.
        ("Command", "new") => Some(("proc", "exec")),

        // ── Environment variables ──
        //
        // `env::var("KEY")` reads an environment variable.
        // `env::set_var("KEY", "VALUE")` sets one.
        // `env::remove_var("KEY")` removes one.
        ("env", "var") => Some(("env", "read")),
        ("env", "var_os") => Some(("env", "read")),
        ("env", "set_var") => Some(("env", "write")),
        ("env", "remove_var") => Some(("env", "write")),

        // ── FFI: memory transmute ──
        //
        // `mem::transmute` reinterprets memory — a sign of FFI or
        // unsafe low-level code.
        ("mem", "transmute") => Some(("ffi", "*")),

        _ => None,
    }
}

// ── AST Visitor ──────────────────────────────────────────────────────
//
// The `CapabilityVisitor` implements `syn::visit::Visit` to walk the
// entire AST of a Rust source file. For each node type we care about,
// we check if it matches a capability pattern and record it.
//
// The visitor pattern works like this:
//
// 1. `syn::parse_file(source)` produces a `syn::File` (the root node)
// 2. We call `visit::visit_file(&mut visitor, &file)`
// 3. The visitor recursively walks every node, calling our `visit_*`
//    methods when it encounters relevant node types
// 4. After the walk, `visitor.detected` contains all capabilities

/// AST visitor that detects OS capability usage in Rust source code.
///
/// Implements [`syn::visit::Visit`] to walk the parsed AST. After
/// visiting, the `detected` field contains all found capabilities.
///
/// # Usage
///
/// ```rust,ignore
/// let file = syn::parse_file(&source)?;
/// let mut visitor = CapabilityVisitor::new("path/to/file.rs");
/// syn::visit::visit_file(&mut visitor, &file);
/// let capabilities = visitor.detected;
/// ```
pub struct CapabilityVisitor {
    /// The filename being analyzed (for reporting)
    filename: String,
    /// Accumulated detected capabilities
    pub detected: Vec<DetectedCapability>,
}

impl CapabilityVisitor {
    /// Create a new visitor for the given filename.
    pub fn new(filename: &str) -> Self {
        Self {
            filename: filename.to_string(),
            detected: Vec::new(),
        }
    }

    /// Record a detected capability.
    fn add(
        &mut self,
        category: &str,
        action: &str,
        target: &str,
        line: usize,
        evidence: &str,
    ) {
        self.detected.push(DetectedCapability {
            category: category.to_string(),
            action: action.to_string(),
            target: target.to_string(),
            file: self.filename.clone(),
            line,
            evidence: evidence.to_string(),
        });
    }

    // ── Helper: Extract string literal from an expression ────────────
    //
    // Many capability-bearing functions take a string argument that tells
    // us the *target* (filename, hostname, env var name, etc.). If the
    // argument is a string literal, we can record the exact target.
    // If it's a variable or expression, we fall back to "*".

    /// Try to extract a string literal value from an expression.
    ///
    /// Returns `Some("the string")` for `ExprLit` nodes containing a
    /// string literal, `None` for everything else.
    fn extract_string_lit(expr: &syn::Expr) -> Option<String> {
        if let syn::Expr::Lit(expr_lit) = expr {
            if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                return Some(lit_str.value());
            }
        }
        None
    }

    // ── Helper: Extract path segments from an expression ─────────────
    //
    // A function call like `File::open(...)` has a function expression
    // that is an `ExprPath` with segments `[File, open]`. We need to
    // extract these segments to match against our capability rules.

    /// Extract the last two path segments from an expression path.
    ///
    /// Returns `Some(("Type", "method"))` for paths like `File::open`,
    /// `std::fs::File::open`, etc. Returns `None` if the path has
    /// fewer than two segments.
    fn extract_call_pair(path: &syn::Path) -> Option<(String, String)> {
        let segments: Vec<_> = path.segments.iter().collect();
        if segments.len() >= 2 {
            let type_seg = &segments[segments.len() - 2];
            let method_seg = &segments[segments.len() - 1];
            Some((
                type_seg.ident.to_string(),
                method_seg.ident.to_string(),
            ))
        } else {
            None
        }
    }

    // ── Helper: Join path segments into a string ─────────────────────
    //
    // For use statements like `use std::fs::File`, we need the full
    // path as a string "std::fs::File" to match against our rules.

    /// Join all segments of a `syn::Path` into a `::` separated string.
    fn path_to_string(path: &syn::Path) -> String {
        path.segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    }

    // ── Helper: Recursively collect paths from a use tree ────────────
    //
    // Rust use statements can be nested with braces:
    //   `use std::fs::{File, read_to_string};`
    // We need to flatten these into individual paths:
    //   ["std::fs::File", "std::fs::read_to_string"]

    /// Recursively collect all leaf paths from a use tree.
    ///
    /// # Examples
    ///
    /// ```text
    /// use std::fs;              → ["std::fs"]
    /// use std::fs::File;        → ["std::fs::File"]
    /// use std::fs::{File, read}; → ["std::fs::File", "std::fs::read"]
    /// use std::fs::*;           → ["std::fs"]
    /// ```
    fn collect_use_paths(prefix: &str, tree: &syn::UseTree) -> Vec<String> {
        match tree {
            // `use std::fs::File;` — a simple path leaf
            syn::UseTree::Path(use_path) => {
                let new_prefix = if prefix.is_empty() {
                    use_path.ident.to_string()
                } else {
                    format!("{}::{}", prefix, use_path.ident)
                };
                Self::collect_use_paths(&new_prefix, &use_path.tree)
            }

            // `use std::fs::File;` — the final name
            syn::UseTree::Name(use_name) => {
                let full = if prefix.is_empty() {
                    use_name.ident.to_string()
                } else {
                    format!("{}::{}", prefix, use_name.ident)
                };
                vec![full]
            }

            // `use std::fs::File as F;` — rename
            syn::UseTree::Rename(use_rename) => {
                let full = if prefix.is_empty() {
                    use_rename.ident.to_string()
                } else {
                    format!("{}::{}", prefix, use_rename.ident)
                };
                vec![full]
            }

            // `use std::fs::*;` — glob import, treat as parent path
            syn::UseTree::Glob(_) => {
                if prefix.is_empty() {
                    vec![]
                } else {
                    vec![prefix.to_string()]
                }
            }

            // `use std::fs::{File, read_to_string};` — group
            syn::UseTree::Group(use_group) => {
                let mut paths = Vec::new();
                for item in &use_group.items {
                    paths.extend(Self::collect_use_paths(prefix, item));
                }
                paths
            }
        }
    }
}

// ── Visit Implementation ─────────────────────────────────────────────
//
// Each `visit_*` method handles one type of AST node. The `syn::visit`
// framework calls these methods as it walks the tree. We must call
// `visit::visit_*` at the end of each method to continue the recursive
// walk into child nodes.

impl<'ast> Visit<'ast> for CapabilityVisitor {
    // ── Use statements ───────────────────────────────────────────────
    //
    // `use std::fs::File;` creates an `ItemUse` node. We extract all
    // leaf paths from the use tree and check each against our mapping.

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        let paths = Self::collect_use_paths("", &node.tree);
        let line = node.span().start().line;

        for path in &paths {
            if let Some((category, action)) = use_path_to_capability(path) {
                self.add(
                    category,
                    action,
                    "*",
                    line,
                    &format!("use {}", path),
                );
            }
        }

        syn::visit::visit_item_use(self, node);
    }

    // ── Function calls (free functions and associated functions) ──────
    //
    // `File::open("x")` or `fs::read_to_string("x")` create `ExprCall`
    // nodes whose `func` is an `ExprPath`. We extract the last two
    // segments of the path and check against our call mapping.

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(expr_path) = &*node.func {
            if let Some((type_name, method_name)) = Self::extract_call_pair(&expr_path.path) {
                if let Some((category, action)) = call_path_to_capability(&type_name, &method_name) {
                    let line = node.span().start().line;

                    // Try to extract the target from the first argument
                    let target = node
                        .args
                        .first()
                        .and_then(Self::extract_string_lit)
                        .unwrap_or_else(|| "*".to_string());

                    let evidence = if target == "*" {
                        format!("{}::{}(...)", type_name, method_name)
                    } else {
                        format!("{}::{}({:?})", type_name, method_name, target)
                    };

                    self.add(category, action, &target, line, &evidence);
                }
            }
        }

        // Continue walking into child nodes (the arguments, etc.)
        syn::visit::visit_expr_call(self, node);
    }

    // ── Unsafe blocks ────────────────────────────────────────────────
    //
    // `unsafe { ... }` indicates code that bypasses Rust's safety
    // guarantees. This is a strong signal of FFI or low-level memory
    // manipulation. We flag it as `ffi:*:*`.
    //
    // Note: We also detect `unsafe` here to complement Clippy. Clippy
    // has `#[deny(unsafe_code)]` but our analyzer records it as a
    // *capability* for manifest comparison purposes.

    fn visit_expr_unsafe(&mut self, node: &'ast syn::ExprUnsafe) {
        let line = node.span().start().line;
        self.add("ffi", "*", "*", line, "unsafe block");

        syn::visit::visit_expr_unsafe(self, node);
    }

    // ── Extern blocks ────────────────────────────────────────────────
    //
    // `extern "C" { fn foo(); }` declares foreign functions. This is
    // direct FFI usage.

    fn visit_item_foreign_mod(&mut self, node: &'ast syn::ItemForeignMod) {
        let line = node.span().start().line;
        let abi = node
            .abi
            .name
            .as_ref()
            .map(|s| s.value())
            .unwrap_or_else(|| "C".to_string());
        self.add("ffi", "*", "*", line, &format!("extern \"{}\" block", abi));

        syn::visit::visit_item_foreign_mod(self, node);
    }

    // ── Macro invocations ────────────────────────────────────────────
    //
    // `include_bytes!("path")` and `include_str!("path")` embed file
    // contents at compile time. They read from the filesystem during
    // compilation.

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let macro_name = Self::path_to_string(&node.path);
        let line = node.span().start().line;

        if macro_name == "include_bytes" || macro_name == "include_str" {
            // Try to extract the path argument from the macro tokens.
            // The tokens are the raw token stream inside the macro parens.
            // For `include_bytes!("data.bin")`, the tokens contain a
            // string literal "data.bin".
            let tokens_str = node.tokens.to_string();
            let target = extract_macro_string_arg(&tokens_str);

            self.add(
                "fs",
                "read",
                &target,
                line,
                &format!("{}!({})", macro_name, if target == "*" { "..." } else { &target }),
            );
        }

        syn::visit::visit_macro(self, node);
    }
}

// ── Helper: Extract string from macro tokens ─────────────────────────
//
// Macro tokens are unparsed token streams. For simple cases like
// `include_bytes!("path/to/file")`, the token stream is just a
// string literal in quotes. We do a simple extraction here.

/// Extract a string literal from macro token text.
///
/// Given `"\"data.bin\""`, returns `"data.bin"`.
/// Given anything else, returns `"*"`.
fn extract_macro_string_arg(tokens: &str) -> String {
    let trimmed = tokens.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        // Remove surrounding quotes
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        "*".to_string()
    }
}

// ── Public API ───────────────────────────────────────────────────────

/// Analyze a single Rust source file for capability usage.
///
/// Parses the file with `syn::parse_file` and walks the AST to detect
/// capability-bearing patterns.
///
/// # Arguments
///
/// * `filepath` — Path to the `.rs` file to analyze
///
/// # Returns
///
/// A vector of detected capabilities, or an error if the file can't
/// be read or parsed.
///
/// # Errors
///
/// Returns an error string if:
/// - The file cannot be read (I/O error)
/// - The file contains invalid Rust syntax (parse error)
pub fn analyze_file(filepath: &str) -> Result<Vec<DetectedCapability>, String> {
    let source = std::fs::read_to_string(filepath)
        .map_err(|e| format!("Failed to read {}: {}", filepath, e))?;

    analyze_source(&source, filepath)
}

/// Analyze Rust source code provided as a string.
///
/// This is useful for testing (pass source directly) and for the
/// file-based API (which reads the file then calls this).
///
/// # Arguments
///
/// * `source` — The Rust source code to analyze
/// * `filename` — The filename to report in detected capabilities
///
/// # Returns
///
/// A vector of detected capabilities, or an error if parsing fails.
pub fn analyze_source(source: &str, filename: &str) -> Result<Vec<DetectedCapability>, String> {
    let file = syn::parse_file(source)
        .map_err(|e| format!("Failed to parse {}: {}", filename, e))?;

    let mut visitor = CapabilityVisitor::new(filename);
    syn::visit::visit_file(&mut visitor, &file);

    Ok(visitor.detected)
}

/// Analyze all `.rs` files in a directory tree.
///
/// Walks the directory recursively, parsing each `.rs` file and
/// collecting all detected capabilities.
///
/// # Arguments
///
/// * `directory` — Root directory to analyze
/// * `exclude_tests` — If true, skip files in `tests/` directories
///
/// # Returns
///
/// A vector of all detected capabilities across all files.
pub fn analyze_directory(
    directory: &str,
    exclude_tests: bool,
) -> Result<Vec<DetectedCapability>, String> {
    let dir_path = std::path::Path::new(directory);
    if !dir_path.is_dir() {
        return Err(format!("{} is not a directory", directory));
    }

    let mut all_detected = Vec::new();

    // Skip directories that aren't source code
    let skip_dirs = [".git", "target", "node_modules", ".venv"];

    fn walk_dir(
        dir: &std::path::Path,
        skip_dirs: &[&str],
        exclude_tests: bool,
        all_detected: &mut Vec<DetectedCapability>,
    ) -> Result<(), String> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Directory entry error: {}", e))?;
            let path = entry.path();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();

            if path.is_dir() {
                // Skip excluded directories
                if skip_dirs.iter().any(|&s| file_name == s) {
                    continue;
                }
                if exclude_tests && (file_name == "tests" || file_name == "test") {
                    continue;
                }
                walk_dir(&path, skip_dirs, exclude_tests, all_detected)?;
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let filepath = path.to_string_lossy().to_string();
                match analyze_file(&filepath) {
                    Ok(detected) => all_detected.extend(detected),
                    Err(_) => {} // Skip files that can't be parsed
                }
            }
        }
        Ok(())
    }

    walk_dir(dir_path, &skip_dirs, exclude_tests, &mut all_detected)?;
    Ok(all_detected)
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper: analyze a source snippet and return capabilities ──

    fn analyze(source: &str) -> Vec<DetectedCapability> {
        analyze_source(source, "test.rs").expect("Failed to parse test source")
    }

    // ── Use statement detection ──────────────────────────────────────

    #[test]
    fn test_use_std_fs() {
        let caps = analyze("use std::fs;");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "fs");
        assert_eq!(caps[0].action, "*");
        assert_eq!(caps[0].evidence, "use std::fs");
    }

    #[test]
    fn test_use_std_fs_file() {
        let caps = analyze("use std::fs::File;");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "fs");
        assert_eq!(caps[0].action, "*");
        assert_eq!(caps[0].evidence, "use std::fs::File");
    }

    #[test]
    fn test_use_std_io() {
        let caps = analyze("use std::io;");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "fs");
        assert_eq!(caps[0].action, "*");
    }

    #[test]
    fn test_use_std_io_bufread() {
        let caps = analyze("use std::io::BufRead;");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "fs");
    }

    #[test]
    fn test_use_std_net() {
        let caps = analyze("use std::net;");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "net");
        assert_eq!(caps[0].action, "*");
    }

    #[test]
    fn test_use_std_net_tcpstream() {
        let caps = analyze("use std::net::TcpStream;");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "net");
    }

    #[test]
    fn test_use_std_process() {
        let caps = analyze("use std::process;");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "proc");
        assert_eq!(caps[0].action, "exec");
    }

    #[test]
    fn test_use_std_process_command() {
        let caps = analyze("use std::process::Command;");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "proc");
    }

    #[test]
    fn test_use_std_env() {
        let caps = analyze("use std::env;");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "env");
        assert_eq!(caps[0].action, "*");
    }

    #[test]
    fn test_use_std_os() {
        let caps = analyze("use std::os;");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "fs");
    }

    #[test]
    fn test_use_libc() {
        let caps = analyze("use libc;");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "ffi");
        assert_eq!(caps[0].action, "*");
    }

    #[test]
    fn test_use_libc_submodule() {
        let caps = analyze("use libc::c_int;");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "ffi");
    }

    #[test]
    fn test_use_no_capability() {
        // Importing standard library modules that aren't capability-bearing
        let caps = analyze("use std::collections::HashMap;");
        assert!(caps.is_empty());
    }

    #[test]
    fn test_use_group() {
        // `use std::fs::{File, read_to_string};` should detect fs capability
        let caps = analyze("use std::fs::{File, read_to_string};");
        // Each leaf path in the group generates a detection
        assert_eq!(caps.len(), 2);
        assert!(caps.iter().all(|c| c.category == "fs"));
    }

    #[test]
    fn test_use_glob() {
        let caps = analyze("use std::fs::*;");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "fs");
    }

    #[test]
    fn test_use_rename() {
        let caps = analyze("use std::fs::File as F;");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "fs");
    }

    #[test]
    fn test_multiple_use_statements() {
        let caps = analyze(
            "use std::fs;\nuse std::net;\nuse std::process;\n",
        );
        assert_eq!(caps.len(), 3);
        let categories: Vec<_> = caps.iter().map(|c| c.category.as_str()).collect();
        assert!(categories.contains(&"fs"));
        assert!(categories.contains(&"net"));
        assert!(categories.contains(&"proc"));
    }

    // ── Function call detection ──────────────────────────────────────

    #[test]
    fn test_file_open_with_literal() {
        let caps = analyze(
            r#"
            fn main() {
                let f = File::open("data.txt");
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "fs");
        assert_eq!(caps[0].action, "read");
        assert_eq!(caps[0].target, "data.txt");
    }

    #[test]
    fn test_file_open_with_variable() {
        let caps = analyze(
            r#"
            fn main() {
                let path = "data.txt";
                let f = File::open(path);
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "fs");
        assert_eq!(caps[0].action, "read");
        assert_eq!(caps[0].target, "*"); // Can't resolve variable statically
    }

    #[test]
    fn test_file_create() {
        let caps = analyze(
            r#"
            fn main() {
                let f = File::create("output.txt");
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "fs");
        assert_eq!(caps[0].action, "write");
        assert_eq!(caps[0].target, "output.txt");
    }

    #[test]
    fn test_fs_read_to_string() {
        let caps = analyze(
            r#"
            fn main() {
                let s = fs::read_to_string("config.toml");
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "fs");
        assert_eq!(caps[0].action, "read");
        assert_eq!(caps[0].target, "config.toml");
    }

    #[test]
    fn test_fs_write() {
        let caps = analyze(
            r#"
            fn main() {
                fs::write("out.txt", "hello");
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "fs");
        assert_eq!(caps[0].action, "write");
        assert_eq!(caps[0].target, "out.txt");
    }

    #[test]
    fn test_fs_remove_file() {
        let caps = analyze(
            r#"
            fn main() {
                fs::remove_file("tmp.txt");
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "fs");
        assert_eq!(caps[0].action, "delete");
    }

    #[test]
    fn test_fs_create_dir() {
        let caps = analyze(
            r#"
            fn main() {
                fs::create_dir("new_dir");
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "fs");
        assert_eq!(caps[0].action, "create");
    }

    #[test]
    fn test_fs_read_dir() {
        let caps = analyze(
            r#"
            fn main() {
                fs::read_dir(".");
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "fs");
        assert_eq!(caps[0].action, "list");
    }

    #[test]
    fn test_tcpstream_connect() {
        let caps = analyze(
            r#"
            fn main() {
                let s = TcpStream::connect("127.0.0.1:8080");
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "net");
        assert_eq!(caps[0].action, "connect");
        assert_eq!(caps[0].target, "127.0.0.1:8080");
    }

    #[test]
    fn test_tcplistener_bind() {
        let caps = analyze(
            r#"
            fn main() {
                let l = TcpListener::bind("0.0.0.0:9090");
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "net");
        assert_eq!(caps[0].action, "listen");
    }

    #[test]
    fn test_udpsocket_bind() {
        let caps = analyze(
            r#"
            fn main() {
                let s = UdpSocket::bind("0.0.0.0:5000");
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "net");
        assert_eq!(caps[0].action, "listen");
    }

    #[test]
    fn test_command_new() {
        let caps = analyze(
            r#"
            fn main() {
                Command::new("ls");
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "proc");
        assert_eq!(caps[0].action, "exec");
        assert_eq!(caps[0].target, "ls");
    }

    #[test]
    fn test_command_new_variable() {
        let caps = analyze(
            r#"
            fn main() {
                let cmd = get_command();
                Command::new(cmd);
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].target, "*");
    }

    #[test]
    fn test_env_var() {
        let caps = analyze(
            r#"
            fn main() {
                let home = env::var("HOME");
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "env");
        assert_eq!(caps[0].action, "read");
        assert_eq!(caps[0].target, "HOME");
    }

    #[test]
    fn test_env_set_var() {
        let caps = analyze(
            r#"
            fn main() {
                env::set_var("MY_KEY", "value");
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "env");
        assert_eq!(caps[0].action, "write");
        assert_eq!(caps[0].target, "MY_KEY");
    }

    #[test]
    fn test_env_remove_var() {
        let caps = analyze(
            r#"
            fn main() {
                env::remove_var("OLD_KEY");
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "env");
        assert_eq!(caps[0].action, "write");
    }

    #[test]
    fn test_mem_transmute() {
        let caps = analyze(
            r#"
            fn main() {
                let x: u32 = mem::transmute(1.0f32);
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "ffi");
    }

    #[test]
    fn test_fully_qualified_path() {
        // `std::fs::File::open("x")` — fully qualified path should still
        // match because we look at the last two segments
        let caps = analyze(
            r#"
            fn main() {
                let f = std::fs::File::open("data.txt");
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "fs");
        assert_eq!(caps[0].action, "read");
        assert_eq!(caps[0].target, "data.txt");
    }

    // ── Banned construct detection ───────────────────────────────────

    #[test]
    fn test_unsafe_block() {
        let caps = analyze(
            r#"
            fn main() {
                unsafe {
                    let ptr = std::ptr::null::<u8>();
                }
            }
            "#,
        );
        assert!(caps.iter().any(|c| c.category == "ffi" && c.evidence == "unsafe block"));
    }

    #[test]
    fn test_extern_c_block() {
        let caps = analyze(
            r#"
            extern "C" {
                fn puts(s: *const i8) -> i32;
            }
            "#,
        );
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].category, "ffi");
        assert!(caps[0].evidence.contains("extern"));
    }

    #[test]
    fn test_include_bytes() {
        let caps = analyze(
            r#"
            fn main() {
                let data = include_bytes!("data.bin");
            }
            "#,
        );
        assert!(caps.iter().any(|c| c.category == "fs" && c.action == "read"));
    }

    #[test]
    fn test_include_str() {
        let caps = analyze(
            r#"
            fn main() {
                let text = include_str!("template.txt");
            }
            "#,
        );
        assert!(caps.iter().any(|c| c.category == "fs" && c.action == "read"));
    }

    // ── Pure code detection ──────────────────────────────────────────

    #[test]
    fn test_pure_code_no_capabilities() {
        let caps = analyze(
            r#"
            /// A pure function that adds two numbers.
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }

            fn main() {
                let result = add(2, 3);
                assert_eq!(result, 5);
            }
            "#,
        );
        assert!(caps.is_empty(), "Pure code should detect zero capabilities");
    }

    #[test]
    fn test_pure_struct_and_impl() {
        let caps = analyze(
            r#"
            struct Point {
                x: f64,
                y: f64,
            }

            impl Point {
                fn distance(&self, other: &Point) -> f64 {
                    ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
                }
            }
            "#,
        );
        assert!(caps.is_empty());
    }

    #[test]
    fn test_pure_enum_and_match() {
        let caps = analyze(
            r#"
            enum Color {
                Red,
                Green,
                Blue,
            }

            fn name(c: &Color) -> &str {
                match c {
                    Color::Red => "red",
                    Color::Green => "green",
                    Color::Blue => "blue",
                }
            }
            "#,
        );
        assert!(caps.is_empty());
    }

    #[test]
    fn test_collections_no_capability() {
        let caps = analyze(
            r#"
            use std::collections::HashMap;

            fn main() {
                let mut map = HashMap::new();
                map.insert("key", 42);
            }
            "#,
        );
        assert!(caps.is_empty());
    }

    // ── Line number accuracy ─────────────────────────────────────────

    #[test]
    fn test_line_numbers_use_statement() {
        let caps = analyze("use std::fs;\n");
        assert_eq!(caps[0].line, 1);
    }

    #[test]
    fn test_line_numbers_call_on_line_3() {
        let caps = analyze(
            r#"
fn main() {
    File::open("x");
}
"#,
        );
        assert_eq!(caps[0].line, 3);
    }

    #[test]
    fn test_line_numbers_multiple() {
        let caps = analyze(
            r#"use std::fs;
use std::net;

fn main() {
    File::open("x");
    TcpStream::connect("host:80");
}
"#,
        );
        assert_eq!(caps.len(), 4); // 2 use + 2 calls
        // Use statements on lines 1 and 2
        assert_eq!(caps[0].line, 1);
        assert_eq!(caps[1].line, 2);
        // Calls on lines 5 and 6
        assert_eq!(caps[2].line, 5);
        assert_eq!(caps[3].line, 6);
    }

    // ── Edge cases ───────────────────────────────────────────────────

    #[test]
    fn test_empty_file() {
        let caps = analyze("");
        assert!(caps.is_empty());
    }

    #[test]
    fn test_comments_only() {
        let caps = analyze(
            r#"
            // This is a comment about File::open
            /* use std::fs; */
            "#,
        );
        assert!(caps.is_empty());
    }

    #[test]
    fn test_mixed_capabilities() {
        let caps = analyze(
            r#"
            use std::fs::File;
            use std::net::TcpStream;
            use std::process::Command;
            use std::env;

            fn main() {
                File::open("config.toml");
                TcpStream::connect("api.example.com:443");
                Command::new("curl");
                env::var("API_KEY");
            }
            "#,
        );
        // 4 use statements + 4 calls = 8 detections
        assert_eq!(caps.len(), 8);

        let categories: Vec<_> = caps.iter().map(|c| c.category.as_str()).collect();
        assert!(categories.contains(&"fs"));
        assert!(categories.contains(&"net"));
        assert!(categories.contains(&"proc"));
        assert!(categories.contains(&"env"));
    }

    // ── extract_macro_string_arg tests ───────────────────────────────

    #[test]
    fn test_extract_macro_string_arg_simple() {
        assert_eq!(extract_macro_string_arg("\"hello.txt\""), "hello.txt");
    }

    #[test]
    fn test_extract_macro_string_arg_not_string() {
        assert_eq!(extract_macro_string_arg("SOME_CONST"), "*");
    }

    #[test]
    fn test_extract_macro_string_arg_empty() {
        assert_eq!(extract_macro_string_arg(""), "*");
    }

    // ── use_path_to_capability tests ─────────────────────────────────

    #[test]
    fn test_use_path_mapping_comprehensive() {
        assert_eq!(use_path_to_capability("std::fs"), Some(("fs", "*")));
        assert_eq!(use_path_to_capability("std::fs::File"), Some(("fs", "*")));
        assert_eq!(use_path_to_capability("std::io"), Some(("fs", "*")));
        assert_eq!(use_path_to_capability("std::net"), Some(("net", "*")));
        assert_eq!(use_path_to_capability("std::net::TcpStream"), Some(("net", "*")));
        assert_eq!(use_path_to_capability("std::process"), Some(("proc", "exec")));
        assert_eq!(use_path_to_capability("std::env"), Some(("env", "*")));
        assert_eq!(use_path_to_capability("std::os"), Some(("fs", "*")));
        assert_eq!(use_path_to_capability("libc"), Some(("ffi", "*")));
        assert_eq!(use_path_to_capability("libc::c_int"), Some(("ffi", "*")));
        assert_eq!(use_path_to_capability("std::collections"), None);
        assert_eq!(use_path_to_capability("serde"), None);
    }

    // ── call_path_to_capability tests ────────────────────────────────

    #[test]
    fn test_call_path_mapping_comprehensive() {
        assert_eq!(call_path_to_capability("File", "open"), Some(("fs", "read")));
        assert_eq!(call_path_to_capability("File", "create"), Some(("fs", "write")));
        assert_eq!(call_path_to_capability("fs", "read_to_string"), Some(("fs", "read")));
        assert_eq!(call_path_to_capability("fs", "write"), Some(("fs", "write")));
        assert_eq!(call_path_to_capability("fs", "remove_file"), Some(("fs", "delete")));
        assert_eq!(call_path_to_capability("fs", "create_dir"), Some(("fs", "create")));
        assert_eq!(call_path_to_capability("fs", "create_dir_all"), Some(("fs", "create")));
        assert_eq!(call_path_to_capability("fs", "read_dir"), Some(("fs", "list")));
        assert_eq!(call_path_to_capability("fs", "rename"), Some(("fs", "write")));
        assert_eq!(call_path_to_capability("fs", "copy"), Some(("fs", "write")));
        assert_eq!(call_path_to_capability("fs", "metadata"), Some(("fs", "read")));
        assert_eq!(call_path_to_capability("TcpStream", "connect"), Some(("net", "connect")));
        assert_eq!(call_path_to_capability("TcpListener", "bind"), Some(("net", "listen")));
        assert_eq!(call_path_to_capability("UdpSocket", "bind"), Some(("net", "listen")));
        assert_eq!(call_path_to_capability("UdpSocket", "connect"), Some(("net", "connect")));
        assert_eq!(call_path_to_capability("Command", "new"), Some(("proc", "exec")));
        assert_eq!(call_path_to_capability("env", "var"), Some(("env", "read")));
        assert_eq!(call_path_to_capability("env", "var_os"), Some(("env", "read")));
        assert_eq!(call_path_to_capability("env", "set_var"), Some(("env", "write")));
        assert_eq!(call_path_to_capability("env", "remove_var"), Some(("env", "write")));
        assert_eq!(call_path_to_capability("mem", "transmute"), Some(("ffi", "*")));
        assert_eq!(call_path_to_capability("String", "from"), None);
        assert_eq!(call_path_to_capability("Vec", "new"), None);
    }

    // ── DetectedCapability formatting ────────────────────────────────

    #[test]
    fn test_detected_capability_display() {
        let cap = DetectedCapability {
            category: "fs".to_string(),
            action: "read".to_string(),
            target: "data.txt".to_string(),
            file: "main.rs".to_string(),
            line: 5,
            evidence: "File::open(\"data.txt\")".to_string(),
        };
        assert_eq!(format!("{}", cap), "fs:read:data.txt");
        assert_eq!(cap.as_triple(), "fs:read:data.txt");
    }
}
