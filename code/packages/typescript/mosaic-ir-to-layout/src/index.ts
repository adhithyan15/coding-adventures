/**
 * @coding-adventures/mosaic-ir-to-layout
 *
 * Converts a MosaicComponent IR (output of mosaic-analyzer) and resolved slot
 * values into a LayoutNode tree ready for layout-flexbox.
 *
 * See: code/specs/UI05-mosaic-ir-to-layout.md
 */

export {
  mosaic_ir_to_layout,
  mosaic_default_theme,
  type SlotValue,
  type SlotMap,
  type MosaicLayoutTheme,
} from "./converter.js";
