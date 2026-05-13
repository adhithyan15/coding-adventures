//! # twig-demo — end-to-end Twig multi-backend demonstration.
//!
//! This binary compiles and executes the same Twig program through six
//! distinct execution backends, printing a summary table at the end:
//!
//! 1. **Interpreter** — tree-walking interpreter (twig-vm)
//! 2. **AOT (ARM64 native)** — ahead-of-time compiler to Mach-O
//! 3. **BEAM (Erlang VM)** — compiles to BEAM bytecode, runs via `erl`
//! 4. **WebAssembly** — compiles to WASM, runs in pure-Rust runtime
//! 5. **JVM (Java VM)** — compiles to JVM class file, runs via `java`
//! 6. **CLR (.NET)** — compiles to CIL bytecode, runs in multi-method simulator
//!
//! ## Pipeline
//!
//! For BEAM, WASM, JVM, and CLR, the compilation chain is:
//!
//! ```text
//! Twig source
//!   → twig-ir-compiler      (IIRModule with "any" type hints)
//!   → pre_lower_builtins    (call_builtin "+" → add, etc.)
//!   → iir-type-checker      (concrete types: "i64", "bool", …)
//!   → fixup_control_flow    (ret/call/label get concrete types)
//!   → backend lowering      (BEAM / WASM / JVM / CIL bytes)
//! ```
//!
//! ## Programs
//!
//! - **Fibonacci** for interpreter, AOT, BEAM, WASM:
//!   `(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 10)`
//!   → expected result: 55
//!
//! - **Same fibonacci** for JVM and CLR using the full pipeline with fixup:
//!   The type-gap between Twig's `"any"` type hints and the typed VM
//!   requirements is bridged by the `fixup_control_flow_types` pass,
//!   mirroring what the BEAM and WASM pipelines already do.

use std::collections::HashMap;
use std::process::Command;
use std::time::Instant;

use interpreter_ir::{IIRInstr, IIRModule, Operand};
use iir_type_checker::infer_and_check;
use twig_ir_compiler::compile_source;
use wasm_runtime::WasmRuntime;

// ── Demo program ─────────────────────────────────────────────────────────────

/// The Twig fibonacci program used across all backends.
///
/// `fib(10)` = 55.  This tests recursion, conditionals, and integer arithmetic
/// — the core of any VM.
const FIB_PROGRAM: &str =
    "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 10)";

/// Expected return value for all backends.
const EXPECTED: i64 = 55;

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    print_banner();

    let mut results: Vec<BackendResult> = Vec::new();

    // 1. Interpreter
    let t = Instant::now();
    let r = run_interpreter(FIB_PROGRAM);
    results.push(BackendResult::new("Interpreter (twig-vm)", t.elapsed().as_millis(), r));

    // 2. AOT
    let t = Instant::now();
    let r = run_aot(FIB_PROGRAM);
    results.push(BackendResult::new("AOT (ARM64 native)", t.elapsed().as_millis(), r));

    // 3. BEAM
    let t = Instant::now();
    let r = run_beam(FIB_PROGRAM);
    results.push(BackendResult::new("BEAM (Erlang VM)", t.elapsed().as_millis(), r));

    // 4. WASM
    let t = Instant::now();
    let r = run_wasm(FIB_PROGRAM);
    results.push(BackendResult::new("WebAssembly (Rust runtime)", t.elapsed().as_millis(), r));

    // 5. JVM
    let t = Instant::now();
    let r = run_jvm(FIB_PROGRAM);
    results.push(BackendResult::new("JVM (Java 21)", t.elapsed().as_millis(), r));

    // 6. CLR
    let t = Instant::now();
    let r = run_clr(FIB_PROGRAM);
    results.push(BackendResult::new("CLR (.NET 9)", t.elapsed().as_millis(), r));

    print_results(&results);
}

// ── Result types ──────────────────────────────────────────────────────────────

struct BackendResult {
    name: String,
    elapsed_ms: u128,
    outcome: Result<i64, String>,
}

impl BackendResult {
    fn new(name: &str, elapsed_ms: u128, outcome: Result<i64, String>) -> Self {
        Self { name: name.to_string(), elapsed_ms, outcome }
    }

}

// ── Banner ────────────────────────────────────────────────────────────────────

fn print_banner() {
    println!("\n{}", "═".repeat(62));
    println!("   🐦 Twig Multi-Backend Demo");
    println!("{}", "═".repeat(62));
    println!();
    println!("Program:  {FIB_PROGRAM}");
    println!("Expected: {EXPECTED}  (fib(10) = Fibonacci number at index 10)");
    println!();
}

fn print_results(results: &[BackendResult]) {
    println!("\n{}", "─".repeat(62));
    println!("{:<32}  {:>8}  {:>12}  {}", "Backend", "Time(ms)", "Result", "Status");
    println!("{}", "─".repeat(62));

    let mut all_pass = true;
    for r in results {
        let (result_str, status) = match &r.outcome {
            Ok(v) => {
                let correct = *v == EXPECTED;
                if !correct { all_pass = false; }
                (v.to_string(), if correct { "✅ PASS" } else { "❌ WRONG" })
            }
            Err(e) => {
                all_pass = false;
                // Truncate long errors
                let short = if e.len() > 30 { format!("{}…", &e[..30]) } else { e.clone() };
                (short, "❌ FAIL")
            }
        };
        println!("{:<32}  {:>8}  {:>12}  {}", r.name, r.elapsed_ms, result_str, status);
    }

    println!("{}", "─".repeat(62));
    println!();
    if all_pass {
        println!("🎉 All 6 backends returned {EXPECTED}. Twig runs everywhere!");
    } else {
        println!("Some backends need attention (see errors above).");
    }
    println!();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Backend 1: Interpreter
// ═══════════════════════════════════════════════════════════════════════════════

fn run_interpreter(source: &str) -> Result<i64, String> {
    let vm = twig_vm::TwigVM::new();
    let val = vm.run(source).map_err(|e| format!("interpreter: {e}"))?;

    // LispyValue is a tagged i64.  For integers, the tag is in the low bits.
    // `as_int()` extracts the numeric value.
    match val.as_int() {
        Some(n) => Ok(n),
        None => Err(format!("interpreter: unexpected return value {:?}", val)),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Backend 2: AOT (ARM64)
// ═══════════════════════════════════════════════════════════════════════════════

fn run_aot(source: &str) -> Result<i64, String> {
    // Write source to a temp .twig file.
    let dir = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
    let src_path = dir.path().join("prog.twig");
    let bin_path = dir.path().join("prog");

    std::fs::write(&src_path, source).map_err(|e| format!("write src: {e}"))?;

    // Compile to native ARM64 Mach-O.
    twig_aot::compile_file_macos_arm64(&src_path, &bin_path)
        .map_err(|e| format!("aot compile: {e:?}"))?;

    // Execute: the process exit code is main() % 256.
    let status = Command::new(&bin_path)
        .status()
        .map_err(|e| format!("aot run: {e}"))?;

    let code = status.code().unwrap_or(-1) as i64;
    Ok(code)
}

// ═══════════════════════════════════════════════════════════════════════════════
// Backend 3: BEAM (Erlang VM)
// ═══════════════════════════════════════════════════════════════════════════════

fn run_beam(source: &str) -> Result<i64, String> {
    use twig_to_beam::compile_twig_to_beam;

    // BEAM module name must be a valid Erlang atom.  Use "twig_fib" so the
    // .beam file name matches the module name embedded in the binary.
    let module_name = "twig_fib";

    let beam_bytes = compile_twig_to_beam(source, module_name)
        .map_err(|e| format!("BEAM compile: {e}"))?;

    // Write .beam file to a temp directory.
    let dir = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
    let beam_path = dir.path().join(format!("{module_name}.beam"));
    std::fs::write(&beam_path, &beam_bytes).map_err(|e| format!("write beam: {e}"))?;

    // Run via `erl -noshell`.  The `-pa` flag adds the temp dir to the code path
    // so `twig_fib:main()` can be loaded.  We use `io:format` to print the
    // integer result followed by a newline so we can parse it.
    let output = Command::new("erl")
        .arg("-noshell")
        .arg("-pa")
        .arg(dir.path())
        .arg("-eval")
        .arg(format!("io:format(\"~w~n\", [{module_name}:main()]), init:stop()"))
        .output()
        .map_err(|e| format!("erl not found: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();

    trimmed.parse::<i64>()
        .map_err(|_| format!("BEAM stdout parse error: {:?} (stderr: {})",
            trimmed, String::from_utf8_lossy(&output.stderr).trim()))
}

// ═══════════════════════════════════════════════════════════════════════════════
// Backend 4: WebAssembly (pure-Rust runtime)
// ═══════════════════════════════════════════════════════════════════════════════

fn run_wasm(source: &str) -> Result<i64, String> {
    use twig_to_wasm::compile_twig_to_wasm;

    let wasm_bytes = compile_twig_to_wasm(source, "twig_fib")
        .map_err(|e| format!("WASM compile: {e}"))?;

    // The twig-to-wasm pipeline exports ALL functions by their IIR name.
    // The Twig compiler synthesises a `main` function that calls the top-level
    // expression.  We call `main` with no arguments.
    let runtime = WasmRuntime::new();
    let result = runtime
        .load_and_run(&wasm_bytes, "main", &[])
        .map_err(|e| format!("WASM run: {e}"))?;

    result.first().copied()
        .ok_or_else(|| "WASM: main returned no values".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Backend 5: JVM (Java 21)
// ═══════════════════════════════════════════════════════════════════════════════

fn run_jvm(source: &str) -> Result<i64, String> {
    use iir_to_jvm_class_file::serialize_jvm_class_file;

    // The JVM pipeline (like BEAM/WASM) requires concrete type hints.
    // Apply the same fixup: pre-lower builtins + infer + control-flow fixup.
    // NOTE: compile_twig_to_jvm already calls twig-ir-compiler + iir-type-checker
    // + iir-builtin-lowering internally, but it uses the strict lowering pass
    // that rejects "any" type hints.  We use a local pre-lower + infer + fixup
    // pipeline (mirroring what twig-to-beam and twig-to-wasm do) and bypass the
    // twig_to_jvm::compile_twig_to_jvm entry point by calling the pipeline directly.
    let class_name = "TwigFib";

    // Build the typed IIR manually using the same steps the BEAM/WASM pipelines use.
    let mut iir = compile_source(source, "twig_fib")
        .map_err(|e| format!("JVM compile: {e}"))?;
    pre_lower_builtins_jvm(&mut iir);
    infer_and_check(&mut iir);
    fixup_control_flow_types(&mut iir);

    // Compile the fixed-up IIR through the JVM backend directly.
    use twig_to_jvm::pipeline::run_pipeline_from_iir;
    use twig_to_jvm::IIRJvmConfig;
    let config = IIRJvmConfig::new(class_name);
    let class_file = run_pipeline_from_iir(iir, config)
        .map_err(|e| format!("JVM backend: {e}"))?;

    // Serialise the class file to bytes.
    let class_bytes = serialize_jvm_class_file(&class_file);

    // Write to a temp directory.
    let dir = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
    let class_path = dir.path().join(format!("{class_name}.class"));
    std::fs::write(&class_path, &class_bytes).map_err(|e| format!("write class: {e}"))?;

    // Generate a tiny launcher class (TwigLauncher.class) that has the
    // required `public static void main(String[])` method.  It calls
    // TwigFib.main() and exits with the integer result (result % 256 = exit code).
    let launcher_bytes = gen_jvm_launcher(class_name, dir.path())?;
    let launcher_path = dir.path().join("TwigLauncher.class");
    std::fs::write(&launcher_path, &launcher_bytes).map_err(|e| format!("write launcher: {e}"))?;

    // Run: java -cp <dir> TwigLauncher
    let status = Command::new("java")
        .arg("-cp")
        .arg(dir.path())
        .arg("TwigLauncher")
        .status()
        .map_err(|e| format!("java not found: {e}"))?;

    let code = status.code().unwrap_or(-1) as i64;
    Ok(code)
}

// ═══════════════════════════════════════════════════════════════════════════════
// Backend 6: CLR (multi-method CIL simulator)
// ═══════════════════════════════════════════════════════════════════════════════

fn run_clr(source: &str) -> Result<i64, String> {
    // Apply the same fixup pipeline as JVM/BEAM/WASM.
    let mut iir = compile_source(source, "twig_fib")
        .map_err(|e| format!("CLR compile: {e}"))?;
    pre_lower_builtins_clr(&mut iir);
    infer_and_check(&mut iir);
    fixup_control_flow_types(&mut iir);

    // Compile through the CLR backend.
    use twig_to_cil::pipeline::run_pipeline_from_iir as run_cil_from_iir;
    use twig_to_cil::IIRClrConfig;
    let config = IIRClrConfig::new("TwigFib");
    let artifact = run_cil_from_iir(iir, config)
        .map_err(|e| format!("CLR backend: {e}"))?;

    // Execute with the multi-method CIL runner.
    run_cil_artifact(&artifact)
        .map_err(|e| format!("CLR execute: {e}"))
}

// ═══════════════════════════════════════════════════════════════════════════════
// IIR fixup passes (mirroring twig-to-beam and twig-to-wasm pipelines)
// ═══════════════════════════════════════════════════════════════════════════════

/// Builtin-name → IIR-op map for JVM (uses `cmp_*` prefix).
const JVM_BUILTIN_MAP: &[(&str, &str)] = &[
    ("+",  "add"),   ("-",  "sub"),  ("*",  "mul"),  ("/",  "div"),
    ("=",  "cmp_eq"), ("<", "cmp_lt"), (">", "cmp_gt"),
    ("<=", "cmp_le"), (">=", "cmp_ge"),
    ("not", "not"),  ("_move", "mov"),
];

/// Builtin-name → IIR-op map for CLR (same as JVM here; the CLR backend uses
/// `cmp_lt`/`cmp_eq` opcode names).
const CLR_BUILTIN_MAP: &[(&str, &str)] = &[
    ("+",  "add"),   ("-",  "sub"),  ("*",  "mul"),  ("/",  "div"),
    ("=",  "cmp_eq"), ("<", "cmp_lt"), (">", "cmp_gt"),
    ("<=", "cmp_le"), (">=", "cmp_ge"),
    ("not", "not"),  ("_move", "mov"),
];

/// Lower arithmetic `call_builtin` instructions before type inference (JVM).
fn pre_lower_builtins_jvm(module: &mut IIRModule) {
    pre_lower_with_map(module, JVM_BUILTIN_MAP);
}

/// Lower arithmetic `call_builtin` instructions before type inference (CLR).
fn pre_lower_builtins_clr(module: &mut IIRModule) {
    pre_lower_with_map(module, CLR_BUILTIN_MAP);
}

fn pre_lower_with_map(module: &mut IIRModule, map: &[(&str, &str)]) {
    for func in &mut module.functions {
        let old = std::mem::take(&mut func.instructions);
        func.instructions = old.into_iter().map(|instr| {
            if instr.op != "call_builtin" { return instr; }
            let name = match instr.srcs.first() {
                Some(Operand::Var(n)) => n.as_str(),
                _ => return instr,
            };
            let Some((_, op)) = map.iter().find(|(b, _)| *b == name) else { return instr; };
            let args: Vec<Operand> = instr.srcs[1..].to_vec();
            IIRInstr::new(*op, instr.dest.clone(), args, &instr.type_hint)
        }).collect();
    }
}

/// Fix up `"any"` type hints on control-flow instructions after type inference.
///
/// This is a direct port of the same pass in `twig-to-wasm/src/pipeline.rs` and
/// `twig-to-beam/src/pipeline.rs`.
fn fixup_control_flow_types(module: &mut IIRModule) {
    for func in &mut module.functions {
        // Pass 0: Concrete-type the function parameters.
        //
        // The Twig IR compiler emits `func.params` with type "any" for all
        // parameters.  The JVM and CLR lowerers read `func.params` to decide
        // the slot type (iload vs lload).  We normalise every "any" parameter to
        // "i64" here so that the lowerers assign Long slots, consistent with the
        // "i64" type hints already on `const` instructions.
        for (_, param_type) in &mut func.params {
            if *param_type == "any" || *param_type == "polymorphic" {
                *param_type = "i64".to_string();
            }
        }

        // Pass 1: build SSA env from all concretely-typed instruction results.
        // Seed function parameters as "i64" (Twig integer runtime type).
        let mut env: HashMap<String, String> = HashMap::new();
        for (param_name, _) in &func.params {
            env.insert(param_name.clone(), "i64".to_string());
        }
        for instr in &func.instructions {
            if let Some(dest) = &instr.dest {
                let ty = &instr.type_hint;
                if ty != "any" && ty != "polymorphic" {
                    env.insert(dest.clone(), ty.clone());
                }
            }
        }

        // Pass 2: fix up "any" on control-flow and arithmetic instructions.
        for instr in &mut func.instructions {
            if instr.type_hint != "any" { continue; }

            let fixed = match instr.op.as_str() {
                "ret_void" | "label" | "jmp" | "jmp_if_true" | "jmp_if_false" => "void".to_string(),

                "ret" => match instr.srcs.first() {
                    Some(Operand::Var(src)) => env.get(src).cloned().unwrap_or_else(|| "void".into()),
                    Some(Operand::Int(_))   => "i64".to_string(),
                    _                       => "void".to_string(),
                },

                "call" => {
                    if let Some(dest) = &instr.dest {
                        env.get(dest).cloned().unwrap_or_else(|| "i64".into())
                    } else {
                        "void".to_string()
                    }
                }

                "mov" => match instr.srcs.first() {
                    Some(Operand::Var(src)) => env.get(src).cloned().unwrap_or_else(|| "i64".into()),
                    _ => "i64".to_string(),
                },

                "add" | "sub" | "mul" | "div" | "mod" | "neg" | "not" => {
                    instr.srcs.iter().find_map(|s| {
                        if let Operand::Var(n) = s { env.get(n).cloned() } else { None }
                    }).unwrap_or_else(|| "i64".into())
                }

                "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge" => "bool".to_string(),
                "eq" | "ne" | "lt" | "le" | "gt" | "ge" => "bool".to_string(),
                "lnot" => "bool".to_string(),

                _ => "any".to_string(),
            };

            if fixed != "any" {
                instr.type_hint = fixed.clone();
                if let Some(dest) = &instr.dest {
                    env.insert(dest.clone(), fixed);
                }
            }
        }

        // Pass 3: propagate concrete return type onto func.return_type.
        //
        // After Pass 2 the env contains types for all resolved variables.
        // If func.return_type is still "any" (the Twig compiler default),
        // look for a `ret` instruction and infer the type from its source.
        // This ensures the JVM/CLR lowerers generate the right method
        // descriptor (e.g. `()J` for long, `()I` for int).
        if func.return_type == "any" || func.return_type == "polymorphic" {
            let inferred = func.instructions.iter()
                .find(|i| i.op == "ret")
                .and_then(|i| i.srcs.first())
                .and_then(|s| match s {
                    Operand::Var(n) => env.get(n).cloned(),
                    Operand::Int(_) => Some("i64".to_string()),
                    Operand::Bool(_) => Some("bool".to_string()),
                    _ => None,
                });
            if let Some(ty) = inferred {
                if ty != "any" && ty != "polymorphic" {
                    func.return_type = ty;
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// JVM launcher class generator
// ═══════════════════════════════════════════════════════════════════════════════

/// Generate `TwigLauncher.class` — a tiny JVM class with
/// `public static void main(String[])` that calls `<twig_class>.main()` (which
/// returns a `long`) converts it to `int`, and passes it to `System.exit`.
///
/// Running `java -cp <dir> TwigLauncher` then returns `(int) twig_class.main()`
/// as the process exit code.
fn gen_jvm_launcher(twig_class: &str, _dir: &std::path::Path) -> Result<Vec<u8>, String> {
    // We build the class file by hand because `build_minimal_class_file` only
    // supports one method with a simple constant pool.  The launcher needs
    // method-refs to classes other than itself (TwigFib, System).
    //
    // Class file structure (JVM §4):
    //   magic(4) version(4) cp_count(2) [cp entries] access(2) this(2) super(2)
    //   ifaces(2) fields(2) methods(2) [methods] attrs(2)
    //
    // Constant pool layout:
    //   #1  Utf8 "TwigLauncher"
    //   #2  Class #1
    //   #3  Utf8 "java/lang/Object"
    //   #4  Class #3
    //   #5  Utf8 <twig_class>
    //   #6  Class #5
    //   #7  Utf8 "main"
    //   #8  Utf8 "()J"                      (descriptor for twig main → long)
    //   #9  NameAndType #7 #8
    //  #10  Methodref #6 #9                 (TwigFib.main()J)
    //  #11  Utf8 "java/lang/System"
    //  #12  Class #11
    //  #13  Utf8 "exit"
    //  #14  Utf8 "(I)V"
    //  #15  NameAndType #13 #14
    //  #16  Methodref #12 #15               (System.exit(I)V)
    //  #17  Utf8 "([Ljava/lang/String;)V"   (descriptor for JVM main)
    //  #18  Utf8 "Code"
    //
    // Total: 18 entries; cp_count field = 19.
    //
    // Note: fixup_control_flow_types Pass 0 normalises all "any" params to "i64"
    // and Pass 3 infers func.return_type as "i64".  The JVM lowerer maps "i64" →
    // JvmType::Long → lreturn, descriptor "J".  So the entry method returns long.

    // Rust's borrow checker disallows multiple closures that all capture the same
    // mutable variable.  We use small free functions that take `&mut Vec<u8>`.

    /// Push a Utf8 constant (tag=1, u16 length, bytes).
    fn cp_utf8(cp: &mut Vec<u8>, s: &str) {
        cp.push(1);
        cp.extend_from_slice(&(s.len() as u16).to_be_bytes());
        cp.extend_from_slice(s.as_bytes());
    }
    /// Push a Class constant (tag=7, u16 name_index).
    fn cp_class(cp: &mut Vec<u8>, idx: u16) {
        cp.push(7);
        cp.extend_from_slice(&idx.to_be_bytes());
    }
    /// Push a NameAndType constant (tag=12, u16 name_idx, u16 desc_idx).
    fn cp_nat(cp: &mut Vec<u8>, ni: u16, di: u16) {
        cp.push(12);
        cp.extend_from_slice(&ni.to_be_bytes());
        cp.extend_from_slice(&di.to_be_bytes());
    }
    /// Push a Methodref constant (tag=10, u16 class_idx, u16 nat_idx).
    fn cp_methodref(cp: &mut Vec<u8>, ci: u16, ni: u16) {
        cp.push(10);
        cp.extend_from_slice(&ci.to_be_bytes());
        cp.extend_from_slice(&ni.to_be_bytes());
    }

    let mut cp: Vec<u8> = Vec::new();

    // #1  Utf8 "TwigLauncher"
    cp_utf8(&mut cp, "TwigLauncher");
    // #2  Class #1
    cp_class(&mut cp, 1);
    // #3  Utf8 "java/lang/Object"
    cp_utf8(&mut cp, "java/lang/Object");
    // #4  Class #3
    cp_class(&mut cp, 3);
    // #5  Utf8 <twig_class>
    cp_utf8(&mut cp, twig_class);
    // #6  Class #5
    cp_class(&mut cp, 5);
    // #7  Utf8 "main"
    cp_utf8(&mut cp, "main");
    // #8  Utf8 "()J"
    cp_utf8(&mut cp, "()J");
    // #9  NameAndType #7 #8
    cp_nat(&mut cp, 7, 8);
    // #10 Methodref #6 #9
    cp_methodref(&mut cp, 6, 9);
    // #11 Utf8 "java/lang/System"
    cp_utf8(&mut cp, "java/lang/System");
    // #12 Class #11
    cp_class(&mut cp, 11);
    // #13 Utf8 "exit"
    cp_utf8(&mut cp, "exit");
    // #14 Utf8 "(I)V"
    cp_utf8(&mut cp, "(I)V");
    // #15 NameAndType #13 #14
    cp_nat(&mut cp, 13, 14);
    // #16 Methodref #12 #15
    cp_methodref(&mut cp, 12, 15);
    // #17 Utf8 "([Ljava/lang/String;)V"
    cp_utf8(&mut cp, "([Ljava/lang/String;)V");
    // #18 Utf8 "Code"
    cp_utf8(&mut cp, "Code");

    // Bytecode for `public static void main(String[])`:
    //   invokestatic #10     (TwigFib.main() → long, descriptor "()J")
    //   l2i                  (long → int so result fits in exit code)
    //   invokestatic #16     (System.exit(int))
    //   return               (unreachable, needed for verifier)
    let code: Vec<u8> = vec![
        0xB8, 0x00, 0x0A,  // invokestatic #10
        0x88,              // l2i
        0xB8, 0x00, 0x10,  // invokestatic #16
        0xB1,              // return
    ];
    let code_len = code.len() as u32;

    // Code attribute body: max_stack(2) max_locals(1) code_len(4) code exception_count(2) attr_count(2)
    let mut code_attr_body: Vec<u8> = Vec::new();
    code_attr_body.extend_from_slice(&2u16.to_be_bytes()); // max_stack: long takes 2 slots
    code_attr_body.extend_from_slice(&1u16.to_be_bytes()); // max_locals: String[] arg
    code_attr_body.extend_from_slice(&code_len.to_be_bytes());
    code_attr_body.extend_from_slice(&code);
    code_attr_body.extend_from_slice(&0u16.to_be_bytes()); // exception_table_length = 0
    code_attr_body.extend_from_slice(&0u16.to_be_bytes()); // attributes_count = 0

    // Full method info for `main`:
    //   access_flags=0x0009 (ACC_PUBLIC|ACC_STATIC)
    //   name_index=#7 ("main"), descriptor_index=#17, attributes_count=1
    let mut method: Vec<u8> = Vec::new();
    method.extend_from_slice(&0x0009u16.to_be_bytes()); // access
    method.extend_from_slice(&7u16.to_be_bytes());      // name_index
    method.extend_from_slice(&17u16.to_be_bytes());     // descriptor_index
    method.extend_from_slice(&1u16.to_be_bytes());      // attributes_count
    // Code attribute:
    method.extend_from_slice(&18u16.to_be_bytes()); // attribute_name_index ("Code")
    method.extend_from_slice(&(code_attr_body.len() as u32).to_be_bytes()); // attribute_length
    method.extend_from_slice(&code_attr_body);

    // Assemble the full class file.
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(&0xCAFEBABEu32.to_be_bytes()); // magic
    file.extend_from_slice(&0u16.to_be_bytes());          // minor version
    file.extend_from_slice(&65u16.to_be_bytes());         // major version: Java 21 = 65
    file.extend_from_slice(&19u16.to_be_bytes());         // cp_count (18 entries + 1)
    file.extend_from_slice(&cp);                          // constant pool
    file.extend_from_slice(&0x0021u16.to_be_bytes());     // access (ACC_PUBLIC | ACC_SUPER)
    file.extend_from_slice(&2u16.to_be_bytes());          // this_class (#2 = TwigLauncher)
    file.extend_from_slice(&4u16.to_be_bytes());          // super_class (#4 = Object)
    file.extend_from_slice(&0u16.to_be_bytes());          // interfaces_count
    file.extend_from_slice(&0u16.to_be_bytes());          // fields_count
    file.extend_from_slice(&1u16.to_be_bytes());          // methods_count
    file.extend_from_slice(&method);                      // method[0]
    file.extend_from_slice(&0u16.to_be_bytes());          // attributes_count

    Ok(file)
}

// ═══════════════════════════════════════════════════════════════════════════════
// CLR multi-method executor
// ═══════════════════════════════════════════════════════════════════════════════

/// Execute a `CILProgramArtifact` from its entry point, supporting inter-method
/// calls via a frame stack.
///
/// This is a simplified CIL interpreter that handles the subset of opcodes
/// emitted by `iir-to-cil-bytecode` for arithmetic Twig programs:
/// - Integer constants: `ldc.i4.*`, `ldc.i4.s`, `ldc.i4`
/// - Locals: `ldloc.0-3`, `ldloc.s`, `stloc.0-3`, `stloc.s`
/// - Parameters: `ldarg.0-3`, `ldarg.s`, `starg.s`
/// - Arithmetic: `add`, `sub`, `mul`, `div`
/// - Comparison: `ceq`, `cgt`, `clt` (two-byte opcodes via 0xFE prefix)
/// - Branches: `br.s`, `br`, `brfalse.s`, `brfalse`, `brtrue.s`, `brtrue`
/// - Method calls: `call` (dispatched via method token → artifact method index)
/// - Return: `ret`
fn run_cil_artifact(artifact: &ir_to_cil_bytecode::backend::CILProgramArtifact) -> Result<i64, String> {
    // Find the entry method.
    //
    // `entry_method()` returns `methods.first()` unconditionally, which is wrong
    // when there are multiple methods (e.g. fib + main).  We look up by
    // `entry_label` instead, falling back to first only if not found.
    let entry = artifact.methods.iter()
        .find(|m| m.name == artifact.entry_label)
        .or_else(|| artifact.methods.first())
        .ok_or_else(|| "CLR: no entry method".to_string())?;

    // Build a token → method-index lookup table.
    // Token for methods[i] = 0x06000001 + i.
    let token_map: HashMap<u32, usize> = artifact.methods.iter().enumerate()
        .map(|(i, _)| (0x06000001u32 + i as u32, i))
        .collect();

    // Execute from entry with no arguments.
    exec_method(&entry.body, &[], &token_map, artifact, 0)
}

/// A single call frame in the CLR simulator.
struct CilFrame {
    bytecode: Vec<u8>,
    pc: usize,
    stack: Vec<i32>,
    locals: Vec<i32>,
    args: Vec<i32>,
}

impl CilFrame {
    fn new(bytecode: &[u8], args: Vec<i32>, num_locals: usize) -> Self {
        CilFrame {
            bytecode: bytecode.to_vec(),
            pc: 0,
            stack: Vec::new(),
            locals: vec![0i32; num_locals],
            args,
        }
    }
}

/// Execute a CIL method body recursively, supporting `call` into other methods.
fn exec_method(
    bytecode: &[u8],
    args: &[i32],
    token_map: &HashMap<u32, usize>,
    artifact: &ir_to_cil_bytecode::backend::CILProgramArtifact,
    depth: usize,
) -> Result<i64, String> {
    const MAX_DEPTH: usize = 200;
    if depth > MAX_DEPTH {
        return Err(format!("CLR: stack overflow (depth > {MAX_DEPTH})"));
    }

    // Estimate local count from bytecode: scan for stloc opcodes to find
    // the highest slot used.
    let num_locals = estimate_local_count(bytecode);
    let mut frame = CilFrame::new(bytecode, args.to_vec(), num_locals);

    loop {
        if frame.pc >= frame.bytecode.len() {
            return Err(format!("CLR: PC {} past end of bytecode ({})", frame.pc, frame.bytecode.len()));
        }

        let opcode = frame.bytecode[frame.pc];

        match opcode {
            // nop
            0x00 => { frame.pc += 1; }

            // ldnull
            0x01 => { frame.stack.push(0); frame.pc += 1; }

            // ldarg.0 .. ldarg.3
            0x02..=0x05 => {
                let idx = (opcode - 0x02) as usize;
                let val = *frame.args.get(idx).ok_or_else(|| format!("CLR: ldarg.{idx} out of range (args={})", frame.args.len()))?;
                frame.stack.push(val);
                frame.pc += 1;
            }

            // ldarg.s N
            0x0E => {
                let idx = frame.bytecode[frame.pc + 1] as usize;
                let val = *frame.args.get(idx).ok_or_else(|| format!("CLR: ldarg.s {idx} out of range"))?;
                frame.stack.push(val);
                frame.pc += 2;
            }

            // starg.s N
            0x10 => {
                let idx = frame.bytecode[frame.pc + 1] as usize;
                let val = frame.stack.pop().ok_or("CLR: starg.s stack underflow")?;
                while frame.args.len() <= idx { frame.args.push(0); }
                frame.args[idx] = val;
                frame.pc += 2;
            }

            // ldloc.0 .. ldloc.3
            0x06..=0x09 => {
                let idx = (opcode - 0x06) as usize;
                let val = *frame.locals.get(idx).ok_or_else(|| format!("CLR: ldloc.{idx} out of range (locals={})", frame.locals.len()))?;
                frame.stack.push(val);
                frame.pc += 1;
            }

            // stloc.0 .. stloc.3
            0x0A..=0x0D => {
                let idx = (opcode - 0x0A) as usize;
                let val = frame.stack.pop().ok_or("CLR: stloc stack underflow")?;
                while frame.locals.len() <= idx { frame.locals.push(0); }
                frame.locals[idx] = val;
                frame.pc += 1;
            }

            // ldloc.s N
            0x11 => {
                let idx = frame.bytecode[frame.pc + 1] as usize;
                let val = *frame.locals.get(idx).ok_or_else(|| format!("CLR: ldloc.s {idx} out of range"))?;
                frame.stack.push(val);
                frame.pc += 2;
            }

            // stloc.s N
            0x13 => {
                let idx = frame.bytecode[frame.pc + 1] as usize;
                let val = frame.stack.pop().ok_or("CLR: stloc.s stack underflow")?;
                while frame.locals.len() <= idx { frame.locals.push(0); }
                frame.locals[idx] = val;
                frame.pc += 2;
            }

            // ldc.i4.M1 (-1), ldc.i4.0 .. ldc.i4.8
            0x15 => { frame.stack.push(-1); frame.pc += 1; }  // ldc.i4.m1
            0x16..=0x1E => {
                frame.stack.push((opcode - 0x16) as i32);
                frame.pc += 1;
            }

            // ldc.i4.s (sign-extended byte)
            0x1F => {
                let raw = frame.bytecode[frame.pc + 1] as i8;
                frame.stack.push(raw as i32);
                frame.pc += 2;
            }

            // ldc.i4 (4-byte little-endian int32)
            0x20 => {
                let bytes = &frame.bytecode[frame.pc+1..frame.pc+5];
                let val = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                frame.stack.push(val);
                frame.pc += 5;
            }

            // dup
            0x25 => {
                let top = *frame.stack.last().ok_or("CLR: dup stack underflow")?;
                frame.stack.push(top);
                frame.pc += 1;
            }

            // pop
            0x26 => {
                frame.stack.pop().ok_or("CLR: pop stack underflow")?;
                frame.pc += 1;
            }

            // call <token4> — dispatch to a user method or built-in
            0x28 => {
                let token = u32::from_le_bytes([
                    frame.bytecode[frame.pc + 1],
                    frame.bytecode[frame.pc + 2],
                    frame.bytecode[frame.pc + 3],
                    frame.bytecode[frame.pc + 4],
                ]);
                frame.pc += 5;

                // Special token: Console.WriteLine(int64)
                if token == 0x0A00_0002 {
                    let val = frame.stack.pop().ok_or("CLR: call writeline stack underflow")?;
                    println!("{val}");
                    continue;
                }

                // User function call.
                let method_idx = token_map.get(&token)
                    .copied()
                    .ok_or_else(|| format!("CLR: unknown call token 0x{token:08X}"))?;

                let callee = &artifact.methods[method_idx];
                let n_params = callee.parameter_types.len();

                // Pop exactly n_params items from the top of the evaluation stack.
                //
                // CIL calling convention: arguments are pushed left-to-right before
                // the `call` instruction, so after draining the top n_params entries
                // we get [arg0, arg1, …, argN-1] — the correct left-to-right order.
                if frame.stack.len() < n_params {
                    return Err(format!("CLR: call arg count mismatch: need {n_params} but stack has {}", frame.stack.len()));
                }
                let call_args: Vec<i32> = frame.stack.drain(frame.stack.len() - n_params..).collect();

                let ret_val = exec_method(&callee.body, &call_args, token_map, artifact, depth + 1)?;
                // Push return value back (truncate to i32 for now; Twig integers fit)
                frame.stack.push(ret_val as i32);
            }

            // ret
            0x2A => {
                let ret_val = frame.stack.pop().map(|v| v as i64).unwrap_or(0);
                return Ok(ret_val);
            }

            // br.s (short unconditional branch)
            0x2B => {
                let offset = frame.bytecode[frame.pc + 1] as i8 as i32;
                frame.pc = ((frame.pc as i32) + 2 + offset) as usize;
            }

            // brfalse.s
            0x2C => {
                let offset = frame.bytecode[frame.pc + 1] as i8 as i32;
                let val = frame.stack.pop().ok_or("CLR: brfalse.s stack underflow")?;
                if val == 0 {
                    frame.pc = ((frame.pc as i32) + 2 + offset) as usize;
                } else {
                    frame.pc += 2;
                }
            }

            // brtrue.s
            0x2D => {
                let offset = frame.bytecode[frame.pc + 1] as i8 as i32;
                let val = frame.stack.pop().ok_or("CLR: brtrue.s stack underflow")?;
                if val != 0 {
                    frame.pc = ((frame.pc as i32) + 2 + offset) as usize;
                } else {
                    frame.pc += 2;
                }
            }

            // beq.s (branch if equal — short)
            0x2E => {
                let offset = frame.bytecode[frame.pc + 1] as i8 as i32;
                let b = frame.stack.pop().ok_or("CLR: beq.s stack underflow (b)")?;
                let a = frame.stack.pop().ok_or("CLR: beq.s stack underflow (a)")?;
                if a == b { frame.pc = ((frame.pc as i32) + 2 + offset) as usize; }
                else { frame.pc += 2; }
            }

            // blt.s (branch if less than — short)
            0x32 => {
                let offset = frame.bytecode[frame.pc + 1] as i8 as i32;
                let b = frame.stack.pop().ok_or("CLR: blt.s underflow")?;
                let a = frame.stack.pop().ok_or("CLR: blt.s underflow")?;
                if a < b { frame.pc = ((frame.pc as i32) + 2 + offset) as usize; }
                else { frame.pc += 2; }
            }

            // br (long unconditional branch, 4-byte offset)
            0x38 => {
                let offset = i32::from_le_bytes([
                    frame.bytecode[frame.pc + 1], frame.bytecode[frame.pc + 2],
                    frame.bytecode[frame.pc + 3], frame.bytecode[frame.pc + 4],
                ]);
                frame.pc = ((frame.pc as i32) + 5 + offset) as usize;
            }

            // brfalse (long)
            0x39 => {
                let offset = i32::from_le_bytes([
                    frame.bytecode[frame.pc + 1], frame.bytecode[frame.pc + 2],
                    frame.bytecode[frame.pc + 3], frame.bytecode[frame.pc + 4],
                ]);
                let val = frame.stack.pop().ok_or("CLR: brfalse stack underflow")?;
                if val == 0 { frame.pc = ((frame.pc as i32) + 5 + offset) as usize; }
                else { frame.pc += 5; }
            }

            // brtrue (long)
            0x3A => {
                let offset = i32::from_le_bytes([
                    frame.bytecode[frame.pc + 1], frame.bytecode[frame.pc + 2],
                    frame.bytecode[frame.pc + 3], frame.bytecode[frame.pc + 4],
                ]);
                let val = frame.stack.pop().ok_or("CLR: brtrue stack underflow")?;
                if val != 0 { frame.pc = ((frame.pc as i32) + 5 + offset) as usize; }
                else { frame.pc += 5; }
            }

            // add
            0x58 => { let b = frame.stack.pop().ok_or("CLR: add underflow")?; let a = frame.stack.pop().ok_or("CLR: add underflow")?; frame.stack.push(a.wrapping_add(b)); frame.pc += 1; }
            // sub
            0x59 => { let b = frame.stack.pop().ok_or("CLR: sub underflow")?; let a = frame.stack.pop().ok_or("CLR: sub underflow")?; frame.stack.push(a.wrapping_sub(b)); frame.pc += 1; }
            // mul
            0x5A => { let b = frame.stack.pop().ok_or("CLR: mul underflow")?; let a = frame.stack.pop().ok_or("CLR: mul underflow")?; frame.stack.push(a.wrapping_mul(b)); frame.pc += 1; }
            // div
            0x5B => {
                let b = frame.stack.pop().ok_or("CLR: div underflow")?;
                let a = frame.stack.pop().ok_or("CLR: div underflow")?;
                if b == 0 { return Err("CLR: division by zero".into()); }
                frame.stack.push(a.wrapping_div(b));
                frame.pc += 1;
            }
            // rem (mod) — not in enum, emitted as raw 0x5D
            0x5D => { let b = frame.stack.pop().ok_or("CLR: rem underflow")?; let a = frame.stack.pop().ok_or("CLR: rem underflow")?; frame.stack.push(a.wrapping_rem(b)); frame.pc += 1; }
            // neg
            0x65 => { let a = frame.stack.pop().ok_or("CLR: neg underflow")?; frame.stack.push(a.wrapping_neg()); frame.pc += 1; }
            // not (bitwise complement)
            0x66 => { let a = frame.stack.pop().ok_or("CLR: not underflow")?; frame.stack.push(!a); frame.pc += 1; }
            // and
            0x5F => { let b = frame.stack.pop().ok_or("CLR: and underflow")?; let a = frame.stack.pop().ok_or("CLR: and underflow")?; frame.stack.push(a & b); frame.pc += 1; }
            // or
            0x60 => { let b = frame.stack.pop().ok_or("CLR: or underflow")?; let a = frame.stack.pop().ok_or("CLR: or underflow")?; frame.stack.push(a | b); frame.pc += 1; }
            // xor
            0x61 => { let b = frame.stack.pop().ok_or("CLR: xor underflow")?; let a = frame.stack.pop().ok_or("CLR: xor underflow")?; frame.stack.push(a ^ b); frame.pc += 1; }
            // shl
            0x62 => { let b = frame.stack.pop().ok_or("CLR: shl underflow")?; let a = frame.stack.pop().ok_or("CLR: shl underflow")?; frame.stack.push(a << (b & 31)); frame.pc += 1; }
            // shr
            0x63 => { let b = frame.stack.pop().ok_or("CLR: shr underflow")?; let a = frame.stack.pop().ok_or("CLR: shr underflow")?; frame.stack.push(a >> (b & 31)); frame.pc += 1; }

            // ret_void (we treat the same as ret but return 0)
            // Note: 0x2A is ret. Some methods may not return a value on stack.

            // Two-byte comparison opcodes (prefix 0xFE)
            0xFE => {
                let second = frame.bytecode[frame.pc + 1];
                let b = frame.stack.pop().ok_or("CLR: cmp underflow")?;
                let a = frame.stack.pop().ok_or("CLR: cmp underflow")?;
                let result = match second {
                    0x01 => if a == b { 1 } else { 0 }, // ceq
                    0x02 => if a > b  { 1 } else { 0 }, // cgt
                    0x04 => if a < b  { 1 } else { 0 }, // clt
                    _ => return Err(format!("CLR: unknown two-byte opcode 0xFE 0x{second:02X}")),
                };
                frame.stack.push(result);
                frame.pc += 2;
            }

            other => {
                return Err(format!("CLR: unknown opcode 0x{other:02X} at PC={}", frame.pc));
            }
        }
    }
}

/// Scan bytecode to estimate how many local variable slots are needed.
///
/// This walks all `stloc` instructions to find the highest slot index used,
/// which gives us the minimum local count for the frame.
fn estimate_local_count(bytecode: &[u8]) -> usize {
    let mut max_slot = 0usize;
    let mut i = 0;
    while i < bytecode.len() {
        let op = bytecode[i];
        match op {
            0x0A..=0x0D => { // stloc.0-3
                let slot = (op - 0x0A) as usize;
                if slot + 1 > max_slot { max_slot = slot + 1; }
                i += 1;
            }
            0x13 => { // stloc.s N
                if i + 1 < bytecode.len() {
                    let slot = bytecode[i + 1] as usize;
                    if slot + 1 > max_slot { max_slot = slot + 1; }
                }
                i += 2;
            }
            0x06..=0x09 => { i += 1; } // ldloc.0-3
            0x11 => { i += 2; }        // ldloc.s
            0x02..=0x05 => { i += 1; } // ldarg.0-3
            0x0E | 0x10 => { i += 2; } // ldarg.s / starg.s
            0x15 | 0x16..=0x1E => { i += 1; } // ldc.i4.*
            0x1F => { i += 2; }        // ldc.i4.s
            0x20 => { i += 5; }        // ldc.i4
            0x28 => { i += 5; }        // call <token4>
            0x2B | 0x2C | 0x2D | 0x2E | 0x2F..=0x35 => { i += 2; } // br.s, brfalse.s, etc.
            0x38..=0x3F | 0x45 => { i += 5; } // br, brfalse, brtrue, etc. (long)
            0xFE => { i += 2; }        // two-byte opcode
            0x8D | 0x9E | 0x94 => { i += 5; } // newarr, stelem.i4, ldelem.i4
            _ => { i += 1; }           // all other single-byte opcodes
        }
    }
    max_slot
}
