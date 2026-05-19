/**
 * # Browser entry point — uses the pure-TS `CpuMatrixBackend`
 *
 * Re-exports the same symbols as `entry-node.ts` but routes
 * `CpuMatrixBackend` to the pure-TypeScript implementation
 * (`backends/cpu-pure-ts.ts`).  Picked up by the `browser` key in
 * `package.json`'s `exports` conditional, and also by the `default`
 * fallback for non-bundler ESM environments (Deno, future runtimes,
 * etc.).
 *
 * No napi addon load — bundlers honour the `browser` conditional
 * and won't try to resolve `@coding-adventures/matrix-rust-napi`.
 */

export {
  Matrix,
  type MatrixBackend,
  getMatrixBackend,
  setMatrixBackend,
  resetMatrixBackend,
} from "./matrix";

// Browser-side: the original pure-TS CpuMatrixBackend, unchanged.
export { CpuMatrixBackend } from "./backends/cpu-pure-ts";
