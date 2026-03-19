/**
 * Tests for the instruction encoder.
 *
 * These tests verify that individual encoding functions produce the correct
 * 32-bit ARM instruction words. We compare against known ARM encodings
 * that match the Python ARM simulator's test suite.
 *
 * === How to read ARM instruction hex values ===
 *
 * ARM instructions are 32-bit values often written in hexadecimal:
 *
 *   0xE3A00001 = MOV R0, #1
 *
 * Breaking it down:
 *   E = 1110 = condition AL (always)
 *   3 = 0011 = 00 (data processing) + I=1 (immediate)
 *   A = 1010 = opcode 1101 (MOV) upper bit + S=0
 *   0 = 0000 = opcode lower bits + Rn upper bits
 *   0 = 0000 = Rn=0000
 *   0 = 0000 = Rd=0000
 *   0 = 0000 = rotate=0000
 *   1 = 0001 = imm8=00000001
 *
 * Wait, that doesn't look right at the nibble level. Let's be precise
 * with the full 32-bit binary:
 *
 *   1110 00 1 1101 0 0000 0000 0000 00000001
 *   cond    I opco S  Rn   Rd  rot   imm8
 *
 *   = 0xE3A00001
 */

import { describe, it, expect } from "vitest";
import {
  encodeDataProcessing,
  encodeBranch,
  encodeMemory,
  encodeMovImm,
  encodeAdd,
  encodeSub,
  encodeHlt,
  encodeImmediate,
  instructionsToBytes,
} from "../src/encoder.js";
import { CONDITION_CODES, OPCODES, HLT_INSTRUCTION } from "../src/types.js";

// ---------------------------------------------------------------------------
// Convenience encoder tests (matching Python ARM simulator test suite)
// ---------------------------------------------------------------------------

describe("Convenience encoders", () => {
  describe("encodeMovImm", () => {
    it("should encode MOV R0, #1 to 0xE3A00001", () => {
      // This is the canonical test from the Python ARM simulator:
      // cond=1110 00 I=1 opcode=1101 S=0 Rn=0000 Rd=0000 imm=00000001
      expect(encodeMovImm(0, 1)).toBe(0xE3A00001);
    });

    it("should encode MOV R1, #2 to 0xE3A01002", () => {
      expect(encodeMovImm(1, 2)).toBe(0xE3A01002);
    });

    it("should encode MOV R0, #42 correctly", () => {
      const word = encodeMovImm(0, 42);
      // Verify by extracting fields
      expect((word >>> 28) & 0xF).toBe(0b1110);  // cond = AL
      expect((word >>> 25) & 0x1).toBe(1);         // I = 1 (immediate)
      expect((word >>> 21) & 0xF).toBe(0b1101);   // opcode = MOV
      expect((word >>> 12) & 0xF).toBe(0);          // Rd = R0
      expect(word & 0xFF).toBe(42);                  // imm8 = 42
    });

    it("should encode MOV R15, #0 (writing to PC)", () => {
      const word = encodeMovImm(15, 0);
      expect((word >>> 12) & 0xF).toBe(15);  // Rd = R15
      expect(word & 0xFF).toBe(0);             // imm8 = 0
    });

    it("should encode MOV R0, #255 (maximum 8-bit immediate)", () => {
      const word = encodeMovImm(0, 255);
      expect(word & 0xFF).toBe(255);
    });
  });

  describe("encodeAdd", () => {
    it("should encode ADD R2, R0, R1 to 0xE0802001", () => {
      // cond=1110 00 I=0 opcode=0100 S=0 Rn=0000 Rd=0010 Rm=0001
      expect(encodeAdd(2, 0, 1)).toBe(0xE0802001);
    });

    it("should encode ADD R0, R0, R0 (self-add)", () => {
      const word = encodeAdd(0, 0, 0);
      expect((word >>> 21) & 0xF).toBe(0b0100);  // opcode = ADD
      expect((word >>> 16) & 0xF).toBe(0);         // Rn = R0
      expect((word >>> 12) & 0xF).toBe(0);         // Rd = R0
      expect(word & 0xF).toBe(0);                   // Rm = R0
    });
  });

  describe("encodeSub", () => {
    it("should encode SUB R2, R0, R1 to 0xE0402001", () => {
      // cond=1110 00 I=0 opcode=0010 S=0 Rn=0000 Rd=0010 Rm=0001
      expect(encodeSub(2, 0, 1)).toBe(0xE0402001);
    });
  });

  describe("encodeHlt", () => {
    it("should encode HLT to 0xFFFFFFFF", () => {
      expect(encodeHlt()).toBe(0xFFFFFFFF);
    });

    it("should match HLT_INSTRUCTION constant", () => {
      expect(encodeHlt()).toBe(HLT_INSTRUCTION);
    });
  });
});

// ---------------------------------------------------------------------------
// Data processing encoder tests
// ---------------------------------------------------------------------------

describe("encodeDataProcessing", () => {
  it("should place condition code in bits [31:28]", () => {
    // Use condition EQ (0b0000) — all other bits default
    const word = encodeDataProcessing(0b0000, 0, false, 0, 0, 0, false);
    expect((word >>> 28) & 0xF).toBe(0b0000);
  });

  it("should set I bit (bit 25) when immediate=true", () => {
    const wordImm = encodeDataProcessing(0b1110, 0, false, 0, 0, 1, true);
    expect((wordImm >>> 25) & 0x1).toBe(1);

    const wordReg = encodeDataProcessing(0b1110, 0, false, 0, 0, 1, false);
    expect((wordReg >>> 25) & 0x1).toBe(0);
  });

  it("should set S bit (bit 20) when s=true", () => {
    const wordS = encodeDataProcessing(0b1110, 0, true, 0, 0, 0, false);
    expect((wordS >>> 20) & 0x1).toBe(1);

    const wordNoS = encodeDataProcessing(0b1110, 0, false, 0, 0, 0, false);
    expect((wordNoS >>> 20) & 0x1).toBe(0);
  });

  it("should place opcode in bits [24:21]", () => {
    // Test with ADD opcode (0b0100)
    const word = encodeDataProcessing(0b1110, 0b0100, false, 0, 0, 0, false);
    expect((word >>> 21) & 0xF).toBe(0b0100);
  });

  it("should place Rn in bits [19:16]", () => {
    const word = encodeDataProcessing(0b1110, 0, false, 5, 0, 0, false);
    expect((word >>> 16) & 0xF).toBe(5);
  });

  it("should place Rd in bits [15:12]", () => {
    const word = encodeDataProcessing(0b1110, 0, false, 0, 7, 0, false);
    expect((word >>> 12) & 0xF).toBe(7);
  });

  it("should place operand2 in bits [11:0]", () => {
    const word = encodeDataProcessing(0b1110, 0, false, 0, 0, 0xABC, false);
    expect(word & 0xFFF).toBe(0xABC);
  });

  it("should encode all data processing opcodes correctly", () => {
    // Verify each opcode value is placed correctly
    const opcodeTests: [string, number][] = [
      ["AND", 0b0000],
      ["EOR", 0b0001],
      ["SUB", 0b0010],
      ["ADD", 0b0100],
      ["CMP", 0b1010],
      ["ORR", 0b1100],
      ["MOV", 0b1101],
      ["MVN", 0b1111],
    ];

    for (const [name, expected] of opcodeTests) {
      const opcode = OPCODES.get(name)!;
      expect(opcode).toBe(expected);
    }
  });
});

// ---------------------------------------------------------------------------
// Branch encoder tests
// ---------------------------------------------------------------------------

describe("encodeBranch", () => {
  it("should set bits [27:25] to 101 (branch identifier)", () => {
    const word = encodeBranch(0b1110, 0);
    expect((word >>> 25) & 0b111).toBe(0b101);
  });

  it("should set L bit (bit 24) for BL (branch with link)", () => {
    const wordB = encodeBranch(0b1110, 0, false);
    expect((wordB >>> 24) & 0x1).toBe(0);

    const wordBL = encodeBranch(0b1110, 0, true);
    expect((wordBL >>> 24) & 0x1).toBe(1);
  });

  it("should encode positive offset correctly", () => {
    const word = encodeBranch(0b1110, 5);
    expect(word & 0x00FFFFFF).toBe(5);
  });

  it("should encode negative offset in two's complement", () => {
    // Offset -2 should be encoded as 24-bit two's complement
    const word = encodeBranch(0b1110, -2);
    expect(word & 0x00FFFFFF).toBe(0xFFFFFE);
  });

  it("should place condition code in bits [31:28]", () => {
    // BEQ: condition = EQ (0b0000)
    const word = encodeBranch(0b0000, 0);
    expect((word >>> 28) & 0xF).toBe(0b0000);
  });
});

// ---------------------------------------------------------------------------
// Memory encoder tests
// ---------------------------------------------------------------------------

describe("encodeMemory", () => {
  it("should set bits [27:26] to 01 (memory access identifier)", () => {
    const word = encodeMemory(0b1110, true, 0, 0, 0);
    expect((word >>> 26) & 0b11).toBe(0b01);
  });

  it("should set L bit (bit 20) for LDR", () => {
    const ldr = encodeMemory(0b1110, true, 0, 0, 0);
    expect((ldr >>> 20) & 0x1).toBe(1);
  });

  it("should clear L bit (bit 20) for STR", () => {
    const str = encodeMemory(0b1110, false, 0, 0, 0);
    expect((str >>> 20) & 0x1).toBe(0);
  });

  it("should set U bit (bit 23) for positive offset", () => {
    const word = encodeMemory(0b1110, true, 1, 0, 4);
    expect((word >>> 23) & 0x1).toBe(1);
    expect(word & 0xFFF).toBe(4);
  });

  it("should clear U bit (bit 23) for negative offset", () => {
    const word = encodeMemory(0b1110, true, 1, 0, -4);
    expect((word >>> 23) & 0x1).toBe(0);
    expect(word & 0xFFF).toBe(4);  // absolute value
  });

  it("should place Rn in bits [19:16] and Rd in bits [15:12]", () => {
    const word = encodeMemory(0b1110, true, 3, 7, 0);
    expect((word >>> 16) & 0xF).toBe(3);  // Rn = R3
    expect((word >>> 12) & 0xF).toBe(7);  // Rd = R7
  });
});

// ---------------------------------------------------------------------------
// Immediate encoding tests
// ---------------------------------------------------------------------------

describe("encodeImmediate", () => {
  it("should encode small values (0-255) with no rotation", () => {
    // Values 0-255 fit directly in imm8 with rotate=0
    expect(encodeImmediate(0)).toBe(0);
    expect(encodeImmediate(1)).toBe(1);
    expect(encodeImmediate(42)).toBe(42);
    expect(encodeImmediate(255)).toBe(255);
  });

  it("should encode 0xFF (255) as rotate=0, imm8=0xFF", () => {
    const encoded = encodeImmediate(0xFF)!;
    const rotate = (encoded >> 8) & 0xF;
    const imm8 = encoded & 0xFF;
    expect(rotate).toBe(0);
    expect(imm8).toBe(0xFF);
  });

  it("should return null for values that cannot be encoded", () => {
    // 0x101 = 257, which is 100000001 in binary — more than 8 bits,
    // and no rotation produces an 8-bit value
    expect(encodeImmediate(0x101)).toBeNull();
  });

  it("should return null for 0x1FF (511)", () => {
    // 511 = 111111111 in binary (9 bits) — can't be represented
    expect(encodeImmediate(0x1FF)).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// instructionsToBytes tests
// ---------------------------------------------------------------------------

describe("instructionsToBytes", () => {
  it("should convert a single instruction to 4 little-endian bytes", () => {
    // 0xE3A00001 in little-endian: [0x01, 0x00, 0xA0, 0xE3]
    const bytes = instructionsToBytes([0xE3A00001]);
    expect(bytes.length).toBe(4);
    expect(bytes[0]).toBe(0x01);
    expect(bytes[1]).toBe(0x00);
    expect(bytes[2]).toBe(0xA0);
    expect(bytes[3]).toBe(0xE3);
  });

  it("should convert multiple instructions", () => {
    const bytes = instructionsToBytes([
      encodeMovImm(0, 1),   // MOV R0, #1
      encodeMovImm(1, 2),   // MOV R1, #2
      encodeAdd(2, 0, 1),   // ADD R2, R0, R1
      encodeHlt(),           // HLT
    ]);
    expect(bytes.length).toBe(16);  // 4 instructions * 4 bytes each
  });

  it("should handle empty input", () => {
    const bytes = instructionsToBytes([]);
    expect(bytes.length).toBe(0);
  });

  it("should handle HLT instruction (0xFFFFFFFF)", () => {
    const bytes = instructionsToBytes([0xFFFFFFFF]);
    expect(bytes[0]).toBe(0xFF);
    expect(bytes[1]).toBe(0xFF);
    expect(bytes[2]).toBe(0xFF);
    expect(bytes[3]).toBe(0xFF);
  });
});

// ---------------------------------------------------------------------------
// Condition code constant tests
// ---------------------------------------------------------------------------

describe("CONDITION_CODES", () => {
  it("should have AL (always) = 0b1110", () => {
    expect(CONDITION_CODES.get("AL")).toBe(0b1110);
  });

  it("should have EQ (equal) = 0b0000", () => {
    expect(CONDITION_CODES.get("EQ")).toBe(0b0000);
  });

  it("should have NE (not equal) = 0b0001", () => {
    expect(CONDITION_CODES.get("NE")).toBe(0b0001);
  });

  it("should have all 15 condition codes", () => {
    // 15 unique names (CS/HS and CC/LO are aliases)
    expect(CONDITION_CODES.size).toBe(17);
  });
});
