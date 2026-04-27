import { describe, expect, it } from "vitest";
import { BSTNode, BinarySearchTree } from "../src/index";

function populated(): BinarySearchTree<number> {
  let tree = BinarySearchTree.empty<number>();
  for (const value of [5, 1, 8, 3, 7]) {
    tree = tree.insert(value);
  }
  return tree;
}

describe("BinarySearchTree", () => {
  it("inserts, searches, ranks, and deletes", () => {
    const tree = populated();

    expect(tree.toSortedArray()).toEqual([1, 3, 5, 7, 8]);
    expect(tree.size()).toBe(5);
    expect(tree.contains(7)).toBe(true);
    expect(tree.search(7)?.value).toBe(7);
    expect(tree.minValue()).toBe(1);
    expect(tree.maxValue()).toBe(8);
    expect(tree.predecessor(5)).toBe(3);
    expect(tree.successor(5)).toBe(7);
    expect(tree.rank(4)).toBe(2);
    expect(tree.kthSmallest(4)).toBe(7);

    const deleted = tree.delete(5);
    expect(deleted.contains(5)).toBe(false);
    expect(deleted.isValid()).toBe(true);
    expect(tree.contains(5)).toBe(true);
  });

  it("builds balanced trees from sorted arrays", () => {
    const tree = BinarySearchTree.fromSortedArray([1, 2, 3, 4, 5, 6, 7]);

    expect(tree.toSortedArray()).toEqual([1, 2, 3, 4, 5, 6, 7]);
    expect(tree.height()).toBe(2);
    expect(tree.size()).toBe(7);
    expect(tree.isValid()).toBe(true);
  });

  it("handles empty trees and edge queries", () => {
    const tree = BinarySearchTree.empty<number>();

    expect(tree.search(1)).toBeNull();
    expect(tree.minValue()).toBeNull();
    expect(tree.maxValue()).toBeNull();
    expect(tree.predecessor(1)).toBeNull();
    expect(tree.successor(1)).toBeNull();
    expect(tree.kthSmallest(0)).toBeNull();
    expect(tree.kthSmallest(1)).toBeNull();
    expect(tree.rank(1)).toBe(0);
    expect(tree.height()).toBe(-1);
    expect(tree.size()).toBe(0);
    expect(tree.toString()).toBe("BinarySearchTree(root=null, size=0)");
  });

  it("ignores duplicates and deletes one-child nodes", () => {
    const tree = BinarySearchTree.fromSortedArray([2, 4, 6, 8]);

    expect(tree.root?.value).toBe(6);
    expect(tree.insert(4).toSortedArray()).toEqual(tree.toSortedArray());
    expect(tree.delete(2).toSortedArray()).toEqual([4, 6, 8]);
  });

  it("validates ordering and size metadata", () => {
    const badOrder = new BinarySearchTree(new BSTNode(5, new BSTNode(6)));
    const badSize = new BinarySearchTree(new BSTNode(5, new BSTNode(3), null, 99));

    expect(badOrder.isValid()).toBe(false);
    expect(badSize.isValid()).toBe(false);
  });

  it("supports custom comparators", () => {
    const byLength = (left: string, right: string) => left.length - right.length;
    let tree = BinarySearchTree.empty(byLength);
    tree = tree.insert("bbb").insert("a").insert("cc");

    expect(tree.toSortedArray()).toEqual(["a", "cc", "bbb"]);
    expect(tree.contains("zz")).toBe(true);
  });
});
