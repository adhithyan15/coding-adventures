class RadixNode<V> {
  readonly children = new Map<string, RadixEdge<V>>();
  terminal = false;
  value: V | undefined;
}

interface RadixEdge<V> {
  label: string;
  child: RadixNode<V>;
}

export class RadixTree<V> {
  private readonly root = new RadixNode<V>();
  private keyCount = 0;

  constructor(entries: Iterable<readonly [string, V]> = []) {
    for (const [key, value] of entries) {
      this.insert(key, value);
    }
  }

  get size(): number {
    return this.keyCount;
  }

  insert(key: string, value: V): void {
    if (this.insertRecursive(this.root, key, value)) {
      this.keyCount += 1;
    }
  }

  put(key: string, value: V): void {
    this.insert(key, value);
  }

  search(key: string): V | undefined {
    let node = this.root;
    let remaining = key;

    while (remaining.length > 0) {
      const edge = node.children.get(firstChar(remaining));
      if (edge === undefined) {
        return undefined;
      }
      const common = commonPrefixLength(remaining, edge.label);
      if (common < edge.label.length) {
        return undefined;
      }
      remaining = remaining.slice(common);
      node = edge.child;
    }

    return node.terminal ? node.value : undefined;
  }

  get(key: string): V | undefined {
    return this.search(key);
  }

  containsKey(key: string): boolean {
    return this.keyExists(key);
  }

  delete(key: string): boolean {
    const [deleted] = this.deleteRecursive(this.root, key);
    if (deleted) {
      this.keyCount -= 1;
    }
    return deleted;
  }

  startsWith(prefix: string): boolean {
    if (prefix.length === 0) {
      return this.keyCount > 0;
    }

    let node = this.root;
    let remaining = prefix;
    while (remaining.length > 0) {
      const edge = node.children.get(firstChar(remaining));
      if (edge === undefined) {
        return false;
      }
      const common = commonPrefixLength(remaining, edge.label);
      if (common === remaining.length) {
        return true;
      }
      if (common < edge.label.length) {
        return false;
      }
      remaining = remaining.slice(common);
      node = edge.child;
    }

    return node.terminal || node.children.size > 0;
  }

  wordsWithPrefix(prefix: string): string[] {
    if (prefix.length === 0) {
      return this.keys();
    }

    let node = this.root;
    let remaining = prefix;
    let path = "";

    while (remaining.length > 0) {
      const edge = node.children.get(firstChar(remaining));
      if (edge === undefined) {
        return [];
      }
      const common = commonPrefixLength(remaining, edge.label);
      if (common === remaining.length) {
        if (common === edge.label.length) {
          path += edge.label;
          node = edge.child;
          remaining = "";
        } else {
          const results: string[] = [];
          this.collectKeys(edge.child, path + edge.label, results);
          return results;
        }
      } else if (common < edge.label.length) {
        return [];
      } else {
        path += edge.label;
        remaining = remaining.slice(common);
        node = edge.child;
      }
    }

    const results: string[] = [];
    this.collectKeys(node, path, results);
    return results;
  }

  longestPrefixMatch(key: string): string | undefined {
    let node = this.root;
    let remaining = key;
    let consumed = 0;
    let best = node.terminal ? "" : undefined;

    while (remaining.length > 0) {
      const edge = node.children.get(firstChar(remaining));
      if (edge === undefined) {
        break;
      }
      const common = commonPrefixLength(remaining, edge.label);
      if (common < edge.label.length) {
        break;
      }
      consumed += common;
      remaining = remaining.slice(common);
      node = edge.child;
      if (node.terminal) {
        best = key.slice(0, consumed);
      }
    }

    return best;
  }

  keys(): string[] {
    const results: string[] = [];
    this.collectKeys(this.root, "", results);
    return results;
  }

  values(): V[] {
    return Array.from(this.toMap().values());
  }

  toMap(): Map<string, V> {
    const result = new Map<string, V>();
    this.collectValues(this.root, "", result);
    return result;
  }

  nodeCount(): number {
    return this.countNodes(this.root);
  }

  isEmpty(): boolean {
    return this.keyCount === 0;
  }

  toString(): string {
    const preview = Array.from(this.toMap()).slice(0, 5);
    return `RadixTree(${this.keyCount} keys: ${JSON.stringify(preview)})`;
  }

  private insertRecursive(node: RadixNode<V>, key: string, value: V): boolean {
    if (key.length === 0) {
      const added = !node.terminal;
      node.terminal = true;
      node.value = value;
      return added;
    }

    const first = firstChar(key);
    const edge = node.children.get(first);
    if (edge === undefined) {
      node.children.set(first, { label: key, child: leaf(value) });
      return true;
    }

    const common = commonPrefixLength(key, edge.label);
    if (common === edge.label.length) {
      return this.insertRecursive(edge.child, key.slice(common), value);
    }

    const commonLabel = edge.label.slice(0, common);
    const labelRest = edge.label.slice(common);
    const keyRest = key.slice(common);
    const splitNode = new RadixNode<V>();
    splitNode.children.set(firstChar(labelRest), { label: labelRest, child: edge.child });

    if (keyRest.length === 0) {
      splitNode.terminal = true;
      splitNode.value = value;
    } else {
      splitNode.children.set(firstChar(keyRest), { label: keyRest, child: leaf(value) });
    }

    node.children.set(first, { label: commonLabel, child: splitNode });
    return true;
  }

  private deleteRecursive(node: RadixNode<V>, key: string): [deleted: boolean, mergeable: boolean] {
    if (key.length === 0) {
      if (!node.terminal) {
        return [false, false];
      }
      node.terminal = false;
      node.value = undefined;
      return [true, node.children.size === 1];
    }

    const first = firstChar(key);
    const edge = node.children.get(first);
    if (edge === undefined) {
      return [false, false];
    }
    const common = commonPrefixLength(key, edge.label);
    if (common < edge.label.length) {
      return [false, false];
    }

    const [deleted, childMergeable] = this.deleteRecursive(edge.child, key.slice(common));
    if (!deleted) {
      return [false, false];
    }

    if (childMergeable) {
      const [[, grandchildEdge]] = edge.child.children;
      node.children.set(first, {
        label: edge.label + grandchildEdge.label,
        child: grandchildEdge.child,
      });
    } else if (!edge.child.terminal && edge.child.children.size === 0) {
      node.children.delete(first);
    }

    return [true, !node.terminal && node.children.size === 1];
  }

  private keyExists(key: string): boolean {
    let node = this.root;
    let remaining = key;
    while (remaining.length > 0) {
      const edge = node.children.get(firstChar(remaining));
      if (edge === undefined) {
        return false;
      }
      const common = commonPrefixLength(remaining, edge.label);
      if (common < edge.label.length) {
        return false;
      }
      remaining = remaining.slice(common);
      node = edge.child;
    }
    return node.terminal;
  }

  private collectKeys(node: RadixNode<V>, current: string, results: string[]): void {
    if (node.terminal) {
      results.push(current);
    }
    for (const first of Array.from(node.children.keys()).sort()) {
      const edge = node.children.get(first)!;
      this.collectKeys(edge.child, current + edge.label, results);
    }
  }

  private collectValues(node: RadixNode<V>, current: string, result: Map<string, V>): void {
    if (node.terminal) {
      result.set(current, node.value as V);
    }
    for (const first of Array.from(node.children.keys()).sort()) {
      const edge = node.children.get(first)!;
      this.collectValues(edge.child, current + edge.label, result);
    }
  }

  private countNodes(node: RadixNode<V>): number {
    let count = 1;
    for (const edge of node.children.values()) {
      count += this.countNodes(edge.child);
    }
    return count;
  }
}

function leaf<V>(value: V): RadixNode<V> {
  const node = new RadixNode<V>();
  node.terminal = true;
  node.value = value;
  return node;
}

function firstChar(value: string): string {
  return value[0]!;
}

function commonPrefixLength(a: string, b: string): number {
  let index = 0;
  while (index < a.length && index < b.length && a[index] === b[index]) {
    index += 1;
  }
  return index;
}
