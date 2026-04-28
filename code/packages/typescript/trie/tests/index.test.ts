import { describe, expect, it } from "vitest";
import { Trie } from "../src/index.js";

function makeTrie(words: string[]): Trie<boolean> {
  const trie = new Trie<boolean>();
  for (const word of words) {
    trie.insert(word, true);
  }
  return trie;
}

describe("Trie", () => {
  it("starts empty", () => {
    const trie = new Trie<number>();
    expect(trie.size).toBe(0);
    expect(trie.isEmpty()).toBe(true);
    expect(trie.search("anything")).toBeUndefined();
    expect(trie.startsWith("a")).toBe(false);
    expect(trie.isValid()).toBe(true);
  });

  it("inserts, searches, and updates exact keys", () => {
    const trie = new Trie<number>();
    trie.insert("hello", 42);
    expect(trie.search("hello")).toBe(42);
    expect(trie.search("hell")).toBeUndefined();
    expect(trie.search("hellos")).toBeUndefined();

    trie.insert("hello", 99);
    expect(trie.search("hello")).toBe(99);
    expect(trie.size).toBe(1);
    expect(trie.containsKey("hello")).toBe(true);
  });

  it("returns lexicographic words for prefixes and all keys", () => {
    const trie = makeTrie(["banana", "app", "apple", "apply", "apt"]);
    expect(trie.wordsWithPrefix("app").map(([key]) => key)).toEqual(["app", "apple", "apply"]);
    expect(trie.wordsWithPrefix("xyz")).toEqual([]);
    expect(trie.keys()).toEqual(["app", "apple", "apply", "apt", "banana"]);
    expect(trie.entries().length).toBe(5);
  });

  it("deletes leaf and shared-prefix keys without disturbing siblings", () => {
    const trie = makeTrie(["app", "apple", "apt"]);
    expect(trie.delete("app")).toBe(true);
    expect(trie.containsKey("app")).toBe(false);
    expect(trie.containsKey("apple")).toBe(true);
    expect(trie.containsKey("apt")).toBe(true);
    expect(trie.size).toBe(2);
    expect(trie.delete("missing")).toBe(false);
    expect(trie.delete("ap")).toBe(false);
    expect(trie.delete("apple")).toBe(true);
    expect(trie.delete("apt")).toBe(true);
    expect(trie.isEmpty()).toBe(true);
    expect(trie.isValid()).toBe(true);
  });

  it("finds the longest stored prefix", () => {
    const trie = Trie.fromEntries([
      ["a", 1],
      ["ab", 2],
      ["abc", 3],
      ["abcd", 4],
    ]);

    expect(trie.longestPrefixMatch("abcde")).toEqual(["abcd", 4]);
    expect(trie.longestPrefixMatch("xyz")).toBeUndefined();
    expect(trie.longestPrefixMatch("a")).toEqual(["a", 1]);
  });

  it("supports unicode and empty string keys", () => {
    const trie = new Trie<string>();
    trie.insert("", "root");
    trie.insert("cafe", "plain");
    trie.insert("cafe\u0301", "accent-combining");
    trie.insert("caf\u00e9", "accent-single");

    expect(trie.search("")).toBe("root");
    expect(trie.startsWith("")).toBe(true);
    expect(trie.startsWith("caf")).toBe(true);
    expect(trie.search("caf\u00e9")).toBe("accent-single");
    expect(trie.longestPrefixMatch("cafe\u0301-au-lait")).toEqual([
      "cafe\u0301",
      "accent-combining",
    ]);
  });

  it("handles empty-string deletion and string rendering", () => {
    const trie = new Trie<string>([
      ["", "root"],
      ["a", "letter"],
    ]);
    expect(trie.delete("")).toBe(true);
    expect(trie.search("")).toBeUndefined();
    expect(trie.search("a")).toBe("letter");
    expect(String(trie)).toContain("1 keys");
  });
});
