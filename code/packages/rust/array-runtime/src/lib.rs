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
//!    [`accel::plan_backend`], so the dispatch decision is tested.
//!
//! 3. **[`exec`]** — end-to-end execution (MA-2, MXF-3). [`execute`] plans the
//!    lowered graph and **runs it** through `matrix-cpu`'s executor, returning
//!    real results from the same pipeline a GPU would use. As of MX12 / MXF-3,
//!    `matrix-ir` has a `DType::F64`, so `execute` lowers `f64` arrays to an
//!    **`F64`** graph and crosses the boundary as **8-byte** doubles — no `f32`
//!    round-trip. Its result is therefore **bit-exact** with the [`ops`]
//!    reference path, even on values `f32` cannot represent. ([`execute_sum`]
//!    adds an `f64` whole-array reduction on the same path; the legacy `f32`
//!    lowering is still reachable for `f32` callers and agrees only to `f32`
//!    precision, by construction.)
//!
//! ```
//! use coding_adventures_array_runtime::{Array, ops, execute, Kernel, BinOp};
//!
//! let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
//! let b = Array::eye(2);
//! let c = ops::matmul(&a, &b).unwrap(); // reference path: a · I == a
//! assert_eq!(c.data(), a.data());
//!
//! // The same op, executed end-to-end on the CPU executor.
//! let runtime = execute(Kernel::MatMul, &a, &b).unwrap();
//! assert_eq!(runtime.shape(), a.shape());
//! ```

pub mod accel;
pub mod exec;
pub mod ops;
pub mod value;

pub use accel::{plan_backend, Kernel};
pub use exec::{execute, execute_sum};
pub use matrix_ir::DType;
pub use ops::BinOp;
pub use value::Array;
