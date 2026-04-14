use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::VhdlLexer; 1 }, 'module loads' );

# ============================================================================
# Helper: collect token types (excluding EOF) from a source string
# ============================================================================

sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::VhdlLexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub values_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::VhdlLexer->tokenize($source);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub types_of_version {
    my ($source, $version) = @_;
    my $tokens = CodingAdventures::VhdlLexer->tokenize($source, $version);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub values_of_version {
    my ($source, $version) = @_;
    my $tokens = CodingAdventures::VhdlLexer->tokenize($source, $version);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'whitespace-only produces only EOF' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize("   \t\n  ");
    is( scalar @$tokens, 1,     '1 token after skipping whitespace' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

subtest 'default version matches explicit 2008' => sub {
    is(
        types_of('entity'),
        types_of_version('entity', '2008'),
        'default token types match 2008'
    );
    is(
        values_of('ENTITY'),
        values_of_version('ENTITY', '2008'),
        'default token values match 2008'
    );
};

subtest 'unknown version raises die' => sub {
    ok(
        dies { CodingAdventures::VhdlLexer->tokenize('entity', '2099') },
        'unsupported version dies'
    );
};

subtest 'VHDL line comment only produces EOF' => sub {
    # VHDL single-line comments start with -- (two dashes)
    # Unlike Verilog (// comments) or C (// and /* */)
    my $tokens = CodingAdventures::VhdlLexer->tokenize('-- this is a VHDL comment');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'comment consumed, only EOF' );
};

# ============================================================================
# Structure keywords
# ============================================================================
#
# VHDL programs are organized into design units:
#   entity: declares the external interface (ports)
#   architecture: describes the internal implementation
# One entity can have multiple architectures (behavioral, structural, RTL).

subtest 'keyword: entity' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('entity');
    is( $tokens->[0]{type},  'ENTITY', 'type is ENTITY' );
    is( $tokens->[0]{value}, 'entity', 'value is entity' );
};

subtest 'keyword: architecture' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('architecture');
    is( $tokens->[0]{type},  'ARCHITECTURE', 'type is ARCHITECTURE' );
    is( $tokens->[0]{value}, 'architecture', 'value is architecture' );
};

subtest 'keyword: is' => sub {
    # `is` connects a name to its definition: "entity adder is"
    my $tokens = CodingAdventures::VhdlLexer->tokenize('is');
    is( $tokens->[0]{type},  'IS', 'type is IS' );
    is( $tokens->[0]{value}, 'is', 'value is is' );
};

subtest 'keyword: of' => sub {
    # `of` links an architecture to its entity: "architecture rtl of adder is"
    my $tokens = CodingAdventures::VhdlLexer->tokenize('of');
    is( $tokens->[0]{type},  'OF', 'type is OF' );
    is( $tokens->[0]{value}, 'of', 'value is of' );
};

subtest 'keywords: begin and end' => sub {
    is(
        types_of('begin end'),
        [qw(BEGIN END)],
        'begin and end types'
    );
};

subtest 'keyword: port' => sub {
    # `port` introduces the port list of an entity
    my $tokens = CodingAdventures::VhdlLexer->tokenize('port');
    is( $tokens->[0]{type},  'PORT', 'type is PORT' );
    is( $tokens->[0]{value}, 'port', 'value is port' );
};

subtest 'keyword: generic' => sub {
    # `generic` introduces customization parameters
    my $tokens = CodingAdventures::VhdlLexer->tokenize('generic');
    is( $tokens->[0]{type},  'GENERIC', 'type is GENERIC' );
    is( $tokens->[0]{value}, 'generic', 'value is generic' );
};

subtest 'keyword: component' => sub {
    # `component` declares a lower-level module for instantiation
    my $tokens = CodingAdventures::VhdlLexer->tokenize('component');
    is( $tokens->[0]{type},  'COMPONENT', 'type is COMPONENT' );
    is( $tokens->[0]{value}, 'component', 'value is component' );
};

subtest 'keyword: package' => sub {
    # `package` groups declarations for reuse (like a C header or Python module)
    my $tokens = CodingAdventures::VhdlLexer->tokenize('package');
    is( $tokens->[0]{type},  'PACKAGE', 'type is PACKAGE' );
    is( $tokens->[0]{value}, 'package', 'value is package' );
};

subtest 'keyword: use' => sub {
    # `use` imports declarations: use ieee.std_logic_1164.all;
    my $tokens = CodingAdventures::VhdlLexer->tokenize('use');
    is( $tokens->[0]{type},  'USE', 'type is USE' );
    is( $tokens->[0]{value}, 'use', 'value is use' );
};

subtest 'keyword: library' => sub {
    # `library` names a design library: library ieee;
    my $tokens = CodingAdventures::VhdlLexer->tokenize('library');
    is( $tokens->[0]{type},  'LIBRARY', 'type is LIBRARY' );
    is( $tokens->[0]{value}, 'library', 'value is library' );
};

# ============================================================================
# Type / signal declaration keywords
# ============================================================================

subtest 'keyword: signal' => sub {
    # `signal` declares a hardware net (like wire/reg in Verilog)
    my $tokens = CodingAdventures::VhdlLexer->tokenize('signal');
    is( $tokens->[0]{type},  'SIGNAL', 'type is SIGNAL' );
    is( $tokens->[0]{value}, 'signal', 'value is signal' );
};

subtest 'keyword: variable' => sub {
    # `variable` declares a software variable (only in processes)
    my $tokens = CodingAdventures::VhdlLexer->tokenize('variable');
    is( $tokens->[0]{type},  'VARIABLE', 'type is VARIABLE' );
    is( $tokens->[0]{value}, 'variable', 'value is variable' );
};

subtest 'keyword: constant' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('constant');
    is( $tokens->[0]{type},  'CONSTANT', 'type is CONSTANT' );
    is( $tokens->[0]{value}, 'constant', 'value is constant' );
};

subtest 'keyword: type' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('type');
    is( $tokens->[0]{type},  'TYPE', 'type is TYPE' );
    is( $tokens->[0]{value}, 'type', 'value is type' );
};

subtest 'keyword: subtype' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('subtype');
    is( $tokens->[0]{type},  'SUBTYPE', 'type is SUBTYPE' );
    is( $tokens->[0]{value}, 'subtype', 'value is subtype' );
};

subtest 'port direction keywords: in, out, inout, buffer' => sub {
    is(
        types_of('in out inout buffer'),
        [qw(IN OUT INOUT BUFFER)],
        'port direction types'
    );
};

# ============================================================================
# Control flow keywords
# ============================================================================

subtest 'keywords: if, elsif, else, then' => sub {
    # VHDL uses `elsif` (one word), like Ruby
    is(
        types_of('if elsif else then'),
        [qw(IF ELSIF ELSE THEN)],
        'if/elsif/else/then types'
    );
};

subtest 'keyword: case' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('case');
    is( $tokens->[0]{type},  'CASE', 'type is CASE' );
    is( $tokens->[0]{value}, 'case', 'value is case' );
};

subtest 'keywords: when and others' => sub {
    # `when` introduces a case alternative; `others` is the default
    is(
        types_of('when others'),
        [qw(WHEN OTHERS)],
        'when and others types'
    );
};

subtest 'keywords: for, while, loop' => sub {
    # for i in 0 to 7 loop ... end loop;
    is(
        types_of('for while loop'),
        [qw(FOR WHILE LOOP)],
        'for while loop types'
    );
};

subtest 'keyword: process' => sub {
    # `process` introduces a sequential block triggered by its sensitivity list
    my $tokens = CodingAdventures::VhdlLexer->tokenize('process');
    is( $tokens->[0]{type},  'PROCESS', 'type is PROCESS' );
    is( $tokens->[0]{value}, 'process', 'value is process' );
};

subtest 'keyword: wait' => sub {
    # wait until rising_edge(clk); — suspend process until condition
    my $tokens = CodingAdventures::VhdlLexer->tokenize('wait');
    is( $tokens->[0]{type},  'WAIT', 'type is WAIT' );
    is( $tokens->[0]{value}, 'wait', 'value is wait' );
};

# ============================================================================
# Operator keywords
# ============================================================================
#
# VHDL uses English-word operators for logical operations, inheriting from
# Ada. Compare Verilog: assign y = (a & b) | c;
#         vs VHDL:      y <= (a and b) or c;

subtest 'logical operator keywords: and, or, not' => sub {
    is(
        types_of('and or not'),
        [qw(AND OR NOT)],
        'and or not types'
    );
};

subtest 'logical operator keywords: nand, nor, xor, xnor' => sub {
    is(
        types_of('nand nor xor xnor'),
        [qw(NAND NOR XOR XNOR)],
        'nand nor xor xnor types'
    );
};

# ============================================================================
# Symbol operators
# ============================================================================
#
# VHDL operator comparison table:
#
#   Meaning      | VHDL | Verilog | C
#   -------------|------|---------|---
#   Signal assign| <=   | <=      | -
#   Var assign   | :=   | =       | =
#   Not equal    | /=   | !=      | !=
#   Named assoc  | =>   | -       | -

subtest 'signal assignment / less-equals: <=' => sub {
    # `<=` is VHDL signal assignment AND less-than-or-equal; context resolves
    my $tokens = CodingAdventures::VhdlLexer->tokenize('<=');
    is( $tokens->[0]{type},  'LESS_EQUALS', 'type is LESS_EQUALS' );
    is( $tokens->[0]{value}, '<=',          'value is <=' );
};

subtest 'variable assignment: :=' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize(':=');
    is( $tokens->[0]{type},  'VAR_ASSIGN', 'type is VAR_ASSIGN' );
    is( $tokens->[0]{value}, ':=',         'value is :=' );
};

subtest 'equality comparison: =' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('=');
    is( $tokens->[0]{type},  'EQUALS', 'type is EQUALS' );
    is( $tokens->[0]{value}, '=',      'value is =' );
};

subtest 'not equals: /=' => sub {
    # VHDL uses /= for inequality (not != like C/Verilog)
    my $tokens = CodingAdventures::VhdlLexer->tokenize('/=');
    is( $tokens->[0]{type},  'NOT_EQUALS', 'type is NOT_EQUALS' );
    is( $tokens->[0]{value}, '/=',         'value is /=' );
};

subtest 'arrow: =>' => sub {
    # => is used in port maps and aggregates: port map (clk => system_clock)
    my $tokens = CodingAdventures::VhdlLexer->tokenize('=>');
    is( $tokens->[0]{type},  'ARROW', 'type is ARROW' );
    is( $tokens->[0]{value}, '=>',    'value is =>' );
};

subtest 'power: **' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('**');
    is( $tokens->[0]{type},  'POWER', 'type is POWER' );
    is( $tokens->[0]{value}, '**',    'value is **' );
};

subtest 'less than: <' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('<');
    is( $tokens->[0]{type},  'LESS_THAN', 'type is LESS_THAN' );
    is( $tokens->[0]{value}, '<',         'value is <' );
};

subtest 'greater than: >' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('>');
    is( $tokens->[0]{type},  'GREATER_THAN', 'type is GREATER_THAN' );
    is( $tokens->[0]{value}, '>',            'value is >' );
};

subtest 'greater-equals: >=' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('>=');
    is( $tokens->[0]{type},  'GREATER_EQUALS', 'type is GREATER_EQUALS' );
    is( $tokens->[0]{value}, '>=',             'value is >=' );
};

subtest 'arithmetic operators: + - * /' => sub {
    is(
        types_of('+ - * /'),
        [qw(PLUS MINUS STAR SLASH)],
        'arithmetic operator types'
    );
};

subtest 'concatenation: &' => sub {
    # & is string/bit concatenation in VHDL, NOT bitwise AND
    # Bitwise AND is the keyword `and`
    my $tokens = CodingAdventures::VhdlLexer->tokenize('&');
    is( $tokens->[0]{type},  'AMPERSAND', 'type is AMPERSAND' );
    is( $tokens->[0]{value}, '&',         'value is &' );
};

# ============================================================================
# Literals
# ============================================================================

subtest 'plain integer' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('42');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '42',     'value is 42' );
};

subtest 'integer with underscore separator' => sub {
    # 1_000_000 = 1000000 — underscores are visual separators
    my $tokens = CodingAdventures::VhdlLexer->tokenize('1_000');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '1_000',  'value is 1_000' );
};

subtest 'hex bit string: X"FF"' => sub {
    # X"FF" = 8 bits, hex value FF — VHDL equivalent of Verilog's 8'hFF
    my $tokens = CodingAdventures::VhdlLexer->tokenize('X"FF"');
    is( $tokens->[0]{type}, 'BIT_STRING', 'type is BIT_STRING' );
    # case_sensitive:false lowercases the prefix
    like( $tokens->[0]{value}, qr/x"ff"/i, 'value contains x"ff"' );
};

subtest 'binary bit string: B"1010"' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('B"1010"');
    is( $tokens->[0]{type}, 'BIT_STRING', 'type is BIT_STRING' );
};

subtest 'octal bit string: O"77"' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('O"77"');
    is( $tokens->[0]{type}, 'BIT_STRING', 'type is BIT_STRING' );
};

subtest 'double-quoted string' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('"hello"');
    is( $tokens->[0]{type},  'STRING',  'type is STRING' );
    is( $tokens->[0]{value}, '"hello"', 'value preserved with quotes' );
};

subtest 'empty double-quoted string' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('""');
    is( $tokens->[0]{type},  'STRING', 'type is STRING' );
    is( $tokens->[0]{value}, '""',     'empty string value' );
};

subtest "std_logic char literal: '0'" => sub {
    # '0' = logic low in std_logic type
    my $tokens = CodingAdventures::VhdlLexer->tokenize("'0'");
    is( $tokens->[0]{type},  'CHAR_LITERAL', 'type is CHAR_LITERAL' );
    is( $tokens->[0]{value}, "'0'",          "value is '0'" );
};

subtest "std_logic char literal: '1'" => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize("'1'");
    is( $tokens->[0]{type},  'CHAR_LITERAL', 'type is CHAR_LITERAL' );
    is( $tokens->[0]{value}, "'1'",          "value is '1'" );
};

# ============================================================================
# Comments
# ============================================================================

subtest 'line comment after tokens is skipped' => sub {
    is(
        types_of('signal clk : std_logic; -- clock input'),
        [qw(SIGNAL NAME COLON NAME SEMICOLON)],
        'no comment token in output'
    );
};

subtest 'comment on its own line is consumed' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize("-- comment\nentity foo is");
    my ($entity_tok) = grep { $_->{type} eq 'ENTITY' } @$tokens;
    ok( $entity_tok,               'has ENTITY token' );
    is( $entity_tok->{value}, 'entity', 'ENTITY value is entity' );
};

# ============================================================================
# Case insensitivity
# ============================================================================
#
# VHDL is case-insensitive — a unique feature among HDLs. The grammar sets
# case_sensitive: false, causing the lexer to lowercase all input.

subtest 'ENTITY (uppercase) tokenizes as entity keyword' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('ENTITY');
    is( $tokens->[0]{type},  'ENTITY', 'type is ENTITY' );
    is( $tokens->[0]{value}, 'entity', 'value is lowercased to entity' );
};

subtest 'Architecture (mixed case) tokenizes as architecture keyword' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('Architecture');
    is( $tokens->[0]{type},  'ARCHITECTURE', 'type is ARCHITECTURE' );
    is( $tokens->[0]{value}, 'architecture', 'value is lowercased' );
};

subtest 'SIGNAL and signal produce identical tokens' => sub {
    my $t1 = CodingAdventures::VhdlLexer->tokenize('SIGNAL')->[0];
    my $t2 = CodingAdventures::VhdlLexer->tokenize('signal')->[0];
    is( $t1->{type},  $t2->{type},  'same type' );
    is( $t1->{value}, $t2->{value}, 'same value' );
};

# ============================================================================
# Identifiers
# ============================================================================

subtest 'simple identifier' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('data_bus');
    is( $tokens->[0]{type},  'NAME',     'type is NAME' );
    is( $tokens->[0]{value}, 'data_bus', 'value is data_bus' );
};

subtest 'identifier with digits' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('clk32');
    is( $tokens->[0]{type},  'NAME',  'type is NAME' );
    is( $tokens->[0]{value}, 'clk32', 'value is clk32' );
};

subtest 'non-keyword name is NAME' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('std_logic');
    is( $tokens->[0]{type},  'NAME',      'type is NAME' );
    is( $tokens->[0]{value}, 'std_logic', 'value is std_logic' );
};

# ============================================================================
# Composite expressions
# ============================================================================

subtest 'entity declaration header: entity adder is' => sub {
    is(
        types_of('entity adder is'),
        [qw(ENTITY NAME IS)],
        'entity declaration types'
    );
    my $tokens = CodingAdventures::VhdlLexer->tokenize('entity adder is');
    is( $tokens->[1]{value}, 'adder', 'entity name is adder' );
};

subtest 'architecture header: architecture rtl of adder is' => sub {
    is(
        types_of('architecture rtl of adder is'),
        [qw(ARCHITECTURE NAME OF NAME IS)],
        'architecture header types'
    );
};

subtest 'port declaration: port ( clk : in std_logic )' => sub {
    is(
        types_of('port ( clk : in std_logic )'),
        [qw(PORT LPAREN NAME COLON IN NAME RPAREN)],
        'port declaration types'
    );
};

subtest 'signal assignment: y <= a and b' => sub {
    # In VHDL, `and` is a keyword operator (not a symbol like & in Verilog)
    is(
        types_of('y <= a and b'),
        [qw(NAME LESS_EQUALS NAME AND NAME)],
        'signal assignment with and'
    );
};

subtest 'variable assignment: count := count + 1' => sub {
    is(
        types_of('count := count + 1'),
        [qw(NAME VAR_ASSIGN NAME PLUS NUMBER)],
        'variable assignment types'
    );
};

subtest 'if/elsif/else keywords in context' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize(
        'if x = 1 then y := 0; elsif x = 2 then y := 1; else y := 2; end if;'
    );
    my ($if_tok)    = grep { $_->{type} eq 'IF'    } @$tokens;
    my ($elsif_tok) = grep { $_->{type} eq 'ELSIF' } @$tokens;
    my ($else_tok)  = grep { $_->{type} eq 'ELSE'  } @$tokens;
    ok( $if_tok,    'has IF token' );
    ok( $elsif_tok, 'has ELSIF token' );
    ok( $else_tok,  'has ELSE token' );
};

subtest 'case/when/others structure' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize(
        'case sel when "00" => y := 0; when others => y := 1; end case;'
    );
    my ($case_tok)   = grep { $_->{type} eq 'CASE'   } @$tokens;
    my ($when_tok)   = grep { $_->{type} eq 'WHEN'   } @$tokens;
    my ($others_tok) = grep { $_->{type} eq 'OTHERS' } @$tokens;
    ok( $case_tok,   'has CASE token' );
    ok( $when_tok,   'has WHEN token' );
    ok( $others_tok, 'has OTHERS token' );
};

subtest 'process with sensitivity list: process (clk)' => sub {
    is(
        types_of('process (clk)'),
        [qw(PROCESS LPAREN NAME RPAREN)],
        'process sensitivity list types'
    );
};

subtest 'library use clause: library ieee;' => sub {
    is(
        types_of('library ieee;'),
        [qw(LIBRARY NAME SEMICOLON)],
        'library clause types'
    );
};

subtest 'logical expression: a and b or not c' => sub {
    is(
        types_of('a and b or not c'),
        [qw(NAME AND NAME OR NOT NAME)],
        'logical expression types'
    );
};

subtest 'not-equals comparison: a /= b' => sub {
    is(
        types_of('a /= b'),
        [qw(NAME NOT_EQUALS NAME)],
        'not equals types'
    );
};

subtest 'constant declaration: constant WIDTH : integer := 8;' => sub {
    is(
        types_of('constant WIDTH : integer := 8;'),
        [qw(CONSTANT NAME COLON NAME VAR_ASSIGN NUMBER SEMICOLON)],
        'constant declaration types'
    );
};

# ============================================================================
# Whitespace handling
# ============================================================================

subtest 'spaces between tokens are consumed silently' => sub {
    is(
        types_of('a <= b'),
        [qw(NAME LESS_EQUALS NAME)],
        'no WHITESPACE tokens in output'
    );
};

subtest 'tabs and newlines consumed' => sub {
    is(
        types_of("a\t<=\nb"),
        [qw(NAME LESS_EQUALS NAME)],
        'tabs and newlines consumed'
    );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking' => sub {
    # a _ < = _ b
    # 1 2 3 4 5 6
    my $tokens = CodingAdventures::VhdlLexer->tokenize('a <= b');
    is( $tokens->[0]{col}, 1, 'a at col 1' );
    is( $tokens->[1]{col}, 3, '<= at col 3' );
    is( $tokens->[2]{col}, 6, 'b at col 6' );
};

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('entity foo is');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::VhdlLexer->tokenize('1');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected character raises die' => sub {
    ok(
        dies { CodingAdventures::VhdlLexer->tokenize("\x{FFFE}") },
        'unexpected character causes die'
    );
};

done_testing;
