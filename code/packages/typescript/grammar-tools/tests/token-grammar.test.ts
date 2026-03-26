/**
 * Tests for the .tokens file parser and validator.
 *
 * These tests verify that parseTokenGrammar correctly reads the declarative
 * token definition format, and that validateTokenGrammar catches common
 * mistakes. Each test focuses on one aspect of the format: regex patterns,
 * literal patterns, comments, keywords, error handling, and validation.
 */

import { describe, expect, it } from "vitest";
import {
  TokenGrammarError,
  parseTokenGrammar,
  validateTokenGrammar,
  tokenNames,
  effectiveTokenNames,
} from "../src/token-grammar.js";

// ---------------------------------------------------------------------------
// Parsing: happy paths
// ---------------------------------------------------------------------------

describe("ParseMinimal", () => {
  it("should parse a single regex token", () => {
    /** A file with one regex-based token definition. */
    const source = "NUMBER = /[0-9]+/";
    const grammar = parseTokenGrammar(source);
    expect(grammar.definitions).toHaveLength(1);
    const defn = grammar.definitions[0];
    expect(defn.name).toBe("NUMBER");
    expect(defn.pattern).toBe("[0-9]+");
    expect(defn.isRegex).toBe(true);
    expect(defn.lineNumber).toBe(1);
  });

  it("should parse a single literal token", () => {
    /** A file with one literal-string token definition. */
    const source = 'PLUS = "+"';
    const grammar = parseTokenGrammar(source);
    expect(grammar.definitions).toHaveLength(1);
    const defn = grammar.definitions[0];
    expect(defn.name).toBe("PLUS");
    expect(defn.pattern).toBe("+");
    expect(defn.isRegex).toBe(false);
  });

  it("should parse multiple tokens in order", () => {
    /** Multiple token definitions are parsed in order. */
    const source = `NUMBER = /[0-9]+/
PLUS   = "+"
MINUS  = "-"
`;
    const grammar = parseTokenGrammar(source);
    expect(grammar.definitions).toHaveLength(3);
    expect(grammar.definitions.map((d) => d.name)).toEqual([
      "NUMBER",
      "PLUS",
      "MINUS",
    ]);
  });
});

describe("ParseKeywords", () => {
  it("should parse keywords section", () => {
    /** Keywords are parsed from indented lines after 'keywords:'. */
    const source = `NAME = /[a-zA-Z_][a-zA-Z0-9_]*/

keywords:
  if
  else
  while
`;
    const grammar = parseTokenGrammar(source);
    expect(grammar.keywords).toEqual(["if", "else", "while"]);
  });

  it("should parse keywords indented with tabs", () => {
    /** Keywords indented with tabs should also work. */
    const source = "NAME = /[a-z]+/\nkeywords:\n\tif\n\telse";
    const grammar = parseTokenGrammar(source);
    expect(grammar.keywords).toEqual(["if", "else"]);
  });

  it("should have empty keywords when no keywords section", () => {
    /** A file without keywords: section has an empty keywords list. */
    const source = "NUMBER = /[0-9]+/";
    const grammar = parseTokenGrammar(source);
    expect(grammar.keywords).toEqual([]);
  });
});

describe("ParseCommentsAndBlanks", () => {
  it("should ignore comments", () => {
    /** Lines starting with # are skipped. */
    const source = `# This is a comment
NUMBER = /[0-9]+/
# Another comment
PLUS   = "+"
`;
    const grammar = parseTokenGrammar(source);
    expect(grammar.definitions).toHaveLength(2);
  });

  it("should ignore blank lines", () => {
    /** Empty and whitespace-only lines are skipped. */
    const source = `
NUMBER = /[0-9]+/

PLUS   = "+"

`;
    const grammar = parseTokenGrammar(source);
    expect(grammar.definitions).toHaveLength(2);
  });

  it("should ignore comments in keywords section", () => {
    /** Comments inside the keywords section are skipped. */
    const source = `NAME = /[a-z]+/
keywords:
  # this is a comment
  if
  else
`;
    const grammar = parseTokenGrammar(source);
    expect(grammar.keywords).toEqual(["if", "else"]);
  });
});

describe("ParseRegexVsLiteral", () => {
  it("should identify regex patterns", () => {
    /** Regex patterns are delimited by /slashes/. */
    const grammar = parseTokenGrammar("NAME = /[a-zA-Z_][a-zA-Z0-9_]*/");
    expect(grammar.definitions[0].isRegex).toBe(true);
    expect(grammar.definitions[0].pattern).toBe("[a-zA-Z_][a-zA-Z0-9_]*");
  });

  it("should identify literal patterns", () => {
    /** Literal patterns are delimited by "quotes". */
    const grammar = parseTokenGrammar('EQUALS = "="');
    expect(grammar.definitions[0].isRegex).toBe(false);
    expect(grammar.definitions[0].pattern).toBe("=");
  });

  it("should handle multi-character literals", () => {
    /** Multi-character literals work correctly. */
    const grammar = parseTokenGrammar('EQUALS_EQUALS = "=="');
    expect(grammar.definitions[0].pattern).toBe("==");
  });
});

describe("tokenNames", () => {
  it("should return all token names", () => {
    const source = `NUMBER = /[0-9]+/
PLUS   = "+"
NAME   = /[a-z]+/
`;
    const grammar = parseTokenGrammar(source);
    expect(tokenNames(grammar)).toEqual(new Set(["NUMBER", "PLUS", "NAME"]));
  });

  it("should return empty set for empty grammar", () => {
    const grammar = parseTokenGrammar("");
    expect(tokenNames(grammar)).toEqual(new Set());
  });
});

// ---------------------------------------------------------------------------
// Parsing: error cases
// ---------------------------------------------------------------------------

describe("ParseErrors", () => {
  it("should allow duplicate token names (caught by validator)", () => {
    /** Duplicate names are not a parse error (caught by validator). */
    const source = `NUMBER = /[0-9]+/
NUMBER = /[0-9]+\\.?[0-9]*/
`;
    const grammar = parseTokenGrammar(source);
    expect(grammar.definitions).toHaveLength(2);
  });

  it("should throw on missing pattern", () => {
    /** A line with name and = but no pattern raises an error. */
    expect(() => parseTokenGrammar("NUMBER =")).toThrow(TokenGrammarError);
    expect(() => parseTokenGrammar("NUMBER =")).toThrow(/Missing pattern/);
  });

  it("should throw on malformed line without equals", () => {
    /** A line without = raises an error. */
    expect(() => parseTokenGrammar("NUMBER /[0-9]+/")).toThrow(
      TokenGrammarError
    );
    expect(() => parseTokenGrammar("NUMBER /[0-9]+/")).toThrow(
      /Expected token definition/
    );
  });

  it("should throw on invalid pattern delimiters", () => {
    /** A pattern that is neither /regex/ nor "literal" raises an error. */
    expect(() => parseTokenGrammar("NUMBER = [0-9]+")).toThrow(
      TokenGrammarError
    );
    expect(() => parseTokenGrammar("NUMBER = [0-9]+")).toThrow(
      /must be \/regex\/ or/
    );
  });

  it("should throw on empty regex pattern", () => {
    /** An empty regex // raises an error. */
    expect(() => parseTokenGrammar("NUMBER = //")).toThrow(TokenGrammarError);
    expect(() => parseTokenGrammar("NUMBER = //")).toThrow(/Empty regex/);
  });

  it("should throw on empty literal pattern", () => {
    /** An empty literal "" raises an error. */
    expect(() => parseTokenGrammar('EMPTY = ""')).toThrow(TokenGrammarError);
    expect(() => parseTokenGrammar('EMPTY = ""')).toThrow(/Empty literal/);
  });

  it("should include correct line number in errors", () => {
    /** Error messages include the correct line number. */
    const source = `# comment
NUMBER = /[0-9]+/
BADLINE
`;
    try {
      parseTokenGrammar(source);
      expect.fail("Expected TokenGrammarError");
    } catch (e) {
      expect(e).toBeInstanceOf(TokenGrammarError);
      expect((e as TokenGrammarError).lineNumber).toBe(3);
    }
  });
});

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

describe("Validation", () => {
  it("should produce no issues for a valid grammar", () => {
    /** A well-formed grammar produces no warnings. */
    const source = `NUMBER = /[0-9]+/
PLUS   = "+"
NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
`;
    const grammar = parseTokenGrammar(source);
    const issues = validateTokenGrammar(grammar);
    expect(issues).toEqual([]);
  });

  it("should flag duplicate token names", () => {
    /** Duplicate token names are flagged. */
    const source = `NUMBER = /[0-9]+/
NUMBER = /[0-9]+\\.?[0-9]*/
`;
    const grammar = parseTokenGrammar(source);
    const issues = validateTokenGrammar(grammar);
    expect(issues.some((i) => i.includes("Duplicate"))).toBe(true);
  });

  it("should flag invalid regex patterns", () => {
    /** An invalid regex pattern is flagged. */
    const source = "BAD = /[invalid/";
    const grammar = parseTokenGrammar(source);
    const issues = validateTokenGrammar(grammar);
    expect(issues.some((i) => i.includes("Invalid regex"))).toBe(true);
  });

  it("should flag non-uppercase token names", () => {
    /** Token names that aren't UPPER_CASE are flagged. */
    const source = "number = /[0-9]+/";
    const grammar = parseTokenGrammar(source);
    const issues = validateTokenGrammar(grammar);
    expect(issues.some((i) => i.includes("UPPER_CASE"))).toBe(true);
  });

  it("should flag mixed-case token names", () => {
    /** Mixed case names like 'Number' are flagged. */
    const source = "Number = /[0-9]+/";
    const grammar = parseTokenGrammar(source);
    const issues = validateTokenGrammar(grammar);
    expect(issues.some((i) => i.includes("UPPER_CASE"))).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Mode directive
// ---------------------------------------------------------------------------

describe("ParseModeDirective", () => {
  it("should parse mode: indentation", () => {
    const grammar = parseTokenGrammar("mode: indentation\nNAME = /[a-z]+/");
    expect(grammar.mode).toBe("indentation");
  });

  it("should have undefined mode when not specified", () => {
    const grammar = parseTokenGrammar("NAME = /[a-z]+/");
    expect(grammar.mode).toBeUndefined();
  });

  it("should throw on missing mode value", () => {
    expect(() => parseTokenGrammar("mode:")).toThrow(TokenGrammarError);
    expect(() => parseTokenGrammar("mode:")).toThrow(/Missing mode value/);
  });
});

// ---------------------------------------------------------------------------
// Skip section
// ---------------------------------------------------------------------------

describe("ParseSkipSection", () => {
  it("should parse skip definitions", () => {
    const source = `NAME = /[a-z]+/
skip:
  WHITESPACE = /[ \\t]+/
  COMMENT = /#[^\\n]*/`;
    const grammar = parseTokenGrammar(source);
    expect(grammar.skipDefinitions).toHaveLength(2);
    expect(grammar.skipDefinitions![0].name).toBe("WHITESPACE");
    expect(grammar.skipDefinitions![1].name).toBe("COMMENT");
  });

  it("should throw on skip definition without equals", () => {
    expect(() => parseTokenGrammar("skip:\n  BAD_PATTERN")).toThrow(
      TokenGrammarError,
    );
  });

  it("should throw on incomplete skip definition", () => {
    expect(() => parseTokenGrammar("skip:\n  BAD =")).toThrow(
      TokenGrammarError,
    );
  });
});

// ---------------------------------------------------------------------------
// Reserved keywords section
// ---------------------------------------------------------------------------

describe("ParseReservedSection", () => {
  it("should parse reserved keywords", () => {
    const source = `NAME = /[a-z]+/
reserved:
  class
  import`;
    const grammar = parseTokenGrammar(source);
    expect(grammar.reservedKeywords).toEqual(["class", "import"]);
  });
});

// ---------------------------------------------------------------------------
// Aliases
// ---------------------------------------------------------------------------

describe("ParseAlias", () => {
  it("should parse regex alias", () => {
    const source = `STRING_DQ = /"[^"]*"/ -> STRING`;
    const grammar = parseTokenGrammar(source);
    expect(grammar.definitions[0].alias).toBe("STRING");
  });

  it("should parse literal alias", () => {
    const source = `PLUS_SIGN = "+" -> PLUS`;
    const grammar = parseTokenGrammar(source);
    expect(grammar.definitions[0].alias).toBe("PLUS");
  });

  it("should throw on missing alias name", () => {
    expect(() => parseTokenGrammar(`FOO = /x/ ->`)).toThrow(TokenGrammarError);
    expect(() => parseTokenGrammar(`FOO = /x/ ->`)).toThrow(/Missing alias/);
  });

  it("should include aliases in tokenNames", () => {
    const source = `STRING_DQ = /"[^"]*"/ -> STRING`;
    const grammar = parseTokenGrammar(source);
    const names = tokenNames(grammar);
    expect(names.has("STRING_DQ")).toBe(true);
    expect(names.has("STRING")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Error cases for new syntax
// ---------------------------------------------------------------------------

describe("ParseNewSyntaxErrors", () => {
  it("should throw on unclosed regex", () => {
    expect(() => parseTokenGrammar("FOO = /unclosed")).toThrow(TokenGrammarError);
    expect(() => parseTokenGrammar("FOO = /unclosed")).toThrow(/Unclosed regex/);
  });

  it("should throw on unclosed literal", () => {
    expect(() => parseTokenGrammar(`FOO = "unclosed`)).toThrow(TokenGrammarError);
    expect(() => parseTokenGrammar(`FOO = "unclosed`)).toThrow(/Unclosed literal/);
  });
});

// ---------------------------------------------------------------------------
// Full example
// ---------------------------------------------------------------------------

describe("FullExample", () => {
  it("should parse a complete tokens file", () => {
    const source = `# Token definitions for a simple expression language

NAME        = /[a-zA-Z_][a-zA-Z0-9_]*/
NUMBER      = /[0-9]+/
STRING      = /"([^"\\\\]|\\\\.)*"/

EQUALS_EQUALS = "=="
EQUALS      = "="
PLUS        = "+"
MINUS       = "-"
STAR        = "*"
SLASH       = "/"
LPAREN      = "("
RPAREN      = ")"
COMMA       = ","
COLON       = ":"

# Keywords section
keywords:
  if
  else
  def
  return
  while
  for
  True
  False
  None
`;
    const grammar = parseTokenGrammar(source);
    expect(grammar.definitions).toHaveLength(13);
    expect(grammar.keywords).toEqual([
      "if",
      "else",
      "def",
      "return",
      "while",
      "for",
      "True",
      "False",
      "None",
    ]);
    const issues = validateTokenGrammar(grammar);
    expect(issues).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Starlark-like full example
// ---------------------------------------------------------------------------

describe("StarlarkTokens", () => {
  it("should parse a starlark-like tokens file", () => {
    const source = `
mode: indentation

NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
INT = /[0-9]+/
EQUALS = "="
PLUS = "+"
COLON = ":"
LPAREN = "("
RPAREN = ")"
COMMA = ","

keywords:
  def
  return
  if
  else
  for
  in
  pass

reserved:
  class
  import

skip:
  WHITESPACE = /[ \\t]+/
  COMMENT = /#[^\\n]*/
`;
    const grammar = parseTokenGrammar(source);
    expect(grammar.mode).toBe("indentation");
    expect(grammar.reservedKeywords).toEqual(["class", "import"]);
    expect(grammar.skipDefinitions).toHaveLength(2);
    expect(grammar.keywords).toHaveLength(7);
    const issues = validateTokenGrammar(grammar);
    expect(issues).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Pattern groups: happy paths
// ---------------------------------------------------------------------------

describe("PatternGroups", () => {
  it("should parse a basic group section", () => {
    /** A simple group section is parsed into a PatternGroup. */
    const source = `TEXT = /[^<]+/
TAG_OPEN = "<"

group tag:
  TAG_NAME = /[a-zA-Z]+/
  TAG_CLOSE = ">"
`;
    const grammar = parseTokenGrammar(source);

    // Default group patterns
    expect(grammar.definitions).toHaveLength(2);
    expect(grammar.definitions[0].name).toBe("TEXT");
    expect(grammar.definitions[1].name).toBe("TAG_OPEN");

    // Named group
    expect(grammar.groups).toBeDefined();
    expect(grammar.groups!["tag"]).toBeDefined();
    const group = grammar.groups!["tag"];
    expect(group.name).toBe("tag");
    expect(group.definitions).toHaveLength(2);
    expect(group.definitions[0].name).toBe("TAG_NAME");
    expect(group.definitions[1].name).toBe("TAG_CLOSE");
  });

  it("should parse multiple groups", () => {
    /** Multiple groups can be defined in the same file. */
    const source = `TEXT = /[^<]+/

group tag:
  TAG_NAME = /[a-zA-Z]+/

group cdata:
  CDATA_TEXT = /[^]]+/
  CDATA_END = "]]>"
`;
    const grammar = parseTokenGrammar(source);

    expect(Object.keys(grammar.groups!)).toHaveLength(2);
    expect(grammar.groups!["tag"]).toBeDefined();
    expect(grammar.groups!["cdata"]).toBeDefined();
    expect(grammar.groups!["tag"].definitions).toHaveLength(1);
    expect(grammar.groups!["cdata"].definitions).toHaveLength(2);
  });

  it("should parse group definitions with aliases", () => {
    /** Definitions inside groups support -> ALIAS. */
    const source = `TEXT = /[^<]+/

group tag:
  ATTR_VALUE_DQ = /"[^"]*"/ -> ATTR_VALUE
  ATTR_VALUE_SQ = /'[^']*'/ -> ATTR_VALUE
`;
    const grammar = parseTokenGrammar(source);

    const group = grammar.groups!["tag"];
    expect(group.definitions[0].name).toBe("ATTR_VALUE_DQ");
    expect(group.definitions[0].alias).toBe("ATTR_VALUE");
    expect(group.definitions[1].name).toBe("ATTR_VALUE_SQ");
    expect(group.definitions[1].alias).toBe("ATTR_VALUE");
  });

  it("should parse groups with mixed regex and literal patterns", () => {
    /** Groups support both regex and literal patterns. */
    const source = `TEXT = /[^<]+/

group tag:
  EQUALS = "="
  TAG_NAME = /[a-zA-Z]+/
`;
    const grammar = parseTokenGrammar(source);

    const group = grammar.groups!["tag"];
    expect(group.definitions[0].isRegex).toBe(false);
    expect(group.definitions[0].pattern).toBe("=");
    expect(group.definitions[1].isRegex).toBe(true);
  });

  it("should have no groups for files without group sections", () => {
    /** Files without groups have undefined groups (backward compat). */
    const source = `NUMBER = /[0-9]+/
PLUS = "+"
`;
    const grammar = parseTokenGrammar(source);

    expect(grammar.groups).toBeUndefined();
    expect(grammar.definitions).toHaveLength(2);
  });

  it("should coexist with skip and keywords sections", () => {
    /** skip: and group: sections coexist correctly. */
    const source = `skip:
  WS = /[ \\t]+/

TEXT = /[^<]+/

group tag:
  TAG_NAME = /[a-zA-Z]+/
`;
    const grammar = parseTokenGrammar(source);

    expect(grammar.skipDefinitions).toHaveLength(1);
    expect(grammar.definitions).toHaveLength(1);
    expect(Object.keys(grammar.groups!)).toHaveLength(1);
  });

  it("should include group names in tokenNames()", () => {
    /** tokenNames() includes names from all groups. */
    const source = `TEXT = /[^<]+/

group tag:
  TAG_NAME = /[a-zA-Z]+/
  ATTR_DQ = /"[^"]*"/ -> ATTR_VALUE
`;
    const grammar = parseTokenGrammar(source);

    const names = tokenNames(grammar);
    expect(names.has("TEXT")).toBe(true);
    expect(names.has("TAG_NAME")).toBe(true);
    expect(names.has("ATTR_DQ")).toBe(true);
    expect(names.has("ATTR_VALUE")).toBe(true);
  });

  it("should include group aliases in effectiveTokenNames()", () => {
    /** effectiveTokenNames() includes aliased names from groups. */
    const source = `TEXT = /[^<]+/

group tag:
  ATTR_DQ = /"[^"]*"/ -> ATTR_VALUE
`;
    const grammar = parseTokenGrammar(source);

    const names = effectiveTokenNames(grammar);
    expect(names.has("TEXT")).toBe(true);
    expect(names.has("ATTR_VALUE")).toBe(true);
    // alias replaces name in effective names
    expect(names.has("ATTR_DQ")).toBe(false);
  });

  it("should validate group definitions (bad regex)", () => {
    /** Definitions in groups are validated (e.g., bad regex). */
    const grammar = {
      definitions: [],
      keywords: [],
      version: 0,
      caseInsensitive: false,
      groups: {
        tag: {
          name: "tag",
          definitions: [
            {
              name: "BAD",
              pattern: "[invalid",
              isRegex: true,
              lineNumber: 5,
            },
          ],
        },
      },
    };
    const issues = validateTokenGrammar(grammar);
    expect(issues.some((i) => i.includes("Invalid regex"))).toBe(true);
  });

  it("should warn on empty groups", () => {
    /** An empty group produces a validation warning. */
    const grammar = {
      definitions: [],
      keywords: [],
      version: 0,
      caseInsensitive: false,
      groups: {
        empty: {
          name: "empty",
          definitions: [],
        },
      },
    };
    const issues = validateTokenGrammar(grammar);
    expect(issues.some((i) => i.includes("Empty pattern group"))).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Magic comments
// ---------------------------------------------------------------------------

describe("MagicComments", () => {
  it("should set version from # @version magic comment", () => {
    /**
     * `# @version 1` sets the version field to the integer 1.
     *
     * Version numbers let tooling refuse to process grammar files
     * written for a different schema generation than the tool expects.
     */
    const source = "# @version 1\nNUMBER = /[0-9]+/";
    const grammar = parseTokenGrammar(source);
    expect(grammar.version).toBe(1);
  });

  it("should default version to 0 when no magic comment", () => {
    /**
     * When no `# @version` comment is present the field is 0.
     *
     * 0 is the sentinel for "unversioned / latest": tools that do not
     * yet understand versioning can safely accept these files.
     */
    const source = "NUMBER = /[0-9]+/";
    const grammar = parseTokenGrammar(source);
    expect(grammar.version).toBe(0);
  });

  it("should set caseInsensitive = true from # @case_insensitive true", () => {
    /**
     * `# @case_insensitive true` enables case-insensitive matching.
     *
     * Useful for languages like SQL where keywords such as SELECT and
     * select are interchangeable.
     */
    const source = "# @case_insensitive true\nNAME = /[a-z]+/";
    const grammar = parseTokenGrammar(source);
    expect(grammar.caseInsensitive).toBe(true);
  });

  it("should set caseInsensitive = false from # @case_insensitive false", () => {
    /**
     * Explicitly writing `# @case_insensitive false` leaves the field
     * at its default value — but the comment is still valid and parsed.
     */
    const source = "# @case_insensitive false\nNAME = /[a-z]+/";
    const grammar = parseTokenGrammar(source);
    expect(grammar.caseInsensitive).toBe(false);
  });

  it("should default caseInsensitive to false when no magic comment", () => {
    /**
     * The overwhelming majority of languages are case-sensitive, so
     * false is the correct default. No opt-out comment is needed.
     */
    const source = "NAME = /[a-z]+/";
    const grammar = parseTokenGrammar(source);
    expect(grammar.caseInsensitive).toBe(false);
  });

  it("should silently ignore unknown magic comment keys", () => {
    /**
     * Forward-compatibility: keys we don't recognise are discarded.
     *
     * This means a grammar written for a future version of the toolchain
     * can still be parsed by an older version without errors.
     */
    const source = "# @unknown_key some_value\nNUMBER = /[0-9]+/";
    // Should not throw and should still parse the token definition.
    const grammar = parseTokenGrammar(source);
    expect(grammar.definitions).toHaveLength(1);
    expect(grammar.version).toBe(0);
    expect(grammar.caseInsensitive).toBe(false);
  });

  it("should parse both magic comments together", () => {
    /**
     * Multiple magic comments can coexist in the same file.
     * Each is processed independently.
     */
    const source = [
      "# @version 3",
      "# @case_insensitive true",
      "NAME = /[a-z]+/",
    ].join("\n");
    const grammar = parseTokenGrammar(source);
    expect(grammar.version).toBe(3);
    expect(grammar.caseInsensitive).toBe(true);
  });

  it("should handle magic comments with extra whitespace", () => {
    /**
     * The spec says the regex is `/^#\s*@(\w+)\s*(.*)/`, so any amount
     * of whitespace between `#` and `@`, or between the key and value,
     * is accepted.
     */
    const source = "#   @version   42\nNUMBER = /[0-9]+/";
    const grammar = parseTokenGrammar(source);
    expect(grammar.version).toBe(42);
  });

  it("should parse a magic comment that appears after token definitions", () => {
    /**
     * Magic comments are scanned before the main parse loop, so their
     * position in the file does not matter.
     */
    const source = "NUMBER = /[0-9]+/\n# @version 7\nPLUS = \"+\"";
    const grammar = parseTokenGrammar(source);
    expect(grammar.version).toBe(7);
  });
});

// ---------------------------------------------------------------------------
// Pattern groups: error cases
// ---------------------------------------------------------------------------

describe("PatternGroupErrors", () => {
  it("should throw on missing group name", () => {
    /** 'group :' with no name raises an error. */
    const source = "TEXT = /abc/\ngroup :\n  FOO = /x/\n";
    expect(() => parseTokenGrammar(source)).toThrow(TokenGrammarError);
    expect(() => parseTokenGrammar(source)).toThrow(/Missing group name/);
  });

  it("should throw on uppercase group name", () => {
    /** Uppercase group names are rejected. */
    const source = "TEXT = /abc/\ngroup Tag:\n  FOO = /x/\n";
    expect(() => parseTokenGrammar(source)).toThrow(TokenGrammarError);
    expect(() => parseTokenGrammar(source)).toThrow(/Invalid group name/);
  });

  it("should throw on group name starting with digit", () => {
    /** Group names starting with a digit are rejected. */
    const source = "TEXT = /abc/\ngroup 1tag:\n  FOO = /x/\n";
    expect(() => parseTokenGrammar(source)).toThrow(TokenGrammarError);
    expect(() => parseTokenGrammar(source)).toThrow(/Invalid group name/);
  });

  it("should throw on reserved group name 'default'", () => {
    /** 'group default:' is rejected as reserved. */
    const source = "TEXT = /abc/\ngroup default:\n  FOO = /x/\n";
    expect(() => parseTokenGrammar(source)).toThrow(TokenGrammarError);
    expect(() => parseTokenGrammar(source)).toThrow(/Reserved group name/);
  });

  it("should throw on reserved group name 'skip'", () => {
    /** 'group skip:' is rejected as reserved. */
    const source = "TEXT = /abc/\ngroup skip:\n  FOO = /x/\n";
    expect(() => parseTokenGrammar(source)).toThrow(TokenGrammarError);
    expect(() => parseTokenGrammar(source)).toThrow(/Reserved group name/);
  });

  it("should throw on reserved group name 'keywords'", () => {
    /** 'group keywords:' is rejected as reserved. */
    const source = "TEXT = /abc/\ngroup keywords:\n  FOO = /x/\n";
    expect(() => parseTokenGrammar(source)).toThrow(TokenGrammarError);
    expect(() => parseTokenGrammar(source)).toThrow(/Reserved group name/);
  });

  it("should throw on duplicate group name", () => {
    /** Two groups with the same name raises an error. */
    const source = `TEXT = /abc/
group tag:
  FOO = /x/
group tag:
  BAR = /y/
`;
    expect(() => parseTokenGrammar(source)).toThrow(TokenGrammarError);
    expect(() => parseTokenGrammar(source)).toThrow(/Duplicate group name/);
  });

  it("should throw on bad definition in group", () => {
    /** Invalid definition inside a group raises an error. */
    const source = "TEXT = /abc/\ngroup tag:\n  not a definition\n";
    expect(() => parseTokenGrammar(source)).toThrow(TokenGrammarError);
    expect(() => parseTokenGrammar(source)).toThrow(
      /Expected token definition/,
    );
  });

  it("should throw on incomplete definition in group", () => {
    /** Missing pattern in group definition raises an error. */
    const source = "TEXT = /abc/\ngroup tag:\n  FOO = \n";
    expect(() => parseTokenGrammar(source)).toThrow(TokenGrammarError);
    expect(() => parseTokenGrammar(source)).toThrow(/Incomplete definition/);
  });
});
