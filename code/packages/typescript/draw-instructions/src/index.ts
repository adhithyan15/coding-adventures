/**
 * @coding-adventures/draw-instructions
 *
 * This package defines a tiny, backend-neutral scene model.
 *
 * The key idea is separation of concerns:
 * - producer packages decide WHAT should be drawn
 * - renderer packages decide HOW to serialize or paint it
 *
 * In barcode terms:
 * - Code 39 decides where the bars and labels go
 * - this package provides generic rectangles/text/groups
 * - SVG, PNG, Canvas, or terminal renderers can all consume the same scene
 *
 * Why use rectangles instead of a special "barcode bar" primitive?
 * Because a 1D barcode bar is just a tall thin rectangle, and a 2D barcode
 * module is just a small square rectangle. A general rectangle primitive
 * covers both cases cleanly.
 */
export const VERSION = "0.1.0";

/**
 * Metadata is intentionally lightweight.
 *
 * It lets producers attach domain meaning without polluting the shared scene
 * model with domain-specific fields. For example, a barcode package might store:
 *   - source character
 *   - source index
 *   - symbology name
 *
 * while some other visualization might store:
 *   - node id
 *   - pipeline stage
 *   - semantic label
 */
export type DrawMetadataValue = string | number | boolean;
export type DrawMetadata = Record<string, DrawMetadataValue>;

/**
 * A rectangle in scene coordinates.
 *
 * Rectangles can be filled, stroked, or both. A filled rectangle with no
 * stroke draws a solid block of color. A stroked rectangle with no fill
 * draws an outline. Both together draw a filled box with a visible border.
 *
 * The table component uses filled rects for header/row backgrounds and
 * stroked rects for focus rings.
 */
export interface DrawRectInstruction {
  kind: "rect";
  x: number;
  y: number;
  width: number;
  height: number;
  fill: string;
  /** Optional border color. When set, a stroke is drawn around the rect. */
  stroke?: string;
  /** Border thickness in scene units. Only meaningful when stroke is set. */
  strokeWidth?: number;
  metadata?: DrawMetadata;
}

/**
 * A text label positioned directly in scene coordinates.
 *
 * The table component uses text instructions for header labels and cell
 * values. Bold text (fontWeight: "bold") distinguishes headers from body
 * cells.
 */
export interface DrawTextInstruction {
  kind: "text";
  x: number;
  y: number;
  value: string;
  fill: string;
  fontFamily: string;
  fontSize: number;
  align: "start" | "middle" | "end";
  /** Font weight. Default: "normal". Headers typically use "bold". */
  fontWeight?: "normal" | "bold";
  metadata?: DrawMetadata;
}

/**
 * A group provides hierarchical structure without introducing transforms yet.
 *
 * Groups are useful when producers want to preserve semantic structure.
 * Example:
 * - one group per encoded symbol
 * - one group per overlay layer
 * - one group for guides vs final artwork
 */
export interface DrawGroupInstruction {
  kind: "group";
  children: DrawInstruction[];
  metadata?: DrawMetadata;
}

/**
 * A straight line segment between two points.
 *
 * Lines are the backbone of grid rendering. A table's horizontal and
 * vertical grid lines are each one DrawLineInstruction. Lines are always
 * stroked (never filled).
 */
export interface DrawLineInstruction {
  kind: "line";
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  stroke: string;
  strokeWidth: number;
  metadata?: DrawMetadata;
}

/**
 * A clipping region that constrains its children.
 *
 * Any drawing by children that falls outside the clip rectangle is
 * invisible. This is how the table prevents cell text from bleeding
 * into adjacent columns — each cell's text is wrapped in a clip
 * instruction bounded to the cell's dimensions.
 *
 * Clip instructions nest: a child clip intersects with its parent.
 */
export interface DrawClipInstruction {
  kind: "clip";
  x: number;
  y: number;
  width: number;
  height: number;
  children: DrawInstruction[];
  metadata?: DrawMetadata;
}

export type DrawInstruction =
  | DrawRectInstruction
  | DrawTextInstruction
  | DrawGroupInstruction
  | DrawLineInstruction
  | DrawClipInstruction;

export interface DrawScene {
  width: number;
  height: number;
  background: string;
  instructions: DrawInstruction[];
  metadata?: DrawMetadata;
}

/**
 * A renderer consumes a full scene and returns some backend-specific output.
 *
 * Common examples:
 * - DrawRenderer<string> for SVG output
 * - DrawRenderer<Uint8Array> for a PNG encoder
 * - DrawRenderer<void> for painting directly to a canvas context
 */
export interface DrawRenderer<Output> {
  render(scene: DrawScene): Output;
}

/**
 * Convenience constructor for rectangles.
 *
 * The basic form creates a filled rectangle. The last parameter accepts
 * either a metadata object (backward compatible) or an options object
 * with stroke, strokeWidth, and metadata.
 */
export function drawRect(
  x: number,
  y: number,
  width: number,
  height: number,
  fill: string = "#000000",
  metadataOrOptions?: DrawMetadata | {
    stroke?: string;
    strokeWidth?: number;
    metadata?: DrawMetadata;
  },
): DrawRectInstruction {
  // Backward compatibility: if the 6th arg has `stroke` or `strokeWidth`,
  // it's the new options form. Otherwise treat it as metadata directly.
  if (
    metadataOrOptions !== undefined &&
    ("stroke" in metadataOrOptions || "strokeWidth" in metadataOrOptions)
  ) {
    const opts = metadataOrOptions as {
      stroke?: string;
      strokeWidth?: number;
      metadata?: DrawMetadata;
    };
    return {
      kind: "rect",
      x,
      y,
      width,
      height,
      fill,
      stroke: opts.stroke,
      strokeWidth: opts.strokeWidth,
      metadata: opts.metadata,
    };
  }

  return {
    kind: "rect",
    x,
    y,
    width,
    height,
    fill,
    metadata: metadataOrOptions as DrawMetadata | undefined,
  };
}

/**
 * Convenience constructor for text.
 *
 * Defaults are deliberately conservative: monospace font, centered alignment,
 * black fill. Producers can override them when needed, but the common case
 * stays terse.
 */
export function drawText(
  x: number,
  y: number,
  value: string,
  options: Partial<Omit<DrawTextInstruction, "kind" | "x" | "y" | "value">> = {},
): DrawTextInstruction {
  return {
    kind: "text",
    x,
    y,
    value,
    fill: options.fill ?? "#000000",
    fontFamily: options.fontFamily ?? "monospace",
    fontSize: options.fontSize ?? 16,
    align: options.align ?? "middle",
    fontWeight: options.fontWeight,
    metadata: options.metadata,
  };
}

/**
 * Convenience constructor for a line segment.
 *
 * Lines are always stroked. Use these for grid lines, separators, and
 * borders.
 */
export function drawLine(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  stroke: string = "#000000",
  strokeWidth: number = 1,
  metadata?: DrawMetadata,
): DrawLineInstruction {
  return { kind: "line", x1, y1, x2, y2, stroke, strokeWidth, metadata };
}

/**
 * Convenience constructor for a clip region.
 *
 * Everything drawn by `children` is clipped to the rectangle defined by
 * (x, y, width, height). Content outside the rectangle is invisible.
 */
export function drawClip(
  x: number,
  y: number,
  width: number,
  height: number,
  children: DrawInstruction[],
  metadata?: DrawMetadata,
): DrawClipInstruction {
  return { kind: "clip", x, y, width, height, children, metadata };
}

/** Convenience constructor for a group of instructions. */
export function drawGroup(
  children: DrawInstruction[],
  metadata?: DrawMetadata,
): DrawGroupInstruction {
  return { kind: "group", children, metadata };
}

/**
 * Create a complete scene.
 *
 * A scene is the unit renderers consume. Width and height are explicit because
 * renderers should not have to infer output bounds from the instructions.
 */
export function createScene(
  width: number,
  height: number,
  instructions: DrawInstruction[],
  options: { background?: string; metadata?: DrawMetadata } = {},
): DrawScene {
  return {
    width,
    height,
    background: options.background ?? "#ffffff",
    instructions,
    metadata: options.metadata,
  };
}

/** Delegate rendering to a backend implementation. */
export function renderWith<Output>(
  scene: DrawScene,
  renderer: DrawRenderer<Output>,
): Output {
  return renderer.render(scene);
}
