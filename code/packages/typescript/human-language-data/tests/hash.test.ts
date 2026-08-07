import { describe, expect, it } from "vitest";
import { combineLessonHashes, fnv1a64 } from "../src/hash.js";

describe("canonical source fingerprints", () => {
  it("matches the published FNV-1a 64-bit test vectors", () => {
    expect(fnv1a64("")).toBe("fnv1a64:cbf29ce484222325");
    expect(fnv1a64("hello")).toBe("fnv1a64:a430d84680aabd0b");
  });

  it("combines lesson hashes by authored sequence, independent of discovery order", () => {
    const forward = [
      { id: "A", sequence: 10, sourceHash: fnv1a64("a") },
      { id: "B", sequence: 20, sourceHash: fnv1a64("b") },
    ];
    expect(combineLessonHashes([...forward].reverse())).toBe(combineLessonHashes(forward));
    expect(combineLessonHashes(forward)).not.toBe(
      combineLessonHashes([{ ...forward[1]!, sequence: 5 }, forward[0]!]),
    );
  });
});
