/**
 * Tests for the Brainfuck translator (source -> CodeObject).
 *
 * These tests verify that the translator correctly:
 * 1. Maps each BF character to the right opcode.
 * 2. Ignores non-BF characters (comments).
 * 3. Matches brackets ([ and ]) with correct jump targets.
 * 4. Detects mismatched brackets and raises TranslationError.
 * 5. Always appends a HALT instruction at the end.
 * 6. Produces empty constant and name pools (BF doesn't use them).
 */

import { describe, it, expect } from "vitest";

import { Op } from "../src/opcodes.js";
import { translate, TranslationError } from "../src/translator.js";

// =========================================================================
// Basic Translation
// =========================================================================

describe("BasicTranslation", () => {
  /** Each BF character maps to one instruction. */

  it("translates empty program to just HALT", () => {
    const code = translate("");
    expect(code.instructions.length).toBe(1); // just HALT
    expect(code.instructions[0].opcode).toBe(Op.HALT);
  });

  it("translates single > to RIGHT + HALT", () => {
    const code = translate(">");
    expect(code.instructions[0].opcode).toBe(Op.RIGHT);
    expect(code.instructions[1].opcode).toBe(Op.HALT);
  });

  it("translates single < to LEFT", () => {
    const code = translate("<");
    expect(code.instructions[0].opcode).toBe(Op.LEFT);
  });

  it("translates single + to INC", () => {
    const code = translate("+");
    expect(code.instructions[0].opcode).toBe(Op.INC);
  });

  it("translates single - to DEC", () => {
    const code = translate("-");
    expect(code.instructions[0].opcode).toBe(Op.DEC);
  });

  it("translates single . to OUTPUT", () => {
    const code = translate(".");
    expect(code.instructions[0].opcode).toBe(Op.OUTPUT);
  });

  it("translates single , to INPUT", () => {
    const code = translate(",");
    expect(code.instructions[0].opcode).toBe(Op.INPUT);
  });

  it("translates multiple commands in order", () => {
    const code = translate("+++>.");
    const ops = code.instructions.map((i) => i.opcode);
    expect(ops).toEqual([Op.INC, Op.INC, Op.INC, Op.RIGHT, Op.OUTPUT, Op.HALT]);
  });

  it("ignores non-BF characters (comments)", () => {
    /** Non-BF characters are treated as comments. */
    const code = translate("hello + world - !");
    const ops = code.instructions.map((i) => i.opcode);
    expect(ops).toEqual([Op.INC, Op.DEC, Op.HALT]);
  });

  it("ignores whitespace", () => {
    const code = translate("  +  +  +  ");
    const ops = code.instructions.map((i) => i.opcode);
    expect(ops).toEqual([Op.INC, Op.INC, Op.INC, Op.HALT]);
  });

  it("produces empty constant pool", () => {
    const code = translate("+++");
    expect(code.constants).toEqual([]);
  });

  it("produces empty name pool", () => {
    const code = translate("+++");
    expect(code.names).toEqual([]);
  });
});

// =========================================================================
// Bracket Matching
// =========================================================================

describe("BracketMatching", () => {
  /** [ and ] are matched during translation. */

  it("matches simple loop [>+<-]", () => {
    /** [>+<-] -- the simplest loop. */
    const code = translate("[>+<-]");
    // Instructions: LOOP_START, RIGHT, INC, LEFT, DEC, LOOP_END, HALT
    expect(code.instructions.length).toBe(7);

    const loopStart = code.instructions[0];
    const loopEnd = code.instructions[5];

    expect(loopStart.opcode).toBe(Op.LOOP_START);
    expect(loopStart.operand).toBe(6); // jump past LOOP_END (index 5) to HALT (index 6)

    expect(loopEnd.opcode).toBe(Op.LOOP_END);
    expect(loopEnd.operand).toBe(0); // jump back to LOOP_START
  });

  it("matches nested loops ++[>++[>+<-]<-]", () => {
    /**
     * Instruction layout:
     *     0: INC, 1: INC, 2: LOOP_START(15),
     *     3: RIGHT, 4: INC, 5: INC, 6: LOOP_START(12),
     *     7: RIGHT, 8: INC, 9: LEFT, 10: DEC, 11: LOOP_END(6),
     *     12: LEFT, 13: DEC, 14: LOOP_END(2),
     *     15: HALT
     */
    const code = translate("++[>++[>+<-]<-]");

    // Find the loop instructions
    const loops = code.instructions
      .map((inst, i) => ({ i, inst }))
      .filter(({ inst }) =>
        inst.opcode === Op.LOOP_START || inst.opcode === Op.LOOP_END,
      );

    // Should have 2 LOOP_STARTs and 2 LOOP_ENDs
    expect(loops.length).toBe(4);

    // Outer [ at index 2, inner [ at index 6
    const outerStart = code.instructions[2];
    const innerStart = code.instructions[6];

    expect(outerStart.opcode).toBe(Op.LOOP_START);
    expect(innerStart.opcode).toBe(Op.LOOP_START);

    // Inner ] at index 11, outer ] at index 14
    const innerEnd = code.instructions[11];
    const outerEnd = code.instructions[14];

    // Inner loop: [ at 6 jumps to 12 (past ] at 11), ] at 11 jumps back to 6
    expect(innerStart.operand).toBe(12);
    expect(innerEnd.operand).toBe(6);

    // Outer loop: [ at 2 jumps to 15 (past ] at 14), ] at 14 jumps back to 2
    expect(outerStart.operand).toBe(15);
    expect(outerEnd.operand).toBe(2);
  });

  it("matches empty loop []", () => {
    /** [] -- an empty loop (infinite if cell != 0). */
    const code = translate("[]");
    expect(code.instructions[0].opcode).toBe(Op.LOOP_START);
    expect(code.instructions[0].operand).toBe(2); // past LOOP_END
    expect(code.instructions[1].opcode).toBe(Op.LOOP_END);
    expect(code.instructions[1].operand).toBe(0); // back to LOOP_START
  });

  it("matches adjacent loops [][]", () => {
    /** [][] -- two loops side by side. */
    const code = translate("[][]");
    // First loop: [0] -> LOOP_START(2), [1] -> LOOP_END(0)
    // Second loop: [2] -> LOOP_START(4), [3] -> LOOP_END(2)
    expect(code.instructions[0].operand).toBe(2);
    expect(code.instructions[1].operand).toBe(0);
    expect(code.instructions[2].operand).toBe(4);
    expect(code.instructions[3].operand).toBe(2);
  });
});

// =========================================================================
// Bracket Errors
// =========================================================================

describe("BracketErrors", () => {
  /** Mismatched brackets are caught during translation. */

  it("detects unmatched open bracket", () => {
    expect(() => translate("[")).toThrow(TranslationError);
    expect(() => translate("[")).toThrow(/Unmatched '\['/);
  });

  it("detects unmatched close bracket", () => {
    expect(() => translate("]")).toThrow(TranslationError);
    expect(() => translate("]")).toThrow(/Unmatched '\]'/);
  });

  it("detects extra open bracket", () => {
    expect(() => translate("[[]")).toThrow(TranslationError);
    expect(() => translate("[[]")).toThrow(/Unmatched '\['/);
  });

  it("detects extra close bracket", () => {
    expect(() => translate("[]]")).toThrow(TranslationError);
    expect(() => translate("[]]")).toThrow(/Unmatched '\]'/);
  });

  it("detects multiple unmatched open brackets", () => {
    expect(() => translate("[[")).toThrow(TranslationError);
    expect(() => translate("[[")).toThrow(/2 unclosed/);
  });
});
