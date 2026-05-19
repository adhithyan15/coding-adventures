/**
 * # Node entry point — routes `CpuMatrixBackend` through the Rust napi addon
 *
 * Re-exports everything from `matrix.ts` with one substitution: the
 * `CpuMatrixBackend` symbol is the napi-backed implementation, not
 * the pure-TS one.  Picked up by the `node` key in `package.json`'s
 * `exports` conditional.
 *
 * Consumers see a `Matrix` class, a `MatrixBackend` interface, and a
 * `CpuMatrixBackend` class — same names, same shapes.  The
 * implementation behind the class is the only thing that differs
 * between Node and browser.
 */

export {
  Matrix,
  type MatrixBackend,
  getMatrixBackend,
  setMatrixBackend,
  resetMatrixBackend,
} from "./matrix";

// Node-side: napi-backed CpuMatrixBackend.
export { CpuMatrixBackend } from "./backends/cpu-rust-napi";
