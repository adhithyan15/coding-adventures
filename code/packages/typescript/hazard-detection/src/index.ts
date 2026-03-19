/**
 * Hazard Detection — keeping the CPU pipeline from tripping over itself.
 *
 * This package detects and resolves the three types of pipeline hazards
 * that occur in a pipelined CPU:
 *
 * - **Data hazards** (RAW): an instruction needs a register value that
 *   a previous instruction hasn't written yet. Resolved by forwarding
 *   or stalling.
 *
 * - **Control hazards**: a branch was mispredicted, so the pipeline
 *   fetched wrong instructions. Resolved by flushing.
 *
 * - **Structural hazards**: two instructions need the same hardware
 *   resource at the same time. Resolved by stalling.
 *
 * This package is standalone — it works with any pipeline implementation.
 * It only needs PipelineSlot descriptors of what's in each stage.
 *
 * Quick start:
 *
 *     import { HazardUnit, PipelineSlot, HazardAction } from "@coding-adventures/hazard-detection";
 *
 *     const unit = new HazardUnit();
 *     const result = unit.check(ifSlot, idSlot, exSlot, memSlot);
 *
 *     if (result.action === HazardAction.STALL) {
 *         // freeze the pipeline for result.stallCycles cycles
 *     }
 */

export { ControlHazardDetector } from "./controlHazard.js";
export { DataHazardDetector } from "./dataHazard.js";
export { HazardUnit, pickHighestPriority } from "./hazardUnit.js";
export { StructuralHazardDetector } from "./structuralHazard.js";
export {
  HazardAction,
  HazardResult,
  PipelineSlot,
} from "./types.js";
export type { HazardResultFields, PipelineSlotFields } from "./types.js";
