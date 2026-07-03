/**
 * # `@coding-adventures/ml-framework-core`
 *
 * Idiomatic TypeScript Tensor + autograd built on the Rust matrix-cpu
 * engine.  v0.1.0 ships the bottom layer: a pure-TypeScript `Tensor`
 * class.  Future PRs add autograd (PR #2), forward op dispatch through
 * the workspace's `@coding-adventures/matrix-rust-napi` package (PR #3),
 * backward + end-to-end MLP test (PR #4), and v1.0.0 polish (PR #5).
 *
 * ## Usage
 *
 * ```ts
 * import { Tensor } from "@coding-adventures/ml-framework-core";
 *
 * const t = Tensor.zeros(2, 3);
 * console.log(t.shape);       // → [2, 3]
 * const t2 = t.add(1.0);      // element-wise + 1
 * console.log(t2.toArray());  // → [1, 1, 1, 1, 1, 1]
 * ```
 *
 * ## Layered architecture
 *
 * ```
 * @coding-adventures/ml-framework-core   ← this package (v0.1.0)
 *     ↓
 * @coding-adventures/matrix-rust-napi    ← TS wrapper for the Rust addon (v0.4.0)
 *     ↓
 * matrix-rust-napi (Rust cdylib)          ← exposes runGraphOnCpu via N-API (v0.3.0)
 *     ↓
 * node-bridge (Rust workspace crate)      ← zero-dep N-API wrapper (v0.1.0)
 *     ↓
 * matrix-ir-json → matrix-ir → matrix-runtime → matrix-cpu
 * ```
 *
 * v0.1.0 doesn't touch the Rust dispatch yet — every method here is pure
 * TypeScript.  The dispatch wiring lands in PR #3 once the autograd
 * machinery (PR #2) is in place.
 */

export { Tensor, inferShape, flattenToFloat32 } from "./tensor.js";
export type { Dtype, Shape, TensorOptions } from "./tensor.js";
export { Function, Identity, backwardImpl } from "./autograd.js";
export {
  AddOp,
  SubOp,
  MulOp,
  DivOp,
  NegOp,
  AbsOp,
  PowOp,
  MatMulOp,
  ReLUOp,
  SigmoidOp,
  TanhOp,
  GELUOp,
  SoftmaxOp,
  SumOp,
  MeanOp,
  EmbeddingOp,
  LayerNormOp,
  BatchNormOp,
  DropoutOp,
  Conv2DOp,
  MaxPool2DOp,
  BroadcastOp,
  DISPATCH_THRESHOLD,
  packF32Hex,
  unpackF32Hex,
  runEnvelope,
  broadcastShapes,
  broadcastDataTo,
  unbroadcastDataTo,
} from "./ops.js";
export { VERSION } from "./version.js";
export { getMode, setMode } from "./mode.js";
export type { Mode } from "./mode.js";
export { Optimizer, SGD, Adam } from "./optim.js";
export { Module, Linear, Sequential, Fn } from "./nn.js";
export { saveSafetensors, loadSafetensors } from "./safetensors.js";
export type { LoadResult } from "./safetensors.js";
