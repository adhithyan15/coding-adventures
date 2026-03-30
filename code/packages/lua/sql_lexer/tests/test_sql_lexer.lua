-- Tests for sql_lexer
-- ====================
--
-- Comprehensive busted test suite for the SQL lexer package.
--
-- SQL (Structured Query Language) is a declarative language for relational
-- databases. This suite exercises all token types produced by the
-- `sql.tokens` grammar.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty input produces only EOF
--   - SELECT * FROM users WHERE id = 1
--   - INSERT INTO ... VALUES ...
--   - UPDATE ... SET ...
--   - DELETE FROM ...
--   - String literals (single-quoted)
--   - Comments (line comments --, block comments /* */)
--   - Operators: =, !=, <>, <, >, <=, >=, +, -, *, /, %
--   - NULL, TRUE, FALSE literals
--   - Numeric literals: integer and decimal
--   - Keywords are case-insensitive
--   - Identifiers (NAME tokens)
--   - Delimiters: ( ) , ; .
--   - Token positions (line, col) tracked correctly
--   - Unexpected character raises an error

-- Resolve sibling packages from the monorepo so busted can find them
-- without requiring a global luarocks install.
package.path = (
    "../src/?.lua;"                                           ..
    "../src/?/init.lua;"                                      ..
    "../../grammar_tools/src/?.lua;"                          ..
    "../../grammar_tools/src/?/init.lua;"                     ..
    "../../lexer/src/?.lua;"                                  ..
    "../../lexer/src/?/init.lua;"                             ..
    "../../state_machine/src/?.lua;"                          ..
    "../../state_machine/src/?/init.lua;"                     ..
    "../../directed_graph/src/?.lua;"                         ..
    "../../directed_graph/src/?/init.lua;"                    ..
    package.path
)

local sql_lexer = require("coding_adventures.sql_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by sql_lexer.tokenize.
-- @return table         Ordered list of type strings (no EOF entry).
local function types(tokens)
    local out = {}
    for _, tok in ipairs(tokens) do
        if tok.type ~= "EOF" then
            out[#out + 1] = tok.type
        end
    end
    return out
end

--- Collect token values from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by sql_lexer.tokenize.
-- @return table         Ordered list of value strings (no EOF entry).
local function values(tokens)
    local out = {}
    for _, tok in ipairs(tokens) do
        if tok.type ~= "EOF" then
            out[#out + 1] = tok.value
        end
    end
    return out
end

--- Find the first token with the given type.
-- @param tokens  table   Token list.
-- @param typ     string  Token type to search for.
-- @return table|nil      The first matching token, or nil.
local function first_of(tokens, typ)
    for _, tok in ipairs(tokens) do
        if tok.type == typ then return tok end
    end
    return nil
end

--- Count tokens of a given type.
-- @param tokens table   Token list.
-- @param typ    string  Token type to count.
-- @return number        Number of tokens with that type.
local function count_of(tokens, typ)
    local n = 0
    for _, tok in ipairs(tokens) do
        if tok.type == typ then n = n + 1 end
    end
    return n
end

-- =========================================================================
-- Module surface
-- =========================================================================

describe("sql_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(sql_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(sql_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", sql_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(sql_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(sql_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = sql_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = sql_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input produces only EOF", function()
        local tokens = sql_lexer.tokenize("   \t\r\n  ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("line comment only produces only EOF", function()
        local tokens = sql_lexer.tokenize("-- this is a comment")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("block comment only produces only EOF", function()
        local tokens = sql_lexer.tokenize("/* block comment */")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- SELECT queries
-- =========================================================================

describe("SELECT queries", function()
    it("tokenizes SELECT * FROM users", function()
        local tokens = sql_lexer.tokenize("SELECT * FROM users")
        local t = types(tokens)
        assert.are.same({"SELECT", "STAR", "FROM", "NAME"}, t)
    end)

    it("SELECT value is case-preserved", function()
        -- Keywords are case-insensitive but values preserve original casing.
        local tokens = sql_lexer.tokenize("SELECT * FROM users")
        assert.are.equal("SELECT", tokens[1].value)
        assert.are.equal("users",  tokens[4].value)
    end)

    it("tokenizes select (lowercase) → SELECT token", function()
        local tokens = sql_lexer.tokenize("select * from users")
        assert.are.equal("SELECT", tokens[1].type)
        assert.are.equal("FROM",   tokens[3].type)
    end)

    it("tokenizes SELECT * FROM users WHERE id = 1", function()
        local tokens = sql_lexer.tokenize("SELECT * FROM users WHERE id = 1")
        local t = types(tokens)
        assert.are.same(
            {"SELECT", "STAR", "FROM", "NAME", "WHERE", "NAME", "EQUALS", "NUMBER"},
            t
        )
    end)

    it("tokenizes SELECT with column list", function()
        local tokens = sql_lexer.tokenize("SELECT id, name FROM users")
        local t = types(tokens)
        assert.are.same(
            {"SELECT", "NAME", "COMMA", "NAME", "FROM", "NAME"},
            t
        )
    end)

    it("tokenizes SELECT with WHERE and comparison operators", function()
        local tokens = sql_lexer.tokenize("SELECT * FROM t WHERE age >= 18 AND age < 65")
        local t = types(tokens)
        assert.are.same(
            {"SELECT", "STAR", "FROM", "NAME",
             "WHERE", "NAME", "GREATER_EQUALS", "NUMBER",
             "AND", "NAME", "LESS_THAN", "NUMBER"},
            t
        )
    end)

    it("tokenizes SELECT with ORDER BY and LIMIT", function()
        local tokens = sql_lexer.tokenize("SELECT * FROM t ORDER BY id DESC LIMIT 10")
        local t = types(tokens)
        assert.are.same(
            {"SELECT", "STAR", "FROM", "NAME",
             "ORDER", "BY", "NAME", "DESC", "LIMIT", "NUMBER"},
            t
        )
    end)

    it("tokenizes SELECT DISTINCT", function()
        local tokens = sql_lexer.tokenize("SELECT DISTINCT name FROM users")
        local t = types(tokens)
        assert.are.same({"SELECT", "DISTINCT", "NAME", "FROM", "NAME"}, t)
    end)
end)

-- =========================================================================
-- INSERT queries
-- =========================================================================

describe("INSERT queries", function()
    it("tokenizes INSERT INTO table VALUES (1, 'str')", function()
        local tokens = sql_lexer.tokenize("INSERT INTO users VALUES (1, 'Alice')")
        local t = types(tokens)
        assert.are.same(
            {"INSERT", "INTO", "NAME", "VALUES",
             "LPAREN", "NUMBER", "COMMA", "STRING", "RPAREN"},
            t
        )
    end)

    it("tokenizes INSERT with column list", function()
        local tokens = sql_lexer.tokenize(
            "INSERT INTO t (id, name) VALUES (1, 'Bob')"
        )
        local t = types(tokens)
        assert.are.same(
            {"INSERT", "INTO", "NAME",
             "LPAREN", "NAME", "COMMA", "NAME", "RPAREN",
             "VALUES",
             "LPAREN", "NUMBER", "COMMA", "STRING", "RPAREN"},
            t
        )
    end)
end)

-- =========================================================================
-- UPDATE and DELETE queries
-- =========================================================================

describe("UPDATE queries", function()
    it("tokenizes UPDATE t SET col = val WHERE id = 1", function()
        local tokens = sql_lexer.tokenize("UPDATE t SET name = 'Bob' WHERE id = 1")
        local t = types(tokens)
        assert.are.same(
            {"UPDATE", "NAME", "SET", "NAME", "EQUALS", "STRING",
             "WHERE", "NAME", "EQUALS", "NUMBER"},
            t
        )
    end)
end)

describe("DELETE queries", function()
    it("tokenizes DELETE FROM t WHERE id = 1", function()
        local tokens = sql_lexer.tokenize("DELETE FROM t WHERE id = 1")
        local t = types(tokens)
        assert.are.same(
            {"DELETE", "FROM", "NAME", "WHERE", "NAME", "EQUALS", "NUMBER"},
            t
        )
    end)
end)

-- =========================================================================
-- String literals
-- =========================================================================

describe("string literals", function()
    it("tokenizes a single-quoted string", function()
        local tokens = sql_lexer.tokenize("'hello'")
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal("hello", tokens[1].value)
    end)

    it("tokenizes an empty single-quoted string", function()
        local tokens = sql_lexer.tokenize("''")
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal("", tokens[1].value)
    end)

    it("tokenizes a string with an escaped single quote", function()
        local tokens = sql_lexer.tokenize("'it\\'s'")
        assert.are.equal("STRING", tokens[1].type)
    end)

    it("tokenizes multiple strings separated by comma", function()
        local tokens = sql_lexer.tokenize("'a', 'b'")
        local t = types(tokens)
        assert.are.same({"STRING", "COMMA", "STRING"}, t)
    end)
end)

-- =========================================================================
-- Numeric literals
-- =========================================================================

describe("numeric literals", function()
    it("tokenizes a positive integer", function()
        local tokens = sql_lexer.tokenize("42")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("42", tokens[1].value)
    end)

    it("tokenizes zero", function()
        local tokens = sql_lexer.tokenize("0")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("0", tokens[1].value)
    end)

    it("tokenizes a decimal number", function()
        local tokens = sql_lexer.tokenize("3.14")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("3.14", tokens[1].value)
    end)

    it("tokenizes a large integer", function()
        local tokens = sql_lexer.tokenize("1000000")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("1000000", tokens[1].value)
    end)
end)

-- =========================================================================
-- NULL, TRUE, FALSE literals
-- =========================================================================

describe("special value literals", function()
    it("tokenizes NULL", function()
        local tokens = sql_lexer.tokenize("NULL")
        assert.are.equal("NULL", tokens[1].type)
        assert.are.equal("NULL", tokens[1].value)
    end)

    it("tokenizes null (lowercase) → NULL token", function()
        local tokens = sql_lexer.tokenize("null")
        assert.are.equal("NULL", tokens[1].type)
    end)

    it("tokenizes TRUE", function()
        local tokens = sql_lexer.tokenize("TRUE")
        assert.are.equal("TRUE", tokens[1].type)
    end)

    it("tokenizes FALSE", function()
        local tokens = sql_lexer.tokenize("FALSE")
        assert.are.equal("FALSE", tokens[1].type)
    end)

    it("tokenizes IS NULL expression", function()
        local tokens = sql_lexer.tokenize("col IS NULL")
        local t = types(tokens)
        assert.are.same({"NAME", "IS", "NULL"}, t)
    end)

    it("tokenizes IS NOT NULL expression", function()
        local tokens = sql_lexer.tokenize("col IS NOT NULL")
        local t = types(tokens)
        assert.are.same({"NAME", "IS", "NOT", "NULL"}, t)
    end)
end)

-- =========================================================================
-- Operators
-- =========================================================================

describe("comparison operators", function()
    it("tokenizes = (EQUALS)", function()
        local tokens = sql_lexer.tokenize("a = 1")
        assert.are.equal("EQUALS", tokens[2].type)
    end)

    it("tokenizes != (NOT_EQUALS)", function()
        local tokens = sql_lexer.tokenize("a != 1")
        assert.are.equal("NOT_EQUALS", tokens[2].type)
        assert.are.equal("!=", tokens[2].value)
    end)

    it("tokenizes <> (NOT_EQUALS via NEQ_ANSI alias)", function()
        local tokens = sql_lexer.tokenize("a <> 1")
        -- <> is aliased to NOT_EQUALS in the grammar
        assert.are.equal("NOT_EQUALS", tokens[2].type)
        assert.are.equal("<>", tokens[2].value)
    end)

    it("tokenizes < (LESS_THAN)", function()
        local tokens = sql_lexer.tokenize("a < 1")
        assert.are.equal("LESS_THAN", tokens[2].type)
    end)

    it("tokenizes > (GREATER_THAN)", function()
        local tokens = sql_lexer.tokenize("a > 1")
        assert.are.equal("GREATER_THAN", tokens[2].type)
    end)

    it("tokenizes <= (LESS_EQUALS)", function()
        local tokens = sql_lexer.tokenize("a <= 1")
        assert.are.equal("LESS_EQUALS", tokens[2].type)
        assert.are.equal("<=", tokens[2].value)
    end)

    it("tokenizes >= (GREATER_EQUALS)", function()
        local tokens = sql_lexer.tokenize("a >= 1")
        assert.are.equal("GREATER_EQUALS", tokens[2].type)
        assert.are.equal(">=", tokens[2].value)
    end)

    it("<= matched before < (longest match wins)", function()
        -- If < were tried first, "<=1" would match as LESS_THAN then EQUALS NUMBER.
        -- The grammar orders <= before < to ensure longest match.
        local tokens = sql_lexer.tokenize("a <= 1")
        local t = types(tokens)
        assert.are.same({"NAME", "LESS_EQUALS", "NUMBER"}, t)
    end)
end)

describe("arithmetic operators", function()
    it("tokenizes + (PLUS)", function()
        local tokens = sql_lexer.tokenize("1 + 2")
        assert.are.equal("PLUS", tokens[2].type)
    end)

    it("tokenizes - (MINUS)", function()
        local tokens = sql_lexer.tokenize("1 - 2")
        assert.are.equal("MINUS", tokens[2].type)
    end)

    it("tokenizes * (STAR)", function()
        local tokens = sql_lexer.tokenize("SELECT *")
        assert.are.equal("STAR", tokens[2].type)
    end)

    it("tokenizes / (SLASH)", function()
        local tokens = sql_lexer.tokenize("a / b")
        assert.are.equal("SLASH", tokens[2].type)
    end)

    it("tokenizes % (PERCENT)", function()
        local tokens = sql_lexer.tokenize("a % b")
        assert.are.equal("PERCENT", tokens[2].type)
    end)
end)

-- =========================================================================
-- Delimiters
-- =========================================================================

describe("delimiters", function()
    it("tokenizes ( and )", function()
        local tokens = sql_lexer.tokenize("(1)")
        local t = types(tokens)
        assert.are.same({"LPAREN", "NUMBER", "RPAREN"}, t)
    end)

    it("tokenizes , (COMMA)", function()
        local tokens = sql_lexer.tokenize("1, 2")
        assert.are.equal("COMMA", tokens[2].type)
    end)

    it("tokenizes ; (SEMICOLON)", function()
        local tokens = sql_lexer.tokenize("SELECT 1;")
        assert.are.equal("SEMICOLON", tokens[3].type)
    end)

    it("tokenizes . (DOT) for table.column", function()
        local tokens = sql_lexer.tokenize("t.col")
        local t = types(tokens)
        assert.are.same({"NAME", "DOT", "NAME"}, t)
    end)
end)

-- =========================================================================
-- Comments
-- =========================================================================

describe("comment handling", function()
    it("line comment after tokens is consumed silently", function()
        local tokens = sql_lexer.tokenize("SELECT 1 -- this is a comment")
        local t = types(tokens)
        assert.are.same({"SELECT", "NUMBER"}, t)
    end)

    it("block comment between tokens is consumed silently", function()
        local tokens = sql_lexer.tokenize("SELECT /* comment */ 1")
        local t = types(tokens)
        assert.are.same({"SELECT", "NUMBER"}, t)
    end)

    it("block comment spanning multiple lines is consumed", function()
        local tokens = sql_lexer.tokenize("SELECT /*\n  big comment\n*/ 1")
        local t = types(tokens)
        assert.are.same({"SELECT", "NUMBER"}, t)
    end)
end)

-- =========================================================================
-- JOIN clauses
-- =========================================================================

describe("JOIN clauses", function()
    it("tokenizes INNER JOIN ... ON", function()
        local tokens = sql_lexer.tokenize(
            "SELECT * FROM a INNER JOIN b ON a.id = b.id"
        )
        local t = types(tokens)
        assert.are.same(
            {"SELECT", "STAR", "FROM", "NAME",
             "INNER", "JOIN", "NAME",
             "ON", "NAME", "DOT", "NAME", "EQUALS", "NAME", "DOT", "NAME"},
            t
        )
    end)

    it("tokenizes LEFT JOIN", function()
        local tokens = sql_lexer.tokenize("SELECT * FROM a LEFT JOIN b ON a.id = b.id")
        assert.are.equal("LEFT", tokens[5].type)
        assert.are.equal("JOIN", tokens[6].type)
    end)
end)

-- =========================================================================
-- Whitespace handling
-- =========================================================================

describe("whitespace handling", function()
    it("strips spaces between tokens", function()
        local tokens = sql_lexer.tokenize("SELECT   *   FROM   t")
        local t = types(tokens)
        assert.are.same({"SELECT", "STAR", "FROM", "NAME"}, t)
    end)

    it("strips newlines between tokens", function()
        local tokens = sql_lexer.tokenize("SELECT\n*\nFROM\nt")
        local t = types(tokens)
        assert.are.same({"SELECT", "STAR", "FROM", "NAME"}, t)
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("tracks column for single-line input", function()
        -- Input: SELECT 1
        -- col:   1234567 89
        local tokens = sql_lexer.tokenize("SELECT 1")
        assert.are.equal(1, tokens[1].col)  -- SELECT
        assert.are.equal(8, tokens[2].col)  -- 1
    end)

    it("all tokens start on line 1 for a single-line input", function()
        local tokens = sql_lexer.tokenize("SELECT * FROM t")
        for _, tok in ipairs(tokens) do
            assert.are.equal(1, tok.line)
        end
    end)
end)

-- =========================================================================
-- Composite SQL structures
-- =========================================================================

describe("composite SQL structures", function()
    it("tokenizes a real-world SELECT with GROUP BY and HAVING", function()
        local src = [[
SELECT department, COUNT(*) AS cnt
FROM employees
WHERE salary > 50000
GROUP BY department
HAVING cnt > 5
ORDER BY cnt DESC
]]
        local tokens = sql_lexer.tokenize(src)
        assert.truthy(#tokens > 20)

        local first_select = first_of(tokens, "SELECT")
        assert.is_not_nil(first_select)

        local first_from = first_of(tokens, "FROM")
        assert.is_not_nil(first_from)

        assert.truthy(count_of(tokens, "NAME") >= 5)
        assert.truthy(count_of(tokens, "NUMBER") >= 2)
    end)

    it("tokenizes a CREATE TABLE statement", function()
        local src = "CREATE TABLE users (id NUMBER, name STRING)"
        local tokens = sql_lexer.tokenize(src)
        local t = types(tokens)
        assert.are.same(
            {"CREATE", "TABLE", "NAME",
             "LPAREN",
             "NAME", "NAME", "COMMA",
             "NAME", "NAME",
             "RPAREN"},
            t
        )
    end)

    it("tokenizes BETWEEN...AND", function()
        local tokens = sql_lexer.tokenize("age BETWEEN 18 AND 65")
        local t = types(tokens)
        assert.are.same(
            {"NAME", "BETWEEN", "NUMBER", "AND", "NUMBER"},
            t
        )
    end)

    it("tokenizes LIKE pattern matching", function()
        local tokens = sql_lexer.tokenize("name LIKE '%Alice%'")
        local t = types(tokens)
        assert.are.same({"NAME", "LIKE", "STRING"}, t)
    end)

    it("tokenizes IN list", function()
        local tokens = sql_lexer.tokenize("id IN (1, 2, 3)")
        local t = types(tokens)
        assert.are.same(
            {"NAME", "IN",
             "LPAREN", "NUMBER", "COMMA", "NUMBER", "COMMA", "NUMBER", "RPAREN"},
            t
        )
    end)
end)

-- =========================================================================
-- EOF token
-- =========================================================================

describe("EOF token", function()
    it("is always the last token", function()
        local tokens = sql_lexer.tokenize("SELECT 1")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = sql_lexer.tokenize("SELECT 1")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error on unexpected character", function()
        assert.has_error(function()
            sql_lexer.tokenize("@")
        end)
    end)
end)
