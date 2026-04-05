/**
 * @coding-adventures/layout-to-paint
 *
 * Converts a `PositionedNode` tree (output of any layout algorithm) into a
 * `PaintScene` (input to any paint-vm backend).
 *
 * See: code/specs/UI04-layout-to-paint.md
 */

export {
  layout_to_paint,
  colorToCss,
  type LayoutToPaintOptions,
  type PaintExt,
} from "./paint.js";
