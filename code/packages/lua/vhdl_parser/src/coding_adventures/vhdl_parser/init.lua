-- vhdl_parser -- Builds an AST from VHDL source using the grammar-driven engine
-- =================================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- It sits in the hardware-description layer alongside verilog_parser, above the
-- lexer, grammar_tools, and vhdl_lexer packages.
--
-- # What does a VHDL parser do?
--
-- VHDL (VHSIC Hardware Description Language) is an HDL designed for describing,
-- simulating, and synthesizing digital circuits. It takes a fundamentally more
-- explicit and strongly-typed approach than Verilog — closer to Ada than to C.
--
-- A lexer breaks raw VHDL source into a flat token stream:
--
--   'entity and_gate is port(a : in std_logic); end entity;'
--   →  KEYWORD("entity") NAME("and_gate") KEYWORD("is") …
--
-- A parser takes that flat stream and builds a tree capturing the structure:
--
--   design_file
--   └── design_unit
--       └── entity_declaration
--           ├── KEYWORD  "entity"
--           ├── NAME     "and_gate"
--           ├── port_clause
--           │   └── interface_list
--           │       └── interface_element: a : in std_logic
--           └── KEYWORD  "end"
--
-- # VHDL vs Verilog
--
-- VHDL separates interface from implementation:
--
--   entity full_adder is          — INTERFACE: defines ports (pins)
--     port (a, b, cin : in std_logic;
--           sum, cout : out std_logic);
--   end entity full_adder;
--
--   architecture rtl of full_adder is  — IMPLEMENTATION: defines behavior
--     signal carry : std_logic;
--   begin
--     sum  <= a xor b xor cin;         -- concurrent signal assignment
--     cout <= (a and b) or (carry and cin);
--   end architecture rtl;
--
-- This separation allows multiple implementations (behavioral, structural,
-- RTL) for the same entity — useful for verification and optimization.
--
-- # Grammar
--
-- The grammar is defined in `code/grammars/vhdl.grammar`.  It covers the
-- synthesizable subset (IEEE 1076-2008):
--
--   design_file       = { design_unit } ;
--   design_unit       = { context_item } library_unit ;
--   library_unit      = entity_declaration | architecture_body
--                     | package_declaration | package_body ;
--   entity_declaration = "entity" NAME "is"
--                        [generic_clause] [port_clause]
--                        "end" ["entity"] [NAME] SEMICOLON ;
--   architecture_body = "architecture" NAME "of" NAME "is"
--                       { block_declarative_item }
--                       "begin"
--                       { concurrent_statement }
--                       "end" ["architecture"] [NAME] SEMICOLON ;
--   concurrent_statement = process_statement | signal_assignment_concurrent
--                        | component_instantiation | generate_statement ;
--   sequential_statement = signal_assignment_seq | variable_assignment
--                        | if_statement | case_statement | loop_statement
--                        | return_statement | null_statement ;
--   expression         = logical_expr ;
--   … (full expression hierarchy with VHDL operator precedence)
--
-- # VHDL peculiarities
--
-- 1. Case-insensitive: ENTITY, Entity, entity are the same.
--    The lexer normalizes to lowercase before tokenizing.
--
-- 2. The <= operator is overloaded:
--    - As signal assignment:  q <= d;   (statement context)
--    - As less-than-or-equal: a <= b    (expression context)
--    The grammar structure disambiguates: expressions appear inside
--    relational rules, not at statement level.
--
-- 3. Logical operators are keywords (and, or, xor, not) instead of symbols.
--    This makes VHDL code more self-documenting:
--    Verilog: y = (a & b) | (c ^ d)
--    VHDL:    y <= (a and b) or (c xor d);
--
-- 4. Aggregates construct composite values:
--    (others => '0')        — fill signal with zeros
--    (0 => '1', others => '0') — set one bit
--
-- # Architecture
--
-- 1. **Tokenize** — call `vhdl_lexer.tokenize(source)` to get a token list.
-- 2. **Load grammar** — call `grammar_tools.parse_parser_grammar(content)`
--    to get a `ParserGrammar` with `.rules`.
-- 3. **Parse** — construct a `GrammarParser` (from the `parser` package)
--    and call `:parse()`.  The engine interprets the grammar rules against
--    the token stream, producing an AST.
--
-- # Path navigation
--
-- This file lives at:
--   code/packages/lua/vhdl_parser/src/coding_adventures/vhdl_parser/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path (prefixed with "@").
-- Stripping the prefix and walking up 6 levels reaches `code/`, the repo root.
--
-- Directory structure from script_dir upward:
--   vhdl_parser/    (1)  ← this module's dir
--   coding_adventures/  (2)
--   src/                (3)
--   vhdl_parser/        (4)  ← the package directory
--   lua/                (5)
--   packages/           (6)
--   code/               → then /grammars/vhdl.grammar

local grammar_tools = require("coding_adventures.grammar_tools")
local vhdl_lexer    = require("coding_adventures.vhdl_lexer")
local parser_pkg    = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================

--- Return the directory portion of a file path (no trailing slash).
-- Example:  "/a/b/c/init.lua"  →  "/a/b/c"
-- @param path string
-- @return string
local function dirname(path)
    return path:match("(.+)/[^/]+$") or "."
end

--- Return the absolute directory of this source file.
-- Lua prepends "@" to the source path in debug info — we strip it.
-- When busted runs tests with a relative path containing ".." the
-- dirname-only approach produces a path that collapses to "." after
-- up() steps, so the grammar file cannot be found.  We resolve to an
-- absolute path via "cd <dir> && pwd" to give up() an absolute anchor.
-- @return string Absolute directory of this init.lua file.
local function get_script_dir()
    local info = debug.getinfo(1, "S")
    local src  = info.source
    if src:sub(1, 1) == "@" then
        src = src:sub(2)
    end
    -- Normalize Windows backslashes to forward slashes for cross-platform
    -- path handling (on Linux/macOS this is a no-op).
    src = src:gsub("\\", "/")
    local dir = src:match("(.+)/[^/]+$") or "."
    -- Security: Do not attempt shell-based path resolution via io.popen.
    -- Passing unsanitised directory strings to a shell command introduces
    -- OS command injection risk (path could contain single-quotes or shell
    -- metacharacters). String-based path arithmetic in up_n_levels works
    -- correctly for both absolute and relative source paths.
    -- Fixed: 2026-04-10 security review.
    return dir
end

--- Walk up `levels` directory levels from `path`.
-- @param path   string  Starting directory.
-- @param levels number  How many levels to climb.
-- @return string
local function up(path, levels)
    local result = path
    for _ = 1, levels do
        result = dirname(result)
    end
    return result
end

-- =========================================================================
-- Grammar loading
-- =========================================================================

local _grammar_cache = nil

--- Load and parse `vhdl.grammar`, with caching.
-- On the first call, opens the file, parses it with
-- `grammar_tools.parse_parser_grammar`, and caches the result.
-- @return ParserGrammar  The parsed VHDL parser grammar.
-- @error                 Raises an error if the file cannot be opened or parsed.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate: 6 levels up from this file's directory → code/ root.
    local script_dir   = get_script_dir()
    local repo_root    = up(script_dir, 6)
    local grammar_path = repo_root .. "/grammars/vhdl.grammar"

    local f, open_err = io.open(grammar_path, "r")
    if not f then
        error(
            "vhdl_parser: cannot open grammar file: " .. grammar_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error(
            "vhdl_parser: failed to parse vhdl.grammar: " ..
            (parse_err or "unknown error")
        )
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a VHDL source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `vhdl_lexer.tokenize`.
--   2. Loads the VHDL parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "design_file"` (the first rule in
-- the VHDL grammar).
--
-- Example constructs:
--   library IEEE;
--   use IEEE.std_logic_1164.all;
--
--   entity and_gate is
--     port (a, b : in std_logic; y : out std_logic);
--   end entity and_gate;
--
--   architecture rtl of and_gate is
--   begin
--     y <= a and b;
--   end architecture rtl;
--
-- @param source string  The VHDL text to parse.
-- @return ASTNode       Root of the AST.
-- @error                Raises an error on lexer or parser failure.
--
-- Example:
--
--   local vhdl_parser = require("coding_adventures.vhdl_parser")
--   local ast = vhdl_parser.parse([[
--     entity empty is end entity;
--   ]])
--   -- ast.rule_name  → "design_file"
function M.parse(source)
    local tokens = vhdl_lexer.tokenize(source)
    local grammar = get_grammar()
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("vhdl_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a VHDL source string without immediately parsing.
--
-- Use this when you want to control parsing yourself — for example, to
-- use trace mode or to inspect the token stream before parsing.
--
-- @param source string   The VHDL text to tokenize.
-- @return GrammarParser  An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = vhdl_parser.create_parser("entity empty is end entity;")
--   local ast, err = p:parse()
function M.create_parser(source)
    local tokens = vhdl_lexer.tokenize(source)
    local grammar = get_grammar()
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for VHDL.
--
-- Exposed so callers can inspect the grammar rules directly — for example,
-- to enumerate rule names or check the grammar structure.
--
-- @return ParserGrammar  The parsed VHDL parser grammar.
function M.get_grammar()
    return get_grammar()
end

return M
