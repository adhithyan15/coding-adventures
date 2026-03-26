export const VERSION = "0.1.0";

export type DrawMetadataValue = string | number | boolean;
export type DrawMetadata = Record<string, DrawMetadataValue>;

export interface DrawRectInstruction {
  kind: "rect";
  x: number;
  y: number;
  width: number;
  height: number;
  fill: string;
  metadata?: DrawMetadata;
}

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

export interface DrawRenderer<Output> {
  render(scene: DrawScene): Output;
}

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

export function drawGroup(
  children: DrawInstruction[],
  metadata?: DrawMetadata,
): DrawGroupInstruction {
  return { kind: "group", children, metadata };
}

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

export function renderWith<Output>(
  scene: DrawScene,
  renderer: DrawRenderer<Output>,
): Output {
  return renderer.render(scene);
}
