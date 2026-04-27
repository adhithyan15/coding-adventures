class TrieNode<V> {
  readonly children = new Map<string, TrieNode<V>>();
  terminal = false;
  value: V | undefined;
}

export type TrieEntry<V> = [key: string, value: V];

export class Trie<V> {
  private readonly root = new TrieNode<V>();
  private keyCount = 0;

  constructor(entries: Iterable<readonly [string, V]> = []) {
    for (const [key, value] of entries) {
      this.insert(key, value);
    }
  }

  static fromEntries<V>(entries: Iterable<readonly [string, V]>): Trie<V> {
    return new Trie(entries);
  }

  get size(): number {
    return this.keyCount;
  }

  insert(key: string, value: V): void {
    let node = this.root;
    for (const char of key) {
      let child = node.children.get(char);
      if (child === undefined) {
        child = new TrieNode<V>();
        node.children.set(char, child);
      }
      node = child;
    }

    if (!node.terminal) {
      this.keyCount += 1;
    }
    node.terminal = true;
    node.value = value;
  }

  search(key: string): V | undefined {
    const node = this.findNode(key);
    return node?.terminal === true ? node.value : undefined;
  }

  containsKey(key: string): boolean {
    return this.keyExists(key);
  }

  delete(key: string): boolean {
    if (!this.keyExists(key)) {
      return false;
    }

    this.deleteRecursive(this.root, Array.from(key), 0);
    this.keyCount -= 1;
    return true;
  }

  startsWith(prefix: string): boolean {
    if (prefix.length === 0) {
      return this.keyCount > 0;
    }
    return this.findNode(prefix) !== undefined;
  }

  wordsWithPrefix(prefix: string): TrieEntry<V>[] {
    const node = this.findNode(prefix);
    if (node === undefined) {
      return [];
    }

    const results: TrieEntry<V>[] = [];
    this.collect(node, prefix, results);
    return results;
  }

  allWords(): TrieEntry<V>[] {
    const results: TrieEntry<V>[] = [];
    this.collect(this.root, "", results);
    return results;
  }

  entries(): TrieEntry<V>[] {
    return this.allWords();
  }

  keys(): string[] {
    return this.allWords().map(([key]) => key);
  }

  longestPrefixMatch(input: string): TrieEntry<V> | undefined {
    let node = this.root;
    let current = "";
    let best: TrieEntry<V> | undefined = node.terminal ? ["", node.value as V] : undefined;

    for (const char of input) {
      const child = node.children.get(char);
      if (child === undefined) {
        break;
      }
      current += char;
      node = child;
      if (node.terminal) {
        best = [current, node.value as V];
      }
    }

    return best;
  }

  isEmpty(): boolean {
    return this.keyCount === 0;
  }

  isValid(): boolean {
    return this.countEndpoints(this.root) === this.keyCount;
  }

  toString(): string {
    const preview = this.allWords().slice(0, 5);
    return `Trie(${this.keyCount} keys: ${JSON.stringify(preview)})`;
  }

  private findNode(key: string): TrieNode<V> | undefined {
    let node = this.root;
    for (const char of key) {
      const child = node.children.get(char);
      if (child === undefined) {
        return undefined;
      }
      node = child;
    }
    return node;
  }

  private keyExists(key: string): boolean {
    return this.findNode(key)?.terminal === true;
  }

  private collect(node: TrieNode<V>, current: string, results: TrieEntry<V>[]): void {
    if (node.terminal) {
      results.push([current, node.value as V]);
    }

    for (const char of Array.from(node.children.keys()).sort()) {
      this.collect(node.children.get(char)!, current + char, results);
    }
  }

  private deleteRecursive(node: TrieNode<V>, chars: string[], depth: number): boolean {
    if (depth === chars.length) {
      node.terminal = false;
      node.value = undefined;
      return node.children.size === 0;
    }

    const char = chars[depth]!;
    const child = node.children.get(char);
    if (child === undefined) {
      return false;
    }

    if (this.deleteRecursive(child, chars, depth + 1)) {
      node.children.delete(char);
    }

    return node.children.size === 0 && !node.terminal;
  }

  private countEndpoints(node: TrieNode<V>): number {
    let count = node.terminal ? 1 : 0;
    for (const child of node.children.values()) {
      count += this.countEndpoints(child);
    }
    return count;
  }
}
