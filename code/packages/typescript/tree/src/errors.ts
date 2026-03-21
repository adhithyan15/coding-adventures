/**
 * errors.ts -- Custom Error Classes for the Tree Library
 * ======================================================
 *
 * Trees impose strict structural constraints on top of directed graphs. When
 * those constraints are violated, we need clear, specific errors rather than
 * generic `Error`. Each error class here corresponds to one particular kind
 * of violation:
 *
 * - `TreeError` -- the base class for all tree-specific errors. You can
 *   catch this to handle any tree error generically, or catch a more
 *   specific subclass when you want to handle one case differently.
 *
 * - `NodeNotFoundError` -- raised when you reference a node that doesn't
 *   exist in the tree. This is the tree-level equivalent of the directed
 *   graph's `NodeNotFoundError`, but we define our own so that callers can
 *   catch tree errors without importing the graph library.
 *
 * - `DuplicateNodeError` -- raised when you try to add a node that already
 *   exists. In a tree, every node name must be unique because each node has
 *   exactly one position in the hierarchy.
 *
 * - `RootRemovalError` -- raised when you try to remove the root node. The
 *   root is the anchor of the entire tree; removing it would leave a
 *   disconnected collection of subtrees.
 */

/**
 * Base class for all tree-related errors.
 *
 * This exists so callers can write `catch (e) { if (e instanceof TreeError) }`
 * to handle any tree error without listing every subclass.
 */
export class TreeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TreeError";
  }
}

/**
 * Thrown when an operation references a node not in the tree.
 *
 * The `node` property carries the missing node's name, so error messages
 * can tell you exactly what was missing.
 */
export class NodeNotFoundError extends TreeError {
  public readonly node: string;

  constructor(node: string) {
    super(`Node not found in tree: "${node}"`);
    this.name = "NodeNotFoundError";
    this.node = node;
  }
}

/**
 * Thrown when trying to add a node that already exists in the tree.
 *
 * In a tree, every node occupies a unique position. If you could add a
 * node twice, it would have two parents -- violating the tree invariant
 * that every non-root node has exactly one parent.
 */
export class DuplicateNodeError extends TreeError {
  public readonly node: string;

  constructor(node: string) {
    super(`Node already exists in tree: "${node}"`);
    this.name = "DuplicateNodeError";
    this.node = node;
  }
}

/**
 * Thrown when trying to remove the root node.
 *
 * The root is special: it's the only node with no parent, and every other
 * node is reachable from it. Removing the root would destroy the tree's
 * connected structure.
 */
export class RootRemovalError extends TreeError {
  constructor() {
    super("Cannot remove the root node");
    this.name = "RootRemovalError";
  }
}
