//! `debug-sidecar` — source-location companion for the IIR pipeline (LANG13).
//!
//! This crate provides a compact, JSON-backed sidecar that maps IIR instruction
//! indices back to their original source locations and tracks variable liveness.
//! It is the bridge between the compiler and all downstream debugger/insight tools:
//!
//! ```text
//! Compiler (emits IIRInstr)
//!   │ calls DebugSidecarWriter::record()
//!   │
//!   ↓ DebugSidecarWriter::finish() → Vec<u8>   (opaque bytes)
//!   │
//!   ├──→ written to .sidecar file alongside .aot binary
//!   │
//! Debugger / native-debug-info
//!   │ calls DebugSidecarReader::new(bytes)
//!   │
//!   ├── lookup("fib", 7)           → SourceLocation("fib.tetrad:3:5")
//!   ├── find_instr("fib.tetrad", 3) → Some(7)
//!   └── live_variables("fib", 7)   → [Variable { name: "n", … }]
//! ```
//!
//! # Public API
//!
//! - [`DebugSidecarWriter`] — append-only builder; call [`finish`][DebugSidecarWriter::finish]
//!   to serialise.
//! - [`DebugSidecarReader`] — query engine; answers lookup/find_instr/live_variables.
//! - [`SourceLocation`] — a frozen `(file, line, col)` triple.
//! - [`Variable`] — a register binding with name, type hint, and live range.
//!
//! # Quick start
//!
//! ```
//! use debug_sidecar::{DebugSidecarWriter, DebugSidecarReader};
//!
//! let mut w = DebugSidecarWriter::new();
//! let fid = w.add_source_file("fibonacci.tetrad", b"");
//! w.begin_function("fibonacci", 0, 1);
//! w.declare_variable("fibonacci", 0, "n", "any", 0, 12);
//! w.record("fibonacci", 0, fid, 3, 5);
//! w.end_function("fibonacci", 12);
//!
//! let sidecar = w.finish();
//!
//! let r = DebugSidecarReader::new(&sidecar).unwrap();
//! let loc = r.lookup("fibonacci", 0).unwrap();
//! assert_eq!(loc.to_string(), "fibonacci.tetrad:3:5");
//! ```

pub mod reader;
pub mod types;
pub mod writer;

pub use reader::{DebugSidecarReader, LineRow};
pub use types::{SourceLocation, Variable};
pub use writer::DebugSidecarWriter;
