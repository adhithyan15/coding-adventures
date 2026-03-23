/**
 * Tests for the VHDL Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes
 * VHDL (IEEE 1076-2008) source code when loaded with the `vhdl.tokens`
 * grammar file, and that VHDL-specific post-processing (case normalization)
 * works correctly.
 *
 * VHDL is a Hardware Description Language — the token types and keywords
 * are specific to hardware design (entity, architecture, signal, process, etc.),
 * and the syntax is Ada-like with strong typing and explicit declarations.
 *
 * Key differences from Verilog tests:
 *   - No preprocessor tests (VHDL has no preprocessor)
 *   - Case normalization tests (VHDL is case-insensitive)
 *   - Character literals and bit strings (VHDL-specific literal forms)
 *   - Keyword operators (and, or, xor, not — instead of &, |, ^, ~)
 */

import { describe, it, expect } from "vitest";
import { tokenizeVhdl, createVhdlLexer } from "../src/index.js";

// ============================================================================
// Helpers
// ============================================================================

/**
 * Helper: extract just the token types from a VHDL source string.
 * Makes assertions concise — compare arrays of type strings instead
 * of inspecting full Token objects.
 */
function tokenTypes(source: string): string[] {
  return tokenizeVhdl(source).map((t) => t.type);
}

/**
 * Helper: extract just the token values from a VHDL source string.
 */
function tokenValues(source: string): string[] {
  return tokenizeVhdl(source).map((t) => t.value);
}

// ============================================================================
// Case Insensitivity
// ============================================================================

describe("case insensitivity", () => {
  it("normalizes keyword values to lowercase", () => {
    /**
     * VHDL is case-insensitive: ENTITY, Entity, and entity are the same.
     * After tokenization, all KEYWORD values must be lowercased.
     */
    const tokens = tokenizeVhdl("ENTITY my_chip IS END ENTITY;");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "entity", "is", "end", "entity",
    ]);
  });

  it("normalizes NAME values to lowercase", () => {
    /**
     * Identifiers (NAME tokens) are also case-insensitive:
     * My_Signal, MY_SIGNAL, and my_signal all refer to the same object.
     */
    const tokens = tokenizeVhdl("signal MY_SIG : std_logic;");
    const names = tokens.filter((t) => t.type === "NAME");
    expect(names.map((n) => n.value)).toEqual(["my_sig", "std_logic"]);
  });

  it("lowercases string literal values (source-level lowercasing)", () => {
    /**
     * With case_sensitive: false, the base lexer lowercases the entire
     * source text before tokenization, so string literal content is
     * also lowercased.
     */
    const tokens = tokenizeVhdl('"Hello World"');
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("hello world");
  });

  it("lowercases bit string literal values", () => {
    /**
     * Bit string literals like X"FF" are lowercased by the base lexer
     * since case_sensitive: false lowercases the entire source text.
     */
    const tokens = tokenizeVhdl('X"FF"');
    expect(tokens[0].type).toBe("BIT_STRING");
    expect(tokens[0].value).toBe('x"ff"');
  });

  it("handles mixed case keywords correctly", () => {
    /**
     * All variations of casing should produce the same lowercase keyword.
     */
    const tokens1 = tokenizeVhdl("Entity");
    const tokens2 = tokenizeVhdl("ENTITY");
    const tokens3 = tokenizeVhdl("entity");
    const tokens4 = tokenizeVhdl("eNtItY");

    expect(tokens1[0].value).toBe("entity");
    expect(tokens2[0].value).toBe("entity");
    expect(tokens3[0].value).toBe("entity");
    expect(tokens4[0].value).toBe("entity");

    // All should be recognized as KEYWORD, not NAME
    expect(tokens1[0].type).toBe("KEYWORD");
    expect(tokens2[0].type).toBe("KEYWORD");
    expect(tokens3[0].type).toBe("KEYWORD");
    expect(tokens4[0].type).toBe("KEYWORD");
  });

  it("lowercases character literal values (source-level lowercasing)", () => {
    const tokens = tokenizeVhdl("'A'");
    expect(tokens[0].type).toBe("CHAR_LITERAL");
    expect(tokens[0].value).toBe("'a'");
  });
});

// ============================================================================
// Entity Declaration
// ============================================================================

describe("entity declarations", () => {
  it("tokenizes a simple entity declaration", () => {
    /**
     * An entity in VHDL declares the interface of a hardware component.
     * It lists the ports (inputs and outputs) that the outside world
     * can connect to. Think of it as the "pin layout" of a chip.
     *
     *     entity my_chip is
     *     end entity my_chip;
     */
    const source = "entity my_chip is end entity my_chip;";
    const types = tokenTypes(source);
    expect(types).toEqual([
      "KEYWORD", "NAME", "KEYWORD", "KEYWORD", "KEYWORD", "NAME",
      "SEMICOLON", "EOF",
    ]);
  });

  it("tokenizes entity with port declarations", () => {
    /**
     * Ports define the external connections of a component:
     *   in  = input port (read-only from inside)
     *   out = output port (write-only from inside)
     */
    const source = `entity adder is
      port(
        a : in std_logic;
        b : in std_logic;
        sum : out std_logic
      );
    end entity adder;`;

    const tokens = tokenizeVhdl(source);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toContain("entity");
    expect(keywords.map((k) => k.value)).toContain("port");
    expect(keywords.map((k) => k.value)).toContain("in");
    expect(keywords.map((k) => k.value)).toContain("out");
    expect(keywords.map((k) => k.value)).toContain("end");
  });
});

// ============================================================================
// Architecture
// ============================================================================

describe("architecture declarations", () => {
  it("tokenizes a simple architecture", () => {
    /**
     * An architecture describes the IMPLEMENTATION of an entity.
     * VHDL strictly separates interface (entity) from implementation
     * (architecture), unlike Verilog which combines them in a module.
     *
     * One entity can have multiple architectures — for example, a
     * behavioral description for simulation and a structural one
     * for synthesis.
     *
     *     architecture rtl of my_chip is
     *     begin
     *     end architecture rtl;
     */
    const source = "architecture rtl of my_chip is begin end architecture rtl;";
    const types = tokenTypes(source);
    expect(types).toEqual([
      "KEYWORD", "NAME", "KEYWORD", "NAME", "KEYWORD",
      "KEYWORD", "KEYWORD", "KEYWORD", "NAME", "SEMICOLON", "EOF",
    ]);
  });

  it("tokenizes signal assignment in architecture", () => {
    /**
     * Signal assignment uses <= (LESS_EQUALS token), which also serves
     * as "less than or equal" in comparisons. The parser distinguishes
     * based on context; the lexer just produces a single token type.
     *
     *     y <= a and b;
     *
     * This creates a continuous connection: whenever a or b changes,
     * y updates. It's equivalent to Verilog's: assign y = a & b;
     */
    const source = "y <= a and b;";
    const types = tokenTypes(source);
    expect(types).toContain("LESS_EQUALS");
    expect(types).toContain("KEYWORD"); // "and" is a keyword
  });
});

// ============================================================================
// Character Literals
// ============================================================================

describe("character literals", () => {
  it("tokenizes std_logic character literals", () => {
    /**
     * VHDL's std_logic type uses character literals for its nine values:
     *   'U' — Uninitialized    'X' — Forcing unknown
     *   '0' — Forcing zero     '1' — Forcing one
     *   'Z' — High impedance   'W' — Weak unknown
     *   'L' — Weak zero        'H' — Weak one
     *   '-' — Don't care
     *
     * These are the building blocks of VHDL digital logic.
     */
    const tokens = tokenizeVhdl("'0'");
    expect(tokens[0].type).toBe("CHAR_LITERAL");
    expect(tokens[0].value).toBe("'0'");
  });

  it("tokenizes various character values", () => {
    const inputs   = ["'1'", "'X'", "'Z'", "'U'", "'H'", "'L'", "'-'"];
    const expected = ["'1'", "'x'", "'z'", "'u'", "'h'", "'l'", "'-'"];
    for (let i = 0; i < inputs.length; i++) {
      const tokens = tokenizeVhdl(inputs[i]);
      expect(tokens[0].type).toBe("CHAR_LITERAL");
      expect(tokens[0].value).toBe(expected[i]);
    }
  });

  it("distinguishes character literals from tick (attribute) access", () => {
    /**
     * The tick (') has two uses in VHDL:
     *   1. Character literals: '0', 'A', etc.
     *   2. Attribute access: signal'event, signal'length
     *
     * The grammar handles this by ordering: CHAR_LITERAL (which matches
     * 'X' — tick, single char, tick) comes before TICK (bare tick).
     * When 'event appears, 'e' would match CHAR_LITERAL, but "vent"
     * is left over. The correct parse is TICK NAME.
     *
     * In practice, the regex /'[^']'/ matches character literals, and
     * a standalone ' matches as TICK. The attribute access pattern
     * clk'event is tokenized as NAME TICK NAME.
     */
    const tokens = tokenizeVhdl("clk'event");
    // Should be: NAME("clk") TICK("'") NAME("event")
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("clk");
    // The middle token should be TICK, not CHAR_LITERAL
    expect(tokens[1].type).toBe("TICK");
    expect(tokens[2].type).toBe("NAME");
    expect(tokens[2].value).toBe("event");
  });
});

// ============================================================================
// Bit String Literals
// ============================================================================

describe("bit string literals", () => {
  it("tokenizes binary bit strings", () => {
    /**
     * B"1010" — binary string, each character is one bit.
     * This is VHDL's equivalent of Verilog's 4'b1010.
     */
    const tokens = tokenizeVhdl('B"1010"');
    expect(tokens[0].type).toBe("BIT_STRING");
    expect(tokens[0].value).toBe('b"1010"');
  });

  it("tokenizes hexadecimal bit strings", () => {
    /**
     * X"FF" — hex string, each character is four bits.
     * Equivalent to Verilog's 8'hFF.
     */
    const tokens = tokenizeVhdl('X"FF"');
    expect(tokens[0].type).toBe("BIT_STRING");
    expect(tokens[0].value).toBe('x"ff"');
  });

  it("tokenizes octal bit strings", () => {
    const tokens = tokenizeVhdl('O"77"');
    expect(tokens[0].type).toBe("BIT_STRING");
    expect(tokens[0].value).toBe('o"77"');
  });

  it("tokenizes decimal bit strings (VHDL-2008)", () => {
    /**
     * D"42" — decimal bit string (VHDL-2008 addition).
     * The value 42 is converted to binary representation.
     */
    const tokens = tokenizeVhdl('D"42"');
    expect(tokens[0].type).toBe("BIT_STRING");
    expect(tokens[0].value).toBe('d"42"');
  });

  it("handles lowercase bit string prefixes", () => {
    const tokens = tokenizeVhdl('x"FF"');
    expect(tokens[0].type).toBe("BIT_STRING");

    const tokens2 = tokenizeVhdl('b"1010"');
    expect(tokens2[0].type).toBe("BIT_STRING");
  });

  it("tokenizes bit strings with underscores", () => {
    /**
     * Underscores in bit strings are visual separators:
     *   X"DEAD_BEEF" is the same as X"DEADBEEF"
     */
    const tokens = tokenizeVhdl('X"DEAD_BEEF"');
    expect(tokens[0].type).toBe("BIT_STRING");
  });
});

// ============================================================================
// Number Literals
// ============================================================================

describe("number literals", () => {
  it("tokenizes plain integers", () => {
    const tokens = tokenizeVhdl("42");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("42");
  });

  it("tokenizes integers with underscores", () => {
    /**
     * Like Verilog, VHDL allows underscores as visual separators:
     *   1_000_000 is the same as 1000000
     */
    const tokens = tokenizeVhdl("1_000_000");
    expect(tokens[0].type).toBe("NUMBER");
  });

  it("tokenizes real numbers", () => {
    const tokens = tokenizeVhdl("3.14");
    expect(tokens[0].type).toBe("REAL_NUMBER");

    const tokens2 = tokenizeVhdl("1.5e-3");
    expect(tokens2[0].type).toBe("REAL_NUMBER");
  });

  it("tokenizes based literals", () => {
    /**
     * VHDL based literals use the format: base#digits#
     *   16#FF#    — hex 255
     *   2#1010#   — binary 10
     *   8#77#     — octal 63
     *
     * The base can be any value from 2 to 16. This is more explicit
     * than Verilog's format where the base is a single letter.
     */
    const tokens = tokenizeVhdl("16#FF#");
    expect(tokens[0].type).toBe("BASED_LITERAL");
    expect(tokens[0].value).toBe("16#ff#");
  });

  it("tokenizes binary based literals", () => {
    const tokens = tokenizeVhdl("2#1010#");
    expect(tokens[0].type).toBe("BASED_LITERAL");
  });

  it("tokenizes based literals with exponents", () => {
    const tokens = tokenizeVhdl("16#FF#E2");
    expect(tokens[0].type).toBe("BASED_LITERAL");
  });
});

// ============================================================================
// Operators
// ============================================================================

describe("operators", () => {
  it("tokenizes single-character arithmetic operators", () => {
    const types = tokenTypes("a + b - c * d / e");
    expect(types).toEqual([
      "NAME", "PLUS", "NAME", "MINUS", "NAME",
      "STAR", "NAME", "SLASH", "NAME",
      "EOF",
    ]);
  });

  it("tokenizes signal assignment operator", () => {
    /**
     * Signal assignment (<=) is the fundamental output statement
     * in VHDL. It schedules a signal update after a delta cycle:
     *
     *     y <= a and b;
     *
     * This is like Verilog's non-blocking assignment (<=).
     */
    const types = tokenTypes("y <= a");
    expect(types).toContain("LESS_EQUALS");
  });

  it("tokenizes variable assignment operator", () => {
    /**
     * Variable assignment (:=) updates immediately (no delta delay):
     *
     *     variable x : integer := 0;
     *     x := x + 1;
     *
     * Variables only exist inside processes and subprograms.
     */
    const types = tokenTypes("x := 0");
    expect(types).toContain("VAR_ASSIGN");
  });

  it("tokenizes comparison operators", () => {
    const types = tokenTypes("a >= b");
    expect(types).toContain("GREATER_EQUALS");
  });

  it("tokenizes not-equals operator", () => {
    /**
     * VHDL uses /= for not-equals (from Ada), not != like C/Verilog.
     */
    const types = tokenTypes("a /= b");
    expect(types).toContain("NOT_EQUALS");
  });

  it("tokenizes the arrow operator", () => {
    /**
     * The => operator is used in port maps and case statements:
     *   port map(a => input_a, b => input_b)
     *   when "00" => output <= '0';
     */
    const types = tokenTypes("a => b");
    expect(types).toContain("ARROW");
  });

  it("tokenizes the power operator", () => {
    const types = tokenTypes("2 ** 10");
    expect(types).toContain("POWER");
  });

  it("tokenizes the box operator", () => {
    /**
     * The <> operator indicates an unconstrained range:
     *   type byte_array is array(natural range <>) of byte;
     */
    const types = tokenTypes("range <>");
    expect(types).toContain("BOX");
  });

  it("tokenizes concatenation operator", () => {
    /**
     * VHDL's & operator is concatenation (NOT bitwise AND):
     *   "Hello" & " " & "World"  →  "Hello World"
     *   '1' & "010"              →  "1010"
     *
     * Bitwise AND in VHDL is the keyword 'and'.
     */
    const types = tokenTypes('"Hello" & " "');
    expect(types).toContain("AMPERSAND");
  });
});

// ============================================================================
// Keywords
// ============================================================================

describe("VHDL keywords", () => {
  it("recognizes entity/architecture structure keywords", () => {
    const tokens = tokenizeVhdl("entity architecture begin end");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "entity", "architecture", "begin", "end",
    ]);
  });

  it("recognizes port direction keywords", () => {
    const tokens = tokenizeVhdl("in out inout buffer");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "in", "out", "inout", "buffer",
    ]);
  });

  it("recognizes logical operator keywords", () => {
    /**
     * VHDL uses keyword operators for logic instead of symbols:
     *   and, or, xor, nand, nor, xnor, not
     *
     * This makes VHDL code read more like natural language:
     *   y <= (a and b) or (c xor d);
     *
     * Compare with Verilog: assign y = (a & b) | (c ^ d);
     */
    const tokens = tokenizeVhdl("and or xor nand nor xnor not");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "and", "or", "xor", "nand", "nor", "xnor", "not",
    ]);
  });

  it("recognizes shift operator keywords", () => {
    /**
     * VHDL has named shift operators (from VHDL-1993):
     *   sll — shift left logical    srl — shift right logical
     *   sla — shift left arithmetic  sra — shift right arithmetic
     *   rol — rotate left            ror — rotate right
     */
    const tokens = tokenizeVhdl("sll srl sla sra rol ror");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "sll", "srl", "sla", "sra", "rol", "ror",
    ]);
  });

  it("recognizes arithmetic keyword operators", () => {
    const tokens = tokenizeVhdl("mod rem abs");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["mod", "rem", "abs"]);
  });

  it("recognizes data object keywords", () => {
    const tokens = tokenizeVhdl("signal variable constant");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "signal", "variable", "constant",
    ]);
  });

  it("recognizes type keywords", () => {
    const tokens = tokenizeVhdl("type subtype array record");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "type", "subtype", "array", "record",
    ]);
  });

  it("recognizes control flow keywords", () => {
    const tokens = tokenizeVhdl("if then else elsif case when");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "if", "then", "else", "elsif", "case", "when",
    ]);
  });

  it("recognizes loop keywords", () => {
    const tokens = tokenizeVhdl("for while loop exit next");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "for", "while", "loop", "exit", "next",
    ]);
  });

  it("recognizes process keyword", () => {
    /**
     * A process in VHDL is a sequential execution block — similar to
     * Verilog's always block. It runs whenever its sensitivity list
     * signals change.
     */
    const tokens = tokenizeVhdl("process");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("process");
  });

  it("recognizes generate keyword", () => {
    const tokens = tokenizeVhdl("generate");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("generate");
  });

  it("recognizes component keyword", () => {
    const tokens = tokenizeVhdl("component");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("component");
  });

  it("recognizes library and use keywords", () => {
    const tokens = tokenizeVhdl("library use");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["library", "use"]);
  });

  it("recognizes package keyword", () => {
    const tokens = tokenizeVhdl("package body");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["package", "body"]);
  });

  it("recognizes range keywords", () => {
    const tokens = tokenizeVhdl("to downto range");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["to", "downto", "range"]);
  });

  it("recognizes port and generic keywords", () => {
    const tokens = tokenizeVhdl("port generic map");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["port", "generic", "map"]);
  });

  it("recognizes function and procedure keywords", () => {
    const tokens = tokenizeVhdl("function procedure return");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual([
      "function", "procedure", "return",
    ]);
  });
});

// ============================================================================
// Comments
// ============================================================================

describe("comment skipping", () => {
  it("skips single-line comments (-- style)", () => {
    /**
     * VHDL uses -- for comments (from Ada), not // like C/Verilog:
     *   signal clk : std_logic; -- this is a comment
     */
    const types = tokenTypes("signal clk : std_logic; -- clock input");
    expect(types).not.toContain("COMMENT");
    expect(types).toEqual([
      "KEYWORD", "NAME", "COLON", "NAME", "SEMICOLON", "EOF",
    ]);
  });

  it("handles comment-only lines", () => {
    const tokens = tokenizeVhdl("-- this is just a comment");
    expect(tokens.length).toBe(1);
    expect(tokens[0].type).toBe("EOF");
  });

  it("handles comments between code lines", () => {
    const source = "signal a : std_logic;\n-- comment\nsignal b : std_logic;";
    const tokens = tokenizeVhdl(source);
    const names = tokens.filter((t) => t.type === "NAME");
    expect(names.map((n) => n.value)).toEqual(["a", "std_logic", "b", "std_logic"]);
  });
});

// ============================================================================
// Delimiters
// ============================================================================

describe("delimiters", () => {
  it("tokenizes parentheses", () => {
    const types = tokenTypes("( )");
    expect(types).toEqual(["LPAREN", "RPAREN", "EOF"]);
  });

  it("tokenizes brackets", () => {
    const types = tokenTypes("[ ]");
    expect(types).toEqual(["LBRACKET", "RBRACKET", "EOF"]);
  });

  it("tokenizes semicolons, commas, dots, colons", () => {
    const types = tokenTypes("; , . :");
    expect(types).toEqual([
      "SEMICOLON", "COMMA", "DOT", "COLON", "EOF",
    ]);
  });
});

// ============================================================================
// String Literals
// ============================================================================

describe("string literals", () => {
  it("tokenizes double-quoted strings", () => {
    /**
     * The grammar lexer strips the surrounding double quotes from
     * string token values, so the value is the content only.
     */
    const tokens = tokenizeVhdl('"Hello, world!"');
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("hello, world!");
  });

  it("tokenizes strings with doubled quotes (VHDL escape)", () => {
    /**
     * VHDL escapes quotes by doubling them (no backslash escaping):
     *   "He said ""hello"""  contains:  He said "hello"
     */
    const tokens = tokenizeVhdl('"He said ""hello"""');
    expect(tokens[0].type).toBe("STRING");
  });
});

// ============================================================================
// Extended Identifiers
// ============================================================================

describe("extended identifiers", () => {
  it("tokenizes backslash-delimited extended identifiers", () => {
    /**
     * Extended identifiers allow special characters in names:
     *   \my odd name\     — spaces in identifier
     *   \VHDL-2008\       — hyphens in identifier
     *
     * Extended identifiers are case-SENSITIVE and are NOT normalized
     * to lowercase (they keep their original casing).
     */
    const tokens = tokenizeVhdl("\\my_ident\\");
    expect(tokens[0].type).toBe("EXTENDED_IDENT");
  });
});

// ============================================================================
// Position Tracking
// ============================================================================

describe("position tracking", () => {
  it("tracks line and column for each token", () => {
    const tokens = tokenizeVhdl("signal a : std_logic;\nsignal b : std_logic;");

    expect(tokens[0].line).toBe(1);
    expect(tokens[0].column).toBe(1);

    // Find the second 'signal' keyword (on line 2)
    const secondSignal = tokens.filter((t) => t.value === "signal")[1];
    expect(secondSignal).toBeDefined();
    expect(secondSignal!.line).toBe(2);
  });
});

// ============================================================================
// createVhdlLexer
// ============================================================================

describe("createVhdlLexer", () => {
  it("returns a GrammarLexer that produces tokens", () => {
    /**
     * createVhdlLexer returns a GrammarLexer instance rather than
     * an array of tokens. Calling .tokenize() on it should produce
     * tokens (though without case normalization applied).
     */
    const source = "entity my_chip is end entity;";
    const lexer = createVhdlLexer(source);
    const tokens = lexer.tokenize();

    expect(tokens.length).toBeGreaterThan(1);
    expect(tokens[tokens.length - 1].type).toBe("EOF");
  });

  it("produces raw (non-normalized) tokens", () => {
    /**
     * The GrammarLexer from createVhdlLexer does NOT apply case
     * normalization — that's done only by tokenizeVhdl. This is
     * useful when you need the original casing.
     */
    const source = "ENTITY";
    const lexer = createVhdlLexer(source);
    const tokens = lexer.tokenize();
    // Raw tokens may retain original casing
    // (Whether it's NAME or KEYWORD depends on the grammar matching)
    expect(tokens[0].type === "NAME" || tokens[0].type === "KEYWORD").toBe(true);
  });
});

// ============================================================================
// Complete VHDL Constructs
// ============================================================================

describe("complete VHDL constructs", () => {
  it("tokenizes a full entity with ports", () => {
    /**
     * A complete entity declaration with typed ports.
     * This is one of the most common VHDL constructs.
     */
    const source = `entity and_gate is
      port(
        a : in std_logic;
        b : in std_logic;
        y : out std_logic
      );
    end entity and_gate;`;

    const tokens = tokenizeVhdl(source);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    const names = tokens.filter((t) => t.type === "NAME");

    expect(keywords.map((k) => k.value)).toContain("entity");
    expect(keywords.map((k) => k.value)).toContain("port");
    expect(keywords.map((k) => k.value)).toContain("in");
    expect(keywords.map((k) => k.value)).toContain("out");
    expect(names.map((n) => n.value)).toContain("and_gate");
    expect(names.map((n) => n.value)).toContain("std_logic");
  });

  it("tokenizes an architecture with signal assignment", () => {
    const source = `architecture rtl of and_gate is
    begin
      y <= a and b;
    end architecture rtl;`;

    const tokens = tokenizeVhdl(source);
    const types = tokens.map((t) => t.type);

    expect(types).toContain("KEYWORD"); // architecture, of, is, begin, end, and
    expect(types).toContain("LESS_EQUALS"); // <=
    expect(types).toContain("NAME"); // rtl, and_gate, y, a, b
  });

  it("tokenizes a process block", () => {
    /**
     * A process is a sequential execution region. The sensitivity list
     * (in parentheses after 'process') determines when the process
     * re-executes — similar to Verilog's always @(...) block.
     */
    const source = `process(clk)
    begin
      if rising_edge(clk) then
        q <= d;
      end if;
    end process;`;

    const tokens = tokenizeVhdl(source);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toContain("process");
    expect(keywords.map((k) => k.value)).toContain("begin");
    expect(keywords.map((k) => k.value)).toContain("if");
    expect(keywords.map((k) => k.value)).toContain("then");
    expect(keywords.map((k) => k.value)).toContain("end");
  });

  it("tokenizes a library/use clause", () => {
    /**
     * VHDL uses libraries instead of includes/imports:
     *   library ieee;
     *   use ieee.std_logic_1164.all;
     *
     * This is analogous to Python's "import" or Java's "import".
     */
    const source = "library ieee; use ieee.std_logic_1164.all;";
    const tokens = tokenizeVhdl(source);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toContain("library");
    expect(keywords.map((k) => k.value)).toContain("use");
    expect(keywords.map((k) => k.value)).toContain("all");
  });

  it("tokenizes a component declaration", () => {
    const source = `component adder is
      port(a, b : in std_logic; sum : out std_logic);
    end component adder;`;

    const tokens = tokenizeVhdl(source);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toContain("component");
  });

  it("tokenizes a generic map with arrow operator", () => {
    /**
     * Generic maps use => to associate formal parameters with actuals:
     *   generic map(width => 8)
     */
    const source = "generic map(width => 8)";
    const tokens = tokenizeVhdl(source);
    const types = tokens.map((t) => t.type);
    expect(types).toContain("ARROW");
    expect(types).toContain("KEYWORD"); // generic, map
  });

  it("tokenizes a case statement", () => {
    const source = `case sel is
      when "00" => y <= a;
      when "01" => y <= b;
      when others => y <= '0';
    end case;`;

    const tokens = tokenizeVhdl(source);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toContain("case");
    expect(keywords.map((k) => k.value)).toContain("is");
    expect(keywords.map((k) => k.value)).toContain("when");
    expect(keywords.map((k) => k.value)).toContain("others");
    expect(keywords.map((k) => k.value)).toContain("end");
  });

  it("tokenizes a for-generate loop", () => {
    /**
     * Generate statements create repeated hardware structures:
     *   gen: for i in 0 to 7 generate
     *     ...
     *   end generate;
     */
    const source = "for i in 0 to 7 generate end generate;";
    const tokens = tokenizeVhdl(source);
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toContain("for");
    expect(keywords.map((k) => k.value)).toContain("in");
    expect(keywords.map((k) => k.value)).toContain("to");
    expect(keywords.map((k) => k.value)).toContain("generate");
  });

  it("tokenizes a complete design unit", () => {
    /**
     * A complete, minimal VHDL design unit with library, entity,
     * and architecture.
     */
    const source = `library ieee;
use ieee.std_logic_1164.all;

entity inverter is
  port(
    a : in std_logic;
    y : out std_logic
  );
end entity inverter;

architecture rtl of inverter is
begin
  y <= not a;
end architecture rtl;`;

    const tokens = tokenizeVhdl(source);

    // Should have library, use, entity, port, in, out, end (multiple),
    // architecture, of, is (multiple), begin, not
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    const kwValues = keywords.map((k) => k.value);
    expect(kwValues).toContain("library");
    expect(kwValues).toContain("use");
    expect(kwValues).toContain("entity");
    expect(kwValues).toContain("port");
    expect(kwValues).toContain("architecture");
    expect(kwValues).toContain("not");

    // All keyword values should be lowercase
    for (const kw of keywords) {
      expect(kw.value).toBe(kw.value.toLowerCase());
    }

    // All NAME values should be lowercase
    const names = tokens.filter((t) => t.type === "NAME");
    for (const name of names) {
      expect(name.value).toBe(name.value.toLowerCase());
    }
  });
});

// ============================================================================
// Edge Cases
// ============================================================================

describe("edge cases", () => {
  it("tokenizes empty input", () => {
    const tokens = tokenizeVhdl("");
    expect(tokens.length).toBe(1);
    expect(tokens[0].type).toBe("EOF");
  });

  it("tokenizes whitespace-only input", () => {
    const tokens = tokenizeVhdl("   \n\n   ");
    expect(tokens[tokens.length - 1].type).toBe("EOF");
  });

  it("handles multiple semicolons", () => {
    const types = tokenTypes(";;;");
    expect(types).toEqual(["SEMICOLON", "SEMICOLON", "SEMICOLON", "EOF"]);
  });

  it("handles less-than and greater-than as standalone operators", () => {
    const types = tokenTypes("a < b");
    expect(types).toContain("LESS_THAN");

    const types2 = tokenTypes("a > b");
    expect(types2).toContain("GREATER_THAN");
  });

  it("handles equals sign", () => {
    const types = tokenTypes("a = b");
    expect(types).toContain("EQUALS");
  });

  it("handles pipe operator", () => {
    /**
     * The | operator in VHDL is used in choice lists within case statements:
     *   when "00" | "01" => ...
     */
    const types = tokenTypes("a | b");
    expect(types).toContain("PIPE");
  });
});
