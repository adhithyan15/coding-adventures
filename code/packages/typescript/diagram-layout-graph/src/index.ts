export const VERSION = "0.1.0";

import { CycleError, Graph } from "@coding-adventures/directed-graph";
import {
  type DiagramDirection,
  type GraphDiagram,
  type LayoutedGraphDiagram,
  type LayoutedGraphEdge,
  type LayoutedGraphNode,
  resolveStyle,
} from "@coding-adventures/diagram-ir";

export interface GraphLayoutOptions {
  margin?: number;
  rankGap?: number;
  nodeGap?: number;
  titleGap?: number;
  minNodeWidth?: number;
  nodeHeight?: number;
  horizontalPadding?: number;
  charWidth?: number;
}

const DEFAULTS: Required<GraphLayoutOptions> = {
  margin: 24,
  rankGap: 96,
  nodeGap: 56,
  titleGap: 48,
  minNodeWidth: 96,
  nodeHeight: 52,
  horizontalPadding: 24,
  charWidth: 8,
};

function nodeWidth(label: string, options: Required<GraphLayoutOptions>): number {
  return Math.max(
    options.minNodeWidth,
    options.horizontalPadding * 2 + label.length * options.charWidth,
  );
}

function orderedRanks(diagram: GraphDiagram): string[][] {
  const graph = new Graph({ allowSelfLoops: true });
  for (const node of diagram.nodes) {
    graph.addNode(node.id);
  }
  for (const edge of diagram.edges) {
    graph.addEdge(edge.from.node, edge.to.node);
  }

  try {
    const order = graph.topologicalSort();
    const rankMap = new Map<string, number>();

    for (const nodeId of order) {
      const predecessors = graph.predecessors(nodeId);
      const rank =
        predecessors.length === 0
          ? 0
          : Math.max(...predecessors.map((pred) => rankMap.get(pred) ?? 0)) + 1;
      rankMap.set(nodeId, rank);
    }

    const ranks = new Map<number, string[]>();
    for (const node of diagram.nodes) {
      const rank = rankMap.get(node.id) ?? 0;
      if (!ranks.has(rank)) {
        ranks.set(rank, []);
      }
      ranks.get(rank)!.push(node.id);
    }

    return Array.from(ranks.entries())
      .sort(([a], [b]) => a - b)
      .map(([, ids]) => ids);
  } catch (error) {
    if (!(error instanceof CycleError)) {
      throw error;
    }

    return diagram.nodes.map((node) => [node.id]);
  }
}

function placeNodes(
  diagram: GraphDiagram,
  options: Required<GraphLayoutOptions>,
): { nodes: LayoutedGraphNode[]; width: number; height: number } {
  const direction = diagram.direction;
  const ranks = orderedRanks(diagram);
  const topInset = diagram.title ? options.titleGap : 0;
  const rankSizes = ranks.map((rank) =>
    Math.max(
      ...rank.map((id) => {
        const node = diagram.nodes.find((candidate) => candidate.id === id)!;
        return nodeWidth(node.label.text, options);
      }),
    ),
  );

  const nodes: LayoutedGraphNode[] = [];

  for (let rankIndex = 0; rankIndex < ranks.length; rankIndex++) {
    const rank = ranks[rankIndex];
    for (let itemIndex = 0; itemIndex < rank.length; itemIndex++) {
      const nodeId = rank[itemIndex];
      const node = diagram.nodes.find((candidate) => candidate.id === nodeId)!;
      const width = nodeWidth(node.label.text, options);
      const height = options.nodeHeight;

      const majorIndex =
        direction === "rl" || direction === "bt"
          ? ranks.length - rankIndex - 1
          : rankIndex;

      let x: number;
      let y: number;

      if (direction === "lr" || direction === "rl") {
        x =
          options.margin +
          majorIndex * (Math.max(...rankSizes) + options.rankGap);
        y = options.margin + topInset + itemIndex * (height + options.nodeGap);
      } else {
        x = options.margin + itemIndex * (width + options.nodeGap);
        y =
          options.margin +
          topInset +
          majorIndex * (height + options.rankGap);
      }

      nodes.push({
        id: node.id,
        label: node.label,
        shape: node.shape ?? "rounded_rect",
        x,
        y,
        width,
        height,
        style: resolveStyle(node.style),
      });
    }
  }

  const maxX = Math.max(...nodes.map((node) => node.x + node.width), options.margin * 2);
  const maxY = Math.max(...nodes.map((node) => node.y + node.height), options.margin * 2);

  return {
    nodes,
    width: maxX + options.margin,
    height: maxY + options.margin,
  };
}

function edgeEndpoints(
  direction: DiagramDirection,
  fromNode: LayoutedGraphNode,
  toNode: LayoutedGraphNode,
): { start: { x: number; y: number }; end: { x: number; y: number } } {
  if (fromNode.id === toNode.id) {
    const centerX = fromNode.x + fromNode.width / 2;
    return {
      start: { x: fromNode.x + fromNode.width, y: fromNode.y + fromNode.height / 2 },
      end: { x: centerX, y: fromNode.y },
    };
  }

  switch (direction) {
    case "lr":
      return {
        start: { x: fromNode.x + fromNode.width, y: fromNode.y + fromNode.height / 2 },
        end: { x: toNode.x, y: toNode.y + toNode.height / 2 },
      };
    case "rl":
      return {
        start: { x: fromNode.x, y: fromNode.y + fromNode.height / 2 },
        end: { x: toNode.x + toNode.width, y: toNode.y + toNode.height / 2 },
      };
    case "bt":
      return {
        start: { x: fromNode.x + fromNode.width / 2, y: fromNode.y },
        end: { x: toNode.x + toNode.width / 2, y: toNode.y + toNode.height },
      };
    case "tb":
    default:
      return {
        start: { x: fromNode.x + fromNode.width / 2, y: fromNode.y + fromNode.height },
        end: { x: toNode.x + toNode.width / 2, y: toNode.y },
      };
  }
}

function routeEdge(
  edge: GraphDiagram["edges"][number],
  direction: DiagramDirection,
  nodesById: Map<string, LayoutedGraphNode>,
): LayoutedGraphEdge {
  const fromNode = nodesById.get(edge.from.node);
  const toNode = nodesById.get(edge.to.node);

  if (!fromNode || !toNode) {
    throw new Error(`Cannot route edge ${edge.from.node} -> ${edge.to.node}: missing node`);
  }

  const { start, end } = edgeEndpoints(direction, fromNode, toNode);
  const points =
    fromNode.id === toNode.id
      ? [
          start,
          { x: start.x + 28, y: start.y },
          { x: start.x + 28, y: fromNode.y - 28 },
          { x: fromNode.x + fromNode.width / 2, y: fromNode.y - 28 },
          end,
        ]
      : [start, end];

  return {
    id: edge.id,
    fromNodeId: fromNode.id,
    toNodeId: toNode.id,
    kind: edge.kind,
    points,
    label: edge.label,
    labelPosition: edge.label
      ? {
          x: (start.x + end.x) / 2,
          y: (start.y + end.y) / 2 - 8,
        }
      : undefined,
    style: resolveStyle(edge.style, {
      fill: "none",
      stroke: "#4b5563",
      strokeWidth: 2,
      textColor: "#374151",
      fontSize: 12,
      cornerRadius: 0,
    }),
  };
}

export function layoutGraphDiagram(
  diagram: GraphDiagram,
  options?: GraphLayoutOptions,
): LayoutedGraphDiagram {
  const resolved = { ...DEFAULTS, ...options };
  const placement = placeNodes(diagram, resolved);
  const nodesById = new Map(placement.nodes.map((node) => [node.id, node]));

  return {
    kind: "layouted_graph",
    direction: diagram.direction,
    width: placement.width,
    height: placement.height,
    title: diagram.title,
    nodes: placement.nodes,
    edges: diagram.edges.map((edge) =>
      routeEdge(edge, diagram.direction, nodesById),
    ),
  };
}
