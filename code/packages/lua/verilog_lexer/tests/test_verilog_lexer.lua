-- Tests for verilog_lexer
-- =======================
--
-- Comprehensive busted test suite for the Verilog lexer package.
--
-- Verilog (IEEE 1364-2005) is a Hardware Description Language. Unlike software
-- languages, Verilog describes parallel hardware: wires, gates, flip-flops.
-- The lexer must handle hardware-specific tokens like sized numbers (4'b1010),
-- system tasks ($display), compiler directives (`define), and sensitivity
-- lists (@(posedge clk)).
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty and whitespace-only input produces only EOF
--   - Module structure keywords: module, endmodule, input, output, inout,
--     reg, wire, parameter, localparam
--   - Control flow: if, else, case, casez, casex, endcase, for, while (not
--     in grammar — note: verilog uses always/initial instead of while),
--     always, initial, begin, end
--   - Gate primitives: and, or, not, nand, nor, xor, xnor, buf
--   - Number literals: decimal, hex, binary, octal sized numbers
--   - Operators: =, <=, ==, !=, &, |, ^, ~, <<, >>, +, -, *, /
--   - Special: #delay, @event, $system_task
--   - Comments: line comment, block comment
--   - Identifiers: simple, underscore-prefixed, with digits
--   - Token positions (line, col) are tracked correctly
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

local vl = require("coding_adventures.verilog_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by vl.tokenize.
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
-- @param tokens  table  The token list returned by vl.tokenize.
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

describe("verilog_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(vl)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(vl.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", vl.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(vl.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(vl.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = vl.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = vl.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input produces only EOF", function()
        local tokens = vl.tokenize("   \t  \n  ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("line comment only produces EOF", function()
        -- Verilog line comments start with //
        local tokens = vl.tokenize("// this is a comment")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("block comment only produces EOF", function()
        -- Verilog block comments: /* ... */ (same as C)
        local tokens = vl.tokenize("/* this is a block comment */")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Module structure keywords
-- =========================================================================
--
-- These keywords define the structural boundary of a Verilog module.
-- A module is like a class in software: it encapsulates a hardware component
-- with named inputs and outputs.

describe("module structure keywords", function()
    it("tokenizes module", function()
        -- `module` begins a hardware component definition
        local tokens = vl.tokenize("module")
        assert.are.equal("MODULE", tokens[1].type)
        assert.are.equal("module", tokens[1].value)
    end)

    it("tokenizes endmodule", function()
        -- `endmodule` closes the module block (like `end` in Ruby or `}` in C)
        local tokens = vl.tokenize("endmodule")
        assert.are.equal("ENDMODULE", tokens[1].type)
        assert.are.equal("endmodule", tokens[1].value)
    end)

    it("tokenizes input", function()
        -- Port direction: signal flows into this module from the outside
        local tokens = vl.tokenize("input")
        assert.are.equal("INPUT", tokens[1].type)
        assert.are.equal("input", tokens[1].value)
    end)

    it("tokenizes output", function()
        -- Port direction: signal flows out of this module to the outside
        local tokens = vl.tokenize("output")
        assert.are.equal("OUTPUT", tokens[1].type)
        assert.are.equal("output", tokens[1].value)
    end)

    it("tokenizes inout", function()
        -- Bidirectional port: can be driven from either side (like a bus)
        local tokens = vl.tokenize("inout")
        assert.are.equal("INOUT", tokens[1].type)
        assert.are.equal("inout", tokens[1].value)
    end)

    it("tokenizes reg", function()
        -- `reg` declares a register: a flip-flop that holds its value
        -- between clock edges. Driven by procedural (always/initial) blocks.
        local tokens = vl.tokenize("reg")
        assert.are.equal("REG", tokens[1].type)
        assert.are.equal("reg", tokens[1].value)
    end)

    it("tokenizes wire", function()
        -- `wire` declares a combinational net: it has a value only when driven.
        -- Unlike `reg`, a wire cannot store state.
        local tokens = vl.tokenize("wire")
        assert.are.equal("WIRE", tokens[1].type)
        assert.are.equal("wire", tokens[1].value)
    end)

    it("tokenizes parameter", function()
        -- `parameter` declares a compile-time constant, like a C #define.
        -- Can be overridden during module instantiation (e.g. #(WIDTH=16)).
        local tokens = vl.tokenize("parameter")
        assert.are.equal("PARAMETER", tokens[1].type)
        assert.are.equal("parameter", tokens[1].value)
    end)

    it("tokenizes localparam", function()
        -- `localparam` is like parameter but cannot be overridden externally.
        -- Used for internal constants that should not be configurable.
        local tokens = vl.tokenize("localparam")
        assert.are.equal("LOCALPARAM", tokens[1].type)
        assert.are.equal("localparam", tokens[1].value)
    end)
end)

-- =========================================================================
-- Control flow keywords
-- =========================================================================
--
-- Verilog procedural blocks (always, initial) contain sequential control flow
-- just like a software function. These keywords operate within those blocks.

describe("control flow keywords", function()
    it("tokenizes always", function()
        -- `always` block: re-executes whenever sensitivity signals change.
        -- Models combinational or sequential logic (e.g. flip-flops).
        local tokens = vl.tokenize("always")
        assert.are.equal("ALWAYS", tokens[1].type)
        assert.are.equal("always", tokens[1].value)
    end)

    it("tokenizes initial", function()
        -- `initial` block: executes once at time zero. Simulation only.
        -- Used to set up testbench stimuli or initialize memories.
        local tokens = vl.tokenize("initial")
        assert.are.equal("INITIAL", tokens[1].type)
        assert.are.equal("initial", tokens[1].value)
    end)

    it("tokenizes begin", function()
        -- `begin...end` groups multiple statements like `{...}` in C.
        -- Required when a conditional or loop body has more than one statement.
        local tokens = vl.tokenize("begin")
        assert.are.equal("BEGIN", tokens[1].type)
        assert.are.equal("begin", tokens[1].value)
    end)

    it("tokenizes end", function()
        local tokens = vl.tokenize("end")
        assert.are.equal("END", tokens[1].type)
        assert.are.equal("end", tokens[1].value)
    end)

    it("tokenizes if", function()
        local tokens = vl.tokenize("if")
        assert.are.equal("IF", tokens[1].type)
        assert.are.equal("if", tokens[1].value)
    end)

    it("tokenizes else", function()
        local tokens = vl.tokenize("else")
        assert.are.equal("ELSE", tokens[1].type)
        assert.are.equal("else", tokens[1].value)
    end)

    it("tokenizes case", function()
        -- `case` in Verilog matches a value against multiple alternatives.
        -- In synthesis, maps directly to a multiplexer.
        local tokens = vl.tokenize("case")
        assert.are.equal("CASE", tokens[1].type)
        assert.are.equal("case", tokens[1].value)
    end)

    it("tokenizes casez", function()
        -- `casez` treats z (high-impedance) bits as don't-care in matching.
        -- Useful for instruction decode with partially-specified opcodes.
        local tokens = vl.tokenize("casez")
        assert.are.equal("CASEZ", tokens[1].type)
        assert.are.equal("casez", tokens[1].value)
    end)

    it("tokenizes casex", function()
        -- `casex` treats both x (unknown) and z bits as don't-care.
        local tokens = vl.tokenize("casex")
        assert.are.equal("CASEX", tokens[1].type)
        assert.are.equal("casex", tokens[1].value)
    end)

    it("tokenizes endcase", function()
        local tokens = vl.tokenize("endcase")
        assert.are.equal("ENDCASE", tokens[1].type)
        assert.are.equal("endcase", tokens[1].value)
    end)

    it("tokenizes for", function()
        -- `for` loop — synthesizes to unrolled logic or (with genvar) generates
        -- repeated hardware structure.
        local tokens = vl.tokenize("for")
        assert.are.equal("FOR", tokens[1].type)
        assert.are.equal("for", tokens[1].value)
    end)
end)

-- =========================================================================
-- Gate primitives
-- =========================================================================
--
-- Verilog has built-in gate keywords that instantiate primitive logic gates
-- directly, without defining a separate module. These map 1:1 to physical
-- gates in a cell library.
--
--   and  a(out, in1, in2);   — 2-input AND gate, output first
--   not  n(out, in);         — inverter

describe("gate primitive keywords", function()
    it("tokenizes and", function()
        local tokens = vl.tokenize("and")
        assert.are.equal("AND", tokens[1].type)
        assert.are.equal("and", tokens[1].value)
    end)

    it("tokenizes or", function()
        local tokens = vl.tokenize("or")
        assert.are.equal("OR", tokens[1].type)
        assert.are.equal("or", tokens[1].value)
    end)

    it("tokenizes not", function()
        local tokens = vl.tokenize("not")
        assert.are.equal("NOT", tokens[1].type)
        assert.are.equal("not", tokens[1].value)
    end)

    it("tokenizes nand", function()
        -- NAND: output is 1 unless all inputs are 1. Universal gate.
        local tokens = vl.tokenize("nand")
        assert.are.equal("NAND", tokens[1].type)
        assert.are.equal("nand", tokens[1].value)
    end)

    it("tokenizes nor", function()
        -- NOR: output is 1 only when all inputs are 0. Universal gate.
        local tokens = vl.tokenize("nor")
        assert.are.equal("NOR", tokens[1].type)
        assert.are.equal("nor", tokens[1].value)
    end)

    it("tokenizes xor", function()
        -- XOR: output is 1 when inputs differ. Used in adders and CRC circuits.
        local tokens = vl.tokenize("xor")
        assert.are.equal("XOR", tokens[1].type)
        assert.are.equal("xor", tokens[1].value)
    end)

    it("tokenizes xnor", function()
        -- XNOR: output is 1 when inputs are equal. Equivalence gate.
        local tokens = vl.tokenize("xnor")
        assert.are.equal("XNOR", tokens[1].type)
        assert.are.equal("xnor", tokens[1].value)
    end)

    it("tokenizes buf", function()
        -- `buf` is a buffer: passes the input through unchanged.
        -- Used to drive large fanout signals without degrading signal strength.
        local tokens = vl.tokenize("buf")
        assert.are.equal("BUF", tokens[1].type)
        assert.are.equal("buf", tokens[1].value)
    end)
end)

-- =========================================================================
-- Number literals
-- =========================================================================
--
-- Verilog has a unique number format inherited from hardware design needs.
-- Every signal has a specific bit width, so numbers carry size information:
--
--   [size]'[signed][base]digits
--
-- This allows the tool to know exactly how many bits to allocate.

describe("number literals", function()
    it("tokenizes a plain decimal integer", function()
        -- Plain integers (no size prefix) are unsized — width inferred from context
        local tokens = vl.tokenize("32")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("32", tokens[1].value)
    end)

    it("tokenizes a hex sized number: 8'hFF", function()
        -- 8-bit hex literal: the `h` base specifier, FF = 255 decimal
        -- The `'` separator is not the Ruby string delimiter — it's a size separator!
        local tokens = vl.tokenize("8'hFF")
        assert.are.equal("SIZED_NUMBER", tokens[1].type)
        assert.are.equal("8'hFF", tokens[1].value)
    end)

    it("tokenizes a binary sized number: 4'b1010", function()
        -- 4-bit binary: 1010 = 10 decimal. Binary is most explicit about bit pattern.
        local tokens = vl.tokenize("4'b1010")
        assert.are.equal("SIZED_NUMBER", tokens[1].type)
        assert.are.equal("4'b1010", tokens[1].value)
    end)

    it("tokenizes an octal sized number: 8'o77", function()
        -- 8-bit octal: o77 = 63 decimal
        local tokens = vl.tokenize("8'o77")
        assert.are.equal("SIZED_NUMBER", tokens[1].type)
        assert.are.equal("8'o77", tokens[1].value)
    end)

    it("tokenizes a decimal sized number: 32'd42", function()
        -- 32-bit decimal: explicit width + decimal value
        local tokens = vl.tokenize("32'd42")
        assert.are.equal("SIZED_NUMBER", tokens[1].type)
        assert.are.equal("32'd42", tokens[1].value)
    end)

    it("tokenizes a sized number with x (unknown) bits: 4'bxxzz", function()
        -- x = unknown, z = high-impedance. Hardware-specific states!
        -- A freshly powered flip-flop is 'x'. A floating wire is 'z'.
        local tokens = vl.tokenize("4'bxxzz")
        assert.are.equal("SIZED_NUMBER", tokens[1].type)
        assert.are.equal("4'bxxzz", tokens[1].value)
    end)

    it("tokenizes numbers separated by an operator: 8+3", function()
        local tokens = vl.tokenize("8+3")
        local t = types(tokens)
        assert.are.same({"NUMBER", "PLUS", "NUMBER"}, t)
    end)
end)

-- =========================================================================
-- Operators
-- =========================================================================
--
-- Verilog operators mirror C with hardware-specific additions.
-- The bitwise operators (&, |, ^, ~) double as reduction operators when
-- applied unary: &8'b11111111 = 1 (reduce AND all bits).

describe("operator tokens", function()
    -- Assignment operators
    it("tokenizes = (blocking assignment)", function()
        -- `=` is a blocking assignment in procedural blocks: executes sequentially
        local tokens = vl.tokenize("=")
        assert.are.equal("EQUALS", tokens[1].type)
        assert.are.equal("=", tokens[1].value)
    end)

    it("tokenizes <= (non-blocking assignment / less-equals)", function()
        -- `<=` is the non-blocking assignment: all RHS evaluated before any LHS
        -- updated. Models the parallel update of flip-flops on a clock edge.
        -- Context determines whether it is assignment or comparison.
        local tokens = vl.tokenize("<=")
        assert.are.equal("LESS_EQUALS", tokens[1].type)
        assert.are.equal("<=", tokens[1].value)
    end)

    it("tokenizes == (equality)", function()
        -- 2-state equality: returns x if either operand has x or z bits
        local tokens = vl.tokenize("==")
        assert.are.equal("EQUALS_EQUALS", tokens[1].type)
        assert.are.equal("==", tokens[1].value)
    end)

    it("tokenizes != (inequality)", function()
        local tokens = vl.tokenize("!=")
        assert.are.equal("NOT_EQUALS", tokens[1].type)
        assert.are.equal("!=", tokens[1].value)
    end)

    -- Bitwise / reduction operators
    it("tokenizes & (bitwise/reduction AND)", function()
        local tokens = vl.tokenize("&")
        assert.are.equal("AMP", tokens[1].type)
        assert.are.equal("&", tokens[1].value)
    end)

    it("tokenizes | (bitwise/reduction OR)", function()
        local tokens = vl.tokenize("|")
        assert.are.equal("PIPE", tokens[1].type)
        assert.are.equal("|", tokens[1].value)
    end)

    it("tokenizes ^ (bitwise XOR)", function()
        local tokens = vl.tokenize("^")
        assert.are.equal("CARET", tokens[1].type)
        assert.are.equal("^", tokens[1].value)
    end)

    it("tokenizes ~ (bitwise NOT)", function()
        local tokens = vl.tokenize("~")
        assert.are.equal("TILDE", tokens[1].type)
        assert.are.equal("~", tokens[1].value)
    end)

    it("tokenizes << (left shift)", function()
        -- Logical left shift: shift left, fill with 0s
        local tokens = vl.tokenize("<<")
        assert.are.equal("LEFT_SHIFT", tokens[1].type)
        assert.are.equal("<<", tokens[1].value)
    end)

    it("tokenizes >> (right shift)", function()
        -- Logical right shift: shift right, fill with 0s
        local tokens = vl.tokenize(">>")
        assert.are.equal("RIGHT_SHIFT", tokens[1].type)
        assert.are.equal(">>", tokens[1].value)
    end)

    it("tokenizes + (plus)", function()
        local tokens = vl.tokenize("+")
        assert.are.equal("PLUS", tokens[1].type)
        assert.are.equal("+", tokens[1].value)
    end)

    it("tokenizes - (minus)", function()
        local tokens = vl.tokenize("-")
        assert.are.equal("MINUS", tokens[1].type)
        assert.are.equal("-", tokens[1].value)
    end)

    it("tokenizes * (multiply)", function()
        local tokens = vl.tokenize("*")
        assert.are.equal("STAR", tokens[1].type)
        assert.are.equal("*", tokens[1].value)
    end)

    it("tokenizes / (divide)", function()
        local tokens = vl.tokenize("/")
        assert.are.equal("SLASH", tokens[1].type)
        assert.are.equal("/", tokens[1].value)
    end)

    it("tokenizes ** (power)", function()
        local tokens = vl.tokenize("**")
        assert.are.equal("POWER", tokens[1].type)
        assert.are.equal("**", tokens[1].value)
    end)

    it("tokenizes >= (greater-equals)", function()
        local tokens = vl.tokenize(">=")
        assert.are.equal("GREATER_EQUALS", tokens[1].type)
        assert.are.equal(">=", tokens[1].value)
    end)

    it("tokenizes < (less than)", function()
        local tokens = vl.tokenize("<")
        assert.are.equal("LESS_THAN", tokens[1].type)
        assert.are.equal("<", tokens[1].value)
    end)

    it("tokenizes > (greater than)", function()
        local tokens = vl.tokenize(">")
        assert.are.equal("GREATER_THAN", tokens[1].type)
        assert.are.equal(">", tokens[1].value)
    end)
end)

-- =========================================================================
-- Special tokens
-- =========================================================================
--
-- Verilog has three kinds of special-prefix identifiers unique to the language:
--   $ — system tasks/functions ($display, $time, $finish)
--   ` — compiler directives (`define, `ifdef, `include)
--   # — delay operator (#10 = delay 10 time units)
--   @ — event control (@(posedge clk) = wait for rising clock edge)

describe("special tokens", function()
    it("tokenizes a system task: $display", function()
        -- System tasks call the Verilog simulator runtime.
        -- $display is like printf for hardware simulation.
        local tokens = vl.tokenize("$display")
        assert.are.equal("SYSTEM_ID", tokens[1].type)
        assert.are.equal("$display", tokens[1].value)
    end)

    it("tokenizes a system function: $time", function()
        -- $time returns the current simulation time in time units
        local tokens = vl.tokenize("$time")
        assert.are.equal("SYSTEM_ID", tokens[1].type)
        assert.are.equal("$time", tokens[1].value)
    end)

    it("tokenizes a compiler directive: `define", function()
        -- Compiler directives preprocess the source before tokenization.
        -- `define WIDTH 8 creates a macro like C's #define.
        local tokens = vl.tokenize("`define")
        assert.are.equal("DIRECTIVE", tokens[1].type)
        assert.are.equal("`define", tokens[1].value)
    end)

    it("tokenizes hash #delay", function()
        -- # introduces a delay: #10 means "wait 10 time units"
        -- In synthesis, delays are ignored — they only affect simulation.
        local tokens = vl.tokenize("#")
        assert.are.equal("HASH", tokens[1].type)
        assert.are.equal("#", tokens[1].value)
    end)

    it("tokenizes @ event control", function()
        -- @ introduces an event control: @(posedge clk) means
        -- "wait for the rising edge of clk". This triggers flip-flop updates.
        local tokens = vl.tokenize("@")
        assert.are.equal("AT", tokens[1].type)
        assert.are.equal("@", tokens[1].value)
    end)

    it("tokenizes #10 as HASH followed by NUMBER", function()
        -- #10 is a delay: HASH token + NUMBER token
        local tokens = vl.tokenize("#10")
        local t = types(tokens)
        assert.are.same({"HASH", "NUMBER"}, t)
        assert.are.equal("10", tokens[2].value)
    end)

    it("tokenizes @(posedge clk) structure", function()
        -- Sensitivity list: wait for rising edge of clock.
        -- posedge = positive edge = 0→1 transition.
        local tokens = vl.tokenize("@(posedge clk)")
        local t = types(tokens)
        assert.are.same({"AT", "LPAREN", "POSEDGE", "NAME", "RPAREN"}, t)
    end)
end)

-- =========================================================================
-- Comments
-- =========================================================================
--
-- Verilog comments are identical to C: // for line comments, /* */ for block.
-- Comments are declared as skip patterns and never appear in the token stream.

describe("comments", function()
    it("skips a line comment", function()
        -- Everything after // to end of line is a comment
        local tokens = vl.tokenize("wire a; // clock signal")
        local t = types(tokens)
        -- Only wire, identifier, semicolon — no comment token
        assert.are.same({"WIRE", "NAME", "SEMICOLON"}, t)
    end)

    it("skips a block comment", function()
        -- /* ... */ can span multiple lines
        local tokens = vl.tokenize("wire /* combinational */ a")
        local t = types(tokens)
        assert.are.same({"WIRE", "NAME"}, t)
    end)

    it("skips a multi-line block comment", function()
        local tokens = vl.tokenize("a /* line1\n   line2 */ b")
        local t = types(tokens)
        assert.are.same({"NAME", "NAME"}, t)
        assert.are.equal("a", tokens[1].value)
        assert.are.equal("b", tokens[2].value)
    end)
end)

-- =========================================================================
-- Identifiers
-- =========================================================================

describe("identifier tokens", function()
    it("tokenizes a simple identifier", function()
        local tokens = vl.tokenize("simple_id")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("simple_id", tokens[1].value)
    end)

    it("tokenizes an underscore-prefixed identifier", function()
        -- Common convention: _private signals are internal-only
        local tokens = vl.tokenize("_private")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("_private", tokens[1].value)
    end)

    it("tokenizes an identifier with digits", function()
        local tokens = vl.tokenize("id123")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("id123", tokens[1].value)
    end)

    it("does not classify non-keyword names as keywords", function()
        -- "clk" is a common signal name — should be NAME, not a keyword
        local tokens = vl.tokenize("clk")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("clk", tokens[1].value)
    end)
end)

-- =========================================================================
-- String literals
-- =========================================================================
--
-- Verilog strings use C-style double-quoted syntax with backslash escaping.
-- Primarily used with $display and other system tasks.

describe("string literals", function()
    it("tokenizes a double-quoted string", function()
        local tokens = vl.tokenize('"hello"')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('hello', tokens[1].value)
    end)

    it("tokenizes an empty string", function()
        local tokens = vl.tokenize('""')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('', tokens[1].value)
    end)
end)

-- =========================================================================
-- Delimiter tokens
-- =========================================================================

describe("delimiter tokens", function()
    it("tokenizes parentheses", function()
        local tokens = vl.tokenize("()")
        local t = types(tokens)
        assert.are.same({"LPAREN", "RPAREN"}, t)
    end)

    it("tokenizes square brackets", function()
        -- Square brackets select bit ranges: wire[7:0] selects 8-bit bus
        local tokens = vl.tokenize("[]")
        local t = types(tokens)
        assert.are.same({"LBRACKET", "RBRACKET"}, t)
    end)

    it("tokenizes curly braces (concatenation)", function()
        -- Curly braces concatenate buses: {a, b} joins two signals
        local tokens = vl.tokenize("{}")
        local t = types(tokens)
        assert.are.same({"LBRACE", "RBRACE"}, t)
    end)

    it("tokenizes semicolon", function()
        local tokens = vl.tokenize(";")
        assert.are.equal("SEMICOLON", tokens[1].type)
    end)

    it("tokenizes comma", function()
        local tokens = vl.tokenize(",")
        assert.are.equal("COMMA", tokens[1].type)
    end)

    it("tokenizes dot (hierarchy)", function()
        -- The dot accesses sub-module ports: adder.sum
        local tokens = vl.tokenize(".")
        assert.are.equal("DOT", tokens[1].type)
    end)
end)

-- =========================================================================
-- Composite expressions
-- =========================================================================

describe("composite expressions", function()
    it("tokenizes a module declaration header", function()
        -- module adder(input a, input b, output y);
        local tokens = vl.tokenize("module adder(input a, input b, output y);")
        local t = types(tokens)
        assert.are.same({
            "MODULE", "NAME",
            "LPAREN",
            "INPUT", "NAME", "COMMA",
            "INPUT", "NAME", "COMMA",
            "OUTPUT", "NAME",
            "RPAREN", "SEMICOLON"
        }, t)
        assert.are.equal("adder", tokens[2].value)
    end)

    it("tokenizes wire declaration with bit range", function()
        -- wire[7:0] declares an 8-bit bus (bits 7 down to 0)
        local tokens = vl.tokenize("wire [7:0] bus;")
        local t = types(tokens)
        assert.are.same({
            "WIRE", "LBRACKET", "NUMBER", "COLON", "NUMBER", "RBRACKET",
            "NAME", "SEMICOLON"
        }, t)
    end)

    it("tokenizes always block with sensitivity list", function()
        -- always @(posedge clk) triggers on rising clock edge: models flip-flop
        local tokens = vl.tokenize("always @(posedge clk)")
        local t = types(tokens)
        assert.are.same({"ALWAYS", "AT", "LPAREN", "POSEDGE", "NAME", "RPAREN"}, t)
    end)

    it("tokenizes non-blocking assignment: q <= d", function()
        -- Non-blocking assignment: all RHS evaluated before any assignment,
        -- models synchronous flip-flop update
        local tokens = vl.tokenize("q <= d")
        local t = types(tokens)
        assert.are.same({"NAME", "LESS_EQUALS", "NAME"}, t)
        assert.are.equal("q", tokens[1].value)
    end)

    it("tokenizes if/else structure", function()
        local tokens = vl.tokenize("if (a == b) begin end else begin end")
        local first_if = first_of(tokens, "IF")
        assert.is_not_nil(first_if)
        local first_else = first_of(tokens, "ELSE")
        assert.is_not_nil(first_else)
    end)

    it("tokenizes case statement with endcase", function()
        -- case maps to a multiplexer in synthesis
        local tokens = vl.tokenize("case (sel) endcase")
        local first_case = first_of(tokens, "CASE")
        assert.is_not_nil(first_case)
        local first_endcase = first_of(tokens, "ENDCASE")
        assert.is_not_nil(first_endcase)
    end)

    it("tokenizes arithmetic: a + b - c", function()
        local tokens = vl.tokenize("a + b - c")
        local t = types(tokens)
        assert.are.same({"NAME", "PLUS", "NAME", "MINUS", "NAME"}, t)
    end)

    it("tokenizes bitwise: a & b | c ^ d", function()
        local tokens = vl.tokenize("a & b | c ^ d")
        local t = types(tokens)
        assert.are.same({"NAME", "AMP", "NAME", "PIPE", "NAME", "CARET", "NAME"}, t)
    end)

    it("tokenizes comparison: a != b", function()
        local tokens = vl.tokenize("a != b")
        local t = types(tokens)
        assert.are.same({"NAME", "NOT_EQUALS", "NAME"}, t)
    end)

    it("tokenizes a parameter declaration", function()
        -- parameter WIDTH = 8; — compile-time constant
        local tokens = vl.tokenize("parameter WIDTH = 8;")
        local t = types(tokens)
        assert.are.same({"PARAMETER", "NAME", "EQUALS", "NUMBER", "SEMICOLON"}, t)
    end)
end)

-- =========================================================================
-- Whitespace handling
-- =========================================================================

describe("whitespace handling", function()
    it("strips spaces between tokens", function()
        local tokens = vl.tokenize("a = 1")
        local t = types(tokens)
        assert.are.same({"NAME", "EQUALS", "NUMBER"}, t)
    end)

    it("strips tabs between tokens", function()
        local tokens = vl.tokenize("a\t=\t1")
        local t = types(tokens)
        assert.are.same({"NAME", "EQUALS", "NUMBER"}, t)
    end)

    it("strips newlines between tokens", function()
        local tokens = vl.tokenize("a\n=\n1")
        local t = types(tokens)
        assert.are.same({"NAME", "EQUALS", "NUMBER"}, t)
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("tracks column for single-line input: a = 1", function()
        local tokens = vl.tokenize("a = 1")
        assert.are.equal(1, tokens[1].col)  -- a
        assert.are.equal(3, tokens[2].col)  -- =
        assert.are.equal(5, tokens[3].col)  -- 1
    end)

    it("all tokens on line 1 for single-line input", function()
        local tokens = vl.tokenize("module test;")
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
        local tokens = vl.tokenize("1")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = vl.tokenize("1")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)
