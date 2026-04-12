use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::JavaLexer; 1 }, 'module loads' );

# ============================================================================
# Helper: collect token types (excluding EOF) from a source string
# ============================================================================

sub types_of {
    my ($source, $version) = @_;
    my $tokens = CodingAdventures::JavaLexer->tokenize($source, $version);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub values_of {
    my ($source, $version) = @_;
    my $tokens = CodingAdventures::JavaLexer->tokenize($source, $version);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'whitespace-only produces only EOF' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize("   \t\r\n  ");
    is( scalar @$tokens, 1,     '1 token after skipping whitespace' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ============================================================================
# Identifiers
# ============================================================================

subtest 'simple identifier' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('myVar');
    is( $tokens->[0]{type},  'NAME',  'type is NAME' );
    is( $tokens->[0]{value}, 'myVar', 'value is myVar' );
};

subtest 'identifier with underscore prefix' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('_priv');
    is( $tokens->[0]{type},  'NAME',  'type is NAME' );
    is( $tokens->[0]{value}, '_priv', 'value is _priv' );
};

# ============================================================================
# Number tokens
# ============================================================================

subtest 'integer number' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('42');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '42',     'value is 42' );
};

subtest 'zero' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('0');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '0',      'value is 0' );
};

# ============================================================================
# String tokens
# ============================================================================

subtest 'double-quoted string' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('"hello"');
    is( $tokens->[0]{type},  'STRING',  'type is STRING' );
};

subtest 'empty double-quoted string' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('""');
    is( $tokens->[0]{type},  'STRING', 'type is STRING' );
};

# ============================================================================
# Punctuation tokens
# ============================================================================

subtest 'parentheses' => sub {
    is( types_of('()'), [qw(LPAREN RPAREN)], 'paren types' );
};

subtest 'braces' => sub {
    is( types_of('{}'), [qw(LBRACE RBRACE)], 'brace types' );
};

subtest 'brackets' => sub {
    is( types_of('[]'), [qw(LBRACKET RBRACKET)], 'bracket types' );
};

subtest 'semicolon' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize(';');
    is( $tokens->[0]{type},  'SEMICOLON', 'type is SEMICOLON' );
    is( $tokens->[0]{value}, ';',         'value is ;' );
};

subtest 'comma' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize(',');
    is( $tokens->[0]{type},  'COMMA', 'type is COMMA' );
    is( $tokens->[0]{value}, ',',     'value is ,' );
};

subtest 'dot' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('.');
    is( $tokens->[0]{type},  'DOT', 'type is DOT' );
    is( $tokens->[0]{value}, '.',   'value is .' );
};

# ============================================================================
# Whitespace handling
# ============================================================================

subtest 'spaces between tokens are consumed silently' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;');
    my @types = map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens;
    ok( !grep({ $_ eq 'WHITESPACE' } @types), 'no WHITESPACE tokens in output' );
};

subtest 'tabs and newlines consumed silently' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize("int\n\tx\n=\n1;");
    my @types = map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens;
    ok( !grep({ $_ eq 'WHITESPACE' } @types), 'only value tokens in output' );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('1');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected character ` raises die' => sub {
    ok(
        dies { CodingAdventures::JavaLexer->tokenize('`') },
        'unexpected ` causes die'
    );
};

# ============================================================================
# Version-aware tokenization
# ============================================================================

subtest 'tokenize with default version (no version specified)' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;');
    ok( scalar @$tokens > 0, 'produced tokens' );
};

subtest 'tokenize with version 1.0' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;', '1.0');
    ok( scalar @$tokens > 0, 'produced tokens' );
};

subtest 'tokenize with version 1.1' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;', '1.1');
    ok( scalar @$tokens > 0, 'produced tokens' );
};

subtest 'tokenize with version 1.4' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;', '1.4');
    ok( scalar @$tokens > 0, 'produced tokens' );
};

subtest 'tokenize with version 5' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;', '5');
    ok( scalar @$tokens > 0, 'produced tokens' );
};

subtest 'tokenize with version 7' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;', '7');
    ok( scalar @$tokens > 0, 'produced tokens' );
};

subtest 'tokenize with version 8' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;', '8');
    ok( scalar @$tokens > 0, 'produced tokens' );
};

subtest 'tokenize with version 10' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;', '10');
    ok( scalar @$tokens > 0, 'produced tokens' );
};

subtest 'tokenize with version 14' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;', '14');
    ok( scalar @$tokens > 0, 'produced tokens' );
};

subtest 'tokenize with version 17' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;', '17');
    ok( scalar @$tokens > 0, 'produced tokens' );
};

subtest 'tokenize with version 21' => sub {
    my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;', '21');
    ok( scalar @$tokens > 0, 'produced tokens' );
};

subtest 'grammar is cached per version' => sub {
    my $t1 = CodingAdventures::JavaLexer->tokenize('int x = 1;', '8');
    my $t2 = CodingAdventures::JavaLexer->tokenize('int x = 1;', '8');
    is( $t1->[0]{type}, $t2->[0]{type}, 'same type from cached grammar' );
};

subtest 'unknown version raises die' => sub {
    ok(
        dies { CodingAdventures::JavaLexer->tokenize('int x = 1;', '99') },
        'unknown version 99 causes die'
    );
};

subtest 'invalid version string is rejected' => sub {
    ok(
        dies { CodingAdventures::JavaLexer->tokenize('int x = 1;', 'java21') },
        'java21 is not a valid Java version'
    );
};

done_testing;
