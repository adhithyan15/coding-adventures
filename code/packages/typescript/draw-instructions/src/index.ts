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

/** A filled rectangle in scene coordinates. */
export interface DrawRectInstruction {
  kind: "rect";
  x: number;
  y: number;
  width: number;
  height: number;
  fill: string;
  metadata?: DrawMetadata;
}

/** A text label positioned directly in scene coordinates. */
export interface DrawTextInstruction {
  kind: "text";
  x: number;
  y: number;
  value: string;
  fill: string;
  fontFamily: string;
  fontSize: number;
  align: "start" | "middle" | "end";
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

export type DrawInstruction =
  | DrawRectInstruction
  | DrawTextInstruction
  | DrawGroupInstruction;

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

/** Convenience constructor for filled rectangles. */
export function drawRect(
  x: number,
  y: number,
  width: number,
  height: number,
  fill: string = "#000000",
  metadata?: DrawMetadata,
): DrawRectInstruction {
  return { kind: "rect", x, y, width, height, fill, metadata };
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
    metadata: options.metadata,
  };
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
