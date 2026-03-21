/**
 * tree.ts -- A Rooted Tree Backed by a Directed Graph
 * ====================================================
 *
 * What Is a Tree?
 * ---------------
 *
 * A **tree** is one of the most fundamental data structures in computer science.
 * You encounter trees everywhere:
 *
 * - File systems: directories contain files and subdirectories
 * - HTML/XML: elements contain child elements
 * - Programming languages: Abstract Syntax Trees (ASTs) represent code structure
 * - Organization charts: managers have direct reports
 *
 * Formally, a tree is a connected, acyclic graph where:
 *
 * 1. There is exactly **one root** node (a node with no parent).
 * 2. Every other node has exactly **one parent**.
 * 3. There are **no cycles** -- you can never follow edges and return to where
 *    you started.
 *
 * These constraints mean a tree with N nodes always has exactly N-1 edges.
 *
 *     Tree vs. Graph
 *     ~~~~~~~~~~~~~~
 *
 *     A tree IS a graph (specifically, a directed acyclic graph with the
 *     single-parent constraint). We leverage this by building our Tree on top
 *     of the `Graph` class from the directed-graph package. The `Graph` handles
 *     all the low-level node/edge storage, while this `Tree` class enforces the
 *     tree invariants and provides tree-specific operations like traversals,
 *     depth calculation, and lowest common ancestor.
 *
 *     Edges point from parent to child:
 *
 *         Program
 *         +-- Assignment    (edge: Program -> Assignment)
 *         |   +-- Name      (edge: Assignment -> Name)
 *         |   +-- BinaryOp  (edge: Assignment -> BinaryOp)
 *         +-- Print         (edge: Program -> Print)
 *
 *
 * Tree Terminology
 * ----------------
 *
 * - **Root**: The topmost node. No parent. Every tree has exactly one.
 * - **Parent**: The node directly above another node.
 * - **Child**: A node directly below another node.
 * - **Siblings**: Nodes that share the same parent.
 * - **Leaf**: A node with no children.
 * - **Depth**: Number of edges from root to a node. Root = 0.
 * - **Height**: Maximum depth of any node in the tree.
 * - **Subtree**: A node together with all its descendants.
 * - **Path**: The sequence of nodes from root to a given node.
 * - **LCA**: Lowest Common Ancestor -- deepest common ancestor of two nodes.
 *
 * Implementation Strategy
 * -----------------------
 *
 * We store the tree as a `Graph` with edges pointing parent -> child.
 * This means:
 *
 * - `graph.successors(node)` returns the children
 * - `graph.predecessors(node)` returns a list with 0 or 1 element
 *   (the parent, or empty for the root)
 *
 * We maintain the tree invariants by checking them in `addChild`:
 *
 * - The parent must already exist in the tree
 * - The child must NOT already exist (no duplicate nodes)
 * - Since we only add one parent edge per child, cycles are impossible
 */

import { Graph } from "@coding-adventures/directed-graph";
import {
  NodeNotFoundError,
  DuplicateNodeError,
  RootRemovalError,
} from "./errors.js";

/**
 * A rooted tree backed by a `Graph` from the directed-graph package.
 *
 * A tree is a directed graph with three constraints:
 *
 * 1. Exactly one root (no predecessors)
 * 2. Every non-root node has exactly one parent
 * 3. No cycles
 *
 * Edges point parent -> child. Build the tree by specifying a root node,
 * then adding children one at a time with `addChild`.
 *
 * Example:
 *
 *     const t = new Tree("Program");
 *     t.addChild("Program", "Assignment");
 *     t.addChild("Program", "Print");
 *     t.addChild("Assignment", "Name");
 *     t.addChild("Assignment", "BinaryOp");
 *
 *     console.log(t.toAscii());
 *     // Program
 *     // +-- Assignment
 *     // |   +-- BinaryOp
 *     // |   +-- Name
 *     // +-- Print
 */
export class Tree {
  // ------------------------------------------------------------------
  // Internal state
  // ------------------------------------------------------------------
  // The underlying directed graph stores all nodes and edges.
  // The root is recorded at construction time and never changes.

  private readonly _graph: Graph;
  private readonly _root: string;

  // ------------------------------------------------------------------
  // Construction
  // ------------------------------------------------------------------
  // A tree always starts with a root. You can't have an empty tree.

  constructor(root: string) {
    this._graph = new Graph();
    this._graph.addNode(root);
    this._root = root;
  }

  // ------------------------------------------------------------------
  // Mutation
  // ------------------------------------------------------------------

  /**
   * Add a child node under the given parent.
   *
   * This is the primary way to build up a tree. Each call adds one new
   * node and one edge (parent -> child).
   *
   * Throws `NodeNotFoundError` if `parent` is not in the tree.
   * Throws `DuplicateNodeError` if `child` is already in the tree.
   */
  addChild(parent: string, child: string): void {
    if (!this._graph.hasNode(parent)) {
      throw new NodeNotFoundError(parent);
    }
    if (this._graph.hasNode(child)) {
      throw new DuplicateNodeError(child);
    }

    this._graph.addEdge(parent, child);
  }

  /**
   * Remove a node and all its descendants from the tree.
   *
   * This is a "prune" operation -- it cuts off an entire branch.
   *
   * Throws `NodeNotFoundError` if `node` is not in the tree.
   * Throws `RootRemovalError` if `node` is the root.
   */
  removeSubtree(node: string): void {
    if (!this._graph.hasNode(node)) {
      throw new NodeNotFoundError(node);
    }
    if (node === this._root) {
      throw new RootRemovalError();
    }

    // Collect subtree via BFS, then remove in reverse (children first)
    const toRemove = this._collectSubtreeNodes(node);

    for (let i = toRemove.length - 1; i >= 0; i--) {
      this._graph.removeNode(toRemove[i]);
    }
  }

  /**
   * Collect all nodes in the subtree rooted at `node` using BFS.
   * Returns a list starting with `node` and then all descendants.
   */
  private _collectSubtreeNodes(node: string): string[] {
    const result: string[] = [];
    const queue: string[] = [node];

    while (queue.length > 0) {
      const current = queue.shift()!;
      result.push(current);
      for (const child of this._graph.successors(current).sort()) {
        queue.push(child);
      }
    }

    return result;
  }

  // ------------------------------------------------------------------
  // Queries
  // ------------------------------------------------------------------

  /** The root node of the tree. Set at construction time, never changes. */
  get root(): string {
    return this._root;
  }

  /**
   * Return the parent of a node, or null if the node is the root.
   *
   * Throws `NodeNotFoundError` if the node is not in the tree.
   */
  parent(node: string): string | null {
    if (!this._graph.hasNode(node)) {
      throw new NodeNotFoundError(node);
    }

    const preds = this._graph.predecessors(node);
    return preds.length === 0 ? null : preds[0];
  }

  /**
   * Return the children of a node (sorted alphabetically).
   *
   * Throws `NodeNotFoundError` if the node is not in the tree.
   */
  children(node: string): string[] {
    if (!this._graph.hasNode(node)) {
      throw new NodeNotFoundError(node);
    }

    return this._graph.successors(node).sort();
  }

  /**
   * Return the siblings of a node (other children of the same parent).
   *
   * Throws `NodeNotFoundError` if the node is not in the tree.
   */
  siblings(node: string): string[] {
    if (!this._graph.hasNode(node)) {
      throw new NodeNotFoundError(node);
    }

    const parentNode = this.parent(node);
    if (parentNode === null) {
      return [];
    }

    return this.children(parentNode).filter((c) => c !== node);
  }

  /**
   * Return true if the node has no children (a leaf).
   *
   * Throws `NodeNotFoundError` if the node is not in the tree.
   */
  isLeaf(node: string): boolean {
    if (!this._graph.hasNode(node)) {
      throw new NodeNotFoundError(node);
    }

    return this._graph.successors(node).length === 0;
  }

  /**
   * Return true if the node is the root of the tree.
   *
   * Throws `NodeNotFoundError` if the node is not in the tree.
   */
  isRoot(node: string): boolean {
    if (!this._graph.hasNode(node)) {
      throw new NodeNotFoundError(node);
    }

    return node === this._root;
  }

  /**
   * Return the depth of a node (distance from root).
   *
   * Root = 0, its children = 1, grandchildren = 2, etc.
   *
   * Throws `NodeNotFoundError` if the node is not in the tree.
   */
  depth(node: string): number {
    if (!this._graph.hasNode(node)) {
      throw new NodeNotFoundError(node);
    }

    let d = 0;
    let current = node;
    while (current !== this._root) {
      const preds = this._graph.predecessors(current);
      current = preds[0];
      d++;
    }

    return d;
  }

  /**
   * Return the height of the tree (maximum depth of any node).
   *
   * A single-node tree has height 0.
   */
  height(): number {
    let maxDepth = 0;
    const queue: [string, number][] = [[this._root, 0]];

    while (queue.length > 0) {
      const [current, d] = queue.shift()!;
      if (d > maxDepth) {
        maxDepth = d;
      }
      for (const child of this._graph.successors(current)) {
        queue.push([child, d + 1]);
      }
    }

    return maxDepth;
  }

  /** Return the total number of nodes in the tree. */
  size(): number {
    return this._graph.size;
  }

  /** Return a sorted list of all nodes in the tree. */
  nodes(): string[] {
    return this._graph.nodes().sort();
  }

  /** Return all leaf nodes (sorted alphabetically). */
  leaves(): string[] {
    return this._graph
      .nodes()
      .filter((n) => this._graph.successors(n).length === 0)
      .sort();
  }

  /** Return true if the node exists in the tree. */
  hasNode(node: string): boolean {
    return this._graph.hasNode(node);
  }

  // ------------------------------------------------------------------
  // Traversals
  // ------------------------------------------------------------------
  //
  // Tree traversals visit every node exactly once, in different orders.
  //
  // 1. **Preorder** (root first): Visit a node, then visit all its
  //    children. Top-down. Good for: copying a tree, prefix notation.
  //
  // 2. **Postorder** (root last): Visit all children, then the node.
  //    Bottom-up. Good for: computing sizes, deleting trees.
  //
  // 3. **Level-order** (BFS): Visit all nodes at depth 0, then 1,
  //    then 2, etc.
  //
  // For a tree:
  //       A
  //      / \
  //     B   C
  //    / \
  //   D   E
  //
  // Preorder:    A, B, D, E, C
  // Postorder:   D, E, B, C, A
  // Level-order: A, B, C, D, E

  /**
   * Return nodes in preorder (parent before children).
   *
   * Uses an explicit stack. Children are pushed in reverse sorted order
   * so that the smallest pops first.
   */
  preorder(): string[] {
    const result: string[] = [];
    const stack: string[] = [this._root];

    while (stack.length > 0) {
      const node = stack.pop()!;
      result.push(node);
      const childrenSorted = this._graph.successors(node).sort().reverse();
      stack.push(...childrenSorted);
    }

    return result;
  }

  /**
   * Return nodes in postorder (children before parent).
   *
   * Uses a recursive helper. Children visited in sorted order.
   */
  postorder(): string[] {
    const result: string[] = [];
    this._postorderRecursive(this._root, result);
    return result;
  }

  private _postorderRecursive(node: string, result: string[]): void {
    for (const child of this._graph.successors(node).sort()) {
      this._postorderRecursive(child, result);
    }
    result.push(node);
  }

  /**
   * Return nodes in level-order (breadth-first).
   *
   * Classic BFS using a queue. Children visited in sorted order.
   */
  levelOrder(): string[] {
    const result: string[] = [];
    const queue: string[] = [this._root];

    while (queue.length > 0) {
      const node = queue.shift()!;
      result.push(node);
      for (const child of this._graph.successors(node).sort()) {
        queue.push(child);
      }
    }

    return result;
  }

  // ------------------------------------------------------------------
  // Utilities
  // ------------------------------------------------------------------

  /**
   * Return the path from the root to the given node.
   *
   * Throws `NodeNotFoundError` if the node is not in the tree.
   */
  pathTo(node: string): string[] {
    if (!this._graph.hasNode(node)) {
      throw new NodeNotFoundError(node);
    }

    const path: string[] = [];
    let current: string | null = node;

    while (current !== null) {
      path.push(current);
      current = this.parent(current);
    }

    path.reverse();
    return path;
  }

  /**
   * Return the lowest common ancestor (LCA) of nodes a and b.
   *
   * The LCA is the deepest node that is an ancestor of both a and b.
   *
   * Throws `NodeNotFoundError` if a or b is not in the tree.
   */
  lca(a: string, b: string): string {
    if (!this._graph.hasNode(a)) {
      throw new NodeNotFoundError(a);
    }
    if (!this._graph.hasNode(b)) {
      throw new NodeNotFoundError(b);
    }

    const pathA = this.pathTo(a);
    const pathB = this.pathTo(b);

    let lcaNode = this._root;
    const minLen = Math.min(pathA.length, pathB.length);
    for (let i = 0; i < minLen; i++) {
      if (pathA[i] === pathB[i]) {
        lcaNode = pathA[i];
      } else {
        break;
      }
    }

    return lcaNode;
  }

  /**
   * Extract the subtree rooted at the given node.
   *
   * Returns a NEW Tree object. The original tree is not modified.
   *
   * Throws `NodeNotFoundError` if the node is not in the tree.
   */
  subtree(node: string): Tree {
    if (!this._graph.hasNode(node)) {
      throw new NodeNotFoundError(node);
    }

    const newTree = new Tree(node);
    const queue: string[] = [node];

    while (queue.length > 0) {
      const current = queue.shift()!;
      for (const child of this._graph.successors(current).sort()) {
        newTree.addChild(current, child);
        queue.push(child);
      }
    }

    return newTree;
  }

  // ------------------------------------------------------------------
  // Visualization
  // ------------------------------------------------------------------

  /**
   * Render the tree as an ASCII art diagram.
   *
   * Produces output like:
   *
   *     Program
   *     +-- Assignment
   *     |   +-- BinaryOp
   *     |   +-- Name
   *     +-- Print
   */
  toAscii(): string {
    const lines: string[] = [];
    this._asciiRecursive(this._root, "", "", lines);
    return lines.join("\n");
  }

  private _asciiRecursive(
    node: string,
    prefix: string,
    childPrefix: string,
    lines: string[]
  ): void {
    lines.push(prefix + node);
    const kids = this._graph.successors(node).sort();

    for (let i = 0; i < kids.length; i++) {
      if (i < kids.length - 1) {
        this._asciiRecursive(
          kids[i],
          childPrefix + "\u251C\u2500\u2500 ",
          childPrefix + "\u2502   ",
          lines
        );
      } else {
        this._asciiRecursive(
          kids[i],
          childPrefix + "\u2514\u2500\u2500 ",
          childPrefix + "    ",
          lines
        );
      }
    }
  }

  // ------------------------------------------------------------------
  // Graph access
  // ------------------------------------------------------------------

  /** Access the underlying Graph. */
  get graph(): Graph {
    return this._graph;
  }

  // ------------------------------------------------------------------
  // String representation
  // ------------------------------------------------------------------

  toString(): string {
    return `Tree(root="${this._root}", size=${this.size()})`;
  }
}
