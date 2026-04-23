export const VERSION = "0.1.0";

export type DiagramDirection = "lr" | "rl" | "tb" | "bt";

export interface DiagramLabel {
  text: string;
}

export type DiagramShape = "rect" | "rounded_rect" | "ellipse" | "diamond";

export interface DiagramStyle {
  fill?: string;
  stroke?: string;
  strokeWidth?: number;
  textColor?: string;
  fontSize?: number;
  cornerRadius?: number;
}

export interface ResolvedDiagramStyle {
  fill: string;
  stroke: string;
  strokeWidth: number;
  textColor: string;
  fontSize: number;
  cornerRadius: number;
}

export interface GraphNode {
  id: string;
  label: DiagramLabel;
  shape?: DiagramShape;
  style?: DiagramStyle;
}

export interface GraphEdge {
  id?: string;
  from: { node: string };
  to: { node: string };
  label?: DiagramLabel;
  kind: "directed" | "undirected";
  style?: DiagramStyle;
}

export interface GraphDiagram {
  kind: "graph";
  direction: DiagramDirection;
  title?: string;
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface Point {
  x: number;
  y: number;
}

export interface LayoutedGraphNode {
  id: string;
  label: DiagramLabel;
  shape: DiagramShape;
  x: number;
  y: number;
  width: number;
  height: number;
  style: ResolvedDiagramStyle;
}

export interface LayoutedGraphEdge {
  id?: string;
  fromNodeId: string;
  toNodeId: string;
  kind: "directed" | "undirected";
  points: Point[];
  label?: DiagramLabel;
  labelPosition?: Point;
  style: ResolvedDiagramStyle;
}

export interface LayoutedGraphDiagram {
  kind: "layouted_graph";
  direction: DiagramDirection;
  width: number;
  height: number;
  title?: string;
  nodes: LayoutedGraphNode[];
  edges: LayoutedGraphEdge[];
}

export function graphNode(
  id: string,
  options?: Partial<Omit<GraphNode, "id">>,
): GraphNode {
  return {
    id,
    label: options?.label ?? { text: id },
    shape: options?.shape ?? "rounded_rect",
    style: options?.style,
  };
}

export function graphEdge(
  from: string,
  to: string,
  options?: Partial<Omit<GraphEdge, "from" | "to" | "kind">> & {
    kind?: GraphEdge["kind"];
  },
): GraphEdge {
  return {
    id: options?.id,
    from: { node: from },
    to: { node: to },
    label: options?.label,
    kind: options?.kind ?? "directed",
    style: options?.style,
  };
}

export function graphDiagram(
  nodes: GraphNode[],
  edges: GraphEdge[],
  options?: Partial<Omit<GraphDiagram, "kind" | "nodes" | "edges">>,
): GraphDiagram {
  return {
    kind: "graph",
    direction: options?.direction ?? "tb",
    title: options?.title,
    nodes,
    edges,
  };
}

export function resolveStyle(
  style: DiagramStyle | undefined,
  defaults?: Partial<ResolvedDiagramStyle>,
): ResolvedDiagramStyle {
  return {
    fill: style?.fill ?? defaults?.fill ?? "#ffffff",
    stroke: style?.stroke ?? defaults?.stroke ?? "#1f2937",
    strokeWidth: style?.strokeWidth ?? defaults?.strokeWidth ?? 2,
    textColor: style?.textColor ?? defaults?.textColor ?? "#111827",
    fontSize: style?.fontSize ?? defaults?.fontSize ?? 14,
    cornerRadius: style?.cornerRadius ?? defaults?.cornerRadius ?? 10,
  };
}
