/**
 * CSS Grid Layout Algorithm
 *
 * Implements the CSS Grid Layout specification — the subset needed for tables,
 * magazine layouts, and card grids.
 *
 * Pipeline position:
 *
 *   LayoutNode (with ext["grid"]) + Constraints + TextMeasurer
 *       ↓  layout_grid()
 *   PositionedNode
 *
 * Mental Model
 * ------------
 *
 * A CSS grid is a two-dimensional table made of tracks (rows and columns). The
 * container defines how many tracks exist and how wide/tall each is. Each child
 * specifies which cell(s) it occupies. The algorithm resolves track sizes, then
 * places each child into its cell at the correct (x, y) coordinates.
 *
 * Track Sizing Summary:
 *
 *   "200px"         → fixed: always 200px
 *   "1fr"           → flexible: proportional share of remaining space
 *   "auto"          → intrinsic: sized to its content
 *   "minmax(a, b)"  → sized to at least a and at most b
 *   "repeat(N, sz)" → N copies of the track definition
 *
 * Auto Placement:
 *
 *   Items without explicit grid-line placements are auto-placed. The algorithm
 *   scans rows (or columns, for autoFlow:"column") left-to-right, top-to-bottom,
 *   placing each item in the first open cell that fits its span.
 *
 * Implementation note: this is a simplified subset of the full CSS Grid spec.
 * It is intentionally educational — real browsers implement hundreds of edge
 * cases. This covers the cases needed for document tables and card grids.
 *
 * Supported features
 * ------------------
 *
 *   ✅ templateColumns / templateRows as fixed/fr/auto/repeat/minmax
 *   ✅ columnGap / rowGap
 *   ✅ autoRows / autoColumns (implicit track sizing)
 *   ✅ autoFlow: "row" | "column"
 *   ✅ Explicit column/row start/end and span
 *   ✅ Auto-placement for items without explicit position
 *   ✅ alignItems / justifyItems on container
 *   ✅ alignSelf / justifySelf on items
 *   ✅ padding on container
 *   ✅ Flexible fr tracks (free space distribution)
 *   ✅ Auto tracks (content-sized)
 *
 *   ❌ grid-template-areas (named areas)
 *   ❌ autoFlow: "dense" (basic gap-fill only)
 *   ❌ align-content / justify-content (grid-in-container alignment)
 *   ❌ subgrid
 */

import type {
  LayoutNode,
  Constraints,
  PositionedNode,
  TextMeasurer,
} from "@coding-adventures/layout-ir";
import { constraints_width } from "@coding-adventures/layout-ir";

// ── Extension types ────────────────────────────────────────────────────────

/**
 * Properties read from `ext["grid"]` on the **container** node.
 */
export interface GridContainerExt {
  /** Column track sizes. Default: "1fr" */
  templateColumns?: string;
  /** Row track sizes. Default: "auto" */
  templateRows?: string;
  /** Gap between columns. Default: 0 */
  columnGap?: number;
  /** Gap between rows. Default: 0 */
  rowGap?: number;
  /** Size for implicitly created rows. Default: "auto" */
  autoRows?: string;
  /** Size for implicitly created columns. Default: "auto" */
  autoColumns?: string;
  /** Auto-placement direction. Default: "row" */
  autoFlow?: "row" | "column" | "dense";
  /** Default alignment of items on the block axis. Default: "stretch" */
  alignItems?: "start" | "center" | "end" | "stretch";
  /** Default alignment of items on the inline axis. Default: "stretch" */
  justifyItems?: "start" | "center" | "end" | "stretch";
}

/**
 * Properties read from `ext["grid"]` on **item** (child) nodes.
 */
export interface GridItemExt {
  /** 1-based column line start. "auto" = auto-placed. */
  columnStart?: number | "auto";
  /** 1-based exclusive column line end. "auto" = columnStart + columnSpan. */
  columnEnd?: number | "auto";
  /** Number of columns to span. Default: 1 */
  columnSpan?: number;
  /** 1-based row line start. "auto" = auto-placed. */
  rowStart?: number | "auto";
  /** 1-based exclusive row line end. "auto" = rowStart + rowSpan. */
  rowEnd?: number | "auto";
  /** Number of rows to span. Default: 1 */
  rowSpan?: number;
  /** Override alignItems for this item. "auto" = use container default. */
  alignSelf?: "start" | "center" | "end" | "stretch" | "auto";
  /** Override justifyItems for this item. "auto" = use container default. */
  justifySelf?: "start" | "center" | "end" | "stretch" | "auto";
}

/** Union of both ext shapes (both live in ext["grid"]) */
export type GridExt = GridContainerExt & GridItemExt;

// ── Internal track definition ──────────────────────────────────────────────

/**
 * A parsed track size value.
 *
 *   "200px"  →  { kind: "fixed", size: 200 }
 *   "1fr"    →  { kind: "flexible", fraction: 1 }
 *   "auto"   →  { kind: "auto" }
 *   "minmax(100px, 1fr)" → { kind: "minmax", min: fixed(100), max: flexible(1) }
 */
type TrackDefinition =
  | { kind: "fixed"; size: number }
  | { kind: "flexible"; fraction: number }
  | { kind: "auto" }
  | { kind: "minmax"; min: TrackDefinition; max: TrackDefinition };

// ── Track list parser ──────────────────────────────────────────────────────

/**
 * Parse a single track size string into a TrackDefinition.
 *
 * Examples: "200px", "1fr", "auto", "minmax(100px, 1fr)"
 */
function parseTrackSize(s: string): TrackDefinition {
  s = s.trim();
  if (s === "auto") return { kind: "auto" };
  if (s.endsWith("fr")) return { kind: "flexible", fraction: parseFloat(s) };
  if (s.endsWith("px")) return { kind: "fixed", size: parseFloat(s) };
  if (s.startsWith("minmax(") && s.endsWith(")")) {
    const inner = s.slice(7, -1);
    // Split on comma, respecting nested parens (minmax is only one level deep here)
    const commaIdx = inner.indexOf(",");
    const minStr = inner.slice(0, commaIdx).trim();
    const maxStr = inner.slice(commaIdx + 1).trim();
    return { kind: "minmax", min: parseTrackSize(minStr), max: parseTrackSize(maxStr) };
  }
  // Fallback: treat as auto
  return { kind: "auto" };
}

/**
 * Parse a track list string into an array of TrackDefinitions.
 *
 * Handles:
 *   - Whitespace-separated values: "200px 1fr 100px"
 *   - repeat(N, size): "repeat(3, 1fr)"
 *   - Mixed: "100px repeat(2, 1fr) 50px"
 */
function parseTrackList(s: string): TrackDefinition[] {
  s = s.trim();
  const tracks: TrackDefinition[] = [];

  // Tokenize on top-level whitespace (not inside parens)
  const tokens = splitTrackListTokens(s);

  for (const token of tokens) {
    if (token.startsWith("repeat(") && token.endsWith(")")) {
      const inner = token.slice(7, -1);
      const commaIdx = inner.indexOf(",");
      const count = parseInt(inner.slice(0, commaIdx).trim(), 10);
      const sizeStr = inner.slice(commaIdx + 1).trim();
      const trackDef = parseTrackSize(sizeStr);
      for (let i = 0; i < count; i++) tracks.push(trackDef);
    } else {
      tracks.push(parseTrackSize(token));
    }
  }

  return tracks;
}

/**
 * Split a track list string into tokens, respecting parentheses.
 *
 * "200px 1fr repeat(2, 100px)" → ["200px", "1fr", "repeat(2, 100px)"]
 */
function splitTrackListTokens(s: string): string[] {
  const tokens: string[] = [];
  let current = "";
  let depth = 0;

  for (const ch of s) {
    if (ch === "(") { depth++; current += ch; }
    else if (ch === ")") { depth--; current += ch; }
    else if (ch === " " && depth === 0) {
      if (current.length > 0) { tokens.push(current); current = ""; }
    } else {
      current += ch;
    }
  }
  if (current.length > 0) tokens.push(current);
  return tokens;
}

// ── Grid item placement ────────────────────────────────────────────────────

/**
 * Resolved placement of a grid item in the grid.
 * All values are 1-based; end is exclusive (like CSS grid-line numbers).
 */
interface GridPlacement {
  rowStart: number;     // 1-based
  rowEnd: number;       // exclusive
  colStart: number;     // 1-based
  colEnd: number;       // exclusive
}

/**
 * Place all items into the grid, respecting explicit placements and running
 * the auto-placement algorithm for items without explicit positions.
 *
 * @returns Array of placements, one per child (same order as container.children)
 */
function placeItems(
  children: LayoutNode[],
  colCount: number,
  rowCount: number,
  autoFlow: "row" | "column" | "dense"
): { placements: GridPlacement[]; maxRow: number; maxCol: number } {
  // Track which cells are occupied: Set<"row,col"> where row and col are 1-based
  const occupied = new Set<string>();

  const occupy = (p: GridPlacement) => {
    for (let r = p.rowStart; r < p.rowEnd; r++) {
      for (let c = p.colStart; c < p.colEnd; c++) {
        occupied.add(`${r},${c}`);
      }
    }
  };

  const isFree = (rowStart: number, colStart: number, rowSpan: number, colSpan: number): boolean => {
    for (let r = rowStart; r < rowStart + rowSpan; r++) {
      for (let c = colStart; c < colStart + colSpan; c++) {
        if (occupied.has(`${r},${c}`)) return false;
        if (colCount > 0 && c > colCount) return false;
      }
    }
    return true;
  };

  const placements: (GridPlacement | null)[] = new Array(children.length).fill(null);

  // Pass 1: place all explicitly positioned items
  for (let i = 0; i < children.length; i++) {
    const itemExt = (children[i].ext["grid"] as GridItemExt | undefined) ?? {};
    const colSpan = itemExt.columnSpan ?? 1;
    const rowSpan = itemExt.rowSpan ?? 1;

    const colStart = typeof itemExt.columnStart === "number" ? itemExt.columnStart : null;
    const colEnd = typeof itemExt.columnEnd === "number" ? itemExt.columnEnd : null;
    const rowStart = typeof itemExt.rowStart === "number" ? itemExt.rowStart : null;
    const rowEnd = typeof itemExt.rowEnd === "number" ? itemExt.rowEnd : null;

    if (colStart !== null && rowStart !== null) {
      const cs = colStart;
      const rs = rowStart;
      const ce = colEnd ?? cs + colSpan;
      const re = rowEnd ?? rs + rowSpan;
      placements[i] = { rowStart: rs, rowEnd: re, colStart: cs, colEnd: ce };
      occupy(placements[i]!);
    }
  }

  // Pass 2: auto-place remaining items
  let autoRow = 1;
  let autoCol = 1;
  const effectiveColCount = colCount > 0 ? colCount : 12; // default 12-column grid

  for (let i = 0; i < children.length; i++) {
    if (placements[i] !== null) continue;

    const itemExt = (children[i].ext["grid"] as GridItemExt | undefined) ?? {};
    const colSpan = itemExt.columnSpan ?? 1;
    const rowSpan = itemExt.rowSpan ?? 1;

    if (autoFlow === "column") {
      // Column flow: fill each column top-to-bottom, then advance to next column.
      // Wrap to the next column when we exceed the explicit row count (or 12 default).
      const effectiveRowCount = rowCount > 0 ? rowCount : 12;
      while (true) {
        if (autoRow + rowSpan - 1 > effectiveRowCount) {
          // Wrap to next column
          autoRow = 1;
          autoCol++;
        }
        if (isFree(autoRow, autoCol, rowSpan, colSpan)) break;
        autoRow++;
        if (autoRow + rowSpan - 1 > effectiveRowCount) {
          autoRow = 1;
          autoCol++;
        }
      }
      placements[i] = {
        rowStart: autoRow,
        rowEnd: autoRow + rowSpan,
        colStart: autoCol,
        colEnd: autoCol + colSpan,
      };
      autoRow += rowSpan;
    } else {
      // autoFlow: "row" (default) — row-by-row, wrap at effectiveColCount
      // Reset col to 1 when wrapping to next row
      while (true) {
        if (autoCol + colSpan - 1 > effectiveColCount) {
          // Wrap to next row
          autoRow++;
          autoCol = 1;
        }
        if (isFree(autoRow, autoCol, rowSpan, colSpan)) break;
        autoCol++;
        if (autoCol + colSpan - 1 > effectiveColCount) {
          autoRow++;
          autoCol = 1;
        }
      }
      placements[i] = {
        rowStart: autoRow,
        rowEnd: autoRow + rowSpan,
        colStart: autoCol,
        colEnd: autoCol + colSpan,
      };
      autoCol += colSpan;
    }

    occupy(placements[i]!);
  }

  // Find grid extents
  let maxRow = 1;
  let maxCol = colCount;
  for (const p of placements) {
    if (p) {
      maxRow = Math.max(maxRow, p.rowEnd - 1);
      maxCol = Math.max(maxCol, p.colEnd - 1);
    }
  }

  return { placements: placements as GridPlacement[], maxRow, maxCol };
}

// ── Track sizing ───────────────────────────────────────────────────────────

/**
 * Measure the natural (content) size of a node.
 * Used for sizing "auto" tracks to their content.
 */
function measureNode(node: LayoutNode, maxWidth: number, measurer: TextMeasurer): { w: number; h: number } {
  if (node.content !== null && node.content.kind === "text") {
    const r = measurer.measure(node.content.value, node.content.font, maxWidth);
    return { w: r.width, h: r.height };
  }
  if (node.content !== null && node.content.kind === "image") {
    const w = node.width.kind === "fixed" ? node.width.value : maxWidth;
    return { w, h: w };
  }
  // Container: use its fixed size if known, else a nominal width
  const w = node.width.kind === "fixed" ? node.width.value : maxWidth;
  const h = node.height.kind === "fixed" ? node.height.value : 0;
  return { w, h };
}

/**
 * Resolve track sizes for one axis (columns or rows).
 *
 * Algorithm:
 *   1. Fixed tracks get their fixed size immediately.
 *   2. Auto tracks: find items spanning only this track; use max content size.
 *   3. Flexible (fr) tracks: distribute remaining free space proportionally.
 *
 * @param defs        Track definitions (parsed template)
 * @param totalSpace  Available space for the axis
 * @param gaps        Total gap space (already subtracted from free space for fr)
 * @param items       Array of {placement, node} for sizing auto tracks
 * @param axis        "col" or "row"
 * @param measurer    Text measurer (for auto tracks)
 */
function resolveTrackSizes(
  defs: TrackDefinition[],
  totalSpace: number,
  gapWidth: number,
  items: Array<{ placement: GridPlacement; node: LayoutNode }>,
  axis: "col" | "row",
  measurer: TextMeasurer
): number[] {
  const n = defs.length;
  const sizes: number[] = new Array(n).fill(0);
  const totalGaps = (n - 1) * gapWidth;

  // Pass 1: resolve fixed and apply minmax
  let fixedTotal = 0;
  for (let i = 0; i < n; i++) {
    const def = defs[i];
    if (def.kind === "fixed") {
      sizes[i] = def.size;
      fixedTotal += def.size;
    } else if (def.kind === "minmax") {
      // For minmax, start with min; max will be handled during fr pass
      const minDef = def.min;
      sizes[i] = minDef.kind === "fixed" ? minDef.size : 0;
      if (minDef.kind === "fixed") fixedTotal += sizes[i];
    }
  }

  // Pass 2: resolve auto tracks using content measurement
  for (let i = 0; i < n; i++) {
    const def = defs[i];
    if (def.kind !== "auto" && def.kind !== "minmax") continue;
    const isAuto = def.kind === "auto" || def.kind === "minmax";

    // Find items that span exactly this track
    let maxContent = 0;
    for (const { placement, node } of items) {
      const trackStart = axis === "col" ? placement.colStart : placement.rowStart;
      const trackEnd = axis === "col" ? placement.colEnd : placement.rowEnd;
      // Item spans only track i+1
      if (trackStart === i + 1 && trackEnd === i + 2) {
        const measured = measureNode(node, totalSpace, measurer);
        maxContent = Math.max(maxContent, axis === "col" ? measured.w : measured.h);
      }
    }

    if (def.kind === "auto") {
      sizes[i] = maxContent;
      fixedTotal += maxContent;
    } else if (def.kind === "minmax") {
      // minmax: clamp maxContent between min and max
      const minSize = def.min.kind === "fixed" ? def.min.size : 0;
      // If max is flexible, we handle it in the fr pass; for now use max(min, content)
      sizes[i] = Math.max(minSize, maxContent);
      // Update fixedTotal (removing old min, adding new size)
      fixedTotal = fixedTotal - (def.min.kind === "fixed" ? def.min.size : 0) + sizes[i];
    }
  }

  // Pass 3: distribute free space to flexible (fr) tracks
  const freeSpace = Math.max(0, totalSpace - fixedTotal - totalGaps);
  let totalFractions = 0;
  for (const def of defs) {
    if (def.kind === "flexible") totalFractions += def.fraction;
    if (def.kind === "minmax" && def.max.kind === "flexible") totalFractions += def.max.fraction;
  }

  if (totalFractions > 0 && freeSpace > 0) {
    const perFraction = freeSpace / totalFractions;
    for (let i = 0; i < n; i++) {
      const def = defs[i];
      if (def.kind === "flexible") {
        sizes[i] = def.fraction * perFraction;
      } else if (def.kind === "minmax" && def.max.kind === "flexible") {
        sizes[i] = Math.max(sizes[i], def.max.fraction * perFraction);
      }
    }
  }

  return sizes;
}

// ── Alignment helpers ──────────────────────────────────────────────────────

type Alignment = "start" | "center" | "end" | "stretch";

/**
 * Compute the position and size of an item within a cell, applying alignment.
 *
 * @param cellOffset  The cell's start coordinate (x or y)
 * @param cellSize    The cell's total size
 * @param itemSize    The item's natural content size
 * @param align       The alignment value
 * @returns { offset, size } — the item's final position and size within the cell
 */
function applyAlignment(
  cellOffset: number,
  cellSize: number,
  itemSize: number,
  align: Alignment
): { offset: number; size: number } {
  switch (align) {
    case "stretch":
      return { offset: cellOffset, size: cellSize };
    case "start":
      return { offset: cellOffset, size: itemSize };
    case "end":
      return { offset: cellOffset + cellSize - itemSize, size: itemSize };
    case "center":
      return { offset: cellOffset + (cellSize - itemSize) / 2, size: itemSize };
  }
}

// ── Main function ──────────────────────────────────────────────────────────

/**
 * Lay out a grid container using CSS Grid rules.
 *
 * The algorithm runs in 7 steps, matching the spec:
 *
 *   1. Parse track lists
 *   2. Place items (explicit + auto)
 *   3. Create implicit tracks
 *   4. Resolve track sizes (fixed and auto)
 *   5. Distribute free space to fr tracks
 *   6. Compute item positions and apply alignment
 *   7. Build and return the PositionedNode tree
 *
 * @param container  The grid container node with ext["grid"] metadata
 * @param constraints  Incoming size constraints
 * @param measurer  Text measurement provider
 * @returns A fully-positioned PositionedNode
 */
export function layout_grid(
  container: LayoutNode,
  constraints: Constraints,
  measurer: TextMeasurer
): PositionedNode {
  const containerExt = (container.ext["grid"] as GridContainerExt | undefined) ?? {};
  const padding = container.padding ?? { top: 0, right: 0, bottom: 0, left: 0 };

  // ── Resolve container width ───────────────────────────────────────────────
  const constrainedW = constraints.maxWidth ?? 0;
  let containerWidth: number;
  if (container.width.kind === "fixed") {
    containerWidth = container.width.value;
  } else {
    containerWidth = constrainedW;
  }
  const innerWidth = Math.max(0, containerWidth - padding.left - padding.right);

  const columnGap = containerExt.columnGap ?? 0;
  const rowGap = containerExt.rowGap ?? 0;
  const autoFlow = containerExt.autoFlow ?? "row";
  const alignItems: Alignment = containerExt.alignItems ?? "stretch";
  const justifyItems: Alignment = containerExt.justifyItems ?? "stretch";

  // ── Step 1: Parse track lists ─────────────────────────────────────────────

  const explicitColDefs = parseTrackList(containerExt.templateColumns ?? "1fr");
  const explicitRowDefs = parseTrackList(containerExt.templateRows ?? "auto");
  const autoRowDef = parseTrackSize(containerExt.autoRows ?? "auto");
  const autoColDef = parseTrackSize(containerExt.autoColumns ?? "auto");

  const explicitColCount = explicitColDefs.length;
  const explicitRowCount = explicitRowDefs.length;

  // ── Step 2: Place items ───────────────────────────────────────────────────

  const { placements, maxRow, maxCol } = placeItems(
    container.children,
    explicitColCount,
    explicitRowCount,
    autoFlow
  );

  // ── Step 3: Create implicit tracks ───────────────────────────────────────

  const colDefs: TrackDefinition[] = [...explicitColDefs];
  while (colDefs.length < maxCol) colDefs.push(autoColDef);

  const rowDefs: TrackDefinition[] = [...explicitRowDefs];
  while (rowDefs.length < maxRow) rowDefs.push(autoRowDef);

  // ── Step 4 + 5: Resolve track sizes ──────────────────────────────────────

  const itemsForSizing = placements.map((p, i) => ({ placement: p, node: container.children[i] }));

  const colSizes = resolveTrackSizes(colDefs, innerWidth, columnGap, itemsForSizing, "col", measurer);
  const totalColSize = colSizes.reduce((a, b) => a + b, 0) + Math.max(0, colSizes.length - 1) * columnGap;

  // For rows: available height is either fixed or unconstrained
  const constrainedH = constraints.maxHeight;
  const innerHeight = container.height.kind === "fixed"
    ? container.height.value - padding.top - padding.bottom
    : (constrainedH !== undefined ? constrainedH - padding.top - padding.bottom : 0);

  const rowSizes = resolveTrackSizes(rowDefs, innerHeight, rowGap, itemsForSizing, "row", measurer);
  const totalRowSize = rowSizes.reduce((a, b) => a + b, 0) + Math.max(0, rowSizes.length - 1) * rowGap;

  // ── Step 6: Compute item positions ───────────────────────────────────────

  // Precompute cumulative column and row offsets (from content area origin)
  const colOffsets: number[] = [0];
  for (let i = 0; i < colSizes.length; i++) {
    colOffsets.push(colOffsets[i] + colSizes[i] + columnGap);
  }

  const rowOffsets: number[] = [0];
  for (let i = 0; i < rowSizes.length; i++) {
    rowOffsets.push(rowOffsets[i] + rowSizes[i] + rowGap);
  }

  const positionedChildren: PositionedNode[] = [];

  for (let i = 0; i < container.children.length; i++) {
    const child = container.children[i];
    const p = placements[i];
    const itemExt = (child.ext["grid"] as GridItemExt | undefined) ?? {};

    // Cell bounds (from content area origin)
    const cellX = colOffsets[p.colStart - 1] + padding.left;
    const cellY = rowOffsets[p.rowStart - 1] + padding.top;

    // Cell width and height: sum tracks that the item spans, minus one gap per span gap
    const spanCols = p.colEnd - p.colStart;
    const spanRows = p.rowEnd - p.rowStart;

    let cellW = 0;
    for (let c = p.colStart - 1; c < p.colEnd - 1 && c < colSizes.length; c++) {
      cellW += colSizes[c];
    }
    cellW += Math.max(0, spanCols - 1) * columnGap;

    let cellH = 0;
    for (let r = p.rowStart - 1; r < p.rowEnd - 1 && r < rowSizes.length; r++) {
      cellH += rowSizes[r];
    }
    cellH += Math.max(0, spanRows - 1) * rowGap;

    // Measure item's natural content size for alignment
    const naturalSize = measureNode(child, cellW, measurer);

    // Resolve justify (inline axis = horizontal)
    const jSelf = itemExt.justifySelf === "auto" ? justifyItems : (itemExt.justifySelf ?? justifyItems);
    const { offset: itemX, size: itemW } = applyAlignment(cellX, cellW, naturalSize.w, jSelf);

    // Resolve align (block axis = vertical)
    const aSelf = itemExt.alignSelf === "auto" ? alignItems : (itemExt.alignSelf ?? alignItems);
    const { offset: itemY, size: itemH } = applyAlignment(cellY, cellH, naturalSize.h, aSelf);

    // Lay out child content at its resolved size
    const childConstraints = constraints_width(itemW);
    positionedChildren.push({
      x: itemX,
      y: itemY,
      width: itemW,
      height: itemH,
      content: child.content,
      children: [],
      ext: child.ext,
    });
  }

  // ── Step 7: Return container PositionedNode ───────────────────────────────

  const containerHeight =
    container.height.kind === "fixed"
      ? container.height.value
      : totalRowSize + padding.top + padding.bottom;

  return {
    x: 0,
    y: 0,
    width: containerWidth,
    height: containerHeight,
    content: container.content,
    children: positionedChildren,
    ext: container.ext,
  };
}
