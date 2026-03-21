/**
 * Tests for cross-validation between .tokens and .grammar files.
 *
 * Cross-validation ensures that the two grammar files are consistent with
 * each other: every token referenced in the grammar is defined in the tokens
 * file, and unused tokens are reported as warnings.
 */

import { describe, expect, it } from "vitest";
import { crossValidate } from "../src/cross-validator.js";
import { parseParserGrammar } from "../src/parser-grammar.js";
import { parseTokenGrammar } from "../src/token-grammar.js";

// ---------------------------------------------------------------------------
// Happy paths — grammars are consistent
// ---------------------------------------------------------------------------

describe("CrossValidateHappy", () => {
  it("should produce no errors when all references resolve", () => {
    /** When every token used in the grammar is defined, no errors. */
    const tokens = parseTokenGrammar(`
NUMBER = /[0-9]+/
PLUS   = "+"
NAME   = /[a-zA-Z]+/
LPAREN = "("
RPAREN = ")"
`);
    const grammar = parseParserGrammar(`
expression = term { PLUS term } ;
term       = NUMBER | NAME | LPAREN expression RPAREN ;
`);
    const issues = crossValidate(tokens, grammar);
    // No errors — all references resolve.
    const errors = issues.filter((i) => i.startsWith("Error"));
    expect(errors).toEqual([]);
  });

  it("should produce no unused warnings when all tokens are used", () => {
    /** When every token is used, no unused warnings. */
    const tokens = parseTokenGrammar(`
NUMBER = /[0-9]+/
PLUS   = "+"
`);
    const grammar = parseParserGrammar(`
expression = NUMBER { PLUS NUMBER } ;
`);
    const issues = crossValidate(tokens, grammar);
    expect(issues).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Error cases — grammars are inconsistent
// ---------------------------------------------------------------------------

describe("CrossValidateErrors", () => {
  it("should report missing token references as errors", () => {
    /** A token referenced in the grammar but not in .tokens is an error. */
    const tokens = parseTokenGrammar(`
NUMBER = /[0-9]+/
`);
    const grammar = parseParserGrammar(`
expression = NUMBER PLUS NUMBER ;
`);
    const issues = crossValidate(tokens, grammar);
    const errors = issues.filter((i) => i.startsWith("Error"));
    expect(errors).toHaveLength(1);
    expect(errors[0]).toContain("PLUS");
  });

  it("should report unused tokens as warnings", () => {
    /** A token defined in .tokens but not used in the grammar is a warning. */
    const tokens = parseTokenGrammar(`
NUMBER = /[0-9]+/
PLUS   = "+"
MINUS  = "-"
`);
    const grammar = parseParserGrammar(`
expression = NUMBER { PLUS NUMBER } ;
`);
    const issues = crossValidate(tokens, grammar);
    const warnings = issues.filter((i) => i.startsWith("Warning"));
    expect(warnings).toHaveLength(1);
    expect(warnings[0]).toContain("MINUS");
  });

  it("should report multiple issues at once", () => {
    /** Multiple errors and warnings can be reported at once. */
    const tokens = parseTokenGrammar(`
NUMBER = /[0-9]+/
UNUSED_A = "a"
UNUSED_B = "b"
`);
    const grammar = parseParserGrammar(`
expression = NUMBER PLUS MINUS ;
`);
    const issues = crossValidate(tokens, grammar);
    const errors = issues.filter((i) => i.startsWith("Error"));
    const warnings = issues.filter((i) => i.startsWith("Warning"));
    // Missing: PLUS, MINUS
    expect(errors).toHaveLength(2);
    // Unused: UNUSED_A, UNUSED_B
    expect(warnings).toHaveLength(2);
  });

  it("should handle empty grammars", () => {
    /** Empty grammars produce no issues. */
    const tokens = parseTokenGrammar("");
    const grammar = parseParserGrammar("");
    const issues = crossValidate(tokens, grammar);
    expect(issues).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Indentation mode implicit tokens
// ---------------------------------------------------------------------------

describe("CrossValidateIndentation", () => {
  it("should not report INDENT/DEDENT/NEWLINE as missing in indent mode", () => {
    const tokens = parseTokenGrammar(`
mode: indentation
NAME = /[a-z]+/
COLON = ":"
`);
    const grammar = parseParserGrammar(`
file = { NAME COLON NEWLINE INDENT NAME NEWLINE DEDENT } ;
`);
    const issues = crossValidate(tokens, grammar);
    const errors = issues.filter((i) => i.startsWith("Error"));
    expect(errors).toEqual([]);
  });

  it("should report INDENT as missing when NOT in indent mode", () => {
    const tokens = parseTokenGrammar("NAME = /[a-z]+/");
    const grammar = parseParserGrammar("file = NAME INDENT NAME ;");
    const issues = crossValidate(tokens, grammar);
    const errors = issues.filter((i) => i.startsWith("Error"));
    expect(errors.some((e) => e.includes("INDENT"))).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Alias cross-validation
// ---------------------------------------------------------------------------

describe("CrossValidateAliases", () => {
  it("should not report aliased tokens as unused", () => {
    const tokens = parseTokenGrammar(`
STRING_DQ = /"[^"]*"/ -> STRING
`);
    const grammar = parseParserGrammar("expr = STRING ;");
    const issues = crossValidate(tokens, grammar);
    const warnings = issues.filter((i) => i.startsWith("Warning"));
    expect(warnings).toEqual([]);
  });

  it("should treat EOF as always implicitly available", () => {
    const tokens = parseTokenGrammar("NAME = /[a-z]+/");
    const grammar = parseParserGrammar("file = NAME EOF ;");
    const issues = crossValidate(tokens, grammar);
    const errors = issues.filter((i) => i.startsWith("Error"));
    expect(errors.some((e) => e.includes("EOF"))).toBe(false);
  });
});
