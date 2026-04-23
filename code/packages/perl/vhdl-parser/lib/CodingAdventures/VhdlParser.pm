package CodingAdventures::VhdlParser;

# ============================================================================
# CodingAdventures::VhdlParser — Hand-written recursive-descent VHDL parser
# ============================================================================
#
# This module parses a synthesizable subset of VHDL (IEEE 1076-2008) into
# an Abstract Syntax Tree (AST). The parser is hand-written using the
# recursive-descent technique: each grammar rule is one Perl method.
#
# # What is VHDL?
# ===============
#
# VHDL (VHSIC Hardware Description Language) is an HDL used to describe,
# simulate, and synthesize digital circuits. It takes a fundamentally more
# explicit approach than Verilog: every signal must be declared with its type,
# every entity must have a separate architecture, every port must specify its
# direction and type.
#
# VHDL's verbosity is intentional. The strong type system catches errors
# at compile time that would be silent bugs in Verilog: connecting an 8-bit
# signal to a 16-bit port is a compile error, not a silent truncation.
#
# # VHDL Terminology
# ==================
#
#   entity        — the INTERFACE of a hardware block (ports, generics)
#   architecture  — the IMPLEMENTATION (behavioral or structural description)
#   signal        — a physical wire or register
#   variable      — local to a process; assignment is immediate
#   process       — a sequential region inside the concurrent architecture
#   port map      — connects instance ports to signals
#   generic       — compile-time parameter (like Verilog's parameter)
#   library/use   — import packages (IEEE.std_logic_1164.all)
#
# VHDL separates interface from implementation, allowing multiple architectures
# for the same entity:
#
#   entity full_adder is
#     port (a, b, cin : in std_logic; sum, cout : out std_logic);
#   end entity full_adder;
#
#   architecture behavioral of full_adder is
#   begin
#     sum  <= a xor b xor cin;
#     cout <= (a and b) or ((a xor b) and cin);
#   end architecture behavioral;
#
# # What do we parse?
# ====================
#
# The synthesizable subset (IEEE 1076-2008):
#   - Design files: context clauses (library/use) + design units
#   - Entity declarations: generics and ports
#   - Architecture bodies: signal/constant/type declarations + concurrent statements
#   - Concurrent statements: processes, signal assignments, component instantiation
#   - Sequential statements: signal/variable assignments, if/elsif/else, case/when,
#     for/while loops, return, null
#   - Package declarations and bodies
#   - Function and procedure declarations and bodies
#   - Full VHDL expression grammar
#
# # Token types from CodingAdventures::VhdlLexer
# ===============================================
#
# VHDL is case-insensitive; the lexer normalizes to lowercase.
# Keywords emit specific uppercase token types:
#
#   ENTITY, IS, END, PORT, GENERIC, IN, OUT, INOUT, BUFFER
#   ARCHITECTURE, OF, BEGIN, SIGNAL, CONSTANT, VARIABLE, TYPE, SUBTYPE
#   PROCESS, IF, THEN, ELSIF, ELSE, CASE, WHEN, FOR, WHILE, LOOP,
#   RETURN, NULL, OTHERS, GENERATE, COMPONENT, LIBRARY, USE, PACKAGE,
#   BODY, FUNCTION, PROCEDURE, PURE, IMPURE, AND, OR, XOR, NAND, NOR,
#   XNOR, NOT, ABS, MOD, REM, SLL, SRL, SLA, SRA, ROL, ROR,
#   ARRAY, RECORD, DOWNTO, TO, OPEN, ALL, NEW
#
# Literal/regex tokens:
#   NAME         — identifiers (case-normalized to lowercase)
#   NUMBER       — integer literals: 42, 1_000
#   REAL_NUMBER  — real literals: 3.14, 1.0E-3
#   BASED_LITERAL — based literals: 16#FF#, 2#1010#
#   STRING       — double-quoted strings
#   CHAR_LITERAL — single-char: '0', '1', 'X', 'Z'
#   BIT_STRING   — B"1010", X"FF", O"77"
#   EXTENDED_IDENT — \extended name\
#
# Two-char operators:
#   VAR_ASSIGN (:=), LESS_EQUALS (<=), GREATER_EQUALS (>=)
#   ARROW (=>), NOT_EQUALS (/=), POWER (**), BOX (<>)
#
# Single-char operators:
#   PLUS, MINUS, STAR, SLASH, AMPERSAND
#   LESS_THAN, GREATER_THAN, EQUALS, TICK, PIPE
#
# Delimiters:
#   LPAREN, RPAREN, LBRACKET, RBRACKET, SEMICOLON, COMMA, DOT, COLON
#
# # AST node types (rule_name values)
# ====================================
#
#   design_file              — root; list of design units
#   design_unit              — context_items + library_unit
#   context_item             — library_clause or use_clause
#   library_clause           — library NAME, …;
#   use_clause               — use selected_name;
#   selected_name            — NAME { . NAME }
#   name_list                — NAME {, NAME}
#   entity_declaration       — entity NAME is [generic] [port] end [entity] [NAME];
#   generic_clause           — generic(interface_list);
#   port_clause              — port(interface_list);
#   interface_list           — interface_element {; interface_element}
#   interface_element        — name_list : [mode] type [:= expr]
#   mode                     — in | out | inout | buffer
#   architecture_body        — architecture NAME of NAME is { decl } begin { stmt } end;
#   block_declarative_item   — signal/constant/type/subtype/component/function/procedure
#   signal_declaration       — signal name_list : type [:= expr];
#   constant_declaration     — constant name_list : type := expr;
#   variable_declaration     — variable name_list : type [:= expr];
#   type_declaration         — type NAME is type_def;
#   subtype_declaration      — subtype NAME is subtype_indication;
#   type_definition          — enumeration_type | array_type | record_type
#   enumeration_type         — (NAME/CHAR {, NAME/CHAR})
#   array_type               — array(constraint) of type
#   record_type              — record … end record;
#   subtype_indication       — selected_name [constraint]
#   constraint               — (expr to/downto expr) | range expr to/downto expr
#   component_declaration    — component NAME [is] [generic] [port] end component;
#   concurrent_statement     — process | signal_assign | component_inst | generate
#   process_statement        — [LABEL:] process [(sensitivity)] [is] { decl } begin { seq } end process;
#   sensitivity_list         — NAME {, NAME}
#   signal_assignment_concurrent — NAME <= waveform;
#   signal_assignment_seq    — NAME <= waveform;
#   variable_assignment      — NAME := expr;
#   waveform                 — waveform_element {, waveform_element}
#   waveform_element         — expression
#   component_instantiation  — NAME : NAME [generic map] [port map];
#   generate_statement       — NAME : (for_generate | if_generate)
#   for_generate             — for NAME in discrete_range generate { stmt } end generate;
#   if_generate              — if expr generate { stmt } end generate;
#   sequential_statement     — signal_assign | var_assign | if | case | loop | return | null
#   if_statement             — if expr then { seq } { elsif … } [else { seq }] end if;
#   case_statement           — case expr is { when choices => { seq } } end case;
#   choices                  — choice { | choice }
#   loop_statement           — [LABEL:] [for|while] loop { seq } end loop;
#   return_statement         — return [expr];
#   null_statement           — null;
#   package_declaration      — package NAME is { item } end [package] [NAME];
#   package_body             — package body NAME is { item } end [package body] [NAME];
#   function_declaration     — [pure|impure] function NAME [params] return type;
#   function_body            — [pure|impure] function NAME [params] return type is { decl } begin { seq } end;
#   procedure_declaration    — procedure NAME [params];
#   procedure_body           — procedure NAME [params] is { decl } begin { seq } end;
#   expression               — full expression tree entry point
#   logical_expr, relation, shift_expr, adding_expr,
#   multiplying_expr, unary_expr, power_expr  — expression levels
#   primary                  — atom: number, name, literal, (expr), aggregate
#   aggregate                — (element_association {, element_association})
#   element_association      — [choices =>] expr
#   token                    — leaf node
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';
our $DEFAULT_VERSION = '2008';
our @SUPPORTED_VERSIONS = qw(1987 1993 2002 2008 2019);

use CodingAdventures::VhdlLexer;
use CodingAdventures::VhdlParser::ASTNode;

# The lexer is now version-aware and selects edition-specific token grammars.
# The parser core remains handwritten for historical reasons: this package
# landed before the generic grammar-driven Perl parser stack, and it has not
# been migrated over yet.

sub _resolve_version {
    my ($version) = @_;
    return $DEFAULT_VERSION unless defined $version && length $version;
    return $version if grep { $_ eq $version } @SUPPORTED_VERSIONS;
    die sprintf(
        "CodingAdventures::VhdlParser: unknown VHDL version '%s' (expected one of: %s)",
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

sub new {
    my ($class, $source, $version) = @_;
    $version = _resolve_version($version);
    my $tokens = CodingAdventures::VhdlLexer->tokenize($source, $version);
    return bless {
        _tokens => $tokens,
        _pos    => 0,
        _version => $version,
    }, $class;
}

# ============================================================================
# Token helpers
# ============================================================================

sub _peek {
    my ($self) = @_;
    return $self->{_tokens}[ $self->{_pos} ]
        // { type => 'EOF', value => '', line => 0, col => 0 };
}

sub _peek_ahead {
    my ($self, $n) = @_;
    $n //= 0;
    return $self->{_tokens}[ $self->{_pos} + $n ]
        // { type => 'EOF', value => '', line => 0, col => 0 };
}

sub _advance {
    my ($self) = @_;
    my $tok = $self->_peek();
    $self->{_pos}++ unless $tok->{type} eq 'EOF';
    return $tok;
}

sub _expect {
    my ($self, $type) = @_;
    my $tok = $self->_peek();
    unless ($tok->{type} eq $type) {
        die sprintf(
            "CodingAdventures::VhdlParser: parse error at line %d col %d: "
          . "expected %s but got %s ('%s')\n",
            $tok->{line}, $tok->{col}, $type, $tok->{type}, $tok->{value}
        );
    }
    return $self->_advance();
}

sub _check {
    my ($self, $type, $value) = @_;
    my $tok = $self->_peek();
    return 0 unless $tok->{type} eq $type;
    return 1 unless defined $value;
    return lc($tok->{value}) eq lc($value);
}

sub _match {
    my ($self, $type, $value) = @_;
    return $self->_advance() if $self->_check($type, $value);
    return undef;
}

sub _leaf { CodingAdventures::VhdlParser::ASTNode->new_leaf($_[1]) }
sub _node { CodingAdventures::VhdlParser::ASTNode->new($_[1], [splice @_, 2]) }

# ============================================================================
# Public API
# ============================================================================

sub parse {
    my ($self) = @_;
    return $self->_parse_design_file();
}

# ============================================================================
# Grammar rules
# ============================================================================

# design_file = { design_unit } ;
sub _parse_design_file {
    my ($self) = @_;
    my @children;
    while (!$self->_check('EOF')) {
        push @children, $self->_parse_design_unit();
    }
    return $self->_node('design_file', @children);
}

# design_unit = { context_item } library_unit ;
sub _parse_design_unit {
    my ($self) = @_;
    my @ch;
    # Context items: library and use clauses
    while ($self->_check('LIBRARY') || $self->_check('USE')) {
        push @ch, $self->_parse_context_item();
    }
    # Library unit: entity, architecture, package, or package body
    push @ch, $self->_parse_library_unit();
    return $self->_node('design_unit', @ch);
}

# context_item = library_clause | use_clause ;
sub _parse_context_item {
    my ($self) = @_;
    if ($self->_check('LIBRARY')) {
        return $self->_node('context_item', $self->_parse_library_clause());
    }
    return $self->_node('context_item', $self->_parse_use_clause());
}

# library_clause = "library" name_list SEMICOLON ;
#
# Makes a library visible in the design unit:
#   library IEEE;
sub _parse_library_clause {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LIBRARY'));
    push @ch, $self->_parse_name_list();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('library_clause', @ch);
}

# use_clause = "use" selected_name SEMICOLON ;
#
# Imports names from a package:
#   use IEEE.std_logic_1164.all;
sub _parse_use_clause {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('USE'));
    push @ch, $self->_parse_selected_name();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('use_clause', @ch);
}

# selected_name = NAME { DOT ( NAME | "all" ) } ;
#
# A dotted path: IEEE.std_logic_1164.all
# The 'all' keyword imports all visible names from a package.
sub _parse_selected_name {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NAME'));
    while ($self->_check('DOT')) {
        push @ch, $self->_leaf($self->_advance());
        if ($self->_check('ALL')) {
            push @ch, $self->_leaf($self->_advance());
        } else {
            push @ch, $self->_leaf($self->_expect('NAME'));
        }
    }
    return $self->_node('selected_name', @ch);
}

# name_list = NAME { COMMA NAME } ;
sub _parse_name_list {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NAME'));
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_leaf($self->_expect('NAME'));
    }
    return $self->_node('name_list', @ch);
}

# library_unit = entity_declaration | architecture_body
#              | package_declaration | package_body ;
sub _parse_library_unit {
    my ($self) = @_;
    my $type = $self->_peek()->{type};

    if ($type eq 'ENTITY') {
        return $self->_parse_entity_declaration();
    }
    if ($type eq 'ARCHITECTURE') {
        return $self->_parse_architecture_body();
    }
    if ($type eq 'PACKAGE') {
        # Peek ahead: "package body NAME is …" vs "package NAME is …"
        my $next = $self->_peek_ahead(1);
        if ($next->{type} eq 'BODY') {
            return $self->_parse_package_body();
        }
        return $self->_parse_package_declaration();
    }
    # Unknown — skip to avoid infinite loop
    return $self->_node('library_unit', $self->_leaf($self->_advance()));
}

# ============================================================================
# Entity Declaration
# ============================================================================

# entity_declaration = "entity" NAME "is"
#                      [ generic_clause ]
#                      [ port_clause ]
#                      "end" [ "entity" ] [ NAME ] SEMICOLON ;
#
# An entity defines the INTERFACE: what pins the component has.
# The implementation (behavior) is in the architecture.
sub _parse_entity_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('ENTITY'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('IS'));

    if ($self->_check('GENERIC')) {
        push @ch, $self->_parse_generic_clause();
    }
    if ($self->_check('PORT')) {
        push @ch, $self->_parse_port_clause();
    }

    push @ch, $self->_leaf($self->_expect('END'));
    if ($self->_check('ENTITY')) {
        push @ch, $self->_leaf($self->_advance());
    }
    if ($self->_check('NAME')) {
        push @ch, $self->_leaf($self->_advance());
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('entity_declaration', @ch);
}

# generic_clause = "generic" LPAREN interface_list RPAREN SEMICOLON ;
sub _parse_generic_clause {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('GENERIC'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    push @ch, $self->_parse_interface_list();
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('generic_clause', @ch);
}

# port_clause = "port" LPAREN interface_list RPAREN SEMICOLON ;
sub _parse_port_clause {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('PORT'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    push @ch, $self->_parse_interface_list();
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('port_clause', @ch);
}

# interface_list = interface_element { SEMICOLON interface_element } ;
#
# The interface list appears in both generics and ports.
# Each element looks like: a, b : in std_logic := '0'
sub _parse_interface_list {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_interface_element();
    # More elements separated by semicolons (note: semicolons here are
    # separators, not terminators — the final element has no trailing semicolon)
    while ($self->_check('SEMICOLON') && $self->_peek_ahead(1)->{type} ne 'RPAREN') {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_interface_element();
    }
    return $self->_node('interface_list', @ch);
}

# interface_element = name_list COLON [ mode ] subtype_indication [ VAR_ASSIGN expression ] ;
sub _parse_interface_element {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_name_list();
    push @ch, $self->_leaf($self->_expect('COLON'));

    # Optional mode: in, out, inout, buffer
    if ($self->_check('IN') || $self->_check('OUT')
        || $self->_check('INOUT') || $self->_check('BUFFER')) {
        push @ch, $self->_node('mode', $self->_leaf($self->_advance()));
    }

    push @ch, $self->_parse_subtype_indication();

    # Optional default value: := expr
    if ($self->_check('VAR_ASSIGN')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expression();
    }

    return $self->_node('interface_element', @ch);
}

# subtype_indication = selected_name [ constraint ] ;
#
# A type name with optional constraint:
#   std_logic                     — no constraint
#   std_logic_vector(7 downto 0)  — constrained array
#   integer range 0 to 255        — constrained integer
sub _parse_subtype_indication {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_selected_name();

    # Optional constraint: (expr to/downto expr) or range expr to/downto expr
    if ($self->_check('LPAREN')) {
        push @ch, $self->_parse_constraint();
    } elsif ($self->_check('RANGE')) {
        push @ch, $self->_parse_range_constraint();
    }

    return $self->_node('subtype_indication', @ch);
}

# constraint = LPAREN expression ( "to" | "downto" ) expression RPAREN ;
sub _parse_constraint {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    push @ch, $self->_parse_expression();
    if ($self->_check('DOWNTO') || $self->_check('TO')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expression();
    }
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    return $self->_node('constraint', @ch);
}

# range_constraint = "range" expression ( "to" | "downto" ) expression ;
sub _parse_range_constraint {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('RANGE'));
    push @ch, $self->_parse_expression();
    if ($self->_check('DOWNTO') || $self->_check('TO')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expression();
    }
    return $self->_node('constraint', @ch);
}

# ============================================================================
# Architecture Body
# ============================================================================

# architecture_body = "architecture" NAME "of" NAME "is"
#                     { block_declarative_item }
#                     "begin"
#                     { concurrent_statement }
#                     "end" [ "architecture" ] [ NAME ] SEMICOLON ;
#
# The architecture is where the actual hardware description lives.
# Everything before BEGIN is the declarative region (signals, types, etc.)
# Everything after BEGIN is concurrent — all statements execute simultaneously,
# like real hardware components operating at once.
sub _parse_architecture_body {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('ARCHITECTURE'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('OF'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('IS'));

    # Declarative region
    while (!$self->_check('BEGIN') && !$self->_check('EOF')) {
        my $item = $self->_try_parse_block_declarative_item();
        push @ch, $item if defined $item;
    }

    push @ch, $self->_leaf($self->_expect('BEGIN'));

    # Concurrent statement region
    while (!$self->_check('END') && !$self->_check('EOF')) {
        my $stmt = $self->_try_parse_concurrent_statement();
        push @ch, $stmt if defined $stmt;
    }

    push @ch, $self->_leaf($self->_expect('END'));
    if ($self->_check('ARCHITECTURE')) {
        push @ch, $self->_leaf($self->_advance());
    }
    if ($self->_check('NAME')) {
        push @ch, $self->_leaf($self->_advance());
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('architecture_body', @ch);
}

# _try_parse_block_declarative_item
#
# Parse one declarative item in an architecture or process body.
sub _try_parse_block_declarative_item {
    my ($self) = @_;
    my $type = $self->_peek()->{type};

    if ($type eq 'SIGNAL') {
        return $self->_parse_signal_declaration();
    }
    if ($type eq 'CONSTANT') {
        return $self->_parse_constant_declaration();
    }
    if ($type eq 'VARIABLE') {
        return $self->_parse_variable_declaration();
    }
    if ($type eq 'TYPE') {
        return $self->_parse_type_declaration();
    }
    if ($type eq 'SUBTYPE') {
        return $self->_parse_subtype_declaration();
    }
    if ($type eq 'COMPONENT') {
        return $self->_parse_component_declaration();
    }
    if ($type eq 'FUNCTION' || $type eq 'PURE' || $type eq 'IMPURE') {
        return $self->_try_parse_function_or_decl();
    }
    if ($type eq 'PROCEDURE') {
        return $self->_try_parse_procedure_or_decl();
    }

    # Unknown — skip
    return $self->_node('block_declarative_item', $self->_leaf($self->_advance()));
}

# signal_declaration = "signal" name_list COLON subtype_indication
#                      [ VAR_ASSIGN expression ] SEMICOLON ;
#
# Signals are VHDL's wires and registers. Unlike variables, signal
# assignments take effect after a delta delay, modelling real hardware.
sub _parse_signal_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('SIGNAL'));
    push @ch, $self->_parse_name_list();
    push @ch, $self->_leaf($self->_expect('COLON'));
    push @ch, $self->_parse_subtype_indication();
    if ($self->_check('VAR_ASSIGN')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expression();
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('signal_declaration', @ch);
}

# constant_declaration = "constant" name_list COLON subtype_indication
#                        VAR_ASSIGN expression SEMICOLON ;
sub _parse_constant_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('CONSTANT'));
    push @ch, $self->_parse_name_list();
    push @ch, $self->_leaf($self->_expect('COLON'));
    push @ch, $self->_parse_subtype_indication();
    push @ch, $self->_leaf($self->_expect('VAR_ASSIGN'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('constant_declaration', @ch);
}

# variable_declaration = "variable" name_list COLON subtype_indication
#                        [ VAR_ASSIGN expression ] SEMICOLON ;
#
# Variables live inside processes. Variable assignments (:=) are immediate —
# unlike signals, the new value is visible on the very next line.
sub _parse_variable_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('VARIABLE'));
    push @ch, $self->_parse_name_list();
    push @ch, $self->_leaf($self->_expect('COLON'));
    push @ch, $self->_parse_subtype_indication();
    if ($self->_check('VAR_ASSIGN')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expression();
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('variable_declaration', @ch);
}

# type_declaration = "type" NAME "is" type_definition SEMICOLON ;
sub _parse_type_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('TYPE'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('IS'));
    push @ch, $self->_parse_type_definition();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('type_declaration', @ch);
}

# subtype_declaration = "subtype" NAME "is" subtype_indication SEMICOLON ;
sub _parse_subtype_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('SUBTYPE'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('IS'));
    push @ch, $self->_parse_subtype_indication();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('subtype_declaration', @ch);
}

# type_definition = enumeration_type | array_type | record_type ;
sub _parse_type_definition {
    my ($self) = @_;
    my $type = $self->_peek()->{type};

    if ($type eq 'LPAREN') {
        return $self->_parse_enumeration_type();
    }
    if ($type eq 'ARRAY') {
        return $self->_parse_array_type();
    }
    if ($type eq 'RECORD') {
        return $self->_parse_record_type();
    }
    # Fallthrough
    return $self->_node('type_definition', $self->_leaf($self->_advance()));
}

# enumeration_type = LPAREN ( NAME | CHAR_LITERAL ) { COMMA … } RPAREN ;
#
# Example: type state_t is (IDLE, RUNNING, DONE, ERROR);
# VHDL's std_logic is defined as: ('U','X','0','1','Z','W','L','H','-')
sub _parse_enumeration_type {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    if ($self->_check('NAME') || $self->_check('CHAR_LITERAL')) {
        push @ch, $self->_leaf($self->_advance());
        while ($self->_check('COMMA')) {
            push @ch, $self->_leaf($self->_advance());
            push @ch, $self->_leaf($self->_advance());   # NAME or CHAR_LITERAL
        }
    }
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    return $self->_node('enumeration_type', @ch);
}

# array_type = "array" LPAREN index_constraint RPAREN "of" subtype_indication ;
sub _parse_array_type {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('ARRAY'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    # Parse discrete_range(s) as index constraint
    push @ch, $self->_parse_discrete_range();
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_discrete_range();
    }
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    push @ch, $self->_leaf($self->_expect('OF'));
    push @ch, $self->_parse_subtype_indication();
    return $self->_node('array_type', @ch);
}

# discrete_range = subtype_indication | expression ( "to" | "downto" ) expression ;
#
# The <> (BOX) token indicates an unconstrained range: natural range <>
sub _parse_discrete_range {
    my ($self) = @_;
    my @ch;
    # Unconstrained range: natural range <>
    if ($self->_check('NAME') && $self->_peek_ahead(1)->{type} eq 'RANGE'
        && $self->_peek_ahead(2)->{type} eq 'BOX') {
        push @ch, $self->_leaf($self->_advance());   # type name
        push @ch, $self->_leaf($self->_advance());   # range
        push @ch, $self->_leaf($self->_advance());   # <>
        return $self->_node('discrete_range', @ch);
    }
    # Constrained range: expr to/downto expr
    push @ch, $self->_parse_expression();
    if ($self->_check('TO') || $self->_check('DOWNTO')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expression();
    }
    return $self->_node('discrete_range', @ch);
}

# record_type = "record" { NAME COLON subtype_indication SEMICOLON } "end" "record" [ NAME ] ;
sub _parse_record_type {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('RECORD'));
    while (!$self->_check('END') && !$self->_check('EOF')) {
        push @ch, $self->_leaf($self->_expect('NAME'));
        push @ch, $self->_leaf($self->_expect('COLON'));
        push @ch, $self->_parse_subtype_indication();
        push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    }
    push @ch, $self->_leaf($self->_expect('END'));
    push @ch, $self->_leaf($self->_expect('RECORD'));
    if ($self->_check('NAME')) {
        push @ch, $self->_leaf($self->_advance());
    }
    return $self->_node('record_type', @ch);
}

# component_declaration = "component" NAME [ "is" ]
#                         [ generic_clause ] [ port_clause ]
#                         "end" "component" [ NAME ] SEMICOLON ;
sub _parse_component_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('COMPONENT'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    if ($self->_check('IS')) {
        push @ch, $self->_leaf($self->_advance());
    }
    if ($self->_check('GENERIC')) {
        push @ch, $self->_parse_generic_clause();
    }
    if ($self->_check('PORT')) {
        push @ch, $self->_parse_port_clause();
    }
    push @ch, $self->_leaf($self->_expect('END'));
    push @ch, $self->_leaf($self->_expect('COMPONENT'));
    if ($self->_check('NAME')) {
        push @ch, $self->_leaf($self->_advance());
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('component_declaration', @ch);
}

# ============================================================================
# Concurrent Statements
# ============================================================================

# _try_parse_concurrent_statement
#
# Parse one concurrent statement. The tricky part is that NAME COLON could be
# a labeled process, a labeled generate, or a component instantiation.
sub _try_parse_concurrent_statement {
    my ($self) = @_;
    my $type = $self->_peek()->{type};

    # Process statement: "process" or "LABEL : process"
    if ($type eq 'PROCESS') {
        return $self->_node('concurrent_statement', $self->_parse_process_statement(undef));
    }

    # Generate or component instantiation (may have label)
    if ($type eq 'NAME') {
        my $next = $self->_peek_ahead(1);
        if ($next->{type} eq 'COLON') {
            # LABEL : ...
            my $label     = $self->_leaf($self->_advance());   # NAME (label)
            my $colon     = $self->_leaf($self->_advance());   # COLON
            my $after     = $self->_peek()->{type};
            if ($after eq 'PROCESS') {
                return $self->_node('concurrent_statement',
                    $self->_parse_process_statement([$label, $colon]));
            }
            if ($after eq 'FOR' || $after eq 'IF') {
                return $self->_node('concurrent_statement',
                    $self->_parse_generate_statement($label, $colon));
            }
            # Component instantiation: LABEL : (NAME | entity …)
            return $self->_node('concurrent_statement',
                $self->_parse_component_instantiation($label, $colon));
        }
        # NAME <= … — concurrent signal assignment
        my $n2 = $self->_peek_ahead(1);
        if ($n2->{type} eq 'LESS_EQUALS') {
            return $self->_node('concurrent_statement',
                $self->_parse_signal_assignment_concurrent());
        }
        # Otherwise skip
        return $self->_node('concurrent_statement', $self->_leaf($self->_advance()));
    }

    # Unknown — skip
    return $self->_node('concurrent_statement', $self->_leaf($self->_advance()));
}

# process_statement = [label_ch] "process" [ LPAREN sensitivity_list RPAREN ]
#                     [ "is" ]
#                     { process_declarative_item }
#                     "begin"
#                     { sequential_statement }
#                     "end" "process" [ NAME ] SEMICOLON ;
sub _parse_process_statement {
    my ($self, $label_ch) = @_;
    my @ch = defined $label_ch ? @$label_ch : ();

    push @ch, $self->_leaf($self->_expect('PROCESS'));

    if ($self->_check('LPAREN')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_sensitivity_list();
        push @ch, $self->_leaf($self->_expect('RPAREN'));
    }

    if ($self->_check('IS')) {
        push @ch, $self->_leaf($self->_advance());
    }

    # Declarative items in the process
    while (!$self->_check('BEGIN') && !$self->_check('EOF')) {
        my $item = $self->_try_parse_block_declarative_item();
        push @ch, $item if defined $item;
    }

    push @ch, $self->_leaf($self->_expect('BEGIN'));

    while (!$self->_check('END') && !$self->_check('EOF')) {
        push @ch, $self->_parse_sequential_statement();
    }

    push @ch, $self->_leaf($self->_expect('END'));
    push @ch, $self->_leaf($self->_expect('PROCESS'));
    if ($self->_check('NAME')) {
        push @ch, $self->_leaf($self->_advance());
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('process_statement', @ch);
}

# sensitivity_list = NAME { COMMA NAME } ;
#
# The sensitivity list determines which signals trigger the process.
# Each signal change causes the process to re-execute.
sub _parse_sensitivity_list {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NAME'));
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_leaf($self->_expect('NAME'));
    }
    return $self->_node('sensitivity_list', @ch);
}

# signal_assignment_concurrent = NAME LESS_EQUALS waveform SEMICOLON ;
#
# Concurrent signal assignment models combinational logic:
#   y <= a and b;   — always active, like Verilog's assign
sub _parse_signal_assignment_concurrent {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('LESS_EQUALS'));
    push @ch, $self->_parse_waveform();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('signal_assignment_concurrent', @ch);
}

# waveform = waveform_element { COMMA waveform_element } ;
# waveform_element = expression ;
sub _parse_waveform {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_node('waveform_element', $self->_parse_expression());
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_node('waveform_element', $self->_parse_expression());
    }
    return $self->_node('waveform', @ch);
}

# component_instantiation = NAME COLON ( NAME | "entity" selected_name )
#                           [ "generic" "map" LPAREN assoc_list RPAREN ]
#                           [ "port" "map" LPAREN assoc_list RPAREN ]
#                           SEMICOLON ;
sub _parse_component_instantiation {
    my ($self, $label, $colon) = @_;
    my @ch = ($label, $colon);

    if ($self->_check('ENTITY')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_selected_name();
        if ($self->_check('LPAREN')) {
            push @ch, $self->_leaf($self->_advance());
            push @ch, $self->_leaf($self->_expect('NAME'));
            push @ch, $self->_leaf($self->_expect('RPAREN'));
        }
    } else {
        push @ch, $self->_leaf($self->_expect('NAME'));
    }

    if ($self->_check('GENERIC')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_leaf($self->_expect('MAP'));
        push @ch, $self->_leaf($self->_expect('LPAREN'));
        push @ch, $self->_parse_association_list();
        push @ch, $self->_leaf($self->_expect('RPAREN'));
    }

    if ($self->_check('PORT')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_leaf($self->_expect('MAP'));
        push @ch, $self->_leaf($self->_expect('LPAREN'));
        push @ch, $self->_parse_association_list();
        push @ch, $self->_leaf($self->_expect('RPAREN'));
    }

    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('component_instantiation', @ch);
}

# association_list = association_element { COMMA association_element } ;
# association_element = [ NAME ARROW ] expression | "open" ;
sub _parse_association_list {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_association_element();
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_association_element();
    }
    return $self->_node('association_list', @ch);
}

sub _parse_association_element {
    my ($self) = @_;
    my @ch;
    if ($self->_check('OPEN')) {
        push @ch, $self->_leaf($self->_advance());
        return $self->_node('association_element', @ch);
    }
    # Lookahead: NAME ARROW → named association
    if ($self->_check('NAME') && $self->_peek_ahead(1)->{type} eq 'ARROW') {
        push @ch, $self->_leaf($self->_advance());   # NAME
        push @ch, $self->_leaf($self->_advance());   # =>
    }
    push @ch, $self->_parse_expression();
    return $self->_node('association_element', @ch);
}

# generate_statement = NAME COLON ( for_generate | if_generate ) ;
sub _parse_generate_statement {
    my ($self, $label, $colon) = @_;
    my @ch = ($label, $colon);
    if ($self->_check('FOR')) {
        push @ch, $self->_parse_for_generate();
    } else {
        push @ch, $self->_parse_if_generate();
    }
    return $self->_node('generate_statement', @ch);
}

# for_generate = "for" NAME "in" discrete_range "generate"
#                { concurrent_statement }
#                "end" "generate" [ NAME ] SEMICOLON ;
sub _parse_for_generate {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('FOR'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('IN'));
    push @ch, $self->_parse_discrete_range();
    push @ch, $self->_leaf($self->_expect('GENERATE'));
    while (!$self->_check('END') && !$self->_check('EOF')) {
        my $s = $self->_try_parse_concurrent_statement();
        push @ch, $s if defined $s;
    }
    push @ch, $self->_leaf($self->_expect('END'));
    push @ch, $self->_leaf($self->_expect('GENERATE'));
    if ($self->_check('NAME')) {
        push @ch, $self->_leaf($self->_advance());
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('for_generate', @ch);
}

# if_generate = "if" expression "generate"
#               { concurrent_statement }
#               "end" "generate" [ NAME ] SEMICOLON ;
sub _parse_if_generate {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('IF'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('GENERATE'));
    while (!$self->_check('END') && !$self->_check('EOF')) {
        my $s = $self->_try_parse_concurrent_statement();
        push @ch, $s if defined $s;
    }
    push @ch, $self->_leaf($self->_expect('END'));
    push @ch, $self->_leaf($self->_expect('GENERATE'));
    if ($self->_check('NAME')) {
        push @ch, $self->_leaf($self->_advance());
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('if_generate', @ch);
}

# ============================================================================
# Sequential Statements
# ============================================================================

# sequential_statement = signal_assignment_seq | variable_assignment
#                      | if_statement | case_statement | loop_statement
#                      | return_statement | null_statement ;
sub _parse_sequential_statement {
    my ($self) = @_;
    my $type = $self->_peek()->{type};

    if ($type eq 'IF') {
        return $self->_parse_if_statement();
    }
    if ($type eq 'CASE') {
        return $self->_parse_case_statement();
    }
    if ($type eq 'WHILE' || $type eq 'FOR' || $type eq 'LOOP') {
        return $self->_parse_loop_statement(undef);
    }
    if ($type eq 'RETURN') {
        return $self->_parse_return_statement();
    }
    if ($type eq 'NULL') {
        return $self->_parse_null_statement();
    }
    if ($type eq 'WAIT') {
        return $self->_parse_wait_statement();
    }

    # NAME LESS_EQUALS → signal assignment
    # NAME VAR_ASSIGN  → variable assignment
    # NAME COLON WHILE/FOR/LOOP → labeled loop
    if ($type eq 'NAME') {
        my $next = $self->_peek_ahead(1);
        if ($next->{type} eq 'LESS_EQUALS') {
            return $self->_parse_signal_assignment_seq();
        }
        if ($next->{type} eq 'VAR_ASSIGN') {
            return $self->_parse_variable_assignment();
        }
        if ($next->{type} eq 'COLON') {
            # Labeled loop or other labeled statement
            my $label = $self->_leaf($self->_advance());
            my $colon = $self->_leaf($self->_advance());
            if ($self->_check('WHILE') || $self->_check('FOR') || $self->_check('LOOP')) {
                return $self->_parse_loop_statement([$label, $colon]);
            }
            # Labeled null or other
            return $self->_node('sequential_statement', $label, $colon,
                                $self->_leaf($self->_advance()));
        }
    }

    # Unknown — skip
    return $self->_node('sequential_statement', $self->_leaf($self->_advance()));
}

# signal_assignment_seq = NAME LESS_EQUALS waveform SEMICOLON ;
#
# Note: <= here is signal assignment (not less-than-or-equal).
# Inside a process, multiple assignments to the same signal are legal;
# only the LAST one per process activation takes effect.
sub _parse_signal_assignment_seq {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('LESS_EQUALS'));
    push @ch, $self->_parse_waveform();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('signal_assignment_seq', @ch);
}

# variable_assignment = NAME VAR_ASSIGN expression SEMICOLON ;
#
# Variable assignments (:=) are immediately visible. They model software-like
# local calculations within a process (no delta delay).
sub _parse_variable_assignment {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('VAR_ASSIGN'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('variable_assignment', @ch);
}

# if_statement = "if" expression "then"
#                { sequential_statement }
#                { "elsif" expression "then" { sequential_statement } }
#                [ "else" { sequential_statement } ]
#                "end" "if" SEMICOLON ;
#
# VHDL uses ELSIF (not else if) — this is a single keyword, not two.
# VHDL requires "end if;" to close the statement.
sub _parse_if_statement {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('IF'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('THEN'));
    while (!$self->_check('ELSIF') && !$self->_check('ELSE')
           && !$self->_check('END') && !$self->_check('EOF')) {
        push @ch, $self->_parse_sequential_statement();
    }
    while ($self->_check('ELSIF')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expression();
        push @ch, $self->_leaf($self->_expect('THEN'));
        while (!$self->_check('ELSIF') && !$self->_check('ELSE')
               && !$self->_check('END') && !$self->_check('EOF')) {
            push @ch, $self->_parse_sequential_statement();
        }
    }
    if ($self->_check('ELSE')) {
        push @ch, $self->_leaf($self->_advance());
        while (!$self->_check('END') && !$self->_check('EOF')) {
            push @ch, $self->_parse_sequential_statement();
        }
    }
    push @ch, $self->_leaf($self->_expect('END'));
    push @ch, $self->_leaf($self->_expect('IF'));
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('if_statement', @ch);
}

# case_statement = "case" expression "is"
#                  { "when" choices ARROW { sequential_statement } }
#                  "end" "case" SEMICOLON ;
#
# VHDL uses => (arrow) after when, not colon like Verilog.
# VHDL requires "when others" to be exhaustive.
sub _parse_case_statement {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('CASE'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('IS'));
    while ($self->_check('WHEN') && !$self->_check('EOF')) {
        push @ch, $self->_leaf($self->_advance());   # when
        push @ch, $self->_parse_choices();
        push @ch, $self->_leaf($self->_expect('ARROW'));
        while (!$self->_check('WHEN') && !$self->_check('END')
               && !$self->_check('EOF')) {
            push @ch, $self->_parse_sequential_statement();
        }
    }
    push @ch, $self->_leaf($self->_expect('END'));
    push @ch, $self->_leaf($self->_expect('CASE'));
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('case_statement', @ch);
}

# choices = choice { PIPE choice } ;
# choice  = expression | discrete_range | "others" ;
sub _parse_choices {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_one_choice();
    while ($self->_check('PIPE')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_one_choice();
    }
    return $self->_node('choices', @ch);
}

sub _parse_one_choice {
    my ($self) = @_;
    if ($self->_check('OTHERS')) {
        return $self->_node('choice', $self->_leaf($self->_advance()));
    }
    return $self->_node('choice', $self->_parse_expression());
}

# loop_statement = [ label_ch ] [ "for" NAME "in" discrete_range | "while" expression ]
#                  "loop"
#                  { sequential_statement }
#                  "end" "loop" [ NAME ] SEMICOLON ;
sub _parse_loop_statement {
    my ($self, $label_ch) = @_;
    my @ch = defined $label_ch ? @$label_ch : ();

    if ($self->_check('FOR')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_leaf($self->_expect('NAME'));
        push @ch, $self->_leaf($self->_expect('IN'));
        push @ch, $self->_parse_discrete_range();
    } elsif ($self->_check('WHILE')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expression();
    }

    push @ch, $self->_leaf($self->_expect('LOOP'));
    while (!$self->_check('END') && !$self->_check('EOF')) {
        push @ch, $self->_parse_sequential_statement();
    }
    push @ch, $self->_leaf($self->_expect('END'));
    push @ch, $self->_leaf($self->_expect('LOOP'));
    if ($self->_check('NAME')) {
        push @ch, $self->_leaf($self->_advance());
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('loop_statement', @ch);
}

# return_statement = "return" [ expression ] SEMICOLON ;
sub _parse_return_statement {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('RETURN'));
    if (!$self->_check('SEMICOLON') && !$self->_check('EOF')) {
        push @ch, $self->_parse_expression();
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('return_statement', @ch);
}

# null_statement = "null" SEMICOLON ;
#
# The null statement is a no-op. It's used in case/when branches that
# should do nothing:
#   when IDLE => null;
sub _parse_null_statement {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NULL'));
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('null_statement', @ch);
}

# wait_statement = "wait"
#                  [ "on" sensitivity_list ]
#                  [ "until" expression ]
#                  [ "for" time_expression ]
#                  SEMICOLON ;
#
# time_expression = expression [ NAME ]   -- e.g., 10 ns, 1.5 us
#
# The WAIT statement suspends a process until a condition is met.
# Common forms:
#   wait;                  -- wait forever (or until another signal change)
#   wait for 10 ns;        -- timeout clause (absolute time delay)
#   wait until clk = '1';  -- condition clause
#   wait on a, b;          -- sensitivity clause
sub _parse_wait_statement {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('WAIT'));

    # Optional sensitivity clause: ON sensitivity_list
    if ($self->_check('ON')) {
        push @ch, $self->_leaf($self->_advance());
        # Consume NAME tokens separated by commas
        push @ch, $self->_leaf($self->_expect('NAME'));
        while ($self->_check('COMMA')) {
            push @ch, $self->_leaf($self->_advance());
            push @ch, $self->_leaf($self->_expect('NAME'));
        }
    }

    # Optional condition clause: UNTIL expression
    if ($self->_check('UNTIL')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expression();
    }

    # Optional timeout clause: FOR time_expression
    if ($self->_check('FOR')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expression();
        # Optional time unit (NAME like ns, us, ms, sec)
        if ($self->_check('NAME')) {
            push @ch, $self->_leaf($self->_advance());
        }
    }

    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('wait_statement', @ch);
}

# ============================================================================
# Packages
# ============================================================================

# package_declaration = "package" NAME "is"
#                       { package_declarative_item }
#                       "end" [ "package" ] [ NAME ] SEMICOLON ;
sub _parse_package_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('PACKAGE'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('IS'));
    while (!$self->_check('END') && !$self->_check('EOF')) {
        my $item = $self->_try_parse_block_declarative_item();
        push @ch, $item if defined $item;
    }
    push @ch, $self->_leaf($self->_expect('END'));
    if ($self->_check('PACKAGE')) {
        push @ch, $self->_leaf($self->_advance());
    }
    if ($self->_check('NAME')) {
        push @ch, $self->_leaf($self->_advance());
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('package_declaration', @ch);
}

# package_body = "package" "body" NAME "is"
#                { package_body_declarative_item }
#                "end" [ "package" "body" ] [ NAME ] SEMICOLON ;
sub _parse_package_body {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('PACKAGE'));
    push @ch, $self->_leaf($self->_expect('BODY'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('IS'));
    while (!$self->_check('END') && !$self->_check('EOF')) {
        my $item = $self->_try_parse_block_declarative_item();
        push @ch, $item if defined $item;
    }
    push @ch, $self->_leaf($self->_expect('END'));
    if ($self->_check('PACKAGE')) {
        push @ch, $self->_leaf($self->_advance());
        if ($self->_check('BODY')) {
            push @ch, $self->_leaf($self->_advance());
        }
    }
    if ($self->_check('NAME')) {
        push @ch, $self->_leaf($self->_advance());
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('package_body', @ch);
}

# ============================================================================
# Functions and Procedures
# ============================================================================

sub _try_parse_function_or_decl {
    my ($self) = @_;
    my @ch;
    # Optional pure/impure qualifier
    if ($self->_check('PURE') || $self->_check('IMPURE')) {
        push @ch, $self->_leaf($self->_advance());
    }
    push @ch, $self->_leaf($self->_expect('FUNCTION'));
    push @ch, $self->_leaf($self->_expect('NAME'));

    if ($self->_check('LPAREN')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_interface_list();
        push @ch, $self->_leaf($self->_expect('RPAREN'));
    }

    push @ch, $self->_leaf($self->_expect('RETURN'));
    push @ch, $self->_parse_subtype_indication();

    if ($self->_check('SEMICOLON')) {
        # Function declaration (forward declaration)
        push @ch, $self->_leaf($self->_advance());
        return $self->_node('function_declaration', @ch);
    }

    # Function body
    push @ch, $self->_leaf($self->_expect('IS'));
    while (!$self->_check('BEGIN') && !$self->_check('EOF')) {
        my $item = $self->_try_parse_block_declarative_item();
        push @ch, $item if defined $item;
    }
    push @ch, $self->_leaf($self->_expect('BEGIN'));
    while (!$self->_check('END') && !$self->_check('EOF')) {
        push @ch, $self->_parse_sequential_statement();
    }
    push @ch, $self->_leaf($self->_expect('END'));
    if ($self->_check('FUNCTION')) {
        push @ch, $self->_leaf($self->_advance());
    }
    if ($self->_check('NAME')) {
        push @ch, $self->_leaf($self->_advance());
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('function_body', @ch);
}

sub _try_parse_procedure_or_decl {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('PROCEDURE'));
    push @ch, $self->_leaf($self->_expect('NAME'));

    if ($self->_check('LPAREN')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_interface_list();
        push @ch, $self->_leaf($self->_expect('RPAREN'));
    }

    if ($self->_check('SEMICOLON')) {
        push @ch, $self->_leaf($self->_advance());
        return $self->_node('procedure_declaration', @ch);
    }

    push @ch, $self->_leaf($self->_expect('IS'));
    while (!$self->_check('BEGIN') && !$self->_check('EOF')) {
        my $item = $self->_try_parse_block_declarative_item();
        push @ch, $item if defined $item;
    }
    push @ch, $self->_leaf($self->_expect('BEGIN'));
    while (!$self->_check('END') && !$self->_check('EOF')) {
        push @ch, $self->_parse_sequential_statement();
    }
    push @ch, $self->_leaf($self->_expect('END'));
    if ($self->_check('PROCEDURE')) {
        push @ch, $self->_leaf($self->_advance());
    }
    if ($self->_check('NAME')) {
        push @ch, $self->_leaf($self->_advance());
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('procedure_body', @ch);
}

# ============================================================================
# Expressions — VHDL operator precedence
# ============================================================================
#
# VHDL operator precedence (lowest to highest):
#   1. and, or, xor, nand, nor, xnor   Logical
#   2. =, /=, <, <=, >, >=             Relational
#   3. sll, srl, sla, sra, rol, ror    Shift
#   4. +, -, &                         Adding (& is concatenation)
#   5. *, /, mod, rem                  Multiplying
#   6. **, abs, not                    Miscellaneous (unary)
#
# Important VHDL rule: logical operators cannot be mixed without parentheses.
#   a and b or c   — SYNTAX ERROR in strict VHDL
#   (a and b) or c — OK
# We implement this as a single optional binary operation.

# expression = logical_expr ;
sub _parse_expression {
    my ($self) = @_;
    return $self->_node('expression', $self->_parse_logical_expr());
}

# logical_expr = relation [ logical_op relation ] ;
#
# logical_op = "and" | "or" | "xor" | "nand" | "nor" | "xnor"
sub _parse_logical_expr {
    my ($self) = @_;
    my $left = $self->_parse_relation();
    my $type = $self->_peek()->{type};
    if ($type eq 'AND' || $type eq 'OR' || $type eq 'XOR'
        || $type eq 'NAND' || $type eq 'NOR' || $type eq 'XNOR') {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_relation();
        return $self->_node('logical_expr', $left, $op, $right);
    }
    return $left;
}

# relation = shift_expr [ relational_op shift_expr ] ;
# relational_op = EQUALS | NOT_EQUALS | LESS_THAN | LESS_EQUALS | GREATER_THAN | GREATER_EQUALS ;
#
# Note: LESS_EQUALS here is comparison (≤), not signal assignment.
# The grammar structure ensures this is only called in expression context.
sub _parse_relation {
    my ($self) = @_;
    my $left = $self->_parse_shift_expr();
    my $type = $self->_peek()->{type};
    if ($type eq 'EQUALS' || $type eq 'NOT_EQUALS' || $type eq 'LESS_THAN'
        || $type eq 'LESS_EQUALS' || $type eq 'GREATER_THAN' || $type eq 'GREATER_EQUALS') {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_shift_expr();
        return $self->_node('relation', $left, $op, $right);
    }
    return $left;
}

# shift_expr = adding_expr [ shift_op adding_expr ] ;
# shift_op = "sll" | "srl" | "sla" | "sra" | "rol" | "ror" ;
sub _parse_shift_expr {
    my ($self) = @_;
    my $left = $self->_parse_adding_expr();
    my $type = $self->_peek()->{type};
    if ($type eq 'SLL' || $type eq 'SRL' || $type eq 'SLA'
        || $type eq 'SRA' || $type eq 'ROL' || $type eq 'ROR') {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_adding_expr();
        return $self->_node('shift_expr', $left, $op, $right);
    }
    return $left;
}

# adding_expr = multiplying_expr { adding_op multiplying_expr } ;
# adding_op = PLUS | MINUS | AMPERSAND ;
#
# Note: AMPERSAND (&) is concatenation in VHDL (not bitwise AND like in C/Verilog).
sub _parse_adding_expr {
    my ($self) = @_;
    my $left = $self->_parse_multiplying_expr();
    while ($self->_check('PLUS') || $self->_check('MINUS') || $self->_check('AMPERSAND')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_multiplying_expr();
        $left = $self->_node('adding_expr', $left, $op, $right);
    }
    return $left;
}

# multiplying_expr = unary_expr { multiplying_op unary_expr } ;
# multiplying_op = STAR | SLASH | "mod" | "rem" ;
sub _parse_multiplying_expr {
    my ($self) = @_;
    my $left = $self->_parse_unary_expr();
    while ($self->_check('STAR') || $self->_check('SLASH')
           || $self->_check('MOD') || $self->_check('REM')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_unary_expr();
        $left = $self->_node('multiplying_expr', $left, $op, $right);
    }
    return $left;
}

# unary_expr = "abs" unary_expr | "not" unary_expr
#            | ( PLUS | MINUS ) unary_expr | power_expr ;
sub _parse_unary_expr {
    my ($self) = @_;
    my $type = $self->_peek()->{type};
    if ($type eq 'ABS' || $type eq 'NOT') {
        my $op   = $self->_leaf($self->_advance());
        my $expr = $self->_parse_unary_expr();
        return $self->_node('unary_expr', $op, $expr);
    }
    if ($type eq 'PLUS' || $type eq 'MINUS') {
        my $op   = $self->_leaf($self->_advance());
        my $expr = $self->_parse_unary_expr();
        return $self->_node('unary_expr', $op, $expr);
    }
    return $self->_parse_power_expr();
}

# power_expr = primary [ POWER primary ] ;
sub _parse_power_expr {
    my ($self) = @_;
    my $base = $self->_parse_primary();
    if ($self->_check('POWER')) {
        my $op  = $self->_leaf($self->_advance());
        my $exp = $self->_parse_primary();
        return $self->_node('power_expr', $base, $op, $exp);
    }
    return $base;
}

# primary = NUMBER | REAL_NUMBER | BASED_LITERAL | STRING | CHAR_LITERAL | BIT_STRING
#         | NAME [ TICK NAME ]
#         | NAME LPAREN [ expression { COMMA expression } ] RPAREN
#         | LPAREN expression RPAREN
#         | aggregate
#         | "null" ;
sub _parse_primary {
    my ($self) = @_;
    my $type = $self->_peek()->{type};

    if ($type eq 'NUMBER' || $type eq 'REAL_NUMBER' || $type eq 'BASED_LITERAL') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }
    if ($type eq 'STRING' || $type eq 'CHAR_LITERAL' || $type eq 'BIT_STRING') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }
    if ($type eq 'NULL') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }

    # Parenthesized expression or aggregate
    if ($type eq 'LPAREN') {
        # Could be aggregate: (choices => expr, …) or (expr to/downto expr)
        # or a plain grouped expression: (expr).
        # Use a simple heuristic: if first thing is NAME ARROW or OTHERS, it's an aggregate.
        # Otherwise parse as a grouped expression (which may also be an aggregate).
        return $self->_parse_paren_primary();
    }

    # NAME — identifier, attribute, or function call
    if ($type eq 'NAME') {
        my $name_leaf = $self->_leaf($self->_advance());
        # Attribute access: NAME ' NAME
        if ($self->_check('TICK')) {
            my @ch = ($name_leaf, $self->_leaf($self->_advance()));
            push @ch, $self->_leaf($self->_expect('NAME'));
            return $self->_node('primary', @ch);
        }
        # Function call or type conversion: NAME(args)
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

    # Fallthrough
    return $self->_node('primary', $self->_leaf($self->_advance()));
}

# _parse_paren_primary — handle (…) which could be grouped expr or aggregate
sub _parse_paren_primary {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_advance());   # LPAREN

    # Peek to decide: aggregate if OTHERS or NAME ARROW inside
    if ($self->_check('OTHERS')) {
        # aggregate: (others => expr)
        push @ch, $self->_parse_element_association();
        while ($self->_check('COMMA')) {
            push @ch, $self->_leaf($self->_advance());
            push @ch, $self->_parse_element_association();
        }
        push @ch, $self->_leaf($self->_expect('RPAREN'));
        return $self->_node('aggregate', @ch);
    }

    if ($self->_check('NAME') && $self->_peek_ahead(1)->{type} eq 'ARROW') {
        # named aggregate element: NAME => expr
        push @ch, $self->_parse_element_association();
        while ($self->_check('COMMA')) {
            push @ch, $self->_leaf($self->_advance());
            push @ch, $self->_parse_element_association();
        }
        push @ch, $self->_leaf($self->_expect('RPAREN'));
        return $self->_node('aggregate', @ch);
    }

    # Grouped expression
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    return $self->_node('primary', @ch);
}

# element_association = [ choices ARROW ] expression ;
sub _parse_element_association {
    my ($self) = @_;
    my @ch;
    if ($self->_check('OTHERS')) {
        push @ch, $self->_leaf($self->_advance());   # others
        push @ch, $self->_leaf($self->_expect('ARROW'));
    } elsif ($self->_check('NAME') && $self->_peek_ahead(1)->{type} eq 'ARROW') {
        push @ch, $self->_leaf($self->_advance());   # NAME
        push @ch, $self->_leaf($self->_advance());   # =>
    }
    push @ch, $self->_parse_expression();
    return $self->_node('element_association', @ch);
}

# ============================================================================
# Class-method convenience wrapper
# ============================================================================

sub parse_vhdl {
    my ($class, $source, $version) = @_;
    my $parser = $class->new($source, $version);
    return $parser->parse();
}

1;

__END__

=head1 NAME

CodingAdventures::VhdlParser - Hand-written recursive-descent VHDL parser

=head1 SYNOPSIS

    use CodingAdventures::VhdlParser;

    my $ast = CodingAdventures::VhdlParser->parse_vhdl(<<'VHDL');
    entity and_gate is
      port (a, b : in std_logic; y : out std_logic);
    end entity and_gate;

    architecture rtl of and_gate is
    begin
      y <= a and b;
    end architecture rtl;
    VHDL

    print $ast->rule_name;   # "design_file"

=head1 DESCRIPTION

A hand-written recursive-descent parser for the synthesizable subset of
VHDL (IEEE 1076-2008). Tokenizes input with C<CodingAdventures::VhdlLexer>
(which normalizes to lowercase) and builds an AST.

=head1 METHODS

=head2 new($source)

=head2 parse()

=head2 parse_vhdl($source) — class method

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
