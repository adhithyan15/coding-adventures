import { describe, expect, it } from "vitest";
import { AVLNode, AVLTree } from "../src/index";

describe("AVLTree", () => {
  it("rebalances left and right rotations", () => {
    const rightHeavy = AVLTree.fromValues([10, 20, 30]);
    const leftHeavy = AVLTree.fromValues([30, 20, 10]);

    expect(rightHeavy.root?.value).toBe(20);
    expect(leftHeavy.root?.value).toBe(20);
    expect(rightHeavy.toSortedArray()).toEqual([10, 20, 30]);
    expect(rightHeavy.isValidBst()).toBe(true);
    expect(rightHeavy.isValidAvl()).toBe(true);
    expect(rightHeavy.height()).toBe(1);
    expect(rightHeavy.size()).toBe(3);
  });

  it("searches and computes order statistics", () => {
    const tree = AVLTree.fromValues([40, 20, 60, 10, 30, 50, 70]);

    expect(tree.search(20)?.value).toBe(20);
    expect(tree.contains(50)).toBe(true);
    expect(tree.minValue()).toBe(10);
    expect(tree.maxValue()).toBe(70);
    expect(tree.predecessor(40)).toBe(30);
    expect(tree.successor(40)).toBe(50);
    expect(tree.kthSmallest(4)).toBe(40);
    expect(tree.rank(35)).toBe(3);

    const deleted = tree.delete(20);
    expect(deleted.contains(20)).toBe(false);
    expect(deleted.isValidAvl()).toBe(true);
    expect(tree.contains(20)).toBe(true);
  });

  it("handles empty trees and duplicates", () => {
    const empty = AVLTree.empty<number>();
    expect(empty.search(1)).toBeNull();
    expect(empty.minValue()).toBeNull();
    expect(empty.maxValue()).toBeNull();
    expect(empty.predecessor(1)).toBeNull();
    expect(empty.successor(1)).toBeNull();
    expect(empty.kthSmallest(0)).toBeNull();
    expect(empty.rank(1)).toBe(0);
    expect(empty.balanceFactor(null)).toBe(0);

    const tree = AVLTree.fromValues([30, 20, 40, 10, 25, 35, 50]);
    expect(tree.insert(25).toSortedArray()).toEqual(tree.toSortedArray());
    expect(tree.delete(999).toSortedArray()).toEqual(tree.toSortedArray());
  });

  it("handles double rotations and validation failures", () => {
    expect(AVLTree.fromValues([30, 10, 20]).root?.value).toBe(20);
    expect(AVLTree.fromValues([10, 30, 20]).root?.value).toBe(20);

    const badOrder = new AVLTree(new AVLNode(5, new AVLNode(6), null, 1, 2));
    const badHeight = new AVLTree(new AVLNode(5, new AVLNode(3), null, 99, 2));
    expect(badOrder.isValidBst()).toBe(false);
    expect(badOrder.isValidAvl()).toBe(false);
    expect(badHeight.isValidAvl()).toBe(false);
  });
});
