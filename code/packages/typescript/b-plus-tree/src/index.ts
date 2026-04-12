/**
 * B+ Tree Library — DT12
 * ======================
 *
 * A generic B+ tree with leaf-linked-list for O(1) full scans and efficient
 * range queries. All data lives in the leaves; internal nodes act as routing
 * index only.
 *
 * Quick start:
 *
 *     import { BPlusTree } from "@coding-adventures/b-plus-tree";
 *
 *     const tree = new BPlusTree<number, string>(2, (a, b) => a - b);
 *     tree.insert(10, "ten");
 *     tree.insert(5,  "five");
 *     tree.insert(20, "twenty");
 *
 *     console.log(tree.search(10));             // "ten"
 *     console.log(tree.fullScan());             // [[5, "five"], [10, "ten"], [20, "twenty"]]
 *     console.log(tree.rangeScan(5, 15));       // [[5, "five"], [10, "ten"]]
 *
 *     for (const [k, v] of tree) {
 *       console.log(k, v);  // sorted iteration
 *     }
 *
 * Node types are also exported for advanced introspection.
 */

export { BPlusTree } from "./b-plus-tree.js";
export type { BPlusInternalNode, BPlusLeafNode } from "./b-plus-tree.js";
