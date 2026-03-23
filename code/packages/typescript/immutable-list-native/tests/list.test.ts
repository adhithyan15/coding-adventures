// list.test.ts -- Comprehensive tests for the native ImmutableList addon
// ======================================================================
//
// These tests verify that the Rust ImmutableList implementation is correctly
// exposed to JavaScript via the N-API node-bridge. Every public method
// is tested, including edge cases like empty lists, large lists, structural
// sharing verification, and error conditions.

import { describe, it, expect } from "vitest";
import { ImmutableList } from "../index.js";

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

describe("ImmutableList construction", () => {
  it("creates an empty list with new ImmutableList()", () => {
    const list = new ImmutableList();
    expect(list.length()).toBe(0);
    expect(list.isEmpty()).toBe(true);
    expect(list.toArray()).toEqual([]);
  });

  it("creates a list from an array of strings", () => {
    const list = new ImmutableList(["a", "b", "c"]);
    expect(list.length()).toBe(3);
    expect(list.isEmpty()).toBe(false);
    expect(list.toArray()).toEqual(["a", "b", "c"]);
  });

  it("creates from an empty array", () => {
    const list = new ImmutableList([]);
    expect(list.length()).toBe(0);
    expect(list.isEmpty()).toBe(true);
  });

  it("creates from a single-element array", () => {
    const list = new ImmutableList(["only"]);
    expect(list.length()).toBe(1);
    expect(list.get(0)).toBe("only");
  });

  it("throws on non-string array elements", () => {
    // @ts-expect-error -- intentionally passing wrong type
    expect(() => new ImmutableList([1, 2, 3])).toThrow();
  });

  it("throws on non-array argument", () => {
    // @ts-expect-error -- intentionally passing wrong type
    expect(() => new ImmutableList("hello")).toThrow();
  });
});

// ---------------------------------------------------------------------------
// push()
// ---------------------------------------------------------------------------

describe("ImmutableList.push()", () => {
  it("pushes to an empty list", () => {
    const empty = new ImmutableList();
    const one = empty.push("hello");
    expect(one.length()).toBe(1);
    expect(one.get(0)).toBe("hello");
  });

  it("preserves the original list (immutability)", () => {
    const list1 = new ImmutableList(["a"]);
    const list2 = list1.push("b");
    // Original is unchanged.
    expect(list1.length()).toBe(1);
    expect(list1.get(0)).toBe("a");
    // New list has both elements.
    expect(list2.length()).toBe(2);
    expect(list2.get(0)).toBe("a");
    expect(list2.get(1)).toBe("b");
  });

  it("chains multiple pushes", () => {
    let list = new ImmutableList();
    for (let i = 0; i < 10; i++) {
      list = list.push(`item-${i}`);
    }
    expect(list.length()).toBe(10);
    expect(list.get(0)).toBe("item-0");
    expect(list.get(9)).toBe("item-9");
  });

  it("handles pushing past the 32-element tail buffer", () => {
    // The Rust implementation uses a 32-element tail buffer. Pushing
    // past 32 elements triggers trie promotion. This test ensures that
    // the trie mechanics work correctly through the FFI boundary.
    let list = new ImmutableList();
    for (let i = 0; i < 100; i++) {
      list = list.push(`val-${i}`);
    }
    expect(list.length()).toBe(100);
    expect(list.get(0)).toBe("val-0");
    expect(list.get(31)).toBe("val-31");
    expect(list.get(32)).toBe("val-32");
    expect(list.get(99)).toBe("val-99");
  });

  it("throws when called with no argument", () => {
    const list = new ImmutableList();
    // @ts-expect-error -- intentionally calling with no args
    expect(() => list.push()).toThrow();
  });

  it("throws when called with a non-string argument", () => {
    const list = new ImmutableList();
    // @ts-expect-error -- intentionally passing wrong type
    expect(() => list.push(42)).toThrow();
  });
});

// ---------------------------------------------------------------------------
// get()
// ---------------------------------------------------------------------------

describe("ImmutableList.get()", () => {
  it("retrieves elements by index", () => {
    const list = new ImmutableList(["a", "b", "c"]);
    expect(list.get(0)).toBe("a");
    expect(list.get(1)).toBe("b");
    expect(list.get(2)).toBe("c");
  });

  it("returns undefined for out-of-bounds index", () => {
    const list = new ImmutableList(["a"]);
    expect(list.get(1)).toBeUndefined();
    expect(list.get(100)).toBeUndefined();
  });

  it("returns undefined for negative index (interpreted as large usize)", () => {
    const list = new ImmutableList(["a"]);
    // Negative numbers become very large unsigned values in Rust,
    // which are out of bounds.
    expect(list.get(-1)).toBeUndefined();
  });

  it("returns undefined when getting from an empty list", () => {
    const empty = new ImmutableList();
    expect(empty.get(0)).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// set()
// ---------------------------------------------------------------------------

describe("ImmutableList.set()", () => {
  it("replaces an element and returns a new list", () => {
    const list = new ImmutableList(["a", "b", "c"]);
    const updated = list.set(1, "B");
    // New list has the updated value.
    expect(updated.get(1)).toBe("B");
    // Original is unchanged.
    expect(list.get(1)).toBe("b");
  });

  it("preserves all other elements", () => {
    const list = new ImmutableList(["a", "b", "c", "d"]);
    const updated = list.set(2, "C");
    expect(updated.toArray()).toEqual(["a", "b", "C", "d"]);
    expect(list.toArray()).toEqual(["a", "b", "c", "d"]);
  });

  it("can set the first element", () => {
    const list = new ImmutableList(["x", "y"]);
    const updated = list.set(0, "X");
    expect(updated.get(0)).toBe("X");
    expect(updated.get(1)).toBe("y");
  });

  it("can set the last element", () => {
    const list = new ImmutableList(["x", "y"]);
    const updated = list.set(1, "Y");
    expect(updated.get(0)).toBe("x");
    expect(updated.get(1)).toBe("Y");
  });

  it("throws on out-of-bounds index", () => {
    const list = new ImmutableList(["a"]);
    expect(() => list.set(5, "z")).toThrow();
  });

  it("throws on empty list", () => {
    const list = new ImmutableList();
    expect(() => list.set(0, "z")).toThrow();
  });

  it("throws when missing arguments", () => {
    const list = new ImmutableList(["a"]);
    // @ts-expect-error -- intentionally passing wrong args
    expect(() => list.set(0)).toThrow();
  });
});

// ---------------------------------------------------------------------------
// pop()
// ---------------------------------------------------------------------------

describe("ImmutableList.pop()", () => {
  it("removes the last element and returns [newList, removedValue]", () => {
    const list = new ImmutableList(["a", "b", "c"]);
    const [shorter, removed] = list.pop();
    expect(removed).toBe("c");
    expect(shorter.length()).toBe(2);
    expect(shorter.toArray()).toEqual(["a", "b"]);
  });

  it("preserves the original list (immutability)", () => {
    const list = new ImmutableList(["a", "b"]);
    const [_, removed] = list.pop();
    expect(removed).toBe("b");
    // Original unchanged.
    expect(list.length()).toBe(2);
    expect(list.get(1)).toBe("b");
  });

  it("pops down to empty", () => {
    const list = new ImmutableList(["only"]);
    const [empty, removed] = list.pop();
    expect(removed).toBe("only");
    expect(empty.length()).toBe(0);
    expect(empty.isEmpty()).toBe(true);
  });

  it("throws when popping from an empty list", () => {
    const empty = new ImmutableList();
    expect(() => empty.pop()).toThrow();
  });

  it("chains multiple pops", () => {
    const list = new ImmutableList(["a", "b", "c", "d"]);
    const [l1, v1] = list.pop();
    const [l2, v2] = l1.pop();
    const [l3, v3] = l2.pop();
    const [l4, v4] = l3.pop();
    expect(v1).toBe("d");
    expect(v2).toBe("c");
    expect(v3).toBe("b");
    expect(v4).toBe("a");
    expect(l4.isEmpty()).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// length() and isEmpty()
// ---------------------------------------------------------------------------

describe("ImmutableList.length() and isEmpty()", () => {
  it("empty list has length 0 and isEmpty true", () => {
    const list = new ImmutableList();
    expect(list.length()).toBe(0);
    expect(list.isEmpty()).toBe(true);
  });

  it("non-empty list reports correct length", () => {
    const list = new ImmutableList(["a", "b", "c"]);
    expect(list.length()).toBe(3);
    expect(list.isEmpty()).toBe(false);
  });

  it("length updates correctly after push", () => {
    const list = new ImmutableList().push("x").push("y");
    expect(list.length()).toBe(2);
  });

  it("length updates correctly after pop", () => {
    const list = new ImmutableList(["a", "b", "c"]);
    const [popped] = list.pop();
    expect(popped.length()).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// toArray()
// ---------------------------------------------------------------------------

describe("ImmutableList.toArray()", () => {
  it("returns an empty array for an empty list", () => {
    expect(new ImmutableList().toArray()).toEqual([]);
  });

  it("returns all elements in order", () => {
    const list = new ImmutableList(["x", "y", "z"]);
    expect(list.toArray()).toEqual(["x", "y", "z"]);
  });

  it("returns a fresh array (not a reference to internals)", () => {
    const list = new ImmutableList(["a", "b"]);
    const arr1 = list.toArray();
    const arr2 = list.toArray();
    expect(arr1).toEqual(arr2);
    // Mutating the returned array should not affect the list.
    arr1.push("c");
    expect(list.toArray()).toEqual(["a", "b"]);
  });
});

// ---------------------------------------------------------------------------
// Structural sharing / persistence
// ---------------------------------------------------------------------------

describe("structural sharing and persistence", () => {
  it("multiple versions of a list coexist independently", () => {
    const v0 = new ImmutableList();
    const v1 = v0.push("a");
    const v2 = v1.push("b");
    const v3 = v2.push("c");
    const v4 = v2.set(0, "A"); // branch from v2

    expect(v0.toArray()).toEqual([]);
    expect(v1.toArray()).toEqual(["a"]);
    expect(v2.toArray()).toEqual(["a", "b"]);
    expect(v3.toArray()).toEqual(["a", "b", "c"]);
    expect(v4.toArray()).toEqual(["A", "b"]); // branched version
  });

  it("pop does not affect the original", () => {
    const original = new ImmutableList(["a", "b", "c"]);
    const [popped] = original.pop();
    expect(original.toArray()).toEqual(["a", "b", "c"]);
    expect(popped.toArray()).toEqual(["a", "b"]);
  });

  it("set does not affect the original", () => {
    const original = new ImmutableList(["a", "b", "c"]);
    const modified = original.set(1, "B");
    expect(original.toArray()).toEqual(["a", "b", "c"]);
    expect(modified.toArray()).toEqual(["a", "B", "c"]);
  });
});

// ---------------------------------------------------------------------------
// Large list stress test
// ---------------------------------------------------------------------------

describe("large list operations", () => {
  it("handles 1000 elements with push, get, and toArray", () => {
    let list = new ImmutableList();
    for (let i = 0; i < 1000; i++) {
      list = list.push(`item-${i}`);
    }
    expect(list.length()).toBe(1000);
    expect(list.get(0)).toBe("item-0");
    expect(list.get(500)).toBe("item-500");
    expect(list.get(999)).toBe("item-999");

    const arr = list.toArray();
    expect(arr.length).toBe(1000);
    expect(arr[0]).toBe("item-0");
    expect(arr[999]).toBe("item-999");
  });

  it("handles set on a large list", () => {
    let list = new ImmutableList();
    for (let i = 0; i < 100; i++) {
      list = list.push(`v${i}`);
    }
    const updated = list.set(50, "REPLACED");
    expect(updated.get(50)).toBe("REPLACED");
    expect(list.get(50)).toBe("v50"); // original unchanged
  });

  it("handles pop on a large list", () => {
    let list = new ImmutableList();
    for (let i = 0; i < 100; i++) {
      list = list.push(`v${i}`);
    }
    // Pop 50 elements.
    for (let i = 0; i < 50; i++) {
      const [next, val] = list.pop();
      expect(val).toBe(`v${99 - i}`);
      list = next;
    }
    expect(list.length()).toBe(50);
    expect(list.get(0)).toBe("v0");
    expect(list.get(49)).toBe("v49");
  });
});

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

describe("edge cases", () => {
  it("handles empty strings as elements", () => {
    const list = new ImmutableList([""]);
    expect(list.get(0)).toBe("");
    expect(list.length()).toBe(1);
  });

  it("handles strings with special characters", () => {
    const specials = ["hello\nworld", "tab\there", "null\0byte", "unicode: \u2603"];
    const list = new ImmutableList(specials);
    expect(list.get(0)).toBe("hello\nworld");
    expect(list.get(1)).toBe("tab\there");
    // Note: null byte may be truncated by C string handling
    expect(list.get(3)).toBe("unicode: \u2603");
  });

  it("handles very long strings", () => {
    const longStr = "x".repeat(10000);
    const list = new ImmutableList().push(longStr);
    expect(list.get(0)).toBe(longStr);
  });

  it("push and pop are inverses", () => {
    const list = new ImmutableList(["a", "b"]);
    const pushed = list.push("c");
    const [popped, val] = pushed.pop();
    expect(val).toBe("c");
    expect(popped.toArray()).toEqual(list.toArray());
  });

  it("fromArray and toArray roundtrip", () => {
    const input = ["alpha", "beta", "gamma", "delta"];
    const list = new ImmutableList(input);
    expect(list.toArray()).toEqual(input);
  });
});
