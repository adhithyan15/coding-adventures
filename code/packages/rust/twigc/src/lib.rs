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

/// Run the TW05 fixed-point self-check against the Twig compiler source tree.
///
/// `compiler_dir` must be the directory that contains the eleven compiler
/// `.tw` files (`span.tw`, `lexer.tw`, `main.tw`, …).  This function:
///
/// 1. Derives the module search root as `parent(compiler_dir)`, so that
///    `(import compiler/main)` can resolve at link time.
/// 2. Writes an ephemeral wrapper `.tw` source to a temporary directory:
///    ```scheme
///    (module twigc/self-check-runner
///      (typed lenient)
///      (export main)
///      (import compiler/main))
///    (define (main) (if (fixed-point-check "<dir>") 1 0))
///    ```
///    where `<dir>` is the canonicalized absolute path of `compiler_dir`.
///    `(export main)` is required so the IIR linker uses the wrapper's
///    `main` as the entry point rather than `compiler/main`'s `main`.
/// 3. Compiles and runs the wrapper via `twigc_run`.
/// 4. Returns `Ok(true)` if `fixed-point-check` returns `#t` (result == 1),
///    `Ok(false)` otherwise.
///
/// ## Why an ephemeral wrapper?
///
/// `twig_vm::run_module_tree` always calls the `main` function with no
/// arguments.  `fixed-point-check` requires a `dir` argument.  The wrapper
/// creates a synthetic `main` that embeds the directory path as a string
/// constant and delegates to `fixed-point-check`.
///
/// ## Stack note
///
/// `fixed-point-check` only processes `span.tw` (~365 chars, 2 functions).
/// The IIR compilation of the full 11-module tree happens at load time via
/// the Rust-based driver (not recursively in the VM), so the runtime call
/// depth is modest — no large-stack thread is required.
///
/// # Example
///
/// ```no_run
/// use twigc::twigc_self_check;
/// use std::path::Path;
///
/// let passed = twigc_self_check(Path::new("code/packages/twig/compiler"), &[]).unwrap();
/// assert!(passed, "fixed-point check should always pass on pure Twig");
/// ```
pub fn twigc_self_check(
    compiler_dir: &Path,
    extra_search_paths: &[PathBuf],
) -> Result<bool, TwigcError> {
    // ── 1. Canonicalize the compiler directory ────────────────────────────
    let compiler_dir = compiler_dir
        .canonicalize()
        .map_err(|e| TwigcError::Vm {
            message: format!("cannot resolve compiler dir '{}': {e}", compiler_dir.display()),
        })?;

    // ── 2. Derive the module search root ─────────────────────────────────
    //
    // The search root must be the PARENT of `compiler_dir` so that:
    //   (import compiler/main) → <search_root>/compiler/main.tw ✓
    let search_root = compiler_dir
        .parent()
        .ok_or_else(|| TwigcError::Vm {
            message: format!(
                "compiler_dir '{}' has no parent — cannot derive search root",
                compiler_dir.display()
            ),
        })?
        .to_path_buf();

    // Build the effective search-path list: search_root first, then extras.
    let mut all_paths: Vec<PathBuf> = vec![search_root];
    all_paths.extend_from_slice(extra_search_paths);

    // ── 3. Write an ephemeral wrapper to a temp directory ─────────────────
    //
    // The wrapper calls (fixed-point-check "<dir>") with the absolute path
    // baked in as a string constant.  It returns 1 on pass, 0 on failure
    // so that twigc_run can map the result to bool via `result == 1`.
    //
    // `tmp_guard` is an RAII handle: the temp dir is removed on drop, so
    // the generated `.tw` file (which embeds the compiler path) is cleaned
    // up even if an early `?`-return or panic occurs after this point.
    let tmp_guard = make_tmp_dir("self_check")?;

    // The dir path is a Twig string literal — validate and escape it.
    //
    // We use `?` rather than `unwrap_or("")` because silently embedding an
    // empty string would make fixed-point-check try to read "/span.tw" (the
    // root directory) instead of the intended compiler dir, producing a
    // misleading VM error.  Failing early with a clear message is safer.
    let raw_dir_str = compiler_dir
        .to_str()
        .ok_or_else(|| TwigcError::Vm {
            message: format!(
                "compiler_dir '{}' contains non-UTF-8 characters — \
                 cannot embed path in Twig string literal",
                compiler_dir.display()
            ),
        })?;

    // Reject control characters (ASCII < 0x20 or 0x7F) in the path.
    //
    // On Linux/macOS, directory names may legally contain newlines or other
    // control bytes.  If such a character reached the generated Twig source
    // it could confuse parsers downstream of the Twig compiler itself, or
    // indicate a path-injection attempt.  Failing early with a clear message
    // is safer than attempting to escape every possible byte value.
    if let Some(bad) = raw_dir_str.chars().find(|c| c.is_ascii_control()) {
        return Err(TwigcError::Vm {
            message: format!(
                "compiler_dir '{}' contains a control character (U+{:04X}) — \
                 unsafe to embed in Twig source",
                compiler_dir.display(),
                bad as u32
            ),
        });
    }

    // Escape `\` and `"` for the Twig string literal.  These are the only
    // two characters that have special meaning inside `"…"` per the Twig
    // lexer regex `/"([^"\\]|\\.)*"/`.
    let dir_str = raw_dir_str
        .replace('\\', "\\\\")
        .replace('"', "\\\"");

    // The wrapper exports `main` explicitly so the IIR linker's Pass 1a
    // claims the public name `main` for this module.  Without `(export main)`
    // the linker sees that `compiler/main` already exports `main` and renames
    // this module's `main` to its fully-qualified private form; the VM's entry
    // point would then resolve to `compiler/main`'s `main` (which returns 2),
    // and `twigc_run` would return 2 instead of 1.
    //
    // With `(export main)`, both modules export `main`.  During IIR merging
    // (`add_or_replace` in the linker), the wrapper is processed last in
    // topological order (it imports compiler/main, so it comes after it),
    // meaning the wrapper's `main` overwrites `compiler/main`'s `main` in
    // the merged output.  The VM therefore calls the wrapper's `main`, which
    // calls `(fixed-point-check "<dir>")` and returns 1 on success.
    let wrapper_src = format!(
        "; twigc self-check wrapper — generated, do not edit\n\
         (module twigc/self-check-runner\n\
           (typed lenient)\n\
           (export main)\n\
           (import compiler/main))\n\
         \n\
         ; main: call fixed-point-check with the baked-in compiler dir.\n\
         ; Returns 1 (integer) on pass, 0 on failure, so that twigc_run's\n\
         ; as_int() extraction works correctly.\n\
         (define (main)\n\
           (if (fixed-point-check \"{dir}\")\n\
               1\n\
               0))\n",
        dir = dir_str,
    );

    let wrapper_path = tmp_guard.path().join("self-check-runner.tw");
    std::fs::write(&wrapper_path, &wrapper_src).map_err(|e| TwigcError::Vm {
        message: format!("failed to write self-check wrapper: {e}"),
    })?;

    // ── 4. Compile and run the wrapper ────────────────────────────────────
    //
    // Store the result before `tmp_guard` drops (which deletes the temp dir).
    // The `?` propagation happens after drop to ensure cleanup is not skipped.
    let run_result = twigc_run(&wrapper_path, &all_paths);

    // `tmp_guard` drops here, deleting the temp directory and the generated
    // `.tw` source.  Cleanup is guaranteed even on early return — the RAII
    // guard handles the case where write or run_result returned an error.
    drop(tmp_guard);

    // ── 5. Interpret the integer return ──────────────────────────────────
    //   1 → fixed-point-check returned #t → Ok(true)
    //   0 → fixed-point-check returned #f → Ok(false)
    let result = run_result?;
    Ok(result == 1)
}

// ── Internal helpers ─────────────────────────────────────────────────────────

/// RAII guard for a temporary directory.
///
/// Deletes the directory and all its contents when dropped.  This ensures
/// the generated wrapper `.tw` file (which embeds the compiler path) is
/// removed even if an early return or panic occurs.
///
/// Access the path via [`TmpDirGuard::path`].
struct TmpDirGuard(PathBuf);

impl TmpDirGuard {
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TmpDirGuard {
    fn drop(&mut self) {
        // Best-effort: ignore errors (e.g. already removed by the caller).
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Create a unique temporary directory under `std::env::temp_dir()`,
/// returning an RAII [`TmpDirGuard`] that deletes it on drop.
///
/// The directory is named `twigc_<tag>_<pid>_<nanos>`.
fn make_tmp_dir(tag: &str) -> Result<TmpDirGuard, TwigcError> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let dir = std::env::temp_dir().join(format!(
        "twigc_{tag}_{}_{nonce}",
        std::process::id(),
    ));
    std::fs::create_dir(&dir).map_err(|e| TwigcError::Vm {
        message: format!("failed to create temp dir '{}': {e}", dir.display()),
    })?;
    Ok(TmpDirGuard(dir))
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
        // Navigate: twigc/ → rust/ → packages/ → twig/compiler/
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap()  // rust/
            .parent().unwrap()  // packages/
            .join("twig/compiler")
            .canonicalize()
            .expect("code/packages/twig/compiler/ must exist")
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
        // Ok or any non-TypeErrors Err is acceptable; only TypeErrors is a failure.
        if let Err(TwigcError::Driver(ModuleDriverError::TypeErrors { .. })) = result {
            panic!("lenient mode must never produce TypeErrors from Phase 3.5");
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
                twigc_run(&root, std::slice::from_ref(&dir)).unwrap()
            })
            .expect("failed to spawn thread")
            .join()
            .expect("thread panicked");

        assert_eq!(
            result, 2,
            "twigc_run on compiler tree: (main) should return 2 (span.tw fn count); got {result}"
        );
    }

    // ── Test 7 ───────────────────────────────────────────────────────────────
    //
    // `twigc_self_check` on the real 11-module compiler source tree should
    // return `Ok(true)`.
    //
    // This is the TW05-S end-to-end test: the ephemeral wrapper is compiled,
    // the VM calls `(fixed-point-check "<dir>")`, which compiles `span.tw`
    // twice via the self-hosted pipeline and verifies the opcode summaries
    // are byte-for-byte identical.  Because Twig is purely functional this
    // always holds — the test asserts the invariant is explicit and
    // mechanically verified.
    //
    // `fixed-point-check` only processes span.tw (~365 chars, 2 functions),
    // so the runtime stack depth is modest; no large-stack thread is needed.
    // However, the full 11-module compilation at load time (Phase 1-5 via the
    // Rust driver) may still push the debug-mode stack; we use a 32 MiB
    // thread to match the other compiler-tree tests.

    #[test]
    fn self_check_compiler_tree_fixed_point() {
        let twig_src = compiler_src_dir();
        let dir = make_tempdir("self_check");
        copy_all_tw_modules(&twig_src, &dir);

        // The compiler dir in the temp copy is dir/compiler/.
        // We pass dir/ as extra_search_paths so imports resolve;
        // twigc_self_check also derives the search root from the parent
        // of compiler_dir automatically, so we pass an empty slice here
        // and let the function compute it.
        let compiler_dir = dir.join("compiler");

        let result = std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                // extra_search_paths is empty: twigc_self_check derives
                // the search root as parent(compiler_dir) automatically.
                twigc_self_check(&compiler_dir, &[])
            })
            .expect("failed to spawn thread")
            .join()
            .expect("thread panicked");

        match result {
            Ok(true) => { /* pass */ }
            Ok(false) => {
                panic!(
                    "twigc_self_check returned Ok(false) — fixed-point check returned #f.\n\
                     This means stage1 and stage2 opcode summaries differed, which\n\
                     should never happen for a purely functional compiler."
                );
            }
            Err(ref e) => {
                panic!(
                    "twigc_self_check returned Err: {e}\n\
                     Check that the compiler source directory was copied correctly\n\
                     and that the wrapper was generated without path issues."
                );
            }
        }
    }
}
