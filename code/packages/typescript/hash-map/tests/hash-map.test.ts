import { describe, expect, it } from "vitest";
import { HashMap } from "../src/index.js";

describe("HashMap", () => {
  it("creates, reads, updates, and deletes immutably", () => {
    const empty = new HashMap<string, number>();
    const filled = empty.set("a", 1).set("b", 2);

    expect(empty.size).toBe(0);
    expect(filled.size).toBe(2);
    expect(filled.get("a")).toBe(1);
    expect(filled.has("b")).toBe(true);

    const updated = filled.set("a", 3);
    expect(updated.get("a")).toBe(3);
    expect(filled.get("a")).toBe(1);

    const removed = updated.delete("a");
    expect(removed.has("a")).toBe(false);
    expect(updated.has("a")).toBe(true);
  });

  it("enumerates keys, values, and entries", () => {
    const map = HashMap.fromEntries([
      ["x", 1],
      ["y", 2],
    ] as const);

    expect(map.keys()).toEqual(["x", "y"]);
    expect(map.values()).toEqual([1, 2]);
    expect(map.entries()).toEqual([
      ["x", 1],
      ["y", 2],
    ]);
  });
});
