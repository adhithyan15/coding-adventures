/**
 * @coding-adventures/format-doc-to-paint
 *
 * This package is the first concrete bridge from the `format-doc` algebra to
 * the paint stack. It turns a `DocLayoutTree` into a `PaintScene` made from
 * pre-positioned glyph runs. The actual text output then comes from
 * `paint-vm-ascii`.
 */

import type { Doc, DocLayoutTree, LayoutOptions } from "@coding-adventures/format-doc";
import { layoutDoc } from "@coding-adventures/format-doc";
import type { PaintGlyphRun, PaintScene } from "@coding-adventures/paint-instructions";
import { paintScene } from "@coding-adventures/paint-instructions";

/** Package version, mirrored in tests for smoke coverage. */
export const VERSION = "0.1.0";

/** Options for converting a `DocLayoutTree` into a `PaintScene`. */
export interface DocPaintOptions {
  background?: string;
  fontRef?: string;
  fontSize?: number;
  fill?: string;
  sceneId?: string;
  metadata?: Record<string, string | number | boolean>;
}

/**
 * Convert a realized `DocLayoutTree` into a `PaintScene`.
 *
 * The layout tree already resolved line breaks and span placement in monospace
 * cell units, so this function only needs to translate each span into a
 * pre-positioned glyph run.
 */
export function docLayoutToPaintScene(
  layout: DocLayoutTree,
  options: DocPaintOptions = {},
): PaintScene {
  const instructions: PaintGlyphRun[] = [];
  const fontRef = options.fontRef ?? "monospace";
  const fontSize = options.fontSize ?? 1;
  const fill = options.fill ?? "#000000";

  for (const line of layout.lines) {
    for (const span of line.spans) {
      const glyphs: PaintGlyphRun["glyphs"] = [];
      let offset = 0;

      for (const char of Array.from(span.text)) {
        glyphs.push({
          glyph_id: char.codePointAt(0)!,
          x: span.column + offset,
          y: line.row * layout.lineHeight,
        });
        offset += 1;
      }

      if (glyphs.length > 0) {
        instructions.push({
          kind: "glyph_run",
          glyphs,
          font_ref: fontRef,
          font_size: fontSize,
          fill,
        });
      }
    }
  }

  return paintScene(
    layout.width,
    layout.height,
    options.background ?? "transparent",
    instructions,
    { id: options.sceneId, metadata: options.metadata },
  );
}

/** Convenience helper: realize a `Doc` and immediately convert it to `PaintScene`. */
export function docToPaintScene(
  doc: Doc,
  layoutOptions: LayoutOptions,
  paintOptions: DocPaintOptions = {},
): PaintScene {
  return docLayoutToPaintScene(layoutDoc(doc, layoutOptions), paintOptions);
}
