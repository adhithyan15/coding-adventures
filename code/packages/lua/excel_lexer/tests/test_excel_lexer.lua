-- Tests for excel_lexer
-- =====================
--
-- Comprehensive busted test suite for the Excel formula lexer.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty input produces only EOF
--   - EQUALS token (formula prefix "=")
--   - CELL references: A1, $B$2, mixed absolute/relative
--   - NUMBER literals: integer, float, scientific notation
--   - STRING literals with double-quote escaping
--   - BOOL keywords: TRUE / FALSE (case-insensitive)
--   - ERROR_CONSTANT tokens: #DIV/0!, #VALUE!, etc.
--   - Arithmetic operators: + - * / ^ &
--   - Comparison operators: = <> <= >= < >
--   - PERCENT postfix operator
--   - LPAREN / RPAREN for function calls
--   - COLON for range references
--   - COMMA and SEMICOLON as argument separators
--   - SPACE token (intersection operator — NOT silently skipped)
--   - REF_PREFIX: bare sheet reference like sheet1!
--   - REF_PREFIX: quoted sheet reference like 'my sheet'!
--   - STRUCTURED_KEYWORD: [#All], [#Data], etc.
--   - STRUCTURED_COLUMN: [ColumnName]
--   - AT: @ for dynamic array spill
--   - NAME: identifiers that are not keywords
--   - Composite formula: =SUM(A1:B10)
--   - Composite formula: =IF(A1>0, "pos", "neg")
--   - Cross-sheet reference: =Sheet1!A1
--   - Error on unexpected character

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

local excel_lexer = require("coding_adventures.excel_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF).
-- Also ignores SPACE tokens since those are an implementation detail of
-- the intersection operator that most tests don't care about.
-- @param tokens  table  The token list returned by excel_lexer.tokenize.
-- @return table         Ordered list of type strings (no EOF, no SPACE).
local function types(tokens)
    local out = {}
    for _, tok in ipairs(tokens) do
        if tok.type ~= "EOF" and tok.type ~= "SPACE" then
            out[#out + 1] = tok.type
        end
    end
    return out
end

--- Collect token types INCLUDING space tokens (for intersection-operator tests).
local function types_with_space(tokens)
    local out = {}
    for _, tok in ipairs(tokens) do
        if tok.type ~= "EOF" then
            out[#out + 1] = tok.type
        end
    end
    return out
end

--- Collect token values (ignoring EOF and SPACE).
local function values(tokens)
    local out = {}
    for _, tok in ipairs(tokens) do
        if tok.type ~= "EOF" and tok.type ~= "SPACE" then
            out[#out + 1] = tok.value
        end
    end
    return out
end

--- Find the first token with the given type.
local function first_of(tokens, typ)
    for _, tok in ipairs(tokens) do
        if tok.type == typ then return tok end
    end
    return nil
end

-- =========================================================================
-- Module surface
-- =========================================================================

describe("excel_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(excel_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(excel_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", excel_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(excel_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(excel_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = excel_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = excel_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("only non-space whitespace produces only EOF (tabs/CR/LF skipped)", function()
        local tokens = excel_lexer.tokenize("\t\r\n")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Single-token tests
-- =========================================================================

describe("EQUALS token", function()
    it("tokenizes standalone =", function()
        local tokens = excel_lexer.tokenize("=")
        assert.are.equal("EQUALS", tokens[1].type)
        assert.are.equal("=", tokens[1].value)
    end)
end)

describe("CELL token", function()
    -- A1 style: optional $ before column letter(s) and/or row number.
    -- Dollar signs make the reference absolute (non-adjusting when copied).
    -- E.g.  A1    — relative row and column
    --       $A$1  — absolute row and column ("locked")
    --       $A1   — absolute column, relative row
    --       A$1   — relative column, absolute row

    it("tokenizes a simple cell reference A1", function()
        local tokens = excel_lexer.tokenize("A1")
        assert.are.equal("CELL", tokens[1].type)
        assert.are.equal("a1", tokens[1].value)  -- lowercased
    end)

    it("tokenizes an absolute cell reference $B$2", function()
        local tokens = excel_lexer.tokenize("$B$2")
        assert.are.equal("CELL", tokens[1].type)
        assert.are.equal("$b$2", tokens[1].value)
    end)

    it("tokenizes mixed absolute/relative $C3", function()
        local tokens = excel_lexer.tokenize("$C3")
        assert.are.equal("CELL", tokens[1].type)
        assert.are.equal("$c3", tokens[1].value)
    end)

    it("tokenizes multi-letter column AB100", function()
        local tokens = excel_lexer.tokenize("AB100")
        assert.are.equal("CELL", tokens[1].type)
        assert.are.equal("ab100", tokens[1].value)
    end)
end)

describe("NUMBER token", function()
    it("tokenizes an integer", function()
        local tokens = excel_lexer.tokenize("42")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("42", tokens[1].value)
    end)

    it("tokenizes a float", function()
        local tokens = excel_lexer.tokenize("3.14")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("3.14", tokens[1].value)
    end)

    it("tokenizes scientific notation", function()
        local tokens = excel_lexer.tokenize("1.5e10")
        assert.are.equal("NUMBER", tokens[1].type)
    end)

    it("tokenizes a decimal fraction without leading digit (.5)", function()
        local tokens = excel_lexer.tokenize(".5")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal(".5", tokens[1].value)
    end)
end)

describe("STRING token", function()
    -- Excel strings are delimited by double quotes.
    -- A literal double-quote inside a string is escaped by doubling it: "say ""hi"""

    it("tokenizes a simple string", function()
        local tokens = excel_lexer.tokenize('"hello"')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('hello', tokens[1].value)
    end)

    it("tokenizes an empty string", function()
        local tokens = excel_lexer.tokenize('""')
        assert.are.equal("STRING", tokens[1].type)
    end)

    it("tokenizes a string with doubled-quote escape", function()
        -- Excel: "say ""hi""" → the string contains: say "hi"
        local tokens = excel_lexer.tokenize('"say ""hi"""')
        assert.are.equal("STRING", tokens[1].type)
    end)
end)

describe("BOOL token", function()
    -- In Excel, TRUE and FALSE are worksheet functions that return boolean values,
    -- but they are also recognized as keyword literals.  The grammar declares them
    -- as keywords in excel.tokens, so the lexer emits them with their keyword type.

    it("tokenizes TRUE (uppercase)", function()
        local tokens = excel_lexer.tokenize("TRUE")
        assert.are.equal("TRUE", tokens[1].type)
        assert.are.equal("true", tokens[1].value)  -- normalized to lowercase
    end)

    it("tokenizes FALSE (lowercase)", function()
        local tokens = excel_lexer.tokenize("false")
        assert.are.equal("FALSE", tokens[1].type)
        assert.are.equal("false", tokens[1].value)
    end)

    it("tokenizes True (mixed case — case-insensitive)", function()
        local tokens = excel_lexer.tokenize("True")
        assert.are.equal("TRUE", tokens[1].type)
    end)
end)

describe("ERROR_CONSTANT tokens", function()
    -- Excel error values are special tokens that look like #NAME?.
    -- They are produced by worksheet errors and can also appear in IF formulas.

    it("tokenizes #DIV/0!", function()
        local tokens = excel_lexer.tokenize("#DIV/0!")
        assert.are.equal("ERROR_CONSTANT", tokens[1].type)
    end)

    it("tokenizes #VALUE!", function()
        local tokens = excel_lexer.tokenize("#VALUE!")
        assert.are.equal("ERROR_CONSTANT", tokens[1].type)
    end)

    it("tokenizes #REF!", function()
        local tokens = excel_lexer.tokenize("#REF!")
        assert.are.equal("ERROR_CONSTANT", tokens[1].type)
    end)

    it("tokenizes #NAME?", function()
        local tokens = excel_lexer.tokenize("#NAME?")
        assert.are.equal("ERROR_CONSTANT", tokens[1].type)
    end)

    it("tokenizes #N/A", function()
        local tokens = excel_lexer.tokenize("#N/A")
        assert.are.equal("ERROR_CONSTANT", tokens[1].type)
    end)

    it("tokenizes #NULL!", function()
        local tokens = excel_lexer.tokenize("#NULL!")
        assert.are.equal("ERROR_CONSTANT", tokens[1].type)
    end)
end)

describe("operator tokens", function()
    it("tokenizes arithmetic operators + - * / ^ &", function()
        local tokens = excel_lexer.tokenize("+-*/^&")
        local t = types(tokens)
        assert.are.same({"PLUS","MINUS","STAR","SLASH","CARET","AMP"}, t)
    end)

    it("tokenizes comparison operators = <> <= >= < >", function()
        local tokens = excel_lexer.tokenize("=<><=>=<>")
        local t = types(tokens)
        -- EQUALS, NOT_EQUALS, LESS_EQUALS, GREATER_EQUALS, LESS_THAN, GREATER_THAN
        assert.are.same(
            {"EQUALS","NOT_EQUALS","LESS_EQUALS","GREATER_EQUALS","LESS_THAN","GREATER_THAN"},
            t
        )
    end)

    it("tokenizes PERCENT postfix operator", function()
        local tokens = excel_lexer.tokenize("50%")
        local t = types(tokens)
        assert.are.same({"NUMBER","PERCENT"}, t)
    end)
end)

describe("grouping and separator tokens", function()
    it("tokenizes LPAREN and RPAREN", function()
        local tokens = excel_lexer.tokenize("()")
        local t = types(tokens)
        assert.are.same({"LPAREN","RPAREN"}, t)
    end)

    it("tokenizes COLON (range separator)", function()
        local tokens = excel_lexer.tokenize("A1:B2")
        local t = types(tokens)
        assert.are.same({"CELL","COLON","CELL"}, t)
    end)

    it("tokenizes COMMA (argument separator)", function()
        local tokens = excel_lexer.tokenize("1,2")
        local t = types(tokens)
        assert.are.same({"NUMBER","COMMA","NUMBER"}, t)
    end)

    it("tokenizes SEMICOLON (locale-dependent argument separator)", function()
        -- In some locales (e.g., German), semicolons separate function arguments.
        local tokens = excel_lexer.tokenize("1;2")
        local t = types(tokens)
        assert.are.same({"NUMBER","SEMICOLON","NUMBER"}, t)
    end)
end)

describe("SPACE token (intersection operator)", function()
    -- In Excel, a space between two range references is the INTERSECTION operator.
    -- Example: =SUM(A1:B10 B5:C15)  — intersects two ranges.
    -- This is different from JSON/most languages where spaces are always skipped.
    -- Only non-space whitespace (tabs, carriage return, newline) is silently dropped.

    it("emits SPACE token between two cells", function()
        local tokens = excel_lexer.tokenize("A1 B2")
        local t = types_with_space(tokens)
        assert.are.same({"CELL","SPACE","CELL"}, t)
    end)
end)

describe("REF_PREFIX tokens (cross-sheet references)", function()
    -- REF_PREFIX tokens capture the "sheet!" prefix in cross-sheet references.
    -- Two forms:
    --   Bare:   Sheet1!  — no quotes needed when the name has no spaces
    --   Quoted: 'My Sheet'!  — required when the name contains spaces/special chars

    it("tokenizes a bare sheet reference Sheet1!", function()
        local tokens = excel_lexer.tokenize("Sheet1!A1")
        local t = types(tokens)
        assert.are.same({"REF_PREFIX","CELL"}, t)
    end)

    it("tokenizes a quoted sheet reference 'My Sheet'!A1", function()
        local tokens = excel_lexer.tokenize("'My Sheet'!A1")
        local t = types(tokens)
        assert.are.same({"REF_PREFIX","CELL"}, t)
    end)
end)

describe("STRUCTURED_KEYWORD and STRUCTURED_COLUMN tokens", function()
    -- Structured references allow Excel Tables to be referenced by name.
    -- [#Headers], [#Data], [#All], [#Totals], [#This Row] are special keywords.
    -- [ColumnName] references a table column by name.

    it("tokenizes STRUCTURED_KEYWORD [#Headers]", function()
        local tokens = excel_lexer.tokenize("[#Headers]")
        assert.are.equal("STRUCTURED_KEYWORD", tokens[1].type)
    end)

    it("tokenizes STRUCTURED_KEYWORD [#All]", function()
        local tokens = excel_lexer.tokenize("[#All]")
        assert.are.equal("STRUCTURED_KEYWORD", tokens[1].type)
    end)

    it("tokenizes STRUCTURED_COLUMN [Amount]", function()
        local tokens = excel_lexer.tokenize("[Amount]")
        assert.are.equal("STRUCTURED_COLUMN", tokens[1].type)
    end)
end)

describe("AT and NAME tokens", function()
    it("tokenizes AT (@) for dynamic array spill reference", function()
        local tokens = excel_lexer.tokenize("@")
        assert.are.equal("AT", tokens[1].type)
    end)

    it("tokenizes a bare NAME identifier", function()
        local tokens = excel_lexer.tokenize("MyNamedRange")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("mynamerange", tokens[1].value:lower())
    end)
end)

-- =========================================================================
-- Composite formulas
-- =========================================================================

describe("composite Excel formulas", function()
    it("tokenizes =A1+B2", function()
        local tokens = excel_lexer.tokenize("=A1+B2")
        local t = types(tokens)
        assert.are.same({"EQUALS","CELL","PLUS","CELL"}, t)
    end)

    it("tokenizes =SUM(A1:B10)", function()
        local tokens = excel_lexer.tokenize("=SUM(A1:B10)")
        local t = types(tokens)
        -- SUM is not a keyword → NAME; followed by LPAREN, CELL, COLON, CELL, RPAREN
        assert.are.same({"EQUALS","NAME","LPAREN","CELL","COLON","CELL","RPAREN"}, t)
    end)

    it("tokenizes =IF(A1>0, \"pos\", \"neg\")", function()
        local tokens = excel_lexer.tokenize('=IF(A1>0, "pos", "neg")')
        local t = types(tokens)
        assert.are.same({
            "EQUALS","NAME","LPAREN",
            "CELL","GREATER_THAN","NUMBER",
            "COMMA","STRING",
            "COMMA","STRING",
            "RPAREN"
        }, t)
    end)

    it("tokenizes cross-sheet reference =Sheet1!A1", function()
        local tokens = excel_lexer.tokenize("=Sheet1!A1")
        local t = types(tokens)
        assert.are.same({"EQUALS","REF_PREFIX","CELL"}, t)
    end)

    it("tokenizes percentage formula =A1*100%", function()
        local tokens = excel_lexer.tokenize("=A1*100%")
        local t = types(tokens)
        assert.are.same({"EQUALS","CELL","STAR","NUMBER","PERCENT"}, t)
    end)

    it("tokenizes array constant formula ={1,2;3,4}", function()
        local tokens = excel_lexer.tokenize("={1,2;3,4}")
        local t = types(tokens)
        assert.are.same({
            "EQUALS","LBRACE",
            "NUMBER","COMMA","NUMBER",
            "SEMICOLON",
            "NUMBER","COMMA","NUMBER",
            "RBRACE"
        }, t)
    end)

    it("tokenizes concatenation =A1&\" world\"", function()
        local tokens = excel_lexer.tokenize('=A1&" world"')
        local t = types(tokens)
        assert.are.same({"EQUALS","CELL","AMP","STRING"}, t)
    end)

    it("tokenizes error formula =IFERROR(A1/B1, #DIV/0!)", function()
        local tokens = excel_lexer.tokenize("=IFERROR(A1/B1,#DIV/0!)")
        local t = types(tokens)
        assert.are.same({
            "EQUALS","NAME","LPAREN",
            "CELL","SLASH","CELL",
            "COMMA","ERROR_CONSTANT",
            "RPAREN"
        }, t)
    end)
end)

-- =========================================================================
-- EOF token
-- =========================================================================

describe("EOF token", function()
    it("is always the last token", function()
        local tokens = excel_lexer.tokenize("1")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = excel_lexer.tokenize("1")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("tracks line 1, col 1 for first token", function()
        local tokens = excel_lexer.tokenize("=A1")
        assert.are.equal(1, tokens[1].line)
        assert.are.equal(1, tokens[1].col)
    end)

    it("tracks column correctly across a simple formula", function()
        -- =A1  →  = at col 1, A1 at col 2
        local tokens = excel_lexer.tokenize("=A1")
        assert.are.equal(1, tokens[1].col)  -- EQUALS
        assert.are.equal(2, tokens[2].col)  -- CELL A1
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error on a truly unrecognized character", function()
        -- Backtick is not part of any Excel token
        assert.has_error(function()
            excel_lexer.tokenize("`")
        end)
    end)
end)
