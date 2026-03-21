/**
 * tree.test.ts -- Comprehensive Tests for the Tree Library
 * ========================================================
 *
 * Organized by category:
 *
 * 1. Construction -- creating trees, verifying initial state
 * 2. addChild -- building trees, error cases
 * 3. removeSubtree -- pruning branches, error cases
 * 4. Queries -- parent, children, siblings, isLeaf, isRoot, depth, height, etc.
 * 5. Traversals -- preorder, postorder, levelOrder
 * 6. pathTo -- root-to-node paths
 * 7. lca -- lowest common ancestor
 * 8. subtree -- extracting subtrees
 * 9. toAscii -- ASCII visualization
 * 10. Edge cases -- single-node trees, deep chains, wide trees
 * 11. graph property -- accessing the underlying Graph
 */

import { describe, it, expect } from "vitest";
import { Graph } from "@coding-adventures/directed-graph";
import {
  Tree,
  TreeError,
  NodeNotFoundError,
  DuplicateNodeError,
  RootRemovalError,
} from "../src/index.js";

// =========================================================================
// Helper: Build a sample tree for many tests
// =========================================================================
//
//         A
//        / \
//       B   C
//      / \   \
//     D   E   F
//    /
//   G

function makeSampleTree(): Tree {
  const t = new Tree("A");
  t.addChild("A", "B");
  t.addChild("A", "C");
  t.addChild("B", "D");
  t.addChild("B", "E");
  t.addChild("C", "F");
  t.addChild("D", "G");
  return t;
}

// =========================================================================
// 1. Construction
// =========================================================================

describe("Construction", () => {
  it("creates a tree with the specified root", () => {
    const t = new Tree("root");
    expect(t.root).toBe("root");
  });

  it("new tree has size one", () => {
    const t = new Tree("root");
    expect(t.size()).toBe(1);
  });

  it("root is a leaf in a new tree", () => {
    const t = new Tree("root");
    expect(t.isLeaf("root")).toBe(true);
  });

  it("root is identified as root", () => {
    const t = new Tree("root");
    expect(t.isRoot("root")).toBe(true);
  });

  it("root has no parent", () => {
    const t = new Tree("root");
    expect(t.parent("root")).toBeNull();
  });

  it("root has no children", () => {
    const t = new Tree("root");
    expect(t.children("root")).toEqual([]);
  });

  it("root has depth zero", () => {
    const t = new Tree("root");
    expect(t.depth("root")).toBe(0);
  });

  it("single-node tree has height zero", () => {
    const t = new Tree("root");
    expect(t.height()).toBe(0);
  });

  it("root appears in nodes list", () => {
    const t = new Tree("root");
    expect(t.nodes()).toContain("root");
  });

  it("toString shows root and size", () => {
    const t = new Tree("root");
    expect(t.toString()).toBe('Tree(root="root", size=1)');
  });
});

// =========================================================================
// 2. addChild
// =========================================================================

describe("addChild", () => {
  it("adds one child, size increases to 2", () => {
    const t = new Tree("root");
    t.addChild("root", "child");
    expect(t.size()).toBe(2);
  });

  it("child has correct parent", () => {
    const t = new Tree("root");
    t.addChild("root", "child");
    expect(t.parent("child")).toBe("root");
  });

  it("parent has child in children list", () => {
    const t = new Tree("root");
    t.addChild("root", "child");
    expect(t.children("root")).toContain("child");
  });

  it("adds multiple children to same parent", () => {
    const t = new Tree("root");
    t.addChild("root", "A");
    t.addChild("root", "B");
    t.addChild("root", "C");
    expect(t.children("root")).toEqual(["A", "B", "C"]);
  });

  it("adds child to non-root", () => {
    const t = new Tree("root");
    t.addChild("root", "mid");
    t.addChild("mid", "leaf");
    expect(t.parent("leaf")).toBe("mid");
  });

  it("builds deep tree", () => {
    const t = new Tree("level0");
    for (let i = 1; i < 10; i++) {
      t.addChild(`level${i - 1}`, `level${i}`);
    }
    expect(t.size()).toBe(10);
    expect(t.depth("level9")).toBe(9);
  });

  it("throws NodeNotFoundError for nonexistent parent", () => {
    const t = new Tree("root");
    expect(() => t.addChild("nonexistent", "child")).toThrow(
      NodeNotFoundError
    );
    try {
      t.addChild("nonexistent", "child");
    } catch (e) {
      expect((e as NodeNotFoundError).node).toBe("nonexistent");
    }
  });

  it("throws DuplicateNodeError for duplicate child", () => {
    const t = new Tree("root");
    t.addChild("root", "child");
    expect(() => t.addChild("root", "child")).toThrow(DuplicateNodeError);
    try {
      t.addChild("root", "child");
    } catch (e) {
      expect((e as DuplicateNodeError).node).toBe("child");
    }
  });

  it("throws DuplicateNodeError when adding root as child", () => {
    const t = new Tree("root");
    expect(() => t.addChild("root", "root")).toThrow(DuplicateNodeError);
  });

  it("adding child makes parent not a leaf", () => {
    const t = new Tree("root");
    expect(t.isLeaf("root")).toBe(true);
    t.addChild("root", "child");
    expect(t.isLeaf("root")).toBe(false);
  });

  it("new child is a leaf", () => {
    const t = new Tree("root");
    t.addChild("root", "child");
    expect(t.isLeaf("child")).toBe(true);
  });

  it("errors inherit from TreeError", () => {
    const t = new Tree("root");
    expect(() => t.addChild("nonexistent", "child")).toThrow(TreeError);
    t.addChild("root", "child");
    expect(() => t.addChild("root", "child")).toThrow(TreeError);
  });
});

// =========================================================================
// 3. removeSubtree
// =========================================================================

describe("removeSubtree", () => {
  it("removes a leaf node", () => {
    const t = new Tree("root");
    t.addChild("root", "leaf");
    t.removeSubtree("leaf");
    expect(t.size()).toBe(1);
    expect(t.hasNode("leaf")).toBe(false);
  });

  it("removes subtree with descendants", () => {
    const t = makeSampleTree();
    t.removeSubtree("B");
    expect(t.size()).toBe(3);
    expect(t.hasNode("B")).toBe(false);
    expect(t.hasNode("D")).toBe(false);
    expect(t.hasNode("E")).toBe(false);
    expect(t.hasNode("G")).toBe(false);
  });

  it("preserves siblings when removing subtree", () => {
    const t = makeSampleTree();
    t.removeSubtree("B");
    expect(t.hasNode("C")).toBe(true);
    expect(t.hasNode("F")).toBe(true);
    expect(t.children("A")).toEqual(["C"]);
  });

  it("removes deep subtree", () => {
    const t = makeSampleTree();
    t.removeSubtree("D");
    expect(t.size()).toBe(5);
    expect(t.hasNode("D")).toBe(false);
    expect(t.hasNode("G")).toBe(false);
    expect(t.children("B")).toEqual(["E"]);
  });

  it("throws RootRemovalError when removing root", () => {
    const t = new Tree("root");
    expect(() => t.removeSubtree("root")).toThrow(RootRemovalError);
  });

  it("throws NodeNotFoundError for nonexistent node", () => {
    const t = new Tree("root");
    expect(() => t.removeSubtree("nonexistent")).toThrow(NodeNotFoundError);
  });

  it("can re-add after removal", () => {
    const t = new Tree("root");
    t.addChild("root", "child");
    t.removeSubtree("child");
    t.addChild("root", "child");
    expect(t.hasNode("child")).toBe(true);
  });

  it("parent becomes leaf after removing only child", () => {
    const t = new Tree("root");
    t.addChild("root", "only_child");
    t.removeSubtree("only_child");
    expect(t.isLeaf("root")).toBe(true);
  });

  it("RootRemovalError inherits from TreeError", () => {
    const t = new Tree("root");
    expect(() => t.removeSubtree("root")).toThrow(TreeError);
  });
});

// =========================================================================
// 4. Queries
// =========================================================================

describe("Queries", () => {
  // --- parent ---
  it("parent of child", () => {
    expect(makeSampleTree().parent("B")).toBe("A");
  });

  it("parent of grandchild", () => {
    expect(makeSampleTree().parent("G")).toBe("D");
  });

  it("parent of root is null", () => {
    expect(makeSampleTree().parent("A")).toBeNull();
  });

  it("parent of nonexistent throws", () => {
    expect(() => makeSampleTree().parent("Z")).toThrow(NodeNotFoundError);
  });

  // --- children ---
  it("children of root", () => {
    expect(makeSampleTree().children("A")).toEqual(["B", "C"]);
  });

  it("children of internal node", () => {
    expect(makeSampleTree().children("B")).toEqual(["D", "E"]);
  });

  it("children of leaf is empty", () => {
    expect(makeSampleTree().children("G")).toEqual([]);
  });

  it("children of nonexistent throws", () => {
    expect(() => makeSampleTree().children("Z")).toThrow(NodeNotFoundError);
  });

  // --- siblings ---
  it("siblings of node with sibling", () => {
    expect(makeSampleTree().siblings("B")).toEqual(["C"]);
  });

  it("siblings are mutual", () => {
    expect(makeSampleTree().siblings("C")).toEqual(["B"]);
  });

  it("siblings of only child is empty", () => {
    expect(makeSampleTree().siblings("F")).toEqual([]);
  });

  it("siblings of root is empty", () => {
    expect(makeSampleTree().siblings("A")).toEqual([]);
  });

  it("siblings of nonexistent throws", () => {
    expect(() => makeSampleTree().siblings("Z")).toThrow(NodeNotFoundError);
  });

  it("multiple siblings", () => {
    const t = new Tree("root");
    t.addChild("root", "A");
    t.addChild("root", "B");
    t.addChild("root", "C");
    t.addChild("root", "D");
    expect(t.siblings("B")).toEqual(["A", "C", "D"]);
  });

  // --- isLeaf ---
  it("leaf nodes return true", () => {
    const t = makeSampleTree();
    expect(t.isLeaf("G")).toBe(true);
    expect(t.isLeaf("E")).toBe(true);
    expect(t.isLeaf("F")).toBe(true);
  });

  it("internal nodes return false", () => {
    const t = makeSampleTree();
    expect(t.isLeaf("A")).toBe(false);
    expect(t.isLeaf("B")).toBe(false);
  });

  it("isLeaf on nonexistent throws", () => {
    expect(() => makeSampleTree().isLeaf("Z")).toThrow(NodeNotFoundError);
  });

  // --- isRoot ---
  it("root returns true", () => {
    expect(makeSampleTree().isRoot("A")).toBe(true);
  });

  it("non-root returns false", () => {
    expect(makeSampleTree().isRoot("B")).toBe(false);
  });

  it("isRoot on nonexistent throws", () => {
    expect(() => makeSampleTree().isRoot("Z")).toThrow(NodeNotFoundError);
  });

  // --- depth ---
  it("depth of root is 0", () => {
    expect(makeSampleTree().depth("A")).toBe(0);
  });

  it("depth of level one", () => {
    const t = makeSampleTree();
    expect(t.depth("B")).toBe(1);
    expect(t.depth("C")).toBe(1);
  });

  it("depth of level two", () => {
    const t = makeSampleTree();
    expect(t.depth("D")).toBe(2);
    expect(t.depth("E")).toBe(2);
    expect(t.depth("F")).toBe(2);
  });

  it("depth of level three", () => {
    expect(makeSampleTree().depth("G")).toBe(3);
  });

  it("depth of nonexistent throws", () => {
    expect(() => makeSampleTree().depth("Z")).toThrow(NodeNotFoundError);
  });

  // --- height ---
  it("height of sample tree", () => {
    expect(makeSampleTree().height()).toBe(3);
  });

  it("height of single node", () => {
    expect(new Tree("root").height()).toBe(0);
  });

  it("height of flat tree", () => {
    const t = new Tree("root");
    for (let i = 0; i < 5; i++) t.addChild("root", `child${i}`);
    expect(t.height()).toBe(1);
  });

  it("height of deep chain", () => {
    const t = new Tree("0");
    for (let i = 1; i < 20; i++) t.addChild(`${i - 1}`, `${i}`);
    expect(t.height()).toBe(19);
  });

  // --- size ---
  it("size of sample", () => {
    expect(makeSampleTree().size()).toBe(7);
  });

  it("size after add", () => {
    const t = new Tree("root");
    expect(t.size()).toBe(1);
    t.addChild("root", "A");
    expect(t.size()).toBe(2);
  });

  // --- nodes ---
  it("nodes returns all sorted", () => {
    expect(makeSampleTree().nodes()).toEqual([
      "A",
      "B",
      "C",
      "D",
      "E",
      "F",
      "G",
    ]);
  });

  // --- leaves ---
  it("leaves of sample", () => {
    expect(makeSampleTree().leaves()).toEqual(["E", "F", "G"]);
  });

  it("leaves of single node", () => {
    expect(new Tree("root").leaves()).toEqual(["root"]);
  });

  it("leaves of flat tree", () => {
    const t = new Tree("root");
    t.addChild("root", "A");
    t.addChild("root", "B");
    t.addChild("root", "C");
    expect(t.leaves()).toEqual(["A", "B", "C"]);
  });

  // --- hasNode ---
  it("hasNode true", () => {
    expect(makeSampleTree().hasNode("A")).toBe(true);
  });

  it("hasNode false", () => {
    expect(makeSampleTree().hasNode("Z")).toBe(false);
  });
});

// =========================================================================
// 5. Traversals
// =========================================================================

describe("Traversals", () => {
  // --- preorder ---
  it("preorder of sample", () => {
    expect(makeSampleTree().preorder()).toEqual([
      "A",
      "B",
      "D",
      "G",
      "E",
      "C",
      "F",
    ]);
  });

  it("preorder of single node", () => {
    expect(new Tree("root").preorder()).toEqual(["root"]);
  });

  it("preorder of flat tree", () => {
    const t = new Tree("root");
    t.addChild("root", "C");
    t.addChild("root", "A");
    t.addChild("root", "B");
    expect(t.preorder()).toEqual(["root", "A", "B", "C"]);
  });

  it("preorder of deep chain", () => {
    const t = new Tree("A");
    t.addChild("A", "B");
    t.addChild("B", "C");
    expect(t.preorder()).toEqual(["A", "B", "C"]);
  });

  // --- postorder ---
  it("postorder of sample", () => {
    expect(makeSampleTree().postorder()).toEqual([
      "G",
      "D",
      "E",
      "B",
      "F",
      "C",
      "A",
    ]);
  });

  it("postorder of single node", () => {
    expect(new Tree("root").postorder()).toEqual(["root"]);
  });

  it("postorder of flat tree", () => {
    const t = new Tree("root");
    t.addChild("root", "C");
    t.addChild("root", "A");
    t.addChild("root", "B");
    expect(t.postorder()).toEqual(["A", "B", "C", "root"]);
  });

  it("postorder of deep chain", () => {
    const t = new Tree("A");
    t.addChild("A", "B");
    t.addChild("B", "C");
    expect(t.postorder()).toEqual(["C", "B", "A"]);
  });

  // --- levelOrder ---
  it("level order of sample", () => {
    expect(makeSampleTree().levelOrder()).toEqual([
      "A",
      "B",
      "C",
      "D",
      "E",
      "F",
      "G",
    ]);
  });

  it("level order of single node", () => {
    expect(new Tree("root").levelOrder()).toEqual(["root"]);
  });

  it("level order of flat tree", () => {
    const t = new Tree("root");
    t.addChild("root", "C");
    t.addChild("root", "A");
    t.addChild("root", "B");
    expect(t.levelOrder()).toEqual(["root", "A", "B", "C"]);
  });

  it("level order of deep chain", () => {
    const t = new Tree("A");
    t.addChild("A", "B");
    t.addChild("B", "C");
    expect(t.levelOrder()).toEqual(["A", "B", "C"]);
  });

  // --- consistency ---
  it("all traversals have same length", () => {
    const t = makeSampleTree();
    expect(t.preorder().length).toBe(7);
    expect(t.postorder().length).toBe(7);
    expect(t.levelOrder().length).toBe(7);
  });

  it("all traversals have same elements", () => {
    const t = makeSampleTree();
    expect([...t.preorder()].sort()).toEqual([...t.postorder()].sort());
    expect([...t.preorder()].sort()).toEqual([...t.levelOrder()].sort());
  });

  it("preorder root is first", () => {
    expect(makeSampleTree().preorder()[0]).toBe("A");
  });

  it("postorder root is last", () => {
    const po = makeSampleTree().postorder();
    expect(po[po.length - 1]).toBe("A");
  });

  it("level order root is first", () => {
    expect(makeSampleTree().levelOrder()[0]).toBe("A");
  });
});

// =========================================================================
// 6. pathTo
// =========================================================================

describe("pathTo", () => {
  it("path to root", () => {
    expect(makeSampleTree().pathTo("A")).toEqual(["A"]);
  });

  it("path to child", () => {
    expect(makeSampleTree().pathTo("B")).toEqual(["A", "B"]);
  });

  it("path to grandchild", () => {
    expect(makeSampleTree().pathTo("D")).toEqual(["A", "B", "D"]);
  });

  it("path to deep node", () => {
    expect(makeSampleTree().pathTo("G")).toEqual(["A", "B", "D", "G"]);
  });

  it("path to right branch", () => {
    expect(makeSampleTree().pathTo("F")).toEqual(["A", "C", "F"]);
  });

  it("path to nonexistent throws", () => {
    expect(() => makeSampleTree().pathTo("Z")).toThrow(NodeNotFoundError);
  });

  it("path length equals depth plus one", () => {
    const t = makeSampleTree();
    for (const node of t.nodes()) {
      expect(t.pathTo(node).length).toBe(t.depth(node) + 1);
    }
  });
});

// =========================================================================
// 7. lca
// =========================================================================

describe("lca", () => {
  it("lca of same node", () => {
    expect(makeSampleTree().lca("D", "D")).toBe("D");
  });

  it("lca of siblings", () => {
    expect(makeSampleTree().lca("D", "E")).toBe("B");
  });

  it("lca of parent and child", () => {
    expect(makeSampleTree().lca("B", "D")).toBe("B");
  });

  it("lca is symmetric", () => {
    expect(makeSampleTree().lca("D", "B")).toBe("B");
  });

  it("lca of cousins", () => {
    expect(makeSampleTree().lca("D", "F")).toBe("A");
  });

  it("lca of root and leaf", () => {
    expect(makeSampleTree().lca("A", "G")).toBe("A");
  });

  it("lca of deep nodes", () => {
    expect(makeSampleTree().lca("G", "E")).toBe("B");
  });

  it("lca of leaves in different subtrees", () => {
    expect(makeSampleTree().lca("G", "F")).toBe("A");
  });

  it("lca of nonexistent a throws", () => {
    expect(() => makeSampleTree().lca("Z", "A")).toThrow(NodeNotFoundError);
  });

  it("lca of nonexistent b throws", () => {
    expect(() => makeSampleTree().lca("A", "Z")).toThrow(NodeNotFoundError);
  });

  it("lca of root with root", () => {
    expect(makeSampleTree().lca("A", "A")).toBe("A");
  });
});

// =========================================================================
// 8. subtree
// =========================================================================

describe("subtree", () => {
  it("subtree of leaf is single node", () => {
    const sub = makeSampleTree().subtree("G");
    expect(sub.root).toBe("G");
    expect(sub.size()).toBe(1);
  });

  it("subtree of internal node includes descendants", () => {
    const sub = makeSampleTree().subtree("B");
    expect(sub.root).toBe("B");
    expect(sub.size()).toBe(4);
    expect(sub.hasNode("D")).toBe(true);
    expect(sub.hasNode("E")).toBe(true);
    expect(sub.hasNode("G")).toBe(true);
  });

  it("subtree preserves structure", () => {
    const sub = makeSampleTree().subtree("B");
    expect(sub.children("B")).toEqual(["D", "E"]);
    expect(sub.children("D")).toEqual(["G"]);
    expect(sub.isLeaf("G")).toBe(true);
    expect(sub.isLeaf("E")).toBe(true);
  });

  it("subtree of root is the entire tree", () => {
    const t = makeSampleTree();
    const sub = t.subtree("A");
    expect(sub.size()).toBe(t.size());
    expect(sub.nodes()).toEqual(t.nodes());
  });

  it("subtree does not modify original", () => {
    const t = makeSampleTree();
    const origSize = t.size();
    t.subtree("B");
    expect(t.size()).toBe(origSize);
  });

  it("subtree of nonexistent throws", () => {
    expect(() => makeSampleTree().subtree("Z")).toThrow(NodeNotFoundError);
  });

  it("subtree is independent", () => {
    const t = makeSampleTree();
    const sub = t.subtree("B");
    sub.addChild("E", "new_node");
    expect(t.hasNode("new_node")).toBe(false);
  });

  it("subtree of right branch", () => {
    const sub = makeSampleTree().subtree("C");
    expect(sub.root).toBe("C");
    expect(sub.size()).toBe(2);
    expect(sub.children("C")).toEqual(["F"]);
  });
});

// =========================================================================
// 9. toAscii
// =========================================================================

describe("toAscii", () => {
  it("single node", () => {
    expect(new Tree("root").toAscii()).toBe("root");
  });

  it("root with one child", () => {
    const t = new Tree("root");
    t.addChild("root", "child");
    expect(t.toAscii()).toBe("root\n\u2514\u2500\u2500 child");
  });

  it("root with two children", () => {
    const t = new Tree("root");
    t.addChild("root", "A");
    t.addChild("root", "B");
    expect(t.toAscii()).toBe(
      "root\n\u251C\u2500\u2500 A\n\u2514\u2500\u2500 B"
    );
  });

  it("sample tree", () => {
    const expected = [
      "A",
      "\u251C\u2500\u2500 B",
      "\u2502   \u251C\u2500\u2500 D",
      "\u2502   \u2502   \u2514\u2500\u2500 G",
      "\u2502   \u2514\u2500\u2500 E",
      "\u2514\u2500\u2500 C",
      "    \u2514\u2500\u2500 F",
    ].join("\n");
    expect(makeSampleTree().toAscii()).toBe(expected);
  });

  it("deep chain", () => {
    const t = new Tree("A");
    t.addChild("A", "B");
    t.addChild("B", "C");
    expect(t.toAscii()).toBe(
      "A\n\u2514\u2500\u2500 B\n    \u2514\u2500\u2500 C"
    );
  });

  it("wide tree", () => {
    const t = new Tree("root");
    t.addChild("root", "A");
    t.addChild("root", "B");
    t.addChild("root", "C");
    t.addChild("root", "D");
    expect(t.toAscii()).toBe(
      "root\n\u251C\u2500\u2500 A\n\u251C\u2500\u2500 B\n\u251C\u2500\u2500 C\n\u2514\u2500\u2500 D"
    );
  });
});

// =========================================================================
// 10. Edge Cases
// =========================================================================

describe("Edge cases", () => {
  it("single node traversals", () => {
    const t = new Tree("solo");
    expect(t.preorder()).toEqual(["solo"]);
    expect(t.postorder()).toEqual(["solo"]);
    expect(t.levelOrder()).toEqual(["solo"]);
  });

  it("single node leaves", () => {
    expect(new Tree("solo").leaves()).toEqual(["solo"]);
  });

  it("deep chain height", () => {
    const t = new Tree("n0");
    for (let i = 1; i < 100; i++) t.addChild(`n${i - 1}`, `n${i}`);
    expect(t.height()).toBe(99);
    expect(t.size()).toBe(100);
  });

  it("wide tree height", () => {
    const t = new Tree("root");
    for (let i = 0; i < 100; i++) t.addChild("root", `child${i}`);
    expect(t.height()).toBe(1);
    expect(t.size()).toBe(101);
  });

  it("balanced binary tree", () => {
    const t = new Tree("1");
    t.addChild("1", "2");
    t.addChild("1", "3");
    t.addChild("2", "4");
    t.addChild("2", "5");
    t.addChild("3", "6");
    t.addChild("3", "7");
    expect(t.size()).toBe(7);
    expect(t.height()).toBe(2);
    expect(t.leaves()).toEqual(["4", "5", "6", "7"]);
  });

  it("node names with spaces", () => {
    const t = new Tree("my root");
    t.addChild("my root", "my child");
    expect(t.parent("my child")).toBe("my root");
  });

  it("node names with special chars", () => {
    const t = new Tree("root:main");
    t.addChild("root:main", "child.1");
    expect(t.hasNode("child.1")).toBe(true);
  });

  it("path to single node", () => {
    expect(new Tree("solo").pathTo("solo")).toEqual(["solo"]);
  });

  it("lca in single node tree", () => {
    expect(new Tree("solo").lca("solo", "solo")).toBe("solo");
  });

  it("subtree of single node", () => {
    const sub = new Tree("solo").subtree("solo");
    expect(sub.root).toBe("solo");
    expect(sub.size()).toBe(1);
  });

  it("remove and rebuild", () => {
    const t = new Tree("root");
    t.addChild("root", "A");
    t.addChild("A", "B");
    t.removeSubtree("A");
    t.addChild("root", "A");
    t.addChild("A", "C");
    expect(t.children("A")).toEqual(["C"]);
    expect(t.hasNode("B")).toBe(false);
  });
});

// =========================================================================
// 11. graph property
// =========================================================================

describe("graph property", () => {
  it("graph is a Graph instance", () => {
    expect(makeSampleTree().graph).toBeInstanceOf(Graph);
  });

  it("graph has correct nodes", () => {
    const nodes = new Set(makeSampleTree().graph.nodes());
    expect(nodes).toEqual(new Set(["A", "B", "C", "D", "E", "F", "G"]));
  });

  it("graph has correct edges", () => {
    const edges = makeSampleTree().graph.edges();
    const edgeSet = new Set(edges.map(([a, b]) => `${a}->${b}`));
    expect(edgeSet.has("A->B")).toBe(true);
    expect(edgeSet.has("A->C")).toBe(true);
    expect(edgeSet.has("B->D")).toBe(true);
    expect(edgeSet.has("B->E")).toBe(true);
    expect(edgeSet.has("C->F")).toBe(true);
    expect(edgeSet.has("D->G")).toBe(true);
  });

  it("graph edge count is N-1", () => {
    expect(makeSampleTree().graph.edges().length).toBe(6);
  });

  it("graph has no cycles", () => {
    expect(makeSampleTree().graph.hasCycle()).toBe(false);
  });

  it("graph topological sort starts with root", () => {
    expect(makeSampleTree().graph.topologicalSort()[0]).toBe("A");
  });
});
