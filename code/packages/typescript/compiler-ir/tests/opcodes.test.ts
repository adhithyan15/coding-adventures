/**
 * Tests for IR opcodes — IrOp enum, opToString(), parseOp()
 *
 * These tests verify that:
 *   1. Every opcode has a canonical text name
 *   2. opToString / parseOp form a perfect roundtrip
 *   3. Unknown names return undefined from parseOp
 *   4. All 25 opcodes are present
 */

import { describe, it, expect } from "vitest";
import { IrOp, opToString, parseOp } from "../src/opcodes.js";

// The complete set of (enum value, canonical name) pairs.
// If a new opcode is added, this table must be updated — the test acts as
// a regression guard that forces the author to also update documentation.
const ALL_OPCODES: Array<[IrOp, string]> = [
  [IrOp.LOAD_IMM, "LOAD_IMM"],
  [IrOp.LOAD_ADDR, "LOAD_ADDR"],
  [IrOp.LOAD_BYTE, "LOAD_BYTE"],
  [IrOp.STORE_BYTE, "STORE_BYTE"],
  [IrOp.LOAD_WORD, "LOAD_WORD"],
  [IrOp.STORE_WORD, "STORE_WORD"],
  [IrOp.ADD, "ADD"],
  [IrOp.ADD_IMM, "ADD_IMM"],
  [IrOp.SUB, "SUB"],
  [IrOp.AND, "AND"],
  [IrOp.AND_IMM, "AND_IMM"],
  [IrOp.CMP_EQ, "CMP_EQ"],
  [IrOp.CMP_NE, "CMP_NE"],
  [IrOp.CMP_LT, "CMP_LT"],
  [IrOp.CMP_GT, "CMP_GT"],
  [IrOp.LABEL, "LABEL"],
  [IrOp.JUMP, "JUMP"],
  [IrOp.BRANCH_Z, "BRANCH_Z"],
  [IrOp.BRANCH_NZ, "BRANCH_NZ"],
  [IrOp.CALL, "CALL"],
  [IrOp.RET, "RET"],
  [IrOp.SYSCALL, "SYSCALL"],
  [IrOp.HALT, "HALT"],
  [IrOp.NOP, "NOP"],
  [IrOp.COMMENT, "COMMENT"],
];

describe("IrOp enum", () => {
  it("has exactly 25 opcodes", () => {
    /**
     * The opcode count is a commitment — changing it requires updating the
     * spec and all downstream packages. This test is a speed bump that
     * forces deliberate additions.
     */
    expect(ALL_OPCODES.length).toBe(25);
  });

  it("starts at 0 (LOAD_IMM = 0)", () => {
    /**
     * The first opcode is 0 by convention, matching the Go implementation.
     * This ensures text roundtrips are deterministic.
     */
    expect(IrOp.LOAD_IMM).toBe(0);
  });

  it("HALT is 22", () => {
    expect(IrOp.HALT).toBe(22);
  });

  it("COMMENT is 24 (last opcode in v1)", () => {
    expect(IrOp.COMMENT).toBe(24);
  });
});

describe("opToString", () => {
  it("returns the canonical name for every opcode", () => {
    for (const [op, name] of ALL_OPCODES) {
      expect(opToString(op)).toBe(name);
    }
  });

  it("returns UNKNOWN for an unrecognised opcode value", () => {
    /**
     * Casting a bogus number to IrOp should not crash — it returns "UNKNOWN".
     * This prevents crashes in debug-printing code that might receive a
     * future opcode from a newer IR version.
     */
    expect(opToString(999 as IrOp)).toBe("UNKNOWN");
  });
});

describe("parseOp", () => {
  it("returns the opcode for every canonical name", () => {
    for (const [op, name] of ALL_OPCODES) {
      expect(parseOp(name)).toBe(op);
    }
  });

  it("returns undefined for unknown names", () => {
    expect(parseOp("BOGUS")).toBeUndefined();
    expect(parseOp("")).toBeUndefined();
    expect(parseOp("load_imm")).toBeUndefined(); // case-sensitive
    expect(parseOp("HALT2")).toBeUndefined();
  });

  it("is case-sensitive (lowercase names are not recognised)", () => {
    /**
     * The canonical format uses ALL_CAPS. Lowercase should not match.
     * This prevents ambiguous roundtrips.
     */
    expect(parseOp("halt")).toBeUndefined();
    expect(parseOp("add_imm")).toBeUndefined();
  });
});

describe("opToString / parseOp roundtrip", () => {
  it("opToString → parseOp is identity for all opcodes", () => {
    /**
     * For every opcode op:
     *   parseOp(opToString(op)) === op
     *
     * This ensures the printer and parser agree on every canonical name.
     */
    for (const [op] of ALL_OPCODES) {
      expect(parseOp(opToString(op))).toBe(op);
    }
  });

  it("parseOp → opToString is identity for all names", () => {
    /**
     * For every canonical name:
     *   opToString(parseOp(name)!) === name
     */
    for (const [, name] of ALL_OPCODES) {
      const op = parseOp(name);
      expect(op).toBeDefined();
      expect(opToString(op!)).toBe(name);
    }
  });
});
