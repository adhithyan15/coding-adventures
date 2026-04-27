import { describe, it, expect } from "vitest";
import { HuffmanTree } from "../src/huffman-tree.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Build a code table from a tree and return it as a plain object for easy
 * comparison in assertions.
 */
function tableToObj(map: Map<number, string>): Record<number, string> {
  const obj: Record<number, string> = {};
  for (const [k, v] of map) {
    obj[k] = v;
  }
  return obj;
}

// ---------------------------------------------------------------------------
// build() — validation
// ---------------------------------------------------------------------------

describe("HuffmanTree.build() validation", () => {
  it("throws on empty weights array", () => {
    expect(() => HuffmanTree.build([])).toThrow("weights must not be empty");
  });

  it("throws on zero frequency", () => {
    expect(() => HuffmanTree.build([[65, 0]])).toThrow(
      "frequency must be positive"
    );
  });

  it("throws on negative frequency", () => {
    expect(() => HuffmanTree.build([[65, -1]])).toThrow(
      "frequency must be positive"
    );
  });

  it("includes symbol and freq in the error message", () => {
    expect(() => HuffmanTree.build([[42, 0]])).toThrow("symbol=42, freq=0");
  });
});

// ---------------------------------------------------------------------------
// Single-symbol tree (edge case)
// ---------------------------------------------------------------------------

describe("single-symbol tree", () => {
  const tree = HuffmanTree.build([[65, 5]]);

  it("has symbolCount 1", () => {
    expect(tree.symbolCount()).toBe(1);
  });

  it("has weight equal to the single frequency", () => {
    expect(tree.weight()).toBe(5);
  });

  it("has depth 0", () => {
    // The root is a leaf at depth 0; maximum leaf depth is 0.
    expect(tree.depth()).toBe(0);
  });

  it("codeTable assigns '0' to the single symbol", () => {
    const table = tree.codeTable();
    expect(table.get(65)).toBe("0");
    expect(table.size).toBe(1);
  });

  it("codeFor returns '0' for the single symbol", () => {
    expect(tree.codeFor(65)).toBe("0");
  });

  it("codeFor returns undefined for unknown symbol", () => {
    expect(tree.codeFor(99)).toBeUndefined();
  });

  it("canonicalCodeTable assigns '0' to the single symbol", () => {
    const table = tree.canonicalCodeTable();
    expect(table.get(65)).toBe("0");
  });

  it("decodeAll decodes single symbol from '0' bits", () => {
    expect(tree.decodeAll("000", 3)).toEqual([65, 65, 65]);
  });

  it("decodeAll decodes one symbol from '0'", () => {
    expect(tree.decodeAll("0", 1)).toEqual([65]);
  });

  it("decodeAll on empty bits still decodes for single-leaf (no exhaustion throw)", () => {
    // Single-leaf trees don't require bits — the symbol is always available.
    // The '0' bit is consumed if present, but absence doesn't throw.
    expect(tree.decodeAll("", 1)).toEqual([65]);
  });

  it("leaves returns [[65, '0']]", () => {
    expect(tree.leaves()).toEqual([[65, "0"]]);
  });

  it("isValid returns true", () => {
    expect(tree.isValid()).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Two-symbol tree
// ---------------------------------------------------------------------------

describe("two-symbol tree", () => {
  // A(3) and B(1): A should be heavier so it's the left child.
  // Tie-breaking: leaf before internal, lower symbol first.
  // With A(weight=3) and B(weight=1):
  //   Heap initially: [B(1), A(3)]
  //   Pop B(1), Pop A(3) -> merge -> root(4), left=B, right=A
  // Wait — lower weight pops first. B(1) < A(3), so B pops first as 'left'.
  // Then A(3) pops as 'right'.
  // Tree:   root(4)
  //         /     \
  //        B(1)   A(3)
  // Codes: B='0', A='1'
  const tree = HuffmanTree.build([
    [65, 3],
    [66, 1],
  ]);

  it("has symbolCount 2", () => {
    expect(tree.symbolCount()).toBe(2);
  });

  it("has weight 4", () => {
    expect(tree.weight()).toBe(4);
  });

  it("has depth 1", () => {
    expect(tree.depth()).toBe(1);
  });

  it("isValid returns true", () => {
    expect(tree.isValid()).toBe(true);
  });

  it("codeTable has 2 entries", () => {
    const table = tree.codeTable();
    expect(table.size).toBe(2);
  });

  it("decodes correctly", () => {
    const table = tree.codeTable();
    const bits = table.get(65)! + table.get(66)! + table.get(65)!;
    expect(tree.decodeAll(bits, 3)).toEqual([65, 66, 65]);
  });
});

// ---------------------------------------------------------------------------
// Classic three-symbol example: AAABBC
// ---------------------------------------------------------------------------

describe("three-symbol tree (AAABBC)", () => {
  // A: weight=3, B: weight=2, C: weight=1
  //
  // Step 1: Heap = [C(1,leaf), B(2,leaf), A(3,leaf)]
  //   Pop C(1,leaf) [priority: 1,0,67,MAX]
  //   Pop B(2,leaf) [priority: 2,0,66,MAX]
  //   Merge -> BC_internal(3, order=0), left=C, right=B
  //   Heap = [A(3,leaf), BC_internal(3,order=0)]
  //
  // Step 2: A leaf [3,0,65,MAX] beats BC internal [3,1,MAX,0]
  //   Pop A(3,leaf)
  //   Pop BC_internal(3)
  //   Merge -> root(6), left=A, right=BC_internal
  //
  // Final tree shape:
  //      [6]
  //      / \
  //    A(3) [3]
  //          / \
  //        C(1) B(2)
  // Codes: A='0', C='10', B='11'
  const tree = HuffmanTree.build([
    [65, 3],
    [66, 2],
    [67, 1],
  ]);

  it("has symbolCount 3", () => {
    expect(tree.symbolCount()).toBe(3);
  });

  it("has weight 6", () => {
    expect(tree.weight()).toBe(6);
  });

  it("has depth 2", () => {
    expect(tree.depth()).toBe(2);
  });

  it("codeTable: A='0', C='10', B='11'", () => {
    // C pops before B (lower weight), so C is left child of the merged node
    const table = tree.codeTable();
    expect(table.get(65)).toBe("0");
    expect(table.get(67)).toBe("10"); // C
    expect(table.get(66)).toBe("11"); // B
  });

  it("codeFor(65) === '0'", () => {
    expect(tree.codeFor(65)).toBe("0");
  });

  it("codeFor(67) === '10' (C, lower weight, left child)", () => {
    expect(tree.codeFor(67)).toBe("10");
  });

  it("codeFor(66) === '11' (B, higher weight, right child)", () => {
    expect(tree.codeFor(66)).toBe("11");
  });

  it("codeFor unknown symbol returns undefined", () => {
    expect(tree.codeFor(100)).toBeUndefined();
  });

  it("canonicalCodeTable: A(length 1), B(length 2), C(length 2)", () => {
    // Canonical sorts by (length, symbol): A len=1, B len=2, C len=2
    // Sorted: A(1,len=1), B(66,len=2), C(67,len=2)
    // A -> 0 (len=1), B -> 10 (len=2), C -> 11 (len=2)
    const canonical = tree.canonicalCodeTable();
    expect(canonical.get(65)).toBe("0");
    expect(canonical.get(66)).toBe("10"); // B before C (lower symbol)
    expect(canonical.get(67)).toBe("11");
  });

  it("decodeAll: decode A=0, A=0, C=10, B=11 from '001011'", () => {
    // A='0', C='10', B='11'
    // Sequence: A A C B -> '0' '0' '10' '11' = '001011'
    expect(tree.decodeAll("001011", 4)).toEqual([65, 65, 67, 66]);
  });

  it("decodeAll: decode 1 symbol from '0'", () => {
    expect(tree.decodeAll("0", 1)).toEqual([65]);
  });

  it("decodeAll: decode 2 symbols from '10' + '11'", () => {
    // '10' = C, '11' = B
    expect(tree.decodeAll("1011", 2)).toEqual([67, 66]);
  });

  it("decodeAll throws when bits exhausted mid-decode", () => {
    // Only 1 bit given, need 2 bits for B or C
    expect(() => tree.decodeAll("1", 1)).toThrow("exhausted");
  });

  it("leaves are returned in left-to-right (in-order) order", () => {
    const leavesResult = tree.leaves();
    // In-order: root->left is A, root->right is internal -> left=C, right=B
    // (C was popped first from heap so it becomes the left child)
    expect(leavesResult).toEqual([
      [65, "0"],
      [67, "10"], // C
      [66, "11"], // B
    ]);
  });

  it("isValid returns true", () => {
    expect(tree.isValid()).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Tie-breaking: leaf-before-internal at equal weight
// ---------------------------------------------------------------------------

describe("tie-breaking: leaf-before-internal", () => {
  // A(1), B(1), C(2)
  // Step 1: heap has A(1), B(1), C(2)
  //         Pop A(leaf,1) and B(leaf,1) -> merge -> AB_internal(2, order=0)
  // Step 2: heap has C(leaf,2) and AB(internal,2)
  //         Pop C (leaf, priority [2,0,67,...]) before AB (priority [2,1,...])
  //         Pop AB -> merge -> root(4), left=C, right=AB
  // Codes: C='0', A='10', B='11'
  const tree = HuffmanTree.build([
    [65, 1],
    [66, 1],
    [67, 2],
  ]);

  it("has symbolCount 3", () => {
    expect(tree.symbolCount()).toBe(3);
  });

  it("has weight 4", () => {
    expect(tree.weight()).toBe(4);
  });

  it("isValid", () => {
    expect(tree.isValid()).toBe(true);
  });

  it("leaf-before-internal: C has shorter code than A or B", () => {
    const table = tree.codeTable();
    const lenC = table.get(67)!.length;
    const lenA = table.get(65)!.length;
    const lenB = table.get(66)!.length;
    // C should have shorter code (popped before the merged internal node)
    expect(lenC).toBeLessThan(lenA);
    expect(lenC).toBeLessThan(lenB);
  });

  it("C='0', A='10', B='11'", () => {
    const table = tree.codeTable();
    expect(table.get(67)).toBe("0");
    expect(table.get(65)).toBe("10");
    expect(table.get(66)).toBe("11");
  });
});

// ---------------------------------------------------------------------------
// Tie-breaking: lower symbol value among leaves
// ---------------------------------------------------------------------------

describe("tie-breaking: lower symbol wins among equal-weight leaves", () => {
  // A(1), B(1): A has lower symbol value so A pops first (left child)
  // Single merge -> root(2), left=A, right=B
  // A='0', B='1'
  const tree = HuffmanTree.build([
    [66, 1], // B — higher symbol
    [65, 1], // A — lower symbol
  ]);

  it("A (symbol 65) gets code '0' (pops first)", () => {
    expect(tree.codeFor(65)).toBe("0");
  });

  it("B (symbol 66) gets code '1'", () => {
    expect(tree.codeFor(66)).toBe("1");
  });
});

// ---------------------------------------------------------------------------
// Tie-breaking: FIFO among internal nodes
// ---------------------------------------------------------------------------

describe("tie-breaking: FIFO among internal nodes of equal weight", () => {
  // A(1), B(1), C(1), D(1) — all equal weight
  // Step 1: Pop A(1) and B(1) -> AB_internal(2, order=0)
  //         Heap: [C(1), D(1), AB(2)]
  // Step 2: Pop C(1) and D(1) -> CD_internal(2, order=1)
  //         Heap: [AB(2,order=0), CD(2,order=1)]
  // Step 3: Pop AB (order=0, FIFO wins) and CD (order=1) -> root(4)
  //         left=AB, right=CD
  // Codes: A='00', B='01', C='10', D='11'
  const tree = HuffmanTree.build([
    [65, 1],
    [66, 1],
    [67, 1],
    [68, 1],
  ]);

  it("has symbolCount 4", () => {
    expect(tree.symbolCount()).toBe(4);
  });

  it("has weight 4", () => {
    expect(tree.weight()).toBe(4);
  });

  it("has depth 2", () => {
    expect(tree.depth()).toBe(2);
  });

  it("all symbols have equal-length codes (balanced tree)", () => {
    const table = tree.codeTable();
    const lengths = [...table.values()].map((c) => c.length);
    expect(lengths.every((l) => l === 2)).toBe(true);
  });

  it("codes are A='00', B='01', C='10', D='11'", () => {
    const table = tree.codeTable();
    expect(table.get(65)).toBe("00");
    expect(table.get(66)).toBe("01");
    expect(table.get(67)).toBe("10");
    expect(table.get(68)).toBe("11");
  });

  it("isValid", () => {
    expect(tree.isValid()).toBe(true);
  });

  it("decodeAll roundtrip", () => {
    // '00' + '01' + '10' + '11' = '00011011'
    expect(tree.decodeAll("00011011", 4)).toEqual([65, 66, 67, 68]);
  });
});

// ---------------------------------------------------------------------------
// Larger tree: 5 symbols
// ---------------------------------------------------------------------------

describe("five-symbol tree", () => {
  // Frequencies: A=15, B=7, C=6, D=6, E=5
  // Total = 39
  const tree = HuffmanTree.build([
    [65, 15], // A
    [66, 7],  // B
    [67, 6],  // C
    [68, 6],  // D
    [69, 5],  // E
  ]);

  it("has symbolCount 5", () => {
    expect(tree.symbolCount()).toBe(5);
  });

  it("has weight 39", () => {
    expect(tree.weight()).toBe(39);
  });

  it("A has the shortest code (highest frequency)", () => {
    const table = tree.codeTable();
    const lenA = table.get(65)!.length;
    for (const [sym, code] of table) {
      if (sym !== 65) {
        expect(lenA).toBeLessThanOrEqual(code.length);
      }
    }
  });

  it("all codes are prefix-free", () => {
    const table = tree.codeTable();
    const codes = [...table.values()];
    for (let i = 0; i < codes.length; i++) {
      for (let j = 0; j < codes.length; j++) {
        if (i !== j) {
          expect(codes[j]!.startsWith(codes[i]!)).toBe(false);
        }
      }
    }
  });

  it("isValid", () => {
    expect(tree.isValid()).toBe(true);
  });

  it("decodeAll roundtrip for a message", () => {
    const table = tree.codeTable();
    // Encode "AABCE" and decode it back
    const message = [65, 65, 66, 67, 69];
    const bits = message.map((s) => table.get(s)!).join("");
    expect(tree.decodeAll(bits, 5)).toEqual(message);
  });
});

// ---------------------------------------------------------------------------
// canonicalCodeTable
// ---------------------------------------------------------------------------

describe("canonicalCodeTable", () => {
  it("single symbol returns {sym: '0'}", () => {
    const tree = HuffmanTree.build([[100, 3]]);
    const canonical = tree.canonicalCodeTable();
    expect(canonical.get(100)).toBe("0");
    expect(canonical.size).toBe(1);
  });

  it("canonical codes are sorted by (length, symbol)", () => {
    const tree = HuffmanTree.build([
      [65, 3],
      [66, 2],
      [67, 1],
    ]);
    const canonical = tree.canonicalCodeTable();
    // A: length 1 -> '0', B: length 2 -> '10', C: length 2 -> '11'
    expect(canonical.get(65)).toBe("0");
    expect(canonical.get(66)).toBe("10");
    expect(canonical.get(67)).toBe("11");
  });

  it("canonical codes have correct lengths matching the tree walk codes", () => {
    const tree = HuffmanTree.build([
      [65, 15],
      [66, 7],
      [67, 6],
      [68, 6],
      [69, 5],
    ]);
    const walkTable = tree.codeTable();
    const canonical = tree.canonicalCodeTable();

    // Lengths must match between walk-based and canonical codes
    for (const [sym, code] of walkTable) {
      expect(canonical.get(sym)!.length).toBe(code.length);
    }
  });

  it("canonical codes are prefix-free", () => {
    const tree = HuffmanTree.build([
      [65, 3],
      [66, 2],
      [67, 1],
      [68, 4],
    ]);
    const canonical = tree.canonicalCodeTable();
    const codes = [...canonical.values()];
    for (let i = 0; i < codes.length; i++) {
      for (let j = 0; j < codes.length; j++) {
        if (i !== j) {
          expect(codes[j]!.startsWith(codes[i]!)).toBe(false);
        }
      }
    }
  });

  it("balanced 4-symbol tree has canonical codes 00, 01, 10, 11", () => {
    const tree = HuffmanTree.build([
      [65, 1],
      [66, 1],
      [67, 1],
      [68, 1],
    ]);
    const canonical = tree.canonicalCodeTable();
    expect(canonical.get(65)).toBe("00");
    expect(canonical.get(66)).toBe("01");
    expect(canonical.get(67)).toBe("10");
    expect(canonical.get(68)).toBe("11");
  });
});

// ---------------------------------------------------------------------------
// decodeAll edge cases
// ---------------------------------------------------------------------------

describe("decodeAll edge cases", () => {
  it("decodes 0 symbols from empty string", () => {
    const tree = HuffmanTree.build([[65, 3], [66, 2]]);
    expect(tree.decodeAll("", 0)).toEqual([]);
  });

  it("throws when bit stream exhausted before count reached", () => {
    const tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]]);
    // Need at least 2 bits for B or C but stream has only 1 after A
    expect(() => tree.decodeAll("01", 2)).toThrow("exhausted");
  });

  it("ignores trailing bits beyond what is needed", () => {
    const tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]]);
    // Decode just 1 symbol, even though there are more bits
    expect(tree.decodeAll("010011", 1)).toEqual([65]);
  });

  it("single-leaf: decodes count symbols each from '0'", () => {
    const tree = HuffmanTree.build([[42, 10]]);
    expect(tree.decodeAll("00000", 5)).toEqual([42, 42, 42, 42, 42]);
  });

  it("single-leaf: decodes even if fewer bits than count (no exhaustion error)", () => {
    // Single-leaf trees only consume bits if available; no throw on underflow.
    // This matches the reference Python implementation.
    const tree = HuffmanTree.build([[42, 10]]);
    expect(tree.decodeAll("00", 3)).toEqual([42, 42, 42]);
  });

  it("long decode of alternating A/B codes", () => {
    const tree = HuffmanTree.build([[65, 3], [66, 1]]);
    const table = tree.codeTable();
    const codeA = table.get(65)!;
    const codeB = table.get(66)!;
    const bits = (codeA + codeB).repeat(4);
    expect(tree.decodeAll(bits, 8)).toEqual([65, 66, 65, 66, 65, 66, 65, 66]);
  });
});

// ---------------------------------------------------------------------------
// isValid — structural invariants
// ---------------------------------------------------------------------------

describe("isValid", () => {
  it("returns true for a well-formed tree", () => {
    const tree = HuffmanTree.build([[1, 5], [2, 3], [3, 2]]);
    expect(tree.isValid()).toBe(true);
  });

  it("returns true for a large tree", () => {
    const weights: Array<[number, number]> = [];
    for (let i = 0; i < 256; i++) {
      weights.push([i, i + 1]);
    }
    const tree = HuffmanTree.build(weights);
    expect(tree.isValid()).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Full round-trip: byte-level coding (0–255 symbols)
// ---------------------------------------------------------------------------

describe("byte-level round-trip", () => {
  it("encodes and decodes a simple byte message", () => {
    // Simulate a message: "hello" = [104, 101, 108, 108, 111]
    const freqs = new Map<number, number>();
    const message = [104, 101, 108, 108, 111];
    for (const b of message) {
      freqs.set(b, (freqs.get(b) ?? 0) + 1);
    }
    const weights: Array<[number, number]> = [...freqs.entries()];
    const tree = HuffmanTree.build(weights);

    const table = tree.codeTable();
    const bits = message.map((b) => table.get(b)!).join("");
    const decoded = tree.decodeAll(bits, message.length);
    expect(decoded).toEqual(message);
  });

  it("handles all 256 ASCII values in a tree", () => {
    const weights: Array<[number, number]> = [];
    for (let i = 0; i < 256; i++) {
      weights.push([i, Math.max(1, 256 - i)]); // decreasing frequency
    }
    const tree = HuffmanTree.build(weights);
    expect(tree.symbolCount()).toBe(256);
    expect(tree.isValid()).toBe(true);

    // Symbol 0 (highest frequency) should have a shorter code than symbol 255
    const table = tree.codeTable();
    expect(table.get(0)!.length).toBeLessThanOrEqual(table.get(255)!.length);
  });
});

// ---------------------------------------------------------------------------
// leaves()
// ---------------------------------------------------------------------------

describe("leaves()", () => {
  it("returns leaves in left-to-right order", () => {
    const tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]]);
    const leavesResult = tree.leaves();
    // A is left child of root; right subtree has C (left) and B (right)
    // because C(weight=1) pops before B(weight=2) from the heap
    expect(leavesResult.map(([sym]) => sym)).toEqual([65, 67, 66]);
  });

  it("codes in leaves match codeTable", () => {
    const tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]]);
    const table = tree.codeTable();
    const leavesResult = tree.leaves();
    for (const [sym, code] of leavesResult) {
      expect(table.get(sym)).toBe(code);
    }
  });

  it("leaves count equals symbolCount", () => {
    const weights: Array<[number, number]> = [[1, 5], [2, 3], [3, 2], [4, 1]];
    const tree = HuffmanTree.build(weights);
    expect(tree.leaves().length).toBe(tree.symbolCount());
  });
});

// ---------------------------------------------------------------------------
// weight() and depth()
// ---------------------------------------------------------------------------

describe("weight() and depth()", () => {
  it("weight is sum of all frequencies", () => {
    const tree = HuffmanTree.build([[1, 10], [2, 20], [3, 5]]);
    expect(tree.weight()).toBe(35);
  });

  it("depth is 0 for a single-symbol tree", () => {
    const tree = HuffmanTree.build([[5, 1]]);
    expect(tree.depth()).toBe(0);
  });

  it("depth is 1 for two symbols", () => {
    const tree = HuffmanTree.build([[1, 5], [2, 3]]);
    expect(tree.depth()).toBe(1);
  });

  it("depth is at least log2(n) for n equal-weight symbols", () => {
    // 8 equal-weight symbols -> perfect binary tree of depth 3
    const weights: Array<[number, number]> = Array.from({ length: 8 }, (_, i) => [i, 1]);
    const tree = HuffmanTree.build(weights);
    expect(tree.depth()).toBe(3);
  });
});
