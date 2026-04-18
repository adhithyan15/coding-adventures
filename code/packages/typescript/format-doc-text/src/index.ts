/**
 * @coding-adventures/format-doc-text
 *
 * This package is the first backend for the `format-doc` algebra. It takes the
 * line/span layout produced by `layoutDoc()` and serializes it to a plain
 * string. Later backends can target paint instructions or editor data
 * structures while reusing the same `Doc` IR.
 */

import type { Doc, LayoutDocument, LayoutLine, LayoutOptions } from "@coding-adventures/format-doc";
import { layoutDoc } from "@coding-adventures/format-doc";

/** Package version, mirrored in tests for smoke coverage. */
export const VERSION = "0.1.0";

/** Serialize a realized `LayoutDocument` into a plain string. */
export function renderLayoutToText(layout: LayoutDocument): string {
  return layout.lines.map((line) => renderLine(line, layout)).join("\n");
}

/** Convenience helper: realize a `Doc` and immediately render it to text. */
export function renderDocToText(doc: Doc, options: LayoutOptions): string {
  return renderLayoutToText(layoutDoc(doc, options));
}

function renderLine(line: LayoutLine, layout: LayoutDocument): string {
  return renderIndent(line.indentColumns, layout) + line.spans.map((span) => span.text).join("");
}

function renderIndent(columns: number, layout: LayoutDocument): string {
  if (columns <= 0) {
    return "";
  }

  if (!layout.useTabs) {
    return " ".repeat(columns);
  }

  const tabWidth = layout.indentWidth;
  const tabs = Math.floor(columns / tabWidth);
  const spaces = columns % tabWidth;
  return "\t".repeat(tabs) + " ".repeat(spaces);
}
