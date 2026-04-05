use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::PythonLexer; 1 }, 'module loads' );

# ============================================================================
# Helper: collect token types (excluding EOF) from a source string
# ============================================================================

sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::PythonLexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub types_of_v {
    my ($source, $version) = @_;
    my $tokens = CodingAdventures::PythonLexer->tokenize($source, $version);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub values_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::PythonLexer->tokenize($source);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'whitespace-only produces only EOF' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize("   \t  ");
    is( scalar @$tokens, 1,     '1 token after skipping whitespace' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ============================================================================
# Keywords
# ============================================================================

subtest 'keyword: if' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('if');
    is( $tokens->[0]{type},  'IF', 'type is IF' );
    is( $tokens->[0]{value}, 'if', 'value is if' );
};

subtest 'keyword: elif' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('elif');
    is( $tokens->[0]{type},  'ELIF', 'type is ELIF' );
    is( $tokens->[0]{value}, 'elif', 'value is elif' );
};

subtest 'keyword: else' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('else');
    is( $tokens->[0]{type},  'ELSE', 'type is ELSE' );
    is( $tokens->[0]{value}, 'else', 'value is else' );
};

subtest 'keywords: while and for' => sub {
    is( types_of('while for'), [qw(WHILE FOR)], 'while and for types' );
};

subtest 'keyword: def' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('def');
    is( $tokens->[0]{type},  'DEF', 'type is DEF' );
    is( $tokens->[0]{value}, 'def', 'value is def' );
};

subtest 'keyword: return' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('return');
    is( $tokens->[0]{type},  'RETURN', 'type is RETURN' );
    is( $tokens->[0]{value}, 'return', 'value is return' );
};

subtest 'keyword: class' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('class');
    is( $tokens->[0]{type},  'CLASS', 'type is CLASS' );
    is( $tokens->[0]{value}, 'class', 'value is class' );
};

subtest 'keywords: import, from, as' => sub {
    is( types_of('import from as'), [qw(IMPORT FROM AS)], 'import-related keywords' );
};

subtest 'keyword: True' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('True');
    is( $tokens->[0]{type},  'TRUE', 'type is TRUE' );
    is( $tokens->[0]{value}, 'True', 'value is True' );
};

subtest 'keyword: False' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('False');
    is( $tokens->[0]{type},  'FALSE', 'type is FALSE' );
    is( $tokens->[0]{value}, 'False', 'value is False' );
};

subtest 'keyword: None' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('None');
    is( $tokens->[0]{type},  'NONE', 'type is NONE' );
    is( $tokens->[0]{value}, 'None', 'value is None' );
};

# ============================================================================
# Identifiers
# ============================================================================

subtest 'simple identifier' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('my_var');
    is( $tokens->[0]{type},  'NAME',   'type is NAME' );
    is( $tokens->[0]{value}, 'my_var', 'value is my_var' );
};

subtest 'identifier with underscore prefix' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('_private');
    is( $tokens->[0]{type},  'NAME',     'type is NAME' );
    is( $tokens->[0]{value}, '_private', 'value is _private' );
};

subtest 'dunder identifier' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('__init__');
    is( $tokens->[0]{type},  'NAME',     'type is NAME' );
    is( $tokens->[0]{value}, '__init__', 'value is __init__' );
};

subtest 'identifier with digits' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('abc123');
    is( $tokens->[0]{type},  'NAME',   'type is NAME' );
    is( $tokens->[0]{value}, 'abc123', 'value is abc123' );
};

# ============================================================================
# Number tokens
# ============================================================================

subtest 'integer number' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('42');
    is( $tokens->[0]{type},  'INT', 'type is INT' );
    is( $tokens->[0]{value}, '42',  'value is 42' );
};

subtest 'zero' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('0');
    is( $tokens->[0]{type},  'INT', 'type is INT' );
    is( $tokens->[0]{value}, '0',   'value is 0' );
};

subtest 'numbers separated by operators' => sub {
    is( types_of('1+2'), [qw(INT PLUS INT)], '1+2 types' );
};

# ============================================================================
# String tokens
# ============================================================================

subtest 'double-quoted string' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('"hello"');
    is( $tokens->[0]{type},  'STRING',  'type is STRING' );
    is( $tokens->[0]{value}, '"hello"', 'value preserved with quotes' );
};

subtest 'empty double-quoted string' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('""');
    is( $tokens->[0]{type},  'STRING', 'type is STRING' );
    is( $tokens->[0]{value}, '""',     'empty string value' );
};

subtest 'string with escape sequence' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('"a\\nb"');
    is( $tokens->[0]{type}, 'STRING', 'type is STRING' );
    like( $tokens->[0]{value}, qr/\\n/, 'escape sequence preserved in value' );
};

# ============================================================================
# Operator tokens
# ============================================================================

subtest 'equals equals ==' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('==');
    is( $tokens->[0]{type},  'EQUALS_EQUALS', 'type is EQUALS_EQUALS' );
    is( $tokens->[0]{value}, '==',            'value is ==' );
};

subtest 'assignment =' => sub {
    # = must be matched after ==, so a bare = should still tokenize as EQUALS
    my $tokens = CodingAdventures::PythonLexer->tokenize('=');
    is( $tokens->[0]{type},  'EQUALS', 'type is EQUALS' );
    is( $tokens->[0]{value}, '=',      'value is =' );
};

subtest 'plus +' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('+');
    is( $tokens->[0]{type},  'PLUS', 'type is PLUS' );
    is( $tokens->[0]{value}, '+',    'value is +' );
};

subtest 'minus -' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('-');
    is( $tokens->[0]{type},  'MINUS', 'type is MINUS' );
    is( $tokens->[0]{value}, '-',     'value is -' );
};

subtest 'star *' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('*');
    is( $tokens->[0]{type},  'STAR', 'type is STAR' );
    is( $tokens->[0]{value}, '*',    'value is *' );
};

subtest 'slash /' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('/');
    is( $tokens->[0]{type},  'SLASH', 'type is SLASH' );
    is( $tokens->[0]{value}, '/',     'value is /' );
};

# ============================================================================
# Punctuation tokens
# ============================================================================

subtest 'parentheses' => sub {
    is( types_of('()'), [qw(LPAREN RPAREN)], 'paren types' );
    is( values_of('()'), ['(', ')'], 'paren values' );
};

subtest 'comma' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize(',');
    is( $tokens->[0]{type},  'COMMA', 'type is COMMA' );
    is( $tokens->[0]{value}, ',',     'value is ,' );
};

subtest 'colon' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize(':');
    is( $tokens->[0]{type},  'COLON', 'type is COLON' );
    is( $tokens->[0]{value}, ':',     'value is :' );
};

# ============================================================================
# Composite expressions
# ============================================================================

subtest 'simple assignment: x = 1' => sub {
    is(
        types_of('x = 1'),
        [qw(NAME EQUALS INT)],
        'assignment types'
    );
    my $tokens = CodingAdventures::PythonLexer->tokenize('x = 1');
    is( $tokens->[0]{value}, 'x', 'identifier value is x' );
};

subtest 'equality check: x == 1' => sub {
    is(
        types_of('x == 1'),
        [qw(NAME EQUALS_EQUALS INT)],
        'equality check types'
    );
};

subtest 'function definition header: def foo(x):' => sub {
    is(
        types_of('def foo(x):'),
        [qw(DEF NAME LPAREN NAME RPAREN COLON)],
        'def header types'
    );
    my $tokens = CodingAdventures::PythonLexer->tokenize('def foo(x):');
    is( $tokens->[1]{value}, 'foo', 'function name is foo' );
};

subtest 'function call: foo(a, b)' => sub {
    is(
        types_of('foo(a, b)'),
        [qw(NAME LPAREN NAME COMMA NAME RPAREN)],
        'function call types'
    );
};

subtest 'class definition: class Foo:' => sub {
    is(
        types_of('class Foo:'),
        [qw(CLASS NAME COLON)],
        'class definition types'
    );
};

subtest 'import statement: from os import path' => sub {
    is(
        types_of('from os import path'),
        [qw(FROM NAME IMPORT NAME)],
        'from-import types'
    );
};

subtest 'import as: import os as operating_system' => sub {
    is(
        types_of('import os as operating_system'),
        [qw(IMPORT NAME AS NAME)],
        'import as types'
    );
};

subtest 'return statement: return True' => sub {
    is(
        types_of('return True'),
        [qw(RETURN TRUE)],
        'return true types'
    );
};

subtest 'while loop: while True:' => sub {
    is(
        types_of('while True:'),
        [qw(WHILE TRUE COLON)],
        'while true colon types'
    );
};

subtest 'arithmetic: a + b * c' => sub {
    is(
        types_of('a + b * c'),
        [qw(NAME PLUS NAME STAR NAME)],
        'arithmetic types'
    );
};

subtest 'None literal: x = None' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('x = None');
    my ($none_tok) = grep { $_->{type} eq 'NONE' } @$tokens;
    ok( $none_tok, 'has NONE token' );
    is( $none_tok->{value}, 'None', 'None value preserved' );
};

# ============================================================================
# Whitespace handling
# ============================================================================

subtest 'spaces between tokens are consumed silently' => sub {
    is(
        types_of('x = 1'),
        [qw(NAME EQUALS INT)],
        'no WHITESPACE tokens in output'
    );
};

subtest 'tabs between tokens consumed silently' => sub {
    is(
        types_of("x\t=\t1"),
        [qw(NAME EQUALS INT)],
        'only value tokens in output'
    );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking: x = 42' => sub {
    # x _ = _ 4 2
    # 1 2 3 4 5 6
    my $tokens = CodingAdventures::PythonLexer->tokenize('x = 42');
    is( $tokens->[0]{col}, 1, 'x at col 1' );
    is( $tokens->[1]{col}, 3, '= at col 3' );
    is( $tokens->[2]{col}, 5, 'INT at col 5' );
};

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('x = 1');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('1');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest '@ tokenizes as AT (decorator operator)' => sub {
    my $tokens = CodingAdventures::PythonLexer->tokenize('@');
    is( $tokens->[0]{type},  'AT', 'type is AT' );
    is( $tokens->[0]{value}, '@',  'value is @' );
};

subtest 'unexpected character $ raises die' => sub {
    ok(
        dies { CodingAdventures::PythonLexer->tokenize('$x') },
        'unexpected $ causes die'
    );
};

# ============================================================================
# Version support
# ============================================================================

subtest 'DEFAULT_VERSION is 3.12' => sub {
    is( CodingAdventures::PythonLexer::DEFAULT_VERSION(), '3.12', 'default version' );
};

subtest 'SUPPORTED_VERSIONS contains expected versions' => sub {
    my @versions = @CodingAdventures::PythonLexer::SUPPORTED_VERSIONS;
    is( scalar @versions, 6, '6 supported versions' );
    ok( (grep { $_ eq '3.12' } @versions), '3.12 in supported versions' );
    ok( (grep { $_ eq '2.7' } @versions), '2.7 in supported versions' );
};

subtest 'tokenize with explicit version parameter' => sub {
    is(
        types_of_v('x = 1', '3.12'),
        [qw(NAME EQUALS INT NEWLINE)],
        'explicit version 3.12'
    );
};

subtest 'tokenize with undef version defaults to 3.12' => sub {
    is(
        types_of_v('x = 1', undef),
        [qw(NAME EQUALS INT NEWLINE)],
        'undef version defaults'
    );
};

subtest 'tokenize with empty string version defaults to 3.12' => sub {
    is(
        types_of_v('x = 1', ''),
        [qw(NAME EQUALS INT NEWLINE)],
        'empty string version defaults'
    );
};

done_testing;
