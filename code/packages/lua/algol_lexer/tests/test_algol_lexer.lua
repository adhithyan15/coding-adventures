-- Tests for algol_lexer
-- ======================
--
-- Comprehensive busted test suite for the ALGOL 60 lexer package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - All ALGOL 60 keywords: begin, end, if, then, else, for, do, step,
--     until, while, goto, switch, procedure, integer, real, boolean, string,
--     array, value, true, false, not, and, or, impl, eqv, div, mod
--   - Identifiers: x, sum, A1
--   - Integer literals: 0, 42
--   - Real literals: 3.14, 1.5E3
--   - String literals: 'hello', ''
--   - All operators and delimiters
--   - Multi-character operator disambiguation: :=, **, <=, >=, !=
--   - Keyword boundary: begin → BEGIN, beginning → IDENT
--   - Comment skipping: comment this is ignored; x := 1
--   - Multi-token: x := 1 + 2
--   - Whitespace is consumed silently
--   - Position tracking (line, col)
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

local algol_lexer = require("coding_adventures.algol_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by algol_lexer.tokenize.
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
-- @param tokens  table  The token list returned by algol_lexer.tokenize.
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

-- =========================================================================
-- Module surface
-- =========================================================================

describe("algol_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(algol_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(algol_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", algol_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(algol_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(algol_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = algol_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = algol_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input produces only EOF", function()
        local tokens = algol_lexer.tokenize("   \t\r\n  ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Keywords — block structure
-- =========================================================================

describe("block structure keywords", function()
    it("tokenizes begin", function()
        local tokens = algol_lexer.tokenize("begin")
        assert.are.equal("BEGIN", tokens[1].type)
        assert.are.equal("begin", tokens[1].value)
    end)

    it("tokenizes end", function()
        local tokens = algol_lexer.tokenize("end")
        assert.are.equal("END", tokens[1].type)
    end)

    it("tokenizes BEGIN in uppercase", function()
        local tokens = algol_lexer.tokenize("BEGIN")
        assert.are.equal("BEGIN", tokens[1].type)
        -- value preserves original case
        assert.are.equal("BEGIN", tokens[1].value)
    end)

    it("tokenizes Begin in mixed case", function()
        local tokens = algol_lexer.tokenize("Begin")
        assert.are.equal("BEGIN", tokens[1].type)
        assert.are.equal("Begin", tokens[1].value)
    end)
end)

-- =========================================================================
-- Keywords — control flow
-- =========================================================================

describe("control flow keywords", function()
    it("tokenizes if", function()
        local tokens = algol_lexer.tokenize("if")
        assert.are.equal("IF", tokens[1].type)
    end)

    it("tokenizes then", function()
        local tokens = algol_lexer.tokenize("then")
        assert.are.equal("THEN", tokens[1].type)
    end)

    it("tokenizes else", function()
        local tokens = algol_lexer.tokenize("else")
        assert.are.equal("ELSE", tokens[1].type)
    end)

    it("tokenizes for", function()
        local tokens = algol_lexer.tokenize("for")
        assert.are.equal("FOR", tokens[1].type)
    end)

    it("tokenizes do", function()
        local tokens = algol_lexer.tokenize("do")
        assert.are.equal("DO", tokens[1].type)
    end)

    it("tokenizes step", function()
        local tokens = algol_lexer.tokenize("step")
        assert.are.equal("STEP", tokens[1].type)
    end)

    it("tokenizes until", function()
        local tokens = algol_lexer.tokenize("until")
        assert.are.equal("UNTIL", tokens[1].type)
    end)

    it("tokenizes while", function()
        local tokens = algol_lexer.tokenize("while")
        assert.are.equal("WHILE", tokens[1].type)
    end)

    it("tokenizes goto", function()
        local tokens = algol_lexer.tokenize("goto")
        assert.are.equal("GOTO", tokens[1].type)
    end)
end)

-- =========================================================================
-- Keywords — declarations
-- =========================================================================

describe("declaration keywords", function()
    it("tokenizes switch", function()
        local tokens = algol_lexer.tokenize("switch")
        assert.are.equal("SWITCH", tokens[1].type)
    end)

    it("tokenizes procedure", function()
        local tokens = algol_lexer.tokenize("procedure")
        assert.are.equal("PROCEDURE", tokens[1].type)
    end)

    it("tokenizes array", function()
        local tokens = algol_lexer.tokenize("array")
        assert.are.equal("ARRAY", tokens[1].type)
    end)

    it("tokenizes value", function()
        local tokens = algol_lexer.tokenize("value")
        assert.are.equal("VALUE", tokens[1].type)
    end)
end)

-- =========================================================================
-- Keywords — types
-- =========================================================================

describe("type keywords", function()
    it("tokenizes integer", function()
        local tokens = algol_lexer.tokenize("integer")
        assert.are.equal("INTEGER", tokens[1].type)
    end)

    it("tokenizes real", function()
        local tokens = algol_lexer.tokenize("real")
        assert.are.equal("REAL", tokens[1].type)
    end)

    it("tokenizes boolean", function()
        local tokens = algol_lexer.tokenize("boolean")
        assert.are.equal("BOOLEAN", tokens[1].type)
    end)

    it("tokenizes string (type keyword)", function()
        local tokens = algol_lexer.tokenize("string")
        assert.are.equal("STRING", tokens[1].type)
    end)
end)

-- =========================================================================
-- Keywords — boolean literals and operators
-- =========================================================================

describe("boolean keywords", function()
    it("tokenizes true", function()
        local tokens = algol_lexer.tokenize("true")
        assert.are.equal("TRUE", tokens[1].type)
        assert.are.equal("true", tokens[1].value)
    end)

    it("tokenizes false", function()
        local tokens = algol_lexer.tokenize("false")
        assert.are.equal("FALSE", tokens[1].type)
        assert.are.equal("false", tokens[1].value)
    end)

    it("tokenizes not", function()
        local tokens = algol_lexer.tokenize("not")
        assert.are.equal("NOT", tokens[1].type)
    end)

    it("tokenizes and", function()
        local tokens = algol_lexer.tokenize("and")
        assert.are.equal("AND", tokens[1].type)
    end)

    it("tokenizes or", function()
        local tokens = algol_lexer.tokenize("or")
        assert.are.equal("OR", tokens[1].type)
    end)

    it("tokenizes impl", function()
        local tokens = algol_lexer.tokenize("impl")
        assert.are.equal("IMPL", tokens[1].type)
    end)

    it("tokenizes eqv", function()
        local tokens = algol_lexer.tokenize("eqv")
        assert.are.equal("EQV", tokens[1].type)
    end)
end)

-- =========================================================================
-- Keywords — arithmetic
-- =========================================================================

describe("arithmetic keywords", function()
    it("tokenizes div", function()
        local tokens = algol_lexer.tokenize("div")
        assert.are.equal("DIV", tokens[1].type)
    end)

    it("tokenizes mod", function()
        local tokens = algol_lexer.tokenize("mod")
        assert.are.equal("MOD", tokens[1].type)
    end)
end)

-- =========================================================================
-- Identifiers
-- =========================================================================

describe("identifiers", function()
    it("tokenizes single-letter identifier x", function()
        local tokens = algol_lexer.tokenize("x")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("x", tokens[1].value)
    end)

    it("tokenizes multi-letter identifier sum", function()
        local tokens = algol_lexer.tokenize("sum")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("sum", tokens[1].value)
    end)

    it("tokenizes alphanumeric identifier A1", function()
        local tokens = algol_lexer.tokenize("A1")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("A1", tokens[1].value)
    end)

    it("tokenizes mixed-case identifier myVariable", function()
        local tokens = algol_lexer.tokenize("myVariable")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("myVariable", tokens[1].value)
    end)

    -- Keyword boundary: a name that starts with a keyword but continues further
    -- must be classified as IDENT, not as the keyword.
    it("beginning is IDENT, not BEGIN", function()
        local tokens = algol_lexer.tokenize("beginning")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("beginning", tokens[1].value)
    end)

    it("integer2 is IDENT, not INTEGER", function()
        local tokens = algol_lexer.tokenize("integer2")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("integer2", tokens[1].value)
    end)

    it("endloop is IDENT, not END", function()
        local tokens = algol_lexer.tokenize("endloop")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("endloop", tokens[1].value)
    end)
end)

-- =========================================================================
-- Integer literals
-- =========================================================================

describe("integer literals", function()
    it("tokenizes zero", function()
        local tokens = algol_lexer.tokenize("0")
        assert.are.equal("INTEGER_LIT", tokens[1].type)
        assert.are.equal("0", tokens[1].value)
    end)

    it("tokenizes 42", function()
        local tokens = algol_lexer.tokenize("42")
        assert.are.equal("INTEGER_LIT", tokens[1].type)
        assert.are.equal("42", tokens[1].value)
    end)

    it("tokenizes large integer 100000", function()
        local tokens = algol_lexer.tokenize("100000")
        assert.are.equal("INTEGER_LIT", tokens[1].type)
        assert.are.equal("100000", tokens[1].value)
    end)
end)

-- =========================================================================
-- Real literals
-- =========================================================================

describe("real literals", function()
    it("tokenizes 3.14", function()
        local tokens = algol_lexer.tokenize("3.14")
        assert.are.equal("REAL_LIT", tokens[1].type)
        assert.are.equal("3.14", tokens[1].value)
    end)

    it("tokenizes 1.5E3 (exponent form)", function()
        local tokens = algol_lexer.tokenize("1.5E3")
        assert.are.equal("REAL_LIT", tokens[1].type)
        assert.are.equal("1.5E3", tokens[1].value)
    end)

    it("tokenizes 1.5E-3 (negative exponent)", function()
        local tokens = algol_lexer.tokenize("1.5E-3")
        assert.are.equal("REAL_LIT", tokens[1].type)
        assert.are.equal("1.5E-3", tokens[1].value)
    end)

    it("tokenizes 100E2 (integer + exponent, no decimal point)", function()
        local tokens = algol_lexer.tokenize("100E2")
        assert.are.equal("REAL_LIT", tokens[1].type)
        assert.are.equal("100E2", tokens[1].value)
    end)

    it("tokenizes 0.5 (zero point fractional)", function()
        local tokens = algol_lexer.tokenize("0.5")
        assert.are.equal("REAL_LIT", tokens[1].type)
        assert.are.equal("0.5", tokens[1].value)
    end)

    -- REAL_LIT must come before INTEGER_LIT in the grammar so "3.14"
    -- is not mistakenly tokenized as INTEGER_LIT("3"), DOT, INTEGER_LIT("14").
    it("3.14 is one REAL_LIT token, not three tokens", function()
        local tokens = algol_lexer.tokenize("3.14")
        local t = types(tokens)
        assert.are.same({"REAL_LIT"}, t)
    end)
end)

-- =========================================================================
-- String literals
-- =========================================================================

describe("string literals", function()
    it("tokenizes 'hello'", function()
        local tokens = algol_lexer.tokenize("'hello'")
        assert.are.equal("STRING_LIT", tokens[1].type)
        -- The grammar-driven lexer returns the raw source value including quotes.
        assert.are.equal("'hello'", tokens[1].value)
    end)

    it("tokenizes empty string literal ''", function()
        local tokens = algol_lexer.tokenize("''")
        assert.are.equal("STRING_LIT", tokens[1].type)
        assert.are.equal("''", tokens[1].value)
    end)

    it("tokenizes string with spaces", function()
        local tokens = algol_lexer.tokenize("'hello world'")
        assert.are.equal("STRING_LIT", tokens[1].type)
        assert.are.equal("'hello world'", tokens[1].value)
    end)
end)

-- =========================================================================
-- Operators — multi-character (must come before single-char variants)
-- =========================================================================

describe("multi-character operators", function()
    -- := must be ASSIGN, not COLON followed by EQ
    it("tokenizes := as ASSIGN (not COLON + EQ)", function()
        local tokens = algol_lexer.tokenize(":=")
        local t = types(tokens)
        assert.are.same({"ASSIGN"}, t)
        assert.are.equal(":=", tokens[1].value)
    end)

    -- ** must be POWER, not STAR followed by STAR
    it("tokenizes ** as POWER (not STAR + STAR)", function()
        local tokens = algol_lexer.tokenize("**")
        local t = types(tokens)
        assert.are.same({"POWER"}, t)
        assert.are.equal("**", tokens[1].value)
    end)

    -- <= must be LEQ, not LT followed by EQ
    it("tokenizes <= as LEQ (not LT + EQ)", function()
        local tokens = algol_lexer.tokenize("<=")
        local t = types(tokens)
        assert.are.same({"LEQ"}, t)
        assert.are.equal("<=", tokens[1].value)
    end)

    -- >= must be GEQ, not GT followed by EQ
    it("tokenizes >= as GEQ (not GT + EQ)", function()
        local tokens = algol_lexer.tokenize(">=")
        local t = types(tokens)
        assert.are.same({"GEQ"}, t)
        assert.are.equal(">=", tokens[1].value)
    end)

    -- != must be NEQ
    it("tokenizes != as NEQ", function()
        local tokens = algol_lexer.tokenize("!=")
        local t = types(tokens)
        assert.are.same({"NEQ"}, t)
        assert.are.equal("!=", tokens[1].value)
    end)
end)

-- =========================================================================
-- Operators — single character
-- =========================================================================

describe("single-character operators", function()
    it("tokenizes +", function()
        local tokens = algol_lexer.tokenize("+")
        assert.are.equal("PLUS", tokens[1].type)
        assert.are.equal("+", tokens[1].value)
    end)

    it("tokenizes -", function()
        local tokens = algol_lexer.tokenize("-")
        assert.are.equal("MINUS", tokens[1].type)
    end)

    it("tokenizes *", function()
        local tokens = algol_lexer.tokenize("*")
        assert.are.equal("STAR", tokens[1].type)
    end)

    it("tokenizes /", function()
        local tokens = algol_lexer.tokenize("/")
        assert.are.equal("SLASH", tokens[1].type)
    end)

    it("tokenizes ^ (caret)", function()
        local tokens = algol_lexer.tokenize("^")
        assert.are.equal("CARET", tokens[1].type)
        assert.are.equal("^", tokens[1].value)
    end)

    it("tokenizes = (equality)", function()
        local tokens = algol_lexer.tokenize("=")
        assert.are.equal("EQ", tokens[1].type)
    end)

    it("tokenizes <", function()
        local tokens = algol_lexer.tokenize("<")
        assert.are.equal("LT", tokens[1].type)
    end)

    it("tokenizes >", function()
        local tokens = algol_lexer.tokenize(">")
        assert.are.equal("GT", tokens[1].type)
    end)
end)

-- =========================================================================
-- Delimiters
-- =========================================================================

describe("delimiters", function()
    it("tokenizes (", function()
        local tokens = algol_lexer.tokenize("(")
        assert.are.equal("LPAREN", tokens[1].type)
    end)

    it("tokenizes )", function()
        local tokens = algol_lexer.tokenize(")")
        assert.are.equal("RPAREN", tokens[1].type)
    end)

    it("tokenizes [", function()
        local tokens = algol_lexer.tokenize("[")
        assert.are.equal("LBRACKET", tokens[1].type)
    end)

    it("tokenizes ]", function()
        local tokens = algol_lexer.tokenize("]")
        assert.are.equal("RBRACKET", tokens[1].type)
    end)

    it("tokenizes ;", function()
        local tokens = algol_lexer.tokenize(";")
        assert.are.equal("SEMICOLON", tokens[1].type)
    end)

    it("tokenizes ,", function()
        local tokens = algol_lexer.tokenize(",")
        assert.are.equal("COMMA", tokens[1].type)
    end)

    it("tokenizes : (colon, not :=)", function()
        local tokens = algol_lexer.tokenize(":")
        assert.are.equal("COLON", tokens[1].type)
        assert.are.equal(":", tokens[1].value)
    end)

    it("correct values for all delimiters", function()
        local tokens = algol_lexer.tokenize("()[],;")
        local v = values(tokens)
        assert.are.same({"(", ")", "[", "]", ",", ";"}, v)
    end)
end)

-- =========================================================================
-- Comment skipping
-- =========================================================================

describe("comment skipping", function()
    -- The ALGOL 60 comment syntax: the word "comment" followed by arbitrary
    -- text up to and including the next semicolon. The comment is consumed
    -- silently — no token is emitted for it.
    it("skips a comment before an assignment", function()
        local tokens = algol_lexer.tokenize("comment this is ignored; x := 1")
        local t = types(tokens)
        -- Should see only IDENT ASSIGN INTEGER_LIT EOF
        assert.are.same({"NAME", "ASSIGN", "INTEGER_LIT"}, t)
        assert.are.equal("x", tokens[1].value)
    end)

    it("skips a comment between statements", function()
        local tokens = algol_lexer.tokenize("x := 1 comment increment x; x := x + 1")
        local t = types(tokens)
        assert.are.same({
            "NAME", "ASSIGN", "INTEGER_LIT",
            "NAME", "ASSIGN", "NAME", "PLUS", "INTEGER_LIT"
        }, t)
    end)

    it("empty comment body comment ;", function()
        local tokens = algol_lexer.tokenize("comment ; x := 0")
        local t = types(tokens)
        assert.are.same({"NAME", "ASSIGN", "INTEGER_LIT"}, t)
    end)
end)

-- =========================================================================
-- Whitespace handling
-- =========================================================================

describe("whitespace handling", function()
    it("strips spaces between tokens", function()
        local tokens = algol_lexer.tokenize("x := 1 + 2")
        local t = types(tokens)
        assert.are.same({"NAME", "ASSIGN", "INTEGER_LIT", "PLUS", "INTEGER_LIT"}, t)
    end)

    it("strips tabs and newlines between tokens", function()
        local tokens = algol_lexer.tokenize("x\t:=\n1")
        local t = types(tokens)
        assert.are.same({"NAME", "ASSIGN", "INTEGER_LIT"}, t)
    end)
end)

-- =========================================================================
-- Multi-token sequences
-- =========================================================================

describe("multi-token sequences", function()
    it("tokenizes x := 1 + 2", function()
        local tokens = algol_lexer.tokenize("x := 1 + 2")
        local t = types(tokens)
        assert.are.same({"NAME", "ASSIGN", "INTEGER_LIT", "PLUS", "INTEGER_LIT"}, t)
        local v = values(tokens)
        assert.are.same({"x", ":=", "1", "+", "2"}, v)
    end)

    it("tokenizes begin integer x; x := 42 end", function()
        local tokens = algol_lexer.tokenize("begin integer x; x := 42 end")
        local t = types(tokens)
        assert.are.same({
            "BEGIN", "INTEGER", "NAME", "SEMICOLON",
            "NAME", "ASSIGN", "INTEGER_LIT",
            "END"
        }, t)
    end)

    it("tokenizes if x > 0 then y := 1 else y := 0", function()
        local src = "if x > 0 then y := 1 else y := 0"
        local tokens = algol_lexer.tokenize(src)
        local t = types(tokens)
        assert.are.same({
            "IF", "NAME", "GT", "INTEGER_LIT",
            "THEN", "NAME", "ASSIGN", "INTEGER_LIT",
            "ELSE", "NAME", "ASSIGN", "INTEGER_LIT"
        }, t)
    end)

    it("tokenizes for i := 1 step 1 until 10 do", function()
        local tokens = algol_lexer.tokenize("for i := 1 step 1 until 10 do")
        local t = types(tokens)
        assert.are.same({
            "FOR", "NAME", "ASSIGN", "INTEGER_LIT",
            "STEP", "INTEGER_LIT", "UNTIL", "INTEGER_LIT", "DO"
        }, t)
    end)

    it("tokenizes array declaration array A[1:10]", function()
        local tokens = algol_lexer.tokenize("array A[1:10]")
        local t = types(tokens)
        assert.are.same({
            "ARRAY", "NAME", "LBRACKET", "INTEGER_LIT",
            "COLON", "INTEGER_LIT", "RBRACKET"
        }, t)
    end)

    it("tokenizes boolean expression: x > 0 and y < 10", function()
        local tokens = algol_lexer.tokenize("x > 0 and y < 10")
        local t = types(tokens)
        assert.are.same({
            "NAME", "GT", "INTEGER_LIT",
            "AND",
            "NAME", "LT", "INTEGER_LIT"
        }, t)
    end)

    it("tokenizes procedure declaration header", function()
        local tokens = algol_lexer.tokenize("procedure add(a, b);")
        local t = types(tokens)
        assert.are.same({
            "PROCEDURE", "NAME", "LPAREN", "NAME", "COMMA", "NAME", "RPAREN", "SEMICOLON"
        }, t)
    end)

    it("tokenizes goto statement", function()
        local tokens = algol_lexer.tokenize("goto myLabel")
        local t = types(tokens)
        assert.are.same({"GOTO", "NAME"}, t)
    end)

    it("tokenizes exponentiation with **", function()
        local tokens = algol_lexer.tokenize("x ** 2")
        local t = types(tokens)
        assert.are.same({"NAME", "POWER", "INTEGER_LIT"}, t)
    end)

    it("tokenizes exponentiation with ^", function()
        local tokens = algol_lexer.tokenize("x ^ 2")
        local t = types(tokens)
        assert.are.same({"NAME", "CARET", "INTEGER_LIT"}, t)
    end)

    it("tokenizes integer division: a div b", function()
        local tokens = algol_lexer.tokenize("a div b")
        local t = types(tokens)
        assert.are.same({"NAME", "DIV", "NAME"}, t)
    end)

    it("tokenizes modulo: a mod b", function()
        local tokens = algol_lexer.tokenize("a mod b")
        local t = types(tokens)
        assert.are.same({"NAME", "MOD", "NAME"}, t)
    end)

    it("tokenizes not operator", function()
        local tokens = algol_lexer.tokenize("not flag")
        local t = types(tokens)
        assert.are.same({"NOT", "NAME"}, t)
    end)

    it("tokenizes switch declaration", function()
        local tokens = algol_lexer.tokenize("switch s := L1, L2")
        local t = types(tokens)
        assert.are.same({"SWITCH", "NAME", "ASSIGN", "NAME", "COMMA", "NAME"}, t)
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("tracks column for single-line input", function()
        -- Input: "x := 1"
        -- col:   123456
        local tokens = algol_lexer.tokenize("x := 1")
        assert.are.equal(1, tokens[1].col)  -- x
        assert.are.equal(3, tokens[2].col)  -- :=
        assert.are.equal(6, tokens[3].col)  -- 1
    end)

    it("all tokens start on line 1 for a single-line input", function()
        local tokens = algol_lexer.tokenize("begin x := 0 end")
        for _, tok in ipairs(tokens) do
            assert.are.equal(1, tok.line)
        end
    end)
end)

-- =========================================================================
-- EOF token
-- =========================================================================

describe("EOF token", function()
    it("is always the last token", function()
        local tokens = algol_lexer.tokenize("42")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = algol_lexer.tokenize("42")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error on unexpected character @", function()
        assert.has_error(function()
            algol_lexer.tokenize("@")
        end)
    end)

    it("raises an error on unexpected character #", function()
        assert.has_error(function()
            algol_lexer.tokenize("#")
        end)
    end)

    it("raises an error on unexpected character $", function()
        assert.has_error(function()
            algol_lexer.tokenize("$")
        end)
    end)
end)

-- =========================================================================
-- Realistic ALGOL 60 program fragment
-- =========================================================================

describe("realistic program fragment", function()
    it("tokenizes a full minimal ALGOL 60 program", function()
        local src = [[
begin
    integer x, y;
    x := 10;
    y := x * 2 + 1;
    if y > 15 then
        y := y - 1
    end]]
        local tokens = algol_lexer.tokenize(src)
        -- The program should produce many tokens; spot-check key ones.
        assert.truthy(#tokens > 15)

        assert.are.equal("BEGIN",   tokens[1].type)

        local last_real = tokens[#tokens - 1]
        assert.are.equal("END", last_real.type)

        -- Spot-check: there should be at least one ASSIGN
        assert.is_not_nil(first_of(tokens, "ASSIGN"))
        -- And at least one INTEGER_LIT
        assert.is_not_nil(first_of(tokens, "INTEGER_LIT"))
    end)

    it("tokenizes a while loop fragment", function()
        local src = "for i := 1 while i <= 10 do i := i + 1"
        local tokens = algol_lexer.tokenize(src)
        local t = types(tokens)
        assert.are.same({
            "FOR", "NAME", "ASSIGN", "INTEGER_LIT",
            "WHILE", "NAME", "LEQ", "INTEGER_LIT",
            "DO", "NAME", "ASSIGN", "NAME", "PLUS", "INTEGER_LIT"
        }, t)
    end)

    it("tokenizes a procedure call with real arguments", function()
        local src = "sqrt(2.0)"
        local tokens = algol_lexer.tokenize(src)
        local t = types(tokens)
        assert.are.same({"NAME", "LPAREN", "REAL_LIT", "RPAREN"}, t)
    end)
end)
