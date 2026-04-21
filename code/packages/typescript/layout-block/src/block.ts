/**
 * Block and Inline Flow Layout Algorithm
 *
 * Implements the block and inline flow layout model — the subset of CSS normal
 * flow needed to render structured documents (Markdown, rich text, emails).
 *
 * Pipeline position:
 *
 *   LayoutNode (with ext["block"]) + Constraints + TextMeasurer
 *       ↓  layout_block()
 *   PositionedNode
 *
 * Mental Model
 * ------------
 *
 * Think of this as typesetting. A page (block container) is a vertical stack
 * of paragraphs (block children). Each paragraph is a horizontal river of
 * words (inline children) that wraps at the page margin.
 *
 * Block formatting context rules:
 *
 *   "block"  → like a `<div>`: full-width box, stacks vertically
 *   "inline" → like a `<span>`: inline box that flows with text, wraps at edge
 *
 * When a container mixes block and inline children, inline children are
 * promoted into an anonymous block (CSS anonymous box generation). This keeps
 * the algorithm clean — every child of the block algorithm is a block box.
 *
 * Layout proceeds in three passes:
 *   1. Classify children (block vs inline)
 *   2. Wrap inline runs into anonymous block boxes
 *   3. Stack all block boxes top-to-bottom, collapsing adjacent vertical margins
 *
 * Supported features
 * ------------------
 *
 *   ✅ display: block / inline
 *   ✅ Anonymous block wrapping for mixed inline/block siblings
 *   ✅ Block stacking with margin collapsing (sibling margins only)
 *   ✅ Inline flow with word-level wrapping
 *   ✅ verticalAlign: baseline / top / middle / bottom within line boxes
 *   ✅ overflow: visible / hidden / scroll (sets ext["paint"])
 *   ✅ whiteSpace: normal / pre / nowrap
 *   ✅ padding on containers
 *   ✅ min/maxWidth, min/maxHeight
 *
 *   ❌ float / clear (future)
 *   ❌ position: absolute/fixed/sticky (future)
 *   ❌ display: flex / grid (use layout-flexbox / layout-grid)
 *   ❌ RTL / bidirectional text
 *   ❌ CSS columns
 */

import type {
  LayoutNode,
  Constraints,
  PositionedNode,
  TextMeasurer,
} from "@coding-adventures/layout-ir";
import {
  constraints_width,
  constraints_unconstrained,
} from "@coding-adventures/layout-ir";

// ── Types ──────────────────────────────────────────────────────────────────

/**
 * Extension data read from `ext["block"]`.
 *
 * All fields are optional; missing values use the CSS default.
 */
export interface BlockExt {
  /** Whether this node participates as a block or inline box. Default: "block" */
  display?: "block" | "inline";
  /** Overflow behaviour. Default: "visible" */
  overflow?: "visible" | "hidden" | "scroll";
  /** White-space handling. Default: "normal" */
  whiteSpace?: "normal" | "pre" | "nowrap";
  /** Word-break mode. Default: "normal" */
  wordBreak?: "normal" | "break-all";
  /** Inline vertical alignment within the line box. Default: "baseline" */
  verticalAlign?: "baseline" | "top" | "middle" | "bottom";
  /** Extra space added below block-level paragraphs, on top of margin.bottom */
  paragraphSpacing?: number;
  // float/clear are reserved but not yet implemented
  float?: "none" | "left" | "right";
  clear?: "none" | "left" | "right" | "both";
}

interface Size {
  width: number;
  height: number;
}

// ── Helpers ────────────────────────────────────────────────────────────────

/** Clamp a value between lo and hi (either bound may be undefined = no clamp) */
function clamp(v: number, lo: number | undefined, hi: number | undefined): number {
  if (lo !== undefined && v < lo) v = lo;
  if (hi !== undefined && v > hi) v = hi;
  return v;
}

/** Read the BlockExt for a node, returning an empty object if absent */
function blockExt(node: LayoutNode): BlockExt {
  return (node.ext["block"] as BlockExt | undefined) ?? {};
}

/** Is this node classified as inline (display: inline)? */
function isInline(node: LayoutNode): boolean {
  return blockExt(node).display === "inline";
}

/**
 * Measure an inline node's rendered size.
 *
 * For text leaves: calls the measurer with the available width so the measurer
 * can wrap the text and report the resulting height.
 *
 * For inline containers: recursively measures their children and returns the
 * bounding box of the measured inline run. This is a simplification — a full
 * engine would measure exactly like a sub-inline-format-context.
 */
function measureInline(node: LayoutNode, maxWidth: number, measurer: TextMeasurer): Size {
  const ext = blockExt(node);
  const whiteSpace = ext.whiteSpace ?? "normal";

  if (node.content !== null && node.content.kind === "text") {
    const effectiveMaxWidth = whiteSpace === "nowrap" ? Infinity : maxWidth;
    const r = measurer.measure(node.content.value, node.content.font, effectiveMaxWidth);
    return { width: Math.min(r.width, maxWidth === Infinity ? r.width : maxWidth), height: r.height };
  }

  if (node.content !== null && node.content.kind === "image") {
    // Images are block-sized for inline flow: use the node's fixed dimensions if available.
    const w = node.width.kind === "fixed" ? node.width.value : maxWidth;
    const h = node.height.kind === "fixed" ? node.height.value : w; // square fallback
    return { width: Math.min(w, maxWidth), height: h };
  }

  // Inline container: sum children widths, max height
  let totalW = 0;
  let maxH = 0;
  for (const child of node.children) {
    const s = measureInline(child, maxWidth - totalW, measurer);
    totalW += s.width;
    maxH = Math.max(maxH, s.height);
  }
  return { width: Math.min(totalW, maxWidth), height: maxH };
}

// ── Inline formatting context ──────────────────────────────────────────────

/**
 * A positioned word token for word-wrap.
 *
 * When we encounter a text node in inline flow, we split it into words and
 * treat each word as a separate inline unit. This is the CSS "word wrapping"
 * algorithm.
 */
interface InlineToken {
  node: LayoutNode;
  word: string;           // the word (or entire text if nowrap/pre)
  width: number;
  height: number;
  spaceAfter: number;     // width of space following this word (0 for last word)
}

/**
 * Lay out a sequence of inline nodes into line boxes.
 *
 * Returns a flat array of positioned nodes. Positions are relative to the
 * containing block's content area origin (0, 0).
 *
 * Algorithm:
 *   1. Tokenize inline nodes into words
 *   2. Pack words onto lines, wrapping when a word doesn't fit
 *   3. Apply vertical alignment within each completed line
 */
function layoutInlineRun(
  inlineNodes: LayoutNode[],
  availableWidth: number,
  measurer: TextMeasurer
): PositionedNode[] {
  // ── Step 1: Build token list ─────────────────────────────────────────────

  interface LineToken {
    node: LayoutNode;
    word: string;
    width: number;
    height: number;
    spaceAfter: number;
    verticalAlign: "baseline" | "top" | "middle" | "bottom";
  }

  const tokens: LineToken[] = [];

  for (const node of inlineNodes) {
    const ext = blockExt(node);
    const whiteSpace = ext.whiteSpace ?? "normal";
    const verticalAlign = ext.verticalAlign ?? "baseline";

    if (node.content !== null && node.content.kind === "text") {
      const text = node.content.value;
      const font = node.content.font;
      const spaceWidth = measurer.measure(" ", font, Infinity).width;

      if (whiteSpace === "nowrap" || whiteSpace === "pre") {
        // Treat the whole text as a single token (no wrapping)
        const s = measurer.measure(text, font, Infinity);
        tokens.push({ node, word: text, width: s.width, height: s.height, spaceAfter: 0, verticalAlign });
      } else {
        // Split on whitespace; each word is a separate token. A trailing space
        // on the original leaf (e.g. "is " in "is **bold**") must become a
        // spaceAfter on the LAST token so the next leaf doesn't collide
        // into this one (would render as "isbold").
        const words = text.split(/\s+/).filter(w => w.length > 0);
        const hasTrailingSpace = /\s$/.test(text);
        if (words.length === 0 && text.length > 0) {
          // Leaf is pure whitespace (e.g. a soft_break " "). Emit a single
          // zero-width spacer token so the space between sibling leaves is
          // preserved. Paint backends render an empty string as a no-op.
          tokens.push({
            node,
            word: "",
            width: 0,
            height: measurer.measure("x", font, Infinity).height,
            spaceAfter: spaceWidth,
            verticalAlign,
          });
        }
        for (let i = 0; i < words.length; i++) {
          const w = words[i];
          const s = measurer.measure(w, font, Infinity);
          const isLast = i === words.length - 1;
          const spaceAfter = isLast
            ? (hasTrailingSpace ? spaceWidth : 0)
            : spaceWidth;
          tokens.push({ node, word: w, width: s.width, height: s.height, spaceAfter, verticalAlign });
        }
      }
    } else {
      // Non-text inline node (image, inline container): measure as atomic unit
      const s = measureInline(node, availableWidth, measurer);
      tokens.push({ node, word: "", width: s.width, height: s.height, spaceAfter: 0, verticalAlign });
    }
  }

  // ── Step 2: Pack tokens onto lines ───────────────────────────────────────

  interface LineEntry {
    token: LineToken;
    x: number;
    lineIndex: number;
  }

  const lines: LineEntry[][] = [];
  let currentLine: LineEntry[] = [];
  let currentX = 0;

  for (const token of tokens) {
    const tokenWidth = token.width + token.spaceAfter;

    // Would this token overflow the current line?
    if (currentX > 0 && currentX + token.width > availableWidth) {
      // Break to a new line
      if (currentLine.length > 0) lines.push(currentLine);
      currentLine = [];
      currentX = 0;
    }

    currentLine.push({ token, x: currentX, lineIndex: lines.length });
    currentX += tokenWidth;
  }
  if (currentLine.length > 0) lines.push(currentLine);

  // ── Step 3: Compute line heights, apply vertical alignment, emit positions ──

  const positioned: PositionedNode[] = [];
  let currentY = 0;

  for (const line of lines) {
    // Line height = max of all token heights on this line
    const lineHeight = line.reduce((m, e) => Math.max(m, e.token.height), 0);

    for (const entry of line) {
      const { token, x } = entry;
      let tokenY: number;

      switch (token.verticalAlign) {
        case "top":
          tokenY = currentY;
          break;
        case "middle":
          tokenY = currentY + (lineHeight - token.height) / 2;
          break;
        case "bottom":
          tokenY = currentY + lineHeight - token.height;
          break;
        case "baseline":
        default:
          // Baseline: bottom of text aligns to line's baseline = bottom of line box
          tokenY = currentY + lineHeight - token.height;
          break;
      }

      // Tokens represent individual words sliced out of the node's full text
      // content. The source content object carries the *whole* string, which
      // would cause the paint layer to render the entire paragraph at every
      // word position (catastrophic double-paint for PaintText backends, less
      // visible but still wrong for PaintGlyphRun). Override the value with
      // just this token's word so the paint layer sees what the layout engine
      // actually decided to draw at this position.
      const srcContent = token.node.content;
      const tokenContent =
        srcContent !== null && srcContent.kind === "text"
          ? { ...srcContent, value: token.word }
          : srcContent;

      positioned.push({
        x,
        y: tokenY,
        width: token.width,
        height: token.height,
        content: tokenContent,
        children: [],
        ext: token.node.ext,
      });
    }

    currentY += lineHeight;
  }

  return positioned;
}

// ── Anonymous block generation ─────────────────────────────────────────────

/**
 * CSS anonymous block generation.
 *
 * When a block container has a mix of block and inline children, the inline
 * children must be wrapped in anonymous block boxes (so that the block
 * algorithm only ever sees block children).
 *
 * Example:
 *   <div>
 *     <p>Block paragraph</p>   ← block child
 *     some text                ← inline child → wrapped in anonymous block
 *     <span>more text</span>   ← inline child → same anonymous block
 *     <p>Another block</p>     ← block child → previous anon block closed
 *   </div>
 *
 * Implementation: scan children in order. Whenever we encounter an inline
 * child, open an anonymous block and accumulate inline siblings into it.
 * Close the anonymous block when we hit a block child or the end.
 */
function wrapInlineRuns(children: LayoutNode[]): LayoutNode[] {
  const wrapped: LayoutNode[] = [];
  let inlineRun: LayoutNode[] = [];

  const flushInlineRun = () => {
    if (inlineRun.length === 0) return;
    // Create an anonymous block node that carries the inline children
    wrapped.push({
      width: { kind: "fill" },
      height: { kind: "wrap" },
      content: null,
      children: inlineRun,
      minWidth: undefined,
      maxWidth: undefined,
      minHeight: undefined,
      maxHeight: undefined,
      margin: { top: 0, right: 0, bottom: 0, left: 0 },
      padding: { top: 0, right: 0, bottom: 0, left: 0 },
      ext: { block: { display: "block", _anonymous: true } as BlockExt & Record<string, unknown> },
    });
    inlineRun = [];
  };

  for (const child of children) {
    if (isInline(child)) {
      inlineRun.push(child);
    } else {
      flushInlineRun();
      wrapped.push(child);
    }
  }
  flushInlineRun();

  return wrapped;
}

// ── Main function ──────────────────────────────────────────────────────────

/**
 * Lay out a block container using block/inline flow rules.
 *
 * This is the reference implementation of CSS normal flow (block formatting
 * context) — the layout model for documents, emails, and rich text.
 *
 * The algorithm:
 *   1. Resolve the container's inner width from constraints and sizing hints
 *   2. Wrap inline children into anonymous block boxes
 *   3. Stack block children top-to-bottom with CSS margin collapsing
 *   4. For anonymous blocks (inline runs), use the inline formatting context
 *   5. Resolve the container's height (fixed or shrink-to-content)
 *
 * @param container  The block container node with ext["block"] metadata
 * @param constraints  Incoming size constraints (e.g., the parent's available width)
 * @param measurer  Text measurement provider (injected dependency)
 * @returns A fully-positioned PositionedNode tree
 */
export function layout_block(
  container: LayoutNode,
  constraints: Constraints,
  measurer: TextMeasurer
): PositionedNode {
  // ── Resolve container's inner width ─────────────────────────────────────

  const constrainedW = constraints.maxWidth;

  let containerWidth: number;
  if (container.width.kind === "fixed") {
    containerWidth = clamp(container.width.value, container.minWidth, container.maxWidth);
  } else if (container.width.kind === "fill") {
    // Fill: take all available width
    containerWidth = clamp(constrainedW ?? 0, container.minWidth, container.maxWidth);
  } else {
    // Wrap: measure content, then clamp
    containerWidth = clamp(constrainedW ?? 0, container.minWidth, container.maxWidth);
  }

  const padding = container.padding ?? { top: 0, right: 0, bottom: 0, left: 0 };
  const innerWidth = Math.max(0, containerWidth - padding.left - padding.right);

  // ── Leaf content node (no children) ──────────────────────────────────────
  // A block node with text/image content but no children is a leaf. We measure
  // the content directly instead of running the block stacking loop.
  if (container.content !== null && container.children.length === 0) {
    let leafHeight: number;

    if (container.content.kind === "text") {
      const measured = measurer.measure(container.content.value, container.content.font, innerWidth);
      leafHeight = measured.height;
    } else {
      // image: honour a fixed height, otherwise square
      leafHeight = container.height.kind === "fixed" ? container.height.value : innerWidth;
    }

    const finalHeight =
      container.height.kind === "fixed"
        ? clamp(container.height.value, container.minHeight, container.maxHeight)
        : clamp(padding.top + leafHeight + padding.bottom, container.minHeight, container.maxHeight);

    let leafExt = { ...container.ext };
    const leafBlockExt = blockExt(container);
    if (leafBlockExt.overflow === "hidden" || leafBlockExt.overflow === "scroll") {
      leafExt = {
        ...leafExt,
        paint: {
          ...(leafExt["paint"] as Record<string, unknown> | undefined ?? {}),
          overflow: leafBlockExt.overflow,
        },
      };
    }

    return { x: 0, y: 0, width: containerWidth, height: finalHeight, content: container.content, children: [], ext: leafExt };
  }

  // ── Wrap inline siblings into anonymous block boxes ───────────────────────
  const blockChildren: LayoutNode[] = wrapInlineRuns(container.children);

  // ── Stack block children ─────────────────────────────────────────────────

  const positionedChildren: PositionedNode[] = [];
  let currentY = padding.top;
  let prevMarginBottom = 0; // for margin collapsing

  const ext = blockExt(container);

  for (let i = 0; i < blockChildren.length; i++) {
    const child = blockChildren[i];
    const childExt = blockExt(child);
    const childMargin = child.margin ?? { top: 0, right: 0, bottom: 0, left: 0 };

    // ── Margin collapsing: adjacent sibling margins collapse to max ─────
    // The first child's top margin is not collapsed with the parent (simplified).
    const effectiveTopMargin =
      i === 0
        ? childMargin.top
        : Math.max(prevMarginBottom, childMargin.top);

    currentY += effectiveTopMargin;

    // ── Resolve child width ───────────────────────────────────────────────
    let childWidth: number;
    if (child.width.kind === "fixed") {
      childWidth = clamp(child.width.value, child.minWidth, child.maxWidth);
    } else if (child.width.kind === "fill") {
      childWidth = clamp(
        innerWidth - childMargin.left - childMargin.right,
        child.minWidth,
        child.maxWidth
      );
    } else {
      // wrap: use available inner width (child will shrink to content)
      childWidth = clamp(innerWidth, child.minWidth, child.maxWidth);
    }

    // ── Is this an anonymous inline block? ────────────────────────────────
    const isAnonInline = (childExt as Record<string, unknown>)["_anonymous"] === true;

    let childPositioned: PositionedNode;

    if (isAnonInline) {
      // Inline formatting context: lay out all inline children as a line run
      const inlinePositioned = layoutInlineRun(child.children, childWidth, measurer);
      const inlineHeight = inlinePositioned.reduce(
        (max, n) => Math.max(max, n.y + n.height),
        0
      );
      childPositioned = {
        x: padding.left + childMargin.left,
        y: currentY,
        width: childWidth,
        height: inlineHeight,
        content: null,
        children: inlinePositioned,
        ext: child.ext,
      };
    } else {
      // Recursive block layout
      const childConstraints = constraints_width(childWidth);
      const childResult = layout_block(child, childConstraints, measurer);
      childPositioned = {
        ...childResult,
        x: padding.left + childMargin.left,
        y: currentY,
      };
    }

    positionedChildren.push(childPositioned);

    // Advance Y by child height; track trailing margin for collapsing
    const paragraphSpacing = childExt.paragraphSpacing ?? 0;
    currentY += childPositioned.height;
    prevMarginBottom = Math.max(childMargin.bottom, paragraphSpacing);
  }

  // Final height: y of last child's bottom edge + padding.bottom
  const bottomEdge = positionedChildren.length > 0
    ? Math.max(...positionedChildren.map(n => n.y + n.height))
    : padding.top;
  const containerInnerHeight = bottomEdge + padding.bottom;

  // ── Resolve container height ─────────────────────────────────────────────
  let containerHeight: number;
  if (container.height.kind === "fixed") {
    containerHeight = clamp(container.height.value, container.minHeight, container.maxHeight);
  } else {
    // wrap or fill: shrink to content
    const naturalHeight = containerInnerHeight;
    containerHeight = clamp(naturalHeight, container.minHeight, container.maxHeight);
  }

  // ── Apply overflow to ext["paint"] ───────────────────────────────────────
  let outExt = { ...container.ext };
  if (ext.overflow === "hidden" || ext.overflow === "scroll") {
    outExt = {
      ...outExt,
      paint: {
        ...(outExt["paint"] as Record<string, unknown> | undefined ?? {}),
        overflow: ext.overflow,
      },
    };
  }

  return {
    x: 0,
    y: 0,
    width: containerWidth,
    height: containerHeight,
    content: container.content,
    children: positionedChildren,
    ext: outExt,
  };
}
