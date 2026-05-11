//! # `twig-dap` — Twig Debug Adapter Protocol adapter.
//!
//! **LS03 PR B** — Twig instantiation of [`dap_adapter_core`].  Implements
//! [`LanguageDebugAdapter`] for Twig and provides the `twig-dap` binary.
//!
//! ## What this crate does
//!
//! - [`TwigDebugAdapter::compile`] runs `twig_ir_compiler::compile_source`
//!   on the requested file, then walks the resulting `IIRFunction::source_map`
//!   to emit a [`debug_sidecar`]-format byte blob suitable for
//!   [`dap_adapter_core::SidecarIndex`].
//! - [`TwigDebugAdapter::launch_vm`] spawns the sibling `twig-vm` binary
//!   with `--debug-port <PORT>` so the adapter can connect over TCP.
//! - The `twig-dap` binary wires the above into
//!   [`dap_adapter_core::DapServer`] so editors can drive the
//!   Twig debugger over stdio.
//!
//! ## Architecture
//!
//! ```text
//! Editor (VS Code / Neovim / …)
//!     │  DAP / JSON over stdio
//!     ▼
//! twig-dap binary  (bin/twig_dap.rs in this crate)
//!     │  DapServer::new(TwigDebugAdapter).run_stdio()
//!     ▼
//! dap-adapter-core  (DAP message handling, breakpoints, stepping)
//!     │  TwigDebugAdapter::{compile, launch_vm}
//!     ▼
//! twig-vm --debug-port N  (spawned subprocess; speaks VM debug protocol over TCP)
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Child;

use dap_adapter_core::LanguageDebugAdapter;
use debug_sidecar::DebugSidecarWriter;
use interpreter_ir::IIRModule;

// ---------------------------------------------------------------------------
// TwigDebugAdapter
// ---------------------------------------------------------------------------

/// Per-language hooks for the Twig DAP adapter.
///
/// Stateless — every method recomputes from scratch.  Pass a fresh
/// instance to [`dap_adapter_core::DapServer::new`] per session.
#[derive(Debug, Default, Clone, Copy)]
pub struct TwigDebugAdapter;

impl LanguageDebugAdapter for TwigDebugAdapter {
    /// Compile `source_path` and emit a debug sidecar.
    ///
    /// Returns `(source_path, sidecar_bytes)`.  The first element is the
    /// **source path** rather than a separate bytecode file because the
    /// `twig-vm` CLI takes Twig source directly — there's no pre-built
    /// bytecode artefact in this stack today.  The DAP server passes
    /// this path back to [`Self::launch_vm`] as the "bytecode" arg.
    ///
    /// The `sidecar_bytes` are produced by walking `source_map` for each
    /// compiled function and emitting one row per non-synthetic
    /// instruction.  See [`build_sidecar`] for details.
    fn compile(
        &self,
        source_path: &Path,
        _workspace_root: &Path,
    ) -> Result<(PathBuf, Vec<u8>), String> {
        let source = std::fs::read_to_string(source_path)
            .map_err(|e| format!("read {}: {e}", source_path.display()))?;
        let module = twig_ir_compiler::compile_source(&source, "twig")
            .map_err(|e| format!("twig compile: {e}"))?;
        let sidecar_bytes = build_sidecar(&module, source_path);
        Ok((source_path.to_path_buf(), sidecar_bytes))
    }

    /// Spawn the sibling `twig-vm` binary in debug mode.
    fn launch_vm(
        &self,
        bytecode_path: &Path,
        debug_port: u16,
    ) -> Result<Child, String> {
        let exe = find_sibling_binary("twig-vm")?;
        std::process::Command::new(&exe)
            .arg("--debug-port").arg(debug_port.to_string())
            .arg(bytecode_path)
            .spawn()
            .map_err(|e| format!("spawn {exe:?}: {e}"))
    }

    fn language_name(&self) -> &'static str { "twig" }
    fn file_extensions(&self) -> &'static [&'static str] { &["twig", "tw"] }
}

// ---------------------------------------------------------------------------
// Sidecar builder
// ---------------------------------------------------------------------------

/// Build a [`debug_sidecar`] byte blob from an [`IIRModule`].
///
/// One source file is registered (the absolute path of `source_path`).
/// For each function in the module:
///
/// 1. `begin_function(name, start=0, param_count=params.len())`
/// 2. **Line table** — for every `(instr_index, source_loc)` pair where the
///    loc is non-synthetic (line ≠ 0), `record(...)` emits a row.
/// 3. **Variable declarations** — see below.
/// 4. `end_function(name, n_instrs)`
///
/// ## Variable declarations
///
/// The DAP `variables` panel needs both a human name for each register AND
/// the register's numeric slot index so `vm_conn.get_slot(frame, slot)` can
/// fetch the live value.  `build_sidecar` derives that mapping statically by
/// replicating the same two-phase assignment the VM's [`VMFrame`] uses at
/// runtime:
///
/// **Phase 1 — parameters.**
/// `VMFrame::for_function` maps `params[i]` to register slot `i` before the
/// first instruction executes.  Parameters are live for the entire function
/// body (`live_start=0, live_end=n_instrs`).
///
/// **Phase 2 — SSA temporaries.**
/// `VMFrame::assign` allocates the next sequential slot (`name_to_reg.len()`)
/// whenever a variable name is first written.  Walking instructions in
/// declaration order (which equals execution order for SSA code where each
/// name is defined exactly once) produces the same mapping.  Each temporary
/// is live from the instruction that defines it through to the end of the
/// function (`live_start=def_instr, live_end=n_instrs`) — a conservative
/// approximation that shows the variable as soon as it has a value and keeps
/// it visible until function exit, matching the behaviour of GDB/LLDB for
/// ordinary local variables.
///
/// ### Why declaration order ≈ execution order
///
/// IIR is in SSA form: every variable name appears as a `dest` exactly once.
/// When the VM executes in a straight-line function the first-write order is
/// identical to the instruction-array order.  For functions with branches the
/// approximation may assign a reg_index slightly out of sync with one branch
/// path, but the variable value is still readable at any instruction where the
/// VM has actually written to that slot — the sidecar just exposes all
/// declared variables at all breakpoints (conservative / "always visible")
/// rather than a precise per-path live-range.  This is the standard V1
/// tradeoff for debuggers; LLDB itself uses the same conservative strategy
/// for `-O0` builds.
pub fn build_sidecar(module: &IIRModule, source_path: &Path) -> Vec<u8> {
    let mut w = DebugSidecarWriter::new();
    let path_str = source_path.to_string_lossy().to_string();
    let fid = w.add_source_file(&path_str, &[]);

    for func in &module.functions {
        let n_instrs = func.instructions.len();
        w.begin_function(&func.name, 0, func.params.len());

        // ---- Line table -----------------------------------------------
        for (idx, loc) in func.source_map.iter().enumerate() {
            // SourceLoc::SYNTHETIC is line=0, col=0 — skip; the sidecar
            // reader's DWARF-style "previous row" lookup covers
            // unmapped instructions naturally.
            if loc.line == 0 { continue; }
            w.record(&func.name, idx, fid, loc.line, loc.column);
        }

        // ---- Variable declarations ------------------------------------
        //
        // `name_to_reg` shadows VMFrame::name_to_reg so we can assign the
        // same slot indices the runtime will use.  Insertion order is
        // significant — each new name gets slot `name_to_reg.len()` at the
        // time of first insertion, mirroring `VMFrame::assign`.
        let mut name_to_reg: HashMap<String, u32> = HashMap::new();

        // Phase 1 — parameters.  `VMFrame::for_function` fills these BEFORE
        // execution starts, so they always occupy slots 0..params.len()-1.
        for (i, (param_name, param_type)) in func.params.iter().enumerate() {
            let reg_idx = i as u32;
            name_to_reg.insert(param_name.clone(), reg_idx);
            w.declare_variable(&func.name, reg_idx, param_name, param_type, 0, n_instrs);
        }

        // Phase 2 — SSA temporaries.  Walk instructions in declaration order
        // and assign the next sequential slot to each new dest name.
        for (instr_idx, instr) in func.instructions.iter().enumerate() {
            let dest_name = match &instr.dest {
                Some(d) => d,
                None => continue,       // void instruction — no register produced
            };
            if name_to_reg.contains_key(dest_name) {
                // Already mapped (only possible if the same name appears as a
                // dest twice, which is a violation of SSA but shouldn't crash
                // the sidecar builder — just skip the duplicate).
                continue;
            }
            // Slot index = number of variables already registered, matching
            // VMFrame::assign's `let next_idx = self.name_to_reg.len()`.
            let reg_idx = name_to_reg.len() as u32;
            name_to_reg.insert(dest_name.clone(), reg_idx);

            // Conservative live range: the variable is valid from the
            // instruction that defines it through to the end of the
            // function.  The debugger will show it as soon as the VM
            // executes the defining instruction and it stays visible
            // at every subsequent breakpoint in the same frame.
            w.declare_variable(
                &func.name,
                reg_idx,
                dest_name,
                &instr.type_hint,
                instr_idx,
                n_instrs,
            );
        }

        w.end_function(&func.name, n_instrs);
    }

    w.finish()
}

// ---------------------------------------------------------------------------
// Binary discovery
// ---------------------------------------------------------------------------

/// Locate a sibling binary next to the currently-running executable.
///
/// Used to find `twig-vm` from `twig-dap`.  Falls back to a bare name
/// (PATH lookup) if no sibling is found, supporting both
/// `cargo install`-style installation and ad-hoc development.
///
/// ## Path-traversal guard
///
/// `name` MUST be a bare filename — no directory separators, no `..`,
/// no leading `.`.  This prevents a future caller from accidentally (or
/// maliciously) constructing a path that escapes the current
/// executable's directory.  Today the only call site uses the hardcoded
/// literal `"twig-vm"`, but the guard hardens against drift.
pub fn find_sibling_binary(name: &str) -> Result<PathBuf, String> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
    {
        return Err(format!("find_sibling_binary: invalid name {name:?}"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            #[cfg(windows)] let candidate = dir.join(format!("{name}.exe"));
            #[cfg(not(windows))] let candidate = dir.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Ok(PathBuf::from(name))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use dap_adapter_core::SidecarIndex;
    use std::io::Write;

    /// Write `source` to a temp file with `.twig` extension.
    /// Caller keeps the returned `Vec<u8>`-backed temp dir alive.
    fn write_temp_twig(source: &str) -> (PathBuf, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prog.twig");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(source.as_bytes()).unwrap();
        (path, dir)
    }

    #[test]
    fn adapter_metadata_correct() {
        let a = TwigDebugAdapter;
        assert_eq!(a.language_name(), "twig");
        assert!(a.file_extensions().contains(&"twig"));
        assert!(a.file_extensions().contains(&"tw"));
    }

    #[test]
    fn compile_returns_source_path_unchanged() {
        let (p, _g) = write_temp_twig("(+ 1 2)\n");
        let a = TwigDebugAdapter;
        let (path, _bytes) = a.compile(&p, Path::new(".")).expect("ok");
        assert_eq!(path, p);
    }

    #[test]
    fn compile_emits_parseable_sidecar() {
        let (p, _g) = write_temp_twig("(define (sq x) (* x x))\n(sq 7)\n");
        let a = TwigDebugAdapter;
        let (_, bytes) = a.compile(&p, Path::new(".")).expect("ok");
        let idx = SidecarIndex::from_bytes(&bytes).expect("valid sidecar");
        assert!(!idx.source_files().is_empty());
    }

    #[test]
    fn compile_sidecar_resolves_known_line() {
        let (p, _g) = write_temp_twig("(define (f) 1)\n(f)\n");
        let a = TwigDebugAdapter;
        let (src_path, bytes) = a.compile(&p, Path::new(".")).expect("ok");
        let idx = SidecarIndex::from_bytes(&bytes).expect("valid sidecar");
        let path_str = src_path.to_string_lossy();
        let locs_line_1 = idx.source_to_locs(&path_str, 1);
        assert!(!locs_line_1.is_empty(),
                "line 1 must have at least one VM location: {locs_line_1:?}");
    }

    #[test]
    fn compile_rejects_invalid_twig() {
        let (p, _g) = write_temp_twig("(unbalanced\n");
        let a = TwigDebugAdapter;
        let err = a.compile(&p, Path::new(".")).unwrap_err();
        assert!(err.to_lowercase().contains("compile"), "got: {err}");
    }

    #[test]
    fn compile_rejects_missing_file() {
        let a = TwigDebugAdapter;
        let err = a.compile(Path::new("/nonexistent/xyz.twig"), Path::new(".")).unwrap_err();
        assert!(err.contains("read"), "got: {err}");
    }

    #[test]
    fn build_sidecar_handles_empty_module() {
        let m = IIRModule::new("empty", "twig");
        let bytes = build_sidecar(&m, Path::new("dummy.twig"));
        SidecarIndex::from_bytes(&bytes).expect("parses");
    }

    #[test]
    fn find_sibling_binary_returns_something() {
        let p = find_sibling_binary("nonexistent-xyz").expect("ok");
        assert!(p.to_string_lossy().contains("nonexistent-xyz"));
    }

    #[test]
    fn find_sibling_binary_rejects_path_traversal() {
        assert!(find_sibling_binary("../../bin/sh").is_err());
        assert!(find_sibling_binary("..").is_err());
        assert!(find_sibling_binary(".").is_err());
        assert!(find_sibling_binary("a/b").is_err());
        assert!(find_sibling_binary("a\\b").is_err());
        assert!(find_sibling_binary("").is_err());
        assert!(find_sibling_binary("a\0b").is_err());
    }

    // -----------------------------------------------------------------------
    // Variable introspection tests
    //
    // These tests verify that `build_sidecar` correctly emits variable
    // declarations so the DAP `variables` panel can show live register values.
    // We build `IIRModule` / `IIRFunction` fixtures directly rather than going
    // through the Twig compiler so the tests remain fast, deterministic, and
    // independent of the compiler's current codegen.
    // -----------------------------------------------------------------------

    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};

    /// Build a minimal `IIRModule` containing one function with the given
    /// params and instructions.  No source_map is set (synthetic locs only).
    fn make_module_with_fn(
        fn_name: &str,
        params: Vec<(&str, &str)>,
        instrs: Vec<IIRInstr>,
    ) -> IIRModule {
        let mut m = IIRModule::new("test", "twig");
        let string_params: Vec<(String, String)> = params
            .into_iter()
            .map(|(n, t)| (n.to_string(), t.to_string()))
            .collect();
        let mut func = IIRFunction::new(fn_name, string_params, "any", instrs);
        // No source_map — all locs are synthetic (line=0), which is fine for
        // variable-only tests.
        func.source_map.clear();
        m.add_or_replace(func);
        m
    }

    /// Parse the sidecar and return `live_variables(fn_name, at_instr)`.
    fn live_vars_at(
        bytes: &[u8],
        fn_name: &str,
        at_instr: usize,
    ) -> Vec<debug_sidecar::Variable> {
        let idx = SidecarIndex::from_bytes(bytes).expect("valid sidecar");
        idx.reader().live_variables(fn_name, at_instr)
    }

    // --- Parameter tests ---

    #[test]
    fn params_are_declared_as_variables() {
        // Function `add(a: u8, b: u8)` with two params and one ret instruction.
        let m = make_module_with_fn(
            "add",
            vec![("a", "u8"), ("b", "u8")],
            vec![IIRInstr::new("ret", None, vec![Operand::Var("a".into())], "u8")],
        );
        let bytes = build_sidecar(&m, Path::new("x.twig"));
        // Both params should be live at instruction 0 (the only instruction).
        let vars = live_vars_at(&bytes, "add", 0);
        let names: Vec<&str> = vars.iter().map(|v| v.name.as_str()).collect();
        assert!(names.contains(&"a"), "param 'a' missing: {names:?}");
        assert!(names.contains(&"b"), "param 'b' missing: {names:?}");
    }

    #[test]
    fn param_register_indices_match_declaration_order() {
        // Param 0 → reg 0, param 1 → reg 1.
        let m = make_module_with_fn(
            "f",
            vec![("x", "any"), ("y", "any")],
            vec![IIRInstr::new("ret", None, vec![], "void")],
        );
        let bytes = build_sidecar(&m, Path::new("x.twig"));
        let vars = live_vars_at(&bytes, "f", 0);
        let reg_of = |n: &str| vars.iter().find(|v| v.name == n).map(|v| v.reg_index);
        assert_eq!(reg_of("x"), Some(0), "param 'x' should be reg 0");
        assert_eq!(reg_of("y"), Some(1), "param 'y' should be reg 1");
    }

    #[test]
    fn params_are_live_for_entire_function() {
        // n_instrs = 3; params must be live at instructions 0, 1, 2.
        let m = make_module_with_fn(
            "g",
            vec![("p", "i32")],
            vec![
                IIRInstr::new("const_i32", Some("v0".into()), vec![Operand::Int(1)], "i32"),
                IIRInstr::new("add_i32", Some("v1".into()),
                    vec![Operand::Var("p".into()), Operand::Var("v0".into())], "i32"),
                IIRInstr::new("ret", None, vec![Operand::Var("v1".into())], "i32"),
            ],
        );
        let bytes = build_sidecar(&m, Path::new("x.twig"));
        for i in 0..3 {
            let vars = live_vars_at(&bytes, "g", i);
            let has_p = vars.iter().any(|v| v.name == "p");
            assert!(has_p, "param 'p' must be live at instruction {i}");
        }
    }

    // --- SSA temporary tests ---

    #[test]
    fn ssa_temp_is_declared_as_variable() {
        // `v0 = const_i32(42)` followed by `ret v0`.
        // `v0` should appear as a declared variable.
        let m = make_module_with_fn(
            "main",
            vec![],
            vec![
                IIRInstr::new("const_i32", Some("v0".into()), vec![Operand::Int(42)], "i32"),
                IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
            ],
        );
        let bytes = build_sidecar(&m, Path::new("x.twig"));
        // v0 is defined at instr 0 → live at instr 0.
        let vars = live_vars_at(&bytes, "main", 0);
        assert!(vars.iter().any(|v| v.name == "v0"), "temp v0 missing: {vars:?}");
    }

    #[test]
    fn ssa_temp_register_comes_after_params() {
        // `add(a, b)`: params get regs 0 and 1; first temp should get reg 2.
        let m = make_module_with_fn(
            "add",
            vec![("a", "i32"), ("b", "i32")],
            vec![
                IIRInstr::new("add_i32", Some("v0".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
                IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
            ],
        );
        let bytes = build_sidecar(&m, Path::new("x.twig"));
        let vars = live_vars_at(&bytes, "add", 0);
        let v0_reg = vars.iter().find(|v| v.name == "v0").map(|v| v.reg_index);
        assert_eq!(v0_reg, Some(2), "first temp 'v0' should be reg 2 (after 2 params)");
    }

    #[test]
    fn ssa_temp_not_live_before_defining_instruction() {
        // `v0` is defined at instruction 1; it must NOT be live at instruction 0.
        let m = make_module_with_fn(
            "h",
            vec![],
            vec![
                // instr 0: a no-op (void ret to simulate reaching instr 1)
                IIRInstr::new("noop", None, vec![], "void"),
                // instr 1: v0 defined here
                IIRInstr::new("const_i32", Some("v0".into()), vec![Operand::Int(7)], "i32"),
                IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
            ],
        );
        let bytes = build_sidecar(&m, Path::new("x.twig"));
        let vars_at_0 = live_vars_at(&bytes, "h", 0);
        assert!(
            !vars_at_0.iter().any(|v| v.name == "v0"),
            "v0 must not be live before it is defined (instr 0): {vars_at_0:?}",
        );
        let vars_at_1 = live_vars_at(&bytes, "h", 1);
        assert!(
            vars_at_1.iter().any(|v| v.name == "v0"),
            "v0 must be live at its defining instruction (instr 1): {vars_at_1:?}",
        );
    }

    #[test]
    fn ssa_temp_live_until_end_of_function() {
        // v0 defined at instr 0; n_instrs=3; must be live at instrs 0, 1, 2.
        let m = make_module_with_fn(
            "k",
            vec![],
            vec![
                IIRInstr::new("const_i32", Some("v0".into()), vec![Operand::Int(1)], "i32"),
                IIRInstr::new("const_i32", Some("v1".into()), vec![Operand::Int(2)], "i32"),
                IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
            ],
        );
        let bytes = build_sidecar(&m, Path::new("x.twig"));
        for i in 0..3 {
            let vars = live_vars_at(&bytes, "k", i);
            assert!(
                vars.iter().any(|v| v.name == "v0"),
                "v0 must stay live until function end (checking instr {i})",
            );
        }
    }

    #[test]
    fn type_hint_preserved_for_variable() {
        // Verify the type_hint we provide for each variable is round-tripped.
        let m = make_module_with_fn(
            "typed",
            vec![("n", "i32")],
            vec![
                IIRInstr::new("const_i32", Some("c".into()), vec![Operand::Int(100)], "i32"),
                IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
            ],
        );
        let bytes = build_sidecar(&m, Path::new("x.twig"));
        let vars = live_vars_at(&bytes, "typed", 0);
        let n_type = vars.iter().find(|v| v.name == "n").map(|v| v.type_hint.as_str());
        assert_eq!(n_type, Some("i32"), "param 'n' should have type_hint 'i32'");
        let c_type = vars.iter().find(|v| v.name == "c").map(|v| v.type_hint.as_str());
        assert_eq!(c_type, Some("i32"), "temp 'c' should have type_hint 'i32'");
    }

    #[test]
    fn multiple_temps_get_sequential_registers() {
        // Three temps v0, v1, v2 defined at instructions 0, 1, 2 respectively.
        // No params → they receive reg indices 0, 1, 2 in definition order.
        //
        // To see all three live at the same time we query at instruction 3
        // (the `ret`): by then every temp's live_start has been passed.
        let m = make_module_with_fn(
            "seq",
            vec![],
            vec![
                IIRInstr::new("const_i32", Some("v0".into()), vec![Operand::Int(1)], "i32"), // instr 0
                IIRInstr::new("const_i32", Some("v1".into()), vec![Operand::Int(2)], "i32"), // instr 1
                IIRInstr::new("const_i32", Some("v2".into()), vec![Operand::Int(3)], "i32"), // instr 2
                IIRInstr::new("ret", None, vec![], "void"),                                  // instr 3
            ],
        );
        let bytes = build_sidecar(&m, Path::new("x.twig"));
        // Query at instr 3 — all three temps are live here (live_start ≤ 3 < live_end=4).
        let vars = live_vars_at(&bytes, "seq", 3);
        let reg_of = |n: &str| vars.iter().find(|v| v.name == n).map(|v| v.reg_index);
        assert_eq!(reg_of("v0"), Some(0), "v0 should be reg 0");
        assert_eq!(reg_of("v1"), Some(1), "v1 should be reg 1");
        assert_eq!(reg_of("v2"), Some(2), "v2 should be reg 2");
    }

    #[test]
    fn void_instructions_do_not_produce_variables() {
        // `ret` has no `dest` — should not appear in variable list.
        let m = make_module_with_fn(
            "v",
            vec![],
            vec![IIRInstr::new("ret", None, vec![], "void")],
        );
        let bytes = build_sidecar(&m, Path::new("x.twig"));
        // Only query at instr 0; the void ret itself should not be a variable.
        let vars = live_vars_at(&bytes, "v", 0);
        // No variables should be declared (no params, no dests).
        assert!(vars.is_empty(), "void function should declare no variables: {vars:?}");
    }

    #[test]
    fn no_variables_declared_for_function_with_no_params_and_no_dests() {
        let m = make_module_with_fn("empty_fn", vec![], vec![]);
        let bytes = build_sidecar(&m, Path::new("x.twig"));
        let idx = SidecarIndex::from_bytes(&bytes).expect("valid sidecar");
        // live_variables on empty function should return empty vec at any index.
        let vars = idx.reader().live_variables("empty_fn", 0);
        assert!(vars.is_empty());
    }

    #[test]
    fn compile_sidecar_includes_param_variables_for_named_function() {
        // End-to-end: compile a real Twig function with a named parameter and
        // verify the parameter appears in the sidecar's variable table.
        let (p, _g) = write_temp_twig("(define (sq x) (* x x))\n(sq 7)\n");
        let a = TwigDebugAdapter;
        let (_, bytes) = a.compile(&p, Path::new(".")).expect("compile ok");
        let idx = SidecarIndex::from_bytes(&bytes).expect("valid sidecar");
        let r = idx.reader();
        // Find the "sq" function and confirm "x" appears as a variable.
        // We check at instruction 0 since params are live from the start.
        let vars = r.live_variables("sq", 0);
        assert!(
            vars.iter().any(|v| v.name == "x"),
            "parameter 'x' of 'sq' must be declared as a variable; got: {vars:?}",
        );
    }
}
