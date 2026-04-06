/**
 * Tests for the ALGOL 60 Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes ALGOL 60
 * source text when loaded with the `algol.tokens` grammar file.
 *
 * ALGOL 60 Background
 * -------------------
 *
 * ALGOL 60 (ALGOrithmic Language 1960) is the earliest ancestor of most modern
 * programming languages. It introduced:
 *   - Block structure (begin/end)
 *   - Lexical scoping
 *   - Recursion
 *   - The call stack
 *   - BNF grammar specification
 *
 * Key lexical differences from C-family languages:
 *   - Assignment is :=  (not =).  Equality test is = (not ==).
 *   - Boolean operators are words: and, or, not, impl, eqv.
 *   - Integer division is div, modulo is mod (not / or %).
 *   - Keywords are case-insensitive: BEGIN = Begin = begin.
 *   - Strings use single quotes, no escape sequences.
 *   - Comments use "comment <text>;" — a keyword plus any text up to semicolon.
 *   - Two exponentiation notations: ** and ^ both work.
 *
 * Test Categories
 * ---------------
 *
 *   1. **Keywords** -- all ALGOL 60 reserved words produce the right token kind
 *   2. **Case insensitivity** -- BEGIN, Begin, begin all treated equally
 *   3. **Keyword boundary** -- "beginning" is IDENT, not BEGIN + "ning"
 *   4. **Identifiers** -- simple names, names with digits
 *   5. **Integer literals** -- plain decimal integers
 *   6. **Real literals** -- decimal point, exponent, or both
 *   7. **String literals** -- single-quoted strings, empty string
 *   8. **Operators** -- all arithmetic, relational, and boolean operators
 *   9. **Disambiguation** -- := vs :, ** vs *, <= vs <, >= vs >, != bare
 *  10. **Comments** -- comment...;  consumed silently
 *  11. **Multi-token expressions** -- realistic ALGOL code fragments
 *  12. **Position tracking** -- line and column numbers
 */

import { describe, it, expect } from "vitest";
import { tokenizeAlgol } from "../src/tokenizer.js";

/**
 * Helper: extract just the token types from an ALGOL source string.
 * Makes assertions concise — we compare arrays of type strings rather than
 * inspecting full Token objects.
 */
function tokenTypes(source: string): string[] {
  return tokenizeAlgol(source).map((t) => t.type);
}

/**
 * Helper: extract just the token values from an ALGOL source string.
 * Useful for verifying that the lexer captures the correct text.
 */
function tokenValues(source: string): string[] {
  return tokenizeAlgol(source).map((t) => t.value);
}

describe("keywords — block structure", () => {
  it("tokenizes begin", () => {
    /**
     * `begin` opens a block (like '{' in C). It introduces a new scope
     * and may be followed by declarations and statements.
     */
    const tokens = tokenizeAlgol("begin");
    expect(tokens[0].type).toBe("begin");
    expect(tokens[0].value).toBe("begin");
  });

  it("tokenizes end", () => {
    /**
     * `end` closes a block (like '}' in C).
     */
    const tokens = tokenizeAlgol("end");
    expect(tokens[0].type).toBe("end");
    expect(tokens[0].value).toBe("end");
  });
});

describe("keywords — control flow", () => {
  it("tokenizes if", () => {
    const tokens = tokenizeAlgol("if");
    expect(tokens[0].type).toBe("if");
  });

  it("tokenizes then", () => {
    const tokens = tokenizeAlgol("then");
    expect(tokens[0].type).toBe("then");
  });

  it("tokenizes else", () => {
    const tokens = tokenizeAlgol("else");
    expect(tokens[0].type).toBe("else");
  });

  it("tokenizes for", () => {
    const tokens = tokenizeAlgol("for");
    expect(tokens[0].type).toBe("for");
  });

  it("tokenizes do", () => {
    const tokens = tokenizeAlgol("do");
    expect(tokens[0].type).toBe("do");
  });

  it("tokenizes step", () => {
    /**
     * `step` appears in for-loop range specifiers:
     *   for i := 1 step 1 until 10 do ...
     * This is the ALGOL equivalent of a C-style for loop's increment.
     */
    const tokens = tokenizeAlgol("step");
    expect(tokens[0].type).toBe("step");
  });

  it("tokenizes until", () => {
    const tokens = tokenizeAlgol("until");
    expect(tokens[0].type).toBe("until");
  });

  it("tokenizes while", () => {
    const tokens = tokenizeAlgol("while");
    expect(tokens[0].type).toBe("while");
  });

  it("tokenizes goto", () => {
    /**
     * `goto` transfers control unconditionally to a label.
     * ALGOL 60 supports goto; Dijkstra's famous "Go To Statement Considered
     * Harmful" letter (1968) was largely a response to its overuse in ALGOL programs.
     */
    const tokens = tokenizeAlgol("goto");
    expect(tokens[0].type).toBe("goto");
  });
});

describe("keywords — declarations", () => {
  it("tokenizes switch", () => {
    const tokens = tokenizeAlgol("switch");
    expect(tokens[0].type).toBe("switch");
  });

  it("tokenizes procedure", () => {
    /**
     * `procedure` introduces a subroutine definition. ALGOL procedures
     * can be called by value or by name (the default), and they invented
     * the concept of formal parameters.
     */
    const tokens = tokenizeAlgol("procedure");
    expect(tokens[0].type).toBe("procedure");
  });

  it("tokenizes array", () => {
    const tokens = tokenizeAlgol("array");
    expect(tokens[0].type).toBe("array");
  });

  it("tokenizes value", () => {
    /**
     * `value` specifies that a procedure parameter should be passed by value
     * (evaluated once before the call). Without `value`, ALGOL uses
     * call-by-name (the argument expression is re-evaluated on every use).
     */
    const tokens = tokenizeAlgol("value");
    expect(tokens[0].type).toBe("value");
  });
});

describe("keywords — types", () => {
  it("tokenizes integer", () => {
    const tokens = tokenizeAlgol("integer");
    expect(tokens[0].type).toBe("integer");
  });

  it("tokenizes real", () => {
    const tokens = tokenizeAlgol("real");
    expect(tokens[0].type).toBe("real");
  });

  it("tokenizes boolean", () => {
    const tokens = tokenizeAlgol("boolean");
    expect(tokens[0].type).toBe("boolean");
  });

  it("tokenizes string", () => {
    const tokens = tokenizeAlgol("string");
    expect(tokens[0].type).toBe("string");
  });
});

describe("keywords — boolean literals", () => {
  it("tokenizes true", () => {
    /**
     * In ALGOL, `true` and `false` are keywords, not reclassified
     * identifiers. They are the sole boolean literal values.
     */
    const tokens = tokenizeAlgol("true");
    expect(tokens[0].type).toBe("true");
    expect(tokens[0].value).toBe("true");
  });

  it("tokenizes false", () => {
    const tokens = tokenizeAlgol("false");
    expect(tokens[0].type).toBe("false");
    expect(tokens[0].value).toBe("false");
  });
});

describe("keywords — boolean operators", () => {
  it("tokenizes not", () => {
    /**
     * Boolean negation. ALGOL uses words, not symbols, for boolean operators.
     * This is more readable than C's ! operator.
     */
    const tokens = tokenizeAlgol("not");
    expect(tokens[0].type).toBe("not");
  });

  it("tokenizes and", () => {
    /**
     * Logical conjunction (like && in C). Written as a word in ALGOL.
     */
    const tokens = tokenizeAlgol("and");
    expect(tokens[0].type).toBe("and");
  });

  it("tokenizes or", () => {
    /**
     * Logical disjunction (like || in C).
     */
    const tokens = tokenizeAlgol("or");
    expect(tokens[0].type).toBe("or");
  });

  it("tokenizes impl", () => {
    /**
     * Logical implication: `a impl b` is equivalent to `not a or b`.
     * This operator has no direct equivalent in C-family languages.
     * Truth table: F impl F = T, F impl T = T, T impl F = F, T impl T = T.
     */
    const tokens = tokenizeAlgol("impl");
    expect(tokens[0].type).toBe("impl");
  });

  it("tokenizes eqv", () => {
    /**
     * Logical equivalence: `a eqv b` is true when a and b have the same truth value.
     * Equivalent to XNOR. Also absent from C-family languages.
     * Truth table: F eqv F = T, F eqv T = F, T eqv F = F, T eqv T = T.
     */
    const tokens = tokenizeAlgol("eqv");
    expect(tokens[0].type).toBe("eqv");
  });
});

describe("keywords — arithmetic operators", () => {
  it("tokenizes div", () => {
    /**
     * Integer division (truncates toward zero).
     * ALGOL distinguishes / (real division) from div (integer division) explicitly.
     * Example: 7 div 2 = 3, not 3.5.
     */
    const tokens = tokenizeAlgol("div");
    expect(tokens[0].type).toBe("div");
  });

  it("tokenizes mod", () => {
    /**
     * Modulo (remainder after integer division).
     * Example: 7 mod 2 = 1.
     */
    const tokens = tokenizeAlgol("mod");
    expect(tokens[0].type).toBe("mod");
  });
});

describe("case insensitivity of keywords", () => {
  it("recognizes BEGIN (uppercase)", () => {
    /**
     * ALGOL 60 keywords are case-insensitive. BEGIN, Begin, begin,
     * and bEgIn all produce the same token type.
     */
    const tokens = tokenizeAlgol("BEGIN");
    expect(tokens[0].type).toBe("begin");
  });

  it("recognizes Begin (mixed case)", () => {
    const tokens = tokenizeAlgol("Begin");
    expect(tokens[0].type).toBe("begin");
  });

  it("recognizes END in uppercase", () => {
    const tokens = tokenizeAlgol("END");
    expect(tokens[0].type).toBe("end");
  });

  it("recognizes INTEGER in uppercase", () => {
    const tokens = tokenizeAlgol("INTEGER");
    expect(tokens[0].type).toBe("integer");
  });
});

describe("keyword boundary — prefix must not match", () => {
  it("lexes 'beginning' as IDENT, not begin + ning", () => {
    /**
     * Keywords must match the full identifier. "beginning" starts with "begin"
     * but is not the keyword — it is an identifier. The lexer must match the
     * longest possible token: the full word "beginning" becomes IDENT.
     *
     * This is the "maximum munch" rule. Without it, `beginning` would lex as
     * BEGIN + IDENT("ning"), which would break any ALGOL program that used
     * variable names starting with keywords.
     */
    const tokens = tokenizeAlgol("beginning");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("beginning");
  });

  it("lexes 'endgame' as IDENT, not end + game", () => {
    const tokens = tokenizeAlgol("endgame");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("endgame");
  });

  it("lexes 'integer1' as IDENT", () => {
    const tokens = tokenizeAlgol("integer1");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("integer1");
  });

  it("lexes 'truthy' as IDENT, not true + thy", () => {
    const tokens = tokenizeAlgol("truthy");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("truthy");
  });
});

describe("identifiers", () => {
  it("tokenizes a single-letter identifier", () => {
    /**
     * The simplest identifier: a single letter. ALGOL identifiers start
     * with a letter (a-z, A-Z) followed by zero or more letters or digits.
     * Underscores are NOT allowed — original ALGOL 60 did not include them.
     */
    const tokens = tokenizeAlgol("x");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("x");
  });

  it("tokenizes a multi-letter identifier", () => {
    const tokens = tokenizeAlgol("sum");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("sum");
  });

  it("tokenizes an identifier with digits", () => {
    /**
     * Identifiers can contain digits after the first letter.
     * "A1" is a valid ALGOL identifier (letter followed by digit).
     */
    const tokens = tokenizeAlgol("A1");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("A1");
  });

  it("tokenizes a longer identifier with digits", () => {
    const tokens = tokenizeAlgol("count2");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("count2");
  });
});

describe("integer literals", () => {
  it("tokenizes zero", () => {
    /**
     * The integer literal 0. The INTEGER_LIT regex is /[0-9]+/ so it
     * matches one or more digits without a decimal point.
     */
    const tokens = tokenizeAlgol("0");
    expect(tokens[0].type).toBe("INTEGER_LIT");
    expect(tokens[0].value).toBe("0");
  });

  it("tokenizes a positive integer", () => {
    const tokens = tokenizeAlgol("42");
    expect(tokens[0].type).toBe("INTEGER_LIT");
    expect(tokens[0].value).toBe("42");
  });

  it("tokenizes a large integer", () => {
    const tokens = tokenizeAlgol("1000");
    expect(tokens[0].type).toBe("INTEGER_LIT");
    expect(tokens[0].value).toBe("1000");
  });
});

describe("real literals", () => {
  it("tokenizes a decimal real", () => {
    /**
     * Real literals have a decimal point: 3.14
     * Pattern: [0-9]+\.[0-9]*([eE][+-]?[0-9]+)?
     * REAL_LIT must come before INTEGER_LIT in the grammar file so that "3.14"
     * is not matched as INTEGER_LIT("3") followed by "." followed by INTEGER_LIT("14").
     */
    const tokens = tokenizeAlgol("3.14");
    expect(tokens[0].type).toBe("REAL_LIT");
    expect(tokens[0].value).toBe("3.14");
  });

  it("tokenizes a real with exponent (no decimal point)", () => {
    /**
     * Real with exponent but no decimal point: 1.5E3 = 1500.0
     * The exponent form [0-9]+[eE][+-]?[0-9]+ also matches as REAL_LIT,
     * distinguishing it from a plain integer.
     */
    const tokens = tokenizeAlgol("1.5E3");
    expect(tokens[0].type).toBe("REAL_LIT");
    expect(tokens[0].value).toBe("1.5E3");
  });

  it("tokenizes a real with negative exponent", () => {
    /**
     * Negative exponent: 1.5E-3 = 0.0015
     */
    const tokens = tokenizeAlgol("1.5E-3");
    expect(tokens[0].type).toBe("REAL_LIT");
    expect(tokens[0].value).toBe("1.5E-3");
  });

  it("tokenizes an integer with exponent (no decimal point)", () => {
    /**
     * 100E2 = 10000.0. Even though there is no decimal point, the exponent
     * marker makes this a REAL_LIT, not an INTEGER_LIT.
     */
    const tokens = tokenizeAlgol("100E2");
    expect(tokens[0].type).toBe("REAL_LIT");
    expect(tokens[0].value).toBe("100E2");
  });

  it("tokenizes a real with lowercase e", () => {
    const tokens = tokenizeAlgol("2.5e6");
    expect(tokens[0].type).toBe("REAL_LIT");
    expect(tokens[0].value).toBe("2.5e6");
  });
});

describe("string literals", () => {
  it("tokenizes a single-quoted string", () => {
    /**
     * ALGOL 60 strings use single quotes. Unlike C/Java, there are no escape
     * sequences — a single quote cannot appear inside a string.
     * The STRING_LIT regex: /'[^']*'/
     */
    const tokens = tokenizeAlgol("'hello'");
    expect(tokens[0].type).toBe("STRING_LIT");
    // The generic grammar-lexer strips the surrounding single-quote delimiters
    // from STRING tokens (any token whose name contains "STRING"). The value
    // is the bare string content, not the quoted form.
    expect(tokens[0].value).toBe("hello");
  });

  it("tokenizes an empty string", () => {
    /**
     * Two adjacent single quotes produce an empty string.
     * The regex [^']* matches zero characters.
     */
    const tokens = tokenizeAlgol("''");
    expect(tokens[0].type).toBe("STRING_LIT");
    expect(tokens[0].value).toBe("");
  });

  it("tokenizes a string with spaces", () => {
    const tokens = tokenizeAlgol("'hello world'");
    expect(tokens[0].type).toBe("STRING_LIT");
    expect(tokens[0].value).toBe("hello world");
  });
});

describe("operators — disambiguation", () => {
  it("tokenizes := as ASSIGN (not COLON + EQ)", () => {
    /**
     * The := operator must be recognized as a single token.
     * If : were matched first, x := 5 would become IDENT COLON EQ INTEGER_LIT,
     * which is wrong. The grammar file lists := (ASSIGN) before : (COLON).
     */
    const types = tokenTypes("x := 5");
    expect(types).toContain("ASSIGN");
    expect(types).not.toContain("COLON");
    expect(types).not.toContain("EQ");
  });

  it("tokenizes ** as POWER (not STAR + STAR)", () => {
    /**
     * Double asterisk is exponentiation in ALGOL. Without ordering, 2**3
     * would lex as INTEGER_LIT STAR STAR INTEGER_LIT, which the parser
     * could not distinguish from multiplication by a dereferenced pointer (C pun).
     * The grammar lists ** (POWER) before * (STAR).
     */
    const types = tokenTypes("2 ** 3");
    expect(types).toContain("POWER");
    expect(types).not.toContain("STAR");
  });

  it("tokenizes <= as LEQ (not LT + EQ)", () => {
    /**
     * Less-than-or-equal. The grammar lists <= before < so that the
     * two-character sequence is consumed as one token.
     */
    const types = tokenTypes("x <= 5");
    expect(types).toContain("LEQ");
    expect(types).not.toContain("LT");
  });

  it("tokenizes >= as GEQ (not GT + EQ)", () => {
    const types = tokenTypes("x >= 5");
    expect(types).toContain("GEQ");
    expect(types).not.toContain("GT");
  });

  it("tokenizes != as NEQ (not two separate tokens)", () => {
    const types = tokenTypes("x != 5");
    expect(types).toContain("NEQ");
  });

  it("tokenizes : as COLON when not preceded by =", () => {
    /**
     * A bare colon appears in array bound pairs: A[1:10]
     * It must not be confused with the first character of :=.
     */
    const types = tokenTypes("1:10");
    expect(types).toContain("COLON");
    expect(types).not.toContain("ASSIGN");
  });

  it("tokenizes * as STAR when not followed by *", () => {
    /**
     * Single asterisk is plain multiplication.
     */
    const types = tokenTypes("x * y");
    expect(types).toContain("STAR");
    expect(types).not.toContain("POWER");
  });

  it("tokenizes ^ as CARET (alternate exponentiation)", () => {
    /**
     * Caret ^ is the alternate exponentiation operator.
     * Some ALGOL 60 implementations used ^ instead of ** for exponentiation.
     * Both are valid; the parser treats them identically.
     */
    const types = tokenTypes("2 ^ 3");
    expect(types).toContain("CARET");
  });
});

describe("all single-character operators", () => {
  it("tokenizes + as PLUS", () => {
    expect(tokenTypes("x + y")).toContain("PLUS");
  });

  it("tokenizes - as MINUS", () => {
    expect(tokenTypes("x - y")).toContain("MINUS");
  });

  it("tokenizes / as SLASH", () => {
    expect(tokenTypes("x / y")).toContain("SLASH");
  });

  it("tokenizes = as EQ", () => {
    /**
     * In ALGOL, = is equality comparison, NOT assignment.
     * Assignment is :=. This is one of ALGOL's most praised design choices.
     */
    expect(tokenTypes("x = y")).toContain("EQ");
  });

  it("tokenizes < as LT", () => {
    expect(tokenTypes("x < y")).toContain("LT");
  });

  it("tokenizes > as GT", () => {
    expect(tokenTypes("x > y")).toContain("GT");
  });
});

describe("delimiters", () => {
  it("tokenizes ( and ) as LPAREN and RPAREN", () => {
    const types = tokenTypes("(x + y)");
    expect(types).toContain("LPAREN");
    expect(types).toContain("RPAREN");
  });

  it("tokenizes [ and ] as LBRACKET and RBRACKET", () => {
    /**
     * Square brackets are used for array subscripts:
     *   A[i, j]  -- access element at row i, column j
     */
    const types = tokenTypes("A[1]");
    expect(types).toContain("LBRACKET");
    expect(types).toContain("RBRACKET");
  });

  it("tokenizes ; as SEMICOLON", () => {
    /**
     * Semicolons separate statements and terminate comments.
     */
    const types = tokenTypes("x := 1; y := 2");
    expect(types).toContain("SEMICOLON");
  });

  it("tokenizes , as COMMA", () => {
    /**
     * Commas separate list elements: identifiers, array subscripts,
     * procedure arguments, and for-list elements.
     */
    const types = tokenTypes("x, y, z");
    expect(types).toContain("COMMA");
  });
});

describe("comment skipping", () => {
  it("silently consumes a comment", () => {
    /**
     * An ALGOL comment begins with the keyword `comment` and ends at the
     * next semicolon. The comment text plus the terminating semicolon are
     * consumed silently — no COMMENT token is emitted.
     *
     *   comment this is ignored;  x := 1
     *
     * After the comment is consumed, the next real token is IDENT("x").
     */
    const tokens = tokenizeAlgol("comment this is ignored; x := 1");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("x");
  });

  it("consumes a comment and continues tokenizing", () => {
    /**
     * Everything between `comment` and the semicolon is gone.
     * Only the tokens after the semicolon are emitted.
     */
    const types = tokenTypes("comment skip this whole phrase; x := 1");
    expect(types).not.toContain("comment");
    // x := 1 should still be there
    expect(types).toContain("NAME");
    expect(types).toContain("ASSIGN");
    expect(types).toContain("INTEGER_LIT");
  });

  it("handles a comment at the start of a program", () => {
    const types = tokenTypes("comment compute sum; begin end");
    expect(types).not.toContain("comment");
    expect(types).toContain("begin");
    expect(types).toContain("end");
  });
});

describe("multi-token expression", () => {
  it("tokenizes x := 1 + 2 * 3", () => {
    /**
     * A compound arithmetic expression.
     * Token sequence: IDENT ASSIGN INTEGER_LIT PLUS INTEGER_LIT STAR INTEGER_LIT EOF
     */
    const types = tokenTypes("x := 1 + 2 * 3");
    expect(types).toEqual([
      "NAME", "ASSIGN", "INTEGER_LIT", "PLUS", "INTEGER_LIT", "STAR", "INTEGER_LIT", "EOF",
    ]);
  });

  it("tokenizes a minimal ALGOL block", () => {
    /**
     * The smallest complete ALGOL program:
     *   begin integer x; x := 42 end
     *
     * Token sequence:
     *   begin  integer  IDENT(x)  ;  IDENT(x)  :=  42  end  EOF
     */
    const types = tokenTypes("begin integer x; x := 42 end");
    expect(types).toEqual([
      "begin", "integer", "NAME", "SEMICOLON",
      "NAME", "ASSIGN", "INTEGER_LIT",
      "end", "EOF",
    ]);
  });

  it("tokenizes a boolean expression with word operators", () => {
    /**
     * ALGOL boolean operators are words, not symbols.
     * `x > 0 and y < 10` demonstrates two relational tests combined with `and`.
     */
    const types = tokenTypes("x > 0 and y < 10");
    expect(types).toContain("GT");
    expect(types).toContain("and");
    expect(types).toContain("LT");
  });

  it("tokenizes a for-loop header", () => {
    /**
     * ALGOL for-loop: for i := 1 step 1 until n do
     * Demonstrates: for, IDENT, :=, INTEGER_LIT, step, INTEGER_LIT, until, IDENT, do
     */
    const types = tokenTypes("for i := 1 step 1 until n do");
    expect(types).toContain("for");
    expect(types).toContain("ASSIGN");
    expect(types).toContain("step");
    expect(types).toContain("until");
    expect(types).toContain("do");
  });
});

describe("position tracking", () => {
  it("tracks line 1 column 1 for the first token", () => {
    /**
     * Token positions are 1-indexed. The first token in the source is
     * always at line 1, column 1.
     */
    const tokens = tokenizeAlgol("begin");
    expect(tokens[0].line).toBe(1);
    expect(tokens[0].column).toBe(1);
  });

  it("tracks column positions within a line", () => {
    /**
     * Tokens on the same line have increasing column numbers.
     * `x := 1` -- x is at column 1, := is at column 3.
     */
    const tokens = tokenizeAlgol("x := 1");
    expect(tokens[0].column).toBe(1); // x
    expect(tokens[1].column).toBe(3); // :=
    expect(tokens[2].column).toBe(6); // 1
  });

  it("tracks line numbers across newlines", () => {
    /**
     * Tokens on later lines have larger line numbers.
     * Whitespace (including newlines) is skipped between tokens, but
     * line tracking continues.
     */
    const tokens = tokenizeAlgol("begin\ninteger x");
    expect(tokens[0].line).toBe(1); // begin
    expect(tokens[1].line).toBe(2); // integer
  });
});
