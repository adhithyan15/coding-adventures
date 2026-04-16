/**
 * Tests for the Brainfuck IR compiler.
 *
 * Tests cover:
 *   1. BuildConfig presets (debugConfig, releaseConfig)
 *   2. Empty program
 *   3. Single commands: >, <, +, -, ., ,
 *   4. Loop compilation
 *   5. Debug mode: bounds checks
 *   6. Source map population
 *   7. IR text output (printer integration)
 *   8. Roundtrip: parse(print(program)) == program
 *   9. Complex programs
 *  10. Instruction ID uniqueness
 *  11. Error cases
 */

import { describe, it, expect } from "vitest";
import { parseBrainfuck } from "@coding-adventures/brainfuck";
import { IrOp, IrProgram, printIr, parseIr } from "@coding-adventures/compiler-ir";
import type { IrRegister, IrImmediate } from "@coding-adventures/compiler-ir";
import {
  compile,
  releaseConfig,
  debugConfig,
  type BuildConfig,
  type CompileResult,
} from "../src/index.js";
import type { ASTNode } from "@coding-adventures/parser";

// ──────────────────────────────────────────────────────────────────────────────
// Test helpers
// ──────────────────────────────────────────────────────────────────────────────

/** Tokenize, parse, and compile a Brainfuck source string. */
function compileSource(source: string, config: BuildConfig): CompileResult {
  const ast = parseBrainfuck(source);
  return compile(ast, "test.bf", config);
}

/** Like compileSource but throws on error (for tests that expect success). */
function mustCompile(source: string, config: BuildConfig): CompileResult {
  return compileSource(source, config);
}

/** Count instructions with the given opcode. */
function countOpcode(program: IrProgram, opcode: IrOp): number {
  return program.instructions.filter((i) => i.opcode === opcode).length;
}

/** Check if the program contains a label with the given name. */
function hasLabel(program: IrProgram, name: string): boolean {
  return program.instructions.some(
    (i) =>
      i.opcode === IrOp.LABEL &&
      i.operands.length > 0 &&
      i.operands[0].kind === "label" &&
      i.operands[0].name === name
  );
}

// ──────────────────────────────────────────────────────────────────────────────
// BuildConfig tests
// ──────────────────────────────────────────────────────────────────────────────

describe("debugConfig", () => {
  it("has bounds checks enabled", () => {
    expect(debugConfig().insertBoundsChecks).toBe(true);
  });

  it("has debug locs enabled", () => {
    expect(debugConfig().insertDebugLocs).toBe(true);
  });

  it("has byte masking enabled", () => {
    expect(debugConfig().maskByteArithmetic).toBe(true);
  });

  it("has tape size 30000", () => {
    expect(debugConfig().tapeSize).toBe(30000);
  });
});

describe("releaseConfig", () => {
  it("has bounds checks disabled", () => {
    expect(releaseConfig().insertBoundsChecks).toBe(false);
  });

  it("has debug locs disabled", () => {
    expect(releaseConfig().insertDebugLocs).toBe(false);
  });

  it("has byte masking enabled", () => {
    expect(releaseConfig().maskByteArithmetic).toBe(true);
  });

  it("has tape size 30000", () => {
    expect(releaseConfig().tapeSize).toBe(30000);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Empty program
// ──────────────────────────────────────────────────────────────────────────────

describe("empty program", () => {
  it("produces _start label", () => {
    const { program } = mustCompile("", releaseConfig());
    expect(hasLabel(program, "_start")).toBe(true);
  });

  it("produces exactly one HALT", () => {
    const { program } = mustCompile("", releaseConfig());
    expect(countOpcode(program, IrOp.HALT)).toBe(1);
  });

  it("has version 1", () => {
    const { program } = mustCompile("", releaseConfig());
    expect(program.version).toBe(1);
  });

  it("has entry label '_start'", () => {
    const { program } = mustCompile("", releaseConfig());
    expect(program.entryLabel).toBe("_start");
  });

  it("has exactly one data declaration for the tape", () => {
    const { program } = mustCompile("", releaseConfig());
    expect(program.data).toHaveLength(1);
    expect(program.data[0].label).toBe("tape");
    expect(program.data[0].size).toBe(30000);
    expect(program.data[0].init).toBe(0);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Single command tests
// ──────────────────────────────────────────────────────────────────────────────

describe("INC (+) command", () => {
  it("emits LOAD_BYTE for reading the cell", () => {
    const { program } = mustCompile("+", releaseConfig());
    expect(countOpcode(program, IrOp.LOAD_BYTE)).toBeGreaterThanOrEqual(1);
  });

  it("emits STORE_BYTE for writing the cell", () => {
    const { program } = mustCompile("+", releaseConfig());
    expect(countOpcode(program, IrOp.STORE_BYTE)).toBeGreaterThanOrEqual(1);
  });

  it("emits AND_IMM for byte masking when enabled", () => {
    const { program } = mustCompile("+", releaseConfig());
    expect(countOpcode(program, IrOp.AND_IMM)).toBeGreaterThanOrEqual(1);
  });

  it("does NOT emit AND_IMM when masking is disabled", () => {
    const config = { ...releaseConfig(), maskByteArithmetic: false };
    const { program } = mustCompile("+", config);
    expect(countOpcode(program, IrOp.AND_IMM)).toBe(0);
  });

  it("emits ADD_IMM with delta=1", () => {
    const { program } = mustCompile("+", releaseConfig());
    const found = program.instructions.some(
      (i) =>
        i.opcode === IrOp.ADD_IMM &&
        i.operands.length >= 3 &&
        i.operands[2].kind === "immediate" &&
        (i.operands[2] as IrImmediate).value === 1
    );
    expect(found).toBe(true);
  });
});

describe("DEC (-) command", () => {
  it("emits ADD_IMM with delta=-1", () => {
    const { program } = mustCompile("-", releaseConfig());
    const found = program.instructions.some(
      (i) =>
        i.opcode === IrOp.ADD_IMM &&
        i.operands.length >= 3 &&
        i.operands[2].kind === "immediate" &&
        (i.operands[2] as IrImmediate).value === -1
    );
    expect(found).toBe(true);
  });
});

describe("RIGHT (>) command", () => {
  it("emits ADD_IMM v1, v1, 1", () => {
    const { program } = mustCompile(">", releaseConfig());
    const found = program.instructions.some(
      (i) =>
        i.opcode === IrOp.ADD_IMM &&
        i.operands.length >= 3 &&
        i.operands[0].kind === "register" &&
        (i.operands[0] as IrRegister).index === 1 && // REG_TAPE_PTR = v1
        i.operands[2].kind === "immediate" &&
        (i.operands[2] as IrImmediate).value === 1
    );
    expect(found).toBe(true);
  });
});

describe("LEFT (<) command", () => {
  it("emits ADD_IMM v1, v1, -1", () => {
    const { program } = mustCompile("<", releaseConfig());
    const found = program.instructions.some(
      (i) =>
        i.opcode === IrOp.ADD_IMM &&
        i.operands.length >= 3 &&
        i.operands[0].kind === "register" &&
        (i.operands[0] as IrRegister).index === 1 && // REG_TAPE_PTR = v1
        i.operands[2].kind === "immediate" &&
        (i.operands[2] as IrImmediate).value === -1
    );
    expect(found).toBe(true);
  });
});

describe("OUTPUT (.) command", () => {
  it("emits SYSCALL 1 (write)", () => {
    const { program } = mustCompile(".", releaseConfig());
    const hasCopy = program.instructions.some(
      (i) =>
        i.opcode === IrOp.ADD_IMM &&
        i.operands.length === 3 &&
        i.operands[0].kind === "register" &&
        (i.operands[0] as IrRegister).index === 4 &&
        i.operands[1].kind === "register" &&
        (i.operands[1] as IrRegister).index === 2 &&
        i.operands[2].kind === "immediate" &&
        (i.operands[2] as IrImmediate).value === 0
    );
    const found = program.instructions.some(
      (i) =>
        i.opcode === IrOp.SYSCALL &&
        i.operands.length > 0 &&
        i.operands[0].kind === "immediate" &&
        (i.operands[0] as IrImmediate).value === 1
    );
    expect(hasCopy).toBe(true);
    expect(found).toBe(true);
  });
});

describe("INPUT (,) command", () => {
  it("emits SYSCALL 2 (read)", () => {
    const { program } = mustCompile(",", releaseConfig());
    const found = program.instructions.some(
      (i) =>
        i.opcode === IrOp.SYSCALL &&
        i.operands.length > 0 &&
        i.operands[0].kind === "immediate" &&
        (i.operands[0] as IrImmediate).value === 2
    );
    expect(found).toBe(true);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Loop compilation
// ──────────────────────────────────────────────────────────────────────────────

describe("simple loop [-]", () => {
  it("has loop_0_start label", () => {
    const { program } = mustCompile("[-]", releaseConfig());
    expect(hasLabel(program, "loop_0_start")).toBe(true);
  });

  it("has loop_0_end label", () => {
    const { program } = mustCompile("[-]", releaseConfig());
    expect(hasLabel(program, "loop_0_end")).toBe(true);
  });

  it("has BRANCH_Z for loop entry", () => {
    const { program } = mustCompile("[-]", releaseConfig());
    expect(countOpcode(program, IrOp.BRANCH_Z)).toBeGreaterThanOrEqual(1);
  });

  it("has JUMP for loop back-edge", () => {
    const { program } = mustCompile("[-]", releaseConfig());
    expect(countOpcode(program, IrOp.JUMP)).toBeGreaterThanOrEqual(1);
  });
});

describe("nested loops [>[+<-]]", () => {
  it("has loop_0_start and loop_1_start labels", () => {
    const { program } = mustCompile("[>[+<-]]", releaseConfig());
    expect(hasLabel(program, "loop_0_start")).toBe(true);
    expect(hasLabel(program, "loop_1_start")).toBe(true);
  });
});

describe("empty loop []", () => {
  it("still emits loop labels", () => {
    const { program } = mustCompile("[]", releaseConfig());
    expect(hasLabel(program, "loop_0_start")).toBe(true);
    expect(hasLabel(program, "loop_0_end")).toBe(true);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Bounds checking (debug mode)
// ──────────────────────────────────────────────────────────────────────────────

describe("bounds checking with debug config", () => {
  it("> emits CMP_GT for right bounds check", () => {
    const { program } = mustCompile(">", debugConfig());
    expect(countOpcode(program, IrOp.CMP_GT)).toBeGreaterThanOrEqual(1);
  });

  it("> emits BRANCH_NZ to trap", () => {
    const { program } = mustCompile(">", debugConfig());
    expect(countOpcode(program, IrOp.BRANCH_NZ)).toBeGreaterThanOrEqual(1);
  });

  it("> emits __trap_oob label", () => {
    const { program } = mustCompile(">", debugConfig());
    expect(hasLabel(program, "__trap_oob")).toBe(true);
  });

  it("< emits CMP_LT for left bounds check", () => {
    const { program } = mustCompile("<", debugConfig());
    expect(countOpcode(program, IrOp.CMP_LT)).toBeGreaterThanOrEqual(1);
  });

  it("release mode has NO CMP_GT", () => {
    const { program } = mustCompile("><", releaseConfig());
    expect(countOpcode(program, IrOp.CMP_GT)).toBe(0);
  });

  it("release mode has NO CMP_LT", () => {
    const { program } = mustCompile("><", releaseConfig());
    expect(countOpcode(program, IrOp.CMP_LT)).toBe(0);
  });

  it("release mode has NO __trap_oob label", () => {
    const { program } = mustCompile("><", releaseConfig());
    expect(hasLabel(program, "__trap_oob")).toBe(false);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Source map tests
// ──────────────────────────────────────────────────────────────────────────────

describe("source map: SourceToAst", () => {
  it("+. produces 2 SourceToAst entries (one per command)", () => {
    const { sourceMap } = mustCompile("+.", releaseConfig());
    expect(sourceMap.sourceToAst.entries).toHaveLength(2);
  });

  it("+ is at column 1", () => {
    const { sourceMap } = mustCompile("+.", releaseConfig());
    expect(sourceMap.sourceToAst.entries[0].pos.column).toBe(1);
  });

  it(". is at column 2", () => {
    const { sourceMap } = mustCompile("+.", releaseConfig());
    expect(sourceMap.sourceToAst.entries[1].pos.column).toBe(2);
  });

  it("all entries have file='test.bf'", () => {
    const { sourceMap } = mustCompile("+.", releaseConfig());
    for (const entry of sourceMap.sourceToAst.entries) {
      expect(entry.pos.file).toBe("test.bf");
    }
  });
});

describe("source map: AstToIr", () => {
  it("+ produces 1 AstToIr entry", () => {
    const { sourceMap } = mustCompile("+", releaseConfig());
    expect(sourceMap.astToIr.entries).toHaveLength(1);
  });

  it("+ maps to 4 IR IDs: LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE", () => {
    const { sourceMap } = mustCompile("+", releaseConfig());
    expect(sourceMap.astToIr.entries[0].irIds).toHaveLength(4);
  });

  it("[-] has at least 2 source map entries (loop + command)", () => {
    const { sourceMap } = mustCompile("[-]", releaseConfig());
    expect(sourceMap.sourceToAst.entries.length).toBeGreaterThanOrEqual(2);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// IR printer integration
// ──────────────────────────────────────────────────────────────────────────────

describe("IR printer integration", () => {
  it("printed IR contains .version 1", () => {
    const { program } = mustCompile("+.", releaseConfig());
    expect(printIr(program)).toContain(".version 1");
  });

  it("printed IR contains .data tape 30000 0", () => {
    const { program } = mustCompile("+.", releaseConfig());
    expect(printIr(program)).toContain(".data tape 30000 0");
  });

  it("printed IR contains .entry _start", () => {
    const { program } = mustCompile("+.", releaseConfig());
    expect(printIr(program)).toContain(".entry _start");
  });

  it("printed IR contains LOAD_BYTE", () => {
    const { program } = mustCompile("+.", releaseConfig());
    expect(printIr(program)).toContain("LOAD_BYTE");
  });

  it("printed IR contains HALT", () => {
    const { program } = mustCompile("+.", releaseConfig());
    expect(printIr(program)).toContain("HALT");
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Roundtrip: print then parse
// ──────────────────────────────────────────────────────────────────────────────

describe("IR print/parse roundtrip", () => {
  it("roundtrip preserves instruction count for ++[-].", () => {
    const { program } = mustCompile("++[-].", releaseConfig());
    const text = printIr(program);
    const reparsed = parseIr(text);
    expect(reparsed.instructions).toHaveLength(program.instructions.length);
  });

  it("roundtrip preserves instruction count for ,", () => {
    const { program } = mustCompile(",", releaseConfig());
    const text = printIr(program);
    const reparsed = parseIr(text);
    expect(reparsed.instructions).toHaveLength(program.instructions.length);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Complex programs
// ──────────────────────────────────────────────────────────────────────────────

describe("Hello World fragment: ++++++++[>+++++++++<-]>.", () => {
  it("has loop_0_start label", () => {
    const { program } = mustCompile("++++++++[>+++++++++<-]>.", releaseConfig());
    expect(hasLabel(program, "loop_0_start")).toBe(true);
  });

  it("has SYSCALL 1 (write) for output", () => {
    const { program } = mustCompile("++++++++[>+++++++++<-]>.", releaseConfig());
    const found = program.instructions.some(
      (i) =>
        i.opcode === IrOp.SYSCALL &&
        i.operands[0]?.kind === "immediate" &&
        (i.operands[0] as IrImmediate).value === 1
    );
    expect(found).toBe(true);
  });
});

describe("cat program: ,[.,]", () => {
  it("has SYSCALL 2 (read)", () => {
    const { program } = mustCompile(",[.,]", releaseConfig());
    const found = program.instructions.some(
      (i) =>
        i.opcode === IrOp.SYSCALL &&
        i.operands[0]?.kind === "immediate" &&
        (i.operands[0] as IrImmediate).value === 2
    );
    expect(found).toBe(true);
  });

  it("has SYSCALL 1 (write)", () => {
    const { program } = mustCompile(",[.,]", releaseConfig());
    const found = program.instructions.some(
      (i) =>
        i.opcode === IrOp.SYSCALL &&
        i.operands[0]?.kind === "immediate" &&
        (i.operands[0] as IrImmediate).value === 1
    );
    expect(found).toBe(true);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Custom tape size
// ──────────────────────────────────────────────────────────────────────────────

describe("custom tape size", () => {
  it("uses the custom tape size in the data declaration", () => {
    const config = { ...releaseConfig(), tapeSize: 1000 };
    const { program } = mustCompile("", config);
    expect(program.data[0].size).toBe(1000);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Instruction ID uniqueness
// ──────────────────────────────────────────────────────────────────────────────

describe("instruction IDs", () => {
  it("all non-label instruction IDs are unique", () => {
    const { program } = mustCompile("++[>+<-].", releaseConfig());
    const seen = new Set<number>();
    for (const instr of program.instructions) {
      if (instr.id === -1) continue; // labels
      expect(seen.has(instr.id)).toBe(false);
      seen.add(instr.id);
    }
  });

  it("labels have ID -1", () => {
    const { program } = mustCompile("[-]", releaseConfig());
    for (const instr of program.instructions) {
      if (instr.opcode === IrOp.LABEL) {
        expect(instr.id).toBe(-1);
      }
    }
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Error cases
// ──────────────────────────────────────────────────────────────────────────────

describe("error cases", () => {
  it("throws when AST root is not 'program'", () => {
    const fakeAst: ASTNode = {
      ruleName: "not_a_program",
      children: [],
    };
    expect(() => compile(fakeAst, "test.bf", releaseConfig())).toThrow(
      /expected 'program' AST node/
    );
  });

  it("throws when tapeSize is 0", () => {
    const ast = parseBrainfuck("");
    const config = { ...releaseConfig(), tapeSize: 0 };
    expect(() => compile(ast, "test.bf", config)).toThrow(/invalid tapeSize/);
  });

  it("throws when tapeSize is negative", () => {
    const ast = parseBrainfuck("");
    const config = { ...releaseConfig(), tapeSize: -1 };
    expect(() => compile(ast, "test.bf", config)).toThrow(/invalid tapeSize/);
  });
});
