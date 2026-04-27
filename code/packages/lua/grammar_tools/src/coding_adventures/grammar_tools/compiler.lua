-- compiler.lua — compile TokenGrammar and ParserGrammar to Lua source code
-- ============================================================================
--
-- The grammar-tools library parses .tokens and .grammar files into in-memory
-- Lua tables. This module adds the *compile* step: given a parsed grammar
-- object, generate Lua source code that constructs the grammar as native Lua
-- data — no file I/O or parsing at runtime.
--
-- ## Why compile grammars?
--
-- The default workflow reads .tokens and .grammar files at startup.  This has
-- three costs that compilation eliminates:
--
--   1. File I/O at startup — every process must find and open the files.
--      Packages walk up the directory tree to find code/grammars/, coupling
--      them to the repo layout.
--
--   2. Parse overhead at startup — the grammar is re-parsed every run.
--
--   3. Deployment coupling — .tokens and .grammar files must ship alongside
--      the program script.
--
-- ## Generated output shape (json.tokens → json_tokens.lua)
--
--   -- AUTO-GENERATED FILE — DO NOT EDIT
--   -- Source: json.tokens
--   local gt = require("coding_adventures.grammar_tools")
--
--   local function token_grammar()
--     local g = gt.TokenGrammar.new()
--     g.definitions = {
--       { name="STRING", pattern=[["[^"]*"]], is_regex=true, line_number=1, alias=nil },
--     }
--     g.keywords = {}
--     ...
--     return g
--   end
--
--   return { token_grammar = token_grammar }
--
-- ## Design notes
--
-- - Lua's `string.format("%q", s)` produces properly-escaped double-quoted
--   string literals and is used everywhere strings need escaping.
-- - Grammar elements are rendered as plain Lua tables with a "type" field,
--   matching the structure produced by the parser.
-- - The generated file returns a module table with a single function.  The
--   caller `require`s the file and calls `mod.token_grammar()`.

local M = {}

-- ===========================================================================
-- Public API
-- ===========================================================================

--- Generate Lua source code embedding a TokenGrammar as native data.
--
-- @param grammar     TokenGrammar instance to compile
-- @param source_file (optional) original .tokens filename for the header comment
-- @return string of valid Lua source code
function M.compile_token_grammar(grammar, source_file)
    source_file = source_file or ""
    -- Strip newlines so a crafted filename cannot break out of the comment line
    -- and inject arbitrary code into the generated file.
    source_file = source_file:gsub("[\r\n]", "_")
    local source_line = source_file ~= "" and ("-- Source: " .. source_file .. "\n") or ""

    local defs_src = token_def_list_src(grammar.definitions or {}, "      ")
    local skip_src = token_def_list_src(grammar.skip_definitions or {}, "      ")
    local err_src  = token_def_list_src(grammar.error_definitions or {}, "      ")
    local groups_src = groups_src_fn(grammar.groups or {}, "      ")

    local lines = {
        "-- AUTO-GENERATED FILE \xe2\x80\x94 DO NOT EDIT",
        source_line .. "-- Regenerate with: grammar-tools compile-tokens " .. source_file,
        "--",
        "-- This file embeds a TokenGrammar as native Lua data structures.",
        "-- Call token_grammar() instead of reading and parsing the .tokens file.",
        "",
        'local gt = require("coding_adventures.grammar_tools")',
        "",
        "local function token_grammar()",
        "  local g = gt.TokenGrammar.new()",
        "  g.definitions = " .. defs_src,
        "  g.keywords = " .. lua_string_list(grammar.keywords or {}),
        "  g.mode = " .. lua_opt_string(grammar.mode),
        "  g.escape_mode = " .. lua_opt_string(grammar.escape_mode),
        "  g.skip_definitions = " .. skip_src,
        "  g.reserved_keywords = " .. lua_string_list(grammar.reserved_keywords or {}),
        "  g.context_keywords = " .. lua_string_list(grammar.context_keywords or {}),
        "  g.layout_keywords = " .. lua_string_list(grammar.layout_keywords or {}),
        "  g.soft_keywords = " .. lua_string_list(grammar.soft_keywords or {}),
        "  g.error_definitions = " .. err_src,
        "  g.groups = " .. groups_src,
        "  g.case_sensitive = " .. tostring(grammar.case_sensitive ~= false),
        "  g.version = " .. tostring(grammar.version or 0),
        "  g.case_insensitive = " .. tostring(grammar.case_insensitive == true),
        "  return g",
        "end",
        "",
        "return { token_grammar = token_grammar }",
        "",
    }
    return table.concat(lines, "\n")
end

--- Generate Lua source code embedding a ParserGrammar as native data.
--
-- @param grammar     ParserGrammar instance to compile
-- @param source_file (optional) original .grammar filename for the header comment
-- @return string of valid Lua source code
function M.compile_parser_grammar(grammar, source_file)
    source_file = source_file or ""
    -- Strip newlines so a crafted filename cannot break out of the comment line.
    source_file = source_file:gsub("[\r\n]", "_")
    local source_line = source_file ~= "" and ("-- Source: " .. source_file .. "\n") or ""

    local rules_src
    if not grammar.rules or #grammar.rules == 0 then
        rules_src = "{}"
    else
        local rule_lines = {}
        for _, rule in ipairs(grammar.rules) do
            rule_lines[#rule_lines + 1] = grammar_rule_src(rule, "    ")
        end
        rules_src = "{\n" .. table.concat(rule_lines, ",\n") .. ",\n  }"
    end

    local lines = {
        "-- AUTO-GENERATED FILE \xe2\x80\x94 DO NOT EDIT",
        source_line .. "-- Regenerate with: grammar-tools compile-grammar " .. source_file,
        "--",
        "-- This file embeds a ParserGrammar as native Lua data structures.",
        "-- Call parser_grammar() instead of reading and parsing the .grammar file.",
        "",
        'local gt = require("coding_adventures.grammar_tools")',
        "",
        "local function parser_grammar()",
        "  local g = gt.ParserGrammar.new()",
        "  g.rules = " .. rules_src,
        "  g.version = " .. tostring(grammar.version or 0),
        "  return g",
        "end",
        "",
        "return { parser_grammar = parser_grammar }",
        "",
    }
    return table.concat(lines, "\n")
end

-- ===========================================================================
-- Token grammar helpers
-- ===========================================================================

--- Render a Lua string value as a quoted Lua string literal.
-- Uses string.format("%q") for proper escaping.
local function lua_string(s)
    return string.format("%q", s)
end

--- Render an optional string (nil or string) as a Lua expression.
function lua_opt_string(s)
    if s == nil then return "nil" end
    return lua_string(s)
end

--- Render a list of strings as a Lua table constructor.
function lua_string_list(list)
    if #list == 0 then return "{}" end
    local items = {}
    for _, s in ipairs(list) do
        items[#items + 1] = lua_string(s)
    end
    return "{" .. table.concat(items, ", ") .. "}"
end

--- Render one TokenDefinition as a Lua table constructor.
local function token_def_src(defn, indent)
    local alias_val = defn.alias ~= nil and lua_string(defn.alias) or "nil"
    return indent .. "{\n"
        .. indent .. "  name=" .. lua_string(defn.name) .. ",\n"
        .. indent .. "  pattern=" .. lua_string(defn.pattern) .. ",\n"
        .. indent .. "  is_regex=" .. tostring(defn.is_regex) .. ",\n"
        .. indent .. "  line_number=" .. tostring(defn.line_number) .. ",\n"
        .. indent .. "  alias=" .. alias_val .. ",\n"
        .. indent .. "}"
end

--- Render a list of TokenDefinitions as a Lua table constructor.
function token_def_list_src(defs, indent)
    if #defs == 0 then return "{}" end
    local inner = indent .. "  "
    local items = {}
    for _, d in ipairs(defs) do
        items[#items + 1] = token_def_src(d, inner)
    end
    return "{\n" .. table.concat(items, ",\n") .. ",\n" .. indent .. "}"
end

--- Render the groups table as a Lua table constructor.
function groups_src_fn(groups, indent)
    -- Collect keys for deterministic output.
    local keys = {}
    for k in pairs(groups) do keys[#keys + 1] = k end
    if #keys == 0 then return "{}" end
    table.sort(keys)

    local inner = indent .. "  "
    local inner2 = inner .. "  "
    local entries = {}
    for _, name in ipairs(keys) do
        local group = groups[name]
        local defs_lit = token_def_list_src(group.definitions or {}, inner2 .. "  ")
        entries[#entries + 1] = inner .. "[" .. lua_string(name) .. "] = {\n"
            .. inner2 .. "name=" .. lua_string(group.name) .. ",\n"
            .. inner2 .. "definitions=" .. defs_lit .. ",\n"
            .. inner .. "}"
    end
    return "{\n" .. table.concat(entries, ",\n") .. ",\n" .. indent .. "}"
end

-- ===========================================================================
-- Parser grammar helpers
-- ===========================================================================

--- Render one grammar rule as a Lua table constructor.
function grammar_rule_src(rule, indent)
    local i = indent .. "  "
    local body_src = element_src(rule.body, i)
    return indent .. "{\n"
        .. i .. "name=" .. lua_string(rule.name) .. ",\n"
        .. i .. "body=" .. body_src .. ",\n"
        .. i .. "line_number=" .. tostring(rule.line_number) .. ",\n"
        .. indent .. "}"
end

--- Recursively render a grammar element as a Lua table constructor.
--
-- Grammar elements are plain tables with a "type" field:
--   { type="rule_reference", name="expr", is_token=false }
--   { type="sequence", elements={...} }
-- etc.
function element_src(element, indent)
    local i = indent .. "  "
    local t = element.type

    if t == "rule_reference" then
        return '{ type="rule_reference", name=' .. lua_string(element.name)
            .. ', is_token=' .. tostring(element.is_token) .. ' }'

    elseif t == "literal" then
        return '{ type="literal", value=' .. lua_string(element.value) .. ' }'

    elseif t == "sequence" then
        local items = {}
        for _, e in ipairs(element.elements) do
            items[#items + 1] = i .. element_src(e, i)
        end
        return '{ type="sequence", elements={\n'
            .. table.concat(items, ",\n") .. ",\n" .. indent .. "} }"

    elseif t == "alternation" then
        local items = {}
        for _, c in ipairs(element.choices) do
            items[#items + 1] = i .. element_src(c, i)
        end
        return '{ type="alternation", choices={\n'
            .. table.concat(items, ",\n") .. ",\n" .. indent .. "} }"

    elseif t == "repetition" then
        return '{ type="repetition", element=' .. element_src(element.element, i) .. ' }'

    elseif t == "optional" then
        return '{ type="optional", element=' .. element_src(element.element, i) .. ' }'

    elseif t == "group" then
        return '{ type="group", element=' .. element_src(element.element, i) .. ' }'

    else
        error("Unknown grammar element type: " .. tostring(t))
    end
end

return M
