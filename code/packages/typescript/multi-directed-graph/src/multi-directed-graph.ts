export type GraphPropertyValue = string | number | boolean | null;
export type GraphPropertyBag = Record<string, GraphPropertyValue>;

export interface MultiDirectedGraphOptions {
  readonly allowSelfLoops?: boolean;
}

export interface MultiDirectedEdge<T = string> {
  readonly id: string;
  readonly from: T;
  readonly to: T;
  readonly weight: number;
}

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

function compareNodes<T>(left: T, right: T): number {
  return nodeSortKey(left).localeCompare(nodeSortKey(right));
}

export class NodeNotFoundError<T = unknown> extends Error {
  constructor(public readonly node: T) {
    super(`Node not found: ${String(node)}`);
    this.name = "NodeNotFoundError";
  }
}

export class EdgeNotFoundError extends Error {
  constructor(public readonly edgeId: string) {
    super(`Edge not found: ${edgeId}`);
    this.name = "EdgeNotFoundError";
  }
}

export class DuplicateEdgeIdError extends Error {
  constructor(public readonly edgeId: string) {
    super(`Edge ID already exists: ${edgeId}`);
    this.name = "DuplicateEdgeIdError";
  }
}

export class MultiDirectedGraphCycleError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "MultiDirectedGraphCycleError";
  }
}

export class MultiDirectedGraph<T = string> {
  private readonly _allowSelfLoops: boolean;
  private readonly _nodes: Set<T> = new Set();
  private readonly _edges: Map<string, MultiDirectedEdge<T>> = new Map();
  private readonly _outgoing: Map<T, Set<string>> = new Map();
  private readonly _incoming: Map<T, Set<string>> = new Map();
  private readonly _graphProperties: GraphPropertyBag = {};
  private readonly _nodeProperties: Map<T, GraphPropertyBag> = new Map();
  private readonly _edgeProperties: Map<string, GraphPropertyBag> = new Map();
  private _nextEdgeId = 0;

  constructor(options: MultiDirectedGraphOptions = {}) {
    this._allowSelfLoops = options.allowSelfLoops ?? false;
  }

  get allowSelfLoops(): boolean {
    return this._allowSelfLoops;
  }

  get size(): number {
    return this._nodes.size;
  }

  addNode(node: T, properties: GraphPropertyBag = {}): void {
    if (!this._nodes.has(node)) {
      this._nodes.add(node);
      this._outgoing.set(node, new Set());
      this._incoming.set(node, new Set());
      this._nodeProperties.set(node, {});
    }
    Object.assign(this._nodeProperties.get(node)!, properties);
  }

  removeNode(node: T): void {
    this.assertNode(node);
    const edgeIds = new Set([
      ...this._outgoing.get(node)!,
      ...this._incoming.get(node)!,
    ]);
    for (const edgeId of edgeIds) {
      this.removeEdge(edgeId);
    }
    this._nodes.delete(node);
    this._outgoing.delete(node);
    this._incoming.delete(node);
    this._nodeProperties.delete(node);
  }

  hasNode(node: T): boolean {
    return this._nodes.has(node);
  }

  nodes(): T[] {
    return Array.from(this._nodes);
  }

  addEdge(
    from: T,
    to: T,
    weight = 1.0,
    properties: GraphPropertyBag = {},
    edgeId?: string
  ): string {
    if (from === to && !this._allowSelfLoops) {
      throw new Error(
        `Self-loops are not allowed: ${String(from)} -> ${String(to)}`
      );
    }
    this.validateWeight(weight);

    const id = edgeId ?? this.allocateEdgeId();
    if (this._edges.has(id)) {
      throw new DuplicateEdgeIdError(id);
    }

    this.addNode(from);
    this.addNode(to);

    const edge: MultiDirectedEdge<T> = { id, from, to, weight };
    this._edges.set(id, edge);
    this._outgoing.get(from)!.add(id);
    this._incoming.get(to)!.add(id);
    this._edgeProperties.set(id, { ...properties, weight });
    return id;
  }

  removeEdge(edgeId: string): void {
    const edge = this._edges.get(edgeId);
    if (edge === undefined) {
      throw new EdgeNotFoundError(edgeId);
    }
    this._outgoing.get(edge.from)!.delete(edgeId);
    this._incoming.get(edge.to)!.delete(edgeId);
    this._edges.delete(edgeId);
    this._edgeProperties.delete(edgeId);
  }

  hasEdge(edgeId: string): boolean {
    return this._edges.has(edgeId);
  }

  edge(edgeId: string): MultiDirectedEdge<T> {
    const edge = this._edges.get(edgeId);
    if (edge === undefined) {
      throw new EdgeNotFoundError(edgeId);
    }
    return edge;
  }

  edges(): MultiDirectedEdge<T>[] {
    return Array.from(this._edges.values());
  }

  edgesBetween(from: T, to: T): MultiDirectedEdge<T>[] {
    this.assertNode(from);
    this.assertNode(to);
    return this.outgoingEdges(from).filter((edge) => edge.to === to);
  }

  outgoingEdges(node: T): MultiDirectedEdge<T>[] {
    this.assertNode(node);
    return Array.from(this._outgoing.get(node)!, (edgeId) => this.edge(edgeId));
  }

  incomingEdges(node: T): MultiDirectedEdge<T>[] {
    this.assertNode(node);
    return Array.from(this._incoming.get(node)!, (edgeId) => this.edge(edgeId));
  }

  successors(node: T): T[] {
    return Array.from(new Set(this.outgoingEdges(node).map((edge) => edge.to)));
  }

  predecessors(node: T): T[] {
    return Array.from(new Set(this.incomingEdges(node).map((edge) => edge.from)));
  }

  edgeWeight(edgeId: string): number {
    return this.edge(edgeId).weight;
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

  setNodeProperty(
    node: T,
    key: string,
    value: GraphPropertyValue
  ): void {
    this.assertNode(node);
    this._nodeProperties.get(node)![key] = value;
  }

  removeNodeProperty(node: T, key: string): void {
    this.assertNode(node);
    delete this._nodeProperties.get(node)![key];
  }

  edgeProperties(edgeId: string): GraphPropertyBag {
    this.assertEdge(edgeId);
    return {
      ...(this._edgeProperties.get(edgeId) ?? {}),
      weight: this.edgeWeight(edgeId),
    };
  }

  setEdgeProperty(
    edgeId: string,
    key: string,
    value: GraphPropertyValue
  ): void {
    this.assertEdge(edgeId);
    if (key === "weight") {
      if (typeof value !== "number" || Number.isNaN(value)) {
        throw new Error("Edge property 'weight' must be a number");
      }
      this.setEdgeWeight(edgeId, value);
    }
    this._edgeProperties.get(edgeId)![key] = value;
  }

  removeEdgeProperty(edgeId: string, key: string): void {
    this.assertEdge(edgeId);
    if (key === "weight") {
      this.setEdgeWeight(edgeId, 1.0);
      this._edgeProperties.get(edgeId)!["weight"] = 1.0;
      return;
    }
    delete this._edgeProperties.get(edgeId)![key];
  }

  topologicalSort(): T[] {
    const inDegree = new Map<T, number>();
    for (const node of this._nodes) {
      inDegree.set(node, this._incoming.get(node)!.size);
    }

    const queue = Array.from(this._nodes)
      .filter((node) => inDegree.get(node) === 0)
      .sort(compareNodes);
    const result: T[] = [];

    while (queue.length > 0) {
      const node = queue.shift()!;
      result.push(node);
      for (const edge of this.outgoingEdges(node)) {
        const nextDegree = inDegree.get(edge.to)! - 1;
        inDegree.set(edge.to, nextDegree);
        if (nextDegree === 0) {
          queue.push(edge.to);
          queue.sort(compareNodes);
        }
      }
    }

    if (result.length !== this._nodes.size) {
      throw new MultiDirectedGraphCycleError(
        `Graph contains a cycle: processed ${result.length}/${this._nodes.size} nodes`
      );
    }

    return result;
  }

  hasCycle(): boolean {
    try {
      this.topologicalSort();
      return false;
    } catch (error) {
      if (error instanceof MultiDirectedGraphCycleError) {
        return true;
      }
      throw error;
    }
  }

  independentGroups(): T[][] {
    const inDegree = new Map<T, number>();
    for (const node of this._nodes) {
      inDegree.set(node, this._incoming.get(node)!.size);
    }

    let current = Array.from(this._nodes)
      .filter((node) => inDegree.get(node) === 0)
      .sort(compareNodes);
    const groups: T[][] = [];
    let processed = 0;

    while (current.length > 0) {
      groups.push(current);
      processed += current.length;
      const next = new Set<T>();
      for (const node of current) {
        for (const edge of this.outgoingEdges(node)) {
          const nextDegree = inDegree.get(edge.to)! - 1;
          inDegree.set(edge.to, nextDegree);
          if (nextDegree === 0) {
            next.add(edge.to);
          }
        }
      }
      current = Array.from(next).sort(compareNodes);
    }

    if (processed !== this._nodes.size) {
      throw new MultiDirectedGraphCycleError(
        `Graph contains a cycle: processed ${processed}/${this._nodes.size} nodes`
      );
    }

    return groups;
  }

  toString(): string {
    return `MultiDirectedGraph(nodes=${this.size}, edges=${this._edges.size})`;
  }

  private allocateEdgeId(): string {
    let edgeId = `e${this._nextEdgeId}`;
    while (this._edges.has(edgeId)) {
      this._nextEdgeId += 1;
      edgeId = `e${this._nextEdgeId}`;
    }
    this._nextEdgeId += 1;
    return edgeId;
  }

  private assertNode(node: T): void {
    if (!this._nodes.has(node)) {
      throw new NodeNotFoundError(node);
    }
  }

  private assertEdge(edgeId: string): void {
    if (!this._edges.has(edgeId)) {
      throw new EdgeNotFoundError(edgeId);
    }
  }

  private validateWeight(weight: number): void {
    if (typeof weight !== "number" || Number.isNaN(weight)) {
      throw new Error("Edge weight must be a number");
    }
  }

  private setEdgeWeight(edgeId: string, weight: number): void {
    this.validateWeight(weight);
    const edge = this.edge(edgeId);
    this._edges.set(edgeId, { ...edge, weight });
  }
}
