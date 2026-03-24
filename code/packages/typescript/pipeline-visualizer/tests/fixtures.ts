/**
 * Test Fixtures — Reusable pipeline report data for tests.
 * ==========================================================
 *
 * Building PipelineReport objects by hand in every test would be
 * tedious and error-prone. Instead, we define a set of standard
 * fixtures that tests can import and use.
 *
 * Each fixture represents a realistic pipeline report:
 *
 * - **vmReport**: A VM-only pipeline for `x = 1 + 2`
 *   Stages: lexer -> parser -> compiler -> vm
 *
 * - **riscvReport**: A RISC-V hardware pipeline for `x = 1 + 2`
 *   Stages: lexer -> parser -> assembler -> riscv -> alu -> gates
 *
 * - **minimalReport**: Bare minimum valid report (no stages)
 *
 * - **singleTokenReport**: Edge case with just one token
 */

import type { PipelineReport } from "../src/types.js";

// ===========================================================================
// Shared metadata
// ===========================================================================

const baseMetadata = {
  generated_at: "2026-03-18T12:00:00Z",
  generator_version: "0.1.0",
  packages: {
    lexer: "0.1.0",
    parser: "0.1.0",
  },
};

// ===========================================================================
// VM Pipeline Report
// ===========================================================================

/**
 * A complete VM-only pipeline report for `x = 1 + 2`.
 *
 * This exercises: lexer, parser, compiler, and VM stages.
 * No assembly, hardware, ALU, or gate stages.
 */
export const vmReport: PipelineReport = {
  source: "x = 1 + 2",
  language: "python",
  target: "vm",
  metadata: { ...baseMetadata },
  stages: [
    {
      name: "lexer",
      display_name: "Tokenization",
      input_repr: "x = 1 + 2",
      output_repr: "6 tokens",
      duration_ms: 0.12,
      data: {
        tokens: [
          { type: "NAME", value: "x", line: 1, column: 1 },
          { type: "EQUALS", value: "=", line: 1, column: 3 },
          { type: "NUMBER", value: "1", line: 1, column: 5 },
          { type: "PLUS", value: "+", line: 1, column: 7 },
          { type: "NUMBER", value: "2", line: 1, column: 9 },
          { type: "EOF", value: "", line: 1, column: 10 },
        ],
      },
    },
    {
      name: "parser",
      display_name: "Parsing",
      input_repr: "6 tokens",
      output_repr: "AST with 5 nodes",
      duration_ms: 0.08,
      data: {
        ast: {
          type: "Assignment",
          children: [
            { type: "Name", value: "x", children: [] },
            {
              type: "BinaryOp",
              value: "+",
              children: [
                { type: "Number", value: "1", children: [] },
                { type: "Number", value: "2", children: [] },
              ],
            },
          ],
        },
      },
    },
    {
      name: "compiler",
      display_name: "Bytecode Compilation",
      input_repr: "AST with 5 nodes",
      output_repr: "5 instructions",
      duration_ms: 0.05,
      data: {
        instructions: [
          {
            index: 0,
            opcode: "LOAD_CONST",
            arg: "1",
            stack_effect: "\u2192 1",
          },
          {
            index: 1,
            opcode: "LOAD_CONST",
            arg: "2",
            stack_effect: "\u2192 2",
          },
          {
            index: 2,
            opcode: "ADD",
            arg: null,
            stack_effect: "1, 2 \u2192 3",
          },
          {
            index: 3,
            opcode: "STORE_NAME",
            arg: "x",
            stack_effect: "3 \u2192",
          },
          { index: 4, opcode: "HALT", arg: null, stack_effect: "" },
        ],
        constants: [1, 2],
        names: ["x"],
      },
    },
    {
      name: "vm",
      display_name: "VM Execution",
      input_repr: "5 instructions",
      output_repr: "5 steps",
      duration_ms: 0.03,
      data: {
        steps: [
          {
            index: 0,
            instruction: "LOAD_CONST 1",
            stack_before: [],
            stack_after: [1],
            variables: {},
          },
          {
            index: 1,
            instruction: "LOAD_CONST 2",
            stack_before: [1],
            stack_after: [1, 2],
            variables: {},
          },
          {
            index: 2,
            instruction: "ADD",
            stack_before: [1, 2],
            stack_after: [3],
            variables: {},
          },
          {
            index: 3,
            instruction: "STORE_NAME x",
            stack_before: [3],
            stack_after: [],
            variables: { x: 3 },
          },
          {
            index: 4,
            instruction: "HALT",
            stack_before: [],
            stack_after: [],
            variables: { x: 3 },
          },
        ],
      },
    },
  ],
};

// ===========================================================================
// RISC-V Hardware Pipeline Report
// ===========================================================================

/**
 * A hardware-level pipeline report targeting RISC-V.
 *
 * This exercises: lexer, parser, assembler, riscv, alu, and gates.
 * No compiler or VM stages.
 */
export const riscvReport: PipelineReport = {
  source: "x = 1 + 2",
  language: "python",
  target: "riscv",
  metadata: {
    ...baseMetadata,
    packages: {
      ...baseMetadata.packages,
      assembler: "0.1.0",
      "riscv-simulator": "0.1.0",
    },
  },
  stages: [
    {
      name: "lexer",
      display_name: "Tokenization",
      input_repr: "x = 1 + 2",
      output_repr: "6 tokens",
      duration_ms: 0.12,
      data: {
        tokens: [
          { type: "NAME", value: "x", line: 1, column: 1 },
          { type: "EQUALS", value: "=", line: 1, column: 3 },
          { type: "NUMBER", value: "1", line: 1, column: 5 },
          { type: "PLUS", value: "+", line: 1, column: 7 },
          { type: "NUMBER", value: "2", line: 1, column: 9 },
          { type: "EOF", value: "", line: 1, column: 10 },
        ],
      },
    },
    {
      name: "parser",
      display_name: "Parsing",
      input_repr: "6 tokens",
      output_repr: "AST with 5 nodes",
      duration_ms: 0.08,
      data: {
        ast: {
          type: "Assignment",
          children: [
            { type: "Name", value: "x", children: [] },
            {
              type: "BinaryOp",
              value: "+",
              children: [
                { type: "Number", value: "1", children: [] },
                { type: "Number", value: "2", children: [] },
              ],
            },
          ],
        },
      },
    },
    {
      name: "assembler",
      display_name: "Assembly",
      input_repr: "AST with 5 nodes",
      output_repr: "4 instructions",
      duration_ms: 0.15,
      data: {
        lines: [
          {
            address: 0,
            assembly: "addi x1, x0, 1",
            binary: "0x00100093",
            encoding: {
              imm: "000000000001",
              rs1: "00000",
              funct3: "000",
              rd: "00001",
              opcode: "0010011",
            },
          },
          {
            address: 4,
            assembly: "addi x2, x0, 2",
            binary: "0x00200113",
            encoding: {
              imm: "000000000010",
              rs1: "00000",
              funct3: "000",
              rd: "00010",
              opcode: "0010011",
            },
          },
          {
            address: 8,
            assembly: "add x3, x1, x2",
            binary: "0x002081B3",
            encoding: {
              funct7: "0000000",
              rs2: "00010",
              rs1: "00001",
              funct3: "000",
              rd: "00011",
              opcode: "0110011",
            },
          },
          {
            address: 12,
            assembly: "ecall",
            binary: "0x00000073",
            encoding: {},
          },
        ],
      },
    },
    {
      name: "riscv",
      display_name: "RISC-V Execution",
      input_repr: "4 instructions",
      output_repr: "4 steps",
      duration_ms: 0.1,
      data: {
        steps: [
          {
            address: 0,
            instruction: "addi x1, x0, 1",
            registers_changed: { x1: 1 },
            registers: { x0: 0, x1: 1, x2: 0, x3: 0 },
          },
          {
            address: 4,
            instruction: "addi x2, x0, 2",
            registers_changed: { x2: 2 },
            registers: { x0: 0, x1: 1, x2: 2, x3: 0 },
          },
          {
            address: 8,
            instruction: "add x3, x1, x2",
            registers_changed: { x3: 3 },
            registers: { x0: 0, x1: 1, x2: 2, x3: 3 },
          },
          {
            address: 12,
            instruction: "ecall",
            registers_changed: {},
            registers: { x0: 0, x1: 1, x2: 2, x3: 3 },
          },
        ],
      },
    },
    {
      name: "alu",
      display_name: "ALU Operations",
      input_repr: "add x3, x1, x2",
      output_repr: "1 operation",
      duration_ms: 0.02,
      data: {
        operations: [
          {
            op: "ADD",
            a: 1,
            b: 2,
            result: 3,
            bits_a: "00000001",
            bits_b: "00000010",
            bits_result: "00000011",
            flags: { zero: false, carry: false, negative: false, overflow: false },
          },
        ],
      },
    },
    {
      name: "gates",
      display_name: "Gate Operations",
      input_repr: "ADD 1, 2",
      output_repr: "10 gate evaluations",
      duration_ms: 0.01,
      data: {
        operations: [
          {
            description: "Full adder bit 0",
            gates: [
              { gate: "XOR", inputs: [1, 0], output: 1, label: "A0 XOR B0" },
              { gate: "AND", inputs: [1, 0], output: 0, label: "A0 AND B0" },
              {
                gate: "XOR",
                inputs: [1, 0],
                output: 1,
                label: "Sum0 = partial XOR carry_in",
              },
              {
                gate: "AND",
                inputs: [1, 0],
                output: 0,
                label: "partial AND carry_in",
              },
              { gate: "OR", inputs: [0, 0], output: 0, label: "Carry0" },
            ],
          },
          {
            description: "Full adder bit 1",
            gates: [
              { gate: "XOR", inputs: [0, 1], output: 1, label: "A1 XOR B1" },
              { gate: "AND", inputs: [0, 1], output: 0, label: "A1 AND B1" },
              {
                gate: "XOR",
                inputs: [1, 0],
                output: 1,
                label: "Sum1 = partial XOR carry_in",
              },
              {
                gate: "AND",
                inputs: [1, 0],
                output: 0,
                label: "partial AND carry_in",
              },
              { gate: "OR", inputs: [0, 0], output: 0, label: "Carry1" },
            ],
          },
        ],
      },
    },
  ],
};

// ===========================================================================
// Edge Cases
// ===========================================================================

/**
 * Minimal valid report — no stages at all.
 *
 * Tests that the renderer doesn't crash on empty input.
 */
export const minimalReport: PipelineReport = {
  source: "",
  language: "python",
  target: "vm",
  metadata: {
    generated_at: "2026-03-18T12:00:00Z",
    generator_version: "0.1.0",
    packages: {},
  },
  stages: [],
};

/**
 * Report with a single token — edge case for the lexer renderer.
 */
export const singleTokenReport: PipelineReport = {
  source: "42",
  language: "python",
  target: "vm",
  metadata: { ...baseMetadata },
  stages: [
    {
      name: "lexer",
      display_name: "Tokenization",
      input_repr: "42",
      output_repr: "1 token",
      duration_ms: 0.01,
      data: {
        tokens: [{ type: "NUMBER", value: "42", line: 1, column: 1 }],
      },
    },
  ],
};

/**
 * Report with special characters in source — tests HTML escaping.
 */
export const xssReport: PipelineReport = {
  source: '<script>alert("xss")</script>',
  language: "python",
  target: "vm",
  metadata: { ...baseMetadata },
  stages: [
    {
      name: "lexer",
      display_name: "Tokenization",
      input_repr: '<script>alert("xss")</script>',
      output_repr: "1 token",
      duration_ms: 0.01,
      data: {
        tokens: [
          {
            type: "ERROR",
            value: '<script>alert("xss")</script>',
            line: 1,
            column: 1,
          },
        ],
      },
    },
  ],
};

/**
 * Report with an unknown stage type — tests fallback rendering.
 */
export const unknownStageReport: PipelineReport = {
  source: "test",
  language: "python",
  target: "vm",
  metadata: { ...baseMetadata },
  stages: [
    {
      name: "quantum_simulator",
      display_name: "Quantum Simulation",
      input_repr: "quantum circuit",
      output_repr: "measurement results",
      duration_ms: 42.0,
      data: {
        qubits: 3,
        gates_applied: ["H", "CNOT", "MEASURE"],
        results: [0, 1, 1],
      },
    },
  ],
};

/**
 * Report with a deeply nested AST — tests tree rendering.
 */
export const deepAstReport: PipelineReport = {
  source: "1 + 2 + 3 + 4",
  language: "python",
  target: "vm",
  metadata: { ...baseMetadata },
  stages: [
    {
      name: "parser",
      display_name: "Parsing",
      input_repr: "tokens",
      output_repr: "deeply nested AST",
      duration_ms: 0.1,
      data: {
        ast: {
          type: "BinaryOp",
          value: "+",
          children: [
            {
              type: "BinaryOp",
              value: "+",
              children: [
                {
                  type: "BinaryOp",
                  value: "+",
                  children: [
                    { type: "Number", value: "1", children: [] },
                    { type: "Number", value: "2", children: [] },
                  ],
                },
                { type: "Number", value: "3", children: [] },
              ],
            },
            { type: "Number", value: "4", children: [] },
          ],
        },
      },
    },
  ],
};
