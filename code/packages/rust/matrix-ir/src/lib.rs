//! # `matrix-ir` — The Tensor Algebra IR
//!
//! `matrix-ir` is the upper IR of the matrix execution layer.  Domain
//! libraries (image processing, neural-network layers, signal processing,
//! BLAS) emit `matrix-ir` graphs that describe *what* to compute as a
//! directed acyclic graph of tensor algebra operations.  This crate says
//! nothing about *where* the computation happens; that is `compute-ir`'s
//! job (spec MX02).
//!
//! See `code/specs/MX01-matrix-ir.md` for the full specification and
//! `code/specs/MX00-matrix-execution-overview.md` for the architecture
//! context.
//!
//! ## What lives here
//!
//! - [`Tensor`], [`Shape`], [`DType`] — the data plane.
//! - [`Op`] — the 27-variant operation enum.
//! - [`Graph`], [`Constant`] — a complete computation.
//! - [`GraphBuilder`] — an ergonomic builder that allocates tensor ids,
//!   infers output shapes, and constructs a [`Graph`] step by step.
//! - [`validate`](Graph::validate) — structural and semantic checks.
//! - [`to_bytes`](Graph::to_bytes) / [`from_bytes`](Graph::from_bytes) —
//!   versioned binary wire format that round-trips without loss.
//!
//! ## A worked example
//!
//! Construct a single-layer `y = relu(x @ w + b)` graph:
//!
//! ```
//! use matrix_ir::{DType, GraphBuilder, Shape};
//!
//! let mut g = GraphBuilder::new();
//! let x = g.input(DType::F32, Shape::from(&[1, 4]));
//! let w = g.input(DType::F32, Shape::from(&[4, 2]));
//! let b = g.input(DType::F32, Shape::from(&[1, 2]));
//!
//! // Tiny zero constant for ReLU = max(x, 0).  Bytes are dtype-encoded LE.
//! let zero = g.constant(DType::F32, Shape::from(&[1, 2]), vec![0u8; 8]);
//!
//! let xw    = g.matmul(&x, &w);
//! let xwb   = g.add(&xw, &b);
//! let y     = g.max(&xwb, &zero);
//!
//! g.output(&y);
//! let graph = g.build().unwrap();
//! graph.validate().unwrap();
//! // Const (from constant()) + matmul + add + max = 4 ops
//! assert_eq!(graph.ops.len(), 4);
//! ```
//!
//! ## Zero dependencies
//!
//! Per the spec MX00 zero-dependency mandate, this crate uses only
//! `core`, `alloc`, and `std`.  No `serde`, no `bincode`, no `postcard`,
//! no async runtime.  The wire format is hand-rolled per spec MX03 and
//! is implementation-agnostic — any language with bytes can implement
//! a compatible encoder/decoder from the spec alone.

#![warn(rust_2018_idioms)]

mod tensor;
mod op;
mod graph;
mod builder;
mod validate;
mod wire;

pub use tensor::{DType, OpId, Shape, Tensor, TensorId};
pub use op::Op;
pub use graph::{Constant, Graph};
pub use builder::GraphBuilder;
pub use validate::IrError;

/// Wire format version produced by this crate's `to_bytes`.  Readers that
/// see a different version on the wire fail rather than misinterpret.
pub const WIRE_FORMAT_VERSION: u32 = 1;
