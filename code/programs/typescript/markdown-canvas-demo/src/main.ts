/**
 * Markdown → Canvas demo
 *
 * Pipeline:
 *
 *   1. Parse CommonMark            → Document AST              (@coding-adventures/commonmark)
 *   2. Document AST → LayoutNode   (document_ast_to_layout)
 *   3. LayoutNode → PositionedNode (layout_block + CanvasTextMeasurer, TXT03d)
 *   4. PositionedNode → PaintScene (layout_to_paint, textEmitMode: "text")
 *   5. PaintScene → canvas pixels  (paint-vm-canvas handling PaintText)
 *
 * The output does NOT touch the DOM. The only DOM elements in the page are
 * the editor <textarea> (input) and the <canvas> (output). Every pixel of
 * the rendered markdown lives inside the canvas.
 */

import { parse } from "@coding-adventures/commonmark";
import {
  document_ast_to_layout,
  document_default_theme,
} from "@coding-adventures/document-ast-to-layout";
import { layout_block } from "@coding-adventures/layout-block";
import { size_wrap, size_fill, type LayoutNode, type SizeValue } from "@coding-adventures/layout-ir";
import { createCanvasMeasurer } from "@coding-adventures/layout-text-measure-canvas";
import { layout_to_paint } from "@coding-adventures/layout-to-paint";
import { createCanvasVM } from "@coding-adventures/paint-vm-canvas";

/**
 * Document-ast-to-layout may emit nodes with `width: null` / `height: null`
 * (meaning "no hint from this property"). Layout-block's core loop always
 * dereferences `.kind`, which crashes on null. We normalize to sensible
 * defaults matching CSS block/inline flow:
 *
 *   - block nodes:  width → size_fill(), height → size_wrap()
 *   - inline nodes: width → size_wrap(), height → size_wrap()
 *
 * Inline nodes MUST NOT take size_fill() — that would make each word claim
 * the full container width and stack vertically, destroying the inline
 * formatting context. This is a compatibility shim until layout-block is
 * patched upstream.
 */
function normalizeNulls(node: LayoutNode): LayoutNode {
  const ext = node.ext as {
    block?: { display?: string };
    flex?: { direction?: string };
  } | undefined;
  const blockExt = ext?.block;
  const isInline = blockExt?.display === "inline";
  const width: SizeValue = node.width ?? (isInline ? size_wrap() : size_fill());
  const height: SizeValue = node.height ?? size_wrap();

  // Flex-row containers (list item "bullet + body" rows from document-ast-to-
  // layout) don't have a layout-block implementation — layout-block would
  // stack their children vertically. For the demo we coerce the row into an
  // inline formatting context by flattening the subtree: every text leaf
  // (from any depth) becomes a direct inline child of the row. The bullet
  // and body then flow side-by-side on one line exactly as CSS flex-row
  // would, because layout-block tokenizes inline text leaves word-by-word.
  const isFlexRow = ext?.flex?.direction === "row";
  if (isFlexRow) {
    const leaves: LayoutNode[] = [];
    for (const child of node.children) collectTextLeaves(child, leaves);
    return { ...node, width, height, children: leaves };
  }

  return {
    ...node,
    width,
    height,
    children: node.children.map(normalizeNulls),
  };
}

function collectTextLeaves(node: LayoutNode, out: LayoutNode[]): void {
  if (node.content !== null && node.content.kind === "text") {
    // Make sure this leaf is explicitly inline — layout-block's wrapInlineRuns
    // keys off ext.block.display.
    const existingExt = (node.ext ?? {}) as Record<string, unknown>;
    const nextExt = {
      ...existingExt,
      block: { ...(existingExt["block"] as object | undefined), display: "inline" },
    };
    out.push({ ...node, width: node.width ?? size_wrap(), height: node.height ?? size_wrap(), ext: nextExt });
    return;
  }
  for (const child of node.children) collectTextLeaves(child, out);
}

const SAMPLE_MARKDOWN = `# Markdown on Canvas

This document is **rendered directly into an HTML canvas** — no DOM
elements, no \`<h1>\` or \`<p>\` tags. The browser's Canvas 2D context
handles the final text rasterization via \`ctx.fillText\`.

## The pipeline

1. Parse CommonMark into a Document AST
2. Convert the Document AST to a LayoutNode tree
3. Run layout-block with a CanvasTextMeasurer (spec TXT03d)
4. Emit PaintText instructions via layout-to-paint
5. Dispatch through paint-vm-canvas onto the canvas

## Why a new paint instruction?

Canvas 2D does not expose glyph IDs to JavaScript — only strings go
into fillText. The usual PaintGlyphRun instruction carries pre-shaped
glyph indices, which canvas cannot consume without violating the
font-binding invariant. PaintText is the honest representation: the
layout engine measures, but shaping happens inside the browser at
paint time.

Arrows → work, and so do emoji 🎉. The browser handles font fallback
internally; the paint IR never sees it.

Try editing this markdown on the left.
`;

async function main() {
  const editor = document.getElementById("editor") as HTMLTextAreaElement;
  const canvas = document.getElementById("canvas") as HTMLCanvasElement;
  const stats = document.getElementById("stats") as HTMLDivElement;
  const ctx = canvas.getContext("2d")!;

  // Wait for browser-loaded fonts before measuring — otherwise metrics reflect
  // the fallback font and line wraps are wrong when the real font arrives.
  if (document.fonts && document.fonts.ready) {
    await document.fonts.ready;
  }

  // A dedicated 1×1 canvas used only for measurement. Separating measurement
  // from the render context keeps ctx.font mutations from competing with the
  // paint VM's own ctx.font writes.
  const measureCanvas = document.createElement("canvas");
  measureCanvas.width = 1;
  measureCanvas.height = 1;
  const measureCtx = measureCanvas.getContext("2d")!;
  const measurer = createCanvasMeasurer(measureCtx);

  const vm = createCanvasVM();
  const theme = document_default_theme();

  function render(markdown: string) {
    const t0 = performance.now();

    // 1. Parse
    const doc = parse(markdown);

    // 2. Document AST → LayoutNode (then normalize null sizes)
    const rawTree = document_ast_to_layout(doc, theme);
    const tree = normalizeNulls(rawTree);

    // 3. Run block layout using the canvas-backed measurer
    const maxWidth = canvas.clientWidth || 800;
    const positioned = layout_block(
      tree,
      { maxWidth, maxHeight: Infinity, minWidth: 0, minHeight: 0 },
      measurer,
    );

    // Grow the canvas to fit the laid-out height — keeps scrolling sensible
    // for long documents.
    const laidOutHeight = Math.ceil(positioned.height + 32);
    if (canvas.height !== laidOutHeight) {
      canvas.height = laidOutHeight;
    }
    if (canvas.width !== maxWidth) {
      canvas.width = maxWidth;
    }

    // 4. Layout → PaintScene with the canvas-native emitter
    const scene = layout_to_paint([positioned], {
      width: canvas.width,
      height: canvas.height,
      background: { r: 255, g: 255, b: 255, a: 255 },
      devicePixelRatio: 1.0,
      textEmitMode: "text",
    });

    // 5. Paint
    vm.execute(scene, ctx);

    const elapsed = performance.now() - t0;
    const textCount = countKind(scene.instructions, "text");
    stats.textContent =
      `${scene.instructions.length} instructions · ${textCount} PaintText · ${elapsed.toFixed(1)}ms`;
  }

  editor.value = SAMPLE_MARKDOWN;
  render(editor.value);

  // Re-render on input. Debounce is unnecessary for a demo — layout is fast
  // enough at this document size.
  editor.addEventListener("input", () => render(editor.value));
  window.addEventListener("resize", () => render(editor.value));
}

function countKind(instructions: readonly { kind: string; children?: unknown }[], kind: string): number {
  let n = 0;
  for (const i of instructions) {
    if (i.kind === kind) n++;
    // Recurse into groups/layers/clips for accurate counts.
    const anyI = i as { children?: { kind: string }[] };
    if (Array.isArray(anyI.children)) {
      n += countKind(anyI.children, kind);
    }
  }
  return n;
}

main().catch((err) => {
  console.error(err);
  const stats = document.getElementById("stats");
  if (stats) stats.textContent = `error: ${(err as Error).message}`;
});
