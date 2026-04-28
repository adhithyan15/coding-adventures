import { describe, expect, it } from "vitest";
import {
  BinaryTree,
  BinaryTreeNode,
  find,
  height,
  isComplete,
  isFull,
  isPerfect,
  size,
} from "../src/index";

describe("BinaryTree", () => {
  it("round trips level-order input", () => {
    const tree = BinaryTree.fromLevelOrder([1, 2, 3, 4, 5, 6, 7]);

    expect(tree.root?.value).toBe(1);
    expect(tree.toArray()).toEqual([1, 2, 3, 4, 5, 6, 7]);
    expect(tree.levelOrder()).toEqual([1, 2, 3, 4, 5, 6, 7]);
  });

  it("answers shape queries", () => {
    const tree = BinaryTree.fromLevelOrder([1, 2, null]);

    expect(tree.isFull()).toBe(false);
    expect(tree.isComplete()).toBe(true);
    expect(tree.isPerfect()).toBe(false);
    expect(tree.height()).toBe(1);
    expect(tree.size()).toBe(2);
    expect(tree.leftChild(1)?.value).toBe(2);
    expect(tree.rightChild(1)).toBeNull();
    expect(tree.find(999)).toBeNull();
  });

  it("traverses sparse trees", () => {
    const tree = BinaryTree.fromLevelOrder([1, 2, 3, 4, null, 5, null]);

    expect(tree.preorder()).toEqual([1, 2, 4, 3, 5]);
    expect(tree.inorder()).toEqual([4, 2, 1, 5, 3]);
    expect(tree.postorder()).toEqual([4, 2, 5, 3, 1]);
    expect(tree.levelOrder()).toEqual([1, 2, 3, 4, 5]);
    expect(tree.toArray()).toEqual([1, 2, 3, 4, null, 5, null]);
  });

  it("recognizes perfect full trees", () => {
    const tree = BinaryTree.fromLevelOrder(["A", "B", "C", "D", "E", "F", "G"]);

    expect(tree.isFull()).toBe(true);
    expect(tree.isComplete()).toBe(true);
    expect(tree.isPerfect()).toBe(true);
    expect(tree.leftChild("A")?.value).toBe("B");
    expect(tree.rightChild("A")?.value).toBe("C");
  });

  it("handles empty trees", () => {
    const tree = new BinaryTree<number>();

    expect(tree.root).toBeNull();
    expect(tree.isFull()).toBe(true);
    expect(tree.isComplete()).toBe(true);
    expect(tree.isPerfect()).toBe(true);
    expect(tree.height()).toBe(-1);
    expect(tree.size()).toBe(0);
    expect(tree.toArray()).toEqual([]);
    expect(tree.toAscii()).toBe("");
    expect(tree.levelOrder()).toEqual([]);
    expect(tree.toString()).toBe("BinaryTree(root=null, size=0)");
  });

  it("supports explicit roots and ASCII rendering", () => {
    const root = new BinaryTreeNode(
      "root",
      new BinaryTreeNode("left"),
      new BinaryTreeNode("right"),
    );
    const tree = BinaryTree.withRoot(root);

    const ascii = tree.toAscii();
    expect(ascii).toContain("root");
    expect(ascii).toContain("left");
    expect(ascii).toContain("right");
    expect(tree.toString()).toBe("BinaryTree(root=root, size=3)");
  });

  it("wraps singleton values", () => {
    const tree = BinaryTree.singleton("root");

    expect(tree.root?.value).toBe("root");
    expect(tree.size()).toBe(1);
  });
});

describe("free functions", () => {
  it("operate on nodes directly", () => {
    const root = new BinaryTreeNode(1, new BinaryTreeNode(2));

    expect(find(root, 2)?.value).toBe(2);
    expect(find(root, 3)).toBeNull();
    expect(isFull(root)).toBe(false);
    expect(isComplete(root)).toBe(true);
    expect(isPerfect(root)).toBe(false);
    expect(height(root)).toBe(1);
    expect(size(root)).toBe(2);
  });
});
