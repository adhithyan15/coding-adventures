/**
 * graph.ts
 * ========
 *
 * DT00 introduces the base undirected graph for the data-structure series.
 * We support both adjacency-list and adjacency-matrix storage so the same
 * public API can serve sparse and dense graphs.
 */

export enum GraphRepr {
  ADJACENCY_LIST = "adjacency_list",
  ADJACENCY_MATRIX = "adjacency_matrix",
}

export type GraphPropertyValue = string | number | boolean | null;
export type GraphPropertyBag = Record<string, GraphPropertyValue>;
export type WeightedEdge<T> = readonly [T, T, number];

function nodeSortKey(node: unknown): string {
  if (typeof node === "string") {
    return `string:${node}`;
  }
  if (
    typeof node === "number" ||
    typeof node === "bigint" ||
    typeof node === "boolean" ||
    typeof node === "symbol" ||
    node === null ||
    node === undefined
  ) {
    return `${typeof node}:${String(node)}`;
  }

  try {
    return `json:${JSON.stringify(node)}`;
  } catch {
    return `string:${String(node)}`;
  }
}

export function compareNodes<T>(left: T, right: T): number {
  return nodeSortKey(left).localeCompare(nodeSortKey(right));
}

function edgeKey<T>(left: T, right: T): string {
  const leftKey = nodeSortKey(left);
  const rightKey = nodeSortKey(right);
  return leftKey <= rightKey ? `${leftKey}\0${rightKey}` : `${rightKey}\0${leftKey}`;
}

export class Graph<T> {
  private readonly _repr: GraphRepr;
  private readonly _adj: Map<T, Map<T, number>>;
  private readonly _nodeList: T[];
  private readonly _nodeIndex: Map<T, number>;
  private readonly _matrix: Array<Array<number | null>>;
  private readonly _graphProperties: GraphPropertyBag;
  private readonly _nodeProperties: Map<T, GraphPropertyBag>;
  private readonly _edgeProperties: Map<string, GraphPropertyBag>;

  constructor(repr: GraphRepr = GraphRepr.ADJACENCY_LIST) {
    this._repr = repr;
    this._adj = new Map();
    this._nodeList = [];
    this._nodeIndex = new Map();
    this._matrix = [];
    this._graphProperties = {};
    this._nodeProperties = new Map();
    this._edgeProperties = new Map();
  }

  get repr(): GraphRepr {
    return this._repr;
  }

  get size(): number {
    return this._repr === GraphRepr.ADJACENCY_LIST
      ? this._adj.size
      : this._nodeList.length;
  }

  addNode(node: T, properties: GraphPropertyBag = {}): void {
    if (this._repr === GraphRepr.ADJACENCY_LIST) {
      if (!this._adj.has(node)) {
        this._adj.set(node, new Map());
        this._nodeProperties.set(node, {});
      }
      this.mergeNodeProperties(node, properties);
      return;
    }

    if (this._nodeIndex.has(node)) {
      this.mergeNodeProperties(node, properties);
      return;
    }

    const index = this._nodeList.length;
    this._nodeList.push(node);
    this._nodeIndex.set(node, index);
    this._nodeProperties.set(node, {});
    this.mergeNodeProperties(node, properties);

    for (const row of this._matrix) {
      row.push(null);
    }
    this._matrix.push(Array.from({ length: index + 1 }, () => null));
  }

  removeNode(node: T): void {
    if (this._repr === GraphRepr.ADJACENCY_LIST) {
      const neighbors = this._adj.get(node);
      if (neighbors === undefined) {
        throw new Error(`Node not found: ${String(node)}`);
      }

      for (const neighbor of Array.from(neighbors.keys())) {
        this._adj.get(neighbor)?.delete(node);
        this._edgeProperties.delete(edgeKey(node, neighbor));
      }

      this._adj.delete(node);
      this._nodeProperties.delete(node);
      return;
    }

    const index = this._nodeIndex.get(node);
    if (index === undefined) {
      throw new Error(`Node not found: ${String(node)}`);
    }

    for (const other of this._nodeList) {
      this._edgeProperties.delete(edgeKey(node, other));
    }

    this._nodeIndex.delete(node);
    this._nodeList.splice(index, 1);
    this._matrix.splice(index, 1);
    for (const row of this._matrix) {
      row.splice(index, 1);
    }

    for (let i = index; i < this._nodeList.length; i++) {
      this._nodeIndex.set(this._nodeList[i], i);
    }
    this._nodeProperties.delete(node);
  }

  hasNode(node: T): boolean {
    return this._repr === GraphRepr.ADJACENCY_LIST
      ? this._adj.has(node)
      : this._nodeIndex.has(node);
  }

  nodes(): ReadonlySet<T> {
    return this._repr === GraphRepr.ADJACENCY_LIST
      ? new Set(this._adj.keys())
      : new Set(this._nodeList);
  }

  addEdge(
    left: T,
    right: T,
    weight = 1.0,
    properties: GraphPropertyBag = {}
  ): void {
    this.addNode(left);
    this.addNode(right);
    this.validateWeight(weight);

    if (this._repr === GraphRepr.ADJACENCY_LIST) {
      this._adj.get(left)!.set(right, weight);
      this._adj.get(right)!.set(left, weight);
      this.mergeEdgeProperties(left, right, { ...properties, weight });
      return;
    }

    const leftIndex = this._nodeIndex.get(left)!;
    const rightIndex = this._nodeIndex.get(right)!;
    this._matrix[leftIndex][rightIndex] = weight;
    this._matrix[rightIndex][leftIndex] = weight;
    this.mergeEdgeProperties(left, right, { ...properties, weight });
  }

  removeEdge(left: T, right: T): void {
    if (this._repr === GraphRepr.ADJACENCY_LIST) {
      const leftNeighbors = this._adj.get(left);
      const rightNeighbors = this._adj.get(right);
      if (leftNeighbors === undefined || !leftNeighbors.has(right) || rightNeighbors === undefined) {
        throw new Error(
          `Edge not found: ${String(left)} -- ${String(right)}`
        );
      }

      leftNeighbors.delete(right);
      rightNeighbors.delete(left);
      this._edgeProperties.delete(edgeKey(left, right));
      return;
    }

    const leftIndex = this._nodeIndex.get(left);
    const rightIndex = this._nodeIndex.get(right);
    if (leftIndex === undefined || rightIndex === undefined) {
      throw new Error(`Edge not found: ${String(left)} -- ${String(right)}`);
    }
    if (this._matrix[leftIndex][rightIndex] === null) {
      throw new Error(`Edge not found: ${String(left)} -- ${String(right)}`);
    }

    this._matrix[leftIndex][rightIndex] = null;
    this._matrix[rightIndex][leftIndex] = null;
    this._edgeProperties.delete(edgeKey(left, right));
  }

  hasEdge(left: T, right: T): boolean {
    if (this._repr === GraphRepr.ADJACENCY_LIST) {
      return this._adj.get(left)?.has(right) ?? false;
    }

    const leftIndex = this._nodeIndex.get(left);
    const rightIndex = this._nodeIndex.get(right);
    if (leftIndex === undefined || rightIndex === undefined) {
      return false;
    }

    return this._matrix[leftIndex][rightIndex] !== null;
  }

  edges(): WeightedEdge<T>[] {
    const result: WeightedEdge<T>[] = [];

    if (this._repr === GraphRepr.ADJACENCY_LIST) {
      const seen = new Set<string>();
      for (const [left, neighbors] of this._adj) {
        for (const [right, weight] of neighbors) {
          const key = edgeKey(left, right);
          if (seen.has(key)) {
            continue;
          }
          seen.add(key);
          result.push(
            compareNodes(left, right) <= 0
              ? [left, right, weight]
              : [right, left, weight]
          );
        }
      }
    } else {
      for (let row = 0; row < this._nodeList.length; row++) {
        for (let col = row; col < this._nodeList.length; col++) {
          const weight = this._matrix[row][col];
          if (weight !== null) {
            result.push([this._nodeList[row], this._nodeList[col], weight]);
          }
        }
      }
    }

    result.sort((left, right) => {
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

    return result;
  }

  edgeWeight(left: T, right: T): number {
    if (this._repr === GraphRepr.ADJACENCY_LIST) {
      const weight = this._adj.get(left)?.get(right);
      if (weight === undefined) {
        throw new Error(`Edge not found: ${String(left)} -- ${String(right)}`);
      }
      return weight;
    }

    const leftIndex = this._nodeIndex.get(left);
    const rightIndex = this._nodeIndex.get(right);
    if (leftIndex === undefined || rightIndex === undefined) {
      throw new Error(`Edge not found: ${String(left)} -- ${String(right)}`);
    }

    const weight = this._matrix[leftIndex][rightIndex];
    if (weight === null) {
      throw new Error(`Edge not found: ${String(left)} -- ${String(right)}`);
    }
    return weight;
  }

  graphProperties(): GraphPropertyBag {
    return { ...this._graphProperties };
  }

  setGraphProperty(key: string, value: GraphPropertyValue): void {
    this._graphProperties[key] = value;
  }

  removeGraphProperty(key: string): void {
    delete this._graphProperties[key];
  }

  nodeProperties(node: T): GraphPropertyBag {
    this.assertNode(node);
    return { ...(this._nodeProperties.get(node) ?? {}) };
  }

  setNodeProperty(node: T, key: string, value: GraphPropertyValue): void {
    this.assertNode(node);
    this._nodeProperties.get(node)![key] = value;
  }

  removeNodeProperty(node: T, key: string): void {
    this.assertNode(node);
    delete this._nodeProperties.get(node)![key];
  }

  edgeProperties(left: T, right: T): GraphPropertyBag {
    this.assertEdge(left, right);
    const properties = this._edgeProperties.get(edgeKey(left, right)) ?? {};
    return { ...properties, weight: this.edgeWeight(left, right) };
  }

  setEdgeProperty(
    left: T,
    right: T,
    key: string,
    value: GraphPropertyValue
  ): void {
    this.assertEdge(left, right);
    if (key === "weight") {
      if (typeof value !== "number") {
        throw new Error("Edge property 'weight' must be a number");
      }
      this.setEdgeWeight(left, right, value);
    }
    this.mergeEdgeProperties(left, right, { [key]: value });
  }

  removeEdgeProperty(left: T, right: T, key: string): void {
    this.assertEdge(left, right);
    if (key === "weight") {
      this.setEdgeWeight(left, right, 1.0);
      this.mergeEdgeProperties(left, right, { weight: 1.0 });
      return;
    }
    delete this._edgeProperties.get(edgeKey(left, right))?.[key];
  }

  neighbors(node: T): ReadonlySet<T> {
    if (this._repr === GraphRepr.ADJACENCY_LIST) {
      const neighbors = this._adj.get(node);
      if (neighbors === undefined) {
        throw new Error(`Node not found: ${String(node)}`);
      }
      return new Set(neighbors.keys());
    }

    const index = this._nodeIndex.get(node);
    if (index === undefined) {
      throw new Error(`Node not found: ${String(node)}`);
    }

    const result = new Set<T>();
    for (let col = 0; col < this._nodeList.length; col++) {
      if (this._matrix[index][col] !== null) {
        result.add(this._nodeList[col]);
      }
    }
    return result;
  }

  neighborsWeighted(node: T): ReadonlyMap<T, number> {
    if (this._repr === GraphRepr.ADJACENCY_LIST) {
      const neighbors = this._adj.get(node);
      if (neighbors === undefined) {
        throw new Error(`Node not found: ${String(node)}`);
      }
      return new Map(neighbors);
    }

    const index = this._nodeIndex.get(node);
    if (index === undefined) {
      throw new Error(`Node not found: ${String(node)}`);
    }

    const result = new Map<T, number>();
    for (let col = 0; col < this._nodeList.length; col++) {
      const weight = this._matrix[index][col];
      if (weight !== null) {
        result.set(this._nodeList[col], weight);
      }
    }
    return result;
  }

  degree(node: T): number {
    return this.neighbors(node).size;
  }

  toString(): string {
    return `Graph(nodes=${this.size}, edges=${this.edges().length}, repr=${this._repr})`;
  }

  private assertNode(node: T): void {
    if (!this.hasNode(node)) {
      throw new Error(`Node not found: ${String(node)}`);
    }
  }

  private assertEdge(left: T, right: T): void {
    if (!this.hasEdge(left, right)) {
      throw new Error(`Edge not found: ${String(left)} -- ${String(right)}`);
    }
  }

  private mergeNodeProperties(node: T, properties: GraphPropertyBag): void {
    const existing = this._nodeProperties.get(node);
    if (existing === undefined) {
      return;
    }
    Object.assign(existing, properties);
  }

  private mergeEdgeProperties(
    left: T,
    right: T,
    properties: GraphPropertyBag
  ): void {
    const key = edgeKey(left, right);
    const existing = this._edgeProperties.get(key) ?? {};
    Object.assign(existing, properties);
    this._edgeProperties.set(key, existing);
  }

  private validateWeight(weight: number): void {
    if (typeof weight !== "number" || Number.isNaN(weight)) {
      throw new Error("Edge weight must be a number");
    }
  }

  private setEdgeWeight(left: T, right: T, weight: number): void {
    this.validateWeight(weight);

    if (this._repr === GraphRepr.ADJACENCY_LIST) {
      this._adj.get(left)!.set(right, weight);
      this._adj.get(right)!.set(left, weight);
      return;
    }

    const leftIndex = this._nodeIndex.get(left)!;
    const rightIndex = this._nodeIndex.get(right)!;
    this._matrix[leftIndex][rightIndex] = weight;
    this._matrix[rightIndex][leftIndex] = weight;
  }
}
