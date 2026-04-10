-- verilog_parser -- Builds an AST from Verilog source using the grammar-driven engine
-- =====================================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- It sits in the hardware-description layer alongside vhdl_parser, above the
-- lexer, grammar_tools, and verilog_lexer packages.
--
-- # What does a Verilog parser do?
--
-- Verilog is a Hardware Description Language (HDL): programs are not executed
-- by a processor but synthesized into physical circuits — gates, wires, and
-- flip-flops on a chip. A Verilog source file describes HARDWARE, not software.
--
-- A lexer breaks raw Verilog source into a flat token stream:
--
--   'module adder(input a, output y); assign y = a; endmodule'
--   →  KEYWORD("module") NAME("adder") LPAREN("(") …
--
-- A parser takes that flat stream and builds a tree capturing the structure:
--
--   source_text
--   └── description
--       └── module_declaration
--           ├── KEYWORD  "module"
--           ├── NAME     "adder"
--           ├── port_list
--           │   └── port (input a)
--           │   └── port (output y)
--           ├── continuous_assign
--           │   └── assignment: y = a
--           └── KEYWORD  "endmodule"
--
-- # Verilog grammar
--
-- The grammar is defined in `code/grammars/verilog.grammar`.  It covers the
-- synthesizable subset — constructs that map to real digital hardware:
--
--   source_text         = { description } ;
--   description         = module_declaration ;
--   module_declaration  = "module" NAME [parameter_port_list]
--                         [port_list] SEMICOLON
--                         { module_item }
--                         "endmodule" ;
--   module_item         = net_declaration | reg_declaration
--                       | continuous_assign | always_construct
--                       | module_instantiation | … ;
--   continuous_assign   = "assign" assignment { COMMA assignment } SEMICOLON ;
--   always_construct    = "always" AT sensitivity_list statement ;
--   statement           = block_statement | if_statement | case_statement
--                       | for_statement | blocking_assignment SEMICOLON
--                       | nonblocking_assignment SEMICOLON | … ;
--   expression          = ternary_expr ;
--   ternary_expr        = or_expr [ QUESTION expression COLON ternary_expr ] ;
--   … (full expression hierarchy with operator precedence)
--
-- # Two Verilog paradigms
--
-- Structural: Instantiate other modules and connect their ports:
--   adder u1 (.a(sig_a), .b(sig_b), .y(out));
--
-- Behavioral: Describe what the circuit does (synthesis figures out the gates):
--   always @(posedge clk)
--     if (reset) q <= 0;
--     else q <= d;
--
-- # Architecture
--
-- 1. **Tokenize** — call `verilog_lexer.tokenize(source)` to get a token list.
-- 2. **Load grammar** — call `grammar_tools.parse_parser_grammar(content)`
--    to get a `ParserGrammar` with `.rules`.
-- 3. **Parse** — construct a `GrammarParser` (from the `parser` package)
--    and call `:parse()`.  The engine interprets the grammar rules against
--    the token stream, producing an AST.
--
-- # Path navigation
--
-- This file lives at:
--   code/packages/lua/verilog_parser/src/coding_adventures/verilog_parser/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path (prefixed with "@").
-- Stripping the prefix and walking up 6 levels reaches `code/`, the repo root.
--
-- Directory structure from script_dir upward:
--   verilog_parser/  (1)  ← this module's dir
--   coding_adventures/  (2)
--   src/                (3)
--   verilog_parser/     (4)  ← the package directory
--   lua/                (5)
--   packages/           (6)
--   code/               → then /grammars/verilog.grammar

local grammar_tools  = require("coding_adventures.grammar_tools")
local verilog_lexer  = require("coding_adventures.verilog_lexer")
local parser_pkg     = require("coding_adventures.parser")

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
    return dir
end

--- Walk up `levels` directory levels from `path`.
-- @param path   string  Starting directory.
-- @param levels number  How many levels to climb.
-- @return string
local function up(path, levels)
    local result = path
    for _ = 1, levels do
        result = result .. "/.."
    end
    return result
end

-- =========================================================================
-- Grammar loading
-- =========================================================================

local _grammar_cache = nil

--- Load and parse `verilog.grammar`, with caching.
-- On the first call, opens the file, parses it with
-- `grammar_tools.parse_parser_grammar`, and caches the result.
-- @return ParserGrammar  The parsed Verilog parser grammar.
-- @error                 Raises an error if the file cannot be opened or parsed.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate: 6 levels up from this file's directory → code/ root.
    local script_dir   = get_script_dir()
    local repo_root    = up(script_dir, 6)
    local grammar_path = repo_root .. "/grammars/verilog.grammar"

    local f, open_err = io.open(grammar_path, "r")
    if not f then
        error(
            "verilog_parser: cannot open grammar file: " .. grammar_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error(
            "verilog_parser: failed to parse verilog.grammar: " ..
            (parse_err or "unknown error")
        )
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a Verilog source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `verilog_lexer.tokenize`.
--   2. Loads the Verilog parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "source_text"` (the first rule in
-- the Verilog grammar).
--
-- Example constructs:
--   module and_gate(input a, input b, output y);
--     assign y = a & b;
--   endmodule
--
-- @param source string  The Verilog text to parse.
-- @return ASTNode       Root of the AST.
-- @error                Raises an error on lexer or parser failure.
--
-- Example:
--
--   local verilog_parser = require("coding_adventures.verilog_parser")
--   local ast = verilog_parser.parse("module empty; endmodule")
--   -- ast.rule_name  → "source_text"
function M.parse(source)
    local tokens = verilog_lexer.tokenize(source)
    local grammar = get_grammar()
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("verilog_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a Verilog source string without immediately parsing.
--
-- Use this when you want to control parsing yourself — for example, to
-- use trace mode or to inspect the token stream before parsing.
--
-- @param source string   The Verilog text to tokenize.
-- @return GrammarParser  An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = verilog_parser.create_parser("module empty; endmodule")
--   local ast, err = p:parse()
function M.create_parser(source)
    local tokens = verilog_lexer.tokenize(source)
    local grammar = get_grammar()
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for Verilog.
--
-- Exposed so callers can inspect the grammar rules directly — for example,
-- to enumerate rule names or check the grammar structure.
--
-- @return ParserGrammar  The parsed Verilog parser grammar.
function M.get_grammar()
    return get_grammar()
end

return M
