/**
 * CPU Pipeline -- a configurable N-stage instruction pipeline simulator.
 *
 * This package manages the FLOW of instructions through pipeline stages.
 * It does NOT interpret instructions -- that is the ISA decoder's job.
 *
 * The pipeline moves "tokens" (representing instructions) through stages,
 * handling normal advancement, stalls, flushes, and statistics.
 *
 * @example
 * ```ts
 * import { Pipeline, classic5Stage } from "@coding-adventures/cpu-pipeline";
 *
 * const pipeline = Pipeline.create(
 *   classic5Stage(),
 *   (pc) => memory[pc / 4],           // fetch
 *   (raw, tok) => { ... return tok; }, // decode
 *   (tok) => { ... return tok; },      // execute
 *   (tok) => { ... return tok; },      // memory
 *   (tok) => { ... },                  // writeback
 * );
 *
 * const stats = pipeline.run(1000);
 * console.log(`IPC: ${stats.ipc().toFixed(3)}`);
 * ```
 */

export { Pipeline } from "./pipeline.js";
export { PipelineStats, snapshotToString } from "./snapshot.js";
export type { PipelineSnapshot } from "./snapshot.js";
export {
  HazardAction,
  StageCategory,
  classic5Stage,
  cloneToken,
  deep13Stage,
  newBubble,
  newToken,
  noHazard,
  numStages,
  tokenToString,
  validateConfig,
} from "./token.js";
export type {
  DecodeFunc,
  ExecuteFunc,
  FetchFunc,
  HazardFunc,
  HazardResponse,
  MemoryFunc,
  PipelineConfig,
  PipelineStage,
  PipelineToken,
  PredictFunc,
  WritebackFunc,
} from "./token.js";
