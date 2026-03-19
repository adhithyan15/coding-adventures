/**
 * Tests for the .grammar file parser and validator.
 *
 * These tests verify that parseParserGrammar correctly reads EBNF notation
 * and builds the expected AST, and that validateParserGrammar catches
 * semantic issues like undefined references and unreachable rules.
 */

import { describe, expect, it } from "vitest";
import {
  ParserGrammarError,
  parseParserGrammar,
  validateParserGrammar,
  ruleNames,
  grammarTokenReferences,
  grammarRuleReferences,
  type GrammarElement,
} from "../src/parser-grammar.js";

// ---------------------------------------------------------------------------
// Parsing: happy paths
// ---------------------------------------------------------------------------

describe("ParseMinimal", () => {
  it("should parse a single rule with a token reference", () => {
    /** A grammar with one rule referencing a single token. */
    const grammar = parseParserGrammar("program = NUMBER ;");
    expect(grammar.rules).toHaveLength(1);
    const rule = grammar.rules[0];
    expect(rule.name).toBe("program");
    expect(rule.body.type).toBe("token_reference");
    if (rule.body.type === "token_reference") {
      expect(rule.body.name).toBe("NUMBER");
    }
  });

  it("should parse a rule referencing another rule", () => {
    /** A rule referencing another rule (lowercase name). */
    const grammar = parseParserGrammar(
      "program = expression ;\nexpression = NUMBER ;"
    );
    expect(grammar.rules).toHaveLength(2);
    expect(grammar.rules[0].body.type).toBe("rule_reference");
    if (grammar.rules[0].body.type === "rule_reference") {
      expect(grammar.rules[0].body.name).toBe("expression");
    }
  });

  it("should parse a sequence", () => {
    /** Multiple elements in a row form a Sequence. */
    const grammar = parseParserGrammar("assignment = NAME EQUALS NUMBER ;");
    const rule = grammar.rules[0];
    expect(rule.body.type).toBe("sequence");
    if (rule.body.type === "sequence") {
      expect(rule.body.elements).toHaveLength(3);
      expect(rule.body.elements[0]).toEqual({
        type: "token_reference",
        name: "NAME",
      });
      expect(rule.body.elements[1]).toEqual({
        type: "token_reference",
        name: "EQUALS",
      });
      expect(rule.body.elements[2]).toEqual({
        type: "token_reference",
        name: "NUMBER",
      });
    }
  });
});

describe("ParseAlternation", () => {
  it("should parse a simple alternation", () => {
    /** Two alternatives separated by |. */
    const grammar = parseParserGrammar("value = NUMBER | NAME ;");
    const rule = grammar.rules[0];
    expect(rule.body.type).toBe("alternation");
    if (rule.body.type === "alternation") {
      expect(rule.body.choices).toHaveLength(2);
      expect(rule.body.choices[0]).toEqual({
        type: "token_reference",
        name: "NUMBER",
      });
      expect(rule.body.choices[1]).toEqual({
        type: "token_reference",
        name: "NAME",
      });
    }
  });

  it("should parse three alternatives", () => {
    /** Three alternatives. */
    const grammar = parseParserGrammar("value = NUMBER | NAME | STRING ;");
    const rule = grammar.rules[0];
    expect(rule.body.type).toBe("alternation");
    if (rule.body.type === "alternation") {
      expect(rule.body.choices).toHaveLength(3);
    }
  });
});

describe("ParseRepetition", () => {
  it("should parse a simple repetition", () => {
    /** { statement } becomes Repetition with a rule reference inside. */
    const grammar = parseParserGrammar("program = { statement } ;");
    const rule = grammar.rules[0];
    expect(rule.body.type).toBe("repetition");
    if (rule.body.type === "repetition") {
      expect(rule.body.element).toEqual({
        type: "rule_reference",
        name: "statement",
      });
    }
  });

  it("should parse repetition in a sequence", () => {
    /** Repetition used as part of a sequence. */
    const grammar = parseParserGrammar("expression = term { PLUS term } ;");
    const rule = grammar.rules[0];
    expect(rule.body.type).toBe("sequence");
    if (rule.body.type === "sequence") {
      expect(rule.body.elements).toHaveLength(2);
      expect(rule.body.elements[1].type).toBe("repetition");
    }
  });
});

describe("ParseOptional", () => {
  it("should parse optional with multiple elements", () => {
    /** [ ELSE block ] becomes Optional(Sequence(...)). */
    const grammar = parseParserGrammar(
      "if_stmt = IF expression [ ELSE block ] ;"
    );
    const rule = grammar.rules[0];
    expect(rule.body.type).toBe("sequence");
    if (rule.body.type === "sequence") {
      // The optional is the last element in the sequence.
      const opt = rule.body.elements[2];
      expect(opt.type).toBe("optional");
    }
  });

  it("should parse optional with a single element", () => {
    /** [ SEMICOLON ] with a single element. */
    const grammar = parseParserGrammar("stmt = expression [ SEMICOLON ] ;");
    const rule = grammar.rules[0];
    expect(rule.body.type).toBe("sequence");
    if (rule.body.type === "sequence") {
      expect(rule.body.elements[1].type).toBe("optional");
      if (rule.body.elements[1].type === "optional") {
        expect(rule.body.elements[1].element).toEqual({
          type: "token_reference",
          name: "SEMICOLON",
        });
      }
    }
  });
});

describe("ParseGrouping", () => {
  it("should parse a grouped alternation", () => {
    /** ( PLUS | MINUS ) groups an alternation. */
    const grammar = parseParserGrammar(
      "expression = term { ( PLUS | MINUS ) term } ;"
    );
    const rule = grammar.rules[0];
    expect(rule.body.type).toBe("sequence");
    if (rule.body.type === "sequence") {
      const rep = rule.body.elements[1];
      expect(rep.type).toBe("repetition");
      if (rep.type === "repetition") {
        // Inside the repetition: sequence of (group, term).
        const inner = rep.element;
        expect(inner.type).toBe("sequence");
        if (inner.type === "sequence") {
          const grp = inner.elements[0];
          expect(grp.type).toBe("group");
          if (grp.type === "group") {
            expect(grp.element.type).toBe("alternation");
          }
        }
      }
    }
  });

  it("should parse a simple group", () => {
    /** ( expression ) is a Group wrapping a RuleReference. */
    const grammar = parseParserGrammar("factor = ( expression ) ;");
    const rule = grammar.rules[0];
    expect(rule.body.type).toBe("group");
    if (rule.body.type === "group") {
      expect(rule.body.element).toEqual({
        type: "rule_reference",
        name: "expression",
      });
    }
  });
});

describe("ParseLiteral", () => {
  it("should parse a literal in a rule", () => {
    /** A quoted string becomes a Literal node. */
    const grammar = parseParserGrammar('stmt = "return" expression ;');
    const rule = grammar.rules[0];
    expect(rule.body.type).toBe("sequence");
    if (rule.body.type === "sequence") {
      expect(rule.body.elements[0].type).toBe("literal");
      if (rule.body.elements[0].type === "literal") {
        expect(rule.body.elements[0].value).toBe("return");
      }
    }
  });
});

describe("ParseRecursive", () => {
  it("should parse direct recursion", () => {
    /** A rule that references itself. */
    const grammar = parseParserGrammar(
      "expression = NUMBER | expression PLUS expression ;"
    );
    const rule = grammar.rules[0];
    expect(rule.body.type).toBe("alternation");
  });

  it("should parse mutual recursion", () => {
    /** Rules that reference each other. */
    const source = `
expression = term { PLUS term } ;
term = factor { STAR factor } ;
factor = NUMBER | LPAREN expression RPAREN ;
`;
    const grammar = parseParserGrammar(source);
    expect(grammar.rules).toHaveLength(3);
  });
});

describe("ParseCommentsAndBlanks", () => {
  it("should ignore comments", () => {
    const source = `# This is a comment
program = { statement } ;
# Another comment
statement = NUMBER ;
`;
    const grammar = parseParserGrammar(source);
    expect(grammar.rules).toHaveLength(2);
  });

  it("should ignore blank lines", () => {
    const source = `
program = { statement } ;

statement = NUMBER ;

`;
    const grammar = parseParserGrammar(source);
    expect(grammar.rules).toHaveLength(2);
  });
});

// ---------------------------------------------------------------------------
// Parsing: error cases
// ---------------------------------------------------------------------------

describe("ParseErrors", () => {
  it("should throw on missing semicolon", () => {
    /** A rule without a trailing ; raises an error. */
    expect(() => parseParserGrammar("program = NUMBER")).toThrow(
      ParserGrammarError
    );
    expect(() => parseParserGrammar("program = NUMBER")).toThrow(
      /Expected SEMI/
    );
  });

  it("should throw on unexpected character", () => {
    /** An unexpected character raises an error. */
    expect(() => parseParserGrammar("program = NUMBER @ ;")).toThrow(
      ParserGrammarError
    );
    expect(() => parseParserGrammar("program = NUMBER @ ;")).toThrow(
      /Unexpected character/
    );
  });

  it("should throw on unterminated string", () => {
    /** A string literal without closing quote raises an error. */
    expect(() => parseParserGrammar('program = "hello ;')).toThrow(
      ParserGrammarError
    );
    expect(() => parseParserGrammar('program = "hello ;')).toThrow(
      /Unterminated/
    );
  });

  it("should throw on unmatched brace", () => {
    /** An unmatched { raises an error. */
    expect(() => parseParserGrammar("program = { statement ;")).toThrow(
      ParserGrammarError
    );
  });

  it("should include correct line number in errors", () => {
    /** Errors include the correct line number. */
    const source = `program = { statement } ;
bad_rule = ;
`;
    try {
      parseParserGrammar(source);
      expect.fail("Expected ParserGrammarError");
    } catch (e) {
      expect(e).toBeInstanceOf(ParserGrammarError);
      expect((e as ParserGrammarError).lineNumber).toBe(2);
    }
  });
});

// ---------------------------------------------------------------------------
// Query methods
// ---------------------------------------------------------------------------

describe("QueryMethods", () => {
  const source = `
expression = term { ( PLUS | MINUS ) term } ;
term       = factor { ( STAR | SLASH ) factor } ;
factor     = NUMBER | NAME | LPAREN expression RPAREN ;
`;

  it("should return all rule names", () => {
    const grammar = parseParserGrammar(source);
    expect(ruleNames(grammar)).toEqual(
      new Set(["expression", "term", "factor"])
    );
  });

  it("should return all token references", () => {
    const grammar = parseParserGrammar(source);
    expect(grammarTokenReferences(grammar)).toEqual(
      new Set([
        "PLUS",
        "MINUS",
        "STAR",
        "SLASH",
        "NUMBER",
        "NAME",
        "LPAREN",
        "RPAREN",
      ])
    );
  });

  it("should return all rule references", () => {
    const grammar = parseParserGrammar(source);
    expect(grammarRuleReferences(grammar)).toEqual(
      new Set(["term", "factor", "expression"])
    );
  });
});

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

describe("Validation", () => {
  it("should produce no issues for a valid grammar", () => {
    /** A well-formed grammar produces no warnings. */
    const source = `
program    = { statement } ;
statement  = expression NEWLINE ;
expression = NUMBER ;
`;
    const grammar = parseParserGrammar(source);
    const issues = validateParserGrammar(
      grammar,
      new Set(["NUMBER", "NEWLINE"])
    );
    expect(issues).toEqual([]);
  });

  it("should flag undefined rule references", () => {
    /** Referencing a rule that does not exist is flagged. */
    const source = "program = undefined_rule ;";
    const grammar = parseParserGrammar(source);
    const issues = validateParserGrammar(grammar);
    expect(issues.some((i) => i.includes("Undefined rule"))).toBe(true);
    expect(issues.some((i) => i.includes("undefined_rule"))).toBe(true);
  });

  it("should flag undefined token references", () => {
    /** Referencing a token not in tokenNames is flagged. */
    const source = "program = MISSING_TOKEN ;";
    const grammar = parseParserGrammar(source);
    const issues = validateParserGrammar(grammar, new Set(["NUMBER"]));
    expect(issues.some((i) => i.includes("Undefined token"))).toBe(true);
    expect(issues.some((i) => i.includes("MISSING_TOKEN"))).toBe(true);
  });

  it("should not check tokens without tokenNames", () => {
    /** Without tokenNames, token references are not checked. */
    const source = "program = ANYTHING ;";
    const grammar = parseParserGrammar(source);
    const issues = validateParserGrammar(grammar, null);
    expect(issues.some((i) => i.includes("Undefined token"))).toBe(false);
  });

  it("should flag duplicate rule names", () => {
    /** Duplicate rule names are flagged. */
    const source = `
program = NUMBER ;
program = NAME ;
`;
    const grammar = parseParserGrammar(source);
    const issues = validateParserGrammar(grammar);
    expect(issues.some((i) => i.includes("Duplicate"))).toBe(true);
  });

  it("should flag non-lowercase rule names", () => {
    /** Rule names that aren't lowercase are flagged. */
    const source = "Program = NUMBER ;";
    const grammar = parseParserGrammar(source);
    const issues = validateParserGrammar(grammar);
    expect(issues.some((i) => i.includes("lowercase"))).toBe(true);
  });

  it("should flag unreachable rules", () => {
    /** A rule defined but never referenced is flagged as unreachable. */
    const source = `
program = NUMBER ;
orphan  = NAME ;
`;
    const grammar = parseParserGrammar(source);
    const issues = validateParserGrammar(grammar);
    expect(issues.some((i) => i.includes("unreachable"))).toBe(true);
    expect(issues.some((i) => i.includes("orphan"))).toBe(true);
  });

  it("should not flag the start rule as unreachable", () => {
    /** The first rule (start symbol) is never flagged as unreachable. */
    const source = "program = NUMBER ;";
    const grammar = parseParserGrammar(source);
    const issues = validateParserGrammar(grammar);
    expect(issues.some((i) => i.includes("unreachable"))).toBe(false);
  });

  it("should not flag referenced rules as unreachable", () => {
    /** A rule referenced by another rule is not unreachable. */
    const source = `
program = expression ;
expression = NUMBER ;
`;
    const grammar = parseParserGrammar(source);
    const issues = validateParserGrammar(grammar);
    expect(issues.some((i) => i.includes("unreachable"))).toBe(false);
  });
});
