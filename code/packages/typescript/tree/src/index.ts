/**
 * Tree Library
 * ============
 *
 * A rooted tree data structure backed by a directed graph. Provides traversals
 * (preorder, postorder, level-order), lowest common ancestor, subtree extraction,
 * and ASCII visualization.
 *
 * Quick start:
 *
 *     import { Tree } from "@coding-adventures/tree";
 *
 *     const t = new Tree("root");
 *     t.addChild("root", "child1");
 *     t.addChild("root", "child2");
 *     console.log(t.toAscii());
 *
 * Error classes are available at the top level too:
 *
 *     import { NodeNotFoundError, DuplicateNodeError, RootRemovalError } from "@coding-adventures/tree";
 */

export { Tree } from "./tree.js";
export {
  TreeError,
  NodeNotFoundError,
  DuplicateNodeError,
  RootRemovalError,
} from "./errors.js";
