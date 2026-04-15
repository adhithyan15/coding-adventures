import { createRequire } from "module";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(join(__dirname, "package.json"));
const native = require("./graph_native_node.node");

export const GraphRepr = Object.freeze({
  ADJACENCY_LIST: "adjacency_list",
  ADJACENCY_MATRIX: "adjacency_matrix",
});

function cloneEdge(edge) {
  return [edge[0], edge[1], edge[2]];
}

export class Graph {
  #native;

  constructor(repr = GraphRepr.ADJACENCY_LIST) {
    this.#native = new native.NativeGraph(repr);
  }

  get repr() {
    return this.#native.repr();
  }

  get size() {
    return this.#native.size();
  }

  addNode(node) {
    this.#native.addNode(node);
  }

  removeNode(node) {
    this.#native.removeNode(node);
  }

  hasNode(node) {
    return this.#native.hasNode(node);
  }

  nodes() {
    return new Set(this.#native.nodes());
  }

  addEdge(left, right, weight = 1.0) {
    this.#native.addEdge(left, right, weight);
  }

  removeEdge(left, right) {
    this.#native.removeEdge(left, right);
  }

  hasEdge(left, right) {
    return this.#native.hasEdge(left, right);
  }

  edges() {
    return this.#native.edges().map(cloneEdge);
  }

  edgeWeight(left, right) {
    return this.#native.edgeWeight(left, right);
  }

  neighbors(node) {
    return new Set(this.#native.neighbors(node));
  }

  neighborsWeighted(node) {
    return new Map(this.#native.neighborsWeightedEntries(node));
  }

  degree(node) {
    return this.#native.degree(node);
  }

  bfs(start) {
    return this.#native.bfs(start);
  }

  dfs(start) {
    return this.#native.dfs(start);
  }

  isConnected() {
    return this.#native.isConnected();
  }

  connectedComponents() {
    return this.#native.connectedComponents().map((component) => new Set(component));
  }

  hasCycle() {
    return this.#native.hasCycle();
  }

  shortestPath(start, finish) {
    return this.#native.shortestPath(start, finish);
  }

  minimumSpanningTree() {
    return new Set(this.#native.minimumSpanningTree().map(cloneEdge));
  }

  toString() {
    return this.#native.toString();
  }
}

export function bfs(graph, start) {
  return graph.bfs(start);
}

export function dfs(graph, start) {
  return graph.dfs(start);
}

export function isConnected(graph) {
  return graph.isConnected();
}

export function connectedComponents(graph) {
  return graph.connectedComponents();
}

export function hasCycle(graph) {
  return graph.hasCycle();
}

export function shortestPath(graph, start, finish) {
  return graph.shortestPath(start, finish);
}

export function minimumSpanningTree(graph) {
  return graph.minimumSpanningTree();
}
