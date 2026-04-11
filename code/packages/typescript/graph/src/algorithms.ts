/**
 * algorithms.ts
 * =============
 *
 * Pure algorithms for the DT00 undirected graph. Each helper accepts a graph
 * instance and returns a result without mutating the graph.
 */

import { compareNodes, type Graph, type WeightedEdge } from "./graph.js";

function sortedNodes<T>(nodes: Iterable<T>): T[] {
  return Array.from(nodes).sort(compareNodes);
}

class MinPriorityQueue<T> {
  private readonly _items: Array<{ priority: number; seq: number; value: T }> =
    [];

  push(priority: number, seq: number, value: T): void {
    this._items.push({ priority, seq, value });
    this._bubbleUp(this._items.length - 1);
  }

  pop(): { priority: number; seq: number; value: T } | undefined {
    if (this._items.length === 0) {
      return undefined;
    }

    const top = this._items[0];
    const last = this._items.pop()!;
    if (this._items.length > 0) {
      this._items[0] = last;
      this._bubbleDown(0);
    }
    return top;
  }

  get size(): number {
    return this._items.length;
  }

  private _bubbleUp(index: number): void {
    while (index > 0) {
      const parentIndex = Math.floor((index - 1) / 2);
      if (this._compare(this._items[parentIndex], this._items[index]) <= 0) {
        break;
      }
      [this._items[parentIndex], this._items[index]] = [
        this._items[index],
        this._items[parentIndex],
      ];
      index = parentIndex;
    }
  }

  private _bubbleDown(index: number): void {
    while (true) {
      let smallest = index;
      const left = index * 2 + 1;
      const right = index * 2 + 2;

      if (
        left < this._items.length &&
        this._compare(this._items[left], this._items[smallest]) < 0
      ) {
        smallest = left;
      }

      if (
        right < this._items.length &&
        this._compare(this._items[right], this._items[smallest]) < 0
      ) {
        smallest = right;
      }

      if (smallest === index) {
        return;
      }

      [this._items[index], this._items[smallest]] = [
        this._items[smallest],
        this._items[index],
      ];
      index = smallest;
    }
  }

  private _compare(
    left: { priority: number; seq: number },
    right: { priority: number; seq: number }
  ): number {
    if (left.priority !== right.priority) {
      return left.priority - right.priority;
    }
    return left.seq - right.seq;
  }
}

class UnionFind<T> {
  private readonly _parent = new Map<T, T>();
  private readonly _rank = new Map<T, number>();

  constructor(nodes: Iterable<T>) {
    for (const node of nodes) {
      this._parent.set(node, node);
      this._rank.set(node, 0);
    }
  }

  find(node: T): T {
    const parent = this._parent.get(node);
    if (parent === undefined) {
      throw new Error(`UnionFind node missing: ${String(node)}`);
    }
    if (parent !== node) {
      this._parent.set(node, this.find(parent));
    }
    return this._parent.get(node)!;
  }

  union(left: T, right: T): void {
    let rootLeft = this.find(left);
    let rootRight = this.find(right);
    if (rootLeft === rootRight) {
      return;
    }

    const rankLeft = this._rank.get(rootLeft)!;
    const rankRight = this._rank.get(rootRight)!;
    if (rankLeft < rankRight) {
      [rootLeft, rootRight] = [rootRight, rootLeft];
    }

    this._parent.set(rootRight, rootLeft);
    if (rankLeft === rankRight) {
      this._rank.set(rootLeft, rankLeft + 1);
    }
  }
}

export function bfs<T>(graph: Graph<T>, start: T): T[] {
  if (!graph.hasNode(start)) {
    throw new Error(`Node not found: ${String(start)}`);
  }

  const visited = new Set<T>([start]);
  const queue: T[] = [start];
  const result: T[] = [];

  while (queue.length > 0) {
    const node = queue.shift()!;
    result.push(node);

    for (const neighbor of sortedNodes(graph.neighbors(node))) {
      if (!visited.has(neighbor)) {
        visited.add(neighbor);
        queue.push(neighbor);
      }
    }
  }

  return result;
}

export function dfs<T>(graph: Graph<T>, start: T): T[] {
  if (!graph.hasNode(start)) {
    throw new Error(`Node not found: ${String(start)}`);
  }

  const visited = new Set<T>();
  const stack: T[] = [start];
  const result: T[] = [];

  while (stack.length > 0) {
    const node = stack.pop()!;
    if (visited.has(node)) {
      continue;
    }

    visited.add(node);
    result.push(node);

    for (const neighbor of sortedNodes(graph.neighbors(node)).reverse()) {
      if (!visited.has(neighbor)) {
        stack.push(neighbor);
      }
    }
  }

  return result;
}

export function isConnected<T>(graph: Graph<T>): boolean {
  if (graph.size === 0) {
    return true;
  }

  const [start] = sortedNodes(graph.nodes());
  return bfs(graph, start).length === graph.size;
}

export function connectedComponents<T>(graph: Graph<T>): Array<ReadonlySet<T>> {
  const unvisited = new Set<T>(graph.nodes());
  const result: Array<ReadonlySet<T>> = [];

  while (unvisited.size > 0) {
    const [start] = sortedNodes(unvisited);
    const component = new Set<T>(bfs(graph, start));
    result.push(component);
    for (const node of component) {
      unvisited.delete(node);
    }
  }

  return result;
}

export function hasCycle<T>(graph: Graph<T>): boolean {
  const visited = new Set<T>();

  for (const start of sortedNodes(graph.nodes())) {
    if (visited.has(start)) {
      continue;
    }

    const stack: Array<readonly [T, T | null]> = [[start, null]];
    while (stack.length > 0) {
      const [node, parent] = stack.pop()!;
      if (visited.has(node)) {
        continue;
      }

      visited.add(node);
      for (const neighbor of graph.neighbors(node)) {
        if (!visited.has(neighbor)) {
          stack.push([neighbor, node]);
        } else if (neighbor !== parent) {
          return true;
        }
      }
    }
  }

  return false;
}

export function shortestPath<T>(graph: Graph<T>, start: T, end: T): T[] {
  if (!graph.hasNode(start) || !graph.hasNode(end)) {
    return [];
  }
  if (start === end) {
    return [start];
  }

  const allUnit = graph.edges().every(([, , weight]) => weight === 1.0);
  return allUnit
    ? bfsShortestPath(graph, start, end)
    : dijkstraShortestPath(graph, start, end);
}

function bfsShortestPath<T>(graph: Graph<T>, start: T, end: T): T[] {
  const parent = new Map<T, T | null>([[start, null]]);
  const queue: T[] = [start];

  while (queue.length > 0) {
    const node = queue.shift()!;
    if (node === end) {
      break;
    }

    for (const neighbor of sortedNodes(graph.neighbors(node))) {
      if (!parent.has(neighbor)) {
        parent.set(neighbor, node);
        queue.push(neighbor);
      }
    }
  }

  if (!parent.has(end)) {
    return [];
  }

  const path: T[] = [];
  let current: T | null = end;
  while (current !== null) {
    path.push(current);
    current = parent.get(current) ?? null;
  }

  return path.reverse();
}

function dijkstraShortestPath<T>(graph: Graph<T>, start: T, end: T): T[] {
  const distances = new Map<T, number>();
  const parent = new Map<T, T | null>();
  const queue = new MinPriorityQueue<T>();
  let sequence = 0;

  for (const node of graph.nodes()) {
    distances.set(node, Number.POSITIVE_INFINITY);
  }

  distances.set(start, 0);
  queue.push(0, sequence, start);

  while (queue.size > 0) {
    const current = queue.pop()!;
    const currentDistance = distances.get(current.value)!;
    if (current.priority > currentDistance) {
      continue;
    }
    if (current.value === end) {
      break;
    }

    const neighbors = Array.from(graph.neighborsWeighted(current.value).entries())
      .sort((left, right) => compareNodes(left[0], right[0]));

    for (const [neighbor, weight] of neighbors) {
      const nextDistance = currentDistance + weight;
      if (nextDistance < (distances.get(neighbor) ?? Number.POSITIVE_INFINITY)) {
        distances.set(neighbor, nextDistance);
        parent.set(neighbor, current.value);
        sequence += 1;
        queue.push(nextDistance, sequence, neighbor);
      }
    }
  }

  if ((distances.get(end) ?? Number.POSITIVE_INFINITY) === Number.POSITIVE_INFINITY) {
    return [];
  }

  const path: T[] = [];
  let current: T | null = end;
  while (current !== null) {
    path.push(current);
    current = parent.get(current) ?? null;
  }

  return path.reverse();
}

export function minimumSpanningTree<T>(
  graph: Graph<T>
): ReadonlySet<WeightedEdge<T>> {
  const nodes = Array.from(graph.nodes());
  if (nodes.length === 0 || graph.edges().length === 0) {
    return new Set();
  }

  const sortedEdges = [...graph.edges()].sort((left, right) => {
    const byWeight = left[2] - right[2];
    if (byWeight !== 0) {
      return byWeight;
    }
    const byFirst = compareNodes(left[0], right[0]);
    if (byFirst !== 0) {
      return byFirst;
    }
    return compareNodes(left[1], right[1]);
  });

  const uf = new UnionFind(nodes);
  const mst = new Set<WeightedEdge<T>>();

  for (const edge of sortedEdges) {
    const [left, right] = edge;
    if (uf.find(left) !== uf.find(right)) {
      uf.union(left, right);
      mst.add(edge);
      if (mst.size === nodes.length - 1) {
        break;
      }
    }
  }

  if (mst.size < nodes.length - 1 && nodes.length > 1) {
    throw new Error(
      "minimumSpanningTree: graph is not connected and has no spanning tree"
    );
  }

  return mst;
}
