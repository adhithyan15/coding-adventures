use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::VerilogLexer; 1 }, 'module loads' );

# ============================================================================
# Helper: collect token types (excluding EOF) from a source string
# ============================================================================

sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::VerilogLexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub values_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::VerilogLexer->tokenize($source);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub types_of_version {
    my ($source, $version) = @_;
    my $tokens = CodingAdventures::VerilogLexer->tokenize($source, $version);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub values_of_version {
    my ($source, $version) = @_;
    my $tokens = CodingAdventures::VerilogLexer->tokenize($source, $version);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'whitespace-only produces only EOF' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize("   \t\n  ");
    is( scalar @$tokens, 1,     '1 token after skipping whitespace' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

subtest 'default version matches explicit 2005' => sub {
    is(
        types_of('module',),
        types_of_version('module', '2005'),
        'default token types match 2005'
    );
    is(
        values_of('module'),
        values_of_version('module', '2005'),
        'default token values match 2005'
    );
};

subtest 'unknown version raises die' => sub {
    ok(
        dies { CodingAdventures::VerilogLexer->tokenize('module', '2099') },
        'unsupported version dies'
    );
};

subtest 'line comment only produces EOF' => sub {
    # Verilog line comments start with //
    my $tokens = CodingAdventures::VerilogLexer->tokenize('// this is a comment');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'comment consumed, only EOF' );
};

subtest 'block comment only produces EOF' => sub {
    # Verilog block comments: /* ... */
    my $tokens = CodingAdventures::VerilogLexer->tokenize('/* block comment */');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'block comment consumed, only EOF' );
};

# ============================================================================
# Module structure keywords
# ============================================================================
#
# These keywords form the structural skeleton of a Verilog module.
# A module is the fundamental unit of hardware: an encapsulated component
# with named ports. `module` opens it, `endmodule` closes it.

subtest 'keyword: module' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('module');
    is( $tokens->[0]{type},  'MODULE', 'type is MODULE' );
    is( $tokens->[0]{value}, 'module', 'value is module' );
};

subtest 'keyword: endmodule' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('endmodule');
    is( $tokens->[0]{type},  'ENDMODULE', 'type is ENDMODULE' );
    is( $tokens->[0]{value}, 'endmodule', 'value is endmodule' );
};

subtest 'keyword: input' => sub {
    # Port direction: signal flows into this module from outside
    my $tokens = CodingAdventures::VerilogLexer->tokenize('input');
    is( $tokens->[0]{type},  'INPUT', 'type is INPUT' );
    is( $tokens->[0]{value}, 'input', 'value is input' );
};

subtest 'keyword: output' => sub {
    # Port direction: signal flows out of this module
    my $tokens = CodingAdventures::VerilogLexer->tokenize('output');
    is( $tokens->[0]{type},  'OUTPUT', 'type is OUTPUT' );
    is( $tokens->[0]{value}, 'output', 'value is output' );
};

subtest 'keyword: inout' => sub {
    # Bidirectional port: can be driven from either side
    my $tokens = CodingAdventures::VerilogLexer->tokenize('inout');
    is( $tokens->[0]{type},  'INOUT', 'type is INOUT' );
    is( $tokens->[0]{value}, 'inout', 'value is inout' );
};

subtest 'keyword: reg' => sub {
    # `reg` declares a storage element (flip-flop or latch)
    my $tokens = CodingAdventures::VerilogLexer->tokenize('reg');
    is( $tokens->[0]{type},  'REG', 'type is REG' );
    is( $tokens->[0]{value}, 'reg', 'value is reg' );
};

subtest 'keyword: wire' => sub {
    # `wire` declares a combinational net — no storage, just a connection
    my $tokens = CodingAdventures::VerilogLexer->tokenize('wire');
    is( $tokens->[0]{type},  'WIRE', 'type is WIRE' );
    is( $tokens->[0]{value}, 'wire', 'value is wire' );
};

subtest 'keyword: parameter' => sub {
    # `parameter` declares a compile-time constant, overridable at instantiation
    my $tokens = CodingAdventures::VerilogLexer->tokenize('parameter');
    is( $tokens->[0]{type},  'PARAMETER', 'type is PARAMETER' );
    is( $tokens->[0]{value}, 'parameter', 'value is parameter' );
};

subtest 'keyword: localparam' => sub {
    # `localparam` is like parameter but cannot be overridden externally
    my $tokens = CodingAdventures::VerilogLexer->tokenize('localparam');
    is( $tokens->[0]{type},  'LOCALPARAM', 'type is LOCALPARAM' );
    is( $tokens->[0]{value}, 'localparam', 'value is localparam' );
};

# ============================================================================
# Control flow keywords
# ============================================================================

subtest 'keywords: always and initial' => sub {
    # `always` re-executes on sensitivity changes (models flip-flops)
    # `initial` runs once at time 0 (simulation/testbench setup)
    is(
        types_of('always initial'),
        [qw(ALWAYS INITIAL)],
        'always and initial types'
    );
};

subtest 'keywords: begin and end' => sub {
    # begin...end groups multiple statements like { } in C
    is(
        types_of('begin end'),
        [qw(BEGIN END)],
        'begin and end types'
    );
};

subtest 'keywords: if and else' => sub {
    is(
        types_of('if else'),
        [qw(IF ELSE)],
        'if and else types'
    );
};

subtest 'keywords: case, casez, casex, endcase' => sub {
    # case matches a value; casez allows z (high-Z) as don't-care;
    # casex allows both x and z as don't-care
    is(
        types_of('case casez casex endcase'),
        [qw(CASE CASEZ CASEX ENDCASE)],
        'case family types'
    );
};

subtest 'keyword: for' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('for');
    is( $tokens->[0]{type},  'FOR', 'type is FOR' );
    is( $tokens->[0]{value}, 'for', 'value is for' );
};

# ============================================================================
# Gate primitive keywords
# ============================================================================
#
# Verilog gate primitives instantiate physical logic gates directly, without
# defining a separate module. Each maps to a cell in the standard-cell library.
#
#   and  a(out, in1, in2);   — 2-input AND, output listed first

subtest 'gate primitives: and, or, not' => sub {
    is(
        types_of('and or not'),
        [qw(AND OR NOT)],
        'and or not types'
    );
};

subtest 'gate primitives: nand, nor' => sub {
    # NAND and NOR are universal gates — any logic function can be built from them
    is(
        types_of('nand nor'),
        [qw(NAND NOR)],
        'nand nor types'
    );
};

subtest 'gate primitives: xor, xnor, buf' => sub {
    # xor: useful in adders and CRC circuits
    # xnor: equivalence gate
    # buf: driver/amplifier for high-fanout signals
    is(
        types_of('xor xnor buf'),
        [qw(XOR XNOR BUF)],
        'xor xnor buf types'
    );
};

# ============================================================================
# Number literals
# ============================================================================
#
# Verilog's unique sized number format: [size]'[base]digits
# Carries bit-width information that hardware tools need to allocate storage.

subtest 'plain decimal integer' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('32');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '32',     'value is 32' );
};

subtest 'hex sized number: 8\'hFF' => sub {
    # 8-bit hex: 8 bits wide, hexadecimal base, value FF (255 decimal)
    my $tokens = CodingAdventures::VerilogLexer->tokenize("8'hFF");
    is( $tokens->[0]{type},  'SIZED_NUMBER', 'type is SIZED_NUMBER' );
    is( $tokens->[0]{value}, "8'hFF",        "value is 8'hFF" );
};

subtest 'binary sized number: 4\'b1010' => sub {
    # 4-bit binary: most explicit representation of bit patterns
    my $tokens = CodingAdventures::VerilogLexer->tokenize("4'b1010");
    is( $tokens->[0]{type},  'SIZED_NUMBER', 'type is SIZED_NUMBER' );
    is( $tokens->[0]{value}, "4'b1010",      "value is 4'b1010" );
};

subtest 'octal sized number: 8\'o77' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize("8'o77");
    is( $tokens->[0]{type},  'SIZED_NUMBER', 'type is SIZED_NUMBER' );
    is( $tokens->[0]{value}, "8'o77",        "value is 8'o77" );
};

subtest 'sized number with x/z bits: 4\'bxxzz' => sub {
    # x = unknown (uninitialized flip-flop), z = high-impedance (floating wire)
    # These physical states have no equivalent in software languages.
    my $tokens = CodingAdventures::VerilogLexer->tokenize("4'bxxzz");
    is( $tokens->[0]{type},  'SIZED_NUMBER', 'type is SIZED_NUMBER' );
    is( $tokens->[0]{value}, "4'bxxzz",      "value is 4'bxxzz" );
};

subtest 'numbers separated by operator' => sub {
    is( types_of('8+3'), [qw(NUMBER PLUS NUMBER)], '8+3 types' );
};

# ============================================================================
# Operators
# ============================================================================

subtest 'assignment: =' => sub {
    # Blocking assignment in procedural blocks: executes in order
    my $tokens = CodingAdventures::VerilogLexer->tokenize('=');
    is( $tokens->[0]{type},  'EQUALS', 'type is EQUALS' );
    is( $tokens->[0]{value}, '=',      'value is =' );
};

subtest 'non-blocking assignment / less-equals: <=' => sub {
    # Non-blocking assignment: all RHS evaluated before LHS updated.
    # Models synchronous flip-flop update on clock edge.
    my $tokens = CodingAdventures::VerilogLexer->tokenize('<=');
    is( $tokens->[0]{type},  'LESS_EQUALS', 'type is LESS_EQUALS' );
    is( $tokens->[0]{value}, '<=',          'value is <=' );
};

subtest 'equality: ==' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('==');
    is( $tokens->[0]{type},  'EQUALS_EQUALS', 'type is EQUALS_EQUALS' );
    is( $tokens->[0]{value}, '==',            'value is ==' );
};

subtest 'not equals: !=' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('!=');
    is( $tokens->[0]{type},  'NOT_EQUALS', 'type is NOT_EQUALS' );
    is( $tokens->[0]{value}, '!=',         'value is !=' );
};

subtest 'bitwise/reduction AND: &' => sub {
    # As binary: a & b (bitwise AND of two operands)
    # As unary:  &a    (reduce AND — AND all bits together)
    my $tokens = CodingAdventures::VerilogLexer->tokenize('&');
    is( $tokens->[0]{type},  'AMP', 'type is AMP' );
    is( $tokens->[0]{value}, '&',   'value is &' );
};

subtest 'bitwise OR: |' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('|');
    is( $tokens->[0]{type},  'PIPE', 'type is PIPE' );
    is( $tokens->[0]{value}, '|',    'value is |' );
};

subtest 'bitwise XOR: ^' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('^');
    is( $tokens->[0]{type},  'CARET', 'type is CARET' );
    is( $tokens->[0]{value}, '^',     'value is ^' );
};

subtest 'bitwise NOT: ~' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('~');
    is( $tokens->[0]{type},  'TILDE', 'type is TILDE' );
    is( $tokens->[0]{value}, '~',     'value is ~' );
};

subtest 'left shift: <<' => sub {
    # Logical left shift: shift left, fill with 0s
    my $tokens = CodingAdventures::VerilogLexer->tokenize('<<');
    is( $tokens->[0]{type},  'LEFT_SHIFT', 'type is LEFT_SHIFT' );
    is( $tokens->[0]{value}, '<<',         'value is <<' );
};

subtest 'right shift: >>' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('>>');
    is( $tokens->[0]{type},  'RIGHT_SHIFT', 'type is RIGHT_SHIFT' );
    is( $tokens->[0]{value}, '>>',          'value is >>' );
};

subtest 'arithmetic operators: + - * /' => sub {
    is(
        types_of('+ - * /'),
        [qw(PLUS MINUS STAR SLASH)],
        'arithmetic operator types'
    );
};

subtest 'comparison: >= and <' => sub {
    is(
        types_of('>= <'),
        [qw(GREATER_EQUALS LESS_THAN)],
        '>= and < types'
    );
};

# ============================================================================
# Special tokens
# ============================================================================
#
# Verilog has three unique identifier prefixes:
#   $ — system tasks/functions (simulator runtime calls)
#   ` — compiler directives (preprocessor macros)
#   @ — event control (wait for signal edge)
#   # — delay operator (time-based delay)

subtest 'system task: $display' => sub {
    # $display is like printf for hardware simulation
    my $tokens = CodingAdventures::VerilogLexer->tokenize('$display');
    is( $tokens->[0]{type},  'SYSTEM_ID', 'type is SYSTEM_ID' );
    is( $tokens->[0]{value}, '$display',  'value is $display' );
};

subtest 'system function: $time' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('$time');
    is( $tokens->[0]{type},  'SYSTEM_ID', 'type is SYSTEM_ID' );
    is( $tokens->[0]{value}, '$time',     'value is $time' );
};

subtest 'compiler directive: `define' => sub {
    # `define creates a text macro: `define WIDTH 8
    my $tokens = CodingAdventures::VerilogLexer->tokenize('`define');
    is( $tokens->[0]{type},  'DIRECTIVE', 'type is DIRECTIVE' );
    is( $tokens->[0]{value}, '`define',   'value is `define' );
};

subtest 'hash delay: # produces HASH token' => sub {
    # # is used for timing delays (#10) and parameter overrides #(WIDTH=8)
    my $tokens = CodingAdventures::VerilogLexer->tokenize('#');
    is( $tokens->[0]{type},  'HASH', 'type is HASH' );
    is( $tokens->[0]{value}, '#',    'value is #' );
};

subtest 'at event control: @ produces AT token' => sub {
    # @ is used for sensitivity lists: @(posedge clk)
    my $tokens = CodingAdventures::VerilogLexer->tokenize('@');
    is( $tokens->[0]{type},  'AT', 'type is AT' );
    is( $tokens->[0]{value}, '@',  'value is @' );
};

subtest '#10 produces HASH then NUMBER' => sub {
    is( types_of('#10'), [qw(HASH NUMBER)], '#10 types' );
    is( values_of('#10'), ['#', '10'], '#10 values' );
};

subtest '@(posedge clk) tokenizes correctly' => sub {
    # Sensitivity list: wait for rising clock edge
    is(
        types_of('@(posedge clk)'),
        [qw(AT LPAREN POSEDGE NAME RPAREN)],
        '@(posedge clk) types'
    );
};

# ============================================================================
# Comments
# ============================================================================

subtest 'line comment after tokens is skipped' => sub {
    is(
        types_of('wire a; // clock signal'),
        [qw(WIRE NAME SEMICOLON)],
        'no comment token in output'
    );
};

subtest 'block comment between tokens is skipped' => sub {
    is(
        types_of('wire /* internal bus */ a'),
        [qw(WIRE NAME)],
        'block comment consumed'
    );
};

# ============================================================================
# Identifiers
# ============================================================================

subtest 'simple identifier' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('simple_id');
    is( $tokens->[0]{type},  'NAME',      'type is NAME' );
    is( $tokens->[0]{value}, 'simple_id', 'value is simple_id' );
};

subtest 'underscore-prefixed identifier' => sub {
    # Common convention: _private signals are internal-only
    my $tokens = CodingAdventures::VerilogLexer->tokenize('_private');
    is( $tokens->[0]{type},  'NAME',     'type is NAME' );
    is( $tokens->[0]{value}, '_private', 'value is _private' );
};

subtest 'identifier with digits' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('id123');
    is( $tokens->[0]{type},  'NAME',  'type is NAME' );
    is( $tokens->[0]{value}, 'id123', 'value is id123' );
};

subtest 'non-keyword name is NAME' => sub {
    # "clk" is a common signal name, not a keyword
    my $tokens = CodingAdventures::VerilogLexer->tokenize('clk');
    is( $tokens->[0]{type},  'NAME', 'type is NAME' );
    is( $tokens->[0]{value}, 'clk',  'value is clk' );
};

# ============================================================================
# String literals
# ============================================================================

subtest 'double-quoted string' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('"hello"');
    is( $tokens->[0]{type},  'STRING',  'type is STRING' );
    is( $tokens->[0]{value}, '"hello"', 'value preserved with quotes' );
};

subtest 'empty double-quoted string' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('""');
    is( $tokens->[0]{type},  'STRING', 'type is STRING' );
    is( $tokens->[0]{value}, '""',     'empty string value' );
};

# ============================================================================
# Composite expressions
# ============================================================================

subtest 'module declaration header' => sub {
    # module adder(input a, input b, output y);
    is(
        types_of('module adder(input a, input b, output y);'),
        [qw(MODULE NAME LPAREN INPUT NAME COMMA INPUT NAME COMMA OUTPUT NAME RPAREN SEMICOLON)],
        'module declaration types'
    );
    my $tokens = CodingAdventures::VerilogLexer->tokenize('module adder(input a);');
    is( $tokens->[1]{value}, 'adder', 'module name is adder' );
};

subtest 'wire declaration with bit range: wire [7:0] bus;' => sub {
    # [7:0] selects 8 bits: bit 7 (MSB) down to bit 0 (LSB)
    is(
        types_of('wire [7:0] bus;'),
        [qw(WIRE LBRACKET NUMBER COLON NUMBER RBRACKET NAME SEMICOLON)],
        'wire bit range types'
    );
};

subtest 'always block with sensitivity list' => sub {
    # always @(posedge clk) triggers on rising clock edge — models flip-flop
    is(
        types_of('always @(posedge clk)'),
        [qw(ALWAYS AT LPAREN POSEDGE NAME RPAREN)],
        'always sensitivity list types'
    );
};

subtest 'non-blocking assignment: q <= d' => sub {
    # Non-blocking assignment: models synchronous register update
    is(
        types_of('q <= d'),
        [qw(NAME LESS_EQUALS NAME)],
        'non-blocking assignment types'
    );
};

subtest 'if/else structure' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('if (a == b) begin end else begin end');
    my ($if_tok) = grep { $_->{type} eq 'IF' } @$tokens;
    ok( $if_tok, 'has IF token' );
    my ($else_tok) = grep { $_->{type} eq 'ELSE' } @$tokens;
    ok( $else_tok, 'has ELSE token' );
};

subtest 'case statement' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('case (sel) endcase');
    my ($case_tok)    = grep { $_->{type} eq 'CASE'    } @$tokens;
    my ($endcase_tok) = grep { $_->{type} eq 'ENDCASE' } @$tokens;
    ok( $case_tok,    'has CASE token' );
    ok( $endcase_tok, 'has ENDCASE token' );
};

subtest 'arithmetic expression: a + b * c' => sub {
    is(
        types_of('a + b * c'),
        [qw(NAME PLUS NAME STAR NAME)],
        'arithmetic expression types'
    );
};

subtest 'bitwise expression: a & b | c ^ d' => sub {
    is(
        types_of('a & b | c ^ d'),
        [qw(NAME AMP NAME PIPE NAME CARET NAME)],
        'bitwise expression types'
    );
};

subtest 'comparison: a != b' => sub {
    is(
        types_of('a != b'),
        [qw(NAME NOT_EQUALS NAME)],
        'not equals types'
    );
};

subtest 'parameter declaration: parameter WIDTH = 8;' => sub {
    is(
        types_of('parameter WIDTH = 8;'),
        [qw(PARAMETER NAME EQUALS NUMBER SEMICOLON)],
        'parameter declaration types'
    );
};

# ============================================================================
# Whitespace handling
# ============================================================================

subtest 'spaces between tokens are consumed silently' => sub {
    is(
        types_of('a = 1'),
        [qw(NAME EQUALS NUMBER)],
        'no WHITESPACE tokens in output'
    );
};

subtest 'tabs and newlines consumed' => sub {
    is(
        types_of("a\t=\n1"),
        [qw(NAME EQUALS NUMBER)],
        'tabs and newlines consumed'
    );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking: a = 42' => sub {
    # a _ = _ 4 2
    # 1 2 3 4 5 6
    my $tokens = CodingAdventures::VerilogLexer->tokenize('a = 42');
    is( $tokens->[0]{col}, 1, 'a at col 1' );
    is( $tokens->[1]{col}, 3, '= at col 3' );
    is( $tokens->[2]{col}, 5, '42 at col 5' );
};

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('module test;');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::VerilogLexer->tokenize('1');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected character raises die' => sub {
    # The backtick without a valid directive name would fail, but let's use
    # a character that is definitely not in the grammar
    ok(
        dies { CodingAdventures::VerilogLexer->tokenize("\x{FFFE}") },
        'unexpected character causes die'
    );
};

done_testing;
