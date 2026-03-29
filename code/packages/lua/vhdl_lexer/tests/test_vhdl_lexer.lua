-- Tests for vhdl_lexer
-- =====================
--
-- Comprehensive busted test suite for the VHDL lexer package.
--
-- VHDL (IEEE 1076-2008) is a Hardware Description Language designed by the US
-- Department of Defense. Unlike Verilog (C-like), VHDL is Ada-like: verbose,
-- strongly typed, and case-insensitive. The lexer lowercases all input before
-- matching, so ENTITY, Entity, and entity all produce the same token.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty and whitespace-only input produces only EOF
--   - Structure keywords: entity, architecture, is, of, begin, end, port,
--     generic, component, package, use, library
--   - Type/signal keywords: signal, variable, constant, type, subtype,
--     in, out, inout, buffer
--   - Flow keywords: if, elsif, else, case, when, others, for, while, loop,
--     process, wait, then
--   - Operator keywords: and, or, not, nand, nor, xor, xnor
--   - Two-char operators: <=, :=, =>, /=, **
--   - Single-char operators: +, -, *, /, &, <, >, =
--   - Literals: integers, bit strings (X"FF", B"1010"), strings, char literals
--   - Comments: -- line comment
--   - Identifiers (case-insensitive in VHDL)
--   - Token positions (line, col) are tracked correctly

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

local vh = require("coding_adventures.vhdl_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by vh.tokenize.
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
-- @param tokens  table  The token list returned by vh.tokenize.
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

describe("vhdl_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(vh)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(vh.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", vh.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(vh.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(vh.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = vh.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = vh.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input produces only EOF", function()
        local tokens = vh.tokenize("   \t  \n  ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("VHDL line comment only produces EOF", function()
        -- VHDL comments start with -- (two dashes)
        -- This differs from Verilog which uses // (C-style)
        local tokens = vh.tokenize("-- this is a VHDL comment")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Structure keywords
-- =========================================================================
--
-- VHDL programs are organized into design units: entity declarations
-- (the interface) and architecture bodies (the implementation).
--
-- Comparison with Verilog:
--   VHDL entity    ≈ Verilog module (port declarations)
--   VHDL architecture ≈ Verilog module body (logic)

describe("structure keywords", function()
    it("tokenizes entity", function()
        -- `entity` declares the external interface of a hardware component
        local tokens = vh.tokenize("entity")
        assert.are.equal("ENTITY", tokens[1].type)
        assert.are.equal("entity", tokens[1].value)
    end)

    it("tokenizes architecture", function()
        -- `architecture` describes the internal implementation
        -- One entity can have multiple architectures (behavioral, structural, RTL)
        local tokens = vh.tokenize("architecture")
        assert.are.equal("ARCHITECTURE", tokens[1].type)
        assert.are.equal("architecture", tokens[1].value)
    end)

    it("tokenizes is", function()
        -- `is` connects a name to its definition: "entity adder is"
        local tokens = vh.tokenize("is")
        assert.are.equal("IS", tokens[1].type)
        assert.are.equal("is", tokens[1].value)
    end)

    it("tokenizes of", function()
        -- `of` connects an architecture to its entity:
        -- "architecture rtl of adder is"
        local tokens = vh.tokenize("of")
        assert.are.equal("OF", tokens[1].type)
        assert.are.equal("of", tokens[1].value)
    end)

    it("tokenizes begin", function()
        -- `begin` starts the concurrent statement region of an architecture
        local tokens = vh.tokenize("begin")
        assert.are.equal("BEGIN", tokens[1].type)
        assert.are.equal("begin", tokens[1].value)
    end)

    it("tokenizes end", function()
        -- `end` closes entity, architecture, process, or other block
        local tokens = vh.tokenize("end")
        assert.are.equal("END", tokens[1].type)
        assert.are.equal("end", tokens[1].value)
    end)

    it("tokenizes port", function()
        -- `port` introduces the port map: the list of input/output signals
        local tokens = vh.tokenize("port")
        assert.are.equal("PORT", tokens[1].type)
        assert.are.equal("port", tokens[1].value)
    end)

    it("tokenizes generic", function()
        -- `generic` introduces parameters that customize the component.
        -- Like Verilog's `parameter` but in a separate generic map.
        local tokens = vh.tokenize("generic")
        assert.are.equal("GENERIC", tokens[1].type)
        assert.are.equal("generic", tokens[1].value)
    end)

    it("tokenizes component", function()
        -- `component` declares a lower-level module for instantiation.
        -- VHDL requires explicit component declaration before use.
        local tokens = vh.tokenize("component")
        assert.are.equal("COMPONENT", tokens[1].type)
        assert.are.equal("component", tokens[1].value)
    end)

    it("tokenizes package", function()
        -- `package` groups declarations for reuse across design units.
        -- Like a header file in C or a module in Python.
        local tokens = vh.tokenize("package")
        assert.are.equal("PACKAGE", tokens[1].type)
        assert.are.equal("package", tokens[1].value)
    end)

    it("tokenizes use", function()
        -- `use` imports declarations from a package:
        -- use ieee.std_logic_1164.all;
        local tokens = vh.tokenize("use")
        assert.are.equal("USE", tokens[1].type)
        assert.are.equal("use", tokens[1].value)
    end)

    it("tokenizes library", function()
        -- `library` names a design library: library ieee;
        local tokens = vh.tokenize("library")
        assert.are.equal("LIBRARY", tokens[1].type)
        assert.are.equal("library", tokens[1].value)
    end)
end)

-- =========================================================================
-- Type and signal declaration keywords
-- =========================================================================
--
-- VHDL is strongly typed. Every signal, variable, and constant must be
-- declared with an explicit type before use. This verbosity helps catch
-- errors at compile time rather than in simulation.

describe("type and signal declaration keywords", function()
    it("tokenizes signal", function()
        -- `signal` declares a hardware net (like Verilog's wire/reg):
        -- signal clk : std_logic;
        local tokens = vh.tokenize("signal")
        assert.are.equal("SIGNAL", tokens[1].type)
        assert.are.equal("signal", tokens[1].value)
    end)

    it("tokenizes variable", function()
        -- `variable` declares a software variable (only within processes).
        -- Unlike signals, variables update immediately (no delta cycle delay).
        local tokens = vh.tokenize("variable")
        assert.are.equal("VARIABLE", tokens[1].type)
        assert.are.equal("variable", tokens[1].value)
    end)

    it("tokenizes constant", function()
        -- `constant` declares an immutable value: constant WIDTH : integer := 8;
        local tokens = vh.tokenize("constant")
        assert.are.equal("CONSTANT", tokens[1].type)
        assert.are.equal("constant", tokens[1].value)
    end)

    it("tokenizes type", function()
        -- `type` declares a new type: type state_t is (IDLE, FETCH, EXEC);
        local tokens = vh.tokenize("type")
        assert.are.equal("TYPE", tokens[1].type)
        assert.are.equal("type", tokens[1].value)
    end)

    it("tokenizes subtype", function()
        -- `subtype` constrains an existing type:
        -- subtype byte is integer range 0 to 255;
        local tokens = vh.tokenize("subtype")
        assert.are.equal("SUBTYPE", tokens[1].type)
        assert.are.equal("subtype", tokens[1].value)
    end)

    it("tokenizes in", function()
        -- Port direction: signal enters the component from outside
        local tokens = vh.tokenize("in")
        assert.are.equal("IN", tokens[1].type)
        assert.are.equal("in", tokens[1].value)
    end)

    it("tokenizes out", function()
        -- Port direction: signal exits the component to the outside
        local tokens = vh.tokenize("out")
        assert.are.equal("OUT", tokens[1].type)
        assert.are.equal("out", tokens[1].value)
    end)

    it("tokenizes inout", function()
        -- Bidirectional port: can be read and driven from both sides
        local tokens = vh.tokenize("inout")
        assert.are.equal("INOUT", tokens[1].type)
        assert.are.equal("inout", tokens[1].value)
    end)

    it("tokenizes buffer", function()
        -- Like `out` but readable from within the entity.
        -- Useful when the output is also fed back as an input.
        local tokens = vh.tokenize("buffer")
        assert.are.equal("BUFFER", tokens[1].type)
        assert.are.equal("buffer", tokens[1].value)
    end)
end)

-- =========================================================================
-- Control flow keywords
-- =========================================================================
--
-- VHDL control flow appears inside `process` blocks (sequential regions).
-- Outside processes, only concurrent statements are allowed.

describe("control flow keywords", function()
    it("tokenizes if", function()
        local tokens = vh.tokenize("if")
        assert.are.equal("IF", tokens[1].type)
        assert.are.equal("if", tokens[1].value)
    end)

    it("tokenizes elsif", function()
        -- VHDL uses `elsif` (one word), like Ruby but unlike C's `else if`
        local tokens = vh.tokenize("elsif")
        assert.are.equal("ELSIF", tokens[1].type)
        assert.are.equal("elsif", tokens[1].value)
    end)

    it("tokenizes else", function()
        local tokens = vh.tokenize("else")
        assert.are.equal("ELSE", tokens[1].type)
        assert.are.equal("else", tokens[1].value)
    end)

    it("tokenizes then", function()
        -- VHDL `if` syntax: if condition then ... end if;
        local tokens = vh.tokenize("then")
        assert.are.equal("THEN", tokens[1].type)
        assert.are.equal("then", tokens[1].value)
    end)

    it("tokenizes case", function()
        -- `case` in VHDL selects from multiple alternatives.
        -- Each alternative is introduced by `when`.
        local tokens = vh.tokenize("case")
        assert.are.equal("CASE", tokens[1].type)
        assert.are.equal("case", tokens[1].value)
    end)

    it("tokenizes when", function()
        -- `when` introduces a case alternative: when "01" => ...
        local tokens = vh.tokenize("when")
        assert.are.equal("WHEN", tokens[1].type)
        assert.are.equal("when", tokens[1].value)
    end)

    it("tokenizes others", function()
        -- `others` is the default case: when others => ...
        -- Like `default` in C switch or `else` in Python match.
        local tokens = vh.tokenize("others")
        assert.are.equal("OTHERS", tokens[1].type)
        assert.are.equal("others", tokens[1].value)
    end)

    it("tokenizes for", function()
        -- `for` loop: for i in 0 to 7 loop ... end loop;
        local tokens = vh.tokenize("for")
        assert.are.equal("FOR", tokens[1].type)
        assert.are.equal("for", tokens[1].value)
    end)

    it("tokenizes while", function()
        -- `while` loop: while condition loop ... end loop;
        local tokens = vh.tokenize("while")
        assert.are.equal("WHILE", tokens[1].type)
        assert.are.equal("while", tokens[1].value)
    end)

    it("tokenizes loop", function()
        -- `loop` is the keyword that follows `for` or `while`
        local tokens = vh.tokenize("loop")
        assert.are.equal("LOOP", tokens[1].type)
        assert.are.equal("loop", tokens[1].value)
    end)

    it("tokenizes process", function()
        -- `process` introduces a sequential block, triggered by its sensitivity
        -- list: process(clk) models a flip-flop. Inside, events are sequential.
        local tokens = vh.tokenize("process")
        assert.are.equal("PROCESS", tokens[1].type)
        assert.are.equal("process", tokens[1].value)
    end)

    it("tokenizes wait", function()
        -- `wait` suspends a process until a condition:
        -- wait until rising_edge(clk);
        local tokens = vh.tokenize("wait")
        assert.are.equal("WAIT", tokens[1].type)
        assert.are.equal("wait", tokens[1].value)
    end)
end)

-- =========================================================================
-- Operator keywords
-- =========================================================================
--
-- VHDL uses English-word operators for logical operations, unlike Verilog's
-- symbol operators. This is part of VHDL's Ada heritage.
--
-- Compare:
--   Verilog: assign y = (a & b) | (c ^ d);
--   VHDL:    y <= (a and b) or (c xor d);

describe("operator keywords", function()
    it("tokenizes and", function()
        local tokens = vh.tokenize("and")
        assert.are.equal("AND", tokens[1].type)
        assert.are.equal("and", tokens[1].value)
    end)

    it("tokenizes or", function()
        local tokens = vh.tokenize("or")
        assert.are.equal("OR", tokens[1].type)
        assert.are.equal("or", tokens[1].value)
    end)

    it("tokenizes not", function()
        local tokens = vh.tokenize("not")
        assert.are.equal("NOT", tokens[1].type)
        assert.are.equal("not", tokens[1].value)
    end)

    it("tokenizes nand", function()
        local tokens = vh.tokenize("nand")
        assert.are.equal("NAND", tokens[1].type)
        assert.are.equal("nand", tokens[1].value)
    end)

    it("tokenizes nor", function()
        local tokens = vh.tokenize("nor")
        assert.are.equal("NOR", tokens[1].type)
        assert.are.equal("nor", tokens[1].value)
    end)

    it("tokenizes xor", function()
        local tokens = vh.tokenize("xor")
        assert.are.equal("XOR", tokens[1].type)
        assert.are.equal("xor", tokens[1].value)
    end)

    it("tokenizes xnor", function()
        local tokens = vh.tokenize("xnor")
        assert.are.equal("XNOR", tokens[1].type)
        assert.are.equal("xnor", tokens[1].value)
    end)
end)

-- =========================================================================
-- Operator tokens (symbols)
-- =========================================================================
--
-- VHDL operator table vs Verilog vs C:
--
--   Meaning     | VHDL    | Verilog | C
--   ------------|---------|---------|-----
--   Signal assign | <=    | <=      | (n/a)
--   Var assign  | :=      | =       | =
--   Not equal   | /=      | !=      | !=
--   Arrow/map   | =>      | (none)  | (none)
--   Concatenate | &       | {}      | (n/a)

describe("operator tokens", function()
    it("tokenizes <= (signal assignment)", function()
        -- `<=` is VHDL's signal assignment operator. It also means
        -- less-than-or-equal in comparisons — context resolves the ambiguity.
        local tokens = vh.tokenize("<=")
        assert.are.equal("LESS_EQUALS", tokens[1].type)
        assert.are.equal("<=", tokens[1].value)
    end)

    it("tokenizes := (variable assignment)", function()
        -- `:=` assigns to variables and constants (not signals).
        -- Variables update immediately; signals update at the next delta cycle.
        local tokens = vh.tokenize(":=")
        assert.are.equal("VAR_ASSIGN", tokens[1].type)
        assert.are.equal(":=", tokens[1].value)
    end)

    it("tokenizes = (equality comparison)", function()
        -- `=` is equality comparison (not assignment for signals)
        local tokens = vh.tokenize("=")
        assert.are.equal("EQUALS", tokens[1].type)
        assert.are.equal("=", tokens[1].value)
    end)

    it("tokenizes /= (not equals)", function()
        -- VHDL uses /= for inequality (not != like C or Verilog)
        local tokens = vh.tokenize("/=")
        assert.are.equal("NOT_EQUALS", tokens[1].type)
        assert.are.equal("/=", tokens[1].value)
    end)

    it("tokenizes < (less than)", function()
        local tokens = vh.tokenize("<")
        assert.are.equal("LESS_THAN", tokens[1].type)
        assert.are.equal("<", tokens[1].value)
    end)

    it("tokenizes > (greater than)", function()
        local tokens = vh.tokenize(">")
        assert.are.equal("GREATER_THAN", tokens[1].type)
        assert.are.equal(">", tokens[1].value)
    end)

    it("tokenizes >= (greater-equals)", function()
        local tokens = vh.tokenize(">=")
        assert.are.equal("GREATER_EQUALS", tokens[1].type)
        assert.are.equal(">=", tokens[1].value)
    end)

    it("tokenizes => (named association arrow)", function()
        -- => is used in port maps and aggregate expressions:
        -- port map (clk => system_clock, rst => reset)
        local tokens = vh.tokenize("=>")
        assert.are.equal("ARROW", tokens[1].type)
        assert.are.equal("=>", tokens[1].value)
    end)

    it("tokenizes ** (power)", function()
        local tokens = vh.tokenize("**")
        assert.are.equal("POWER", tokens[1].type)
        assert.are.equal("**", tokens[1].value)
    end)

    it("tokenizes + (plus)", function()
        local tokens = vh.tokenize("+")
        assert.are.equal("PLUS", tokens[1].type)
        assert.are.equal("+", tokens[1].value)
    end)

    it("tokenizes - (minus)", function()
        local tokens = vh.tokenize("-")
        assert.are.equal("MINUS", tokens[1].type)
        assert.are.equal("-", tokens[1].value)
    end)

    it("tokenizes * (multiply)", function()
        local tokens = vh.tokenize("*")
        assert.are.equal("STAR", tokens[1].type)
        assert.are.equal("*", tokens[1].value)
    end)

    it("tokenizes / (divide)", function()
        local tokens = vh.tokenize("/")
        assert.are.equal("SLASH", tokens[1].type)
        assert.are.equal("/", tokens[1].value)
    end)

    it("tokenizes & (concatenation)", function()
        -- In VHDL, & is concatenation, NOT bitwise AND.
        -- Bitwise AND is the keyword `and`.
        -- "Hello" & " World" → "Hello World"
        local tokens = vh.tokenize("&")
        assert.are.equal("AMPERSAND", tokens[1].type)
        assert.are.equal("&", tokens[1].value)
    end)
end)

-- =========================================================================
-- Literals
-- =========================================================================
--
-- VHDL literals are richer than Verilog's:
--   - Integers: 42, 1_000_000 (underscore as visual separator)
--   - Bit strings: X"FF" (hex), B"1010" (binary), O"77" (octal)
--   - Based literals: 16#FF#, 2#1010# (any base 2-16)
--   - Strings: "hello" (use "" for embedded quote: "He said ""hi""")
--   - Char literals: '0' '1' 'X' 'Z' — values of std_logic

describe("number literals", function()
    it("tokenizes a plain integer", function()
        local tokens = vh.tokenize("42")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("42", tokens[1].value)
    end)

    it("tokenizes an integer with underscore separator", function()
        -- Underscores are visual separators: 1_000_000 = 1000000
        local tokens = vh.tokenize("1_000")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("1_000", tokens[1].value)
    end)
end)

describe("bit string literals", function()
    it("tokenizes a hex bit string: X\"FF\"", function()
        -- X"FF" = 8 bits with value 255
        -- This is VHDL's equivalent of Verilog's 8'hFF
        local tokens = vh.tokenize('X"FF"')
        assert.are.equal("BIT_STRING", tokens[1].type)
        -- Note: case_sensitive:false lowercases everything
        assert.matches("x", tokens[1].value)
    end)

    it("tokenizes a binary bit string: B\"1010\"", function()
        -- B"1010" = 4 bits: 1, 0, 1, 0
        local tokens = vh.tokenize('B"1010"')
        assert.are.equal("BIT_STRING", tokens[1].type)
    end)

    it("tokenizes an octal bit string: O\"77\"", function()
        -- O"77" = 6 bits (77 octal = 63 decimal)
        local tokens = vh.tokenize('O"77"')
        assert.are.equal("BIT_STRING", tokens[1].type)
    end)
end)

describe("string literals", function()
    it("tokenizes a double-quoted string", function()
        -- VHDL strings use "" to escape a quote inside: "He said ""hi"""
        local tokens = vh.tokenize('"hello"')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('"hello"', tokens[1].value)
    end)

    it("tokenizes an empty string", function()
        local tokens = vh.tokenize('""')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('""', tokens[1].value)
    end)
end)

describe("character literals", function()
    it("tokenizes a std_logic '0' character", function()
        -- '0' represents logic low in VHDL std_logic type
        local tokens = vh.tokenize("'0'")
        assert.are.equal("CHAR_LITERAL", tokens[1].type)
        assert.are.equal("'0'", tokens[1].value)
    end)

    it("tokenizes a std_logic '1' character", function()
        local tokens = vh.tokenize("'1'")
        assert.are.equal("CHAR_LITERAL", tokens[1].type)
        assert.are.equal("'1'", tokens[1].value)
    end)
end)

-- =========================================================================
-- Comments
-- =========================================================================
--
-- VHDL uses only single-line comments starting with -- (two dashes).
-- There are no block comments in standard VHDL (VHDL-2008 added /* */ but
-- this grammar targets the core language).

describe("comments", function()
    it("skips a line comment", function()
        -- Everything after -- to end of line is discarded
        local tokens = vh.tokenize("signal clk : std_logic; -- clock input")
        local t = types(tokens)
        -- signal, clk, colon, std_logic, semicolon — no comment token
        assert.are.same({"SIGNAL", "NAME", "COLON", "NAME", "SEMICOLON"}, t)
    end)

    it("skips a comment on its own line", function()
        local tokens = vh.tokenize("-- Full line comment\nentity foo is")
        local first_entity = first_of(tokens, "ENTITY")
        assert.is_not_nil(first_entity)
        assert.are.equal("entity", first_entity.value)
    end)
end)

-- =========================================================================
-- Case insensitivity
-- =========================================================================
--
-- VHDL is unique among HDLs in being case-insensitive. ENTITY, Entity, and
-- entity are identical identifiers. The grammar sets case_sensitive: false,
-- which causes the lexer to lowercase everything before matching.

describe("case insensitivity", function()
    it("tokenizes ENTITY (uppercase) as the entity keyword", function()
        local tokens = vh.tokenize("ENTITY")
        assert.are.equal("ENTITY", tokens[1].type)
        -- Value should be lowercased by the grammar's case_sensitive: false
        assert.are.equal("entity", tokens[1].value)
    end)

    it("tokenizes Architecture (mixed case) as the architecture keyword", function()
        local tokens = vh.tokenize("Architecture")
        assert.are.equal("ARCHITECTURE", tokens[1].type)
        assert.are.equal("architecture", tokens[1].value)
    end)

    it("treats SIGNAL and signal as the same", function()
        local tok1 = vh.tokenize("SIGNAL")[1]
        local tok2 = vh.tokenize("signal")[1]
        assert.are.equal(tok1.type, tok2.type)
        assert.are.equal(tok1.value, tok2.value)
    end)
end)

-- =========================================================================
-- Identifiers
-- =========================================================================

describe("identifier tokens", function()
    it("tokenizes a simple identifier", function()
        local tokens = vh.tokenize("data_bus")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("data_bus", tokens[1].value)
    end)

    it("tokenizes an identifier with digits", function()
        local tokens = vh.tokenize("clk32")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("clk32", tokens[1].value)
    end)

    it("does not classify non-keyword names as keywords", function()
        local tokens = vh.tokenize("std_logic")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("std_logic", tokens[1].value)
    end)
end)

-- =========================================================================
-- Composite expressions
-- =========================================================================

describe("composite expressions", function()
    it("tokenizes entity declaration header", function()
        -- entity adder is
        local tokens = vh.tokenize("entity adder is")
        local t = types(tokens)
        assert.are.same({"ENTITY", "NAME", "IS"}, t)
        assert.are.equal("adder", tokens[2].value)
    end)

    it("tokenizes architecture header", function()
        -- architecture rtl of adder is
        local tokens = vh.tokenize("architecture rtl of adder is")
        local t = types(tokens)
        assert.are.same({"ARCHITECTURE", "NAME", "OF", "NAME", "IS"}, t)
    end)

    it("tokenizes port declaration", function()
        -- port ( clk : in std_logic )
        local tokens = vh.tokenize("port ( clk : in std_logic )")
        local t = types(tokens)
        assert.are.same({
            "PORT", "LPAREN", "NAME", "COLON", "IN", "NAME", "RPAREN"
        }, t)
    end)

    it("tokenizes signal assignment: y <= a and b", function()
        -- Signal assignment uses <= (the non-blocking equivalent of Verilog)
        -- Here, `and` is a keyword operator, not a symbol.
        local tokens = vh.tokenize("y <= a and b")
        local t = types(tokens)
        assert.are.same({"NAME", "LESS_EQUALS", "NAME", "AND", "NAME"}, t)
    end)

    it("tokenizes variable assignment: count := count + 1", function()
        -- Variable assignment inside a process uses :=
        local tokens = vh.tokenize("count := count + 1")
        local t = types(tokens)
        assert.are.same({"NAME", "VAR_ASSIGN", "NAME", "PLUS", "NUMBER"}, t)
    end)

    it("tokenizes if/elsif/else keywords in sequence", function()
        local tokens = vh.tokenize("if x = 1 then y := 0; elsif x = 2 then y := 1; else y := 2; end if;")
        local first_if   = first_of(tokens, "IF")
        local first_elsif = first_of(tokens, "ELSIF")
        local first_else = first_of(tokens, "ELSE")
        assert.is_not_nil(first_if)
        assert.is_not_nil(first_elsif)
        assert.is_not_nil(first_else)
    end)

    it("tokenizes case/when/others structure", function()
        local tokens = vh.tokenize("case sel when \"00\" => y := 0; when others => y := 1; end case;")
        local first_case   = first_of(tokens, "CASE")
        local first_when   = first_of(tokens, "WHEN")
        local first_others = first_of(tokens, "OTHERS")
        assert.is_not_nil(first_case)
        assert.is_not_nil(first_when)
        assert.is_not_nil(first_others)
    end)

    it("tokenizes process with sensitivity list", function()
        -- process(clk) triggers whenever `clk` changes
        local tokens = vh.tokenize("process (clk)")
        local t = types(tokens)
        assert.are.same({"PROCESS", "LPAREN", "NAME", "RPAREN"}, t)
    end)

    it("tokenizes library use clause", function()
        -- Standard boilerplate at top of every VHDL file:
        -- library ieee;
        -- use ieee.std_logic_1164.all;
        local tokens = vh.tokenize("library ieee;")
        local t = types(tokens)
        assert.are.same({"LIBRARY", "NAME", "SEMICOLON"}, t)
    end)

    it("tokenizes logical expression: a and b or not c", function()
        local tokens = vh.tokenize("a and b or not c")
        local t = types(tokens)
        assert.are.same({"NAME", "AND", "NAME", "OR", "NOT", "NAME"}, t)
    end)

    it("tokenizes not-equals comparison: a /= b", function()
        local tokens = vh.tokenize("a /= b")
        local t = types(tokens)
        assert.are.same({"NAME", "NOT_EQUALS", "NAME"}, t)
    end)

    it("tokenizes constant declaration", function()
        -- constant WIDTH : integer := 8;
        local tokens = vh.tokenize("constant WIDTH : integer := 8;")
        local t = types(tokens)
        assert.are.same({
            "CONSTANT", "NAME", "COLON", "NAME", "VAR_ASSIGN", "NUMBER", "SEMICOLON"
        }, t)
    end)
end)

-- =========================================================================
-- Whitespace handling
-- =========================================================================

describe("whitespace handling", function()
    it("strips spaces between tokens", function()
        local tokens = vh.tokenize("a <= b")
        local t = types(tokens)
        assert.are.same({"NAME", "LESS_EQUALS", "NAME"}, t)
    end)

    it("strips tabs between tokens", function()
        local tokens = vh.tokenize("a\t<=\tb")
        local t = types(tokens)
        assert.are.same({"NAME", "LESS_EQUALS", "NAME"}, t)
    end)

    it("strips newlines between tokens", function()
        local tokens = vh.tokenize("a\n<=\nb")
        local t = types(tokens)
        assert.are.same({"NAME", "LESS_EQUALS", "NAME"}, t)
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("tracks column for single-line input", function()
        -- a _ < = _ b
        -- 1 2 3 4 5 6
        local tokens = vh.tokenize("a <= b")
        assert.are.equal(1, tokens[1].col)  -- a
        assert.are.equal(3, tokens[2].col)  -- <=
        assert.are.equal(6, tokens[3].col)  -- b
    end)

    it("all tokens on line 1 for single-line input", function()
        local tokens = vh.tokenize("entity foo is")
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
        local tokens = vh.tokenize("1")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = vh.tokenize("1")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)
