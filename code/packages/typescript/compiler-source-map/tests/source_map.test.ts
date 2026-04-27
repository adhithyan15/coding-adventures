/**
 * Tests for the source map chain.
 *
 * Covers:
 *   1. SourceToAst: add, lookupByNodeId
 *   2. AstToIr: add, lookupByAstNodeId, lookupByIrId
 *   3. IrToIr: addMapping, addDeletion, lookupByOriginalId, lookupByNewId
 *   4. IrToMachineCode: add, lookupByIrId, lookupByMCOffset
 *   5. SourceMapChain: sourceToMC and mcToSource composite queries
 *   6. sourcePositionToString
 */

import { describe, it, expect } from "vitest";
import {
  SourceToAst,
  AstToIr,
  IrToIr,
  IrToMachineCode,
  SourceMapChain,
  sourcePositionToString,
} from "../src/source_map.js";

// ────────────────────────────────────────────────────────���─────────────────────
// SourcePosition helpers
// ──────────────────────────────────────────────────────────────────────────────

describe("sourcePositionToString", () => {
  it("formats as 'file:line:col (len=N)'", () => {
    expect(
      sourcePositionToString({ file: "hello.bf", line: 1, column: 3, length: 1 })
    ).toBe("hello.bf:1:3 (len=1)");
  });

  it("handles multi-char tokens", () => {
    expect(
      sourcePositionToString({ file: "test.bas", line: 10, column: 5, length: 5 })
    ).toBe("test.bas:10:5 (len=5)");
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// SourceToAst
// ──────────────────────────────────────────────────────────────────────────────

describe("SourceToAst", () => {
  it("starts empty", () => {
    const s = new SourceToAst();
    expect(s.entries).toHaveLength(0);
  });

  it("add() appends an entry", () => {
    const s = new SourceToAst();
    s.add({ file: "hello.bf", line: 1, column: 1, length: 1 }, 42);
    expect(s.entries).toHaveLength(1);
    expect(s.entries[0].astNodeId).toBe(42);
  });

  it("lookupByNodeId returns the source position for a known ID", () => {
    const s = new SourceToAst();
    const pos = { file: "hello.bf", line: 1, column: 3, length: 1 };
    s.add(pos, 42);
    const found = s.lookupByNodeId(42);
    expect(found).not.toBeNull();
    expect(found!.column).toBe(3);
  });

  it("lookupByNodeId returns null for unknown ID", () => {
    const s = new SourceToAst();
    expect(s.lookupByNodeId(999)).toBeNull();
  });

  it("lookupByNodeId returns the first match when multiple entries share an ID", () => {
    const s = new SourceToAst();
    s.add({ file: "a.bf", line: 1, column: 1, length: 1 }, 5);
    s.add({ file: "b.bf", line: 2, column: 2, length: 1 }, 5);
    const found = s.lookupByNodeId(5);
    expect(found!.file).toBe("a.bf");
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// AstToIr
// ──────────────────────────────────────────────────────────────────────────────

describe("AstToIr", () => {
  it("starts empty", () => {
    const a = new AstToIr();
    expect(a.entries).toHaveLength(0);
  });

  it("add() stores AST node → IR IDs mapping", () => {
    const a = new AstToIr();
    a.add(10, [100, 101, 102, 103]);
    expect(a.entries).toHaveLength(1);
    expect(a.entries[0].astNodeId).toBe(10);
    expect(a.entries[0].irIds).toEqual([100, 101, 102, 103]);
  });

  it("lookupByAstNodeId returns IDs for known node", () => {
    const a = new AstToIr();
    a.add(5, [7, 8, 9]);
    expect(a.lookupByAstNodeId(5)).toEqual([7, 8, 9]);
  });

  it("lookupByAstNodeId returns null for unknown node", () => {
    const a = new AstToIr();
    expect(a.lookupByAstNodeId(999)).toBeNull();
  });

  it("lookupByIrId returns the AST node that produced the given IR ID", () => {
    const a = new AstToIr();
    a.add(10, [7, 8, 9]);
    expect(a.lookupByIrId(7)).toBe(10);
    expect(a.lookupByIrId(8)).toBe(10);
    expect(a.lookupByIrId(9)).toBe(10);
  });

  it("lookupByIrId returns -1 for unknown IR ID", () => {
    const a = new AstToIr();
    expect(a.lookupByIrId(999)).toBe(-1);
  });

  it("supports multiple AST nodes", () => {
    const a = new AstToIr();
    a.add(1, [0, 1, 2, 3]);  // "+" → 4 IR instructions
    a.add(2, [4, 5, 6]);     // "." → 3 IR instructions
    expect(a.lookupByAstNodeId(1)).toEqual([0, 1, 2, 3]);
    expect(a.lookupByAstNodeId(2)).toEqual([4, 5, 6]);
    expect(a.lookupByIrId(5)).toBe(2);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// IrToIr
// ──────────────────────────────────────────────────────────────────────────────

describe("IrToIr", () => {
  it("stores the pass name", () => {
    const m = new IrToIr("contraction");
    expect(m.passName).toBe("contraction");
  });

  it("addMapping records original → new IDs", () => {
    const m = new IrToIr("identity");
    m.addMapping(7, [7]);
    expect(m.lookupByOriginalId(7)).toEqual([7]);
  });

  it("addDeletion marks the ID as deleted", () => {
    const m = new IrToIr("dead_store");
    m.addDeletion(42);
    expect(m.deleted.has(42)).toBe(true);
    expect(m.lookupByOriginalId(42)).toBeNull();
  });

  it("lookupByOriginalId returns null for unknown ID", () => {
    const m = new IrToIr("identity");
    expect(m.lookupByOriginalId(999)).toBeNull();
  });

  it("lookupByNewId returns the original ID for a given new ID", () => {
    const m = new IrToIr("contraction");
    // IDs 7, 8, 9 collapse into single instruction 100
    m.addMapping(7, [100]);
    m.addMapping(8, [100]);
    m.addMapping(9, [100]);
    expect(m.lookupByNewId(100)).toBe(7); // first match
  });

  it("lookupByNewId returns -1 for unknown new ID", () => {
    const m = new IrToIr("identity");
    expect(m.lookupByNewId(999)).toBe(-1);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// IrToMachineCode
// ──────────────────────────────────────────────────────────────────────────────

describe("IrToMachineCode", () => {
  it("add() records irId → (offset, length)", () => {
    const m = new IrToMachineCode();
    m.add(5, 0x14, 4);
    expect(m.entries).toHaveLength(1);
    expect(m.entries[0]).toEqual({ irId: 5, mcOffset: 0x14, mcLength: 4 });
  });

  it("lookupByIrId returns offset and length", () => {
    const m = new IrToMachineCode();
    m.add(5, 20, 8);
    const { offset, length } = m.lookupByIrId(5);
    expect(offset).toBe(20);
    expect(length).toBe(8);
  });

  it("lookupByIrId returns -1 offset for unknown ID", () => {
    const m = new IrToMachineCode();
    const { offset, length } = m.lookupByIrId(999);
    expect(offset).toBe(-1);
    expect(length).toBe(0);
  });

  it("lookupByMCOffset finds instruction containing the offset", () => {
    const m = new IrToMachineCode();
    m.add(1, 0, 4);   // instructions 0..3
    m.add(2, 4, 4);   // instructions 4..7
    m.add(3, 8, 8);   // instructions 8..15
    expect(m.lookupByMCOffset(0)).toBe(1);
    expect(m.lookupByMCOffset(3)).toBe(1);
    expect(m.lookupByMCOffset(4)).toBe(2);
    expect(m.lookupByMCOffset(8)).toBe(3);
    expect(m.lookupByMCOffset(15)).toBe(3);
  });

  it("lookupByMCOffset returns -1 for offset outside all entries", () => {
    const m = new IrToMachineCode();
    m.add(1, 0, 4);
    expect(m.lookupByMCOffset(100)).toBe(-1);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// SourceMapChain
// ──────────────────────────────────────────────────────────────────────────────

describe("SourceMapChain", () => {
  it("starts with empty segments and null irToMachineCode", () => {
    const chain = new SourceMapChain();
    expect(chain.sourceToAst.entries).toHaveLength(0);
    expect(chain.astToIr.entries).toHaveLength(0);
    expect(chain.irToIr).toHaveLength(0);
    expect(chain.irToMachineCode).toBeNull();
  });

  it("addOptimizerPass appends an IrToIr segment", () => {
    const chain = new SourceMapChain();
    const pass = new IrToIr("identity");
    chain.addOptimizerPass(pass);
    expect(chain.irToIr).toHaveLength(1);
  });

  it("sourceToMC returns null when irToMachineCode is null (backend not run yet)", () => {
    const chain = new SourceMapChain();
    chain.sourceToAst.add({ file: "t.bf", line: 1, column: 1, length: 1 }, 0);
    chain.astToIr.add(0, [0, 1, 2, 3]);
    const result = chain.sourceToMC({ file: "t.bf", line: 1, column: 1, length: 1 });
    expect(result).toBeNull();
  });

  it("mcToSource returns null when irToMachineCode is null", () => {
    const chain = new SourceMapChain();
    expect(chain.mcToSource(0)).toBeNull();
  });

  describe("full pipeline", () => {
    /**
     * Build a minimal end-to-end source map:
     *
     *   source: "+" at hello.bf:1:1
     *     → AST node 0
     *     → IR instructions [0, 1, 2, 3]  (LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE)
     *     → no optimizer passes
     *     → machine code at offsets [0, 4, 8, 12], each 4 bytes
     */
    function buildFullChain(): SourceMapChain {
      const chain = new SourceMapChain();

      // Segment 1: source → AST
      chain.sourceToAst.add({ file: "hello.bf", line: 1, column: 1, length: 1 }, 0);

      // Segment 2: AST → IR
      chain.astToIr.add(0, [0, 1, 2, 3]);

      // Segment 4: IR → machine code
      chain.irToMachineCode = new IrToMachineCode();
      chain.irToMachineCode.add(0, 0, 4);
      chain.irToMachineCode.add(1, 4, 4);
      chain.irToMachineCode.add(2, 8, 4);
      chain.irToMachineCode.add(3, 12, 4);

      return chain;
    }

    it("sourceToMC finds machine code entries for a source position", () => {
      const chain = buildFullChain();
      const entries = chain.sourceToMC({ file: "hello.bf", line: 1, column: 1, length: 1 });
      expect(entries).not.toBeNull();
      expect(entries!.length).toBe(4);
      expect(entries![0].mcOffset).toBe(0);
      expect(entries![1].mcOffset).toBe(4);
    });

    it("mcToSource traces machine code back to source position", () => {
      const chain = buildFullChain();
      const pos = chain.mcToSource(8); // offset 8 → IR id 2 → AST 0 → hello.bf:1:1
      expect(pos).not.toBeNull();
      expect(pos!.file).toBe("hello.bf");
      expect(pos!.line).toBe(1);
      expect(pos!.column).toBe(1);
    });

    it("sourceToMC returns null for unknown source position", () => {
      const chain = buildFullChain();
      const result = chain.sourceToMC({ file: "other.bf", line: 99, column: 1, length: 1 });
      expect(result).toBeNull();
    });

    it("mcToSource returns null for unknown machine code offset", () => {
      const chain = buildFullChain();
      expect(chain.mcToSource(9999)).toBeNull();
    });
  });

  describe("with optimizer passes", () => {
    /**
     * A contraction optimizer folds instructions 0, 1, 2 into a single
     * new instruction 10. The source map chain should trace through it.
     */
    it("sourceToMC follows IDs through optimizer passes", () => {
      const chain = new SourceMapChain();

      chain.sourceToAst.add({ file: "t.bf", line: 1, column: 1, length: 1 }, 0);
      chain.astToIr.add(0, [0, 1, 2]);

      // Optimizer: 0, 1, 2 → [10]
      const pass = new IrToIr("contraction");
      pass.addMapping(0, [10]);
      pass.addMapping(1, [10]);
      pass.addMapping(2, [10]);
      chain.addOptimizerPass(pass);

      chain.irToMachineCode = new IrToMachineCode();
      chain.irToMachineCode.add(10, 0, 4);

      const entries = chain.sourceToMC({ file: "t.bf", line: 1, column: 1, length: 1 });
      expect(entries).not.toBeNull();
      // All three original IDs map to instruction 10, so we get 3 references
      // but they all point to the same MC entry — deduplication is not done
      expect(entries!.length).toBeGreaterThan(0);
      expect(entries![0].mcOffset).toBe(0);
    });

    it("sourceToMC returns null when all IR IDs are deleted by optimizer", () => {
      const chain = new SourceMapChain();

      chain.sourceToAst.add({ file: "t.bf", line: 1, column: 1, length: 1 }, 0);
      chain.astToIr.add(0, [5]);

      // Optimizer deletes instruction 5 (dead store)
      const pass = new IrToIr("dead_store");
      pass.addDeletion(5);
      chain.addOptimizerPass(pass);

      chain.irToMachineCode = new IrToMachineCode();
      chain.irToMachineCode.add(99, 0, 4);

      const result = chain.sourceToMC({ file: "t.bf", line: 1, column: 1, length: 1 });
      expect(result).toBeNull();
    });

    it("mcToSource traces back through optimizer passes in reverse", () => {
      const chain = new SourceMapChain();

      chain.sourceToAst.add({ file: "t.bf", line: 1, column: 5, length: 1 }, 7);
      chain.astToIr.add(7, [20]);

      // Optimizer maps 20 → 30
      const pass = new IrToIr("identity");
      pass.addMapping(20, [30]);
      chain.addOptimizerPass(pass);

      chain.irToMachineCode = new IrToMachineCode();
      chain.irToMachineCode.add(30, 100, 4);

      const pos = chain.mcToSource(100);
      expect(pos).not.toBeNull();
      expect(pos!.column).toBe(5);
    });
  });
});
