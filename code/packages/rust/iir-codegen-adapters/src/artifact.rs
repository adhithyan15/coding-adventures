//! `IIRBackendArtifact` — the unified output type for all four IIR backends.
//!
//! When you call [`crate::compile_iir`], the result is one of four concrete
//! artifact types depending on which backend was used.  Rather than returning
//! `Box<dyn Any>` (which requires downcasting) or four separate entry points
//! (which forces callers to enumerate backends themselves), this module defines
//! a closed `enum` that wraps all four artifact types.
//!
//! ## Pattern-matching example
//!
//! ```rust
//! use iir_codegen_adapters::{compile_iir, IIRBackendArtifact};
//! use interpreter_ir::{IIRModule, IIRFunction, IIRInstr};
//!
//! let fn_ = IIRFunction::new("main", vec![], "void",
//!     vec![IIRInstr::new("ret_void", None, vec![], "void")]);
//! let module = IIRModule {
//!     name: "demo".into(), functions: vec![fn_],
//!     entry_point: Some("main".into()), language: "test".into(),
//!     exports: vec![], imports: vec![],
//! };
//!
//! let artifact = compile_iir(&module, "iir-wasm").unwrap();
//! match artifact {
//!     IIRBackendArtifact::Wasm(ref m) => println!("Got WASM module with {} type(s)", m.types.len()),
//!     other => panic!("unexpected backend: {}", other.backend_name()),
//! }
//! ```

use std::fmt;

// Use the re-exports from the iir-to-* crates so callers of this crate do not
// need to add the underlying vm-specific crates as direct dependencies.
//
// - `iir_to_beam::BEAMModule`         re-exported by iir-to-beam
// - `wasm_types::WasmModule`           imported directly (not re-exported by iir-to-wasm)
// - `iir_to_jvm_class_file::JvmClassFile`  re-exported by iir-to-jvm-class-file
// - `iir_to_cil_bytecode::CILProgramArtifact` re-exported by iir-to-cil-bytecode
use iir_to_beam::BEAMModule;
use wasm_types::WasmModule;
use iir_to_jvm_class_file::JvmClassFile;
use iir_to_cil_bytecode::CILProgramArtifact;

// ===========================================================================
// IIRBackendArtifact
// ===========================================================================

/// The output of compiling an `IIRModule` to one of the four IIR backends.
///
/// Each variant wraps the natural artifact type produced by that backend.
/// The variant name matches the backend identifier returned by
/// [`crate::list_iir_backends()`].
///
/// ## Accessing the inner value
///
/// Use pattern matching for exhaustive handling, or the typed accessor methods
/// (`as_beam()`, `as_wasm()`, `as_jvm()`, `as_clr()`) when you know which
/// backend was used.
///
/// ## Debug
///
/// `Debug` is implemented manually because not all inner artifact types
/// implement `Debug` (e.g. `CILProgramArtifact`).  The output is the backend
/// name string, e.g. `"IIRBackendArtifact::iir-wasm"`.
pub enum IIRBackendArtifact {
    /// BEAM bytecode module — produced by the `"iir-beam"` backend.
    ///
    /// The inner `BEAMModule` can be encoded to a `.beam` file via
    /// `ir_to_beam::encoder::encode_beam`.
    Beam(BEAMModule),

    /// WebAssembly module — produced by the `"iir-wasm"` backend.
    ///
    /// The inner `WasmModule` can be encoded to a `.wasm` binary via
    /// `wasm_module_encoder::encode_module`.
    Wasm(WasmModule),

    /// JVM class file — produced by the `"iir-jvm"` backend.
    ///
    /// The inner `JvmClassFile` contains method bytecodes and a constant pool.
    Jvm(JvmClassFile),

    /// CIL program artifact — produced by the `"iir-clr"` backend.
    ///
    /// The inner `CILProgramArtifact` contains method CIL byte sequences and
    /// an assembly manifest.
    Clr(CILProgramArtifact),
}

impl IIRBackendArtifact {
    // ── Accessor helpers ─────────────────────────────────────────────────────────

    /// Return a reference to the inner `BEAMModule`, or `None` if this is a
    /// different backend artifact.
    ///
    /// # Example
    /// ```
    /// # use iir_codegen_adapters::{compile_iir, IIRBackendArtifact};
    /// # use interpreter_ir::{IIRModule, IIRFunction, IIRInstr};
    /// # let fn_ = IIRFunction::new("main", vec![], "void",
    /// #     vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    /// # let module = IIRModule { name: "d".into(), functions: vec![fn_], entry_point: Some("main".into()), language: "t".into(), exports: vec![], imports: vec![] };
    /// let art = compile_iir(&module, "iir-beam").unwrap();
    /// assert!(art.as_beam().is_some());
    /// assert!(art.as_wasm().is_none());
    /// ```
    pub fn as_beam(&self) -> Option<&BEAMModule> {
        if let IIRBackendArtifact::Beam(m) = self { Some(m) } else { None }
    }

    /// Return a reference to the inner `WasmModule`, or `None` if this is a
    /// different backend artifact.
    pub fn as_wasm(&self) -> Option<&WasmModule> {
        if let IIRBackendArtifact::Wasm(m) = self { Some(m) } else { None }
    }

    /// Return a reference to the inner `JvmClassFile`, or `None` if this is a
    /// different backend artifact.
    pub fn as_jvm(&self) -> Option<&JvmClassFile> {
        if let IIRBackendArtifact::Jvm(m) = self { Some(m) } else { None }
    }

    /// Return a reference to the inner `CILProgramArtifact`, or `None` if
    /// this is a different backend artifact.
    pub fn as_clr(&self) -> Option<&CILProgramArtifact> {
        if let IIRBackendArtifact::Clr(m) = self { Some(m) } else { None }
    }

    // ── Metadata ─────────────────────────────────────────────────────────────────

    /// The stable backend identifier string for this artifact's origin.
    ///
    /// Matches one of the names returned by [`crate::list_iir_backends()`]:
    ///
    /// | Variant | `backend_name()` |
    /// |---------|-----------------|
    /// | `Beam`  | `"iir-beam"`    |
    /// | `Wasm`  | `"iir-wasm"`    |
    /// | `Jvm`   | `"iir-jvm"`     |
    /// | `Clr`   | `"iir-clr"`     |
    pub fn backend_name(&self) -> &'static str {
        match self {
            IIRBackendArtifact::Beam(_) => "iir-beam",
            IIRBackendArtifact::Wasm(_) => "iir-wasm",
            IIRBackendArtifact::Jvm(_)  => "iir-jvm",
            IIRBackendArtifact::Clr(_)  => "iir-clr",
        }
    }
}

// ---------------------------------------------------------------------------
// Debug (manual — CILProgramArtifact doesn't derive Debug)
// ---------------------------------------------------------------------------

impl fmt::Debug for IIRBackendArtifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IIRBackendArtifact::{}", self.backend_name())
    }
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------
//
// Show a concise summary: backend name + one key metric so it is immediately
// obvious what type of artifact this is and how large it is.

impl fmt::Display for IIRBackendArtifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IIRBackendArtifact::Beam(m) => {
                // BEAMModule tracks atoms and exports (not functions directly).
                write!(f, "Beam(atoms={}, exports={})", m.atoms.len(), m.exports.len())
            }
            IIRBackendArtifact::Wasm(m) => {
                write!(f, "Wasm(types={}, functions={})", m.types.len(), m.functions.len())
            }
            IIRBackendArtifact::Jvm(m) => {
                write!(f, "Jvm(class={:?}, methods={})", m.this_class_name, m.methods.len())
            }
            IIRBackendArtifact::Clr(m) => {
                write!(f, "Clr(methods={})", m.methods.len())
            }
        }
    }
}

// ===========================================================================
// Unit tests
// ===========================================================================

// Unit tests for IIRBackendArtifact live in tests/test_adapters.rs because
// constructing the inner artifact types directly requires non-trivial setup
// (e.g. CILProgramArtifact needs a Box<dyn CILTokenProvider>).
// The integration tests obtain real artifacts via compile_iir() instead.
