import { describe, expect, it } from "vitest";
import { RadixTree } from "../src/index.js";

function treeWith(keys: string[]): RadixTree<number> {
  const tree = new RadixTree<number>();
  keys.forEach((key, index) => tree.insert(key, index + 1));
  return tree;
}

describe("RadixTree", () => {
  it("inserts and searches split cases", () => {
    const tree = new RadixTree<number>();
    tree.insert("application", 1);
    tree.insert("apple", 2);
    tree.insert("app", 3);
    tree.insert("apt", 4);
    expect(tree.search("application")).toBe(1);
    expect(tree.get("apple")).toBe(2);
    expect(tree.search("app")).toBe(3);
    expect(tree.search("apt")).toBe(4);
    expect(tree.search("appl")).toBeUndefined();
    expect(tree.size).toBe(4);
  });

  it("updates duplicate keys without growing", () => {
    const tree = new RadixTree<number>();
    tree.put("foo", 1);
    tree.put("foo", 99);
    expect(tree.search("foo")).toBe(99);
    expect(tree.size).toBe(1);
    expect(tree.containsKey("foo")).toBe(true);
  });

  it("deletes and merges compressed edges", () => {
    const tree = treeWith(["app", "apple"]);
    expect(tree.nodeCount()).toBe(3);
    expect(tree.delete("app")).toBe(true);
    expect(tree.search("app")).toBeUndefined();
    expect(tree.search("apple")).toBe(2);
    expect(tree.nodeCount()).toBe(2);
    expect(tree.delete("missing")).toBe(false);
  });

  it("handles prefix queries and sorted keys", () => {
    const tree = treeWith(["search", "searcher", "searching", "banana"]);
    expect(tree.startsWith("sear")).toBe(true);
    expect(tree.startsWith("seek")).toBe(false);
    expect(tree.wordsWithPrefix("search")).toEqual(["search", "searcher", "searching"]);
    expect(tree.keys()).toEqual(["banana", "search", "searcher", "searching"]);
  });

  it("finds longest prefix matches", () => {
    const tree = treeWith(["a", "ab", "abc", "application"]);
    expect(tree.longestPrefixMatch("abcdef")).toBe("abc");
    expect(tree.longestPrefixMatch("application/json")).toBe("application");
    expect(tree.longestPrefixMatch("xyz")).toBeUndefined();
  });

  it("supports empty string keys", () => {
    const tree = new RadixTree<number>();
    expect(tree.startsWith("")).toBe(false);
    tree.insert("", 1);
    tree.insert("a", 2);
    expect(tree.search("")).toBe(1);
    expect(tree.longestPrefixMatch("xyz")).toBe("");
    expect(tree.delete("")).toBe(true);
    expect(tree.search("")).toBeUndefined();
  });

  it("exports maps, values, and string summaries", () => {
    const tree = treeWith(["foo", "bar", "baz"]);
    expect(Object.fromEntries(tree.toMap())).toEqual({ bar: 2, baz: 3, foo: 1 });
    expect(tree.values()).toEqual([2, 3, 1]);
    expect(String(tree)).toContain("3 keys");
    expect(tree.isEmpty()).toBe(false);
  });
});
