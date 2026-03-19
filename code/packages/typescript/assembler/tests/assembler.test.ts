/**
 * Tests for the two-pass assembler.
 *
 * These tests verify the full assembly pipeline: source text → machine code.
 * They cover:
 *
 *   1. Single instructions (verifying encoding matches known ARM values)
 *   2. Multi-instruction programs
 *   3. Labels and forward references
 *   4. Branch offset calculation
 *   5. Error reporting (undefined labels, invalid syntax, etc.)
 *   6. Symbol table and source map generation
 *   7. End-to-end assembly matching the Python ARM simulator test suite
 */

import { describe, it, expect } from "vitest";
import { Assembler, assemble } from "../src/assembler.js";
import { encodeMovImm, encodeAdd, encodeSub, encodeHlt, instructionsToBytes } from "../src/encoder.js";

// ---------------------------------------------------------------------------
// Helper: extract a 32-bit word from assembled output at a given address
// ---------------------------------------------------------------------------

/**
 * Read a 32-bit little-endian word from a Uint8Array at the given byte offset.
 * This is the inverse of what the assembler does when writing machine code.
 */
function readWord(bytes: Uint8Array, offset: number): number {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return view.getUint32(offset, true /* littleEndian */);
}

// ---------------------------------------------------------------------------
// Single instruction assembly
// ---------------------------------------------------------------------------

describe("Single instruction assembly", () => {
  it("should assemble MOV R0, #1", () => {
    const result = assemble("MOV R0, #1\n");
    expect(result.errors).toEqual([]);
    expect(result.machineCode.length).toBe(4);
    expect(readWord(result.machineCode, 0)).toBe(0xE3A00001);
  });

  it("should assemble MOV R1, #2", () => {
    const result = assemble("MOV R1, #2\n");
    expect(readWord(result.machineCode, 0)).toBe(0xE3A01002);
  });

  it("should assemble ADD R2, R0, R1", () => {
    const result = assemble("ADD R2, R0, R1\n");
    expect(result.errors).toEqual([]);
    expect(readWord(result.machineCode, 0)).toBe(0xE0802001);
  });

  it("should assemble SUB R2, R0, R1", () => {
    const result = assemble("SUB R2, R0, R1\n");
    expect(result.errors).toEqual([]);
    expect(readWord(result.machineCode, 0)).toBe(0xE0402001);
  });

  it("should assemble HLT", () => {
    const result = assemble("HLT\n");
    expect(result.errors).toEqual([]);
    expect(readWord(result.machineCode, 0)).toBe(0xFFFFFFFF);
  });

  it("should assemble NOP as MOV R0, R0", () => {
    const result = assemble("NOP\n");
    expect(result.errors).toEqual([]);
    expect(readWord(result.machineCode, 0)).toBe(0xE1A00000);
  });

  it("should assemble CMP R0, #10", () => {
    const result = assemble("CMP R0, #10\n");
    expect(result.errors).toEqual([]);
    // CMP: cond=1110, opcode=1010, S=1, Rn=R0, Rd=0, I=1, imm=10
    const word = readWord(result.machineCode, 0);
    expect((word >>> 28) & 0xF).toBe(0b1110);  // AL
    expect((word >>> 21) & 0xF).toBe(0b1010);  // CMP
    expect((word >>> 20) & 0x1).toBe(1);         // S=1
    expect((word >>> 25) & 0x1).toBe(1);         // I=1
    expect(word & 0xFF).toBe(10);                 // imm8 = 10
  });

  it("should assemble MOV with immediate #0", () => {
    const result = assemble("MOV R0, #0\n");
    expect(result.errors).toEqual([]);
    const word = readWord(result.machineCode, 0);
    expect(word & 0xFF).toBe(0);
  });

  it("should assemble MOV with immediate #255", () => {
    const result = assemble("MOV R0, #255\n");
    expect(result.errors).toEqual([]);
    const word = readWord(result.machineCode, 0);
    expect(word & 0xFF).toBe(255);
  });

  it("should assemble AND R0, R1, R2", () => {
    const result = assemble("AND R0, R1, R2\n");
    expect(result.errors).toEqual([]);
    const word = readWord(result.machineCode, 0);
    expect((word >>> 21) & 0xF).toBe(0b0000);  // AND opcode
  });

  it("should assemble ORR R0, R1, R2", () => {
    const result = assemble("ORR R0, R1, R2\n");
    expect(result.errors).toEqual([]);
    const word = readWord(result.machineCode, 0);
    expect((word >>> 21) & 0xF).toBe(0b1100);  // ORR opcode
  });

  it("should assemble EOR R0, R1, R2", () => {
    const result = assemble("EOR R0, R1, R2\n");
    expect(result.errors).toEqual([]);
    const word = readWord(result.machineCode, 0);
    expect((word >>> 21) & 0xF).toBe(0b0001);  // EOR opcode
  });

  it("should assemble MVN R0, R1", () => {
    const result = assemble("MVN R0, R1\n");
    expect(result.errors).toEqual([]);
    const word = readWord(result.machineCode, 0);
    expect((word >>> 21) & 0xF).toBe(0b1111);  // MVN opcode
  });
});

// ---------------------------------------------------------------------------
// Multi-instruction programs (matching Python ARM simulator tests)
// ---------------------------------------------------------------------------

describe("Multi-instruction programs", () => {
  it("should assemble x = 1 + 2 (the canonical test program)", () => {
    // This matches the Python ARM simulator's test_x_equals_1_plus_2:
    //   MOV R0, #1; MOV R1, #2; ADD R2, R0, R1; HLT
    const result = assemble(
      "MOV R0, #1\nMOV R1, #2\nADD R2, R0, R1\nHLT\n",
    );

    expect(result.errors).toEqual([]);
    expect(result.machineCode.length).toBe(16);

    // Verify each instruction matches the Python encoder output
    expect(readWord(result.machineCode, 0)).toBe(encodeMovImm(0, 1));
    expect(readWord(result.machineCode, 4)).toBe(encodeMovImm(1, 2));
    expect(readWord(result.machineCode, 8)).toBe(encodeAdd(2, 0, 1));
    expect(readWord(result.machineCode, 12)).toBe(encodeHlt());
  });

  it("should assemble subtraction program (10 - 3 = 7)", () => {
    const result = assemble(
      "MOV R0, #10\nMOV R1, #3\nSUB R2, R0, R1\nHLT\n",
    );
    expect(result.errors).toEqual([]);
    expect(readWord(result.machineCode, 0)).toBe(encodeMovImm(0, 10));
    expect(readWord(result.machineCode, 4)).toBe(encodeMovImm(1, 3));
    expect(readWord(result.machineCode, 8)).toBe(encodeSub(2, 0, 1));
    expect(readWord(result.machineCode, 12)).toBe(encodeHlt());
  });

  it("should assemble 100 + 200 = 300", () => {
    const result = assemble(
      "MOV R0, #100\nMOV R1, #200\nADD R2, R0, R1\nHLT\n",
    );
    expect(result.errors).toEqual([]);
    expect(result.machineCode.length).toBe(16);
  });
});

// ---------------------------------------------------------------------------
// Labels and symbol table
// ---------------------------------------------------------------------------

describe("Labels and symbol table", () => {
  it("should record labels in the symbol table", () => {
    const result = assemble("start:\nMOV R0, #1\ndone:\nHLT\n");
    expect(result.symbolTable.get("start")).toBe(0);
    expect(result.symbolTable.get("done")).toBe(4);
  });

  it("should place label at the address of the next instruction", () => {
    const result = assemble("MOV R0, #1\nloop:\nADD R0, R0, #1\nHLT\n");
    // "loop:" comes after MOV (4 bytes), so loop = address 4
    expect(result.symbolTable.get("loop")).toBe(4);
  });

  it("should handle multiple labels", () => {
    const result = assemble("a:\nMOV R0, #1\nb:\nMOV R1, #2\nc:\nHLT\n");
    expect(result.symbolTable.get("a")).toBe(0);
    expect(result.symbolTable.get("b")).toBe(4);
    expect(result.symbolTable.get("c")).toBe(8);
  });

  it("should report error for duplicate labels", () => {
    const result = assemble("loop:\nMOV R0, #1\nloop:\nHLT\n");
    expect(result.errors.length).toBeGreaterThan(0);
    expect(result.errors[0].message).toContain("Duplicate label");
  });
});

// ---------------------------------------------------------------------------
// Branch instructions with labels
// ---------------------------------------------------------------------------

describe("Branch instructions", () => {
  it("should resolve forward branch (label after branch)", () => {
    const result = assemble(
      "B done\nMOV R0, #1\ndone:\nHLT\n",
    );
    expect(result.errors).toEqual([]);
    // B done: branch from address 0 to address 8
    // offset = (8 - 0 - 8) / 4 = 0
    const word = readWord(result.machineCode, 0);
    expect((word >>> 25) & 0b111).toBe(0b101);  // branch identifier
    expect(word & 0x00FFFFFF).toBe(0);            // offset = 0
  });

  it("should resolve backward branch (label before branch)", () => {
    const source = `
      loop:
      MOV R0, #1
      B loop
      HLT
    `;
    const result = assemble(source);
    expect(result.errors).toEqual([]);
    // B loop: branch from address 4 to address 0
    // offset = (0 - 4 - 8) / 4 = -3
    const word = readWord(result.machineCode, 4);
    expect((word >>> 25) & 0b111).toBe(0b101);
    // -3 in 24-bit two's complement = 0xFFFFFD
    expect(word & 0x00FFFFFF).toBe(0xFFFFFD);
  });

  it("should resolve BEQ (conditional branch)", () => {
    const source = `
      CMP R0, #0
      BEQ done
      MOV R0, #1
      done:
      HLT
    `;
    const result = assemble(source);
    expect(result.errors).toEqual([]);
    // BEQ done: branch from address 4 to address 12
    // offset = (12 - 4 - 8) / 4 = 0
    const word = readWord(result.machineCode, 4);
    expect((word >>> 28) & 0xF).toBe(0b0000);  // EQ condition
    expect((word >>> 25) & 0b111).toBe(0b101);  // branch
    expect(word & 0x00FFFFFF).toBe(0);            // offset = 0
  });

  it("should resolve BNE (branch if not equal)", () => {
    const source = `
      MOV R0, #10
      loop:
      SUB R0, R0, #1
      CMP R0, #0
      BNE loop
      HLT
    `;
    const result = assemble(source);
    expect(result.errors).toEqual([]);
    // BNE loop: branch from address 12 to address 4
    // offset = (4 - 12 - 8) / 4 = -4
    const word = readWord(result.machineCode, 12);
    expect((word >>> 28) & 0xF).toBe(0b0001);  // NE condition
    expect(word & 0x00FFFFFF).toBe(0xFFFFFC);   // -4 in 24-bit two's complement
  });

  it("should report error for undefined label", () => {
    const result = assemble("B nonexistent\nHLT\n");
    expect(result.errors.length).toBeGreaterThan(0);
    expect(result.errors[0].message).toContain("Undefined label");
  });

  it("should resolve BL (branch with link)", () => {
    const source = `
      BL func
      HLT
      func:
      MOV R0, #1
      MOV PC, LR
    `;
    const result = assemble(source);
    expect(result.errors).toEqual([]);
    const word = readWord(result.machineCode, 0);
    // BL: L bit should be set (bit 24)
    expect((word >>> 24) & 0x1).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// Memory instructions
// ---------------------------------------------------------------------------

describe("Memory instructions", () => {
  it("should assemble LDR R0, [R1]", () => {
    const result = assemble("LDR R0, [R1]\n");
    expect(result.errors).toEqual([]);
    const word = readWord(result.machineCode, 0);
    expect((word >>> 26) & 0b11).toBe(0b01);   // memory identifier
    expect((word >>> 20) & 0x1).toBe(1);         // L=1 (load)
    expect((word >>> 16) & 0xF).toBe(1);         // Rn = R1
    expect((word >>> 12) & 0xF).toBe(0);         // Rd = R0
  });

  it("should assemble LDR R0, [R1, #4]", () => {
    const result = assemble("LDR R0, [R1, #4]\n");
    expect(result.errors).toEqual([]);
    const word = readWord(result.machineCode, 0);
    expect((word >>> 23) & 0x1).toBe(1);  // U=1 (positive offset)
    expect(word & 0xFFF).toBe(4);          // offset = 4
  });

  it("should assemble STR R2, [R3]", () => {
    const result = assemble("STR R2, [R3]\n");
    expect(result.errors).toEqual([]);
    const word = readWord(result.machineCode, 0);
    expect((word >>> 20) & 0x1).toBe(0);  // L=0 (store)
    expect((word >>> 16) & 0xF).toBe(3);  // Rn = R3
    expect((word >>> 12) & 0xF).toBe(2);  // Rd = R2
  });
});

// ---------------------------------------------------------------------------
// Source map
// ---------------------------------------------------------------------------

describe("Source map", () => {
  it("should map addresses to line numbers", () => {
    const result = assemble("MOV R0, #1\nMOV R1, #2\nHLT\n");
    expect(result.sourceMap.get(0)).toBe(1);  // address 0 → line 1
    expect(result.sourceMap.get(4)).toBe(2);  // address 4 → line 2
    expect(result.sourceMap.get(8)).toBe(3);  // address 8 → line 3
  });

  it("should not include labels in the source map (they generate no code)", () => {
    const result = assemble("loop:\nMOV R0, #1\nHLT\n");
    // "loop:" doesn't generate code, so address 0 maps to line 2 (MOV)
    expect(result.sourceMap.get(0)).toBe(2);
  });

  it("should handle gaps from labels correctly", () => {
    const result = assemble("MOV R0, #1\nloop:\nADD R0, R0, #1\nHLT\n");
    expect(result.sourceMap.get(0)).toBe(1);  // MOV at line 1
    expect(result.sourceMap.get(4)).toBe(3);  // ADD at line 3 (label is line 2)
    expect(result.sourceMap.get(8)).toBe(4);  // HLT at line 4
  });
});

// ---------------------------------------------------------------------------
// Error reporting
// ---------------------------------------------------------------------------

describe("Error reporting", () => {
  it("should report error for unknown instruction", () => {
    const result = assemble("FOOBAR R0, #1\n");
    expect(result.errors.length).toBeGreaterThan(0);
  });

  it("should report error for wrong number of operands (MOV)", () => {
    const result = assemble("MOV R0\n");
    expect(result.errors.length).toBeGreaterThan(0);
    expect(result.errors[0].message).toContain("requires 2 operands");
  });

  it("should report error for wrong number of operands (ADD)", () => {
    const result = assemble("ADD R0, R1\n");
    expect(result.errors.length).toBeGreaterThan(0);
    expect(result.errors[0].message).toContain("requires 3 operands");
  });

  it("should report error for undefined label in branch", () => {
    const result = assemble("B missing_label\n");
    expect(result.errors.length).toBeGreaterThan(0);
    expect(result.errors[0].message).toContain("Undefined label");
  });

  it("should include line numbers in errors", () => {
    const result = assemble("MOV R0, #1\nFOOBAR\nHLT\n");
    const fooError = result.errors.find(e => e.message.includes("FOOBAR"));
    expect(fooError).toBeDefined();
    expect(fooError!.line).toBe(2);
  });

  it("should continue assembling after errors (collect all errors)", () => {
    const result = assemble("FOOBAR\nBAZQUX\nHLT\n");
    expect(result.errors.length).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// Assembler class
// ---------------------------------------------------------------------------

describe("Assembler class", () => {
  it("should be reusable for multiple programs", () => {
    const asm = new Assembler();
    const r1 = asm.assemble("MOV R0, #1\nHLT\n");
    const r2 = asm.assemble("MOV R0, #2\nHLT\n");
    expect(r1.errors).toEqual([]);
    expect(r2.errors).toEqual([]);
    // They should produce different machine code
    expect(readWord(r1.machineCode, 0)).not.toBe(readWord(r2.machineCode, 0));
  });

  it("should produce empty output for empty source", () => {
    const result = assemble("");
    expect(result.errors).toEqual([]);
    expect(result.machineCode.length).toBe(0);
  });

  it("should produce empty output for comment-only source", () => {
    const result = assemble("; just a comment\n// another comment\n");
    expect(result.errors).toEqual([]);
    expect(result.machineCode.length).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Conditional instructions
// ---------------------------------------------------------------------------

describe("Conditional instructions", () => {
  it("should encode MOVGT with GT condition code", () => {
    const result = assemble("MOVGT R0, #1\n");
    expect(result.errors).toEqual([]);
    const word = readWord(result.machineCode, 0);
    expect((word >>> 28) & 0xF).toBe(0b1100);  // GT condition
  });

  it("should encode ADDNE with NE condition code", () => {
    const result = assemble("ADDNE R0, R1, R2\n");
    expect(result.errors).toEqual([]);
    const word = readWord(result.machineCode, 0);
    expect((word >>> 28) & 0xF).toBe(0b0001);  // NE condition
  });

  it("should encode SUBEQ with EQ condition code", () => {
    const result = assemble("SUBEQ R0, R1, R2\n");
    expect(result.errors).toEqual([]);
    const word = readWord(result.machineCode, 0);
    expect((word >>> 28) & 0xF).toBe(0b0000);  // EQ condition
  });
});

// ---------------------------------------------------------------------------
// Immediate with ADD/SUB
// ---------------------------------------------------------------------------

describe("Immediate operand in data processing", () => {
  it("should assemble ADD R0, R0, #1 (register + immediate)", () => {
    const result = assemble("ADD R0, R0, #1\n");
    expect(result.errors).toEqual([]);
    const word = readWord(result.machineCode, 0);
    expect((word >>> 25) & 0x1).toBe(1);  // I=1 (immediate)
    expect(word & 0xFF).toBe(1);           // imm8 = 1
  });

  it("should assemble SUB R0, R0, #5 (register - immediate)", () => {
    const result = assemble("SUB R0, R0, #5\n");
    expect(result.errors).toEqual([]);
    const word = readWord(result.machineCode, 0);
    expect((word >>> 25) & 0x1).toBe(1);  // I=1 (immediate)
    expect(word & 0xFF).toBe(5);           // imm8 = 5
  });
});

// ---------------------------------------------------------------------------
// Case insensitivity
// ---------------------------------------------------------------------------

describe("Case insensitivity", () => {
  it("should accept lowercase mnemonics", () => {
    const result = assemble("mov r0, #1\nhlt\n");
    expect(result.errors).toEqual([]);
    expect(result.machineCode.length).toBe(8);
  });

  it("should accept mixed-case mnemonics", () => {
    const result = assemble("Mov R0, #1\nHlt\n");
    expect(result.errors).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// End-to-end: assemble then verify byte-level output
// ---------------------------------------------------------------------------

describe("End-to-end byte-level verification", () => {
  it("should produce identical bytes to manual encoding (x = 1 + 2)", () => {
    // Manually encode the program using the encoder functions
    const expected = instructionsToBytes([
      encodeMovImm(0, 1),
      encodeMovImm(1, 2),
      encodeAdd(2, 0, 1),
      encodeHlt(),
    ]);

    // Assemble from source text
    const result = assemble("MOV R0, #1\nMOV R1, #2\nADD R2, R0, R1\nHLT\n");

    expect(result.errors).toEqual([]);
    expect(result.machineCode).toEqual(expected);
  });

  it("should produce identical bytes for subtraction program", () => {
    const expected = instructionsToBytes([
      encodeMovImm(0, 10),
      encodeMovImm(1, 3),
      encodeSub(2, 0, 1),
      encodeHlt(),
    ]);

    const result = assemble("MOV R0, #10\nMOV R1, #3\nSUB R2, R0, R1\nHLT\n");

    expect(result.errors).toEqual([]);
    expect(result.machineCode).toEqual(expected);
  });
});

// ---------------------------------------------------------------------------
// Complete program with all features
// ---------------------------------------------------------------------------

describe("Complete program assembly", () => {
  it("should assemble a countdown loop", () => {
    const source = `
      ; Countdown from 10 to 0
      MOV R0, #10       ; counter = 10
      loop:
      SUB R0, R0, #1    ; counter--
      CMP R0, #0        ; compare with 0
      BNE loop          ; if not zero, loop
      HLT               ; done
    `;
    const result = assemble(source);
    expect(result.errors).toEqual([]);
    expect(result.machineCode.length).toBe(20);  // 5 instructions * 4 bytes
    expect(result.symbolTable.get("loop")).toBe(4);
  });

  it("should assemble a program with forward and backward references", () => {
    const source = `
      B start           ; jump forward to start
      data:
      MOV R0, #42       ; this gets skipped
      start:
      MOV R1, #1        ; begin here
      B data            ; jump backward to data
      HLT
    `;
    const result = assemble(source);
    expect(result.errors).toEqual([]);
    expect(result.symbolTable.get("data")).toBe(4);
    expect(result.symbolTable.get("start")).toBe(8);
  });
});
