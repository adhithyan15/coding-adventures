/**
 * Tests for the grammar compiler (compiler.ts).
 *
 * The compiler transforms in-memory TokenGrammar and ParserGrammar objects
 * into TypeScript source code. Tests verify:
 *
 *   1. The generated code contains the expected header / DO NOT EDIT comment.
 *   2. The generated code is valid JavaScript (eval-able after stripping imports).
 *   3. Loading the generated code recreates an equivalent grammar object.
 *   4. All grammar features round-trip: aliases, skip patterns, error patterns,
 *      groups, keywords, mode, escapeMode, caseInsensitive.
 *   5. Edge cases: empty grammars, special chars in patterns, nested elements.
 *
 * Round-trip strategy
 * -------------------
 *
 *   original = parseTokenGrammar(source)
 *   code     = compileTokenGrammar(original)
 *   loaded   = evalTokenGrammar(code)  // strips imports, evals object literal
 *   expect(loaded.definitions).toEqual(original.definitions)
 *
 * We strip `import type ...` statements and `export const` annotations before
 * evaluating, since plain `new Function()` does not support ESM syntax. The
 * object literals themselves are pure JavaScript, so this approach works.
 */

import { describe, expect, it } from "vitest";
import {
  parseTokenGrammar,
  parseParserGrammar,
  compileTokenGrammar,
  compileParserGrammar,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// Eval helpers
// ---------------------------------------------------------------------------

/**
 * Strip ESM syntax from generated TypeScript and evaluate the TOKEN_GRAMMAR
 * object literal using the Function constructor.
 */
function evalTokenGrammar(code: string): any {
  const stripped = code
    .replace(/^\/\/.*\n/gm, "")          // remove single-line comments
    .replace(/import type [^;]+;\n/g, "") // remove import type statements
    .replace(/export const TOKEN_GRAMMAR: TokenGrammar = /, "const TOKEN_GRAMMAR = ")
    .replace(/export const PARSER_GRAMMAR: ParserGrammar = /, "const PARSER_GRAMMAR = ");
  // eslint-disable-next-line no-new-func
  return new Function(`${stripped}; return TOKEN_GRAMMAR;`)();
}

/**
 * Strip ESM syntax and evaluate the PARSER_GRAMMAR object literal.
 */
function evalParserGrammar(code: string): any {
  const stripped = code
    .replace(/^\/\/.*\n/gm, "")
    .replace(/import type [^;]+;\n/g, "")
    .replace(/export const TOKEN_GRAMMAR: TokenGrammar = /, "const TOKEN_GRAMMAR = ")
    .replace(/export const PARSER_GRAMMAR: ParserGrammar = /, "const PARSER_GRAMMAR = ");
  // eslint-disable-next-line no-new-func
  return new Function(`${stripped}; return PARSER_GRAMMAR;`)();
}

// ---------------------------------------------------------------------------
// compileTokenGrammar — output structure
// ---------------------------------------------------------------------------

describe("CompileTokenGrammarOutput", () => {
  it("includes DO NOT EDIT header", () => {
    const code = compileTokenGrammar(parseTokenGrammar(""));
    expect(code).toContain("DO NOT EDIT");
  });

  it("includes source file when given", () => {
    const code = compileTokenGrammar(parseTokenGrammar(""), "json.tokens");
    expect(code).toContain("json.tokens");
  });

  it("omits source line when empty", () => {
    const code = compileTokenGrammar(parseTokenGrammar(""), "");
    expect(code).not.toContain("// Source:");
  });

  it("imports TokenGrammar type", () => {
    const code = compileTokenGrammar(parseTokenGrammar(""));
    expect(code).toContain("@coding-adventures/grammar-tools");
  });

  it("exports TOKEN_GRAMMAR constant", () => {
    const code = compileTokenGrammar(parseTokenGrammar(""));
    expect(code).toContain("TOKEN_GRAMMAR");
  });
});

// ---------------------------------------------------------------------------
// compileTokenGrammar — round-trip tests
// ---------------------------------------------------------------------------

describe("CompileTokenGrammarRoundTrip", () => {
  it("empty grammar round-trips", () => {
    const original = parseTokenGrammar("");
    const loaded = evalTokenGrammar(compileTokenGrammar(original));
    expect(loaded.definitions).toEqual(original.definitions);
    expect(loaded.keywords).toEqual(original.keywords);
    expect(loaded.version).toBe(original.version);
    expect(loaded.caseInsensitive).toBe(original.caseInsensitive);
  });

  it("regex token round-trips", () => {
    const original = parseTokenGrammar("NUMBER = /[0-9]+/");
    const loaded = evalTokenGrammar(compileTokenGrammar(original));
    expect(loaded.definitions).toHaveLength(1);
    expect(loaded.definitions[0].name).toBe("NUMBER");
    expect(loaded.definitions[0].pattern).toBe("[0-9]+");
    expect(loaded.definitions[0].isRegex).toBe(true);
  });

  it("literal token round-trips", () => {
    const original = parseTokenGrammar('PLUS = "+"');
    const loaded = evalTokenGrammar(compileTokenGrammar(original));
    expect(loaded.definitions[0].name).toBe("PLUS");
    expect(loaded.definitions[0].pattern).toBe("+");
    expect(loaded.definitions[0].isRegex).toBe(false);
  });

  it("alias round-trips", () => {
    const original = parseTokenGrammar('STRING_DQ = /"[^"]*"/ -> STRING');
    const loaded = evalTokenGrammar(compileTokenGrammar(original));
    expect(loaded.definitions[0].alias).toBe("STRING");
  });

  it("keywords round-trip", () => {
    const source = "NAME = /[a-z]+/\nkeywords:\n  if\n  else\n  while\n";
    const original = parseTokenGrammar(source);
    const loaded = evalTokenGrammar(compileTokenGrammar(original));
    expect(loaded.keywords).toEqual(["if", "else", "while"]);
  });

  it("skip definitions round-trip", () => {
    const source = "NAME = /[a-z]+/\nskip:\n  WHITESPACE = /[ \\t]+/\n";
    const original = parseTokenGrammar(source);
    const loaded = evalTokenGrammar(compileTokenGrammar(original));
    expect(loaded.skipDefinitions).toHaveLength(1);
    expect(loaded.skipDefinitions[0].name).toBe("WHITESPACE");
  });

  it("mode round-trips", () => {
    const source = "mode: indentation\nNAME = /[a-z]+/";
    const original = parseTokenGrammar(source);
    const loaded = evalTokenGrammar(compileTokenGrammar(original));
    expect(loaded.mode).toBe("indentation");
  });

  it("escapeMode round-trips", () => {
    const source = 'escapes: none\nSTRING = /"[^"]*"/';
    const original = parseTokenGrammar(source);
    const loaded = evalTokenGrammar(compileTokenGrammar(original));
    expect(loaded.escapeMode).toBe("none");
  });

  it("version round-trips", () => {
    const source = "# @version 3\nNAME = /[a-z]+/";
    const original = parseTokenGrammar(source);
    const loaded = evalTokenGrammar(compileTokenGrammar(original));
    expect(loaded.version).toBe(3);
  });

  it("caseInsensitive round-trips", () => {
    const source = "# @case_insensitive true\nNAME = /[a-z]+/";
    const original = parseTokenGrammar(source);
    const loaded = evalTokenGrammar(compileTokenGrammar(original));
    expect(loaded.caseInsensitive).toBe(true);
  });

  it("pattern groups round-trip", () => {
    const source = "TEXT = /[^<]+/\ngroup tag:\n  ATTR = /[a-z]+/\n  EQ = \"=\"\n";
    const original = parseTokenGrammar(source);
    const loaded = evalTokenGrammar(compileTokenGrammar(original));
    expect(loaded.groups).toHaveProperty("tag");
    expect(loaded.groups["tag"].definitions).toHaveLength(2);
  });

  it("special regex characters round-trip", () => {
    const source = String.raw`STRING = /"([^"\\]|\\["\\\/bfnrt]|\\u[0-9a-fA-F]{4})*"/`;
    const original = parseTokenGrammar(source);
    const loaded = evalTokenGrammar(compileTokenGrammar(original));
    expect(loaded.definitions[0].pattern).toBe(original.definitions[0].pattern);
  });
});

// ---------------------------------------------------------------------------
// compileParserGrammar — output structure
// ---------------------------------------------------------------------------

describe("CompileParserGrammarOutput", () => {
  it("includes DO NOT EDIT header", () => {
    const code = compileParserGrammar(parseParserGrammar(""));
    expect(code).toContain("DO NOT EDIT");
  });

  it("exports PARSER_GRAMMAR constant", () => {
    const code = compileParserGrammar(parseParserGrammar(""));
    expect(code).toContain("PARSER_GRAMMAR");
  });

  it("imports ParserGrammar type", () => {
    const code = compileParserGrammar(parseParserGrammar(""));
    expect(code).toContain("@coding-adventures/grammar-tools");
  });
});

// ---------------------------------------------------------------------------
// compileParserGrammar — round-trip tests
// ---------------------------------------------------------------------------

describe("CompileParserGrammarRoundTrip", () => {
  it("empty grammar round-trips", () => {
    const original = parseParserGrammar("");
    const loaded = evalParserGrammar(compileParserGrammar(original));
    expect(loaded.version).toBe(0);
    expect(loaded.rules).toEqual([]);
  });

  it("token reference round-trips", () => {
    const original = parseParserGrammar("value = NUMBER ;");
    const loaded = evalParserGrammar(compileParserGrammar(original));
    expect(loaded.rules).toHaveLength(1);
    expect(loaded.rules[0].name).toBe("value");
    expect(loaded.rules[0].body.type).toBe("token_reference");
    expect(loaded.rules[0].body.name).toBe("NUMBER");
  });

  it("rule reference round-trips", () => {
    const original = parseParserGrammar("program = expr ;\nexpr = NUMBER ;");
    const loaded = evalParserGrammar(compileParserGrammar(original));
    expect(loaded.rules[0].body.type).toBe("rule_reference");
    expect(loaded.rules[0].body.name).toBe("expr");
  });

  it("alternation round-trips", () => {
    const original = parseParserGrammar("value = A | B | C ;");
    const loaded = evalParserGrammar(compileParserGrammar(original));
    expect(loaded.rules[0].body.type).toBe("alternation");
    expect(loaded.rules[0].body.choices).toHaveLength(3);
  });

  it("sequence round-trips", () => {
    const original = parseParserGrammar("pair = KEY COLON VALUE ;");
    const loaded = evalParserGrammar(compileParserGrammar(original));
    expect(loaded.rules[0].body.type).toBe("sequence");
  });

  it("repetition round-trips", () => {
    const original = parseParserGrammar("stmts = { stmt } ;");
    const loaded = evalParserGrammar(compileParserGrammar(original));
    expect(loaded.rules[0].body.type).toBe("repetition");
  });

  it("optional round-trips", () => {
    const original = parseParserGrammar("expr = NUMBER [ PLUS NUMBER ] ;");
    const loaded = evalParserGrammar(compileParserGrammar(original));
    expect(loaded.rules[0].body.type).toBe("sequence");
    expect(loaded.rules[0].body.elements[1].type).toBe("optional");
  });

  it("literal round-trips", () => {
    const original = parseParserGrammar('start = "hello" ;');
    const loaded = evalParserGrammar(compileParserGrammar(original));
    expect(loaded.rules[0].body.type).toBe("literal");
    expect(loaded.rules[0].body.value).toBe("hello");
  });

  it("group round-trips", () => {
    const original = parseParserGrammar("expr = ( A | B ) ;");
    const loaded = evalParserGrammar(compileParserGrammar(original));
    expect(loaded.rules[0].body.type).toBe("group");
  });

  it("version round-trips", () => {
    const original = parseParserGrammar("# @version 4\nvalue = NUMBER ;");
    const loaded = evalParserGrammar(compileParserGrammar(original));
    expect(loaded.version).toBe(4);
  });

  it("JSON grammar full round-trip", () => {
    const source = [
      "value    = object | array | STRING | NUMBER | TRUE | FALSE | NULL ;",
      "object   = LBRACE [ pair { COMMA pair } ] RBRACE ;",
      "pair     = STRING COLON value ;",
      "array    = LBRACKET [ value { COMMA value } ] RBRACKET ;",
    ].join("\n");
    const original = parseParserGrammar(source);
    const loaded = evalParserGrammar(compileParserGrammar(original, "json.grammar"));
    expect(loaded.rules).toHaveLength(4);
    expect(loaded.rules[0].name).toBe("value");
    expect(loaded.rules[3].name).toBe("array");
  });
});
