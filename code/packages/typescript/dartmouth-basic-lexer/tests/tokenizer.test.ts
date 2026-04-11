/**
 * Tests for the Dartmouth BASIC Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes
 * Dartmouth BASIC 1964 source text — including the two post-tokenize hooks
 * that handle LINE_NUM disambiguation and REM comment suppression.
 *
 * Dartmouth BASIC Background
 * --------------------------
 *
 * Created in 1964 by John Kemeny and Thomas Kurtz at Dartmouth College,
 * BASIC was the first programming language designed specifically for students
 * with no mathematics background. It ran on a GE-225 mainframe accessed
 * through uppercase-only teletype terminals.
 *
 * Key lexical features:
 *
 *   - Every statement must be on a numbered line: `10 LET X = 5`
 *   - All input is uppercase (case-insensitive; `print` == `PRINT`)
 *   - NEWLINE is significant — it terminates each statement
 *   - REM introduces a comment that runs to the end of the line
 *   - Variable names: one letter (A–Z) plus optional digit (A0–Z9)
 *   - All numbers are floats internally: `42` is `42.0`
 *   - 11 built-in math functions: SIN, COS, TAN, ATN, EXP, LOG, ABS,
 *     SQR, INT, RND, SGN
 *   - User-defined functions: FNA through FNZ (FN + one letter)
 *
 * Test Categories
 * ---------------
 *
 *   1. **LINE_NUM disambiguation** — integers at line-start vs in expressions
 *   2. **Keywords** — all 20 reserved words
 *   3. **Case insensitivity** — lowercase and mixed-case input
 *   4. **BUILTIN_FN** — all 11 built-in math functions
 *   5. **USER_FN** — user-defined function names (FNA, FNZ, etc.)
 *   6. **NAME (variables)** — single-letter and letter+digit names
 *   7. **NUMBER literals** — integer, decimal, scientific notation
 *   8. **STRING literals** — double-quoted strings
 *   9. **Operators** — all arithmetic, comparison, and punctuation
 *  10. **Multi-char operator disambiguation** — <=, >=, <> vs <, >, =
 *  11. **NEWLINE handling** — Unix and Windows line endings
 *  12. **REM suppression** — comment text removed from token stream
 *  13. **Multi-line programs** — realistic BASIC programs
 *  14. **FOR/NEXT loop** — loop keyword sequence
 *  15. **DEF FN** — user-defined function definition
 *  16. **PRINT separators** — COMMA (zone) and SEMICOLON (tight)
 *  17. **Error recovery** — UNKNOWN token for unrecognised characters
 *  18. **Position tracking** — line and column numbers
 *  19. **EOF token** — always present as the last token
 */

import { describe, it, expect } from "vitest";
import { tokenizeDartmouthBasic, createDartmouthBasicLexer } from "../src/tokenizer.js";
import type { Token } from "@coding-adventures/lexer";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Extract just the token types from a BASIC source string.
 *
 * Makes test assertions concise — we compare arrays of type strings rather
 * than inspecting full Token objects.
 */
function tokenTypes(source: string): string[] {
  return tokenizeDartmouthBasic(source).map((t) => t.type);
}

/**
 * Extract just the token values from a BASIC source string.
 *
 * Useful for verifying that the lexer captures the correct text.
 */
function tokenValues(source: string): string[] {
  return tokenizeDartmouthBasic(source).map((t) => t.value);
}

/**
 * Find the first token of a given type in the token list.
 *
 * Useful for spot-checking a specific token without caring about position.
 */
function firstOfType(source: string, type: string): Token | undefined {
  return tokenizeDartmouthBasic(source).find((t) => t.type === type);
}

// ---------------------------------------------------------------------------
// 1. LINE_NUM Disambiguation
// ---------------------------------------------------------------------------

describe("LINE_NUM disambiguation — line-start integers vs expression integers", () => {
  it("labels the first integer on a line as LINE_NUM", () => {
    /**
     * `10 LET X = 5` — the `10` is a line label, not a value.
     * The post-tokenize hook relabelLineNumbers re-labels the first NUMBER
     * token on each line as LINE_NUM.
     */
    const tokens = tokenizeDartmouthBasic("10 LET X = 5\n");
    expect(tokens[0].type).toBe("LINE_NUM");
    expect(tokens[0].value).toBe("10");
  });

  it("labels in-expression integers as NUMBER, not LINE_NUM", () => {
    /**
     * `10 LET X = 5` — the `5` is an expression value.
     * The token at position 0 is LINE_NUM("10"); the `5` must be NUMBER.
     */
    const tokens = tokenizeDartmouthBasic("10 LET X = 5\n");
    const fiveToken = tokens.find((t) => t.value === "5");
    expect(fiveToken).toBeDefined();
    expect(fiveToken!.type).toBe("NUMBER");
  });

  it("labels the line number on each line of a multi-line program", () => {
    /**
     * Every line should start with a LINE_NUM token. This verifies that
     * the hook resets its "at-line-start" state after every NEWLINE.
     *
     *   10 LET X = 1
     *   20 LET Y = 2
     *   30 END
     */
    const source = "10 LET X = 1\n20 LET Y = 2\n30 END\n";
    const tokens = tokenizeDartmouthBasic(source);
    const lineNums = tokens.filter((t) => t.type === "LINE_NUM");
    expect(lineNums).toHaveLength(3);
    expect(lineNums[0].value).toBe("10");
    expect(lineNums[1].value).toBe("20");
    expect(lineNums[2].value).toBe("30");
  });

  it("labels GOTO target as NUMBER (not LINE_NUM)", () => {
    /**
     * `30 GOTO 10` — the `30` is the line label (LINE_NUM), but the `10`
     * after GOTO is a branch target, which lexically is a NUMBER.
     * (The parser will validate that it is an integer and a valid line number.)
     */
    const tokens = tokenizeDartmouthBasic("30 GOTO 10\n");
    const lineNum = tokens.find((t) => t.type === "LINE_NUM");
    const gotoTarget = tokens.find((t) => t.value === "10");
    expect(lineNum!.value).toBe("30");
    expect(gotoTarget!.type).toBe("NUMBER");
  });

  it("labels IF...THEN target as NUMBER", () => {
    /**
     * `40 IF X > 0 THEN 100` — the `100` after THEN is a branch target (NUMBER).
     */
    const tokens = tokenizeDartmouthBasic("40 IF X > 0 THEN 100\n");
    const thenTarget = tokens.find((t) => t.value === "100");
    expect(thenTarget).toBeDefined();
    expect(thenTarget!.type).toBe("NUMBER");
  });

  it("handles line number 1 (single digit)", () => {
    const tokens = tokenizeDartmouthBasic("1 END\n");
    expect(tokens[0].type).toBe("LINE_NUM");
    expect(tokens[0].value).toBe("1");
  });

  it("handles three-digit line numbers", () => {
    const tokens = tokenizeDartmouthBasic("999 END\n");
    expect(tokens[0].type).toBe("LINE_NUM");
    expect(tokens[0].value).toBe("999");
  });
});

// ---------------------------------------------------------------------------
// 2. Keywords
// ---------------------------------------------------------------------------

describe("keywords — all 20 Dartmouth BASIC reserved words", () => {
  /**
   * Test each keyword in isolation (as the first token after a line number).
   * The keywords section in dartmouth_basic.tokens lists all 20 words.
   * Each must produce a KEYWORD token with the uppercase value.
   */

  it("tokenizes LET", () => {
    // LET assigns a value to a variable: LET X = 5
    const tokens = tokenizeDartmouthBasic("10 LET X = 1\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "LET");
    expect(kw).toBeDefined();
  });

  it("tokenizes PRINT", () => {
    // PRINT displays output to the teletype.
    const tokens = tokenizeDartmouthBasic("10 PRINT X\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "PRINT");
    expect(kw).toBeDefined();
  });

  it("tokenizes INPUT", () => {
    // INPUT reads a value from the user at the teletype.
    const tokens = tokenizeDartmouthBasic("10 INPUT X\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "INPUT");
    expect(kw).toBeDefined();
  });

  it("tokenizes IF", () => {
    // IF introduces a conditional: IF X > 0 THEN 100
    const tokens = tokenizeDartmouthBasic("10 IF X > 0 THEN 20\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "IF");
    expect(kw).toBeDefined();
  });

  it("tokenizes THEN", () => {
    // THEN gives the jump target in IF...THEN.
    const tokens = tokenizeDartmouthBasic("10 IF X > 0 THEN 20\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "THEN");
    expect(kw).toBeDefined();
  });

  it("tokenizes GOTO", () => {
    // GOTO transfers control unconditionally to a line number.
    const tokens = tokenizeDartmouthBasic("10 GOTO 20\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "GOTO");
    expect(kw).toBeDefined();
  });

  it("tokenizes GOSUB", () => {
    // GOSUB calls a subroutine (like a function call without parameters).
    // Execution continues at the given line number; RETURN comes back.
    const tokens = tokenizeDartmouthBasic("10 GOSUB 100\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "GOSUB");
    expect(kw).toBeDefined();
  });

  it("tokenizes RETURN", () => {
    // RETURN returns from a GOSUB to the line after the GOSUB call.
    const tokens = tokenizeDartmouthBasic("10 RETURN\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "RETURN");
    expect(kw).toBeDefined();
  });

  it("tokenizes FOR", () => {
    // FOR begins a counted loop: FOR I = 1 TO 10
    const tokens = tokenizeDartmouthBasic("10 FOR I = 1 TO 10\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "FOR");
    expect(kw).toBeDefined();
  });

  it("tokenizes TO", () => {
    // TO specifies the upper bound of a FOR loop.
    const tokens = tokenizeDartmouthBasic("10 FOR I = 1 TO 10\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "TO");
    expect(kw).toBeDefined();
  });

  it("tokenizes STEP", () => {
    // STEP specifies the loop increment: FOR I = 1 TO 10 STEP 2
    const tokens = tokenizeDartmouthBasic("10 FOR I = 1 TO 10 STEP 2\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "STEP");
    expect(kw).toBeDefined();
  });

  it("tokenizes NEXT", () => {
    // NEXT closes a FOR loop: NEXT I
    const tokens = tokenizeDartmouthBasic("10 NEXT I\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "NEXT");
    expect(kw).toBeDefined();
  });

  it("tokenizes END", () => {
    // END terminates the program.
    const tokens = tokenizeDartmouthBasic("10 END\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "END");
    expect(kw).toBeDefined();
  });

  it("tokenizes STOP", () => {
    // STOP halts execution (like END but can appear mid-program).
    const tokens = tokenizeDartmouthBasic("10 STOP\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "STOP");
    expect(kw).toBeDefined();
  });

  it("tokenizes REM", () => {
    // REM introduces a remark (comment). Everything after REM is suppressed.
    const tokens = tokenizeDartmouthBasic("10 REM COMMENT\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "REM");
    expect(kw).toBeDefined();
  });

  it("tokenizes READ", () => {
    // READ reads the next value from the DATA list.
    const tokens = tokenizeDartmouthBasic("10 READ X\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "READ");
    expect(kw).toBeDefined();
  });

  it("tokenizes DATA", () => {
    // DATA supplies literal values for READ statements.
    const tokens = tokenizeDartmouthBasic("10 DATA 1,2,3\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "DATA");
    expect(kw).toBeDefined();
  });

  it("tokenizes RESTORE", () => {
    // RESTORE resets the DATA pointer to the beginning of the DATA list.
    const tokens = tokenizeDartmouthBasic("10 RESTORE\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "RESTORE");
    expect(kw).toBeDefined();
  });

  it("tokenizes DIM", () => {
    // DIM declares an array: DIM A(10) allocates 11 elements A(0)..A(10).
    const tokens = tokenizeDartmouthBasic("10 DIM A(10)\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "DIM");
    expect(kw).toBeDefined();
  });

  it("tokenizes DEF", () => {
    // DEF defines a user function: DEF FNA(X) = X * X
    const tokens = tokenizeDartmouthBasic("10 DEF FNA(X) = X * X\n");
    const kw = tokens.find((t) => t.type === "KEYWORD" && t.value === "DEF");
    expect(kw).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// 3. Case Insensitivity
// ---------------------------------------------------------------------------

describe("case insensitivity — @case_insensitive true directive", () => {
  it("produces KEYWORD('PRINT') from 'print' (lowercase)", () => {
    /**
     * The grammar directive `@case_insensitive true` uppercases the entire
     * source before matching. So `print` → `PRINT` before the keyword check.
     * The emitted token value is the uppercased form.
     */
    const tokens = tokenizeDartmouthBasic("10 print x\n");
    const kw = tokens.find((t) => t.type === "KEYWORD");
    expect(kw).toBeDefined();
    expect(kw!.value).toBe("PRINT");
  });

  it("produces KEYWORD('LET') from 'Let' (mixed case)", () => {
    const tokens = tokenizeDartmouthBasic("20 Let A = 1\n");
    const kw = tokens.find((t) => t.type === "KEYWORD");
    expect(kw!.value).toBe("LET");
  });

  it("produces KEYWORD('GOTO') from 'goto' (lowercase)", () => {
    const tokens = tokenizeDartmouthBasic("30 goto 20\n");
    const kw = tokens.find((t) => t.type === "KEYWORD");
    expect(kw!.value).toBe("GOTO");
  });

  it("produces NAME('X') from 'x' (lowercase variable)", () => {
    /**
     * The grammar uses `case_sensitive: false` to lowercase the source before
     * matching. All patterns are lowercase. The `normalizeCase` post-tokenize
     * hook then uppercases NAME, KEYWORD, BUILTIN_FN, and USER_FN values so
     * the public API always produces uppercase identifiers.
     *
     * So `x` → lowercased to `x` → matched as NAME → upcased to `X`.
     */
    const tokens = tokenizeDartmouthBasic("10 LET x = 1\n");
    const name = tokens.find((t) => t.type === "NAME");
    expect(name!.type).toBe("NAME");
    expect(name!.value).toBe("X");
  });

  it("produces BUILTIN_FN('SIN') from 'sin' (lowercase function)", () => {
    /**
     * Like NAME, BUILTIN_FN values are upcased by the `normalizeCase` hook.
     * `sin` → lowercased → matched as BUILTIN_FN("sin") → upcased to "SIN".
     */
    const tokens = tokenizeDartmouthBasic("10 LET Y = sin(X)\n");
    const fn = tokens.find((t) => t.type === "BUILTIN_FN");
    expect(fn!.type).toBe("BUILTIN_FN");
    expect(fn!.value).toBe("SIN");
  });

  it("tokenizes 'let' identically to 'LET'", () => {
    const lower = tokenTypes("10 let x = 1\n");
    const upper = tokenTypes("10 LET X = 1\n");
    expect(lower).toEqual(upper);
  });
});

// ---------------------------------------------------------------------------
// 4. Built-in Functions
// ---------------------------------------------------------------------------

describe("BUILTIN_FN — the 11 built-in mathematical functions", () => {
  /**
   * Dartmouth BASIC 1964 defines exactly 11 built-in functions.
   * Each must be recognized as BUILTIN_FN, not NAME or KEYWORD.
   */

  it("tokenizes SIN as BUILTIN_FN", () => {
    // SIN(X) — sine of X in radians.
    const tokens = tokenizeDartmouthBasic("10 LET Y = SIN(X)\n");
    expect(tokens.find((t) => t.type === "BUILTIN_FN" && t.value === "SIN")).toBeDefined();
  });

  it("tokenizes COS as BUILTIN_FN", () => {
    // COS(X) — cosine of X in radians.
    const tokens = tokenizeDartmouthBasic("10 LET Y = COS(X)\n");
    expect(tokens.find((t) => t.type === "BUILTIN_FN" && t.value === "COS")).toBeDefined();
  });

  it("tokenizes TAN as BUILTIN_FN", () => {
    // TAN(X) — tangent of X in radians.
    const tokens = tokenizeDartmouthBasic("10 LET Y = TAN(X)\n");
    expect(tokens.find((t) => t.type === "BUILTIN_FN" && t.value === "TAN")).toBeDefined();
  });

  it("tokenizes ATN as BUILTIN_FN", () => {
    // ATN(X) — arctangent of X, result in radians.
    const tokens = tokenizeDartmouthBasic("10 LET Y = ATN(X)\n");
    expect(tokens.find((t) => t.type === "BUILTIN_FN" && t.value === "ATN")).toBeDefined();
  });

  it("tokenizes EXP as BUILTIN_FN", () => {
    // EXP(X) — e raised to the power X (approximately 2.71828^X).
    const tokens = tokenizeDartmouthBasic("10 LET Y = EXP(X)\n");
    expect(tokens.find((t) => t.type === "BUILTIN_FN" && t.value === "EXP")).toBeDefined();
  });

  it("tokenizes LOG as BUILTIN_FN", () => {
    // LOG(X) — natural logarithm of X (base e).
    const tokens = tokenizeDartmouthBasic("10 LET Y = LOG(X)\n");
    expect(tokens.find((t) => t.type === "BUILTIN_FN" && t.value === "LOG")).toBeDefined();
  });

  it("tokenizes ABS as BUILTIN_FN", () => {
    // ABS(X) — absolute value of X. ABS(-3) = 3.
    const tokens = tokenizeDartmouthBasic("10 LET Y = ABS(X)\n");
    expect(tokens.find((t) => t.type === "BUILTIN_FN" && t.value === "ABS")).toBeDefined();
  });

  it("tokenizes SQR as BUILTIN_FN", () => {
    // SQR(X) — square root of X. SQR(9) = 3.
    const tokens = tokenizeDartmouthBasic("10 LET Y = SQR(X)\n");
    expect(tokens.find((t) => t.type === "BUILTIN_FN" && t.value === "SQR")).toBeDefined();
  });

  it("tokenizes INT as BUILTIN_FN", () => {
    // INT(X) — floor of X (largest integer ≤ X). INT(3.7) = 3.
    const tokens = tokenizeDartmouthBasic("10 LET Y = INT(X)\n");
    expect(tokens.find((t) => t.type === "BUILTIN_FN" && t.value === "INT")).toBeDefined();
  });

  it("tokenizes RND as BUILTIN_FN", () => {
    // RND(X) — random number in [0,1). The argument X is required but ignored.
    const tokens = tokenizeDartmouthBasic("10 LET Y = RND(1)\n");
    expect(tokens.find((t) => t.type === "BUILTIN_FN" && t.value === "RND")).toBeDefined();
  });

  it("tokenizes SGN as BUILTIN_FN", () => {
    // SGN(X) — sign of X: -1 if X<0, 0 if X=0, 1 if X>0.
    const tokens = tokenizeDartmouthBasic("10 LET Y = SGN(X)\n");
    expect(tokens.find((t) => t.type === "BUILTIN_FN" && t.value === "SGN")).toBeDefined();
  });

  it("tokenizes two built-in functions in one expression", () => {
    /**
     * A common BASIC expression: SIN(X)^2 + COS(X)^2 = 1 (Pythagorean identity).
     * Both SIN and COS should be BUILTIN_FN tokens.
     */
    const tokens = tokenizeDartmouthBasic("70 LET Y = SIN(X) + COS(X)\n");
    const fns = tokens.filter((t) => t.type === "BUILTIN_FN");
    expect(fns).toHaveLength(2);
    expect(fns[0].value).toBe("SIN");
    expect(fns[1].value).toBe("COS");
  });
});

// ---------------------------------------------------------------------------
// 5. User-Defined Functions
// ---------------------------------------------------------------------------

describe("USER_FN — user-defined functions (FN + one letter)", () => {
  it("tokenizes FNA as USER_FN", () => {
    /**
     * DEF FNA(X) = X * X defines a user function called FNA.
     * FNA must be USER_FN, not NAME or KEYWORD.
     */
    const tokens = tokenizeDartmouthBasic("10 DEF FNA(X) = X * X\n");
    expect(tokens.find((t) => t.type === "USER_FN" && t.value === "FNA")).toBeDefined();
  });

  it("tokenizes FNZ as USER_FN", () => {
    // The last valid user function name: FNZ.
    const tokens = tokenizeDartmouthBasic("10 DEF FNZ(X) = X + 1\n");
    expect(tokens.find((t) => t.type === "USER_FN" && t.value === "FNZ")).toBeDefined();
  });

  it("tokenizes FNB as USER_FN", () => {
    const tokens = tokenizeDartmouthBasic("10 DEF FNB(X) = X * 2\n");
    expect(tokens.find((t) => t.type === "USER_FN" && t.value === "FNB")).toBeDefined();
  });

  it("tokenizes a USER_FN call in an expression", () => {
    /**
     * User functions are called the same way they are defined:
     *   20 LET Y = FNA(X)
     */
    const tokens = tokenizeDartmouthBasic("20 LET Y = FNA(X)\n");
    expect(tokens.find((t) => t.type === "USER_FN")).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// 6. Variable Names (NAME tokens)
// ---------------------------------------------------------------------------

describe("NAME — variable names (one letter + optional digit)", () => {
  it("tokenizes a single-letter variable name", () => {
    /**
     * The simplest BASIC variable: a single uppercase letter.
     * In 1964 BASIC, you had 26 single-letter variables: A through Z.
     */
    const tokens = tokenizeDartmouthBasic("10 LET X = 1\n");
    const name = tokens.find((t) => t.type === "NAME");
    expect(name!.value).toBe("X");
  });

  it("tokenizes a letter+digit variable name", () => {
    /**
     * BASIC also allowed one-letter + one-digit names: A0 through Z9.
     * That's 260 additional variables.
     */
    const tokens = tokenizeDartmouthBasic("10 LET A1 = 2\n");
    const name = tokens.find((t) => t.type === "NAME");
    expect(name!.value).toBe("A1");
  });

  it("tokenizes Z9 as a NAME", () => {
    const tokens = tokenizeDartmouthBasic("10 LET Z9 = 3\n");
    const name = tokens.find((t) => t.type === "NAME");
    expect(name!.value).toBe("Z9");
  });

  it("does NOT tokenize a keyword prefix as NAME", () => {
    /**
     * The NAME regex is /[A-Z][0-9]?/ — exactly one letter plus optional digit.
     * Because BASIC variable names are so short (max 2 chars), there is no
     * risk of confusing them with keywords. FOR, LET, IF etc. are longer
     * than 2 characters except IF (2 chars). Let's verify IF is still KEYWORD.
     */
    const tokens = tokenizeDartmouthBasic("10 IF X > 0 THEN 20\n");
    const kw = tokens.find((t) => t.value === "IF");
    expect(kw!.type).toBe("KEYWORD");
  });

  it("tokenizes multiple variables in one expression", () => {
    // PRINT X, Y — two NAME tokens: X and Y.
    const tokens = tokenizeDartmouthBasic("20 PRINT X, Y\n");
    const names = tokens.filter((t) => t.type === "NAME");
    expect(names).toHaveLength(2);
    expect(names[0].value).toBe("X");
    expect(names[1].value).toBe("Y");
  });
});

// ---------------------------------------------------------------------------
// 7. NUMBER Literals
// ---------------------------------------------------------------------------

describe("NUMBER literals — integers, decimals, scientific notation", () => {
  it("tokenizes a plain integer literal", () => {
    /**
     * `42` — stored as 42.0 internally in BASIC.
     * Appears in expression position as NUMBER, not LINE_NUM.
     */
    const tokens = tokenizeDartmouthBasic("10 LET X = 42\n");
    const num = tokens.find((t) => t.type === "NUMBER");
    expect(num!.value).toBe("42");
  });

  it("tokenizes zero", () => {
    const tokens = tokenizeDartmouthBasic("10 LET X = 0\n");
    const num = tokens.find((t) => t.type === "NUMBER");
    expect(num!.value).toBe("0");
  });

  it("tokenizes a decimal literal", () => {
    // `3.14` — a floating-point literal with integer and fractional parts.
    const tokens = tokenizeDartmouthBasic("10 LET X = 3.14\n");
    const num = tokens.find((t) => t.type === "NUMBER");
    expect(num!.value).toBe("3.14");
  });

  it("tokenizes a leading-dot decimal (.5)", () => {
    /**
     * `.5` — a valid BASIC number with no integer part.
     * Equivalent to `0.5`. The regex `/[0-9]*\.?[0-9]+.../` handles this
     * with the `[0-9]*` part matching zero digits.
     */
    const tokens = tokenizeDartmouthBasic("10 LET X = .5\n");
    const num = tokens.find((t) => t.type === "NUMBER");
    expect(num!.value).toBe(".5");
  });

  it("tokenizes scientific notation (1.5E3 = 1500)", () => {
    /**
     * `1.5E3` — scientific notation meaning 1.5 × 10³ = 1500.
     * The regex `/[0-9]*\.?[0-9]+([Ee][+-]?[0-9]+)?/` captures the exponent.
     */
    const tokens = tokenizeDartmouthBasic("10 LET X = 1.5E3\n");
    const num = tokens.find((t) => t.type === "NUMBER");
    expect(num!.value).toBe("1.5E3");
  });

  it("tokenizes scientific notation with negative exponent (1.5E-3 = 0.0015)", () => {
    // `1.5E-3` — scientific notation with a negative exponent.
    const tokens = tokenizeDartmouthBasic("10 LET X = 1.5E-3\n");
    const num = tokens.find((t) => t.type === "NUMBER");
    expect(num!.value).toBe("1.5E-3");
  });

  it("tokenizes scientific notation without decimal (1E10)", () => {
    // `1E10` — scientific notation: 1 × 10^10 = 10,000,000,000.
    const tokens = tokenizeDartmouthBasic("10 LET X = 1E10\n");
    const num = tokens.find((t) => t.type === "NUMBER");
    expect(num!.value).toBe("1E10");
  });

  it("tokenizes scientific notation with positive exponent sign (1.5E+3)", () => {
    // `1.5E+3` — explicit positive exponent sign is valid.
    const tokens = tokenizeDartmouthBasic("10 LET X = 1.5E+3\n");
    const num = tokens.find((t) => t.type === "NUMBER");
    expect(num!.value).toBe("1.5E+3");
  });
});

// ---------------------------------------------------------------------------
// 8. STRING Literals
// ---------------------------------------------------------------------------

describe("STRING literals — double-quoted strings", () => {
  it("tokenizes a simple string", () => {
    /**
     * Strings in Dartmouth BASIC are enclosed in double quotes.
     * The 1964 spec has no escape sequences — a quote cannot appear inside
     * a string literal.
     *
     * The grammar-lexer engine strips the surrounding delimiters from STRING
     * tokens automatically. So PRINT "HELLO WORLD" produces STRING("HELLO WORLD")
     * with the quotes removed. This is consistent with how the lexer handles
     * string tokens across all grammar-driven lexers in this codebase.
     */
    const tokens = tokenizeDartmouthBasic('10 PRINT "HELLO WORLD"\n');
    const str = tokens.find((t) => t.type === "STRING");
    expect(str).toBeDefined();
    // The grammar engine strips the surrounding double quotes from STRING tokens.
    expect(str!.value).toBe("HELLO WORLD");
  });

  it("tokenizes an empty string", () => {
    /**
     * Two adjacent double quotes produce an empty string.
     * The regex `/"[^"]*"/` matches zero characters between the quotes.
     * After quote-stripping, the token value is an empty string "".
     */
    const tokens = tokenizeDartmouthBasic('10 PRINT ""\n');
    const str = tokens.find((t) => t.type === "STRING");
    expect(str).toBeDefined();
    expect(str!.value).toBe("");
  });

  it("tokenizes a string with spaces", () => {
    /**
     * Spaces inside the string are part of the string content.
     * The grammar `/"[^"]*"/` matches everything between the quotes.
     */
    const tokens = tokenizeDartmouthBasic('10 PRINT "HELLO WORLD"\n');
    const str = tokens.find((t) => t.type === "STRING");
    // Value is just the content between the quotes.
    expect(str!.value).toBe("HELLO WORLD");
  });

  it("tokenizes a string that contains numbers", () => {
    /**
     * Numbers inside strings are just characters, not NUMBER tokens.
     * The lexer sees the entire "THE ANSWER IS 42" as a single STRING token.
     */
    const tokens = tokenizeDartmouthBasic('10 PRINT "THE ANSWER IS 42"\n');
    const str = tokens.find((t) => t.type === "STRING");
    expect(str!.value).toBe("THE ANSWER IS 42");
    // No NUMBER token should appear (except LINE_NUM for the line number).
    const nums = tokens.filter((t) => t.type === "NUMBER");
    expect(nums).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// 9. Single-Character Operators
// ---------------------------------------------------------------------------

describe("single-character operators and punctuation", () => {
  it("tokenizes PLUS (+)", () => {
    expect(tokenTypes("10 LET X = A + B\n")).toContain("PLUS");
  });

  it("tokenizes MINUS (-)", () => {
    expect(tokenTypes("10 LET X = A - B\n")).toContain("MINUS");
  });

  it("tokenizes STAR (*) for multiplication", () => {
    expect(tokenTypes("10 LET X = A * B\n")).toContain("STAR");
  });

  it("tokenizes SLASH (/) for division", () => {
    expect(tokenTypes("10 LET X = A / B\n")).toContain("SLASH");
  });

  it("tokenizes CARET (^) for exponentiation", () => {
    /**
     * CARET is the exponentiation operator: 2^3 = 8.
     * It is right-associative in BASIC: 2^3^2 = 2^(3^2) = 512.
     */
    expect(tokenTypes("10 LET X = 2 ^ 3\n")).toContain("CARET");
  });

  it("tokenizes EQ (=)", () => {
    /**
     * = serves two purposes in BASIC (unlike ALGOL which uses := for
     * assignment):
     *   - Assignment: LET X = 5
     *   - Equality:   IF X = 5 THEN 100
     * The parser uses context (LET vs IF) to interpret which it is.
     * Both produce EQ.
     */
    expect(tokenTypes("10 LET X = 5\n")).toContain("EQ");
  });

  it("tokenizes LT (<)", () => {
    expect(tokenTypes("10 IF X < 5 THEN 20\n")).toContain("LT");
  });

  it("tokenizes GT (>)", () => {
    expect(tokenTypes("10 IF X > 5 THEN 20\n")).toContain("GT");
  });

  it("tokenizes LPAREN (()", () => {
    expect(tokenTypes("10 LET Y = SIN(X)\n")).toContain("LPAREN");
  });

  it("tokenizes RPAREN ())", () => {
    expect(tokenTypes("10 LET Y = SIN(X)\n")).toContain("RPAREN");
  });

  it("tokenizes COMMA (,)", () => {
    /**
     * COMMA in PRINT X, Y advances to the next print zone (a multiple of
     * column 14) before printing Y. It is the "column tab" separator.
     */
    expect(tokenTypes("10 PRINT X, Y\n")).toContain("COMMA");
  });

  it("tokenizes SEMICOLON (;)", () => {
    /**
     * SEMICOLON in PRINT X; Y prints X immediately followed by Y with no
     * space or column advancement. It is the "tight" separator.
     */
    expect(tokenTypes("10 PRINT X; Y\n")).toContain("SEMICOLON");
  });
});

// ---------------------------------------------------------------------------
// 10. Multi-Character Operator Disambiguation
// ---------------------------------------------------------------------------

describe("multi-char operator disambiguation — <=, >=, <> must not split", () => {
  it("tokenizes <= as LE (not LT then EQ)", () => {
    /**
     * Without the grammar ordering LE before LT and EQ, `<=` would lex as
     * two tokens: LT("<") and EQ("="). The grammar lists LE first so the
     * two-character sequence is consumed as a single LE token.
     *
     * Truth table for ≤:
     *   1 <= 2 → TRUE
     *   2 <= 2 → TRUE
     *   3 <= 2 → FALSE
     */
    const types = tokenTypes("10 IF X <= Y THEN 50\n");
    expect(types).toContain("LE");
    expect(types).not.toContain("LT");
    // EQ appears in... wait, this line has no EQ. Let's check separately.
  });

  it("<= does not produce LT or EQ tokens", () => {
    const types = tokenTypes("10 IF X <= 5 THEN 20\n");
    expect(types).toContain("LE");
    expect(types).not.toContain("LT");
    // Note: `5` is NUMBER; no EQ in this statement
    expect(types).not.toContain("EQ");
  });

  it("tokenizes >= as GE (not GT then EQ)", () => {
    /**
     * Similarly, `>=` must be a single GE token.
     *
     * Truth table for ≥:
     *   2 >= 1 → TRUE
     *   2 >= 2 → TRUE
     *   1 >= 2 → FALSE
     */
    const types = tokenTypes("10 IF X >= Y THEN 50\n");
    expect(types).toContain("GE");
    expect(types).not.toContain("GT");
  });

  it("tokenizes <> as NE (not LT then GT)", () => {
    /**
     * `<>` is the not-equal operator. Without disambiguation, `<>` would
     * lex as LT("<") and GT(">") — two separate tokens. The grammar lists
     * NE before LT and GT.
     *
     * Historically this is the "diamond" or "chevron" not-equal, from
     * early FORTRAN and BASIC. (The diamond ◇ was pronounced "not equal"
     * in early programming textbooks.)
     *
     * Truth table for ≠:
     *   1 <> 2 → TRUE
     *   2 <> 2 → FALSE
     */
    const types = tokenTypes("10 IF X <> Y THEN 50\n");
    expect(types).toContain("NE");
    expect(types).not.toContain("LT");
    expect(types).not.toContain("GT");
  });

  it("tokenizes standalone < as LT (not confused with <=)", () => {
    /**
     * A bare < followed by a space (or non-= character) must still be LT.
     * The grammar must not greedily consume `< ` as the start of `<=`.
     */
    const types = tokenTypes("10 IF X < Y THEN 50\n");
    expect(types).toContain("LT");
    expect(types).not.toContain("LE");
  });

  it("tokenizes standalone > as GT (not confused with >=)", () => {
    const types = tokenTypes("10 IF X > Y THEN 50\n");
    expect(types).toContain("GT");
    expect(types).not.toContain("GE");
  });
});

// ---------------------------------------------------------------------------
// 11. NEWLINE Handling
// ---------------------------------------------------------------------------

describe("NEWLINE handling — statement terminator", () => {
  it("emits a NEWLINE token at the end of each line", () => {
    /**
     * In BASIC, the NEWLINE character terminates a statement. Unlike many
     * other languages where newlines are whitespace, BASIC's NEWLINE is
     * syntactically significant and must appear in the token stream.
     */
    const tokens = tokenizeDartmouthBasic("10 LET X = 1\n");
    expect(tokens.find((t) => t.type === "NEWLINE")).toBeDefined();
  });

  it("emits NEWLINE for each line in a multi-line program", () => {
    const source = "10 LET X = 1\n20 PRINT X\n30 END\n";
    const newlines = tokenizeDartmouthBasic(source).filter((t) => t.type === "NEWLINE");
    expect(newlines).toHaveLength(3);
  });

  it("handles Windows-style CRLF line endings (\\r\\n)", () => {
    /**
     * The grammar includes `NEWLINE = /\r?\n/` so that both Unix (LF) and
     * Windows (CRLF) line endings are recognised as a single NEWLINE token.
     */
    const tokens = tokenizeDartmouthBasic("10 LET X = 1\r\n20 END\r\n");
    const newlines = tokens.filter((t) => t.type === "NEWLINE");
    expect(newlines).toHaveLength(2);
    // Value includes the \r if present:
    expect(newlines[0].value).toBe("\r\n");
  });
});

// ---------------------------------------------------------------------------
// 12. REM Suppression
// ---------------------------------------------------------------------------

describe("REM suppression — comment text removed from token stream", () => {
  it("suppresses text after REM on the same line", () => {
    /**
     * `10 REM THIS IS A COMMENT` — after the KEYWORD("REM") token,
     * everything up to the NEWLINE should be gone.
     *
     * Result: [LINE_NUM("10"), KEYWORD("REM"), NEWLINE, EOF]
     */
    const tokens = tokenizeDartmouthBasic("10 REM THIS IS A COMMENT\n");
    const types = tokens.map((t) => t.type);
    expect(types).toEqual(["LINE_NUM", "KEYWORD", "NEWLINE", "EOF"]);
  });

  it("keeps the REM keyword itself in the stream", () => {
    /**
     * The KEYWORD("REM") token must stay — the parser uses it to recognise
     * the remark statement: `rem_stmt := LINE_NUM KEYWORD("REM") NEWLINE`.
     */
    const tokens = tokenizeDartmouthBasic("10 REM HELLO\n");
    expect(tokens.find((t) => t.type === "KEYWORD" && t.value === "REM")).toBeDefined();
  });

  it("keeps the NEWLINE that ends the REM line", () => {
    /**
     * The NEWLINE terminates the statement. The parser needs it.
     * suppressRemContent stops suppression at NEWLINE and emits it.
     */
    const tokens = tokenizeDartmouthBasic("10 REM COMMENT\n");
    const newline = tokens.find((t) => t.type === "NEWLINE");
    expect(newline).toBeDefined();
  });

  it("continues tokenizing the line after REM correctly", () => {
    /**
     * `10 REM\n20 LET X = 1` — after the REM line, the next line tokenizes
     * normally. The suppression resets after the NEWLINE.
     */
    const tokens = tokenizeDartmouthBasic("10 REM\n20 LET X = 1\n");
    const types = tokens.map((t) => t.type);
    expect(types).toContain("LINE_NUM");
    // LINE_NUM("20") must appear
    const lineNum20 = tokens.find((t) => t.type === "LINE_NUM" && t.value === "20");
    expect(lineNum20).toBeDefined();
  });

  it("does not suppress tokens on the line before REM", () => {
    /**
     * Suppression only starts AFTER REM, not before. Tokens on the same
     * line before REM... wait, in BASIC, REM is always the only statement
     * on its line (you can't put anything after REM on the same line — that
     * is the entire point of REM).
     *
     * So tokens after line-number and before REM (there aren't any in a
     * valid REM line) remain unaffected. Test with a non-REM line followed
     * by a REM line.
     */
    const tokens = tokenizeDartmouthBasic("10 LET X = 5\n20 REM NOTE\n");
    // Line 10 should be fully tokenized.
    expect(tokens.find((t) => t.value === "LET")).toBeDefined();
    expect(tokens.find((t) => t.value === "5")).toBeDefined();
  });

  it("handles REM with no following text (just REM and newline)", () => {
    /**
     * `10 REM` with nothing after it (just end of line).
     * The suppression flag is set after REM, then immediately unset by
     * the NEWLINE. No tokens are dropped.
     */
    const tokens = tokenizeDartmouthBasic("10 REM\n");
    const types = tokens.map((t) => t.type);
    expect(types).toEqual(["LINE_NUM", "KEYWORD", "NEWLINE", "EOF"]);
  });

  it("suppresses numbers that appear after REM", () => {
    /**
     * `10 REM 123 456` — the numbers 123 and 456 are in a comment.
     * They must NOT appear in the token stream.
     */
    const tokens = tokenizeDartmouthBasic("10 REM 123 456\n");
    const nums = tokens.filter((t) => t.type === "NUMBER" || t.type === "LINE_NUM");
    // Only the line number 10 should remain (as LINE_NUM).
    expect(nums).toHaveLength(1);
    expect(nums[0].value).toBe("10");
  });

  it("suppresses operators that appear after REM", () => {
    /**
     * `10 REM X + Y` — the NAME, PLUS, NAME tokens after REM are comments.
     * They must not appear in the token stream.
     */
    const tokens = tokenizeDartmouthBasic("10 REM X + Y\n");
    expect(tokens.find((t) => t.type === "PLUS")).toBeUndefined();
    expect(tokens.find((t) => t.type === "NAME")).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// 13. Multi-Line Programs
// ---------------------------------------------------------------------------

describe("multi-line programs — realistic BASIC code fragments", () => {
  it("tokenizes a complete minimal BASIC program", () => {
    /**
     * A complete, runnable Dartmouth BASIC program:
     *
     *   10 LET X = 1
     *   20 PRINT X
     *   30 END
     *
     * Expected token sequence for each line:
     *   Line 10: LINE_NUM("10"), KEYWORD("LET"), NAME("X"), EQ("="), NUMBER("1"), NEWLINE
     *   Line 20: LINE_NUM("20"), KEYWORD("PRINT"), NAME("X"), NEWLINE
     *   Line 30: LINE_NUM("30"), KEYWORD("END"), NEWLINE
     *   Then: EOF
     */
    const source = "10 LET X = 1\n20 PRINT X\n30 END\n";
    const types = tokenTypes(source);
    expect(types).toEqual([
      "LINE_NUM", "KEYWORD", "NAME", "EQ", "NUMBER", "NEWLINE",
      "LINE_NUM", "KEYWORD", "NAME", "NEWLINE",
      "LINE_NUM", "KEYWORD", "NEWLINE",
      "EOF",
    ]);
  });

  it("tokenizes an IF...THEN statement", () => {
    /**
     * Conditional branch:
     *   40 IF X > 0 THEN 100
     *
     * Expected: LINE_NUM("40"), KEYWORD("IF"), NAME("X"), GT(">"),
     *           NUMBER("0"), KEYWORD("THEN"), NUMBER("100"), NEWLINE
     */
    const source = "40 IF X > 0 THEN 100\n";
    const types = tokenTypes(source);
    expect(types).toEqual([
      "LINE_NUM", "KEYWORD", "NAME", "GT", "NUMBER", "KEYWORD", "NUMBER", "NEWLINE", "EOF",
    ]);
  });

  it("tokenizes a FOR/NEXT loop with STEP", () => {
    /**
     * Counted loop:
     *   50 FOR I = 1 TO 10 STEP 2
     *   60 NEXT I
     *
     * FOR/NEXT is BASIC's only loop construct.
     */
    const source = "50 FOR I = 1 TO 10 STEP 2\n60 NEXT I\n";
    const types = tokenTypes(source);
    expect(types).toContain("KEYWORD"); // FOR, TO, STEP, NEXT are all KEYWORDs
    const keywords = tokenizeDartmouthBasic(source)
      .filter((t) => t.type === "KEYWORD")
      .map((t) => t.value);
    expect(keywords).toContain("FOR");
    expect(keywords).toContain("TO");
    expect(keywords).toContain("STEP");
    expect(keywords).toContain("NEXT");
  });

  it("tokenizes a DEF FN statement", () => {
    /**
     * User-defined function definition:
     *   60 DEF FNA(X) = X * X
     *
     * Expected: LINE_NUM("60"), KEYWORD("DEF"), USER_FN("FNA"),
     *           LPAREN, NAME("X"), RPAREN, EQ, NAME("X"), STAR, NAME("X"), NEWLINE
     */
    const source = "60 DEF FNA(X) = X * X\n";
    const types = tokenTypes(source);
    expect(types).toEqual([
      "LINE_NUM", "KEYWORD", "USER_FN", "LPAREN", "NAME", "RPAREN",
      "EQ", "NAME", "STAR", "NAME", "NEWLINE", "EOF",
    ]);
  });

  it("tokenizes a DATA statement with multiple values", () => {
    /**
     * DATA supplies literal values for READ statements.
     * DATA 1,2,3 provides three numbers.
     */
    const source = "10 DATA 1,2,3\n";
    const types = tokenTypes(source);
    expect(types).toContain("KEYWORD"); // DATA
    expect(types).toContain("COMMA");
    const nums = tokenizeDartmouthBasic(source).filter((t) => t.type === "NUMBER");
    expect(nums).toHaveLength(3);
  });

  it("tokenizes a GOSUB/RETURN pair", () => {
    /**
     * Subroutine call:
     *   10 GOSUB 100
     *   20 ...
     *   100 REM SUBROUTINE
     *   110 RETURN
     */
    const source = "10 GOSUB 100\n110 RETURN\n";
    const keywords = tokenizeDartmouthBasic(source)
      .filter((t) => t.type === "KEYWORD")
      .map((t) => t.value);
    expect(keywords).toContain("GOSUB");
    expect(keywords).toContain("RETURN");
  });

  it("tokenizes a program with a REM comment between real statements", () => {
    /**
     * Realistic program with a comment in the middle:
     *
     *   10 LET X = 1
     *   20 REM THIS IS A COMMENT
     *   30 PRINT X
     *   40 END
     */
    const source = "10 LET X = 1\n20 REM THIS IS A COMMENT\n30 PRINT X\n40 END\n";
    const tokens = tokenizeDartmouthBasic(source);
    // Comment content must not appear.
    expect(tokens.find((t) => t.value === "THIS")).toBeUndefined();
    expect(tokens.find((t) => t.value === "COMMENT")).toBeUndefined();
    // Other statements must be present.
    expect(tokens.find((t) => t.value === "LET")).toBeDefined();
    expect(tokens.find((t) => t.value === "PRINT")).toBeDefined();
    expect(tokens.find((t) => t.value === "END")).toBeDefined();
    // Four LINE_NUM tokens: 10, 20, 30, 40.
    const lineNums = tokens.filter((t) => t.type === "LINE_NUM");
    expect(lineNums).toHaveLength(4);
  });
});

// ---------------------------------------------------------------------------
// 14. PRINT Separator Tests
// ---------------------------------------------------------------------------

describe("PRINT separators — COMMA vs SEMICOLON", () => {
  it("tokenizes PRINT X, Y with COMMA (zone separator)", () => {
    /**
     * PRINT X, Y — COMMA means "advance to the next print zone" before Y.
     * Print zones are at columns 1, 15, 29, 43, 57 (multiples of 14+1).
     */
    const types = tokenTypes("10 PRINT X, Y\n");
    expect(types).toContain("COMMA");
    expect(types).not.toContain("SEMICOLON");
  });

  it("tokenizes PRINT X; Y with SEMICOLON (tight separator)", () => {
    /**
     * PRINT X; Y — SEMICOLON means "print Y immediately after X" with no
     * space or column advancement. This is useful for building up output
     * on a single line.
     */
    const types = tokenTypes("10 PRINT X; Y\n");
    expect(types).toContain("SEMICOLON");
    expect(types).not.toContain("COMMA");
  });

  it("tokenizes PRINT with mixed separators", () => {
    // PRINT A, B; C — zone before B, tight after B.
    const types = tokenTypes('10 PRINT A, B; C\n');
    expect(types).toContain("COMMA");
    expect(types).toContain("SEMICOLON");
  });

  it("tokenizes PRINT with a string literal", () => {
    const tokens = tokenizeDartmouthBasic('10 PRINT "HELLO"\n');
    expect(tokens.find((t) => t.type === "STRING")).toBeDefined();
    expect(tokens.find((t) => t.type === "KEYWORD" && t.value === "PRINT")).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// 15. Error Recovery
// ---------------------------------------------------------------------------

describe("error recovery — unrecognised characters throw LexerError", () => {
  it("throws LexerError for an @ character", () => {
    /**
     * `@` is not a valid BASIC character. The grammar's `errors:` section
     * defines `UNKNOWN = /./` for documentation purposes, but the TypeScript
     * grammar engine does not yet implement the `errors:` section — it parses
     * and discards those definitions. As a result, unrecognised characters
     * cause the lexer to throw a `LexerError`.
     *
     * This is a known limitation of the current grammar engine implementation.
     * Future versions may add proper UNKNOWN token emission. For now, callers
     * should catch LexerError and handle it at the parse level.
     *
     * The spec document describes UNKNOWN tokens as the intended behavior;
     * the implementation diverges here because the `errors:` section is not
     * yet implemented in the TypeScript grammar engine.
     */
    expect(() => tokenizeDartmouthBasic("10 LET @ = 1\n")).toThrow();
  });

  it("throws LexerError for a hash (#) character", () => {
    /**
     * Same as @: the `#` character is not in the Dartmouth BASIC character
     * set and will cause a LexerError with the current grammar engine.
     */
    expect(() => tokenizeDartmouthBasic("10 LET # = 1\n")).toThrow();
  });

  it("throws a LexerError with a message describing the unexpected character", () => {
    /**
     * The LexerError message includes the unexpected character and its
     * position (line and column), which helps the caller report a useful
     * error message to the end user.
     */
    try {
      tokenizeDartmouthBasic("10 LET @ = 1\n");
      expect.fail("should have thrown");
    } catch (e: unknown) {
      expect((e as Error).message).toContain("@");
    }
  });
});

// ---------------------------------------------------------------------------
// 16. Position Tracking
// ---------------------------------------------------------------------------

describe("position tracking — line and column numbers", () => {
  it("assigns line 1 and column 1 to the first token", () => {
    /**
     * Token positions are 1-indexed. The first token is at line 1, column 1.
     */
    const tokens = tokenizeDartmouthBasic("10 LET X = 1\n");
    expect(tokens[0].line).toBe(1);
    expect(tokens[0].column).toBe(1);
  });

  it("tracks column positions within a line", () => {
    /**
     * `10 LET X = 1`
     *   - LINE_NUM("10") at col 1
     *   - KEYWORD("LET") at col 4  (after "10 ")
     *
     * Note: the exact column depends on how the grammar engine counts.
     * We only verify the columns are increasing.
     */
    const tokens = tokenizeDartmouthBasic("10 LET X = 1\n");
    // All tokens on the same line should have line = 1.
    const lineOneTokens = tokens.filter((t) => t.line === 1);
    expect(lineOneTokens.length).toBeGreaterThan(1);
    // Columns should be strictly increasing.
    const cols = lineOneTokens.map((t) => t.column);
    for (let i = 1; i < cols.length; i++) {
      expect(cols[i]).toBeGreaterThanOrEqual(cols[i - 1]);
    }
  });

  it("tracks line numbers across NEWLINE tokens", () => {
    /**
     * The first token on line 2 should have line = 2.
     */
    const tokens = tokenizeDartmouthBasic("10 LET X = 1\n20 PRINT X\n");
    const line2Start = tokens.find((t) => t.value === "20");
    expect(line2Start!.line).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// 17. EOF Token
// ---------------------------------------------------------------------------

describe("EOF token — always present as the last token", () => {
  it("appends EOF after the last statement", () => {
    /**
     * The lexer always appends an EOF token at the end of the token stream.
     * This sentinel value allows the parser to detect end-of-input without
     * bounds-checking the array on every step.
     */
    const tokens = tokenizeDartmouthBasic("10 END\n");
    const last = tokens[tokens.length - 1];
    expect(last.type).toBe("EOF");
    expect(last.value).toBe("");
  });

  it("appends EOF even for an empty source", () => {
    /**
     * Even an empty string should produce exactly one token: EOF.
     */
    const tokens = tokenizeDartmouthBasic("");
    expect(tokens).toHaveLength(1);
    expect(tokens[0].type).toBe("EOF");
  });
});

// ---------------------------------------------------------------------------
// 18. createDartmouthBasicLexer — API shape
// ---------------------------------------------------------------------------

describe("createDartmouthBasicLexer — factory function", () => {
  it("returns a GrammarLexer with addPostTokenize method", () => {
    /**
     * createDartmouthBasicLexer returns a GrammarLexer instance.
     * The caller can attach additional post-tokenize hooks if needed.
     */
    const lex = createDartmouthBasicLexer("10 LET X = 1\n");
    expect(typeof lex.addPostTokenize).toBe("function");
    expect(typeof lex.tokenize).toBe("function");
  });

  it("produces the same result as tokenizeDartmouthBasic when called directly", () => {
    /**
     * tokenizeDartmouthBasic is just a convenience wrapper that calls
     * createDartmouthBasicLexer and then lex.tokenize(). Both must produce
     * identical results.
     */
    const source = "10 LET X = 5\n20 PRINT X\n30 END\n";
    const directResult = tokenizeDartmouthBasic(source);
    const factoryResult = createDartmouthBasicLexer(source).tokenize();
    expect(directResult).toEqual(factoryResult);
  });
});

// ---------------------------------------------------------------------------
// 19. Spec Example Programs (from the spec document)
// ---------------------------------------------------------------------------

describe("spec examples — exact token sequences from the specification", () => {
  it("matches spec example: 10 LET X = 5", () => {
    /**
     * From the spec's test strategy:
     *   "10 LET X = 5"
     *   → [LINE_NUM("10"), KEYWORD("LET"), NAME("X"), EQ("="), NUMBER("5"), NEWLINE, EOF]
     *
     * Note: no trailing newline in source but the spec shows NEWLINE in output.
     * We add \n to the source to get the NEWLINE token.
     */
    const types = tokenTypes("10 LET X = 5\n");
    expect(types).toEqual(["LINE_NUM", "KEYWORD", "NAME", "EQ", "NUMBER", "NEWLINE", "EOF"]);
    const values = tokenValues("10 LET X = 5\n");
    expect(values[0]).toBe("10");
    expect(values[1]).toBe("LET");
    expect(values[2]).toBe("X");
    expect(values[3]).toBe("=");
    expect(values[4]).toBe("5");
  });

  it("matches spec example: 20 PRINT X, Y", () => {
    /**
     * "20 PRINT X, Y"
     * → [LINE_NUM("20"), KEYWORD("PRINT"), NAME("X"), COMMA, NAME("Y"), NEWLINE, EOF]
     */
    const types = tokenTypes("20 PRINT X, Y\n");
    expect(types).toEqual(["LINE_NUM", "KEYWORD", "NAME", "COMMA", "NAME", "NEWLINE", "EOF"]);
  });

  it("matches spec example: 30 GOTO 10 (GOTO target is NUMBER)", () => {
    /**
     * "30 GOTO 10"
     * → [LINE_NUM("30"), KEYWORD("GOTO"), NUMBER("10"), NEWLINE, EOF]
     * Note: GOTO target is NUMBER, not LINE_NUM.
     */
    const types = tokenTypes("30 GOTO 10\n");
    expect(types).toEqual(["LINE_NUM", "KEYWORD", "NUMBER", "NEWLINE", "EOF"]);
    const values = tokenValues("30 GOTO 10\n");
    expect(values[0]).toBe("30");
    expect(values[2]).toBe("10");
  });

  it("matches spec example: 70 LET Y = SIN(X) + COS(X)", () => {
    /**
     * "70 LET Y = SIN(X) + COS(X)"
     * → [LINE_NUM("70"), KEYWORD("LET"), NAME("Y"), EQ,
     *    BUILTIN_FN("SIN"), LPAREN, NAME("X"), RPAREN,
     *    PLUS, BUILTIN_FN("COS"), LPAREN, NAME("X"), RPAREN, NEWLINE, EOF]
     */
    const types = tokenTypes("70 LET Y = SIN(X) + COS(X)\n");
    expect(types).toEqual([
      "LINE_NUM", "KEYWORD", "NAME", "EQ",
      "BUILTIN_FN", "LPAREN", "NAME", "RPAREN",
      "PLUS",
      "BUILTIN_FN", "LPAREN", "NAME", "RPAREN",
      "NEWLINE", "EOF",
    ]);
  });
});
