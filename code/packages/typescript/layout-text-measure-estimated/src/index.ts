/**
 * @coding-adventures/layout-text-measure-estimated
 *
 * Fast, zero-dependency, deterministic text measurer using a fixed
 * character-width model. Suitable for CI, server-side layout, headless
 * environments, and first-pass progressive rendering.
 *
 * See: code/specs/UI09-layout-text-measure.md
 */

export { createEstimatedMeasurer, type EstimatedMeasurerOptions } from "./measurer.js";
