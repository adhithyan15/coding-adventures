//! # twigc — Twig compiler driver library (TW05-R / LANG73)
//!
//! This crate provides three thin, testable functions that wrap
//! [`twig_module_driver::compile_module_tree`] and [`twig_vm`]:
//!
//! | Function | Description |
//! |----------|-------------|
//! | [`twigc_check`]    | Type-check only; return `Ok(())` or `Err(TwigcError)`. |
//! | [`twigc_emit_iir`] | Compile to IIR and return a human-readable listing. |
//! | [`twigc_run`]      | Compile and run via twig-vm; return the `i64` result. |
//!
//! ## Error model
//!
//! All three functions return `Result<_, TwigcError>`.  [`TwigcError`] wraps
//! [`twig_module_driver::ModuleDriverError`] (covers parse errors, import
//! resolution failures, type errors) and adds a `Vm` variant for runtime
//! traps.
//!
//! ## Example
//!
//! ```no_run
//! use twigc::twigc_run;
//! use std::path::Path;
//!
//! let result = twigc_run(Path::new("main.tw"), &[]).unwrap();
//! println!("→ {result}");
//! ```

use std::fmt;
use std::path::{Path, PathBuf};

use interpreter_ir::IIRModule;
use twig_module_driver::{compile_module_tree, ModuleDriverError};

// ── Error type ────────────────────────────────────────────────────────────────

/// All errors that `twigc` operations can produce.
///
/// Each variant carries a human-readable message so callers can display it
/// directly without inspecting the underlying library types.
#[derive(Debug)]
pub enum TwigcError {
    /// The module driver failed (parse, import resolution, or type errors).
    ///
    /// The `ModuleDriverError` inside covers all phases up to IIR compilation.
    Driver(ModuleDriverError),

    /// The compiled module trapped at runtime.
    ///
    /// `message` is the stringified `twig_vm::TwigFileError`.
    Vm { message: String },
}

impl fmt::Display for TwigcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TwigcError::Driver(e) => write!(f, "{e}"),
            TwigcError::Vm { message } => write!(f, "runtime error: {message}"),
        }
    }
}

impl From<ModuleDriverError> for TwigcError {
    fn from(e: ModuleDriverError) -> Self {
        TwigcError::Driver(e)
    }
}

// ── Core helpers ──────────────────────────────────────────────────────────────

/// Convert `&[PathBuf]` to `Vec<&Path>` for the module-driver / twig-vm APIs.
fn as_path_refs(v: &[PathBuf]) -> Vec<&Path> {
    v.iter().map(PathBuf::as_path).collect()
}

/// Compile `path` (multi-file) and return the linked `IIRModule`.
///
/// This is the shared kernel used by all three public functions.
/// It calls `compile_module_tree` which runs:
///   - Phase 1: recursive import discovery
///   - Phase 2: cycle detection
///   - Phase 3: extern name collection
///   - Phase 3.5: topological type-check (LANG72)
///   - Phase 4: IIR compilation per module
///   - Phase 5: IIR linking
fn compile(path: &Path, search_paths: &[PathBuf]) -> Result<IIRModule, TwigcError> {
    let refs = as_path_refs(search_paths);
    compile_module_tree(path, &refs).map_err(TwigcError::Driver)
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Type-check a Twig program without running it.
///
/// Runs the full compilation pipeline through Phase 3.5 (type-check) and
/// Phase 4 (IIR compilation), but does **not** execute the resulting module.
///
/// Returns `Ok(())` if the program compiles successfully.  Returns
/// `Err(TwigcError::Driver(TypeErrors { … }))` if a `(typed strict)` module
/// has type errors.
///
/// # Example
///
/// ```no_run
/// use twigc::twigc_check;
/// use std::path::Path;
///
/// twigc_check(Path::new("main.tw"), &[]).unwrap();
/// ```
pub fn twigc_check(path: &Path, search_paths: &[PathBuf]) -> Result<(), TwigcError> {
    compile(path, search_paths)?;
    Ok(())
}

/// Compile a Twig program and return a human-readable IIR listing.
///
/// Each function in the linked module is formatted as:
///
/// ```text
/// fn <name>:
///   <index>  <op>  <dest?>  <srcs…>
/// ```
///
/// This is the `--emit=iir` mode: it lets you inspect what the compiler
/// produces without running the program.
///
/// # Example
///
/// ```no_run
/// use twigc::twigc_emit_iir;
/// use std::path::Path;
///
/// let iir_text = twigc_emit_iir(Path::new("main.tw"), &[]).unwrap();
/// println!("{iir_text}");
/// ```
pub fn twigc_emit_iir(path: &Path, search_paths: &[PathBuf]) -> Result<String, TwigcError> {
    let module = compile(path, search_paths)?;
    let mut out = String::new();

    // A module contains all functions from all linked source files.
    // We emit them in definition order, which matches the link order.
    for func in &module.functions {
        out.push_str(&format!("fn {}:\n", func.name));
        for (i, instr) in func.instructions.iter().enumerate() {
            // Format: <index>  <op>  [dest]  [srcs…]
            let dest_str = instr
                .dest
                .as_deref()
                .unwrap_or("-");
            let srcs_str: Vec<String> = instr
                .srcs
                .iter()
                .map(|op| format!("{op:?}"))
                .collect();
            out.push_str(&format!(
                "  {i:>4}  {op:<16}  {dest:<12}  {srcs}\n",
                op = instr.op,
                dest = dest_str,
                srcs = srcs_str.join("  "),
            ));
        }
        out.push('\n');
    }

    Ok(out)
}

/// Compile and run a Twig program via the interpreter, returning the integer
/// result of `main()`.
///
/// The return value is extracted from the `LispyValue` returned by the VM.
/// If `main()` returns a non-integer value (e.g. `#t` or a cons cell), the
/// function returns `Ok(0)` as a safe default.
///
/// # Example
///
/// ```no_run
/// use twigc::twigc_run;
/// use std::path::Path;
///
/// let result = twigc_run(Path::new("main.tw"), &[]).unwrap();
/// assert_eq!(result, 42);
/// ```
pub fn twigc_run(path: &Path, search_paths: &[PathBuf]) -> Result<i64, TwigcError> {
    let refs = as_path_refs(search_paths);
    let value = twig_vm::run_module_tree(path, &refs).map_err(|e| TwigcError::Vm {
        message: format!("{e:?}"),
    })?;
    // Extract the integer value; non-integer results (booleans, cons cells,
    // strings) are mapped to 0 — callers who need exact values should use
    // `compile_module_tree` + `twig_vm::run` directly.
    Ok(value.as_int().unwrap_or(0))
}

// ── Tests ─────────────────────────────────────────────────────────────────────
//
// All tests write temporary `.tw` files to a unique temp directory so they
// are hermetic and can run in parallel.

#[cfg(test)]
mod twigc_tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    // ── helper ───────────────────────────────────────────────────────────────

    fn make_tempdir(tag: &str) -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let dir = std::env::temp_dir().join(format!(
            "twigc_{tag}_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir(&dir).unwrap_or_else(|e| {
            panic!("make_tempdir: could not create {}: {e}", dir.display())
        });
        dir
    }

    fn compiler_src_dir() -> PathBuf {
        // Navigate: twigc/ → rust/ → packages/ → code/ → twig/compiler/
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap()  // rust/
            .parent().unwrap()  // packages/
            .parent().unwrap()  // code/
            .join("twig/compiler")
            .canonicalize()
            .expect("code/twig/compiler/ must exist")
    }

    fn copy_tw(twig_src: &Path, dest_dir: &Path, name: &str) {
        let src = twig_src.join(format!("{name}.tw"));
        let dest = dest_dir.join("compiler");
        fs::create_dir_all(&dest).unwrap();
        fs::copy(&src, dest.join(format!("{name}.tw")))
            .unwrap_or_else(|e| panic!("copy {name}.tw: {e}"));
    }

    fn copy_all_tw_modules(twig_src: &Path, dest_dir: &Path) {
        for name in &[
            "span", "token", "diagnostic", "ast", "iir-types",
            "iir-builder", "lexer", "cst-parser", "parser", "emit", "main",
        ] {
            copy_tw(twig_src, dest_dir, name);
        }
    }

    // ── Test 1 ───────────────────────────────────────────────────────────────
    //
    // `twigc_check` on a clean `(typed strict)` program should return `Ok(())`.

    #[test]
    fn check_clean_strict_program_ok() {
        let dir = make_tempdir("check_clean");
        let src = r#"
(module twigc/test1
  (typed strict)
  (export main))

(define (main) (+ 21 21))
"#;
        let root = dir.join("main.tw");
        fs::write(&root, src).unwrap();
        let result = twigc_check(&root, &[]);
        assert!(result.is_ok(), "clean strict program should type-check: {result:?}");
    }

    // ── Test 2 ───────────────────────────────────────────────────────────────
    //
    // `twigc_check` on a `(typed strict)` program that calls an undefined
    // function should return `Err(Driver(TypeErrors { … }))`.

    #[test]
    fn check_strict_program_with_type_error_fails() {
        let dir = make_tempdir("check_fail");
        let src = r#"
(module twigc/test2
  (typed strict)
  (export main))

(define (main) (no-such-function 99))
"#;
        let root = dir.join("main.tw");
        fs::write(&root, src).unwrap();
        let result = twigc_check(&root, &[]);
        match result {
            Err(TwigcError::Driver(ModuleDriverError::TypeErrors { .. })) => {
                // correct
            }
            Ok(()) => panic!("expected TypeErrors but got Ok"),
            Err(e) => panic!("expected TypeErrors but got: {e}"),
        }
    }

    // ── Test 3 ───────────────────────────────────────────────────────────────
    //
    // `twigc_check` on a `(typed lenient)` program with an undefined function
    // should NOT return TypeErrors (lenient mode never fails Phase 3.5).

    #[test]
    fn check_lenient_bad_varref_passes() {
        let dir = make_tempdir("check_lenient");
        let src = r#"
(module twigc/test3
  (typed lenient)
  (export main))

(define (main) (no-such-function 99))
"#;
        let root = dir.join("main.tw");
        fs::write(&root, src).unwrap();
        let result = twigc_check(&root, &[]);
        match result {
            Err(TwigcError::Driver(ModuleDriverError::TypeErrors { .. })) => {
                panic!("lenient mode must never produce TypeErrors from Phase 3.5");
            }
            _ => {} // Ok or any non-TypeErrors Err is acceptable
        }
    }

    // ── Test 4 ───────────────────────────────────────────────────────────────
    //
    // `twigc_emit_iir` on a simple program should produce a string that
    // contains the function name.

    #[test]
    fn emit_iir_produces_fn_listing() {
        let dir = make_tempdir("emit_iir");
        let src = r#"(define (main) (+ 21 21))"#;
        let root = dir.join("main.tw");
        fs::write(&root, src).unwrap();
        let listing = twigc_emit_iir(&root, &[]).unwrap();
        assert!(
            listing.contains("fn main:"),
            "IIR listing should contain 'fn main:'; got:\n{listing}"
        );
        assert!(
            listing.contains("add") || listing.contains("const"),
            "IIR listing should contain arithmetic opcodes; got:\n{listing}"
        );
    }

    // ── Test 5 ───────────────────────────────────────────────────────────────
    //
    // `twigc_run` on `(define (main) (+ 21 21))` should return `Ok(42)`.

    #[test]
    fn run_arithmetic_returns_value() {
        let dir = make_tempdir("run_arith");
        let src = r#"(define (main) (+ 21 21))"#;
        let root = dir.join("main.tw");
        fs::write(&root, src).unwrap();
        let result = twigc_run(&root, &[]).unwrap();
        assert_eq!(result, 42, "expected 42 but got {result}");
    }

    // ── Test 6 ───────────────────────────────────────────────────────────────
    //
    // `twigc_run` on the full 11-module compiler tree runs `(main)` which
    // compiles a stripped in-memory version of `span.tw` and returns 2
    // (the number of function definitions in span.tw: make-span + dummy-span).
    //
    // This verifies that the multi-module driver, type-checker (Phase 3.5),
    // and interpreter all wire together correctly end-to-end through twigc.
    //
    // Note: `(self-compile-all dir)` returns 179 but requires a `dir` argument
    // and uses `host/read_file` — that is tested separately in twig-module-driver
    // tw05n_tests.  `(main)` uses an in-memory source string and is safe to call
    // without extra arguments.

    #[test]
    fn run_compiler_tree_main_returns_2() {
        let twig_src = compiler_src_dir();
        let dir = make_tempdir("run_compiler");
        copy_all_tw_modules(&twig_src, &dir);

        // Spawn with a 32 MiB stack to accommodate deep recursive-descent
        // parsing and type-checking over ~500 lines of typed Twig in debug mode.
        let result = std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                let root = dir.join("compiler").join("main.tw");
                twigc_run(&root, &[dir.clone()]).unwrap()
            })
            .expect("failed to spawn thread")
            .join()
            .expect("thread panicked");

        assert_eq!(
            result, 2,
            "twigc_run on compiler tree: (main) should return 2 (span.tw fn count); got {result}"
        );
    }
}
