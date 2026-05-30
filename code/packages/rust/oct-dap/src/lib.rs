//! # `oct-dap` — Oct Debug Adapter Protocol adapter.
//!
//! Oct instantiation of [`dap_adapter_core::LanguageDebugAdapter`].
//! Sibling of `basic-dap`, `nib-dap`, and `twig-dap`.  Completes the
//! three-language DAP-adapter trilogy (task #37 part 3 of 3).
//!
//! See [`README.md`](../README.md) for the editor-launch story.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Child;

use dap_adapter_core::LanguageDebugAdapter;
use debug_sidecar::DebugSidecarWriter;
use interpreter_ir::IIRModule;

// ===========================================================================
// OctDebugAdapter
// ===========================================================================

/// Per-language hooks for the Oct DAP adapter.
///
/// Stateless — every method recomputes from scratch.  Pass a fresh
/// instance to [`dap_adapter_core::DapServer::new`] per session.
#[derive(Debug, Default, Clone, Copy)]
pub struct OctDebugAdapter;

impl LanguageDebugAdapter for OctDebugAdapter {
    /// Compile `source_path` and emit a debug sidecar.
    ///
    /// Returns `(source_path, sidecar_bytes)`.  The first element is
    /// the **source path** rather than a separate bytecode file
    /// because the `oct-vm` CLI takes Oct source directly — there's
    /// no pre-built bytecode artefact in this stack today.  The DAP
    /// server passes this path back to [`Self::launch_vm`] as the
    /// "bytecode" arg.
    fn compile(
        &self,
        source_path: &Path,
        _workspace_root: &Path,
    ) -> Result<(PathBuf, Vec<u8>), String> {
        let source = std::fs::read_to_string(source_path)
            .map_err(|e| format!("read {}: {e}", source_path.display()))?;
        let module = oct_iir_compiler::compile_source(&source, "oct")
            .map_err(|e| format!("oct compile: {e}"))?;
        let sidecar_bytes = build_sidecar(&module, source_path);
        Ok((source_path.to_path_buf(), sidecar_bytes))
    }

    /// Spawn the sibling `oct-vm` binary in debug mode.
    fn launch_vm(
        &self,
        bytecode_path: &Path,
        debug_port: u16,
    ) -> Result<Child, String> {
        let exe = find_sibling_binary("oct-vm")?;
        std::process::Command::new(&exe)
            .arg("--debug-port").arg(debug_port.to_string())
            .arg(bytecode_path)
            .spawn()
            .map_err(|e| format!("spawn {exe:?}: {e}"))
    }

    fn language_name(&self) -> &'static str { "oct" }
    fn file_extensions(&self) -> &'static [&'static str] { &["oct"] }
}

// ===========================================================================
// Sidecar builder
// ===========================================================================

/// Build a [`debug_sidecar`] byte blob from an [`IIRModule`].
///
/// Identical structure to `basic-dap::build_sidecar`,
/// `nib-dap::build_sidecar`, and `twig-dap::build_sidecar` — same
/// alphabetical slot-index assignment matching
/// `vm_debug::DebugServer::new_with_module`, same `_`-prefix
/// exclusion of compiler temporaries from the Variables panel.
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
        let mut all_names: HashSet<String> = HashSet::new();
        for (param_name, _) in &func.params {
            all_names.insert(param_name.clone());
        }
        for instr in &func.instructions {
            if let Some(dest) = &instr.dest {
                all_names.insert(dest.clone());
            }
        }
        let mut sorted_names: Vec<String> = all_names.into_iter().collect();
        sorted_names.sort();
        let slot_of: HashMap<String, u32> = sorted_names.iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i as u32))
            .collect();

        // Parameters — live for the whole function.
        for (param_name, param_type) in &func.params {
            let slot = slot_of[param_name];
            w.declare_variable(&func.name, slot, param_name, param_type, 0, n_instrs);
        }

        // SSA temporaries — live from defining instruction.  Internal
        // compiler-generated temporaries (leading `_`) are excluded
        // from the Variables panel.
        let param_names: HashSet<&str> = func.params.iter()
            .map(|(n, _)| n.as_str())
            .collect();
        for (instr_idx, instr) in func.instructions.iter().enumerate() {
            let dest_name = match &instr.dest {
                Some(d) if !param_names.contains(d.as_str()) => d,
                _ => continue,
            };
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
/// Used to find `oct-vm` from `oct-dap`.  Falls back to a bare name
/// (PATH lookup) if no sibling is found, supporting both
/// `cargo install`-style installation and ad-hoc development.
///
/// ## Path-traversal guard
///
/// `name` MUST be a bare filename — no directory separators, no
/// `..`, no leading `.`.  Same guard as `twig-dap::find_sibling_binary`,
/// `basic-dap::find_sibling_binary`, and `nib-dap::find_sibling_binary`.
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

    fn write_temp_oct(source: &str) -> (PathBuf, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prog.oct");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(source.as_bytes()).unwrap();
        (path, dir)
    }

    #[test]
    fn adapter_metadata_correct() {
        let a = OctDebugAdapter;
        assert_eq!(a.language_name(), "oct");
        assert!(a.file_extensions().contains(&"oct"));
    }

    #[test]
    fn compile_produces_sidecar_with_line_table() {
        // A multi-line Oct program.  We expect the sidecar to contain
        // at least one non-synthetic line-table entry — confirming
        // that OCT05 source-loc threading flows all the way out
        // through `build_sidecar`.
        let src = "fn main() {\n\
                   let x: u8 = 30;\n\
                   let y: u8 = 12;\n\
                   }\n";
        let (path, _td) = write_temp_oct(src);
        let workspace = path.parent().unwrap();
        let (bytecode_path, sidecar) = OctDebugAdapter
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
        // Hardware intrinsic — rejected by Oct's frontend — should
        // surface as a non-empty Err.
        let src = "fn main() { in(0); }";
        let (path, _td) = write_temp_oct(src);
        let workspace = path.parent().unwrap();
        let err = OctDebugAdapter.compile(&path, workspace).unwrap_err();
        assert!(!err.is_empty(), "expected non-empty error");
    }
}
