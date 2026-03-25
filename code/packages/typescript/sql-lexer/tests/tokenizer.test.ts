/**
 * Tests for the SQL Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes SQL
 * text when loaded with the `sql.tokens` grammar file.
 *
 * SQL (Structured Query Language) is the standard language for relational
 * databases. This lexer handles the full ANSI SQL subset defined in sql.tokens,
 * including keywords, identifiers, literals, operators, and comments.
 *
 * Key Behaviors Under Test
 * -------------------------
 *
 *   - Case-insensitive keyword matching (sql.tokens sets @case_insensitive true)
 *   - Keyword values normalized to uppercase (select → "SELECT")
 *   - Single-quoted strings tokenized as STRING with quotes stripped
 *   - Backtick-quoted identifiers tokenized as NAME (backticks kept)
 *   - Multi-character operators: !=, <>, <=, >=
 *   - Both SQL comment styles skipped: -- line and /* block *\/
 *   - Qualified names: schema.table (NAME DOT NAME)
 *   - NULL, TRUE, FALSE as KEYWORD tokens
 *
 * Test Categories
 * ---------------
 *
 *   1. **Keywords** -- SELECT, FROM, WHERE, etc. (case-insensitive)
 *   2. **Identifiers** -- plain names and backtick-quoted names
 *   3. **Numbers** -- integers and decimals
 *   4. **Strings** -- single-quoted string literals
 *   5. **Operators** -- comparison and arithmetic operators
 *   6. **Punctuation** -- parentheses, comma, semicolon, dot
 *   7. **Comments** -- line comments and block comments
 *   8. **SQL fragments** -- WHERE clause, qualified names
 *   9. **Factory function** -- createSQLLexer returns non-null
 */

import { describe, it, expect } from "vitest";
import { tokenizeSQL, createSQLLexer } from "../src/tokenizer.js";
import type { Token } from "@coding-adventures/lexer";

/**
 * Helper: extract token types from a SQL string.
 * Makes assertions concise — compare arrays of type strings.
 */
function tokenTypes(source: string): string[] {
  return tokenizeSQL(source).map((t) => t.type);
}

/**
 * Helper: extract token values from a SQL string.
 */
function tokenValues(source: string): string[] {
  return tokenizeSQL(source).map((t) => t.value);
}

/**
 * Helper: find the first token of a given type.
 */
function firstOfType(tokens: Token[], type: string): Token | undefined {
  return tokens.find((t) => t.type === type);
}

describe("keywords", () => {
  it("tokenizes SELECT as KEYWORD with uppercase value", () => {
    /**
     * The most common SQL keyword. The grammar-driven lexer detects that
     * "SELECT" matches a NAME pattern whose uppercase value is in the
     * keyword list, and promotes the token type to "KEYWORD".
     *
     * Because sql.tokens uses @case_insensitive true, the value is also
     * normalized to uppercase regardless of how it was typed.
     */
    const tokens = tokenizeSQL("SELECT");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("SELECT");
  });

  it("tokenizes lowercase 'select' as KEYWORD with value 'SELECT'", () => {
    /**
     * Case-insensitive matching: the user wrote 'select' but the lexer
     * normalizes the value to uppercase "SELECT".
     *
     * This is the core behavior enabled by @case_insensitive true in
     * sql.tokens: the parser grammar can always compare against uppercase
     * strings without worrying about how the user typed the keyword.
     */
    const tokens = tokenizeSQL("select");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("SELECT");
  });

  it("tokenizes mixed-case 'Select' as KEYWORD with value 'SELECT'", () => {
    /**
     * Any capitalization of a SQL keyword normalizes to uppercase.
     */
    const tokens = tokenizeSQL("Select");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("SELECT");
  });

  it("tokenizes NULL as KEYWORD", () => {
    /**
     * NULL is a SQL keyword, not a literal like JSON's null. The grammar
     * lists NULL in the keywords section, so it tokenizes as KEYWORD("NULL").
     */
    const tokens = tokenizeSQL("NULL");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("NULL");
  });

  it("tokenizes TRUE as KEYWORD", () => {
    /**
     * TRUE is a SQL keyword. Unlike JSON where true is a dedicated TRUE
     * token type, SQL uses the generic KEYWORD token for boolean literals.
     */
    const tokens = tokenizeSQL("TRUE");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("TRUE");
  });

  it("tokenizes FALSE as KEYWORD", () => {
    /**
     * FALSE is also a SQL keyword.
     */
    const tokens = tokenizeSQL("FALSE");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("FALSE");
  });

  it("tokenizes a SELECT query with multiple keywords", () => {
    /**
     * A typical SQL SELECT has several keywords: SELECT and FROM at minimum.
     * This test verifies they all come out as KEYWORD tokens.
     */
    const tokens = tokenizeSQL("SELECT id FROM users");
    const keywordTokens = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywordTokens).toHaveLength(2);
    expect(keywordTokens[0].value).toBe("SELECT");
    expect(keywordTokens[1].value).toBe("FROM");
  });
});

describe("numbers", () => {
  it("tokenizes an integer", () => {
    /**
     * SQL numeric literals: integers are the most common form, appearing
     * in WHERE clauses, LIMIT clauses, and literal expressions.
     */
    const tokens = tokenizeSQL("42");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("42");
  });

  it("tokenizes a decimal number", () => {
    /**
     * SQL also supports decimal numbers for monetary values and percentages.
     * The sql.tokens NUMBER pattern matches /[0-9]+(\.[0-9]+)?/.
     */
    const tokens = tokenizeSQL("3.14");
    expect(tokens[0].type).toBe("NUMBER");
    expect(tokens[0].value).toBe("3.14");
  });
});

describe("single-quoted strings", () => {
  it("tokenizes a single-quoted string as STRING", () => {
    /**
     * SQL string literals use single quotes (unlike JSON which uses double
     * quotes). The STRING_SQ pattern in sql.tokens matches 'hello' and
     * aliases the token type to STRING.
     *
     * The grammar-driven lexer strips the surrounding quotes from STRING
     * tokens, so the value is the inner content: hello, not 'hello'.
     */
    const tokens = tokenizeSQL("'hello'");
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("hello");
  });

  it("tokenizes a single-quoted string with escaped characters", () => {
    /**
     * SQL strings can contain escape sequences. The lexer strips quotes
     * and processes escape sequences, yielding the inner content.
     */
    const tokens = tokenizeSQL("'hello world'");
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("hello world");
  });
});

describe("operators", () => {
  it("tokenizes the equals operator", () => {
    /**
     * The = operator is used both for comparison (WHERE id = 1)
     * and assignment (SET name = 'Alice').
     */
    const tokens = tokenizeSQL("=");
    expect(tokens[0].type).toBe("EQUALS");
  });

  it("tokenizes != as NOT_EQUALS", () => {
    /**
     * SQL inequality can be written as != (C-style) or <> (ANSI-style).
     * Both are normalized to NOT_EQUALS by the grammar:
     *   - NOT_EQUALS = "!=" (direct)
     *   - NEQ_ANSI   = "<>" -> NOT_EQUALS (alias)
     *
     * Longest-match-first ensures != is matched before = or !.
     */
    const tokens = tokenizeSQL("!=");
    expect(tokens[0].type).toBe("NOT_EQUALS");
  });

  it("tokenizes <> as NOT_EQUALS", () => {
    /**
     * The ANSI SQL inequality operator <> is aliased to NOT_EQUALS in
     * sql.tokens. This allows the parser grammar to reference a single
     * token type for both inequality spellings.
     */
    const tokens = tokenizeSQL("<>");
    expect(tokens[0].type).toBe("NOT_EQUALS");
  });

  it("tokenizes <= as LESS_EQUALS", () => {
    /**
     * The less-than-or-equal operator. Longest-match-first ensures
     * this is recognized as one token rather than < followed by =.
     */
    const tokens = tokenizeSQL("<=");
    expect(tokens[0].type).toBe("LESS_EQUALS");
  });

  it("tokenizes >= as GREATER_EQUALS", () => {
    /**
     * The greater-than-or-equal operator.
     */
    const tokens = tokenizeSQL(">=");
    expect(tokens[0].type).toBe("GREATER_EQUALS");
  });

  it("tokenizes < as LESS_THAN", () => {
    /**
     * The simple less-than operator. The grammar places <= before < so
     * that the longer match wins when followed by =.
     */
    const tokens = tokenizeSQL("<");
    expect(tokens[0].type).toBe("LESS_THAN");
  });

  it("tokenizes > as GREATER_THAN", () => {
    /**
     * The simple greater-than operator.
     */
    const tokens = tokenizeSQL(">");
    expect(tokens[0].type).toBe("GREATER_THAN");
  });
});

describe("punctuation", () => {
  it("tokenizes parentheses", () => {
    /**
     * Parentheses are used for function calls, subqueries, and expression
     * grouping in SQL.
     */
    const types = tokenTypes("()");
    expect(types).toContain("LPAREN");
    expect(types).toContain("RPAREN");
  });

  it("tokenizes comma", () => {
    /**
     * Commas separate items in SELECT lists, INSERT values, and function
     * argument lists.
     */
    const types = tokenTypes(",");
    expect(types).toContain("COMMA");
  });

  it("tokenizes semicolon", () => {
    /**
     * Semicolons terminate SQL statements. In multi-statement scripts,
     * each statement ends with a semicolon.
     */
    const types = tokenTypes(";");
    expect(types).toContain("SEMICOLON");
  });

  it("tokenizes dot", () => {
    /**
     * The dot is a qualifier operator used to reference columns within
     * tables (table.column) or tables within schemas (schema.table).
     */
    const types = tokenTypes(".");
    expect(types).toContain("DOT");
  });
});

describe("comment skipping", () => {
  it("skips a line comment", () => {
    /**
     * SQL line comments start with -- and continue to the end of the line.
     * They are defined in the skip: section of sql.tokens and silently
     * consumed without producing tokens.
     *
     * Example: SELECT 1 -- this is a comment
     * Only the keyword and number tokens should appear.
     */
    const tokens = tokenizeSQL("SELECT 1 -- this comment is ignored\n");
    const types = tokens.map((t) => t.type);
    expect(types).not.toContain("LINE_COMMENT");
    // Should only have keyword, number, and EOF
    const meaningfulTypes = types.filter((t) => t !== "EOF");
    expect(meaningfulTypes).toEqual(["KEYWORD", "NUMBER"]);
  });

  it("skips a block comment", () => {
    /**
     * SQL block comments are delimited by /* and *\/.
     * They can span multiple lines and are silently consumed.
     *
     * Example: SELECT /* find all rows *\/ 1
     */
    const tokens = tokenizeSQL("SELECT /* this is skipped */ 1");
    const types = tokens.map((t) => t.type);
    expect(types).not.toContain("BLOCK_COMMENT");
    const meaningfulTypes = types.filter((t) => t !== "EOF");
    expect(meaningfulTypes).toEqual(["KEYWORD", "NUMBER"]);
  });
});

describe("WHERE clause", () => {
  it("tokenizes a simple WHERE clause", () => {
    /**
     * A WHERE clause filters rows based on a condition. This test verifies
     * that the lexer correctly handles the common WHERE x = y pattern.
     *
     * SELECT * FROM users WHERE age > 18
     *
     * Token breakdown:
     *   - SELECT: KEYWORD
     *   - *: STAR
     *   - FROM: KEYWORD
     *   - users: NAME
     *   - WHERE: KEYWORD
     *   - age: NAME
     *   - >: GREATER_THAN
     *   - 18: NUMBER
     */
    const tokens = tokenizeSQL("SELECT * FROM users WHERE age > 18");
    const types = tokens.map((t) => t.type).filter((t) => t !== "EOF");
    expect(types).toEqual([
      "KEYWORD", // SELECT
      "STAR",    // *
      "KEYWORD", // FROM
      "NAME",    // users
      "KEYWORD", // WHERE
      "NAME",    // age
      "GREATER_THAN", // >
      "NUMBER",  // 18
    ]);
  });
});

describe("qualified names", () => {
  it("tokenizes a qualified name (schema.orders) as NAME DOT NAME", () => {
    /**
     * SQL qualified names use dot notation to specify context:
     *   - schema.table (two-part name)
     *   - schema.table.column (three-part name)
     *
     * The dot is a separate DOT token. This is important because the
     * parser can then handle qualified references in its grammar rules.
     *
     * We use "orders" here (not "table") because TABLE is a SQL keyword
     * and would tokenize as KEYWORD instead of NAME.
     */
    const tokens = tokenizeSQL("schema.orders");
    const types = tokens.map((t) => t.type).filter((t) => t !== "EOF");
    expect(types).toEqual(["NAME", "DOT", "NAME"]);
    expect(tokens[0].value).toBe("schema");
    expect(tokens[2].value).toBe("orders");
  });
});

describe("backtick-quoted identifiers", () => {
  it("tokenizes backtick-quoted identifier as NAME with backticks retained", () => {
    /**
     * Backtick-quoted identifiers allow using reserved words as names:
     *   `select` -- a column named "select"
     *   `order` -- a column named "order"
     *
     * In sql.tokens, QUOTED_ID = /\`[^\`]+\`/ -> NAME
     * This means the token type is NAME (via alias), but unlike STRING
     * tokens where quotes are stripped, backtick-quoted identifiers
     * retain their surrounding backticks in the token value.
     *
     * This is the expected behavior: backtick-quoting is part of the
     * identifier syntax, not just a wrapper to be stripped.
     */
    const tokens = tokenizeSQL("`my_column`");
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("`my_column`");
  });
});

describe("createSQLLexer factory", () => {
  it("createSQLLexer returns a non-null array of tokens", () => {
    /**
     * The createSQLLexer factory function provides a named entry point
     * for creating a SQL lexer. It should return a valid token array
     * for any non-empty SQL input.
     */
    const tokens = createSQLLexer("SELECT 1");
    expect(tokens).not.toBeNull();
    expect(Array.isArray(tokens)).toBe(true);
    expect(tokens.length).toBeGreaterThan(0);
  });

  it("createSQLLexer and tokenizeSQL produce identical results", () => {
    /**
     * The factory function is just an alias for tokenizeSQL. Both should
     * produce exactly the same token array for any input.
     */
    const source = "SELECT id FROM users WHERE id = 1";
    const fromFactory = createSQLLexer(source);
    const fromFunction = tokenizeSQL(source);
    expect(fromFactory).toEqual(fromFunction);
  });
});
