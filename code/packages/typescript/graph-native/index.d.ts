export type WeightedEdge = readonly [string, string, number];

export const GraphRepr: Readonly<{
  ADJACENCY_LIST: "adjacency_list";
  ADJACENCY_MATRIX: "adjacency_matrix";
}>;

export type GraphReprValue = (typeof GraphRepr)[keyof typeof GraphRepr];

export class Graph {
  constructor(repr?: GraphReprValue);
  readonly repr: GraphReprValue;
  readonly size: number;
  addNode(node: string): void;
  removeNode(node: string): void;
  hasNode(node: string): boolean;
  nodes(): Set<string>;
  addEdge(left: string, right: string, weight?: number): void;
  removeEdge(left: string, right: string): void;
  hasEdge(left: string, right: string): boolean;
  edges(): WeightedEdge[];
  edgeWeight(left: string, right: string): number;
  neighbors(node: string): Set<string>;
  neighborsWeighted(node: string): Map<string, number>;
  degree(node: string): number;
  bfs(start: string): string[];
  dfs(start: string): string[];
  isConnected(): boolean;
  connectedComponents(): Set<string>[];
  hasCycle(): boolean;
  shortestPath(start: string, finish: string): string[];
  minimumSpanningTree(): Set<WeightedEdge>;
  toString(): string;
}

export function bfs(graph: Graph, start: string): string[];
export function dfs(graph: Graph, start: string): string[];
export function isConnected(graph: Graph): boolean;
export function connectedComponents(graph: Graph): Set<string>[];
export function hasCycle(graph: Graph): boolean;
export function shortestPath(graph: Graph, start: string, finish: string): string[];
export function minimumSpanningTree(graph: Graph): Set<WeightedEdge>;
