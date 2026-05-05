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
/// For each function:
/// - `begin_function(name, start=0, param_count=params.len())`
/// - For every `(instr_index, source_loc)` pair where `source_loc` is
///   non-synthetic (line ≠ 0), `record(...)` emits a row.
/// - `end_function(name, n_instrs)`
///
/// Variable declarations are not emitted — Twig's IIR doesn't carry user
/// variable names through to register IDs in a way that maps cleanly to
/// the sidecar's `(name, reg_index)` shape.  The DAP `variables` panel
/// will be empty until that mapping lands as a follow-up.
pub fn build_sidecar(module: &IIRModule, source_path: &Path) -> Vec<u8> {
    let mut w = DebugSidecarWriter::new();
    let path_str = source_path.to_string_lossy().to_string();
    let fid = w.add_source_file(&path_str, &[]);

    for func in &module.functions {
        let n_instrs = func.instructions.len();
        w.begin_function(&func.name, 0, func.params.len());
        for (idx, loc) in func.source_map.iter().enumerate() {
            // SourceLoc::SYNTHETIC is line=0, col=0 — skip; the sidecar
            // reader's DWARF-style "previous row" lookup covers
            // unmapped instructions naturally.
            if loc.line == 0 { continue; }
            w.record(&func.name, idx, fid, loc.line, loc.column);
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
pub fn find_sibling_binary(name: &str) -> Result<PathBuf, String> {
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
}
