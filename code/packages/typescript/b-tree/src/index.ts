/**
 * B-Tree Library — DT11
 * =====================
 *
 * A generic B-tree data structure with proactive top-down splitting and full
 * deletion support (all CLRS cases).
 *
 * Quick start:
 *
 *     import { BTree } from "@coding-adventures/b-tree";
 *
 *     const tree = new BTree<number, string>(2, (a, b) => a - b);
 *     tree.insert(10, "ten");
 *     tree.insert(5,  "five");
 *     tree.insert(20, "twenty");
 *
 *     console.log(tree.search(10));       // "ten"
 *     console.log(tree.inorder());        // [[5, "five"], [10, "ten"], [20, "twenty"]]
 *     console.log(tree.rangeQuery(5, 15)); // [[5, "five"], [10, "ten"]]
 *
 * The `BTreeNode` interface is also exported for advanced use cases.
 */

export { BTree } from "./b-tree.js";
export type { BTreeNode } from "./b-tree.js";
