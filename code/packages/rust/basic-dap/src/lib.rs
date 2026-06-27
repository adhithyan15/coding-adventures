//! # `basic-dap` — Dartmouth BASIC Debug Adapter Protocol adapter.
//!
//! BASIC instantiation of [`dap_adapter_core::LanguageDebugAdapter`].
//! Modelled on `twig-dap` but rooted in the generic `vm-debug`
//! substrate so it doesn't depend on `twig-vm`.
//!
//! See [`README.md`](../README.md) for the editor-launch story.
//!
//! ## What this crate does
//!
//! - [`BasicDebugAdapter::compile`] runs
//!   `dartmouth_basic_iir_compiler::compile_source` on the requested
//!   file, then walks the resulting `IIRFunction::source_map` to
//!   emit a [`debug_sidecar`]-format byte blob suitable for
//!   [`dap_adapter_core::SidecarIndex`].
//! - [`BasicDebugAdapter::launch_vm`] spawns the sibling `basic-vm`
//!   binary with `--debug-port <PORT>` so the adapter can connect
//!   over TCP.
//! - The `basic-dap` binary wires the above into
//!   [`dap_adapter_core::DapServer`] so editors can drive the BASIC
//!   debugger over stdio.
//!
//! ## Architecture
//!
//! ```text
//! Editor (VS Code / Neovim / …)
//!     │  DAP / JSON over stdio
//!     ▼
//! basic-dap binary  (bin/basic_dap.rs in this crate)
//!     │  DapServer::new(BasicDebugAdapter).run_stdio()
//!     ▼
//! dap-adapter-core  (DAP message handling, breakpoints, stepping)
//!     │  BasicDebugAdapter::{compile, launch_vm}
//!     ▼
//! basic-vm --debug-port N  (spawned subprocess; speaks vm-debug protocol over TCP)
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Child;

use dap_adapter_core::LanguageDebugAdapter;
use debug_sidecar::DebugSidecarWriter;
use interpreter_ir::IIRModule;

// ===========================================================================
// BasicDebugAdapter
// ===========================================================================

/// Per-language hooks for the BASIC DAP adapter.
///
/// Stateless — every method recomputes from scratch.  Pass a fresh
/// instance to [`dap_adapter_core::DapServer::new`] per session.
#[derive(Debug, Default, Clone, Copy)]
pub struct BasicDebugAdapter;

impl LanguageDebugAdapter for BasicDebugAdapter {
    /// Compile `source_path` and emit a debug sidecar.
    ///
    /// Returns `(source_path, sidecar_bytes)`.  The first element is
    /// the **source path** rather than a separate bytecode file
    /// because the `basic-vm` CLI takes BASIC source directly —
    /// there's no pre-built bytecode artefact in this stack today.
    /// The DAP server passes this path back to [`Self::launch_vm`]
    /// as the "bytecode" arg.
    ///
    /// The `sidecar_bytes` are produced by walking `source_map` for
    /// each compiled function and emitting one row per
    /// non-synthetic instruction.  See [`build_sidecar`] for
    /// details.
    fn compile(
        &self,
        source_path: &Path,
        _workspace_root: &Path,
    ) -> Result<(PathBuf, Vec<u8>), String> {
        let source = std::fs::read_to_string(source_path)
            .map_err(|e| format!("read {}: {e}", source_path.display()))?;
        let module = dartmouth_basic_iir_compiler::compile_source(&source, "basic")
            .map_err(|e| format!("basic compile: {e}"))?;
        let sidecar_bytes = build_sidecar(&module, source_path);
        Ok((source_path.to_path_buf(), sidecar_bytes))
    }

    /// Spawn the sibling `basic-vm` binary in debug mode.
    ///
    /// Looks for `basic-vm` next to the running `basic-dap`
    /// executable and falls back to a PATH lookup.  If neither
    /// finds it (which is the current state on machines without a
    /// `basic-vm` build), `spawn` returns the standard
    /// "executable not found" error from the OS — surfaced verbatim
    /// to the caller.
    fn launch_vm(
        &self,
        bytecode_path: &Path,
        debug_port: u16,
    ) -> Result<Child, String> {
        let exe = find_sibling_binary("basic-vm")?;
        std::process::Command::new(&exe)
            .arg("--debug-port").arg(debug_port.to_string())
            .arg(bytecode_path)
            .spawn()
            .map_err(|e| format!("spawn {exe:?}: {e}"))
    }

    fn language_name(&self) -> &'static str { "basic" }
    fn file_extensions(&self) -> &'static [&'static str] { &["bas", "basic"] }
}

// ===========================================================================
// Sidecar builder
// ===========================================================================

/// Build a [`debug_sidecar`] byte blob from an [`IIRModule`].
///
/// One source file is registered (the absolute path of
/// `source_path`).  For each function in the module:
///
/// 1. `begin_function(name, start=0, param_count=params.len())`
/// 2. **Line table** — for every `(instr_index, source_loc)` pair
///    where the loc is non-synthetic (line ≠ 0), `record(...)` emits
///    a row.
/// 3. **Variable declarations** — see below.
/// 4. `end_function(name, n_instrs)`
///
/// ## Variable declarations
///
/// Identical to the `twig-dap::build_sidecar` strategy.  The
/// `vm-debug::DebugServer::new_with_module` slot assignment is the
/// canonical one: it collects every variable name from a function
/// (params + instruction `dest` fields), sorts alphabetically, and
/// hands out slot indices in that order — so this sidecar walks the
/// same algorithm to keep `reg_index` values aligned with what
/// `get_slot(slot)` will return at runtime.
///
/// **Internal temporaries are excluded.**  The BASIC compiler
/// prefixes its synthetic temporaries with `_` (`_t0`, `_for_0_test`,
/// …).  These are compiler-implementation details and do not appear
/// in the VS Code Variables panel.  User-visible slot indices remain
/// correct because the sidecar and the VM both use the *full*
/// alphabetically-sorted name list (including `_`-prefixed names)
/// when mapping `get_slot` indices.
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
        // The vm-debug DebugServer assigns `get_slot` indices by sorting
        // ALL variable names of the current function alphabetically.  We
        // must use the exact same ordering here so that the slot index
        // in the sidecar matches what `get_slot(slot)` will return.

        // Step 1 — collect unique names.
        let mut all_names: HashSet<String> = HashSet::new();
        for (param_name, _) in &func.params {
            all_names.insert(param_name.clone());
        }
        for instr in &func.instructions {
            if let Some(dest) = &instr.dest {
                all_names.insert(dest.clone());
            }
        }

        // Step 2 — sort alphabetically.
        let mut sorted_names: Vec<String> = all_names.into_iter().collect();
        sorted_names.sort();

        // Step 3 — name → slot_index.
        let slot_of: HashMap<String, u32> = sorted_names.iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i as u32))
            .collect();

        // Step 4a — emit parameters (live for the whole function).
        for (param_name, param_type) in &func.params {
            let slot = slot_of[param_name];
            w.declare_variable(&func.name, slot, param_name, param_type, 0, n_instrs);
        }

        // Step 4b — emit SSA temporaries (live from defining instruction).
        let param_names: HashSet<&str> = func.params.iter()
            .map(|(n, _)| n.as_str())
            .collect();
        for (instr_idx, instr) in func.instructions.iter().enumerate() {
            let dest_name = match &instr.dest {
                Some(d) if !param_names.contains(d.as_str()) => d,
                _ => continue,
            };
            // Skip compiler-internal temporaries (leading `_`).
            if dest_name.starts_with('_') { continue; }
            let slot = match slot_of.get(dest_name) {
                Some(&s) => s,
                None => continue,
            };
            w.declare_variable(
                &func.name,
                slot,
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

// ===========================================================================
// Binary discovery
// ===========================================================================

/// Locate a sibling binary next to the currently-running executable.
///
/// Used to find `basic-vm` from `basic-dap`.  Falls back to a bare
/// name (PATH lookup) if no sibling is found, supporting both
/// `cargo install`-style installation and ad-hoc development.
///
/// ## Path-traversal guard
///
/// `name` MUST be a bare filename — no directory separators, no
/// `..`, no leading `.`.  This prevents a future caller from
/// accidentally (or maliciously) constructing a path that escapes
/// the current executable's directory.  Today the only call site
/// uses the hardcoded literal `"basic-vm"`, but the guard hardens
/// against drift — same shape as `twig-dap::find_sibling_binary`.
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

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_basic(source: &str) -> (PathBuf, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prog.bas");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(source.as_bytes()).unwrap();
        (path, dir)
    }

    #[test]
    fn adapter_metadata_correct() {
        let a = BasicDebugAdapter;
        assert_eq!(a.language_name(), "basic");
        assert!(a.file_extensions().contains(&"bas"));
        assert!(a.file_extensions().contains(&"basic"));
    }

    #[test]
    fn compile_produces_sidecar_with_line_table() {
        // A 3-line BASIC program.  We expect the sidecar to contain
        // at least one non-synthetic line-table entry — confirming
        // that BASIC05 source-loc threading flows all the way out
        // through `build_sidecar`.
        let src = "10 LET A = 30\n\
                   20 LET B = 12\n\
                   30 PRINT A\n\
                   40 END\n";
        let (path, _td) = write_temp_basic(src);
        let workspace = path.parent().unwrap();
        let (bytecode_path, sidecar) = BasicDebugAdapter
            .compile(&path, workspace).expect("compile ok");
        assert_eq!(bytecode_path, path);
        assert!(!sidecar.is_empty(), "sidecar bytes should not be empty");
    }

    #[test]
    fn find_sibling_binary_rejects_path_traversal() {
        for bad in &["", ".", "..", "/etc/passwd", "..\\evil.exe",
                     "C:\\Windows\\System32\\evil",
                     "foo\0bar"]
        {
            let r = find_sibling_binary(bad);
            assert!(r.is_err(), "should reject {bad:?}, got {r:?}");
        }
    }

    #[test]
    fn compile_propagates_compile_errors() {
        // A string literal in `PRINT` is still unsupported (waits for LANG77 /
        // E4 strings) — it should surface as a non-empty Err. (GOSUB used to be
        // the rejection here, but BA1 made GOSUB/RETURN compile.)
        let src = "10 PRINT \"HELLO\"\n20 END\n";
        let (path, _td) = write_temp_basic(src);
        let workspace = path.parent().unwrap();
        let err = BasicDebugAdapter.compile(&path, workspace).unwrap_err();
        assert!(!err.is_empty(), "expected non-empty error");
    }
}
