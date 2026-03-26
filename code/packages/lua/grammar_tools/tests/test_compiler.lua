-- Tests for coding_adventures.grammar_tools.compiler
-- ============================================================================
--
-- The compiler transforms in-memory TokenGrammar and ParserGrammar tables
-- into Lua source code.  Tests verify:
--
--   1. The generated code contains expected header / DO NOT EDIT comment.
--   2. The generated code is valid Lua (loadable via load()).
--   3. Loading the generated code recreates an equivalent grammar table.
--   4. All grammar features round-trip: aliases, skip patterns, error
--      patterns, groups, keywords, mode, escape_mode, case_sensitive,
--      case_insensitive.
--   5. Edge cases: empty grammars, special chars in patterns.
--
-- ## Round-trip strategy
--
--   grammar, err = grammar_tools.parse_token_grammar(source)
--   code         = grammar_tools.compile_token_grammar(grammar)
--   mod          = load(code)()        -- execute to get the module table
--   loaded       = mod.token_grammar() -- call the factory function
--   assert loaded.definitions == grammar.definitions
--
-- load() compiles Lua source into a function and executes it; the generated
-- file returns a module table with a single factory function.

-- Add src/ to the module search path.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local grammar_tools = require("coding_adventures.grammar_tools")

-- ===========================================================================
-- Helpers: eval generated Lua code and return the grammar object.
-- ===========================================================================

--- Execute generated token grammar Lua code and return the grammar object.
-- @param code  String of Lua source (as produced by compile_token_grammar)
-- @return TokenGrammar table
local function eval_token_grammar(code)
    local fn, err = load(code)
    if not fn then
        error("Failed to load generated code: " .. tostring(err) .. "\n\nCode:\n" .. code)
    end
    local mod = fn()
    return mod.token_grammar()
end

--- Execute generated parser grammar Lua code and return the grammar object.
-- @param code  String of Lua source (as produced by compile_parser_grammar)
-- @return ParserGrammar table
local function eval_parser_grammar(code)
    local fn, err = load(code)
    if not fn then
        error("Failed to load generated code: " .. tostring(err) .. "\n\nCode:\n" .. code)
    end
    local mod = fn()
    return mod.parser_grammar()
end

--- Parse a token grammar and assert success.
local function parse_tokens(source)
    local g, err = grammar_tools.parse_token_grammar(source)
    assert(g ~= nil, "parse_token_grammar failed: " .. tostring(err))
    return g
end

--- Parse a parser grammar and assert success.
local function parse_grammar(source)
    local g, err = grammar_tools.parse_parser_grammar(source)
    assert(g ~= nil, "parse_parser_grammar failed: " .. tostring(err))
    return g
end

-- ===========================================================================
-- compile_token_grammar — output structure
-- ===========================================================================

describe("compile_token_grammar output structure", function()
    it("includes DO NOT EDIT header", function()
        local g = grammar_tools.TokenGrammar.new()
        local code = grammar_tools.compile_token_grammar(g)
        assert.is_truthy(code:find("DO NOT EDIT", 1, true))
    end)

    it("includes source file when given", function()
        local g = grammar_tools.TokenGrammar.new()
        local code = grammar_tools.compile_token_grammar(g, "json.tokens")
        assert.is_truthy(code:find("json.tokens", 1, true))
    end)

    it("omits source line when empty string given", function()
        local g = grammar_tools.TokenGrammar.new()
        local code = grammar_tools.compile_token_grammar(g, "")
        assert.is_falsy(code:find("Source:", 1, true))
    end)

    it("includes require grammar_tools", function()
        local g = grammar_tools.TokenGrammar.new()
        local code = grammar_tools.compile_token_grammar(g)
        assert.is_truthy(code:find("grammar_tools", 1, true))
    end)

    it("includes token_grammar function", function()
        local g = grammar_tools.TokenGrammar.new()
        local code = grammar_tools.compile_token_grammar(g)
        assert.is_truthy(code:find("token_grammar", 1, true))
    end)
end)

-- ===========================================================================
-- compile_token_grammar — round-trip tests
-- ===========================================================================

describe("compile_token_grammar round-trip", function()
    it("empty grammar round-trips", function()
        local original = grammar_tools.TokenGrammar.new()
        local code = grammar_tools.compile_token_grammar(original)
        local loaded = eval_token_grammar(code)
        assert.same({}, loaded.definitions)
        assert.same({}, loaded.keywords)
        assert.equals(0, loaded.version)
        assert.is_false(loaded.case_insensitive)
    end)

    it("regex token round-trips", function()
        local original = parse_tokens("NUMBER = /[0-9]+/")
        local code = grammar_tools.compile_token_grammar(original)
        local loaded = eval_token_grammar(code)
        assert.equals(1, #loaded.definitions)
        local defn = loaded.definitions[1]
        assert.equals("NUMBER", defn.name)
        assert.equals("[0-9]+", defn.pattern)
        assert.is_true(defn.is_regex)
    end)

    it("literal token round-trips", function()
        local original = parse_tokens('PLUS = "+"')
        local code = grammar_tools.compile_token_grammar(original)
        local loaded = eval_token_grammar(code)
        local defn = loaded.definitions[1]
        assert.equals("PLUS", defn.name)
        assert.equals("+", defn.pattern)
        assert.is_false(defn.is_regex)
    end)

    it("alias round-trips", function()
        local original = parse_tokens('STRING_DQ = /"[^"]*"/ -> STRING')
        local code = grammar_tools.compile_token_grammar(original)
        local loaded = eval_token_grammar(code)
        assert.equals("STRING", loaded.definitions[1].alias)
    end)

    it("nil alias round-trips", function()
        local original = parse_tokens("NAME = /[a-z]+/")
        local code = grammar_tools.compile_token_grammar(original)
        local loaded = eval_token_grammar(code)
        assert.is_nil(loaded.definitions[1].alias)
    end)

    it("keywords round-trip", function()
        local source = "NAME = /[a-z]+/\nkeywords:\n  if\n  else\n  while\n"
        local original = parse_tokens(source)
        local code = grammar_tools.compile_token_grammar(original)
        local loaded = eval_token_grammar(code)
        assert.same({"if", "else", "while"}, loaded.keywords)
    end)

    it("skip definitions round-trip", function()
        local source = "NAME = /[a-z]+/\nskip:\n  WHITESPACE = /[ \\t]+/\n"
        local original = parse_tokens(source)
        local code = grammar_tools.compile_token_grammar(original)
        local loaded = eval_token_grammar(code)
        assert.equals(1, #loaded.skip_definitions)
        assert.equals("WHITESPACE", loaded.skip_definitions[1].name)
    end)

    it("error definitions round-trip", function()
        local source = 'STRING = /"[^"]*"/\nerrors:\n  BAD = /"[^"\\n]*/\n'
        local original = parse_tokens(source)
        local code = grammar_tools.compile_token_grammar(original)
        local loaded = eval_token_grammar(code)
        assert.equals(1, #loaded.error_definitions)
        assert.equals("BAD", loaded.error_definitions[1].name)
    end)

    it("mode round-trips", function()
        local source = "mode: indentation\nNAME = /[a-z]+/"
        local original = parse_tokens(source)
        local code = grammar_tools.compile_token_grammar(original)
        local loaded = eval_token_grammar(code)
        assert.equals("indentation", loaded.mode)
    end)

    it("escape_mode round-trips", function()
        local source = 'escapes: none\nSTRING = /"[^"]*"/'
        local original = parse_tokens(source)
        local code = grammar_tools.compile_token_grammar(original)
        local loaded = eval_token_grammar(code)
        assert.equals("none", loaded.escape_mode)
    end)

    it("case_insensitive round-trips", function()
        local source = "# @case_insensitive true\nNAME = /[a-z]+/"
        local original = parse_tokens(source)
        local code = grammar_tools.compile_token_grammar(original)
        local loaded = eval_token_grammar(code)
        assert.is_true(loaded.case_insensitive)
    end)

    it("version round-trips", function()
        local source = "# @version 3\nNAME = /[a-z]+/"
        local original = parse_tokens(source)
        local code = grammar_tools.compile_token_grammar(original)
        local loaded = eval_token_grammar(code)
        assert.equals(3, loaded.version)
    end)

    it("pattern groups round-trip", function()
        local source = 'TEXT = /[^<]+/\ngroup tag:\n  ATTR = /[a-z]+/\n  EQ = "="\n'
        local original = parse_tokens(source)
        local code = grammar_tools.compile_token_grammar(original)
        local loaded = eval_token_grammar(code)
        assert.is_not_nil(loaded.groups["tag"])
        assert.equals(2, #loaded.groups["tag"].definitions)
    end)

    it("multiple tokens round-trip", function()
        local source = "STRING = /\"[^\"]*\"/\nNUMBER = /[0-9]+/\nNAME = /[a-z]+/"
        local original = parse_tokens(source)
        local code = grammar_tools.compile_token_grammar(original)
        local loaded = eval_token_grammar(code)
        assert.equals(3, #loaded.definitions)
    end)
end)

-- ===========================================================================
-- compile_parser_grammar — output structure
-- ===========================================================================

describe("compile_parser_grammar output structure", function()
    it("includes DO NOT EDIT header", function()
        local g = grammar_tools.ParserGrammar.new()
        local code = grammar_tools.compile_parser_grammar(g)
        assert.is_truthy(code:find("DO NOT EDIT", 1, true))
    end)

    it("includes parser_grammar function", function()
        local g = grammar_tools.ParserGrammar.new()
        local code = grammar_tools.compile_parser_grammar(g)
        assert.is_truthy(code:find("parser_grammar", 1, true))
    end)

    it("includes require grammar_tools", function()
        local g = grammar_tools.ParserGrammar.new()
        local code = grammar_tools.compile_parser_grammar(g)
        assert.is_truthy(code:find("grammar_tools", 1, true))
    end)

    it("includes source file when given", function()
        local g = grammar_tools.ParserGrammar.new()
        local code = grammar_tools.compile_parser_grammar(g, "json.grammar")
        assert.is_truthy(code:find("json.grammar", 1, true))
    end)
end)

-- ===========================================================================
-- compile_parser_grammar — round-trip tests
-- ===========================================================================

describe("compile_parser_grammar round-trip", function()
    it("empty grammar round-trips", function()
        local original = grammar_tools.ParserGrammar.new()
        local code = grammar_tools.compile_parser_grammar(original)
        local loaded = eval_parser_grammar(code)
        assert.equals(0, loaded.version)
        assert.same({}, loaded.rules)
    end)

    it("rule reference round-trips", function()
        local original = parse_grammar("value = NUMBER ;")
        local code = grammar_tools.compile_parser_grammar(original)
        local loaded = eval_parser_grammar(code)
        assert.equals(1, #loaded.rules)
        local rule = loaded.rules[1]
        assert.equals("value", rule.name)
        assert.equals("rule_reference", rule.body.type)
        assert.equals("NUMBER", rule.body.name)
        assert.is_true(rule.body.is_token)
    end)

    it("alternation round-trips", function()
        local original = parse_grammar("value = A | B | C ;")
        local code = grammar_tools.compile_parser_grammar(original)
        local loaded = eval_parser_grammar(code)
        local body = loaded.rules[1].body
        assert.equals("alternation", body.type)
        assert.equals(3, #body.choices)
    end)

    it("sequence round-trips", function()
        local original = parse_grammar("pair = KEY COLON VALUE ;")
        local code = grammar_tools.compile_parser_grammar(original)
        local loaded = eval_parser_grammar(code)
        assert.equals("sequence", loaded.rules[1].body.type)
    end)

    it("repetition round-trips", function()
        local original = parse_grammar("stmts = { stmt } ;")
        local code = grammar_tools.compile_parser_grammar(original)
        local loaded = eval_parser_grammar(code)
        assert.equals("repetition", loaded.rules[1].body.type)
    end)

    it("optional round-trips", function()
        local original = parse_grammar("expr = NUMBER [ PLUS NUMBER ] ;")
        local code = grammar_tools.compile_parser_grammar(original)
        local loaded = eval_parser_grammar(code)
        local body = loaded.rules[1].body
        assert.equals("sequence", body.type)
        assert.equals("optional", body.elements[2].type)
    end)

    it("literal round-trips", function()
        local original = parse_grammar('start = "hello" ;')
        local code = grammar_tools.compile_parser_grammar(original)
        local loaded = eval_parser_grammar(code)
        local body = loaded.rules[1].body
        assert.equals("literal", body.type)
        assert.equals("hello", body.value)
    end)

    it("group round-trips", function()
        local original = parse_grammar("expr = ( A | B ) ;")
        local code = grammar_tools.compile_parser_grammar(original)
        local loaded = eval_parser_grammar(code)
        assert.equals("group", loaded.rules[1].body.type)
    end)

    it("version round-trips", function()
        local original = parse_grammar("# @version 4\nvalue = NUMBER ;")
        local code = grammar_tools.compile_parser_grammar(original)
        local loaded = eval_parser_grammar(code)
        assert.equals(4, loaded.version)
    end)

    it("line_number preserved in rules", function()
        local original = parse_grammar("value = NUMBER ;")
        local code = grammar_tools.compile_parser_grammar(original)
        local loaded = eval_parser_grammar(code)
        assert.equals(1, loaded.rules[1].line_number)
    end)

    it("JSON grammar full round-trip", function()
        local source = [[
value    = object | array | STRING | NUMBER | TRUE | FALSE | NULL ;
object   = LBRACE [ pair { COMMA pair } ] RBRACE ;
pair     = STRING COLON value ;
array    = LBRACKET [ value { COMMA value } ] RBRACKET ;
]]
        local original = parse_grammar(source)
        local code = grammar_tools.compile_parser_grammar(original, "json.grammar")
        local loaded = eval_parser_grammar(code)
        assert.equals(4, #loaded.rules)
        assert.equals("value", loaded.rules[1].name)
        assert.equals("array", loaded.rules[4].name)
    end)
end)
