/**
 * Tests for the Verilog Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes
 * Verilog (IEEE 1364-2005) source code when loaded with the `verilog.tokens`
 * grammar file, and that the preprocessor correctly handles compiler directives.
 *
 * Verilog is a Hardware Description Language — the token types and keywords
 * are specific to hardware design (module, wire, reg, assign, posedge, etc.),
 * and the number format includes hardware-specific features like sized literals
 * (4'b1010) and four-value logic (x for unknown, z for high-impedance).
 */

import { describe, it, expect } from "vitest";
import {
  tokenizeVerilog,
  createVerilogLexer,
  verilogPreprocess,
} from "../src/index.js";

// ============================================================================
// Helpers
// ============================================================================

/**
 * Helper: extract just the token types from a Verilog source string.
 * Makes assertions concise — compare arrays of type strings instead
 * of inspecting full Token objects.
 */
function tokenTypes(source: string, preprocess = true): string[] {
  return tokenizeVerilog(source, { preprocess }).map((t) => t.type);
}

/**
 * Helper: extract just the token values from a Verilog source string.
 */
function tokenValues(source: string, preprocess = true): string[] {
  return tokenizeVerilog(source, { preprocess }).map((t) => t.value);
}

// ============================================================================
// Basic tokenization
// ============================================================================

describe("basic expressions", () => {
  it("tokenizes a simple assignment statement", () => {
    /**
     * The simplest Verilog continuous assignment:
     *     assign y = a & b;
     *
     * 'assign' is a keyword that creates a continuous connection —
     * whenever a or b changes, y updates immediately.
     */
    const types = tokenTypes("assign y = a & b;");
    expect(types).toEqual([
      "KEYWORD", "NAME", "EQUALS", "NAME", "AMP", "NAME",
      "SEMICOLON", "EOF",
    ]);
  });

  it("captures correct values", () => {
    const tokens = tokenizeVerilog("assign y = a & b;");
    const meaningful = tokens.filter(
      (t) => t.type !== "EOF",
    );
    expect(meaningful.map((t) => t.value)).toEqual([
      "assign", "y", "=", "a", "&", "b", ";",
    ]);
  });

  it("tokenizes all single-character operators", () => {
    /**
     * Verilog has the standard set of operators. The & | ^ ~ operators
     * serve double duty: binary (a & b) and reduction (&a) operations.
     * The lexer produces the same token type; the parser distinguishes.
     */
    const types = tokenTypes("a + b - c * d / e % f");
    expect(types).toEqual([
      "NAME", "PLUS", "NAME", "MINUS", "NAME",
      "STAR", "NAME", "SLASH", "NAME", "PERCENT", "NAME",
      "EOF",
    ]);
  });

  it("tokenizes bitwise operators", () => {
    const types = tokenTypes("a & b | c ^ d");
    expect(types).toContain("AMP");
    expect(types).toContain("PIPE");
    expect(types).toContain("CARET");
  });

  it("tokenizes unary operators", () => {
    const types = tokenTypes("~a");
    expect(types).toContain("TILDE");

    const types2 = tokenTypes("!b");
    expect(types2).toContain("BANG");
  });
});

// ============================================================================
// Multi-character operators
// ============================================================================

describe("multi-character operators", () => {
  it("tokenizes == and != comparison operators", () => {
    const types = tokenTypes("a == b");
    expect(types).toContain("EQUALS_EQUALS");

    const types2 = tokenTypes("a != b");
    expect(types2).toContain("NOT_EQUALS");
  });

  it("tokenizes <= and >= comparison operators", () => {
    /**
     * Note: In Verilog, <= serves double duty:
     *   - As a comparison operator: if (a <= b)
     *   - As non-blocking assignment: q <= d  (in always blocks)
     * The lexer produces the same token; the parser distinguishes by context.
     */
    const types = tokenTypes("a <= b");
    expect(types).toContain("LESS_EQUALS");

    const types2 = tokenTypes("a >= b");
    expect(types2).toContain("GREATER_EQUALS");
  });

  it("tokenizes shift operators", () => {
    const types = tokenTypes("a << 2");
    expect(types).toContain("LEFT_SHIFT");

    const types2 = tokenTypes("a >> 2");
    expect(types2).toContain("RIGHT_SHIFT");
  });

  it("tokenizes arithmetic shift operators", () => {
    /**
     * Arithmetic shifts preserve the sign bit:
     *   a >>> 2  — shift right, fill with sign bit (not zero)
     *   a <<< 2  — shift left (same as << but semantically distinct)
     */
    const types = tokenTypes("a >>> 2");
    expect(types).toContain("ARITH_RIGHT_SHIFT");

    const types2 = tokenTypes("a <<< 2");
    expect(types2).toContain("ARITH_LEFT_SHIFT");
  });

  it("tokenizes case equality operators", () => {
    /**
     * Case equality compares including x and z values:
     *   a === b  — true only if ALL bits match (including x and z)
     *   a !== b  — true if any bit differs
     *
     * Regular == returns x if either operand has x bits.
     */
    const types = tokenTypes("a === b");
    expect(types).toContain("CASE_EQ");

    const types2 = tokenTypes("a !== b");
    expect(types2).toContain("CASE_NEQ");
  });

  it("tokenizes logical operators", () => {
    const types = tokenTypes("a && b || c");
    expect(types).toContain("LOGIC_AND");
    expect(types).toContain("LOGIC_OR");
  });

  it("tokenizes power operator", () => {
    const types = tokenTypes("2 ** 10");
    expect(types).toContain("POWER");
  });

  it("tokenizes trigger operator", () => {
    /**
     * The -> operator is used in event triggers:
     *   -> event_name;  (triggers a named event)
     */
    const types = tokenTypes("a -> b");
    expect(types).toContain("TRIGGER");
  });
});

// ============================================================================
// Keywords
// ============================================================================

describe("Verilog keywords", () => {
  it("recognizes module structure keywords", () => {
    /**
     * Module is the fundamental building block in Verilog — it describes
     * a hardware component with inputs and outputs.
     */
    const tokens = tokenizeVerilog("module endmodule");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["module", "endmodule"]);
  });

  it("recognizes port direction keywords", () => {
    const tokens = tokenizeVerilog("input output inout");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["input", "output", "inout"]);
  });

  it("recognizes data type keywords", () => {
    /**
     * wire = combinational signal (no storage, like a physical wire)
     * reg  = sequential signal (stores a value, like a flip-flop)
     */
    const tokens = tokenizeVerilog("wire reg integer");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["wire", "reg", "integer"]);
  });

  it("recognizes procedural keywords", () => {
    const tokens = tokenizeVerilog("always initial begin end if else");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "always", "initial", "begin", "end", "if", "else",
    ]);
  });

  it("recognizes case keywords", () => {
    const tokens = tokenizeVerilog("case casex casez endcase default");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "case", "casex", "casez", "endcase", "default",
    ]);
  });

  it("recognizes sensitivity keywords", () => {
    /**
     * posedge/negedge specify clock edge sensitivity:
     *   always @(posedge clk)  — triggers on rising edge of clock
     *   always @(negedge clk)  — triggers on falling edge
     */
    const tokens = tokenizeVerilog("posedge negedge or");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["posedge", "negedge", "or"]);
  });

  it("recognizes assign keyword", () => {
    const tokens = tokenizeVerilog("assign");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("assign");
  });

  it("recognizes gate primitive keywords", () => {
    const tokens = tokenizeVerilog("and nand or nor xor xnor not buf");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "and", "nand", "or", "nor", "xor", "xnor", "not", "buf",
    ]);
  });

  it("recognizes parameter keywords", () => {
    const tokens = tokenizeVerilog("parameter localparam");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["parameter", "localparam"]);
  });

  it("recognizes function and task keywords", () => {
    const tokens = tokenizeVerilog("function endfunction task endtask");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "function", "endfunction", "task", "endtask",
    ]);
  });

  it("recognizes generate keywords", () => {
    const tokens = tokenizeVerilog("generate endgenerate genvar");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "generate", "endgenerate", "genvar",
    ]);
  });
});

// ============================================================================
// Number literals
// ============================================================================

describe("number literals", () => {
  it("tokenizes plain integers", () => {
    const tokens = tokenizeVerilog("42");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("42");
  });

  it("tokenizes integers with underscores", () => {
    /**
     * Underscores are visual separators in Verilog numbers, ignored by
     * the language. They improve readability of large values:
     *   1_000_000 is the same as 1000000
     */
    const tokens = tokenizeVerilog("1_000_000");
    expect(tokens[0].type).toBe("NUMBER");
  });

  it("tokenizes sized binary literals", () => {
    /**
     * 4'b1010 means: a 4-bit value, in binary, equal to 1010 (decimal 10).
     * The format is: [size]'[base]digits
     *   size = number of bits
     *   base = b (binary), o (octal), d (decimal), h (hex)
     */
    const tokens = tokenizeVerilog("4'b1010");
    expect(tokens[0].type).toBe("SIZED_NUMBER");
    expect(tokens[0].value).toBe("4'b1010");
  });

  it("tokenizes sized hex literals", () => {
    const tokens = tokenizeVerilog("8'hFF");
    expect(tokens[0].type).toBe("SIZED_NUMBER");
    expect(tokens[0].value).toBe("8'hFF");
  });

  it("tokenizes sized decimal literals", () => {
    const tokens = tokenizeVerilog("32'd42");
    expect(tokens[0].type).toBe("SIZED_NUMBER");
  });

  it("tokenizes unsized based literals", () => {
    /**
     * Omitting the size gives a default width (implementation-defined,
     * typically 32 bits). The leading ' is still required for the base.
     */
    const tokens = tokenizeVerilog("'o77");
    expect(tokens[0].type).toBe("SIZED_NUMBER");
  });

  it("tokenizes sized literals with x and z values", () => {
    /**
     * x = unknown (uninitialized flip-flop, floating logic)
     * z = high-impedance (disconnected wire, tri-state buffer)
     * These are physical states unique to hardware:
     *   4'bxxzz means "4 bits: unknown, unknown, high-Z, high-Z"
     */
    const tokens = tokenizeVerilog("4'bxxzz");
    expect(tokens[0].type).toBe("SIZED_NUMBER");
  });

  it("tokenizes signed sized literals", () => {
    /**
     * The 's' modifier makes the literal signed:
     *   8'sd255 is a signed 8-bit value (-1 in two's complement)
     */
    const tokens = tokenizeVerilog("8'sd255");
    expect(tokens[0].type).toBe("SIZED_NUMBER");
  });

  it("tokenizes real numbers", () => {
    const tokens = tokenizeVerilog("3.14");
    expect(tokens[0].type).toBe("REAL_NUMBER");

    const tokens2 = tokenizeVerilog("1.5e-3");
    expect(tokens2[0].type).toBe("REAL_NUMBER");
  });
});

// ============================================================================
// String literals
// ============================================================================

describe("string literals", () => {
  it("tokenizes double-quoted strings", () => {
    const tokens = tokenizeVerilog('"Hello, world!"');
    expect(tokens[0].type).toBe("STRING");
  });

  it("tokenizes strings with escape sequences", () => {
    const tokens = tokenizeVerilog('"Value: %d\\n"');
    expect(tokens[0].type).toBe("STRING");
  });
});

// ============================================================================
// Special identifiers
// ============================================================================

describe("special identifiers", () => {
  it("tokenizes system tasks with $ prefix", () => {
    /**
     * System tasks are built-in simulation functions:
     *   $display("x = %d", x)  — print to console
     *   $finish                — end simulation
     *   $time                  — get current simulation time
     */
    const tokens = tokenizeVerilog("$display");
    expect(tokens[0].type).toBe("SYSTEM_ID");
    expect(tokens[0].value).toBe("$display");
  });

  it("tokenizes various system tasks", () => {
    const tokens = tokenizeVerilog("$finish $time $random");
    const systemIds = tokens.filter((t) => t.type === "SYSTEM_ID");
    expect(systemIds.map((t) => t.value)).toEqual([
      "$finish", "$time", "$random",
    ]);
  });

  it("tokenizes compiler directives with backtick prefix", () => {
    /**
     * Compiler directives start with backtick (`). When preprocessing
     * is disabled, they appear as DIRECTIVE tokens.
     */
    const tokens = tokenizeVerilog("`default_nettype", { preprocess: false });
    expect(tokens[0].type).toBe("DIRECTIVE");
    expect(tokens[0].value).toBe("`default_nettype");
  });

  it("tokenizes escaped identifiers", () => {
    /**
     * Escaped identifiers start with backslash and end at whitespace.
     * They allow any characters in identifier names:
     *   \my.odd.name   — dots in identifier
     *   \bus[0]        — brackets in identifier
     */
    const tokens = tokenizeVerilog("\\my.odd.name ");
    expect(tokens[0].type).toBe("ESCAPED_IDENT");
  });
});

// ============================================================================
// Delimiters
// ============================================================================

describe("delimiters", () => {
  it("tokenizes parentheses, brackets, braces", () => {
    const types = tokenTypes("( ) [ ] { }");
    expect(types).toEqual([
      "LPAREN", "RPAREN", "LBRACKET", "RBRACKET",
      "LBRACE", "RBRACE", "EOF",
    ]);
  });

  it("tokenizes semicolons, commas, dots", () => {
    const types = tokenTypes("; , .");
    expect(types).toEqual(["SEMICOLON", "COMMA", "DOT", "EOF"]);
  });

  it("tokenizes hash and at symbols", () => {
    /**
     * # is used for delays and parameter overrides:
     *   #10         — delay 10 time units
     *   #(8, 16)    — parameter override
     *
     * @ is used for sensitivity lists:
     *   @(posedge clk)  — trigger on clock rising edge
     *   @(*)            — sensitivity to all read signals
     */
    const types = tokenTypes("# @");
    expect(types).toEqual(["HASH", "AT", "EOF"]);
  });

  it("tokenizes ternary operator components", () => {
    /**
     * Verilog uses ? : for conditional (mux) operations:
     *   assign y = sel ? a : b;
     * This describes a multiplexer in hardware.
     */
    const types = tokenTypes("sel ? a : b");
    expect(types).toContain("QUESTION");
    expect(types).toContain("COLON");
  });
});

// ============================================================================
// Comment skipping
// ============================================================================

describe("comment skipping", () => {
  it("skips single-line comments", () => {
    const types = tokenTypes("wire a; // this is a comment");
    expect(types).not.toContain("COMMENT");
    expect(types).toEqual([
      "KEYWORD", "NAME", "SEMICOLON", "EOF",
    ]);
  });

  it("skips block comments", () => {
    const types = tokenTypes("wire /* block comment */ a;");
    expect(types).not.toContain("COMMENT");
    expect(types).toEqual([
      "KEYWORD", "NAME", "SEMICOLON", "EOF",
    ]);
  });
});

// ============================================================================
// Position tracking
// ============================================================================

describe("position tracking", () => {
  it("tracks line and column for each token", () => {
    const tokens = tokenizeVerilog("wire a;\nreg b;");

    expect(tokens[0].line).toBe(1);
    expect(tokens[0].column).toBe(1);

    const regToken = tokens.find((t) => t.value === "reg");
    expect(regToken).toBeDefined();
    expect(regToken!.line).toBe(2);
  });
});

// ============================================================================
// createVerilogLexer
// ============================================================================

describe("createVerilogLexer", () => {
  it("returns a GrammarLexer that produces the same tokens", () => {
    /**
     * createVerilogLexer returns a GrammarLexer instance rather than
     * an array of tokens. Calling .tokenize() on it should produce
     * the same result as tokenizeVerilog.
     */
    const source = "assign y = a & b;";
    const directTokens = tokenizeVerilog(source);
    const lexer = createVerilogLexer(source);
    const lexerTokens = lexer.tokenize();

    expect(lexerTokens.map((t) => t.type)).toEqual(
      directTokens.map((t) => t.type),
    );
  });

  it("supports the preprocess option", () => {
    const source = "`define W 8\nreg [`W-1:0] data;";
    const lexer = createVerilogLexer(source, { preprocess: true });
    const tokens = lexer.tokenize();
    const values = tokens.map((t) => t.value);
    expect(values).toContain("8");
    expect(values).not.toContain("`W");
  });

  it("can be used without preprocessing", () => {
    const source = "`define W 8";
    const lexer = createVerilogLexer(source, { preprocess: false });
    const tokens = lexer.tokenize();
    expect(tokens[0].type).toBe("DIRECTIVE");
    expect(tokens[0].value).toBe("`define");
  });
});

// ============================================================================
// Complete Verilog constructs
// ============================================================================

describe("complete Verilog constructs", () => {
  it("tokenizes a simple module declaration", () => {
    /**
     * A Verilog module is the fundamental hardware building block.
     * This declares a module named 'top' with no ports.
     */
    const source = "module top; endmodule";
    const types = tokenTypes(source);
    expect(types).toEqual([
      "KEYWORD", "NAME", "SEMICOLON", "KEYWORD", "EOF",
    ]);
  });

  it("tokenizes a module with ports", () => {
    const source = "module adder(input a, input b, output sum);";
    const tokens = tokenizeVerilog(source);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "module", "input", "input", "output",
    ]);
  });

  it("tokenizes an always block with sensitivity list", () => {
    /**
     * always @(posedge clk) describes a flip-flop:
     * "every time the clock rises, execute this block"
     */
    const source = "always @(posedge clk) begin end";
    const types = tokenTypes(source);
    expect(types).toContain("KEYWORD");
    expect(types).toContain("AT");
    expect(types).toContain("LPAREN");
  });

  it("tokenizes a wire declaration with bit range", () => {
    /**
     * wire [7:0] data; declares an 8-bit wire (bus).
     * The [7:0] notation specifies MSB:LSB bit indices.
     */
    const source = "wire [7:0] data;";
    const types = tokenTypes(source);
    expect(types).toEqual([
      "KEYWORD", "LBRACKET", "NUMBER", "COLON",
      "NUMBER", "RBRACKET", "NAME", "SEMICOLON", "EOF",
    ]);
  });

  it("tokenizes a continuous assignment", () => {
    const source = "assign sum = a ^ b;";
    const types = tokenTypes(source);
    expect(types).toEqual([
      "KEYWORD", "NAME", "EQUALS", "NAME", "CARET", "NAME",
      "SEMICOLON", "EOF",
    ]);
  });

  it("tokenizes a gate instantiation", () => {
    /**
     * Built-in gate primitives describe individual logic gates:
     *   and g1(out, in1, in2);
     * This creates an AND gate named g1 with output 'out'
     * and inputs 'in1', 'in2'.
     */
    const source = "and g1(out, in1, in2);";
    const types = tokenTypes(source);
    expect(types).toEqual([
      "KEYWORD", "NAME", "LPAREN", "NAME", "COMMA",
      "NAME", "COMMA", "NAME", "RPAREN", "SEMICOLON", "EOF",
    ]);
  });
});

// ============================================================================
// Preprocessor tests
// ============================================================================

describe("preprocessor — verilogPreprocess", () => {
  describe("`define and macro expansion", () => {
    it("expands simple macros", () => {
      const source = "`define WIDTH 8\nreg [`WIDTH-1:0] data;";
      const result = verilogPreprocess(source);
      expect(result).toContain("8-1:0");
      expect(result).not.toContain("`WIDTH");
    });

    it("handles macros with no body (flag macros)", () => {
      /**
       * A macro with no body is used as a flag for `ifdef/`ifndef.
       * `define DEBUG is equivalent to `define DEBUG (empty body).
       */
      const source = "`define DEBUG\n`ifdef DEBUG\nactive\n`endif";
      const result = verilogPreprocess(source);
      expect(result).toContain("active");
    });

    it("expands parameterized macros", () => {
      const source = "`define MAX(a, b) ((a) > (b) ? (a) : (b))\nassign result = `MAX(x, y);";
      const result = verilogPreprocess(source);
      expect(result).toContain("((x) > (y) ? (x) : (y))");
    });

    it("handles parameterized macros with nested parens", () => {
      const source = "`define ADD(a, b) ((a) + (b))\nassign z = `ADD((x+1), (y+2));";
      const result = verilogPreprocess(source);
      expect(result).toContain("(((x+1)) + ((y+2)))");
    });

    it("leaves undefined macros as-is", () => {
      const source = "assign y = `UNDEFINED;";
      const result = verilogPreprocess(source);
      expect(result).toContain("`UNDEFINED");
    });

    it("handles multiple macro definitions", () => {
      const source = "`define A 1\n`define B 2\nassign x = `A + `B;";
      const result = verilogPreprocess(source);
      expect(result).toContain("1 + 2");
    });
  });

  describe("`undef", () => {
    it("removes a previously defined macro", () => {
      const source = "`define WIDTH 8\n`undef WIDTH\nassign x = `WIDTH;";
      const result = verilogPreprocess(source);
      // After undef, `WIDTH should NOT be expanded
      expect(result).toContain("`WIDTH");
    });

    it("makes ifdef false after undef", () => {
      const source = "`define FLAG\n`undef FLAG\n`ifdef FLAG\nshould_not_appear\n`endif";
      const result = verilogPreprocess(source);
      expect(result).not.toContain("should_not_appear");
    });
  });

  describe("`ifdef / `ifndef / `else / `endif", () => {
    it("includes code when ifdef condition is true", () => {
      const source = "`define DEBUG\n`ifdef DEBUG\nwire debug_out;\n`endif";
      const result = verilogPreprocess(source);
      expect(result).toContain("wire debug_out;");
    });

    it("excludes code when ifdef condition is false", () => {
      const source = "`ifdef DEBUG\nwire debug_out;\n`endif";
      const result = verilogPreprocess(source);
      expect(result).not.toContain("wire debug_out;");
    });

    it("handles else branch", () => {
      const source = "`ifdef DEBUG\nwire debug;\n`else\nwire release;\n`endif";
      const result = verilogPreprocess(source);
      expect(result).not.toContain("wire debug;");
      expect(result).toContain("wire release;");
    });

    it("handles ifdef with else when condition is true", () => {
      const source = "`define DEBUG\n`ifdef DEBUG\nwire debug;\n`else\nwire release;\n`endif";
      const result = verilogPreprocess(source);
      expect(result).toContain("wire debug;");
      expect(result).not.toContain("wire release;");
    });

    it("handles ifndef (include if NOT defined)", () => {
      const source = "`ifndef FEATURE_X\nwire fallback;\n`endif";
      const result = verilogPreprocess(source);
      expect(result).toContain("wire fallback;");
    });

    it("handles ifndef when macro IS defined", () => {
      const source = "`define FEATURE_X\n`ifndef FEATURE_X\nwire fallback;\n`endif";
      const result = verilogPreprocess(source);
      expect(result).not.toContain("wire fallback;");
    });

    it("handles nested conditionals", () => {
      const source = [
        "`define A",
        "`define B",
        "`ifdef A",
        "`ifdef B",
        "both_defined",
        "`endif",
        "`endif",
      ].join("\n");
      const result = verilogPreprocess(source);
      expect(result).toContain("both_defined");
    });

    it("handles nested conditionals where inner is false", () => {
      const source = [
        "`define A",
        "`ifdef A",
        "`ifdef B",
        "should_not_appear",
        "`else",
        "b_not_defined",
        "`endif",
        "`endif",
      ].join("\n");
      const result = verilogPreprocess(source);
      expect(result).not.toContain("should_not_appear");
      expect(result).toContain("b_not_defined");
    });

    it("handles nested conditionals where outer is false", () => {
      const source = [
        "`ifdef A",
        "`ifdef B",
        "inner_code",
        "`endif",
        "`endif",
      ].join("\n");
      const result = verilogPreprocess(source);
      expect(result).not.toContain("inner_code");
    });

    it("does not define macros in inactive regions", () => {
      const source = "`ifdef NEVER\n`define SECRET 42\n`endif\nassign x = `SECRET;";
      const result = verilogPreprocess(source);
      expect(result).toContain("`SECRET");
    });
  });

  describe("`include (stubbed)", () => {
    it("replaces include with a stub comment", () => {
      const source = '`include "types.v"';
      const result = verilogPreprocess(source);
      expect(result).toContain("stubbed");
      expect(result).toContain("types.v");
    });
  });

  describe("`timescale (stripped)", () => {
    it("strips timescale directives", () => {
      const source = "`timescale 1ns/1ps\nmodule top;";
      const result = verilogPreprocess(source);
      expect(result).not.toContain("timescale");
      expect(result).toContain("module top;");
    });
  });
});

// ============================================================================
// Preprocessor integration with tokenizer
// ============================================================================

describe("preprocessor integration", () => {
  it("tokenizes after macro expansion", () => {
    const source = "`define WIDTH 8\nwire [`WIDTH-1:0] data;";
    const tokens = tokenizeVerilog(source);
    const values = tokens.map((t) => t.value);
    expect(values).toContain("8");
    expect(values).not.toContain("`WIDTH");
  });

  it("skips code in false ifdef branches", () => {
    const source = "`ifdef DEBUG\nwire debug_sig;\n`endif\nwire real_sig;";
    const tokens = tokenizeVerilog(source);
    const names = tokens
      .filter((t) => t.type === "NAME")
      .map((t) => t.value);
    expect(names).not.toContain("debug_sig");
    expect(names).toContain("real_sig");
  });

  it("can disable preprocessing", () => {
    const source = "`define WIDTH 8";
    const tokens = tokenizeVerilog(source, { preprocess: false });
    expect(tokens[0].type).toBe("DIRECTIVE");
    expect(tokens[0].value).toBe("`define");
  });

  it("handles timescale before module", () => {
    const source = "`timescale 1ns/1ps\nmodule top; endmodule";
    const tokens = tokenizeVerilog(source);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords[0].value).toBe("module");
  });
});

// ============================================================================
// Edge cases
// ============================================================================

describe("edge cases", () => {
  it("tokenizes empty input", () => {
    const tokens = tokenizeVerilog("");
    expect(tokens.length).toBe(1);
    expect(tokens[0].type).toBe("EOF");
  });

  it("tokenizes whitespace-only input", () => {
    const tokens = tokenizeVerilog("   \n\n   ");
    expect(tokens[tokens.length - 1].type).toBe("EOF");
  });

  it("handles multiple semicolons", () => {
    const types = tokenTypes(";;;");
    expect(types).toEqual(["SEMICOLON", "SEMICOLON", "SEMICOLON", "EOF"]);
  });

  it("handles identifiers with dollar signs", () => {
    /**
     * Regular identifiers in Verilog can contain $ characters
     * (after the first character).
     */
    const tokens = tokenizeVerilog("my_var$1");
    expect(tokens[0].type).toBe("NAME");
  });
});
