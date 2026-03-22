/**
 * Tests for Starlark Opcodes -- verifying the instruction set definitions.
 *
 * These tests ensure that all 46 opcodes are defined with the correct hex
 * values and that the operator-to-opcode mapping tables are complete.
 */

import { describe, it, expect } from "vitest";
import {
  Op,
  BINARY_OP_MAP,
  COMPARE_OP_MAP,
  AUGMENTED_ASSIGN_MAP,
  UNARY_OP_MAP,
} from "../src/opcodes.js";

describe("Op (Starlark Opcodes)", () => {
  // =====================================================================
  // Stack Operations
  // =====================================================================

  it("defines LOAD_CONST as 0x01", () => {
    expect(Op.LOAD_CONST).toBe(0x01);
  });

  it("defines POP as 0x02", () => {
    expect(Op.POP).toBe(0x02);
  });

  it("defines DUP as 0x03", () => {
    expect(Op.DUP).toBe(0x03);
  });

  it("defines LOAD_NONE as 0x04", () => {
    expect(Op.LOAD_NONE).toBe(0x04);
  });

  it("defines LOAD_TRUE as 0x05", () => {
    expect(Op.LOAD_TRUE).toBe(0x05);
  });

  it("defines LOAD_FALSE as 0x06", () => {
    expect(Op.LOAD_FALSE).toBe(0x06);
  });

  // =====================================================================
  // Variable Operations
  // =====================================================================

  it("defines STORE_NAME as 0x10", () => {
    expect(Op.STORE_NAME).toBe(0x10);
  });

  it("defines LOAD_NAME as 0x11", () => {
    expect(Op.LOAD_NAME).toBe(0x11);
  });

  it("defines STORE_LOCAL as 0x12", () => {
    expect(Op.STORE_LOCAL).toBe(0x12);
  });

  it("defines LOAD_LOCAL as 0x13", () => {
    expect(Op.LOAD_LOCAL).toBe(0x13);
  });

  it("defines STORE_CLOSURE as 0x14", () => {
    expect(Op.STORE_CLOSURE).toBe(0x14);
  });

  it("defines LOAD_CLOSURE as 0x15", () => {
    expect(Op.LOAD_CLOSURE).toBe(0x15);
  });

  // =====================================================================
  // Arithmetic Operations
  // =====================================================================

  it("defines ADD as 0x20", () => {
    expect(Op.ADD).toBe(0x20);
  });

  it("defines SUB as 0x21", () => {
    expect(Op.SUB).toBe(0x21);
  });

  it("defines MUL as 0x22", () => {
    expect(Op.MUL).toBe(0x22);
  });

  it("defines DIV as 0x23", () => {
    expect(Op.DIV).toBe(0x23);
  });

  it("defines FLOOR_DIV as 0x24", () => {
    expect(Op.FLOOR_DIV).toBe(0x24);
  });

  it("defines MOD as 0x25", () => {
    expect(Op.MOD).toBe(0x25);
  });

  it("defines POWER as 0x26", () => {
    expect(Op.POWER).toBe(0x26);
  });

  it("defines NEGATE as 0x27", () => {
    expect(Op.NEGATE).toBe(0x27);
  });

  it("defines BIT_AND as 0x28", () => {
    expect(Op.BIT_AND).toBe(0x28);
  });

  it("defines BIT_OR as 0x29", () => {
    expect(Op.BIT_OR).toBe(0x29);
  });

  it("defines BIT_XOR as 0x2A", () => {
    expect(Op.BIT_XOR).toBe(0x2a);
  });

  it("defines BIT_NOT as 0x2B", () => {
    expect(Op.BIT_NOT).toBe(0x2b);
  });

  it("defines LSHIFT as 0x2C", () => {
    expect(Op.LSHIFT).toBe(0x2c);
  });

  it("defines RSHIFT as 0x2D", () => {
    expect(Op.RSHIFT).toBe(0x2d);
  });

  // =====================================================================
  // Comparison Operations
  // =====================================================================

  it("defines CMP_EQ as 0x30", () => {
    expect(Op.CMP_EQ).toBe(0x30);
  });

  it("defines CMP_NE as 0x31", () => {
    expect(Op.CMP_NE).toBe(0x31);
  });

  it("defines CMP_LT as 0x32", () => {
    expect(Op.CMP_LT).toBe(0x32);
  });

  it("defines CMP_GT as 0x33", () => {
    expect(Op.CMP_GT).toBe(0x33);
  });

  it("defines CMP_LE as 0x34", () => {
    expect(Op.CMP_LE).toBe(0x34);
  });

  it("defines CMP_GE as 0x35", () => {
    expect(Op.CMP_GE).toBe(0x35);
  });

  it("defines CMP_IN as 0x36", () => {
    expect(Op.CMP_IN).toBe(0x36);
  });

  it("defines CMP_NOT_IN as 0x37", () => {
    expect(Op.CMP_NOT_IN).toBe(0x37);
  });

  it("defines NOT as 0x38", () => {
    expect(Op.NOT).toBe(0x38);
  });

  // =====================================================================
  // Control Flow
  // =====================================================================

  it("defines JUMP as 0x40", () => {
    expect(Op.JUMP).toBe(0x40);
  });

  it("defines JUMP_IF_FALSE as 0x41", () => {
    expect(Op.JUMP_IF_FALSE).toBe(0x41);
  });

  it("defines JUMP_IF_TRUE as 0x42", () => {
    expect(Op.JUMP_IF_TRUE).toBe(0x42);
  });

  it("defines JUMP_IF_FALSE_OR_POP as 0x43", () => {
    expect(Op.JUMP_IF_FALSE_OR_POP).toBe(0x43);
  });

  it("defines JUMP_IF_TRUE_OR_POP as 0x44", () => {
    expect(Op.JUMP_IF_TRUE_OR_POP).toBe(0x44);
  });

  // =====================================================================
  // Function Operations
  // =====================================================================

  it("defines MAKE_FUNCTION as 0x50", () => {
    expect(Op.MAKE_FUNCTION).toBe(0x50);
  });

  it("defines CALL_FUNCTION as 0x51", () => {
    expect(Op.CALL_FUNCTION).toBe(0x51);
  });

  it("defines CALL_FUNCTION_KW as 0x52", () => {
    expect(Op.CALL_FUNCTION_KW).toBe(0x52);
  });

  it("defines RETURN as 0x53", () => {
    expect(Op.RETURN).toBe(0x53);
  });

  // =====================================================================
  // Collection Operations
  // =====================================================================

  it("defines BUILD_LIST as 0x60", () => {
    expect(Op.BUILD_LIST).toBe(0x60);
  });

  it("defines BUILD_DICT as 0x61", () => {
    expect(Op.BUILD_DICT).toBe(0x61);
  });

  it("defines BUILD_TUPLE as 0x62", () => {
    expect(Op.BUILD_TUPLE).toBe(0x62);
  });

  it("defines LIST_APPEND as 0x63", () => {
    expect(Op.LIST_APPEND).toBe(0x63);
  });

  it("defines DICT_SET as 0x64", () => {
    expect(Op.DICT_SET).toBe(0x64);
  });

  // =====================================================================
  // Subscript & Attribute Operations
  // =====================================================================

  it("defines LOAD_SUBSCRIPT as 0x70", () => {
    expect(Op.LOAD_SUBSCRIPT).toBe(0x70);
  });

  it("defines STORE_SUBSCRIPT as 0x71", () => {
    expect(Op.STORE_SUBSCRIPT).toBe(0x71);
  });

  it("defines LOAD_ATTR as 0x72", () => {
    expect(Op.LOAD_ATTR).toBe(0x72);
  });

  it("defines STORE_ATTR as 0x73", () => {
    expect(Op.STORE_ATTR).toBe(0x73);
  });

  it("defines LOAD_SLICE as 0x74", () => {
    expect(Op.LOAD_SLICE).toBe(0x74);
  });

  // =====================================================================
  // Iteration Operations
  // =====================================================================

  it("defines GET_ITER as 0x80", () => {
    expect(Op.GET_ITER).toBe(0x80);
  });

  it("defines FOR_ITER as 0x81", () => {
    expect(Op.FOR_ITER).toBe(0x81);
  });

  it("defines UNPACK_SEQUENCE as 0x82", () => {
    expect(Op.UNPACK_SEQUENCE).toBe(0x82);
  });

  // =====================================================================
  // Module Operations
  // =====================================================================

  it("defines LOAD_MODULE as 0x90", () => {
    expect(Op.LOAD_MODULE).toBe(0x90);
  });

  it("defines IMPORT_FROM as 0x91", () => {
    expect(Op.IMPORT_FROM).toBe(0x91);
  });

  // =====================================================================
  // I/O Operations
  // =====================================================================

  it("defines PRINT as 0xA0", () => {
    expect(Op.PRINT).toBe(0xa0);
  });

  // =====================================================================
  // VM Control
  // =====================================================================

  it("defines HALT as 0xFF", () => {
    expect(Op.HALT).toBe(0xff);
  });

  // =====================================================================
  // Total count
  // =====================================================================

  it("has exactly 61 opcodes", () => {
    // 6 stack + 6 variable + 14 arithmetic + 9 comparison/boolean +
    // 5 control + 4 function + 5 collection + 5 subscript/attr +
    // 3 iteration + 2 module + 1 I/O + 1 VM control = 61
    const opcodeCount = Object.keys(Op).length;
    expect(opcodeCount).toBe(61);
  });
});

describe("BINARY_OP_MAP", () => {
  it("maps all 12 binary operators", () => {
    expect(Object.keys(BINARY_OP_MAP)).toHaveLength(12);
  });

  it("maps + to ADD", () => {
    expect(BINARY_OP_MAP["+"]).toBe(Op.ADD);
  });

  it("maps - to SUB", () => {
    expect(BINARY_OP_MAP["-"]).toBe(Op.SUB);
  });

  it("maps * to MUL", () => {
    expect(BINARY_OP_MAP["*"]).toBe(Op.MUL);
  });

  it("maps / to DIV", () => {
    expect(BINARY_OP_MAP["/"]).toBe(Op.DIV);
  });

  it("maps // to FLOOR_DIV", () => {
    expect(BINARY_OP_MAP["//"]).toBe(Op.FLOOR_DIV);
  });

  it("maps % to MOD", () => {
    expect(BINARY_OP_MAP["%"]).toBe(Op.MOD);
  });

  it("maps ** to POWER", () => {
    expect(BINARY_OP_MAP["**"]).toBe(Op.POWER);
  });

  it("maps bitwise operators", () => {
    expect(BINARY_OP_MAP["&"]).toBe(Op.BIT_AND);
    expect(BINARY_OP_MAP["|"]).toBe(Op.BIT_OR);
    expect(BINARY_OP_MAP["^"]).toBe(Op.BIT_XOR);
    expect(BINARY_OP_MAP["<<"]).toBe(Op.LSHIFT);
    expect(BINARY_OP_MAP[">>"]).toBe(Op.RSHIFT);
  });
});

describe("COMPARE_OP_MAP", () => {
  it("maps all 8 comparison operators", () => {
    expect(Object.keys(COMPARE_OP_MAP)).toHaveLength(8);
  });

  it("maps == to CMP_EQ", () => {
    expect(COMPARE_OP_MAP["=="]).toBe(Op.CMP_EQ);
  });

  it("maps != to CMP_NE", () => {
    expect(COMPARE_OP_MAP["!="]).toBe(Op.CMP_NE);
  });

  it("maps < > <= >= correctly", () => {
    expect(COMPARE_OP_MAP["<"]).toBe(Op.CMP_LT);
    expect(COMPARE_OP_MAP[">"]).toBe(Op.CMP_GT);
    expect(COMPARE_OP_MAP["<="]).toBe(Op.CMP_LE);
    expect(COMPARE_OP_MAP[">="]).toBe(Op.CMP_GE);
  });

  it("maps in and not in", () => {
    expect(COMPARE_OP_MAP["in"]).toBe(Op.CMP_IN);
    expect(COMPARE_OP_MAP["not in"]).toBe(Op.CMP_NOT_IN);
  });
});

describe("AUGMENTED_ASSIGN_MAP", () => {
  it("maps all 12 augmented assignment operators", () => {
    expect(Object.keys(AUGMENTED_ASSIGN_MAP)).toHaveLength(12);
  });

  it("maps += to ADD", () => {
    expect(AUGMENTED_ASSIGN_MAP["+="]).toBe(Op.ADD);
  });

  it("maps -= to SUB", () => {
    expect(AUGMENTED_ASSIGN_MAP["-="]).toBe(Op.SUB);
  });

  it("maps **= to POWER", () => {
    expect(AUGMENTED_ASSIGN_MAP["**="]).toBe(Op.POWER);
  });
});

describe("UNARY_OP_MAP", () => {
  it("maps all 3 unary operators", () => {
    expect(Object.keys(UNARY_OP_MAP)).toHaveLength(3);
  });

  it("maps - to NEGATE", () => {
    expect(UNARY_OP_MAP["-"]).toBe(Op.NEGATE);
  });

  it("maps + to POP (no-op)", () => {
    expect(UNARY_OP_MAP["+"]).toBe(Op.POP);
  });

  it("maps ~ to BIT_NOT", () => {
    expect(UNARY_OP_MAP["~"]).toBe(Op.BIT_NOT);
  });
});
