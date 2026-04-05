/**
 * @coding-adventures/layout-text-measure-canvas
 *
 * Accurate text measurer backed by `CanvasRenderingContext2D.measureText()`.
 * TypeScript/browser only (or Node.js with the `canvas` package).
 *
 * See: code/specs/UI09-layout-text-measure.md
 */

export {
  createCanvasMeasurer,
  fontSpecToCss,
  type CanvasContext2D,
  type TextMetricsLike,
} from "./measurer.js";
