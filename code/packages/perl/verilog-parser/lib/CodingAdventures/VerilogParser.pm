package CodingAdventures::VerilogParser;

# ============================================================================
# CodingAdventures::VerilogParser — Hand-written recursive-descent Verilog parser
# ============================================================================
#
# This module parses a synthesizable subset of Verilog (IEEE 1364-2005) into
# an Abstract Syntax Tree (AST). The parser is hand-written using the
# recursive-descent technique: each grammar rule is one Perl method, and rules
# call each other recursively.
#
# # What is Verilog?
# ==================
#
# Verilog is a Hardware Description Language (HDL). Unlike software languages
# that describe computations executed sequentially on a processor, Verilog
# describes physical structures — gates, wires, flip-flops — that exist
# simultaneously and operate in parallel on a chip.
#
# A Verilog "program" is a collection of modules. Each module is a blueprint
# for a hardware component with named inputs and outputs (ports), internal
# signals, and behavioral or structural descriptions.
#
# # What do we parse?
# ====================
#
# The synthesizable subset — constructs that can be turned into actual hardware:
#
#   module adder #(parameter WIDTH = 8) (
#     input  [WIDTH-1:0] a,
#     input  [WIDTH-1:0] b,
#     output [WIDTH-1:0] sum
#   );
#     assign sum = a + b;   // continuous assignment (combinational logic)
#   endmodule
#
#   module dff(input clk, input d, output reg q);
#     always @(posedge clk)   // sequential logic: fires on clock edge
#       q <= d;               // non-blocking assignment
#   endmodule
#
# # Token types from CodingAdventures::VerilogLexer
# ===================================================
#
# Keywords are emitted as specific uppercase tokens:
#
#   MODULE, ENDMODULE, INPUT, OUTPUT, INOUT
#   WIRE, REG, INTEGER, REAL, SIGNED, UNSIGNED, TRI, SUPPLY0, SUPPLY1
#   ALWAYS, INITIAL, BEGIN, END
#   IF, ELSE, CASE, CASEX, CASEZ, ENDCASE, DEFAULT
#   FOR, ASSIGN, PARAMETER, LOCALPARAM
#   GENERATE, ENDGENERATE, GENVAR
#   POSEDGE, NEGEDGE, OR
#   FUNCTION, ENDFUNCTION, TASK, ENDTASK
#   AND, NAND, NOR, NOT, BUF, XOR, XNOR
#
# Number tokens:
#   SIZED_NUMBER  — e.g. 4'b1010, 8'hFF, 32'd42
#   REAL_NUMBER   — e.g. 3.14, 1.5e-3
#   NUMBER        — plain integers like 42, 1_000
#   STRING        — double-quoted strings
#
# Special identifiers:
#   SYSTEM_ID     — $display, $time, $finish
#   DIRECTIVE     — `define, `ifdef
#   ESCAPED_IDENT — \odd.name
#   NAME          — regular identifier
#
# Operators (multi-char first):
#   ARITH_LEFT_SHIFT (<<<), ARITH_RIGHT_SHIFT (>>>)
#   CASE_EQ (===), CASE_NEQ (!==)
#   LOGIC_AND (&&), LOGIC_OR (||)
#   LEFT_SHIFT (<<), RIGHT_SHIFT (>>)
#   EQUALS_EQUALS (==), NOT_EQUALS (!=)
#   LESS_EQUALS (<=), GREATER_EQUALS (>=), POWER (**)
#
# Single-char operators:
#   PLUS, MINUS, STAR, SLASH, PERCENT
#   AMP, PIPE, CARET, TILDE, BANG
#   LESS_THAN, GREATER_THAN, EQUALS, QUESTION, COLON
#
# Delimiters:
#   LPAREN, RPAREN, LBRACKET, RBRACKET, LBRACE, RBRACE
#   SEMICOLON, COMMA, DOT, HASH, AT
#
# # AST node types (rule_name values)
# ====================================
#
#   source_text            — root; list of descriptions
#   description            — wrapper for module_declaration
#   module_declaration     — module … endmodule
#   parameter_port_list    — #(parameter …)
#   parameter_declaration  — parameter [range] NAME = expr
#   localparam_declaration — localparam [range] NAME = expr
#   port_list              — (port, …)
#   port                   — [direction] [type] [signed] [range] NAME
#   port_direction         — input | output | inout
#   net_type               — wire | reg | tri | supply0 | supply1
#   range                  — [expr:expr]
#   module_item            — net_decl | continuous_assign | always | …
#   port_declaration       — input/output/inout declaration
#   net_declaration        — wire/tri declaration
#   reg_declaration        — reg declaration
#   integer_declaration    — integer declaration
#   name_list              — NAME {, NAME}
#   continuous_assign      — assign lvalue = expr, …;
#   assignment             — lvalue = expr
#   lvalue                 — NAME [range_select] | concatenation
#   range_select           — [expr] or [expr:expr]
#   always_construct       — always @(…) statement
#   initial_construct      — initial statement
#   sensitivity_list       — (* | posedge/negedge expr {, …})
#   sensitivity_item       — [posedge|negedge] expr
#   statement              — block | if | case | for | blocking | nonblocking | task_call | ;
#   block_statement        — begin [: NAME] { statement } end
#   if_statement           — if (expr) stmt [else stmt]
#   case_statement         — case/casex/casez (expr) { case_item } endcase
#   case_item              — expr_list : stmt | default [:]  stmt
#   expression_list        — expr {, expr}
#   for_statement          — for (…) stmt
#   blocking_assignment    — lvalue = expr
#   nonblocking_assignment — lvalue <= expr
#   task_call              — NAME(args)
#   module_instantiation   — NAME [#(params)] instance {, instance} ;
#   instance               — NAME(port_connections)
#   port_connections       — named | positional
#   named_port_connection  — .NAME([expr])
#   generate_region        — generate { item } endgenerate
#   generate_for           — for (…) generate_block
#   generate_if            — if (expr) generate_block [else generate_block]
#   function_declaration   — function [range] NAME ; { item } stmt endfunction
#   task_declaration       — task NAME ; { item } stmt endtask
#   expression             — entry point for full expression grammar
#   ternary_expr, or_expr, and_expr, bit_or_expr, bit_xor_expr, bit_and_expr
#   equality_expr, relational_expr, shift_expr, additive_expr, multiplicative_expr
#   power_expr, unary_expr  — expression precedence levels
#   primary                — atom (number, name, parens, concat, replication)
#   concatenation          — {expr, …}
#   replication            — {expr {expr, …}}
#   token                  — leaf node wrapping a single lexer token
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';
our $DEFAULT_VERSION = '2005';
our @SUPPORTED_VERSIONS = qw(1995 2001 2005);

use CodingAdventures::VerilogLexer;
use CodingAdventures::VerilogParser::ASTNode;

# The lexer is now version-aware and selects edition-specific token grammars.
# The parser core remains handwritten for historical reasons: this package
# landed before the generic grammar-driven Perl parser stack, and it has not
# been migrated over yet.

sub _resolve_version {
    my ($version) = @_;
    return $DEFAULT_VERSION unless defined $version && length $version;
    return $version if grep { $_ eq $version } @SUPPORTED_VERSIONS;
    die sprintf(
        "CodingAdventures::VerilogParser: unknown Verilog version '%s' (expected one of: %s)",
        $version,
        join(', ', @SUPPORTED_VERSIONS)
    );
}

sub default_version {
    return $DEFAULT_VERSION;
}

sub supported_versions {
    return [ @SUPPORTED_VERSIONS ];
}

# ============================================================================
# Constructor
# ============================================================================

# --- new($source) -------------------------------------------------------------
#
# Tokenize `$source` with VerilogLexer and return a ready-to-parse parser.

sub new {
    my ($class, $source, $version) = @_;
    $version = _resolve_version($version);
    my $tokens = CodingAdventures::VerilogLexer->tokenize($source, $version);
    return bless {
        _tokens => $tokens,
        _pos    => 0,
        _version => $version,
    }, $class;
}

# ============================================================================
# Token helpers
# ============================================================================

# Peek at the current token without consuming it.
sub _peek {
    my ($self) = @_;
    return $self->{_tokens}[ $self->{_pos} ]
        // { type => 'EOF', value => '', line => 0, col => 0 };
}

# Peek N positions ahead from the current position.
sub _peek_ahead {
    my ($self, $n) = @_;
    $n //= 0;
    return $self->{_tokens}[ $self->{_pos} + $n ]
        // { type => 'EOF', value => '', line => 0, col => 0 };
}

# Consume and return the current token.
sub _advance {
    my ($self) = @_;
    my $tok = $self->_peek();
    $self->{_pos}++ unless $tok->{type} eq 'EOF';
    return $tok;
}

# Expect a specific token type; die on mismatch.
sub _expect {
    my ($self, $type) = @_;
    my $tok = $self->_peek();
    unless ($tok->{type} eq $type) {
        die sprintf(
            "CodingAdventures::VerilogParser: parse error at line %d col %d: "
          . "expected %s but got %s ('%s')\n",
            $tok->{line}, $tok->{col}, $type, $tok->{type}, $tok->{value}
        );
    }
    return $self->_advance();
}

# Expect a keyword by value (keywords are KEYWORD tokens with specific values).
# In the VerilogLexer, keywords emit their own token type (e.g. MODULE, WIRE).
# This helper accepts a token type directly.
sub _expect_kw {
    my ($self, $kw_type) = @_;
    return $self->_expect($kw_type);
}

# Return 1 if current token matches the given type.
sub _check {
    my ($self, $type, $value) = @_;
    my $tok = $self->_peek();
    return 0 unless $tok->{type} eq $type;
    return 1 unless defined $value;
    return $tok->{value} eq $value;
}

# Consume current token if it matches; otherwise return undef.
sub _match {
    my ($self, $type, $value) = @_;
    return $self->_advance() if $self->_check($type, $value);
    return undef;
}

# Wrap a token as a leaf ASTNode.
sub _leaf {
    my ($self, $tok) = @_;
    return CodingAdventures::VerilogParser::ASTNode->new_leaf($tok);
}

# Create an inner ASTNode.
sub _node {
    my ($self, $rule_name, @children) = @_;
    return CodingAdventures::VerilogParser::ASTNode->new($rule_name, \@children);
}

# Return 1 if the current token is a port direction keyword.
# Port directions: input, output, inout
sub _is_port_direction {
    my ($self) = @_;
    my $t = $self->_peek()->{type};
    return $t eq 'INPUT' || $t eq 'OUTPUT' || $t eq 'INOUT';
}

# Return 1 if the current token is a net type keyword.
# Net types: wire, reg, tri, supply0, supply1
sub _is_net_type {
    my ($self) = @_;
    my $t = $self->_peek()->{type};
    return $t eq 'WIRE' || $t eq 'REG' || $t eq 'TRI'
        || $t eq 'SUPPLY0' || $t eq 'SUPPLY1';
}

# ============================================================================
# Public API
# ============================================================================

sub parse {
    my ($self) = @_;
    return $self->_parse_source_text();
}

# ============================================================================
# Grammar rules
# ============================================================================

# source_text = { description } ;
#
# A Verilog source file contains one or more module declarations.
# We parse until EOF.
sub _parse_source_text {
    my ($self) = @_;
    my @children;
    while (!$self->_check('EOF')) {
        push @children, $self->_parse_description();
    }
    return $self->_node('source_text', @children);
}

# description = module_declaration ;
#
# For this subset we only handle module declarations at the top level.
sub _parse_description {
    my ($self) = @_;
    if ($self->_check('MODULE')) {
        return $self->_node('description', $self->_parse_module_declaration());
    }
    my $tok = $self->_peek();
    die sprintf(
        "CodingAdventures::VerilogParser: parse error at line %d col %d: "
      . "unexpected token %s ('%s') at top level\n",
        $tok->{line}, $tok->{col}, $tok->{type}, $tok->{value}
    );
}

# module_declaration = "module" NAME [ parameter_port_list ]
#                      [ port_list ] SEMICOLON
#                      { module_item }
#                      "endmodule" ;
#
# The MODULE keyword has already been matched to type 'MODULE' by the lexer.
# A module is the fundamental building block of Verilog — like a class in OO
# programming, but representing physical hardware.
sub _parse_module_declaration {
    my ($self) = @_;
    my @ch;

    push @ch, $self->_leaf($self->_expect('MODULE'));
    push @ch, $self->_leaf($self->_expect('NAME'));

    # Optional parameter port list: #(parameter WIDTH = 8, …)
    if ($self->_check('HASH')) {
        push @ch, $self->_parse_parameter_port_list();
    }

    # Optional port list: (input a, output y, …)
    if ($self->_check('LPAREN')) {
        push @ch, $self->_parse_port_list();
    }

    push @ch, $self->_leaf($self->_expect('SEMICOLON'));

    # Module body: zero or more module items (declarations, assigns, always blocks)
    while (!$self->_check('ENDMODULE') && !$self->_check('EOF')) {
        my $item = $self->_try_parse_module_item();
        push @ch, $item if defined $item;
    }

    push @ch, $self->_leaf($self->_expect('ENDMODULE'));
    return $self->_node('module_declaration', @ch);
}

# parameter_port_list = HASH LPAREN parameter_declaration
#                       { COMMA parameter_declaration } RPAREN ;
sub _parse_parameter_port_list {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('HASH'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    push @ch, $self->_parse_parameter_declaration();
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_parameter_declaration();
    }
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    return $self->_node('parameter_port_list', @ch);
}

# parameter_declaration = "parameter" [ range ] NAME EQUALS expression ;
sub _parse_parameter_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('PARAMETER'));
    if ($self->_check('LBRACKET')) {
        push @ch, $self->_parse_range();
    }
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('EQUALS'));
    push @ch, $self->_parse_expression();
    return $self->_node('parameter_declaration', @ch);
}

# port_list = LPAREN port { COMMA port } RPAREN ;
#
# The port list declares all inputs and outputs of the module.
# Modern Verilog (2001+) allows port direction in the list itself:
#   module m(input a, output b);
sub _parse_port_list {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LPAREN'));

    # Handle empty port list
    if (!$self->_check('RPAREN')) {
        push @ch, $self->_parse_port();
        while ($self->_check('COMMA') && !$self->_check('RPAREN')) {
            push @ch, $self->_leaf($self->_advance());
            last if $self->_check('RPAREN');
            push @ch, $self->_parse_port();
        }
    }

    push @ch, $self->_leaf($self->_expect('RPAREN'));
    return $self->_node('port_list', @ch);
}

# port = [ port_direction ] [ net_type ] [ "signed" ] [ range ] NAME ;
sub _parse_port {
    my ($self) = @_;
    my @ch;
    if ($self->_is_port_direction()) {
        push @ch, $self->_node('port_direction', $self->_leaf($self->_advance()));
    }
    if ($self->_is_net_type()) {
        push @ch, $self->_node('net_type', $self->_leaf($self->_advance()));
    }
    if ($self->_check('SIGNED')) {
        push @ch, $self->_leaf($self->_advance());
    }
    if ($self->_check('LBRACKET')) {
        push @ch, $self->_parse_range();
    }
    # Port name — can be NAME or another identifier
    if ($self->_check('NAME')) {
        push @ch, $self->_leaf($self->_advance());
    }
    return $self->_node('port', @ch);
}

# range = LBRACKET expression COLON expression RBRACKET ;
#
# Bit ranges appear in wire/reg declarations, port declarations, and bit-selects.
# The notation [MSB:LSB] specifies the bit positions.
# Example: [7:0] means bits 7 down to 0 (8-bit bus).
# [31:0] is the standard 32-bit range.
sub _parse_range {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LBRACKET'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('COLON'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('RBRACKET'));
    return $self->_node('range', @ch);
}

# _try_parse_module_item
#
# Parse one item from the module body. Returns an ASTNode or undef if we
# can't recognize the current token (in which case we skip it to avoid loops).
sub _try_parse_module_item {
    my ($self) = @_;
    my $type = $self->_peek()->{type};

    # Input/output/inout port declarations (ANSI-style can come here too)
    if ($self->_is_port_direction()) {
        my @ch;
        push @ch, $self->_parse_port_declaration();
        push @ch, $self->_leaf($self->_expect('SEMICOLON'));
        return $self->_node('module_item', @ch);
    }

    # Wire/tri declarations
    if ($type eq 'WIRE' || $type eq 'TRI' || $type eq 'SUPPLY0' || $type eq 'SUPPLY1') {
        my @ch;
        push @ch, $self->_parse_net_declaration();
        push @ch, $self->_leaf($self->_expect('SEMICOLON'));
        return $self->_node('module_item', @ch);
    }

    # Reg declarations
    if ($type eq 'REG') {
        my @ch;
        push @ch, $self->_parse_reg_declaration();
        push @ch, $self->_leaf($self->_expect('SEMICOLON'));
        return $self->_node('module_item', @ch);
    }

    # Integer declarations
    if ($type eq 'INTEGER') {
        my @ch;
        push @ch, $self->_parse_integer_declaration();
        push @ch, $self->_leaf($self->_expect('SEMICOLON'));
        return $self->_node('module_item', @ch);
    }

    # Parameter declarations
    if ($type eq 'PARAMETER') {
        my @ch;
        push @ch, $self->_parse_parameter_declaration();
        push @ch, $self->_leaf($self->_expect('SEMICOLON'));
        return $self->_node('module_item', @ch);
    }

    # Localparam declarations
    if ($type eq 'LOCALPARAM') {
        my @ch;
        push @ch, $self->_parse_localparam_declaration();
        push @ch, $self->_leaf($self->_expect('SEMICOLON'));
        return $self->_node('module_item', @ch);
    }

    # Continuous assignment: assign y = a & b;
    if ($type eq 'ASSIGN') {
        return $self->_node('module_item', $self->_parse_continuous_assign());
    }

    # Always block: always @(posedge clk) begin … end
    if ($type eq 'ALWAYS') {
        return $self->_node('module_item', $self->_parse_always_construct());
    }

    # Initial block: initial begin … end
    if ($type eq 'INITIAL') {
        return $self->_node('module_item', $self->_parse_initial_construct());
    }

    # Generate region: generate … endgenerate
    if ($type eq 'GENERATE') {
        return $self->_node('module_item', $self->_parse_generate_region());
    }

    # Function declaration
    if ($type eq 'FUNCTION') {
        return $self->_node('module_item', $self->_parse_function_declaration());
    }

    # Task declaration
    if ($type eq 'TASK') {
        return $self->_node('module_item', $self->_parse_task_declaration());
    }

    # Module instantiation: NAME [#(params)] instance_name (ports);
    # NAME followed by NAME or HASH suggests a module instantiation.
    if ($type eq 'NAME') {
        return $self->_node('module_item', $self->_parse_module_instantiation());
    }

    # Unknown token — skip to avoid infinite loop
    return $self->_node('module_item', $self->_leaf($self->_advance()));
}

# port_declaration = port_direction [ net_type ] [ "signed" ] [ range ] name_list ;
sub _parse_port_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_node('port_direction', $self->_leaf($self->_advance()));
    if ($self->_is_net_type()) {
        push @ch, $self->_node('net_type', $self->_leaf($self->_advance()));
    }
    if ($self->_check('SIGNED')) {
        push @ch, $self->_leaf($self->_advance());
    }
    if ($self->_check('LBRACKET')) {
        push @ch, $self->_parse_range();
    }
    push @ch, $self->_parse_name_list();
    return $self->_node('port_declaration', @ch);
}

# net_declaration = net_type [ "signed" ] [ range ] name_list ;
sub _parse_net_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_node('net_type', $self->_leaf($self->_advance()));
    if ($self->_check('SIGNED')) {
        push @ch, $self->_leaf($self->_advance());
    }
    if ($self->_check('LBRACKET')) {
        push @ch, $self->_parse_range();
    }
    push @ch, $self->_parse_name_list();
    return $self->_node('net_declaration', @ch);
}

# reg_declaration = "reg" [ "signed" ] [ range ] name_list ;
#
# Note: "reg" in Verilog does NOT always synthesize to a register (flip-flop).
# In a combinational always @(*) block, a reg synthesizes to wires/muxes.
# The name is a historical misnomer.
sub _parse_reg_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('REG'));
    if ($self->_check('SIGNED')) {
        push @ch, $self->_leaf($self->_advance());
    }
    if ($self->_check('LBRACKET')) {
        push @ch, $self->_parse_range();
    }
    push @ch, $self->_parse_name_list();
    return $self->_node('reg_declaration', @ch);
}

# integer_declaration = "integer" name_list ;
#
# Integers are 32-bit signed values. They are used for loop counters and
# intermediate calculations in behavioral code — not typically synthesized
# to hardware registers.
sub _parse_integer_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('INTEGER'));
    push @ch, $self->_parse_name_list();
    return $self->_node('integer_declaration', @ch);
}

# localparam_declaration = "localparam" [ range ] NAME EQUALS expression ;
sub _parse_localparam_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LOCALPARAM'));
    if ($self->_check('LBRACKET')) {
        push @ch, $self->_parse_range();
    }
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('EQUALS'));
    push @ch, $self->_parse_expression();
    return $self->_node('localparam_declaration', @ch);
}

# name_list = NAME { COMMA NAME } ;
sub _parse_name_list {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NAME'));
    while ($self->_check('COMMA') && $self->_peek_ahead(1)->{type} eq 'NAME') {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_leaf($self->_expect('NAME'));
    }
    return $self->_node('name_list', @ch);
}

# continuous_assign = "assign" assignment { COMMA assignment } SEMICOLON ;
#
# Continuous assignments model combinational logic. The right side is always
# evaluated and drives the left side. This is like a permanent connection.
sub _parse_continuous_assign {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('ASSIGN'));
    push @ch, $self->_parse_assignment();
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_assignment();
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('continuous_assign', @ch);
}

# assignment = lvalue EQUALS expression ;
sub _parse_assignment {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_lvalue();
    push @ch, $self->_leaf($self->_expect('EQUALS'));
    push @ch, $self->_parse_expression();
    return $self->_node('assignment', @ch);
}

# lvalue = NAME [ range_select ] | concatenation ;
#
# An lvalue is anything that can appear on the left side of an assignment.
# It can be a signal name, a bit-select, a part-select, or a concatenation
# of any of the above.
sub _parse_lvalue {
    my ($self) = @_;
    my @ch;
    if ($self->_check('LBRACE')) {
        push @ch, $self->_parse_concatenation();
    } else {
        push @ch, $self->_leaf($self->_expect('NAME'));
        if ($self->_check('LBRACKET')) {
            push @ch, $self->_parse_range_select();
        }
    }
    return $self->_node('lvalue', @ch);
}

# range_select = LBRACKET expression [ COLON expression ] RBRACKET ;
#
# Bit-select: a[3]         → single bit at index 3
# Part-select: a[7:4]     → bits 7 down to 4 (4-bit slice)
sub _parse_range_select {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LBRACKET'));
    push @ch, $self->_parse_expression();
    if ($self->_check('COLON')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expression();
    }
    push @ch, $self->_leaf($self->_expect('RBRACKET'));
    return $self->_node('range_select', @ch);
}

# always_construct = "always" AT sensitivity_list statement ;
#
# Always blocks are the heart of behavioral Verilog. They describe what
# happens when signals change. Two major uses:
#
# Sequential (clock-driven):
#   always @(posedge clk) begin
#     q <= d;   // non-blocking: all updates happen simultaneously
#   end
#
# Combinational (any-input-driven):
#   always @(*) begin
#     y = a & b;   // blocking: like normal software assignment
#   end
sub _parse_always_construct {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('ALWAYS'));
    push @ch, $self->_leaf($self->_expect('AT'));
    push @ch, $self->_parse_sensitivity_list();
    push @ch, $self->_parse_statement();
    return $self->_node('always_construct', @ch);
}

# initial_construct = "initial" statement ;
#
# Initial blocks execute once at simulation start. They are NOT synthesizable
# but are common in testbenches. We parse them to avoid errors on simulation code.
sub _parse_initial_construct {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('INITIAL'));
    push @ch, $self->_parse_statement();
    return $self->_node('initial_construct', @ch);
}

# sensitivity_list = LPAREN STAR RPAREN
#                  | LPAREN sensitivity_item { (or | COMMA) sensitivity_item } RPAREN ;
#
# The sensitivity list specifies WHEN the always block executes.
# @(*) is a wildcard: sensitive to all signals read in the block.
sub _parse_sensitivity_list {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LPAREN'));

    if ($self->_check('STAR')) {
        # @(*) — sensitive to all signals (combinational shorthand)
        push @ch, $self->_leaf($self->_advance());
    } else {
        push @ch, $self->_parse_sensitivity_item();
        # Multiple sensitivity items separated by 'or' or ','
        while ($self->_check('OR') || $self->_check('COMMA')) {
            push @ch, $self->_leaf($self->_advance());
            push @ch, $self->_parse_sensitivity_item();
        }
    }

    push @ch, $self->_leaf($self->_expect('RPAREN'));
    return $self->_node('sensitivity_list', @ch);
}

# sensitivity_item = [ "posedge" | "negedge" ] expression ;
#
# posedge: triggers on rising clock edge (0→1)
# negedge: triggers on falling edge (1→0)
# No edge keyword: triggers on any value change
sub _parse_sensitivity_item {
    my ($self) = @_;
    my @ch;
    if ($self->_check('POSEDGE') || $self->_check('NEGEDGE')) {
        push @ch, $self->_leaf($self->_advance());
    }
    push @ch, $self->_parse_expression();
    return $self->_node('sensitivity_item', @ch);
}

# ============================================================================
# Statements
# ============================================================================

# statement = block_statement | if_statement | case_statement | for_statement
#           | blocking_assignment SEMICOLON | nonblocking_assignment SEMICOLON
#           | task_call SEMICOLON | SEMICOLON ;
sub _parse_statement {
    my ($self) = @_;
    my $type = $self->_peek()->{type};

    # begin … end block
    if ($type eq 'BEGIN') {
        return $self->_parse_block_statement();
    }

    # if statement
    if ($type eq 'IF') {
        return $self->_parse_if_statement();
    }

    # case/casex/casez statement
    if ($type eq 'CASE' || $type eq 'CASEX' || $type eq 'CASEZ') {
        return $self->_parse_case_statement();
    }

    # for loop
    if ($type eq 'FOR') {
        return $self->_parse_for_statement();
    }

    # Empty statement: just a semicolon
    if ($type eq 'SEMICOLON') {
        return $self->_node('statement', $self->_leaf($self->_advance()));
    }

    # NAME followed by LPAREN → task call
    # NAME followed by LBRACKET EQUALS or EQUALS → blocking assignment
    # NAME followed by LESS_EQUALS → nonblocking assignment
    # NAME followed by LBRACKET ... LESS_EQUALS → nonblocking with bit-select
    if ($type eq 'NAME' || $type eq 'LBRACE') {
        # Lookahead to determine assignment type
        my $lv = $self->_parse_lvalue();
        if ($self->_check('EQUALS')) {
            # Blocking assignment: lvalue = expr ;
            my @ch = ($lv);
            push @ch, $self->_leaf($self->_advance());   # =
            push @ch, $self->_parse_expression();
            push @ch, $self->_leaf($self->_expect('SEMICOLON'));
            my $assign = $self->_node('blocking_assignment', @ch);
            return $self->_node('statement', $assign);
        } elsif ($self->_check('LESS_EQUALS')) {
            # Non-blocking assignment: lvalue <= expr ;
            # This models hardware flip-flops: all LHS updates happen together
            # at the end of the simulation step, not immediately.
            my @ch = ($lv);
            push @ch, $self->_leaf($self->_advance());   # <=
            push @ch, $self->_parse_expression();
            push @ch, $self->_leaf($self->_expect('SEMICOLON'));
            my $assign = $self->_node('nonblocking_assignment', @ch);
            return $self->_node('statement', $assign);
        } elsif ($self->_check('LPAREN')) {
            # Task call: NAME(args);
            my @ch = ($lv);
            push @ch, $self->_leaf($self->_expect('LPAREN'));
            while (!$self->_check('RPAREN') && !$self->_check('EOF')) {
                push @ch, $self->_parse_expression();
                last unless $self->_check('COMMA');
                push @ch, $self->_leaf($self->_advance());
            }
            push @ch, $self->_leaf($self->_expect('RPAREN'));
            push @ch, $self->_leaf($self->_expect('SEMICOLON'));
            return $self->_node('statement', $self->_node('task_call', @ch));
        }
        # Fallthrough: treat lvalue as a statement on its own
        return $self->_node('statement', $lv);
    }

    # System task call: $display(…);
    if ($type eq 'SYSTEM_ID') {
        my @ch;
        push @ch, $self->_leaf($self->_advance());
        if ($self->_check('LPAREN')) {
            push @ch, $self->_leaf($self->_advance());
            while (!$self->_check('RPAREN') && !$self->_check('EOF')) {
                push @ch, $self->_parse_expression();
                last unless $self->_check('COMMA');
                push @ch, $self->_leaf($self->_advance());
            }
            push @ch, $self->_leaf($self->_expect('RPAREN'));
        }
        push @ch, $self->_leaf($self->_expect('SEMICOLON'));
        return $self->_node('statement', $self->_node('task_call', @ch));
    }

    # Unknown — skip one token
    return $self->_node('statement', $self->_leaf($self->_advance()));
}

# block_statement = "begin" [ COLON NAME ] { statement } "end" ;
#
# Begin/end groups multiple statements (like { } in C).
# Named blocks can be referenced for disable statements:
#   begin : my_block
#     …
#   end
sub _parse_block_statement {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('BEGIN'));
    if ($self->_check('COLON')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_leaf($self->_expect('NAME'));
    }
    while (!$self->_check('END') && !$self->_check('EOF')) {
        push @ch, $self->_parse_statement();
    }
    push @ch, $self->_leaf($self->_expect('END'));
    return $self->_node('block_statement', @ch);
}

# if_statement = "if" LPAREN expression RPAREN statement [ "else" statement ] ;
sub _parse_if_statement {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('IF'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    push @ch, $self->_parse_statement();
    if ($self->_check('ELSE')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_statement();
    }
    return $self->_node('if_statement', @ch);
}

# case_statement = ( "case" | "casex" | "casez" )
#                  LPAREN expression RPAREN
#                  { case_item }
#                  "endcase" ;
sub _parse_case_statement {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_advance());    # CASE / CASEX / CASEZ
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    while (!$self->_check('ENDCASE') && !$self->_check('EOF')) {
        push @ch, $self->_parse_case_item();
    }
    push @ch, $self->_leaf($self->_expect('ENDCASE'));
    return $self->_node('case_statement', @ch);
}

# case_item = expression_list COLON statement
#           | "default" [ COLON ] statement ;
sub _parse_case_item {
    my ($self) = @_;
    my @ch;
    if ($self->_check('DEFAULT')) {
        push @ch, $self->_leaf($self->_advance());
        if ($self->_check('COLON')) {
            push @ch, $self->_leaf($self->_advance());
        }
    } else {
        push @ch, $self->_parse_expression_list();
        push @ch, $self->_leaf($self->_expect('COLON'));
    }
    push @ch, $self->_parse_statement();
    return $self->_node('case_item', @ch);
}

# expression_list = expression { COMMA expression } ;
sub _parse_expression_list {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_expression();
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expression();
    }
    return $self->_node('expression_list', @ch);
}

# for_statement = "for" LPAREN blocking_assignment SEMICOLON
#                 expression SEMICOLON blocking_assignment RPAREN statement ;
sub _parse_for_statement {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('FOR'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));

    # Init: blocking assignment
    my $init_lv = $self->_parse_lvalue();
    my @init_ch = ($init_lv, $self->_leaf($self->_expect('EQUALS')),
                   $self->_parse_expression());
    push @ch, $self->_node('blocking_assignment', @init_ch);
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));

    # Condition
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));

    # Update: blocking assignment
    my $upd_lv = $self->_parse_lvalue();
    my @upd_ch = ($upd_lv, $self->_leaf($self->_expect('EQUALS')),
                  $self->_parse_expression());
    push @ch, $self->_node('blocking_assignment', @upd_ch);
    push @ch, $self->_leaf($self->_expect('RPAREN'));

    push @ch, $self->_parse_statement();
    return $self->_node('for_statement', @ch);
}

# ============================================================================
# Module instantiation
# ============================================================================

# module_instantiation = NAME [ parameter_value_assignment ]
#                        instance { COMMA instance } SEMICOLON ;
#
# Module instantiation creates an instance of another module.
# Example: adder #(.WIDTH(8)) u1 (.a(sig_a), .b(sig_b), .sum(result));
sub _parse_module_instantiation {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NAME'));   # module type name

    # Optional parameter override: #(…)
    if ($self->_check('HASH')) {
        push @ch, $self->_parse_parameter_value_assignment();
    }

    # Instance(s)
    push @ch, $self->_parse_instance();
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_instance();
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('module_instantiation', @ch);
}

# parameter_value_assignment = HASH LPAREN expression { COMMA expression } RPAREN ;
sub _parse_parameter_value_assignment {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('HASH'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    unless ($self->_check('RPAREN')) {
        push @ch, $self->_parse_expression();
        while ($self->_check('COMMA')) {
            push @ch, $self->_leaf($self->_advance());
            push @ch, $self->_parse_expression();
        }
    }
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    return $self->_node('parameter_value_assignment', @ch);
}

# instance = NAME LPAREN port_connections RPAREN ;
sub _parse_instance {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    push @ch, $self->_parse_port_connections();
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    return $self->_node('instance', @ch);
}

# port_connections = named_port_connection { COMMA named_port_connection }
#                  | [ expression { COMMA expression } ] ;
sub _parse_port_connections {
    my ($self) = @_;
    my @ch;
    if ($self->_check('RPAREN')) {
        return $self->_node('port_connections');
    }
    if ($self->_check('DOT')) {
        # Named port connections: .port_name(signal_name)
        push @ch, $self->_parse_named_port_connection();
        while ($self->_check('COMMA')) {
            push @ch, $self->_leaf($self->_advance());
            last if $self->_check('RPAREN');
            push @ch, $self->_parse_named_port_connection();
        }
    } else {
        # Positional port connections: expr, expr, …
        push @ch, $self->_parse_expression();
        while ($self->_check('COMMA')) {
            push @ch, $self->_leaf($self->_advance());
            last if $self->_check('RPAREN');
            push @ch, $self->_parse_expression();
        }
    }
    return $self->_node('port_connections', @ch);
}

# named_port_connection = DOT NAME LPAREN [ expression ] RPAREN ;
#
# Named port connections are explicit and order-independent:
#   .a(sig_a)   — connect port 'a' to signal 'sig_a'
#   .b()        — leave port 'b' unconnected
sub _parse_named_port_connection {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('DOT'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    unless ($self->_check('RPAREN')) {
        push @ch, $self->_parse_expression();
    }
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    return $self->_node('named_port_connection', @ch);
}

# ============================================================================
# Generate regions
# ============================================================================

# generate_region = "generate" { generate_item } "endgenerate" ;
sub _parse_generate_region {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('GENERATE'));
    while (!$self->_check('ENDGENERATE') && !$self->_check('EOF')) {
        if ($self->_check('GENVAR')) {
            my @gc;
            push @gc, $self->_leaf($self->_advance());   # genvar
            push @gc, $self->_leaf($self->_expect('NAME'));
            while ($self->_check('COMMA')) {
                push @gc, $self->_leaf($self->_advance());
                push @gc, $self->_leaf($self->_expect('NAME'));
            }
            push @gc, $self->_leaf($self->_expect('SEMICOLON'));
            push @ch, $self->_node('genvar_declaration', @gc);
        } elsif ($self->_check('FOR')) {
            push @ch, $self->_parse_generate_for();
        } elsif ($self->_check('IF')) {
            push @ch, $self->_parse_generate_if();
        } else {
            my $item = $self->_try_parse_module_item();
            push @ch, $item if defined $item;
        }
    }
    push @ch, $self->_leaf($self->_expect('ENDGENERATE'));
    return $self->_node('generate_region', @ch);
}

# generate_for = "for" LPAREN genvar_assignment SEMICOLON expression
#                SEMICOLON genvar_assignment RPAREN generate_block ;
sub _parse_generate_for {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('FOR'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    # genvar init
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('EQUALS'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    # genvar update
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('EQUALS'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    push @ch, $self->_parse_generate_block();
    return $self->_node('generate_for', @ch);
}

# generate_if = "if" LPAREN expression RPAREN generate_block [ "else" generate_block ] ;
sub _parse_generate_if {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('IF'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    push @ch, $self->_parse_generate_block();
    if ($self->_check('ELSE')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_generate_block();
    }
    return $self->_node('generate_if', @ch);
}

# generate_block = "begin" [ COLON NAME ] { generate_item } "end"
#                | generate_item ;
sub _parse_generate_block {
    my ($self) = @_;
    if ($self->_check('BEGIN')) {
        my @ch;
        push @ch, $self->_leaf($self->_advance());
        if ($self->_check('COLON')) {
            push @ch, $self->_leaf($self->_advance());
            push @ch, $self->_leaf($self->_expect('NAME'));
        }
        while (!$self->_check('END') && !$self->_check('EOF')) {
            my $item = $self->_try_parse_module_item();
            push @ch, $item if defined $item;
        }
        push @ch, $self->_leaf($self->_expect('END'));
        return $self->_node('generate_block', @ch);
    }
    my $item = $self->_try_parse_module_item();
    return $self->_node('generate_block', defined $item ? ($item) : ());
}

# ============================================================================
# Functions and Tasks
# ============================================================================

# function_declaration = "function" [ range ] NAME SEMICOLON
#                        { function_item }
#                        statement
#                        "endfunction" ;
sub _parse_function_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('FUNCTION'));
    if ($self->_check('LBRACKET')) {
        push @ch, $self->_parse_range();
    }
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    # Function items: port/reg/integer/parameter declarations
    while (!$self->_check('BEGIN') && !$self->_check('ENDFUNCTION') && !$self->_check('EOF')) {
        if ($self->_is_port_direction()) {
            my @ic;
            push @ic, $self->_parse_port_declaration();
            push @ic, $self->_leaf($self->_expect('SEMICOLON'));
            push @ch, $self->_node('function_item', @ic);
        } elsif ($self->_check('REG')) {
            my @ic;
            push @ic, $self->_parse_reg_declaration();
            push @ic, $self->_leaf($self->_expect('SEMICOLON'));
            push @ch, $self->_node('function_item', @ic);
        } elsif ($self->_check('INTEGER')) {
            my @ic;
            push @ic, $self->_parse_integer_declaration();
            push @ic, $self->_leaf($self->_expect('SEMICOLON'));
            push @ch, $self->_node('function_item', @ic);
        } else {
            last;
        }
    }
    push @ch, $self->_parse_statement();
    push @ch, $self->_leaf($self->_expect('ENDFUNCTION'));
    return $self->_node('function_declaration', @ch);
}

# task_declaration = "task" NAME SEMICOLON
#                    { task_item }
#                    statement
#                    "endtask" ;
sub _parse_task_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('TASK'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    # Task items
    while (!$self->_check('BEGIN') && !$self->_check('ENDTASK') && !$self->_check('EOF')) {
        if ($self->_is_port_direction()) {
            my @ic;
            push @ic, $self->_parse_port_declaration();
            push @ic, $self->_leaf($self->_expect('SEMICOLON'));
            push @ch, $self->_node('task_item', @ic);
        } elsif ($self->_check('REG')) {
            my @ic;
            push @ic, $self->_parse_reg_declaration();
            push @ic, $self->_leaf($self->_expect('SEMICOLON'));
            push @ch, $self->_node('task_item', @ic);
        } elsif ($self->_check('INTEGER')) {
            my @ic;
            push @ic, $self->_parse_integer_declaration();
            push @ic, $self->_leaf($self->_expect('SEMICOLON'));
            push @ch, $self->_node('task_item', @ic);
        } else {
            last;
        }
    }
    push @ch, $self->_parse_statement();
    push @ch, $self->_leaf($self->_expect('ENDTASK'));
    return $self->_node('task_declaration', @ch);
}

# ============================================================================
# Expressions — full Verilog operator precedence
# ============================================================================
#
# Verilog operator precedence (lowest to highest):
#   1. ?:              Ternary conditional
#   2. ||              Logical OR
#   3. &&              Logical AND
#   4. |               Bitwise OR
#   5. ^ ~^ ^~         Bitwise XOR/XNOR
#   6. &               Bitwise AND
#   7. == != === !==   Equality
#   8. < <= > >=       Relational
#   9. << >> <<< >>>   Shift
#   10. + -            Addition/subtraction
#   11. * / %          Multiplication/division/modulo
#   12. **             Power
#   13. unary          ! ~ + - & | ^ (unary/reduction)
#
# Each rule calls the next higher precedence rule for operands.

# expression = ternary_expr ;
sub _parse_expression {
    my ($self) = @_;
    return $self->_node('expression', $self->_parse_ternary_expr());
}

# ternary_expr = or_expr [ QUESTION expression COLON ternary_expr ] ;
#
# The ternary operator: sel ? true_val : false_val
# In hardware, this is a multiplexer (MUX).
sub _parse_ternary_expr {
    my ($self) = @_;
    my $cond = $self->_parse_or_expr();
    if ($self->_check('QUESTION')) {
        my $q       = $self->_leaf($self->_advance());
        my $true_e  = $self->_parse_expression();
        my $colon   = $self->_leaf($self->_expect('COLON'));
        my $false_e = $self->_parse_ternary_expr();
        return $self->_node('ternary_expr', $cond, $q, $true_e, $colon, $false_e);
    }
    return $cond;
}

# or_expr = and_expr { LOGIC_OR and_expr } ;
sub _parse_or_expr {
    my ($self) = @_;
    my $left = $self->_parse_and_expr();
    while ($self->_check('LOGIC_OR')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_and_expr();
        $left = $self->_node('or_expr', $left, $op, $right);
    }
    return $left;
}

# and_expr = bit_or_expr { LOGIC_AND bit_or_expr } ;
sub _parse_and_expr {
    my ($self) = @_;
    my $left = $self->_parse_bit_or_expr();
    while ($self->_check('LOGIC_AND')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_bit_or_expr();
        $left = $self->_node('and_expr', $left, $op, $right);
    }
    return $left;
}

# bit_or_expr = bit_xor_expr { PIPE bit_xor_expr } ;
sub _parse_bit_or_expr {
    my ($self) = @_;
    my $left = $self->_parse_bit_xor_expr();
    while ($self->_check('PIPE')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_bit_xor_expr();
        $left = $self->_node('bit_or_expr', $left, $op, $right);
    }
    return $left;
}

# bit_xor_expr = bit_and_expr { CARET bit_and_expr } ;
sub _parse_bit_xor_expr {
    my ($self) = @_;
    my $left = $self->_parse_bit_and_expr();
    while ($self->_check('CARET')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_bit_and_expr();
        $left = $self->_node('bit_xor_expr', $left, $op, $right);
    }
    return $left;
}

# bit_and_expr = equality_expr { AMP equality_expr } ;
sub _parse_bit_and_expr {
    my ($self) = @_;
    my $left = $self->_parse_equality_expr();
    while ($self->_check('AMP')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_equality_expr();
        $left = $self->_node('bit_and_expr', $left, $op, $right);
    }
    return $left;
}

# equality_expr = relational_expr { (EQUALS_EQUALS|NOT_EQUALS|CASE_EQ|CASE_NEQ) relational_expr } ;
sub _parse_equality_expr {
    my ($self) = @_;
    my $left = $self->_parse_relational_expr();
    while ($self->_check('EQUALS_EQUALS') || $self->_check('NOT_EQUALS')
           || $self->_check('CASE_EQ') || $self->_check('CASE_NEQ')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_relational_expr();
        $left = $self->_node('equality_expr', $left, $op, $right);
    }
    return $left;
}

# relational_expr = shift_expr { (LESS_THAN|LESS_EQUALS|GREATER_THAN|GREATER_EQUALS) shift_expr } ;
#
# Note: LESS_EQUALS here is comparison (<= meaning ≤).
# When LESS_EQUALS appears in statement context (lvalue <= expr), it is a
# non-blocking assignment — but here in expression context it is comparison.
sub _parse_relational_expr {
    my ($self) = @_;
    my $left = $self->_parse_shift_expr();
    while ($self->_check('LESS_THAN') || $self->_check('LESS_EQUALS')
           || $self->_check('GREATER_THAN') || $self->_check('GREATER_EQUALS')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_shift_expr();
        $left = $self->_node('relational_expr', $left, $op, $right);
    }
    return $left;
}

# shift_expr = additive_expr { (LEFT_SHIFT|RIGHT_SHIFT|ARITH_LEFT_SHIFT|ARITH_RIGHT_SHIFT) additive_expr } ;
sub _parse_shift_expr {
    my ($self) = @_;
    my $left = $self->_parse_additive_expr();
    while ($self->_check('LEFT_SHIFT') || $self->_check('RIGHT_SHIFT')
           || $self->_check('ARITH_LEFT_SHIFT') || $self->_check('ARITH_RIGHT_SHIFT')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_additive_expr();
        $left = $self->_node('shift_expr', $left, $op, $right);
    }
    return $left;
}

# additive_expr = multiplicative_expr { (PLUS|MINUS) multiplicative_expr } ;
sub _parse_additive_expr {
    my ($self) = @_;
    my $left = $self->_parse_multiplicative_expr();
    while ($self->_check('PLUS') || $self->_check('MINUS')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_multiplicative_expr();
        $left = $self->_node('additive_expr', $left, $op, $right);
    }
    return $left;
}

# multiplicative_expr = power_expr { (STAR|SLASH|PERCENT) power_expr } ;
sub _parse_multiplicative_expr {
    my ($self) = @_;
    my $left = $self->_parse_power_expr();
    while ($self->_check('STAR') || $self->_check('SLASH') || $self->_check('PERCENT')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_power_expr();
        $left = $self->_node('multiplicative_expr', $left, $op, $right);
    }
    return $left;
}

# power_expr = unary_expr [ POWER unary_expr ] ;
sub _parse_power_expr {
    my ($self) = @_;
    my $base = $self->_parse_unary_expr();
    if ($self->_check('POWER')) {
        my $op  = $self->_leaf($self->_advance());
        my $exp = $self->_parse_unary_expr();
        return $self->_node('power_expr', $base, $op, $exp);
    }
    return $base;
}

# unary_expr = (PLUS|MINUS|BANG|TILDE|AMP|PIPE|CARET) unary_expr | primary ;
#
# Unary operators in Verilog:
#   +x  -x   — sign
#   !x        — logical NOT (result is 1 bit)
#   ~x        — bitwise NOT (flip every bit)
#   &x  ~&x  — reduction AND / NAND (AND all bits → 1 bit)
#   |x  ~|x  — reduction OR / NOR
#   ^x  ~^x  — reduction XOR / XNOR
sub _parse_unary_expr {
    my ($self) = @_;
    my $type = $self->_peek()->{type};
    if ($type eq 'PLUS' || $type eq 'MINUS' || $type eq 'BANG'
        || $type eq 'TILDE' || $type eq 'AMP' || $type eq 'PIPE'
        || $type eq 'CARET') {
        my $op   = $self->_leaf($self->_advance());
        my $expr = $self->_parse_unary_expr();
        return $self->_node('unary_expr', $op, $expr);
    }
    return $self->_parse_primary();
}

# primary = NUMBER | SIZED_NUMBER | REAL_NUMBER | STRING
#         | NAME [ LBRACKET expr [COLON expr] RBRACKET ]
#         | NAME LPAREN [expr {COMMA expr}] RPAREN
#         | SYSTEM_ID
#         | LPAREN expression RPAREN
#         | concatenation
#         | replication ;
sub _parse_primary {
    my ($self) = @_;
    my $type = $self->_peek()->{type};

    # Number literals
    if ($type eq 'NUMBER' || $type eq 'SIZED_NUMBER' || $type eq 'REAL_NUMBER') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }

    # String literal
    if ($type eq 'STRING') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }

    # System task/function identifier: $display, $time, …
    if ($type eq 'SYSTEM_ID') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }

    # Parenthesized expression
    if ($type eq 'LPAREN') {
        my @ch;
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expression();
        push @ch, $self->_leaf($self->_expect('RPAREN'));
        return $self->_node('primary', @ch);
    }

    # Concatenation { } or replication { N { … } }
    if ($type eq 'LBRACE') {
        return $self->_parse_concat_or_replication();
    }

    # NAME — identifier, bit-select, or function call
    if ($type eq 'NAME') {
        my $name_leaf = $self->_leaf($self->_advance());
        # Bit-select or part-select: a[3] or a[7:4]
        if ($self->_check('LBRACKET')) {
            my @ch = ($name_leaf, $self->_leaf($self->_advance()));
            push @ch, $self->_parse_expression();
            if ($self->_check('COLON')) {
                push @ch, $self->_leaf($self->_advance());
                push @ch, $self->_parse_expression();
            }
            push @ch, $self->_leaf($self->_expect('RBRACKET'));
            return $self->_node('primary', @ch);
        }
        # Function call: name(args)
        if ($self->_check('LPAREN')) {
            my @ch = ($name_leaf, $self->_leaf($self->_advance()));
            unless ($self->_check('RPAREN')) {
                push @ch, $self->_parse_expression();
                while ($self->_check('COMMA')) {
                    push @ch, $self->_leaf($self->_advance());
                    push @ch, $self->_parse_expression();
                }
            }
            push @ch, $self->_leaf($self->_expect('RPAREN'));
            return $self->_node('primary', @ch);
        }
        return $self->_node('primary', $name_leaf);
    }

    # Fallthrough: return a leaf for the unexpected token
    return $self->_node('primary', $self->_leaf($self->_advance()));
}

# _parse_concat_or_replication
#
# Handles both concatenation {a, b} and replication {4{1'b0}}.
# A replication has an expression immediately followed by {…}.
# A concatenation has comma-separated expressions.
sub _parse_concat_or_replication {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LBRACE'));

    my $first = $self->_parse_expression();

    if ($self->_check('LBRACE')) {
        # Replication: { N { expr, … } }
        push @ch, $first;
        push @ch, $self->_parse_concatenation_body();
    } else {
        # Concatenation: { expr, expr, … }
        push @ch, $first;
        while ($self->_check('COMMA')) {
            push @ch, $self->_leaf($self->_advance());
            push @ch, $self->_parse_expression();
        }
    }

    push @ch, $self->_leaf($self->_expect('RBRACE'));
    return $self->_node('concatenation', @ch);
}

# _parse_concatenation_body — parse { expr, … } including braces
sub _parse_concatenation_body {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LBRACE'));
    push @ch, $self->_parse_expression();
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expression();
    }
    push @ch, $self->_leaf($self->_expect('RBRACE'));
    return $self->_node('concatenation', @ch);
}

# ============================================================================
# Class-method convenience wrapper
# ============================================================================

# --- parse_verilog($source) ---------------------------------------------------
#
# Convenience class method: tokenize and parse in one call.
# Returns the root ASTNode. Dies on error.

sub parse_verilog {
    my ($class, $source, $version) = @_;
    my $parser = $class->new($source, $version);
    return $parser->parse();
}

1;

__END__

=head1 NAME

CodingAdventures::VerilogParser - Hand-written recursive-descent Verilog parser

=head1 SYNOPSIS

    use CodingAdventures::VerilogParser;

    # Object-oriented
    my $parser = CodingAdventures::VerilogParser->new("module empty; endmodule");
    my $ast    = $parser->parse();
    print $ast->rule_name;   # "source_text"

    # Convenience class method
    my $ast = CodingAdventures::VerilogParser->parse_verilog(<<'VERILOG');
    module and_gate(input a, input b, output y);
      assign y = a & b;
    endmodule
    VERILOG

=head1 DESCRIPTION

A hand-written recursive-descent parser for the synthesizable subset of
Verilog (IEEE 1364-2005). Tokenizes input with C<CodingAdventures::VerilogLexer>
and builds an Abstract Syntax Tree (AST) of
C<CodingAdventures::VerilogParser::ASTNode> nodes.

Covers: module declarations, port lists, wire/reg declarations, continuous
assignments, always blocks, if/case/for statements, module instantiation,
generate blocks, functions, tasks, and the complete Verilog expression
grammar with correct operator precedence.

=head1 METHODS

=head2 new($source)

Tokenize C<$source> with C<VerilogLexer> and return a parser instance.

=head2 parse()

Parse and return the root AST node (rule_name C<"source_text">). Dies on error.

=head2 parse_verilog($source)

Class method — tokenize and parse in one call. Returns the root ASTNode.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
