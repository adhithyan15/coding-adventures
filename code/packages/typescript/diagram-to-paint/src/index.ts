export const VERSION = "0.1.0";

import type {
  LayoutedGraphDiagram,
  LayoutedGraphEdge,
  LayoutedGraphNode,
  Point,
} from "@coding-adventures/diagram-ir";
import type {
  PaintInstruction,
  PaintPath,
  PaintRect,
  PaintEllipse,
  PaintScene,
  PathCommand,
  PaintText,
} from "@coding-adventures/paint-instructions";

export interface DiagramToPaintOptions {
  background?: string;
  fontFamily?: string;
  titleFontSize?: number;
}

const DEFAULTS: Required<DiagramToPaintOptions> = {
  background: "#ffffff",
  fontFamily: "system-ui",
  titleFontSize: 18,
};

function canvasFontRef(
  family: string,
  size: number,
  weight = 400,
  italic = false,
): string {
  const suffix = italic ? `:${weight}:italic` : `:${weight}`;
  return `canvas:${family}@${size}${suffix}`;
}

function textInstruction(
  x: number,
  y: number,
  text: string,
  fontSize: number,
  fill: string,
  fontFamily: string,
): PaintText {
  return {
    kind: "text",
    x,
    y,
    text,
    font_ref: canvasFontRef(fontFamily, fontSize),
    font_size: fontSize,
    fill,
    text_align: "center",
  };
}

function linePath(points: Point[], stroke: string, strokeWidth: number): PaintPath {
  const commands: PathCommand[] = [];
  points.forEach((point, index) => {
    commands.push(
      index === 0
        ? { kind: "move_to", x: point.x, y: point.y }
        : { kind: "line_to", x: point.x, y: point.y },
    );
  });

  return {
    kind: "path",
    commands,
    stroke,
    stroke_width: strokeWidth,
    fill: "none",
    stroke_cap: "round",
    stroke_join: "round",
  };
}

function arrowhead(edge: LayoutedGraphEdge): PaintPath | null {
  if (edge.kind !== "directed" || edge.points.length < 2) {
    return null;
  }

  const end = edge.points[edge.points.length - 1];
  const prev = edge.points[edge.points.length - 2];
  const dx = end.x - prev.x;
  const dy = end.y - prev.y;
  const length = Math.hypot(dx, dy);
  if (length === 0) {
    return null;
  }

  const ux = dx / length;
  const uy = dy / length;
  const size = 10;
  const baseX = end.x - ux * size;
  const baseY = end.y - uy * size;
  const perpX = -uy;
  const perpY = ux;

  return {
    kind: "path",
    commands: [
      { kind: "move_to", x: end.x, y: end.y },
      { kind: "line_to", x: baseX + perpX * (size * 0.6), y: baseY + perpY * (size * 0.6) },
      { kind: "line_to", x: baseX - perpX * (size * 0.6), y: baseY - perpY * (size * 0.6) },
      { kind: "close" },
    ],
    fill: edge.style.stroke,
    stroke: edge.style.stroke,
    stroke_width: 1,
  };
}

function diamondNode(node: LayoutedGraphNode): PaintPath {
  const centerX = node.x + node.width / 2;
  const centerY = node.y + node.height / 2;
  return {
    kind: "path",
    commands: [
      { kind: "move_to", x: centerX, y: node.y },
      { kind: "line_to", x: node.x + node.width, y: centerY },
      { kind: "line_to", x: centerX, y: node.y + node.height },
      { kind: "line_to", x: node.x, y: centerY },
      { kind: "close" },
    ],
    fill: node.style.fill,
    stroke: node.style.stroke,
    stroke_width: node.style.strokeWidth,
    stroke_join: "round",
  };
}

function nodeShape(node: LayoutedGraphNode): PaintInstruction {
  switch (node.shape) {
    case "ellipse":
      return {
        kind: "ellipse",
        cx: node.x + node.width / 2,
        cy: node.y + node.height / 2,
        rx: node.width / 2,
        ry: node.height / 2,
        fill: node.style.fill,
        stroke: node.style.stroke,
        stroke_width: node.style.strokeWidth,
      } satisfies PaintEllipse;
    case "diamond":
      return diamondNode(node);
    case "rect":
    case "rounded_rect":
    default:
      return {
        kind: "rect",
        x: node.x,
        y: node.y,
        width: node.width,
        height: node.height,
        fill: node.style.fill,
        stroke: node.style.stroke,
        stroke_width: node.style.strokeWidth,
        corner_radius: node.shape === "rounded_rect" ? node.style.cornerRadius : 0,
      } satisfies PaintRect;
  }
}

function nodeLabel(node: LayoutedGraphNode, fontFamily: string): PaintText {
  return textInstruction(
    node.x + node.width / 2,
    node.y + node.height / 2 + node.style.fontSize * 0.35,
    node.label.text,
    node.style.fontSize,
    node.style.textColor,
    fontFamily,
  );
}

function edgeInstructions(edge: LayoutedGraphEdge, fontFamily: string): PaintInstruction[] {
  const result: PaintInstruction[] = [
    linePath(edge.points, edge.style.stroke, edge.style.strokeWidth),
  ];

  const tip = arrowhead(edge);
  if (tip) {
    result.push(tip);
  }

  if (edge.label && edge.labelPosition) {
    result.push(
      textInstruction(
        edge.labelPosition.x,
        edge.labelPosition.y,
        edge.label.text,
        edge.style.fontSize,
        edge.style.textColor,
        fontFamily,
      ),
    );
  }

  return result;
}

export function diagramToPaint(
  diagram: LayoutedGraphDiagram,
  options?: DiagramToPaintOptions,
): PaintScene {
  const resolved = { ...DEFAULTS, ...options };
  const instructions: PaintInstruction[] = [];

  if (diagram.title) {
    instructions.push(
      textInstruction(
        diagram.width / 2,
        28,
        diagram.title,
        resolved.titleFontSize,
        "#111827",
        resolved.fontFamily,
      ),
    );
  }

  for (const edge of diagram.edges) {
    instructions.push(...edgeInstructions(edge, resolved.fontFamily));
  }

  for (const node of diagram.nodes) {
    instructions.push(nodeShape(node));
    instructions.push(nodeLabel(node, resolved.fontFamily));
  }

  return {
    width: diagram.width,
    height: diagram.height,
    background: resolved.background,
    instructions,
  };
}
