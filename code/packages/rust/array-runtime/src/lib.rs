//! # array-runtime — the shared N-D numeric-array substrate
//!
//! `array-runtime` is **Wave 0** of the historical math-languages roadmap
//! (`code/specs/HML00`). It is the dense, rectangular, column-major numeric-array
//! value model that every numerical/array-language frontend — MATLAB first, then
//! Octave/Scilab/APL/J — sits on, the array-family analogue of what the shared
//! S evaluator is to R. Build the value model and its operations once; each
//! language becomes a thin lexer/parser/runtime on top.
//!
//! ## The two halves
//!
//! 1. **[`Array`] + [`ops`]** — a correct, dependency-free CPU **reference**
//!    implementation of the core operations (elementwise arithmetic with scalar
//!    broadcasting, `matmul`, `transpose`, reductions). This is what produces
//!    values *today*.
//!
//! 2. **[`accel`]** — the GPU-dispatch brain. Each op lowers to a `matrix-ir`
//!    graph and is handed to `matrix-runtime`'s cost-based planner, which places
//!    each op on the cheapest available backend (CPU/CUDA/Metal) from a FLOP +
//!    transfer cost model. The placement is observable via
//!    [`accel::plan_backend`], so the dispatch decision is tested today — even
//!    though *executing* the placed graph through a real GPU executor is a later
//!    MA item. Until then the reference path guarantees correct results and the
//!    planner integration guarantees the dispatch decision is right. When the
//!    execution layer lands, compute switches from the reference path to the
//!    planned graph with **no public API change**.
//!
//! ```
//! use coding_adventures_array_runtime::{Array, ops};
//!
//! let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
//! let b = Array::eye(2);
//! let c = ops::matmul(&a, &b).unwrap(); // a · I == a
//! assert_eq!(c.data(), a.data());
//! ```

pub mod accel;
pub mod ops;
pub mod value;

pub use accel::{plan_backend, Kernel};
pub use ops::BinOp;
pub use value::Array;
