export type { IrPass } from "./protocol.js";
export { IrOptimizer } from "./optimizer.js";
export type { OptimizationResult } from "./optimizer.js";
export { ConstantFolder, DeadCodeEliminator, PeepholeOptimizer } from "./passes/index.js";
