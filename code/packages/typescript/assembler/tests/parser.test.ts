/**
 * Tests for the assembly parser.
 *
 * The parser is the first stage of the assembler pipeline. It takes raw
 * assembly text and produces structured data (ParsedLine objects). These
 * tests verify that the parser correctly:
 *
 *   1. Strips comments (both ; and // styles)
 *   2. Identifies labels (name followed by colon)
 *   3. Identifies directives (starts with .)
 *   4. Parses instruction mnemonics and condition codes
 *   5. Parses operands (registers, immediates, labels, memory)
 *   6. Handles edge cases (empty lines, whitespace, mixed case)
 *   7. Reports errors for invalid syntax
 */

import { describe, it, expect } from "vitest";
import { parse, parseRegister, parseNumber } from "../src/parser.js";

// ---------------------------------------------------------------------------
// Comment handling
// ---------------------------------------------------------------------------

describe("Comment handling", () => {
  it("should skip lines that are only comments (semicolon)", () => {
    const { lines } = parse("; this is a comment\n");
    expect(lines.length).toBe(0);
  });

  it("should skip lines that are only comments (double slash)", () => {
    const { lines } = parse("// this is a comment\n");
    expect(lines.length).toBe(0);
  });

  it("should strip inline comments from instructions", () => {
    const { lines } = parse("MOV R0, #1 ; load 1\n");
    expect(lines.length).toBe(1);
    expect(lines[0].mnemonic).toBe("MOV");
  });

  it("should skip empty lines", () => {
    const { lines } = parse("\n\n\n");
    expect(lines.length).toBe(0);
  });

  it("should skip whitespace-only lines", () => {
    const { lines } = parse("   \n  \t  \n");
    expect(lines.length).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Label parsing
// ---------------------------------------------------------------------------

describe("Label parsing", () => {
  it("should parse a label ending with colon", () => {
    const { lines } = parse("loop:\n");
    expect(lines.length).toBe(1);
    expect(lines[0].kind).toBe("label");
    expect(lines[0].name).toBe("loop");
  });

  it("should parse labels with underscores", () => {
    const { lines } = parse("_start:\n");
    expect(lines[0].name).toBe("_start");
  });

  it("should parse labels with mixed alphanumeric characters", () => {
    const { lines } = parse("loop2:\n");
    expect(lines[0].name).toBe("loop2");
  });

  it("should assign correct line numbers to labels", () => {
    const { lines } = parse("MOV R0, #1\nloop:\nHLT\n");
    const label = lines.find(l => l.kind === "label");
    expect(label?.lineNumber).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// Directive parsing
// ---------------------------------------------------------------------------

describe("Directive parsing", () => {
  it("should parse a directive", () => {
    const { lines } = parse(".global _start\n");
    expect(lines.length).toBe(1);
    expect(lines[0].kind).toBe("directive");
    expect(lines[0].name).toBe(".global");
    expect(lines[0].args).toEqual(["_start"]);
  });

  it("should parse a directive without arguments", () => {
    const { lines } = parse(".data\n");
    expect(lines[0].kind).toBe("directive");
    expect(lines[0].name).toBe(".data");
    expect(lines[0].args).toEqual([]);
  });

  it("should parse .text directive", () => {
    const { lines } = parse(".text\n");
    expect(lines[0].kind).toBe("directive");
    expect(lines[0].name).toBe(".text");
  });
});

// ---------------------------------------------------------------------------
// Instruction parsing — basic mnemonics
// ---------------------------------------------------------------------------

describe("Instruction parsing — mnemonics", () => {
  it("should parse MOV instruction", () => {
    const { lines } = parse("MOV R0, #1\n");
    expect(lines[0].kind).toBe("instruction");
    expect(lines[0].mnemonic).toBe("MOV");
    expect(lines[0].condition).toBe("AL");
  });

  it("should parse ADD instruction", () => {
    const { lines } = parse("ADD R2, R0, R1\n");
    expect(lines[0].mnemonic).toBe("ADD");
  });

  it("should parse SUB instruction", () => {
    const { lines } = parse("SUB R2, R0, R1\n");
    expect(lines[0].mnemonic).toBe("SUB");
  });

  it("should parse HLT instruction (no operands)", () => {
    const { lines } = parse("HLT\n");
    expect(lines[0].mnemonic).toBe("HLT");
    expect(lines[0].operands).toEqual([]);
  });

  it("should parse CMP instruction", () => {
    const { lines } = parse("CMP R0, #10\n");
    expect(lines[0].mnemonic).toBe("CMP");
  });

  it("should parse B (branch) instruction", () => {
    const { lines } = parse("B loop\n");
    expect(lines[0].mnemonic).toBe("B");
  });

  it("should parse BL (branch with link) instruction", () => {
    const { lines } = parse("BL func\n");
    expect(lines[0].mnemonic).toBe("BL");
  });

  it("should parse NOP instruction", () => {
    const { lines } = parse("NOP\n");
    expect(lines[0].mnemonic).toBe("NOP");
  });

  it("should parse case-insensitively", () => {
    const { lines } = parse("mov r0, #1\n");
    expect(lines[0].mnemonic).toBe("MOV");
  });

  it("should report errors for unknown instructions", () => {
    const { errors } = parse("FOOBAR R0, #1\n");
    expect(errors.length).toBe(1);
    expect(errors[0].message).toContain("Unknown instruction");
    expect(errors[0].line).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// Condition code parsing
// ---------------------------------------------------------------------------

describe("Condition code parsing", () => {
  it("should parse instruction with EQ condition", () => {
    const { lines } = parse("BEQ loop\n");
    expect(lines[0].mnemonic).toBe("B");
    expect(lines[0].condition).toBe("EQ");
  });

  it("should parse instruction with NE condition", () => {
    const { lines } = parse("BNE loop\n");
    expect(lines[0].mnemonic).toBe("B");
    expect(lines[0].condition).toBe("NE");
  });

  it("should parse ADDNE (ADD with NE condition)", () => {
    const { lines } = parse("ADDNE R0, R1, R2\n");
    expect(lines[0].mnemonic).toBe("ADD");
    expect(lines[0].condition).toBe("NE");
  });

  it("should parse MOVGT (MOV with GT condition)", () => {
    const { lines } = parse("MOVGT R0, #1\n");
    expect(lines[0].mnemonic).toBe("MOV");
    expect(lines[0].condition).toBe("GT");
  });

  it("should parse BLNE (BL with NE condition)", () => {
    const { lines } = parse("BLNE func\n");
    expect(lines[0].mnemonic).toBe("BL");
    expect(lines[0].condition).toBe("NE");
  });

  it("should default to AL (always) when no condition suffix", () => {
    const { lines } = parse("MOV R0, #1\n");
    expect(lines[0].condition).toBe("AL");
  });
});

// ---------------------------------------------------------------------------
// Operand parsing — registers
// ---------------------------------------------------------------------------

describe("Operand parsing — registers", () => {
  it("should parse register operands R0-R15", () => {
    const { lines } = parse("ADD R2, R0, R1\n");
    const ops = lines[0].operands!;
    expect(ops.length).toBe(3);
    expect(ops[0]).toEqual({ type: "register", value: 2 });
    expect(ops[1]).toEqual({ type: "register", value: 0 });
    expect(ops[2]).toEqual({ type: "register", value: 1 });
  });

  it("should parse SP alias as R13", () => {
    const { lines } = parse("MOV SP, #0\n");
    expect(lines[0].operands![0]).toEqual({ type: "register", value: 13 });
  });

  it("should parse LR alias as R14", () => {
    const { lines } = parse("MOV LR, #0\n");
    expect(lines[0].operands![0]).toEqual({ type: "register", value: 14 });
  });

  it("should parse PC alias as R15", () => {
    const { lines } = parse("MOV PC, #0\n");
    expect(lines[0].operands![0]).toEqual({ type: "register", value: 15 });
  });
});

// ---------------------------------------------------------------------------
// Operand parsing — immediates
// ---------------------------------------------------------------------------

describe("Operand parsing — immediates", () => {
  it("should parse decimal immediates", () => {
    const { lines } = parse("MOV R0, #42\n");
    expect(lines[0].operands![1]).toEqual({ type: "immediate", value: 42 });
  });

  it("should parse hexadecimal immediates", () => {
    const { lines } = parse("MOV R0, #0xFF\n");
    expect(lines[0].operands![1]).toEqual({ type: "immediate", value: 255 });
  });

  it("should parse binary immediates", () => {
    const { lines } = parse("MOV R0, #0b1010\n");
    expect(lines[0].operands![1]).toEqual({ type: "immediate", value: 10 });
  });

  it("should parse negative immediates", () => {
    const { lines } = parse("MOV R0, #-1\n");
    expect(lines[0].operands![1]).toEqual({ type: "immediate", value: -1 });
  });

  it("should parse zero", () => {
    const { lines } = parse("MOV R0, #0\n");
    expect(lines[0].operands![1]).toEqual({ type: "immediate", value: 0 });
  });
});

// ---------------------------------------------------------------------------
// Operand parsing — labels
// ---------------------------------------------------------------------------

describe("Operand parsing — labels", () => {
  it("should parse label references in branch instructions", () => {
    const { lines } = parse("B loop\n");
    expect(lines[0].operands![0]).toEqual({ type: "label", value: "loop" });
  });

  it("should parse label references with underscores", () => {
    const { lines } = parse("B _end_loop\n");
    expect(lines[0].operands![0]).toEqual({ type: "label", value: "_end_loop" });
  });
});

// ---------------------------------------------------------------------------
// Operand parsing — memory
// ---------------------------------------------------------------------------

describe("Operand parsing — memory", () => {
  it("should parse LDR with base register only", () => {
    const { lines } = parse("LDR R0, [R1]\n");
    const ops = lines[0].operands!;
    expect(ops[0]).toEqual({ type: "register", value: 0 });  // Rd
    expect(ops[1]).toEqual({ type: "register", value: 1 });  // Rn (base)
    expect(ops[2]).toEqual({ type: "immediate", value: 0 }); // default offset
  });

  it("should parse LDR with offset", () => {
    const { lines } = parse("LDR R0, [R1, #4]\n");
    const ops = lines[0].operands!;
    expect(ops[0]).toEqual({ type: "register", value: 0 });
    expect(ops[1]).toEqual({ type: "register", value: 1 });
    expect(ops[2]).toEqual({ type: "immediate", value: 4 });
  });

  it("should parse STR instruction", () => {
    const { lines } = parse("STR R2, [R3]\n");
    expect(lines[0].mnemonic).toBe("STR");
    const ops = lines[0].operands!;
    expect(ops[0]).toEqual({ type: "register", value: 2 });
    expect(ops[1]).toEqual({ type: "register", value: 3 });
  });
});

// ---------------------------------------------------------------------------
// Multi-line programs
// ---------------------------------------------------------------------------

describe("Multi-line programs", () => {
  it("should parse a complete program", () => {
    const source = `
      MOV R0, #1      ; load 1
      MOV R1, #2      ; load 2
      ADD R2, R0, R1  ; add them
      HLT             ; stop
    `;
    const { lines, errors } = parse(source);
    expect(errors.length).toBe(0);
    expect(lines.length).toBe(4);
    expect(lines[0].mnemonic).toBe("MOV");
    expect(lines[1].mnemonic).toBe("MOV");
    expect(lines[2].mnemonic).toBe("ADD");
    expect(lines[3].mnemonic).toBe("HLT");
  });

  it("should parse a program with labels and branches", () => {
    const source = `
      MOV R0, #10
      loop:
      SUB R0, R0, #1
      CMP R0, #0
      BNE loop
      HLT
    `;
    const { lines, errors } = parse(source);
    expect(errors.length).toBe(0);
    expect(lines.length).toBe(6);
    expect(lines[0].mnemonic).toBe("MOV");
    expect(lines[1].kind).toBe("label");
    expect(lines[1].name).toBe("loop");
    expect(lines[2].mnemonic).toBe("SUB");
    expect(lines[3].mnemonic).toBe("CMP");
    expect(lines[4].mnemonic).toBe("B");
    expect(lines[4].condition).toBe("NE");
    expect(lines[5].mnemonic).toBe("HLT");
  });

  it("should track line numbers correctly", () => {
    const source = "MOV R0, #1\n\nADD R1, R0, #2\n";
    const { lines } = parse(source);
    expect(lines[0].lineNumber).toBe(1);
    expect(lines[1].lineNumber).toBe(3);  // line 2 is empty
  });
});

// ---------------------------------------------------------------------------
// parseRegister utility tests
// ---------------------------------------------------------------------------

describe("parseRegister", () => {
  it("should parse R0-R15", () => {
    expect(parseRegister("R0")).toBe(0);
    expect(parseRegister("R1")).toBe(1);
    expect(parseRegister("R15")).toBe(15);
  });

  it("should be case-insensitive", () => {
    expect(parseRegister("r0")).toBe(0);
    expect(parseRegister("r15")).toBe(15);
  });

  it("should parse aliases", () => {
    expect(parseRegister("SP")).toBe(13);
    expect(parseRegister("LR")).toBe(14);
    expect(parseRegister("PC")).toBe(15);
  });

  it("should return null for invalid registers", () => {
    expect(parseRegister("R16")).toBeNull();
    expect(parseRegister("R99")).toBeNull();
    expect(parseRegister("hello")).toBeNull();
    expect(parseRegister("42")).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// parseNumber utility tests
// ---------------------------------------------------------------------------

describe("parseNumber", () => {
  it("should parse decimal numbers", () => {
    expect(parseNumber("0")).toBe(0);
    expect(parseNumber("42")).toBe(42);
    expect(parseNumber("255")).toBe(255);
  });

  it("should parse hexadecimal numbers", () => {
    expect(parseNumber("0xFF")).toBe(255);
    expect(parseNumber("0x1A")).toBe(26);
    expect(parseNumber("0X1A")).toBe(26);
  });

  it("should parse binary numbers", () => {
    expect(parseNumber("0b1010")).toBe(10);
    expect(parseNumber("0B1111")).toBe(15);
  });

  it("should parse negative numbers", () => {
    expect(parseNumber("-1")).toBe(-1);
    expect(parseNumber("-42")).toBe(-42);
    expect(parseNumber("-0xFF")).toBe(-255);
  });

  it("should return null for invalid input", () => {
    expect(parseNumber("")).toBeNull();
    expect(parseNumber("hello")).toBeNull();
  });
});
