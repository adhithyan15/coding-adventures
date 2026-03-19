/**
 * Tests for individual stage renderers.
 * ======================================
 *
 * Each stage renderer transforms stage-specific data into HTML.
 * These tests verify that the HTML output contains the expected
 * content: token values, AST node labels, bytecode opcodes, etc.
 *
 * We test the HTML as a string (checking for substrings) rather
 * than parsing it into a DOM, because we're mainly interested in
 * verifying that the correct data appears — not the exact HTML
 * structure (which might change for visual tweaks).
 */

import { describe, it, expect } from "vitest";
import {
  renderLexerStage,
  renderParserStage,
  renderCompilerStage,
  renderVMStage,
  renderAssemblerStage,
  renderHardwareExecutionStage,
  renderALUStage,
  renderGateStage,
  renderFallbackStage,
} from "../src/stage-renderers.js";
import type {
  LexerData,
  ParserData,
  CompilerData,
  VMData,
  AssemblerData,
  HardwareExecutionData,
  ALUData,
  GateData,
} from "../src/types.js";

// ===========================================================================
// Lexer Stage
// ===========================================================================

describe("renderLexerStage", () => {
  const lexerData: LexerData = {
    tokens: [
      { type: "NAME", value: "x", line: 1, column: 1 },
      { type: "EQUALS", value: "=", line: 1, column: 3 },
      { type: "NUMBER", value: "1", line: 1, column: 5 },
      { type: "PLUS", value: "+", line: 1, column: 7 },
      { type: "NUMBER", value: "2", line: 1, column: 9 },
      { type: "EOF", value: "", line: 1, column: 10 },
    ],
  };

  it("should render token badges in a token-list container", () => {
    const html = renderLexerStage(lexerData);
    expect(html).toContain("token-list");
  });

  it("should include all token types", () => {
    const html = renderLexerStage(lexerData);
    expect(html).toContain("NAME");
    expect(html).toContain("EQUALS");
    expect(html).toContain("NUMBER");
    expect(html).toContain("PLUS");
    expect(html).toContain("EOF");
  });

  it("should include all token values", () => {
    const html = renderLexerStage(lexerData);
    expect(html).toContain(">x<");
    expect(html).toContain(">=<");
    expect(html).toContain(">1<");
    expect(html).toContain(">+<");
    expect(html).toContain(">2<");
  });

  it("should classify NAME tokens as token-name", () => {
    const html = renderLexerStage(lexerData);
    expect(html).toContain("token-name");
  });

  it("should classify NUMBER tokens as token-number", () => {
    const html = renderLexerStage(lexerData);
    expect(html).toContain("token-number");
  });

  it("should classify operator tokens as token-operator", () => {
    const html = renderLexerStage(lexerData);
    expect(html).toContain("token-operator");
  });

  it('should show empty quotes for empty-value tokens', () => {
    const html = renderLexerStage(lexerData);
    // EOF token has empty value, should show as ""
    expect(html).toContain('""');
  });

  it("should handle keyword tokens", () => {
    const data: LexerData = {
      tokens: [{ type: "KEYWORD_IF", value: "if", line: 1, column: 1 }],
    };
    const html = renderLexerStage(data);
    expect(html).toContain("token-keyword");
  });

  it("should handle string tokens", () => {
    const data: LexerData = {
      tokens: [{ type: "STRING", value: "hello", line: 1, column: 1 }],
    };
    const html = renderLexerStage(data);
    expect(html).toContain("token-string");
  });

  it("should use default class for unrecognized token types", () => {
    const data: LexerData = {
      tokens: [{ type: "WEIRD_TOKEN", value: "?", line: 1, column: 1 }],
    };
    const html = renderLexerStage(data);
    expect(html).toContain("token-default");
  });
});

// ===========================================================================
// Parser Stage (AST)
// ===========================================================================

describe("renderParserStage", () => {
  const parserData: ParserData = {
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
  };

  it("should render an SVG element", () => {
    const html = renderParserStage(parserData);
    expect(html).toContain("<svg");
    expect(html).toContain("</svg>");
  });

  it("should include all node types in the SVG", () => {
    const html = renderParserStage(parserData);
    expect(html).toContain("Assignment");
    expect(html).toContain("Name(x)");
    expect(html).toContain("BinaryOp(+)");
    expect(html).toContain("Number(1)");
    expect(html).toContain("Number(2)");
  });

  it("should draw rectangles for each node", () => {
    const html = renderParserStage(parserData);
    // 5 nodes = 5 rectangles
    const rectCount = (html.match(/<rect/g) || []).length;
    expect(rectCount).toBe(5);
  });

  it("should draw lines connecting parent to children", () => {
    const html = renderParserStage(parserData);
    const lineCount = (html.match(/<line/g) || []).length;
    // 4 edges: Assignment->Name, Assignment->BinaryOp, BinaryOp->Number(1), BinaryOp->Number(2)
    expect(lineCount).toBe(4);
  });

  it("should wrap in an ast-container div", () => {
    const html = renderParserStage(parserData);
    expect(html).toContain("ast-container");
  });

  it("should handle a single-node AST (leaf only)", () => {
    const leafData: ParserData = {
      ast: { type: "Number", value: "42", children: [] },
    };
    const html = renderParserStage(leafData);
    expect(html).toContain("Number(42)");
    expect((html.match(/<rect/g) || []).length).toBe(1);
  });

  it("should handle deeply nested AST", () => {
    const deepData: ParserData = {
      ast: {
        type: "A",
        children: [
          {
            type: "B",
            children: [
              {
                type: "C",
                children: [{ type: "D", value: "leaf", children: [] }],
              },
            ],
          },
        ],
      },
    };
    const html = renderParserStage(deepData);
    expect(html).toContain("D(leaf)");
    expect((html.match(/<rect/g) || []).length).toBe(4);
  });
});

// ===========================================================================
// Compiler Stage (Bytecode)
// ===========================================================================

describe("renderCompilerStage", () => {
  const compilerData: CompilerData = {
    instructions: [
      { index: 0, opcode: "LOAD_CONST", arg: "1", stack_effect: "\u2192 1" },
      { index: 1, opcode: "LOAD_CONST", arg: "2", stack_effect: "\u2192 2" },
      { index: 2, opcode: "ADD", arg: null, stack_effect: "1, 2 \u2192 3" },
      { index: 3, opcode: "STORE_NAME", arg: "x", stack_effect: "3 \u2192" },
      { index: 4, opcode: "HALT", arg: null, stack_effect: "" },
    ],
    constants: [1, 2],
    names: ["x"],
  };

  it("should render a table with instruction rows", () => {
    const html = renderCompilerStage(compilerData);
    expect(html).toContain("<table>");
    expect(html).toContain("LOAD_CONST");
    expect(html).toContain("ADD");
    expect(html).toContain("STORE_NAME");
    expect(html).toContain("HALT");
  });

  it("should include table headers", () => {
    const html = renderCompilerStage(compilerData);
    expect(html).toContain("<th>#</th>");
    expect(html).toContain("<th>Opcode</th>");
    expect(html).toContain("<th>Arg</th>");
    expect(html).toContain("<th>Stack Effect</th>");
  });

  it("should display the constants pool", () => {
    const html = renderCompilerStage(compilerData);
    expect(html).toContain("Constants Pool");
    expect(html).toContain("1, 2");
  });

  it("should display the names table", () => {
    const html = renderCompilerStage(compilerData);
    expect(html).toContain("Names Table");
    expect(html).toContain("x");
  });

  it("should handle empty constants and names", () => {
    const emptyData: CompilerData = {
      instructions: [{ index: 0, opcode: "HALT", arg: null, stack_effect: "" }],
      constants: [],
      names: [],
    };
    const html = renderCompilerStage(emptyData);
    expect(html).not.toContain("Constants Pool");
    expect(html).not.toContain("Names Table");
  });

  it("should leave arg cell empty for null args", () => {
    const html = renderCompilerStage(compilerData);
    // The ADD instruction has null arg, should render as empty td
    expect(html).toContain("<td></td>");
  });
});

// ===========================================================================
// VM Stage
// ===========================================================================

describe("renderVMStage", () => {
  const vmData: VMData = {
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
    ],
  };

  it("should render a table with execution steps", () => {
    const html = renderVMStage(vmData);
    expect(html).toContain("<table>");
    expect(html).toContain("LOAD_CONST 1");
    expect(html).toContain("ADD");
    expect(html).toContain("STORE_NAME x");
  });

  it("should include stack visualizations", () => {
    const html = renderVMStage(vmData);
    expect(html).toContain('class="stack"');
    expect(html).toContain('class="stack-item"');
  });

  it("should show variables when present", () => {
    const html = renderVMStage(vmData);
    expect(html).toContain("var-name");
    expect(html).toContain("var-value");
    expect(html).toContain(">x<");
    expect(html).toContain(">3<");
  });

  it("should show dash for empty stacks", () => {
    const html = renderVMStage(vmData);
    // Empty stack should show "-"
    expect(html).toContain(">-<");
  });

  it("should show dash for empty variables", () => {
    const html = renderVMStage(vmData);
    // Steps with empty variables should have a dash
    expect(html).toContain(">-<");
  });
});

// ===========================================================================
// Assembler Stage
// ===========================================================================

describe("renderAssemblerStage", () => {
  const assemblerData: AssemblerData = {
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
        address: 12,
        assembly: "ecall",
        binary: "0x00000073",
        encoding: {},
      },
    ],
  };

  it("should render a table with assembly lines", () => {
    const html = renderAssemblerStage(assemblerData);
    expect(html).toContain("<table>");
    expect(html).toContain("addi x1, x0, 1");
    expect(html).toContain("ecall");
  });

  it("should display addresses in hex", () => {
    const html = renderAssemblerStage(assemblerData);
    expect(html).toContain("0x00");
    expect(html).toContain("0x0c");
  });

  it("should display binary encoding", () => {
    const html = renderAssemblerStage(assemblerData);
    expect(html).toContain("0x00100093");
  });

  it("should render bit-field encoding labels", () => {
    const html = renderAssemblerStage(assemblerData);
    expect(html).toContain("bit-field-label");
    expect(html).toContain("imm");
    expect(html).toContain("rs1");
    expect(html).toContain("funct3");
    expect(html).toContain("rd");
    expect(html).toContain("opcode");
  });

  it("should render bit-field encoding values", () => {
    const html = renderAssemblerStage(assemblerData);
    expect(html).toContain("000000000001");
    expect(html).toContain("00000");
    expect(html).toContain("0010011");
  });

  it("should handle empty encoding (ecall)", () => {
    const html = renderAssemblerStage(assemblerData);
    // ecall has empty encoding — should not crash
    expect(html).toContain("ecall");
  });
});

// ===========================================================================
// Hardware Execution Stage
// ===========================================================================

describe("renderHardwareExecutionStage", () => {
  const hwData: HardwareExecutionData = {
    steps: [
      {
        address: 0,
        instruction: "addi x1, x0, 1",
        registers_changed: { x1: 1 },
        registers: { x0: 0, x1: 1, x2: 0 },
      },
      {
        address: 4,
        instruction: "addi x2, x0, 2",
        registers_changed: { x2: 2 },
        registers: { x0: 0, x1: 1, x2: 2 },
      },
    ],
  };

  it("should render a table with execution steps", () => {
    const html = renderHardwareExecutionStage(hwData);
    expect(html).toContain("<table>");
    expect(html).toContain("addi x1, x0, 1");
    expect(html).toContain("addi x2, x0, 2");
  });

  it("should highlight changed registers", () => {
    const html = renderHardwareExecutionStage(hwData);
    expect(html).toContain("reg-changed");
  });

  it("should display register values", () => {
    const html = renderHardwareExecutionStage(hwData);
    expect(html).toContain("x0=0");
    expect(html).toContain("x1=1");
    expect(html).toContain("x2=0");
  });

  it("should display addresses in hex", () => {
    const html = renderHardwareExecutionStage(hwData);
    expect(html).toContain("0x00");
    expect(html).toContain("0x04");
  });
});

// ===========================================================================
// ALU Stage
// ===========================================================================

describe("renderALUStage", () => {
  const aluData: ALUData = {
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
  };

  it("should render a table with ALU operations", () => {
    const html = renderALUStage(aluData);
    expect(html).toContain("<table>");
    expect(html).toContain("ADD");
  });

  it("should display binary representations", () => {
    const html = renderALUStage(aluData);
    expect(html).toContain("00000001");
    expect(html).toContain("00000010");
    expect(html).toContain("00000011");
  });

  it("should display flags", () => {
    const html = renderALUStage(aluData);
    expect(html).toContain("Z=0");
    expect(html).toContain("C=0");
    expect(html).toContain("N=0");
    expect(html).toContain("V=0");
  });

  it("should highlight set flags", () => {
    const data: ALUData = {
      operations: [
        {
          op: "SUB",
          a: 5,
          b: 5,
          result: 0,
          bits_a: "00000101",
          bits_b: "00000101",
          bits_result: "00000000",
          flags: { zero: true, carry: false, negative: false, overflow: false },
        },
      ],
    };
    const html = renderALUStage(data);
    expect(html).toContain("flag-set");
    expect(html).toContain("Z=1");
  });
});

// ===========================================================================
// Gate Stage
// ===========================================================================

describe("renderGateStage", () => {
  const gateData: GateData = {
    operations: [
      {
        description: "Full adder bit 0",
        gates: [
          { gate: "XOR", inputs: [1, 0], output: 1, label: "A0 XOR B0" },
          { gate: "AND", inputs: [1, 0], output: 0, label: "A0 AND B0" },
        ],
      },
    ],
  };

  it("should render gate groups", () => {
    const html = renderGateStage(gateData);
    expect(html).toContain("gate-group");
    expect(html).toContain("Full adder bit 0");
  });

  it("should display gate names", () => {
    const html = renderGateStage(gateData);
    expect(html).toContain("XOR");
    expect(html).toContain("AND");
  });

  it("should display gate inputs and outputs", () => {
    const html = renderGateStage(gateData);
    expect(html).toContain("[1, 0]");
  });

  it("should display gate labels", () => {
    const html = renderGateStage(gateData);
    expect(html).toContain("A0 XOR B0");
    expect(html).toContain("A0 AND B0");
  });

  it("should handle multiple operation groups", () => {
    const multiData: GateData = {
      operations: [
        {
          description: "Group A",
          gates: [{ gate: "OR", inputs: [0, 0], output: 0, label: "test" }],
        },
        {
          description: "Group B",
          gates: [{ gate: "NOT", inputs: [1], output: 0, label: "test2" }],
        },
      ],
    };
    const html = renderGateStage(multiData);
    expect(html).toContain("Group A");
    expect(html).toContain("Group B");
  });
});

// ===========================================================================
// Fallback Stage
// ===========================================================================

describe("renderFallbackStage", () => {
  it("should render unknown data as formatted JSON", () => {
    const data = { custom_field: "value", nested: { a: 1 } };
    const html = renderFallbackStage(data);
    expect(html).toContain("<pre>");
    expect(html).toContain("<code>");
    expect(html).toContain("custom_field");
    expect(html).toContain("value");
  });

  it("should escape HTML in JSON values", () => {
    const data = { html: "<script>alert('xss')</script>" };
    const html = renderFallbackStage(data);
    expect(html).toContain("&lt;script&gt;");
    expect(html).not.toContain("<script>");
  });
});
