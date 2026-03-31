use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::RubyLexer; 1 }, 'module loads' );

# ============================================================================
# Helper: collect token types (excluding EOF) from a source string
# ============================================================================

sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::RubyLexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub values_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::RubyLexer->tokenize($source);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'whitespace-only produces only EOF' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize("   \t  ");
    is( scalar @$tokens, 1,     '1 token after skipping whitespace' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ============================================================================
# Keywords
# ============================================================================

subtest 'keyword: def' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('def');
    is( $tokens->[0]{type},  'DEF', 'type is DEF' );
    is( $tokens->[0]{value}, 'def', 'value is def' );
};

subtest 'keyword: end' => sub {
    # Ruby uses end to close blocks — fundamentally different from Python's
    # indentation approach
    my $tokens = CodingAdventures::RubyLexer->tokenize('end');
    is( $tokens->[0]{type},  'END', 'type is END' );
    is( $tokens->[0]{value}, 'end', 'value is end' );
};

subtest 'keyword: class' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('class');
    is( $tokens->[0]{type},  'CLASS', 'type is CLASS' );
    is( $tokens->[0]{value}, 'class', 'value is class' );
};

subtest 'keyword: module' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('module');
    is( $tokens->[0]{type},  'MODULE', 'type is MODULE' );
    is( $tokens->[0]{value}, 'module', 'value is module' );
};

subtest 'keyword: if' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('if');
    is( $tokens->[0]{type},  'IF', 'type is IF' );
    is( $tokens->[0]{value}, 'if', 'value is if' );
};

subtest 'keyword: elsif' => sub {
    # Ruby uses elsif (not elif like Python)
    my $tokens = CodingAdventures::RubyLexer->tokenize('elsif');
    is( $tokens->[0]{type},  'ELSIF', 'type is ELSIF' );
    is( $tokens->[0]{value}, 'elsif', 'value is elsif' );
};

subtest 'keyword: else' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('else');
    is( $tokens->[0]{type},  'ELSE', 'type is ELSE' );
    is( $tokens->[0]{value}, 'else', 'value is else' );
};

subtest 'keyword: unless' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('unless');
    is( $tokens->[0]{type},  'UNLESS', 'type is UNLESS' );
    is( $tokens->[0]{value}, 'unless', 'value is unless' );
};

subtest 'keywords: while, until, for, do' => sub {
    is(
        types_of('while until for do'),
        [qw(WHILE UNTIL FOR DO)],
        'loop keyword types'
    );
};

subtest 'keyword: return' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('return');
    is( $tokens->[0]{type},  'RETURN', 'type is RETURN' );
    is( $tokens->[0]{value}, 'return', 'value is return' );
};

subtest 'exception keywords: begin, rescue, ensure' => sub {
    is(
        types_of('begin rescue ensure'),
        [qw(BEGIN RESCUE ENSURE)],
        'exception keyword types'
    );
};

subtest 'keywords: require and puts' => sub {
    is(
        types_of('require puts'),
        [qw(REQUIRE PUTS)],
        'require and puts types'
    );
};

subtest 'keywords: yield and then' => sub {
    is(
        types_of('yield then'),
        [qw(YIELD THEN)],
        'yield and then types'
    );
};

subtest 'keyword: true' => sub {
    # Ruby uses lowercase true (unlike Python's True)
    my $tokens = CodingAdventures::RubyLexer->tokenize('true');
    is( $tokens->[0]{type},  'TRUE', 'type is TRUE' );
    is( $tokens->[0]{value}, 'true', 'value is true' );
};

subtest 'keyword: false' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('false');
    is( $tokens->[0]{type},  'FALSE', 'type is FALSE' );
    is( $tokens->[0]{value}, 'false', 'value is false' );
};

subtest 'keyword: nil' => sub {
    # Ruby uses nil (not null or None)
    my $tokens = CodingAdventures::RubyLexer->tokenize('nil');
    is( $tokens->[0]{type},  'NIL', 'type is NIL' );
    is( $tokens->[0]{value}, 'nil', 'value is nil' );
};

subtest 'keywords: and, or, not' => sub {
    is(
        types_of('and or not'),
        [qw(AND OR NOT)],
        'logical operator keyword types'
    );
};

# ============================================================================
# Identifiers
# ============================================================================

subtest 'simple identifier' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('my_var');
    is( $tokens->[0]{type},  'NAME',   'type is NAME' );
    is( $tokens->[0]{value}, 'my_var', 'value is my_var' );
};

subtest 'CamelCase class name' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('MyClass');
    is( $tokens->[0]{type},  'NAME',    'type is NAME' );
    is( $tokens->[0]{value}, 'MyClass', 'value is MyClass' );
};

subtest 'identifier with underscore prefix' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('_private');
    is( $tokens->[0]{type},  'NAME',     'type is NAME' );
    is( $tokens->[0]{value}, '_private', 'value is _private' );
};

subtest 'identifier with digits' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('abc123');
    is( $tokens->[0]{type},  'NAME',   'type is NAME' );
    is( $tokens->[0]{value}, 'abc123', 'value is abc123' );
};

# ============================================================================
# Number tokens
# ============================================================================

subtest 'integer number' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('42');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '42',     'value is 42' );
};

subtest 'zero' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('0');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '0',      'value is 0' );
};

subtest 'numbers separated by operators' => sub {
    is( types_of('1+2'), [qw(NUMBER PLUS NUMBER)], '1+2 types' );
};

# ============================================================================
# String tokens
# ============================================================================

subtest 'double-quoted string' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('"hello"');
    is( $tokens->[0]{type},  'STRING',  'type is STRING' );
    is( $tokens->[0]{value}, '"hello"', 'value preserved with quotes' );
};

subtest 'empty double-quoted string' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('""');
    is( $tokens->[0]{type},  'STRING', 'type is STRING' );
    is( $tokens->[0]{value}, '""',     'empty string value' );
};

subtest 'string with escape sequence' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('"a\\nb"');
    is( $tokens->[0]{type}, 'STRING', 'type is STRING' );
    like( $tokens->[0]{value}, qr/\\n/, 'escape sequence preserved in value' );
};

# ============================================================================
# Operator tokens — multi-char (must match before single-char prefixes)
# ============================================================================

subtest 'equals equals ==' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('==');
    is( $tokens->[0]{type},  'EQUALS_EQUALS', 'type is EQUALS_EQUALS' );
    is( $tokens->[0]{value}, '==',            'value is ==' );
};

subtest 'range operator ..' => sub {
    # Ruby range operator — unique to Ruby, e.g. (1..10)
    my $tokens = CodingAdventures::RubyLexer->tokenize('..');
    is( $tokens->[0]{type},  'DOT_DOT', 'type is DOT_DOT' );
    is( $tokens->[0]{value}, '..',      'value is ..' );
};

subtest 'hash rocket =>' => sub {
    # Ruby hash rocket, used in hash literals: { key => value }
    my $tokens = CodingAdventures::RubyLexer->tokenize('=>');
    is( $tokens->[0]{type},  'HASH_ROCKET', 'type is HASH_ROCKET' );
    is( $tokens->[0]{value}, '=>',          'value is =>' );
};

subtest 'not equals !=' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('!=');
    is( $tokens->[0]{type},  'NOT_EQUALS', 'type is NOT_EQUALS' );
    is( $tokens->[0]{value}, '!=',         'value is !=' );
};

subtest 'less than or equal <=' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('<=');
    is( $tokens->[0]{type},  'LESS_EQUALS', 'type is LESS_EQUALS' );
    is( $tokens->[0]{value}, '<=',          'value is <=' );
};

subtest 'greater than or equal >=' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('>=');
    is( $tokens->[0]{type},  'GREATER_EQUALS', 'type is GREATER_EQUALS' );
    is( $tokens->[0]{value}, '>=',             'value is >=' );
};

# ============================================================================
# Operator tokens — single-char
# ============================================================================

subtest 'assignment =' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('=');
    is( $tokens->[0]{type},  'EQUALS', 'type is EQUALS' );
    is( $tokens->[0]{value}, '=',      'value is =' );
};

subtest 'plus +' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('+');
    is( $tokens->[0]{type},  'PLUS', 'type is PLUS' );
    is( $tokens->[0]{value}, '+',    'value is +' );
};

subtest 'minus -' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('-');
    is( $tokens->[0]{type},  'MINUS', 'type is MINUS' );
    is( $tokens->[0]{value}, '-',     'value is -' );
};

subtest 'star *' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('*');
    is( $tokens->[0]{type},  'STAR', 'type is STAR' );
    is( $tokens->[0]{value}, '*',    'value is *' );
};

subtest 'slash /' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('/');
    is( $tokens->[0]{type},  'SLASH', 'type is SLASH' );
    is( $tokens->[0]{value}, '/',     'value is /' );
};

subtest 'less than <' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('<');
    is( $tokens->[0]{type},  'LESS_THAN', 'type is LESS_THAN' );
    is( $tokens->[0]{value}, '<',         'value is <' );
};

subtest 'greater than >' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('>');
    is( $tokens->[0]{type},  'GREATER_THAN', 'type is GREATER_THAN' );
    is( $tokens->[0]{value}, '>',            'value is >' );
};

# ============================================================================
# Punctuation tokens
# ============================================================================

subtest 'parentheses' => sub {
    is( types_of('()'), [qw(LPAREN RPAREN)], 'paren types' );
    is( values_of('()'), ['(', ')'], 'paren values' );
};

subtest 'comma' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize(',');
    is( $tokens->[0]{type},  'COMMA', 'type is COMMA' );
    is( $tokens->[0]{value}, ',',     'value is ,' );
};

subtest 'colon' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize(':');
    is( $tokens->[0]{type},  'COLON', 'type is COLON' );
    is( $tokens->[0]{value}, ':',     'value is :' );
};

# ============================================================================
# Composite expressions
# ============================================================================

subtest 'simple assignment: x = 1' => sub {
    is(
        types_of('x = 1'),
        [qw(NAME EQUALS NUMBER)],
        'assignment types'
    );
    my $tokens = CodingAdventures::RubyLexer->tokenize('x = 1');
    is( $tokens->[0]{value}, 'x', 'identifier value is x' );
};

subtest 'function definition header: def greet(name)' => sub {
    is(
        types_of('def greet(name)'),
        [qw(DEF NAME LPAREN NAME RPAREN)],
        'def header types'
    );
    my $tokens = CodingAdventures::RubyLexer->tokenize('def greet(name)');
    is( $tokens->[1]{value}, 'greet', 'function name is greet' );
};

subtest 'class definition: class Animal' => sub {
    is(
        types_of('class Animal'),
        [qw(CLASS NAME)],
        'class definition types'
    );
};

subtest 'module definition: module Greetable' => sub {
    is(
        types_of('module Greetable'),
        [qw(MODULE NAME)],
        'module definition types'
    );
};

subtest 'return statement: return true' => sub {
    is(
        types_of('return true'),
        [qw(RETURN TRUE)],
        'return true types'
    );
};

subtest 'nil literal: x = nil' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('x = nil');
    my ($nil_tok) = grep { $_->{type} eq 'NIL' } @$tokens;
    ok( $nil_tok,                  'has NIL token' );
    is( $nil_tok->{value}, 'nil',  'nil value preserved' );
};

subtest 'range: 1..10' => sub {
    is(
        types_of('1..10'),
        [qw(NUMBER DOT_DOT NUMBER)],
        'range types'
    );
};

subtest 'hash rocket in hash literal: x => y' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('x => y');
    my ($hr) = grep { $_->{type} eq 'HASH_ROCKET' } @$tokens;
    ok( $hr,               'has HASH_ROCKET token' );
    is( $hr->{value}, '=>', 'hash rocket value is =>' );
};

subtest 'not equals: a != b' => sub {
    is(
        types_of('a != b'),
        [qw(NAME NOT_EQUALS NAME)],
        'not equals types'
    );
};

subtest 'less than or equal: a <= b' => sub {
    is(
        types_of('a <= b'),
        [qw(NAME LESS_EQUALS NAME)],
        'less equals types'
    );
};

subtest 'greater than or equal: a >= b' => sub {
    is(
        types_of('a >= b'),
        [qw(NAME GREATER_EQUALS NAME)],
        'greater equals types'
    );
};

subtest 'arithmetic: a + b * c' => sub {
    is(
        types_of('a + b * c'),
        [qw(NAME PLUS NAME STAR NAME)],
        'arithmetic types'
    );
};

subtest 'rescue clause: rescue RuntimeError' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('rescue RuntimeError');
    my ($rescue_tok) = grep { $_->{type} eq 'RESCUE' } @$tokens;
    ok( $rescue_tok, 'has RESCUE token' );
};

subtest 'unless condition: unless x == 0' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('unless x == 0');
    my ($unless_tok) = grep { $_->{type} eq 'UNLESS' } @$tokens;
    ok( $unless_tok, 'has UNLESS token' );
};

subtest 'require call: require path' => sub {
    is(
        types_of('require path'),
        [qw(REQUIRE NAME)],
        'require types'
    );
};

subtest 'puts call: puts x' => sub {
    is(
        types_of('puts x'),
        [qw(PUTS NAME)],
        'puts types'
    );
};

# ============================================================================
# Whitespace handling
# ============================================================================

subtest 'spaces between tokens are consumed silently' => sub {
    is(
        types_of('x = 1'),
        [qw(NAME EQUALS NUMBER)],
        'no WHITESPACE tokens in output'
    );
};

subtest 'tabs between tokens consumed silently' => sub {
    is(
        types_of("x\t=\t1"),
        [qw(NAME EQUALS NUMBER)],
        'only value tokens in output'
    );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking: x = 42' => sub {
    # x _ = _ 4 2
    # 1 2 3 4 5 6
    my $tokens = CodingAdventures::RubyLexer->tokenize('x = 42');
    is( $tokens->[0]{col}, 1, 'x at col 1' );
    is( $tokens->[1]{col}, 3, '= at col 3' );
    is( $tokens->[2]{col}, 5, '42 at col 5' );
};

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('x = 1');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::RubyLexer->tokenize('1');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected character @ raises die' => sub {
    # @ is used for Ruby instance variables but is not in the grammar
    ok(
        dies { CodingAdventures::RubyLexer->tokenize('@instance') },
        'unexpected @ causes die'
    );
};

subtest 'unexpected character $ raises die' => sub {
    ok(
        dies { CodingAdventures::RubyLexer->tokenize('$global') },
        'unexpected $ causes die'
    );
};

done_testing;
