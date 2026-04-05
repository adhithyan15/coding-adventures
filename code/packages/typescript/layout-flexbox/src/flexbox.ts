/**
 * Flexbox Layout Algorithm
 *
 * Implements CSS Flexible Box Layout (Level 1) — the subset needed for the
 * Mosaic Column/Row/Spacer model and general-purpose UI layout.
 *
 * The algorithm takes a flex container node, reads `ext["flex"]` for container
 * and item properties, and returns a fully positioned node tree.
 *
 * Pipeline position:
 *
 *   LayoutNode (with ext["flex"]) + Constraints + TextMeasurer
 *       ↓  layout_flexbox()
 *   PositionedNode
 *
 * Key Design Decisions
 * --------------------
 *
 * 1. **The algorithm positions only its direct children.** It does not recurse
 *    into grandchildren — that is the pipeline's job. The caller decides which
 *    algorithm to use for each sub-container.
 *
 * 2. **Missing ext fields use defaults.** The algorithm never throws on missing
 *    flex properties; it uses the CSS defaults. Unknown ext keys are ignored.
 *
 * 3. **`measure_node` is internal.** It computes the natural size of a child
 *    for wrap/auto sizing. For containers, it calls layout_flexbox recursively
 *    (using unconstrained constraints).
 *
 * Supported features
 * ------------------
 *
 *   ✅ direction (row / column)
 *   ✅ wrap (nowrap / wrap)
 *   ✅ gap / rowGap / columnGap
 *   ✅ grow / shrink / basis
 *   ✅ alignItems / alignSelf (start/center/end/stretch)
 *   ✅ justifyContent (start/center/end/between/around/evenly)
 *   ✅ order (sort before layout)
 *   ✅ padding on container
 *   ✅ min/maxWidth, min/maxHeight on all nodes
 *
 *   ❌ position: absolute/fixed (out of flow, future)
 *   ❌ align-content for multi-line (future)
 *   ❌ RTL / writing mode
 *   ❌ z-index (handled by paint-ir layer ordering)
 */

import type {
  LayoutNode,
  PositionedNode,
  Constraints,
  TextMeasurer,
  SizeValue,
  Edges,
} from "@coding-adventures/layout-ir";

import {
  constraints_fixed,
  constraints_unconstrained,
  constraints_shrink,
  positioned,
  edges_zero,
} from "@coding-adventures/layout-ir";

// ============================================================================
// Extension schemas
// ============================================================================

/**
 * Flex container properties read from the container node's `ext["flex"]`.
 *
 * All fields are optional — missing fields fall back to CSS defaults.
 */
export interface FlexContainerExt {
  direction?: "row" | "column";
  wrap?: "nowrap" | "wrap";
  alignItems?: "start" | "center" | "end" | "stretch";
  justifyContent?: "start" | "center" | "end" | "between" | "around" | "evenly";
  gap?: number;
  rowGap?: number;
  columnGap?: number;
}

/**
 * Flex item properties read from each child node's `ext["flex"]`.
 *
 * A node may carry both container and item properties simultaneously
 * (when it is both a flex child and a flex container).
 */
export interface FlexItemExt {
  grow?: number;
  shrink?: number;
  basis?: SizeValue | null;
  alignSelf?: "start" | "center" | "end" | "stretch" | "auto";
  order?: number;
}

/** Combined flex ext type — both container and item fields in one map. */
export type FlexExt = FlexContainerExt & FlexItemExt;

// ============================================================================
// Internal helpers
// ============================================================================

/** Resolved container properties with defaults applied. */
interface ResolvedContainerProps {
  direction: "row" | "column";
  wrap: "nowrap" | "wrap";
  alignItems: "start" | "center" | "end" | "stretch";
  justifyContent: "start" | "center" | "end" | "between" | "around" | "evenly";
  mainGap: number;  // gap along main axis
  crossGap: number; // gap between lines (for wrap)
}

function resolveContainerProps(node: LayoutNode): ResolvedContainerProps {
  const ext = (node.ext["flex"] ?? {}) as FlexContainerExt;
  const direction = ext.direction ?? "column";
  const gap = ext.gap ?? 0;
  const isRow = direction === "row";
  return {
    direction,
    wrap: ext.wrap ?? "nowrap",
    alignItems: ext.alignItems ?? "stretch",
    justifyContent: ext.justifyContent ?? "start",
    mainGap: isRow ? (ext.columnGap ?? gap) : (ext.rowGap ?? gap),
    crossGap: isRow ? (ext.rowGap ?? gap) : (ext.columnGap ?? gap),
  };
}

function resolveItemProps(node: LayoutNode): Required<FlexItemExt> {
  const ext = (node.ext["flex"] ?? {}) as FlexItemExt;
  return {
    grow: ext.grow ?? 0,
    shrink: ext.shrink ?? 1,
    basis: ext.basis ?? null,
    alignSelf: ext.alignSelf ?? "auto",
    order: ext.order ?? 0,
  };
}

function getPadding(node: LayoutNode): Edges {
  return node.padding ?? edges_zero();
}

function clamp(value: number, min: number | null | undefined, max: number | null | undefined): number {
  let v = value;
  if (min != null && v < min) v = min;
  if (max != null && v > max) v = max;
  return v;
}

// ============================================================================
// measure_node
// ============================================================================

/** Natural size of a node when unconstrained or within given constraints. */
export interface Size {
  width: number;
  height: number;
}

/**
 * Measure the natural (intrinsic) size of a layout node.
 *
 * For leaf nodes: delegates to the TextMeasurer or returns constraint size for images.
 * For container nodes: recursively calls layout_flexbox with unconstrained constraints.
 */
export function measure_node(
  node: LayoutNode,
  constraints: Constraints,
  measurer: TextMeasurer
): Size {
  if (node.content !== null) {
    // Leaf node.
    if (node.content.kind === "text") {
      const maxWidth = constraints.maxWidth === Infinity ? null : constraints.maxWidth;
      const result = measurer.measure(node.content.value, node.content.font, maxWidth);
      return {
        width: clamp(result.width, node.minWidth, node.maxWidth),
        height: clamp(result.height, node.minHeight, node.maxHeight),
      };
    } else {
      // Image: use constraints as natural size (square by default).
      const size = Math.min(constraints.maxWidth, constraints.maxHeight);
      return {
        width: clamp(isFinite(size) ? size : 0, node.minWidth, node.maxWidth),
        height: clamp(isFinite(size) ? size : 0, node.minHeight, node.maxHeight),
      };
    }
  }

  // Container node: recurse.
  const result = layout_flexbox(node, constraints, measurer);
  return { width: result.width, height: result.height };
}

// ============================================================================
// layout_flexbox
// ============================================================================

/**
 * Lay out a flex container.
 *
 * Takes the container node, available constraints, and a text measurer.
 * Returns a `PositionedNode` with all direct children positioned within
 * the container's content area.
 *
 * Usage:
 *
 *   const result = layout_flexbox(rootNode, constraints_width(800), measurer);
 *   // result.x = 0, result.y = 0
 *   // result.width = resolved container width
 *   // result.children = positioned children
 */
export function layout_flexbox(
  container: LayoutNode,
  constraints: Constraints,
  measurer: TextMeasurer
): PositionedNode {
  const props = resolveContainerProps(container);
  const padding = getPadding(container);

  // ── Step 1: Resolve container size ────────────────────────────────────────

  const horizontalPad = padding.left + padding.right;
  const verticalPad = padding.top + padding.bottom;

  // Available inner area after padding.
  const innerConstraints = constraints_shrink(constraints, horizontalPad, verticalPad);

  const containerWidth = resolveContainerDimension(
    container.width,
    constraints.maxWidth,
    container.minWidth,
    container.maxWidth,
    horizontalPad
  );
  const containerHeight = resolveContainerDimension(
    container.height,
    constraints.maxHeight,
    container.minHeight,
    container.maxHeight,
    verticalPad
  );

  // Inner content width/height (what children get to fill).
  const innerWidth = containerWidth !== null ? containerWidth - horizontalPad : null;
  const innerHeight = containerHeight !== null ? containerHeight - verticalPad : null;

  const isRow = props.direction === "row";
  const mainAvail = isRow ? innerWidth : innerHeight;
  const crossAvail = isRow ? innerHeight : innerWidth;

  // ── Step 2: Collect children sorted by order ───────────────────────────────

  const children = [...container.children]
    .map((child, i) => ({ child, i, itemProps: resolveItemProps(child) }))
    .sort((a, b) => a.itemProps.order - b.itemProps.order);

  if (children.length === 0) {
    // Empty container: resolve size and return (applying min/max clamps).
    const w = clamp(containerWidth ?? 0, container.minWidth, container.maxWidth);
    const h = clamp(containerHeight ?? 0, container.minHeight, container.maxHeight);
    return positioned(0, 0, w, h, {
      id: container.id,
      content: null,
      children: [],
      ext: container.ext,
    });
  }

  // ── Step 3: Determine hypothetical main sizes ──────────────────────────────

  type ItemState = {
    child: LayoutNode;
    itemProps: Required<FlexItemExt>;
    mainSize: number;  // hypothetical main size
    crossSize: number; // hypothetical cross size (0 until step 6)
    frozen: boolean;   // true if size is locked (fixed basis)
  };

  const items: ItemState[] = children.map(({ child, itemProps }) => {
    let mainSize = 0;
    if (itemProps.basis !== null) {
      mainSize = resolveSizeValue(itemProps.basis, mainAvail ?? Infinity);
    } else {
      const mainSizeHint = isRow ? child.width : child.height;
      mainSize = resolveSizeValue(mainSizeHint, mainAvail ?? Infinity);
    }

    // If wrap/fill, measure natural size in unconstrained space.
    const mainHint = itemProps.basis ?? (isRow ? child.width : child.height);
    if (mainHint === null || mainHint?.kind === "wrap") {
      const measured = measure_node(child, constraints_unconstrained(), measurer);
      mainSize = isRow ? measured.width : measured.height;
    }

    mainSize = clampMain(mainSize, child, isRow);
    return { child, itemProps, mainSize, crossSize: 0, frozen: false };
  });

  // ── Step 4: Collect into flex lines ───────────────────────────────────────

  type FlexLine = ItemState[];
  let lines: FlexLine[];

  if (props.wrap === "nowrap" || mainAvail === null) {
    lines = [items];
  } else {
    lines = [];
    let current: FlexLine = [];
    let runningMain = 0;

    for (const item of items) {
      const gapIfAdded = current.length > 0 ? props.mainGap : 0;
      if (current.length > 0 && runningMain + gapIfAdded + item.mainSize > mainAvail) {
        lines.push(current);
        current = [item];
        runningMain = item.mainSize;
      } else {
        current.push(item);
        runningMain += gapIfAdded + item.mainSize;
      }
    }
    if (current.length > 0) lines.push(current);
  }

  // ── Step 5: Resolve flexible lengths ──────────────────────────────────────

  for (const line of lines) {
    if (mainAvail === null) continue;

    const totalGaps = (line.length - 1) * props.mainGap;
    const totalMain = line.reduce((s, it) => s + it.mainSize, 0);
    const freeSpace = mainAvail - totalMain - totalGaps;

    if (freeSpace > 0) {
      const totalGrow = line.reduce((s, it) => s + it.itemProps.grow, 0);
      if (totalGrow > 0) {
        for (const item of line) {
          if (item.itemProps.grow > 0) {
            item.mainSize += (item.itemProps.grow / totalGrow) * freeSpace;
            item.mainSize = clampMain(item.mainSize, item.child, isRow);
          }
        }
      }
    } else if (freeSpace < 0) {
      const totalShrinkWeight = line.reduce(
        (s, it) => s + it.itemProps.shrink * it.mainSize,
        0
      );
      if (totalShrinkWeight > 0) {
        for (const item of line) {
          if (item.itemProps.shrink > 0) {
            const shrinkAmount =
              (item.itemProps.shrink * item.mainSize / totalShrinkWeight) *
              Math.abs(freeSpace);
            item.mainSize = Math.max(0, item.mainSize - shrinkAmount);
            item.mainSize = clampMain(item.mainSize, item.child, isRow);
          }
        }
      }
    }
  }

  // ── Step 6: Resolve cross sizes ───────────────────────────────────────────

  // Measure cross sizes for all items.
  for (const line of lines) {
    // Line cross size = max of all item natural cross sizes.
    let lineCrossSize = 0;
    for (const item of line) {
      const measured = measure_node(
        item.child,
        isRow
          ? constraints_fixed(item.mainSize, crossAvail ?? Infinity)
          : constraints_fixed(crossAvail ?? Infinity, item.mainSize),
        measurer
      );
      item.crossSize = isRow ? measured.height : measured.width;
      item.crossSize = clampCross(item.crossSize, item.child, isRow);
      if (item.crossSize > lineCrossSize) lineCrossSize = item.crossSize;
    }

    // Apply alignment: stretch fills line cross size.
    for (const item of line) {
      const selfAlign = item.itemProps.alignSelf === "auto"
        ? props.alignItems
        : item.itemProps.alignSelf;

      if (selfAlign === "stretch") {
        const crossHint = isRow ? item.child.height : item.child.width;
        if (crossHint === null || crossHint?.kind !== "fixed") {
          item.crossSize = clampCross(lineCrossSize, item.child, isRow);
        }
      }
    }
  }

  // ── Step 7 + 8: Compute positions ─────────────────────────────────────────

  const resolvedInnerWidth =
    innerWidth ??
    (isRow
      ? lines[0].reduce((s, it) => s + it.mainSize, 0) + (lines[0].length - 1) * props.mainGap
      : Math.max(...lines.flatMap(l => l.map(it => it.crossSize))));

  const resolvedInnerHeight =
    innerHeight ??
    (isRow
      ? Math.max(...lines.flatMap(l => l.map(it => it.crossSize)))
      : lines[0].reduce((s, it) => s + it.mainSize, 0) + (lines[0].length - 1) * props.mainGap);

  const positionedChildren: PositionedNode[] = [];
  let lineOffset = 0; // position along cross axis for multi-line

  for (const line of lines) {
    const lineMainSizes = line.map(it => it.mainSize);
    const lineTotalMain = lineMainSizes.reduce((s, v) => s + v, 0);
    const lineTotalGaps = (line.length - 1) * props.mainGap;
    const lineFreeMain = (mainAvail ?? lineTotalMain + lineTotalGaps) - lineTotalMain - lineTotalGaps;

    // Line cross size for alignment: at least the container's inner cross space.
    // This allows alignItems center/end to work relative to the full container height.
    const contentLineCrossSize = Math.max(...line.map(it => it.crossSize), 0);
    const containerInnerCross = isRow
      ? (innerHeight ?? contentLineCrossSize)
      : (innerWidth ?? contentLineCrossSize);
    const lineCrossSize = Math.max(contentLineCrossSize, containerInnerCross);

    // Main axis start positions from justifyContent.
    const mainPositions = computeJustifyPositions(
      line.length,
      lineMainSizes,
      props.mainGap,
      lineFreeMain,
      props.justifyContent
    );

    for (let i = 0; i < line.length; i++) {
      const item = line[i];
      const mainPos = mainPositions[i];
      const selfAlign = item.itemProps.alignSelf === "auto"
        ? props.alignItems
        : item.itemProps.alignSelf;

      const crossPos = computeAlignPosition(
        item.crossSize,
        lineCrossSize,
        selfAlign,
        lineOffset
      );

      const x = isRow ? mainPos + padding.left : crossPos + padding.left;
      const y = isRow ? crossPos + padding.top : mainPos + padding.top;
      const w = isRow ? item.mainSize : item.crossSize;
      const h = isRow ? item.crossSize : item.mainSize;

      positionedChildren.push(
        positioned(x, y, w, h, {
          id: item.child.id,
          content: item.child.content,
          children: [], // children of children — not our job to lay out
          ext: item.child.ext,
        })
      );
    }

    lineOffset += lineCrossSize + props.crossGap;
  }

  // Resolve final container size.
  const totalLinesHeight = lines.reduce((s, line) => {
    return s + Math.max(...line.map(it => it.crossSize), 0);
  }, 0) + Math.max(0, lines.length - 1) * props.crossGap;

  const totalLinesWidth = lines.reduce((s, line) => {
    return s + Math.max(...line.map(it => it.mainSize), 0);
  }, 0);

  let finalWidth: number;
  let finalHeight: number;

  if (container.width?.kind === "fill") {
    finalWidth = clamp(constraints.maxWidth, container.minWidth, container.maxWidth);
  } else if (container.width?.kind === "fixed") {
    finalWidth = clamp(container.width.value, container.minWidth, container.maxWidth);
  } else {
    // wrap or null: size to content
    const contentWidth = isRow
      ? (lines[0]?.reduce((s, it) => s + it.mainSize, 0) ?? 0) + Math.max(0, (lines[0]?.length ?? 1) - 1) * props.mainGap
      : Math.max(...lines.flatMap(l => l.map(it => it.crossSize)), 0);
    finalWidth = clamp(contentWidth + horizontalPad, container.minWidth, container.maxWidth);
  }

  if (container.height?.kind === "fill") {
    finalHeight = clamp(constraints.maxHeight, container.minHeight, container.maxHeight);
  } else if (container.height?.kind === "fixed") {
    finalHeight = clamp(container.height.value, container.minHeight, container.maxHeight);
  } else {
    // wrap or null: size to content
    const contentHeight = isRow
      ? Math.max(...lines.flatMap(l => l.map(it => it.crossSize)), 0)
      : (lines[0]?.reduce((s, it) => s + it.mainSize, 0) ?? 0) + Math.max(0, (lines[0]?.length ?? 1) - 1) * props.mainGap;
    finalHeight = clamp(contentHeight + verticalPad, container.minHeight, container.maxHeight);
  }

  return positioned(0, 0, finalWidth, finalHeight, {
    id: container.id,
    content: null,
    children: positionedChildren,
    ext: container.ext,
  });
}

// ============================================================================
// Private helpers
// ============================================================================

/** Resolve a container dimension (width or height) from its SizeValue. */
function resolveContainerDimension(
  hint: SizeValue | null | undefined,
  available: number,
  min: number | null | undefined,
  max: number | null | undefined,
  _padding: number // reserved for future use
): number | null {
  if (!hint || hint.kind === "wrap") return null; // size to content
  if (hint.kind === "fill") return clamp(available, min, max);
  if (hint.kind === "fixed") return clamp(hint.value, min, max);
  return null;
}

/** Resolve a SizeValue into a concrete number, given available space. */
function resolveSizeValue(
  hint: SizeValue | null | undefined,
  available: number
): number {
  if (!hint) return 0;
  if (hint.kind === "fixed") return hint.value;
  if (hint.kind === "fill") return available;
  return 0; // "wrap" — will be re-measured
}

function clampMain(v: number, node: LayoutNode, isRow: boolean): number {
  return isRow
    ? clamp(v, node.minWidth, node.maxWidth)
    : clamp(v, node.minHeight, node.maxHeight);
}

function clampCross(v: number, node: LayoutNode, isRow: boolean): number {
  return isRow
    ? clamp(v, node.minHeight, node.maxHeight)
    : clamp(v, node.minWidth, node.maxWidth);
}

/**
 * Compute main-axis start positions for each item in a line.
 *
 * Returns an array of `x` or `y` values depending on direction.
 */
function computeJustifyPositions(
  count: number,
  sizes: number[],
  gap: number,
  freeSpace: number,
  justify: ResolvedContainerProps["justifyContent"]
): number[] {
  if (count === 0) return [];

  const positions: number[] = new Array(count).fill(0);

  switch (justify) {
    case "start": {
      let pos = 0;
      for (let i = 0; i < count; i++) {
        positions[i] = pos;
        pos += sizes[i] + gap;
      }
      break;
    }
    case "end": {
      let pos = Math.max(0, freeSpace);
      for (let i = 0; i < count; i++) {
        positions[i] = pos;
        pos += sizes[i] + gap;
      }
      break;
    }
    case "center": {
      let pos = Math.max(0, freeSpace / 2);
      for (let i = 0; i < count; i++) {
        positions[i] = pos;
        pos += sizes[i] + gap;
      }
      break;
    }
    case "between": {
      const totalGap = count > 1 ? freeSpace / (count - 1) : 0;
      let pos = 0;
      for (let i = 0; i < count; i++) {
        positions[i] = pos;
        pos += sizes[i] + gap + (count > 1 ? totalGap : 0);
      }
      break;
    }
    case "around": {
      const perItem = freeSpace / count;
      let pos = perItem / 2;
      for (let i = 0; i < count; i++) {
        positions[i] = pos;
        pos += sizes[i] + gap + perItem;
      }
      break;
    }
    case "evenly": {
      const spacing = freeSpace / (count + 1);
      let pos = spacing;
      for (let i = 0; i < count; i++) {
        positions[i] = pos;
        pos += sizes[i] + gap + spacing;
      }
      break;
    }
  }

  return positions;
}

/**
 * Compute the cross-axis position of an item within a line.
 */
function computeAlignPosition(
  itemCrossSize: number,
  lineCrossSize: number,
  align: "start" | "center" | "end" | "stretch",
  lineOffset: number
): number {
  switch (align) {
    case "start":
    case "stretch":
      return lineOffset;
    case "end":
      return lineOffset + lineCrossSize - itemCrossSize;
    case "center":
      return lineOffset + (lineCrossSize - itemCrossSize) / 2;
  }
}
