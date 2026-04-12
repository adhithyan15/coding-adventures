/**
 * Tests for the Dartmouth BASIC Parser (TypeScript).
 *
 * These tests verify that the grammar-driven parser correctly parses
 * 1964 Dartmouth BASIC source text when loaded with the
 * `dartmouth_basic.grammar` file.
 *
 * The BASIC grammar's top-level rule is `program` — a sequence of
 * numbered lines, each containing one statement terminated by NEWLINE.
 *
 * Historical Note
 * ---------------
 *
 * The original 1964 Dartmouth BASIC had exactly 17 statement types.
 * These tests verify all 17, plus the expression hierarchy. The test
 * programs below are representative of what students at Dartmouth College
 * would have typed on a teletype in 1964.
 *
 * Test Strategy
 * -------------
 *
 * Each test parses a BASIC program and then uses helper functions to walk
 * the resulting AST, looking for specific rule nodes. This approach is
 * robust against minor grammar changes because it verifies structure
 * rather than exact tree shape.
 *
 * Test Categories
 * ---------------
 *
 *   1. **Statement types** -- all 17 BASIC statement types
 *   2. **Expression hierarchy** -- +, -, *, /, ^, unary -, parentheses
 *   3. **Built-in functions** -- SIN, COS, TAN, ATN, EXP, LOG, ABS, SQR,
 *      INT, RND, SGN (all 11)
 *   4. **User-defined functions** -- FNA, FNZ
 *   5. **Array subscripts** -- A(I), A(I+1)
 *   6. **Multi-line programs** -- hello world, counting loop, conditional,
 *      subroutine
 *   7. **Edge cases** -- bare line number
 */

import { describe, it, expect } from "vitest";
import { parseDartmouthBasic } from "../src/parser.js";
import { isASTNode } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

// ===========================================================================
// Helper utilities
// ===========================================================================

/**
 * Recursively find all AST nodes with a given rule name.
 *
 * This is the primary inspection tool for these tests. We verify that
 * expected rule nodes exist in the tree rather than checking exact
 * structure, which makes tests resilient to minor grammar changes.
 *
 * @param node - The root node to search from.
 * @param ruleName - The grammar rule name to find (e.g., "let_stmt").
 * @returns All matching ASTNode objects found anywhere in the subtree.
 */
function findNodes(node: ASTNode, ruleName: string): ASTNode[] {
  const results: ASTNode[] = [];
  if (node.ruleName === ruleName) results.push(node);
  for (const child of node.children) {
    if (isASTNode(child)) results.push(...findNodes(child, ruleName));
  }
  return results;
}

/**
 * Check whether any node with the given rule name exists in the tree.
 * Convenience wrapper around findNodes.
 */
function hasRule(node: ASTNode, ruleName: string): boolean {
  return findNodes(node, ruleName).length > 0;
}

/**
 * Collect all leaf tokens from an AST subtree.
 *
 * Useful for inspecting what tokens are present in a subtree without
 * worrying about nesting structure.
 */
function findTokens(node: ASTNode): Token[] {
  const tokens: Token[] = [];
  for (const child of node.children) {
    if (isASTNode(child)) {
      tokens.push(...findTokens(child));
    } else {
      tokens.push(child as Token);
    }
  }
  return tokens;
}

// ===========================================================================
// Test suite
// ===========================================================================

describe("program structure", () => {
  it("produces a 'program' root node", () => {
    /**
     * The start symbol of the grammar is 'program'. Every valid BASIC
     * program must parse to a root node with this ruleName.
     */
    const ast = parseDartmouthBasic("10 END\n");
    expect(ast.ruleName).toBe("program");
  });

  it("parses a multi-line program into multiple line nodes", () => {
    /**
     * Each numbered line in the source becomes a 'line' node in the AST.
     * A two-line program should produce two line nodes.
     */
    const ast = parseDartmouthBasic("10 LET X = 1\n20 END\n");
    expect(ast.ruleName).toBe("program");

    const lines = findNodes(ast, "line");
    expect(lines.length).toBeGreaterThanOrEqual(2);
  });
});

// ===========================================================================
// Statement type 1: LET
// ===========================================================================

describe("LET statement", () => {
  it("parses a scalar LET assignment", () => {
    /**
     * LET is the assignment statement. In 1964 BASIC, the LET keyword
     * is required (unlike later BASIC dialects that drop it).
     *
     *   10 LET X = 5
     */
    const ast = parseDartmouthBasic("10 LET X = 5\n");
    expect(ast.ruleName).toBe("program");
    expect(hasRule(ast, "let_stmt")).toBe(true);
    expect(hasRule(ast, "variable")).toBe(true);
    expect(hasRule(ast, "expr")).toBe(true);
  });

  it("parses an array element LET assignment", () => {
    /**
     * LET can assign to array elements: LET A(3) = X + 1
     * The variable rule handles both scalar (NAME) and array (NAME(expr)) forms.
     */
    const ast = parseDartmouthBasic("10 LET A(3) = X + 1\n");
    expect(hasRule(ast, "let_stmt")).toBe(true);
    expect(hasRule(ast, "variable")).toBe(true);
  });
});

// ===========================================================================
// Statement type 2: PRINT
// ===========================================================================

describe("PRINT statement", () => {
  it("parses bare PRINT (blank line output)", () => {
    /**
     * PRINT with no arguments outputs a blank line. Useful for spacing
     * output on the teletype.
     */
    const ast = parseDartmouthBasic("10 PRINT\n");
    expect(hasRule(ast, "print_stmt")).toBe(true);
  });

  it("parses PRINT with an expression", () => {
    /**
     * PRINT followed by an expression prints the value.
     */
    const ast = parseDartmouthBasic("10 PRINT X + 1\n");
    expect(hasRule(ast, "print_stmt")).toBe(true);
    expect(hasRule(ast, "print_list")).toBe(true);
  });

  it("parses PRINT with a string literal", () => {
    /**
     * PRINT can output string literals in double quotes.
     * This was the primary way to display messages to users.
     */
    const ast = parseDartmouthBasic('10 PRINT "HELLO"\n');
    expect(hasRule(ast, "print_stmt")).toBe(true);
    expect(hasRule(ast, "print_list")).toBe(true);

    const tokens = findTokens(ast);
    const stringTokens = tokens.filter((t) => t.type === "STRING");
    expect(stringTokens.length).toBeGreaterThanOrEqual(1);
  });

  it("parses PRINT with comma separator", () => {
    /**
     * A comma between print items advances to the next print zone
     * (every 15 characters). Used to create tabular output.
     */
    const ast = parseDartmouthBasic("10 PRINT X, Y\n");
    expect(hasRule(ast, "print_stmt")).toBe(true);
    expect(hasRule(ast, "print_sep")).toBe(true);
  });

  it("parses PRINT with semicolon separator", () => {
    /**
     * A semicolon between print items means print them adjacent with
     * no space between. Useful for concatenating output on one line.
     */
    const ast = parseDartmouthBasic("10 PRINT X; Y\n");
    expect(hasRule(ast, "print_stmt")).toBe(true);
    expect(hasRule(ast, "print_sep")).toBe(true);
  });
});

// ===========================================================================
// Statement type 3: INPUT
// ===========================================================================

describe("INPUT statement", () => {
  it("parses INPUT with a single variable", () => {
    /**
     * INPUT pauses execution and reads a value from the user.
     * The teletype printed "?" as a prompt.
     */
    const ast = parseDartmouthBasic("10 INPUT X\n");
    expect(hasRule(ast, "input_stmt")).toBe(true);
    expect(hasRule(ast, "variable")).toBe(true);
  });

  it("parses INPUT with multiple variables", () => {
    /**
     * INPUT can read multiple values at once, separated by commas.
     * The user types the values separated by commas or on separate lines.
     */
    const ast = parseDartmouthBasic("10 INPUT A, B, C\n");
    expect(hasRule(ast, "input_stmt")).toBe(true);
  });
});

// ===========================================================================
// Statement type 4: IF-THEN (all 6 relational operators)
// ===========================================================================

describe("IF-THEN statement", () => {
  /**
   * IF-THEN is the sole conditional in 1964 BASIC. The full form is:
   *   IF expr relop expr THEN LINE_NUM
   *
   * There is no ELSE clause. The branch target must be a literal line
   * number. All six relational operators are tested below.
   */

  it("parses IF with = (equal)", () => {
    const ast = parseDartmouthBasic("10 IF X = 5 THEN 100\n");
    expect(hasRule(ast, "if_stmt")).toBe(true);
    expect(hasRule(ast, "relop")).toBe(true);
  });

  it("parses IF with < (less than)", () => {
    const ast = parseDartmouthBasic("10 IF X < 5 THEN 100\n");
    expect(hasRule(ast, "if_stmt")).toBe(true);
    expect(hasRule(ast, "relop")).toBe(true);
  });

  it("parses IF with > (greater than)", () => {
    const ast = parseDartmouthBasic("10 IF X > 5 THEN 100\n");
    expect(hasRule(ast, "if_stmt")).toBe(true);
    expect(hasRule(ast, "relop")).toBe(true);
  });

  it("parses IF with <= (less than or equal)", () => {
    const ast = parseDartmouthBasic("10 IF X <= 5 THEN 100\n");
    expect(hasRule(ast, "if_stmt")).toBe(true);
    expect(hasRule(ast, "relop")).toBe(true);
  });

  it("parses IF with >= (greater than or equal)", () => {
    const ast = parseDartmouthBasic("10 IF X >= 5 THEN 100\n");
    expect(hasRule(ast, "if_stmt")).toBe(true);
    expect(hasRule(ast, "relop")).toBe(true);
  });

  it("parses IF with <> (not equal)", () => {
    /**
     * <> is BASIC's not-equal operator. Unlike C (!=) or Pascal (<>
     * being standard there too), BASIC chose <> to mirror set notation.
     */
    const ast = parseDartmouthBasic("10 IF X <> 5 THEN 100\n");
    expect(hasRule(ast, "if_stmt")).toBe(true);
    expect(hasRule(ast, "relop")).toBe(true);
  });
});

// ===========================================================================
// Statement type 5: GOTO
// ===========================================================================

describe("GOTO statement", () => {
  it("parses GOTO with a line number", () => {
    /**
     * GOTO is the unconditional jump. In 1964 BASIC it is spelled "GOTO"
     * (one word). Edsger Dijkstra's famous 1968 letter "Go To Statement
     * Considered Harmful" was partly inspired by BASIC's heavy use of GOTO.
     */
    const ast = parseDartmouthBasic("10 GOTO 50\n");
    expect(hasRule(ast, "goto_stmt")).toBe(true);
  });
});

// ===========================================================================
// Statement types 6 & 7: GOSUB / RETURN
// ===========================================================================

describe("GOSUB and RETURN statements", () => {
  it("parses GOSUB", () => {
    /**
     * GOSUB pushes the current line number onto the call stack and jumps
     * to the target line. Unlike GOTO, execution can return.
     */
    const ast = parseDartmouthBasic("10 GOSUB 200\n");
    expect(hasRule(ast, "gosub_stmt")).toBe(true);
  });

  it("parses RETURN", () => {
    /**
     * RETURN pops the return address from the call stack and resumes
     * execution from there. Together with GOSUB, this implements
     * subroutines (named by line number rather than by name).
     */
    const ast = parseDartmouthBasic("200 RETURN\n");
    expect(hasRule(ast, "return_stmt")).toBe(true);
  });
});

// ===========================================================================
// Statement types 8 & 9: FOR / NEXT
// ===========================================================================

describe("FOR and NEXT statements", () => {
  it("parses FOR without STEP", () => {
    /**
     * FOR/NEXT is the counted loop. The loop variable counts from the
     * start value to the end value, incrementing by 1 each iteration
     * (when STEP is omitted).
     *
     *   10 FOR I = 1 TO 10
     *   20 NEXT I
     */
    const ast = parseDartmouthBasic("10 FOR I = 1 TO 10\n20 NEXT I\n");
    expect(hasRule(ast, "for_stmt")).toBe(true);
    expect(hasRule(ast, "next_stmt")).toBe(true);
  });

  it("parses FOR with STEP", () => {
    /**
     * WITH STEP, the increment can be any value — including negative for
     * counting down. FOR I = 10 TO 1 STEP -1 counts 10, 9, 8, ..., 1.
     */
    const ast = parseDartmouthBasic("10 FOR I = 10 TO 1 STEP -1\n20 NEXT I\n");
    expect(hasRule(ast, "for_stmt")).toBe(true);
    expect(hasRule(ast, "next_stmt")).toBe(true);
  });
});

// ===========================================================================
// Statement types 10 & 11: END / STOP
// ===========================================================================

describe("END and STOP statements", () => {
  it("parses END", () => {
    /**
     * END is the normal program terminator. The 1964 spec required every
     * program to have exactly one END statement as its last line. The
     * system would refuse to run a program without END.
     */
    const ast = parseDartmouthBasic("999 END\n");
    expect(hasRule(ast, "end_stmt")).toBe(true);
  });

  it("parses STOP", () => {
    /**
     * STOP halts execution with a "STOP IN LINE n" message. In the DTSS
     * (Dartmouth Time-Sharing System), the user could then CONTINUE from
     * where execution stopped. Useful for debugging.
     */
    const ast = parseDartmouthBasic("500 STOP\n");
    expect(hasRule(ast, "stop_stmt")).toBe(true);
  });
});

// ===========================================================================
// Statement type 12: REM
// ===========================================================================

describe("REM statement", () => {
  it("parses a REM comment", () => {
    /**
     * REM (remark) is BASIC's comment syntax. The lexer strips everything
     * from REM to the end of the line, so the parser sees only:
     *   LINE_NUM KEYWORD("REM") NEWLINE
     */
    const ast = parseDartmouthBasic("10 REM THIS IS A COMMENT\n");
    expect(hasRule(ast, "rem_stmt")).toBe(true);
  });
});

// ===========================================================================
// Statement types 13, 14, 15: READ / DATA / RESTORE
// ===========================================================================

describe("READ, DATA, and RESTORE statements", () => {
  it("parses READ with a single variable", () => {
    /**
     * READ pops the next value from the DATA pool and assigns it to the
     * named variable. The DATA pool is populated by DATA statements
     * (in line-number order, regardless of where they appear in the source).
     */
    const ast = parseDartmouthBasic("10 READ X\n");
    expect(hasRule(ast, "read_stmt")).toBe(true);
  });

  it("parses READ with multiple variables", () => {
    const ast = parseDartmouthBasic("10 READ A, B, C\n");
    expect(hasRule(ast, "read_stmt")).toBe(true);
  });

  it("parses DATA with numbers", () => {
    /**
     * DATA embeds values directly in the program. The runtime collects all
     * DATA values into a pool in line-number order, then READ consumes
     * them sequentially. This was the primary way to provide test data
     * or lookup tables in a BASIC program.
     */
    const ast = parseDartmouthBasic("10 DATA 1, 2, 3\n");
    expect(hasRule(ast, "data_stmt")).toBe(true);

    const tokens = findTokens(ast);
    const numberTokens = tokens.filter((t) => t.type === "NUMBER");
    expect(numberTokens.length).toBeGreaterThanOrEqual(3);
  });

  it("parses RESTORE", () => {
    /**
     * RESTORE resets the DATA pool pointer to the beginning, so the
     * next READ will return the first DATA value again. Useful for
     * programs that need to scan the data pool multiple times.
     */
    const ast = parseDartmouthBasic("10 RESTORE\n");
    expect(hasRule(ast, "restore_stmt")).toBe(true);
  });
});

// ===========================================================================
// Statement type 16: DIM
// ===========================================================================

describe("DIM statement", () => {
  it("parses DIM with a single array", () => {
    /**
     * DIM declares an array's maximum index. Without DIM, arrays default
     * to 10 elements (indices 1 through 10). DIM is needed for larger arrays.
     *
     *   10 DIM A(100)   declares A with indices 1 through 100
     */
    const ast = parseDartmouthBasic("10 DIM A(100)\n");
    expect(hasRule(ast, "dim_stmt")).toBe(true);
    expect(hasRule(ast, "dim_decl")).toBe(true);
  });

  it("parses DIM with multiple arrays", () => {
    /**
     * DIM can declare multiple arrays in one statement, separated by commas.
     *
     *   10 DIM A(10), B(20), C(5)
     */
    const ast = parseDartmouthBasic("10 DIM A(10), B(20), C(5)\n");
    expect(hasRule(ast, "dim_stmt")).toBe(true);

    const dimDecls = findNodes(ast, "dim_decl");
    expect(dimDecls.length).toBeGreaterThanOrEqual(3);
  });
});

// ===========================================================================
// Statement type 17: DEF
// ===========================================================================

describe("DEF statement", () => {
  it("parses DEF with a simple function body", () => {
    /**
     * DEF defines a user function. The 1964 spec allows 26 functions
     * (FNA through FNZ), each taking one argument. The body is any
     * arithmetic expression.
     *
     *   10 DEF FNA(X) = X * X    defines FNA as the square function
     */
    const ast = parseDartmouthBasic("10 DEF FNA(X) = X * X\n");
    expect(hasRule(ast, "def_stmt")).toBe(true);
    expect(hasRule(ast, "expr")).toBe(true);
  });

  it("parses DEF with a built-in function in the body", () => {
    /**
     * DEF bodies can reference built-in functions. This defines a
     * tangent function in terms of SIN and COS.
     */
    const ast = parseDartmouthBasic("10 DEF FNB(T) = SIN(T) / COS(T)\n");
    expect(hasRule(ast, "def_stmt")).toBe(true);
    expect(hasRule(ast, "primary")).toBe(true);
  });
});

// ===========================================================================
// Expression tests
// ===========================================================================

describe("arithmetic expressions", () => {
  it("parses addition", () => {
    const ast = parseDartmouthBasic("10 LET X = A + B\n");
    expect(hasRule(ast, "expr")).toBe(true);
  });

  it("parses subtraction", () => {
    const ast = parseDartmouthBasic("10 LET X = A - B\n");
    expect(hasRule(ast, "expr")).toBe(true);
  });

  it("parses multiplication", () => {
    const ast = parseDartmouthBasic("10 LET X = A * B\n");
    expect(hasRule(ast, "term")).toBe(true);
  });

  it("parses division", () => {
    const ast = parseDartmouthBasic("10 LET X = A / B\n");
    expect(hasRule(ast, "term")).toBe(true);
  });

  it("parses exponentiation (right-associative)", () => {
    /**
     * Exponentiation is right-associative in BASIC:
     *   2^3^2 = 2^(3^2) = 2^9 = 512  (not (2^3)^2 = 64)
     *
     * This matches mathematical convention and the 1964 Dartmouth spec.
     * The `power` rule encodes right-associativity by recursing on its
     * right operand: `power = unary [ CARET power ]`
     */
    const ast = parseDartmouthBasic("10 LET X = 2 ^ 3 ^ 2\n");
    expect(hasRule(ast, "power")).toBe(true);
  });

  it("parses unary minus", () => {
    /**
     * Unary minus negates its operand: -X, -3.14, -(X + 1).
     * Note: unary PLUS is not in the 1964 Dartmouth BASIC spec.
     */
    const ast = parseDartmouthBasic("10 LET X = -Y\n");
    expect(hasRule(ast, "unary")).toBe(true);
  });

  it("parses parenthesised expressions", () => {
    /**
     * Parentheses override the default precedence rules.
     * (A + B) * C is different from A + B * C.
     */
    const ast = parseDartmouthBasic("10 LET X = (A + B) * C\n");
    expect(hasRule(ast, "primary")).toBe(true);
  });

  it("parses complex precedence correctly", () => {
    /**
     * 2 + 3 * 4 ^ 2 should parse as:
     *   2 + (3 * (4 ^ 2)) = 2 + (3 * 16) = 2 + 48 = 50
     *
     * All three precedence levels must be present in the AST.
     */
    const ast = parseDartmouthBasic("10 LET X = 2 + 3 * 4 ^ 2\n");
    expect(hasRule(ast, "expr")).toBe(true);
    expect(hasRule(ast, "term")).toBe(true);
    expect(hasRule(ast, "power")).toBe(true);
  });
});

// ===========================================================================
// Built-in function tests (all 11)
// ===========================================================================

describe("built-in functions", () => {
  /**
   * 1964 Dartmouth BASIC has exactly 11 built-in mathematical functions.
   * All take a single argument. They are designed for scientific computing
   * on the GE-225 mainframe's floating-point unit.
   */

  it("parses SIN (sine)", () => {
    const ast = parseDartmouthBasic("10 LET X = SIN(3.14159)\n");
    expect(hasRule(ast, "primary")).toBe(true);
  });

  it("parses COS (cosine)", () => {
    const ast = parseDartmouthBasic("10 LET X = COS(0)\n");
    expect(hasRule(ast, "primary")).toBe(true);
  });

  it("parses TAN (tangent)", () => {
    const ast = parseDartmouthBasic("10 LET X = TAN(X)\n");
    expect(hasRule(ast, "primary")).toBe(true);
  });

  it("parses ATN (arctangent)", () => {
    /**
     * ATN(1)*4 approximates π (≈ 3.14159...). A common idiom in BASIC
     * programs that need π without a built-in PI constant.
     */
    const ast = parseDartmouthBasic("10 LET X = ATN(1)\n");
    expect(hasRule(ast, "primary")).toBe(true);
  });

  it("parses EXP (e raised to a power)", () => {
    /**
     * EXP(X) = e^X where e ≈ 2.71828. EXP(1) ≈ 2.71828.
     */
    const ast = parseDartmouthBasic("10 LET X = EXP(1)\n");
    expect(hasRule(ast, "primary")).toBe(true);
  });

  it("parses LOG (natural logarithm)", () => {
    /**
     * LOG computes the natural logarithm (base e), NOT base 10.
     * This was a common source of confusion for students who expected
     * LOG to mean log₁₀.
     */
    const ast = parseDartmouthBasic("10 LET X = LOG(2.71828)\n");
    expect(hasRule(ast, "primary")).toBe(true);
  });

  it("parses ABS (absolute value)", () => {
    const ast = parseDartmouthBasic("10 LET X = ABS(Y - 5)\n");
    expect(hasRule(ast, "primary")).toBe(true);
  });

  it("parses SQR (square root)", () => {
    /**
     * SQR(X) = √X. Equivalent to X^0.5 but more readable.
     */
    const ast = parseDartmouthBasic("10 LET X = SQR(2)\n");
    expect(hasRule(ast, "primary")).toBe(true);
  });

  it("parses INT (integer truncation)", () => {
    /**
     * INT truncates toward negative infinity (floor function):
     *   INT(3.9) = 3
     *   INT(-3.1) = -4  (not -3!)
     *
     * This differs from C's integer cast, which truncates toward zero.
     */
    const ast = parseDartmouthBasic("10 LET X = INT(Y)\n");
    expect(hasRule(ast, "primary")).toBe(true);
  });

  it("parses RND (random number)", () => {
    /**
     * RND(X) returns a random number uniformly distributed in [0, 1).
     * The argument X is required but its value is ignored in most
     * implementations (legacy of the original GE-225 implementation).
     */
    const ast = parseDartmouthBasic("10 LET X = RND(1)\n");
    expect(hasRule(ast, "primary")).toBe(true);
  });

  it("parses SGN (sign function)", () => {
    /**
     * SGN(X) returns:
     *   -1 if X < 0
     *    0 if X = 0
     *   +1 if X > 0
     *
     * Useful for conditional logic without IF-THEN.
     */
    const ast = parseDartmouthBasic("10 LET X = SGN(Y)\n");
    expect(hasRule(ast, "primary")).toBe(true);
  });
});

// ===========================================================================
// User-defined function tests
// ===========================================================================

describe("user-defined functions", () => {
  it("parses FNA call after DEF", () => {
    /**
     * User functions (FNA through FNZ) are declared with DEF and called
     * just like built-in functions. FNA(5) calls the function defined
     * with DEF FNA(X) = ...
     */
    const src = "10 DEF FNA(X) = X * X\n20 LET Y = FNA(5)\n";
    const ast = parseDartmouthBasic(src);
    expect(hasRule(ast, "def_stmt")).toBe(true);
    expect(hasRule(ast, "primary")).toBe(true);
  });

  it("parses FNZ (last user function)", () => {
    /**
     * FNZ is the last of the 26 user-defined function slots. Testing
     * both ends of the range (FNA and FNZ) ensures the lexer's USER_FN
     * recognition is complete.
     */
    const src = "10 DEF FNZ(T) = SIN(T) / COS(T)\n20 LET X = FNZ(1)\n";
    const ast = parseDartmouthBasic(src);
    expect(hasRule(ast, "def_stmt")).toBe(true);
  });
});

// ===========================================================================
// Array subscript tests
// ===========================================================================

describe("array subscripts in expressions", () => {
  it("parses a simple array access A(I)", () => {
    /**
     * Arrays are accessed with parentheses: A(I). The variable rule
     * handles both scalars (NAME) and array elements (NAME(expr)).
     */
    const ast = parseDartmouthBasic("10 LET X = A(I)\n");
    expect(hasRule(ast, "variable")).toBe(true);
  });

  it("parses an array access with a complex index A(I+1)", () => {
    /**
     * The array index can be any expression, including arithmetic.
     * A(I+1) accesses the element at position I+1.
     */
    const ast = parseDartmouthBasic("10 LET X = A(I + 1)\n");
    expect(hasRule(ast, "variable")).toBe(true);
  });
});

// ===========================================================================
// Multi-line program tests
// ===========================================================================

describe("multi-line programs", () => {
  it("parses a hello world program", () => {
    /**
     * The quintessential first BASIC program. In 1964, Dartmouth students
     * would type this on the teletype, press RETURN, and see their message
     * printed back — often for the first time in their programming lives.
     */
    const src = '10 PRINT "HELLO, WORLD"\n20 END\n';
    const ast = parseDartmouthBasic(src);
    expect(ast.ruleName).toBe("program");

    const lines = findNodes(ast, "line");
    expect(lines.length).toBeGreaterThanOrEqual(2);
    expect(hasRule(ast, "print_stmt")).toBe(true);
    expect(hasRule(ast, "end_stmt")).toBe(true);
  });

  it("parses a counting loop program", () => {
    /**
     * A FOR/NEXT loop that prints numbers 1 through 10. This was a
     * standard example in the original 1964 Dartmouth BASIC manual,
     * demonstrating the power of structured loops over GOTO.
     */
    const src = "10 FOR I = 1 TO 10\n20 PRINT I\n30 NEXT I\n40 END\n";
    const ast = parseDartmouthBasic(src);
    expect(hasRule(ast, "for_stmt")).toBe(true);
    expect(hasRule(ast, "print_stmt")).toBe(true);
    expect(hasRule(ast, "next_stmt")).toBe(true);
    expect(hasRule(ast, "end_stmt")).toBe(true);
  });

  it("parses a conditional program using IF-THEN and GOTO", () => {
    /**
     * Before FOR/NEXT was common knowledge, this was how loops were written
     * in BASIC: combine IF-THEN (to check the loop condition) with GOTO
     * (to jump back to the loop start).
     */
    const src =
      "10 LET I = 1\n" +
      "20 IF I > 10 THEN 60\n" +
      "30 PRINT I\n" +
      "40 LET I = I + 1\n" +
      "50 GOTO 20\n" +
      "60 END\n";
    const ast = parseDartmouthBasic(src);
    expect(hasRule(ast, "if_stmt")).toBe(true);
    expect(hasRule(ast, "goto_stmt")).toBe(true);
    expect(hasRule(ast, "let_stmt")).toBe(true);
  });

  it("parses a subroutine program using GOSUB/RETURN", () => {
    /**
     * GOSUB/RETURN lets you reuse code without GOTO spaghetti. This program
     * calls the same subroutine (starting at line 100) twice, then ends.
     */
    const src =
      "10 GOSUB 100\n" +
      "20 GOSUB 100\n" +
      "30 END\n" +
      '100 PRINT "SUBROUTINE"\n' +
      "110 RETURN\n";
    const ast = parseDartmouthBasic(src);
    expect(hasRule(ast, "gosub_stmt")).toBe(true);
    expect(hasRule(ast, "return_stmt")).toBe(true);
  });
});

// ===========================================================================
// Edge case tests
// ===========================================================================

describe("edge cases", () => {
  it("parses a bare line number as a valid no-op line", () => {
    /**
     * A bare line number with no statement is valid BASIC. In the DTSS
     * interactive environment, typing just a line number deleted that
     * line from the program. When parsing a stored program, it becomes
     * a no-op `line` node containing only LINE_NUM and NEWLINE.
     */
    const ast = parseDartmouthBasic("10\n");
    expect(ast.ruleName).toBe("program");

    const lines = findNodes(ast, "line");
    expect(lines).toHaveLength(1);
  });
});
