use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::JsonLexer; 1 }, 'module loads' );

# ============================================================================
# Helper: collect token types (excluding EOF) from a source string
# ============================================================================

sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::JsonLexer->tokenize($source);
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

sub values_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::JsonLexer->tokenize($source);
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$tokens ];
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'whitespace-only produces only EOF' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize("   \t\r\n  ");
    is( scalar @$tokens, 1,     '1 token after skipping whitespace' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ============================================================================
# Structural tokens
# ============================================================================

subtest 'empty object {}' => sub {
    is( types_of('{}'), [qw(LBRACE RBRACE)], 'types match' );
    is( values_of('{}'), ['{', '}'],          'values match' );
};

subtest 'empty array []' => sub {
    is( types_of('[]'), [qw(LBRACKET RBRACKET)], 'types match' );
};

subtest 'all structural tokens' => sub {
    is( types_of('{}[]:,'),
        [qw(LBRACE RBRACE LBRACKET RBRACKET COLON COMMA)],
        'all six structural tokens in order' );
};

# ============================================================================
# String tokens
# ============================================================================

subtest 'simple string' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize('"hello"');
    is( $tokens->[0]{type},  'STRING',  'type is STRING' );
    is( $tokens->[0]{value}, '"hello"', 'value preserved with quotes' );
};

subtest 'empty string' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize('""');
    is( $tokens->[0]{type},  'STRING', 'type is STRING' );
    is( $tokens->[0]{value}, '""',     'empty string value' );
};

subtest 'string with backslash-n escape' => sub {
    # The lexer returns the raw source text — escape sequences are NOT decoded.
    my $tokens = CodingAdventures::JsonLexer->tokenize('"a\\nb"');
    is( $tokens->[0]{type}, 'STRING', 'type is STRING' );
    like( $tokens->[0]{value}, qr/\\n/, 'value contains the escape sequence' );
};

subtest 'string with unicode escape' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize('"\\u0041"');
    is( $tokens->[0]{type}, 'STRING', 'type is STRING' );
    like( $tokens->[0]{value}, qr/\\u0041/, 'unicode escape preserved' );
};

# ============================================================================
# Number tokens
# ============================================================================

subtest 'positive integer' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize('42');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '42',     'value is 42' );
};

subtest 'zero' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize('0');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '0',      'value is 0' );
};

subtest 'negative integer' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize('-7');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '-7',     'value is -7' );
};

subtest 'floating point' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize('3.14');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '3.14',   'value is 3.14' );
};

subtest 'scientific notation e+' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize('1e10');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '1e10',   'value is 1e10' );
};

subtest 'scientific notation E-' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize('2.5E-3');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '2.5E-3', 'value is 2.5E-3' );
};

subtest 'negative float' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize('-0.5');
    is( $tokens->[0]{type},  'NUMBER', 'type is NUMBER' );
    is( $tokens->[0]{value}, '-0.5',   'value is -0.5' );
};

# ============================================================================
# Boolean and null literals
# ============================================================================

subtest 'true literal' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize('true');
    is( $tokens->[0]{type},  'TRUE', 'type is TRUE' );
    is( $tokens->[0]{value}, 'true', 'value is true' );
};

subtest 'false literal' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize('false');
    is( $tokens->[0]{type},  'FALSE', 'type is FALSE' );
    is( $tokens->[0]{value}, 'false', 'value is false' );
};

subtest 'null literal' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize('null');
    is( $tokens->[0]{type},  'NULL', 'type is NULL' );
    is( $tokens->[0]{value}, 'null', 'value is null' );
};

# ============================================================================
# Whitespace handling
# ============================================================================

subtest 'spaces between tokens are consumed silently' => sub {
    is(
        types_of('{ "k" : 1 }'),
        [qw(LBRACE STRING COLON NUMBER RBRACE)],
        'no WHITESPACE tokens in output'
    );
};

subtest 'tabs and newlines consumed silently' => sub {
    is(
        types_of("[\n\t1,\n\t2\n]"),
        [qw(LBRACKET NUMBER COMMA NUMBER RBRACKET)],
        'only value tokens in output'
    );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking on single line' => sub {
    # Input: {"k":1}
    # col:   1234567
    my $tokens = CodingAdventures::JsonLexer->tokenize('{"k":1}');
    is( $tokens->[0]{col}, 1, '{ at col 1' );
    is( $tokens->[1]{col}, 2, '"k" at col 2' );
    is( $tokens->[2]{col}, 5, ': at col 5' );
    is( $tokens->[3]{col}, 6, '1 at col 6' );
    is( $tokens->[4]{col}, 7, '} at col 7' );
};

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize('{"a":1}');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# Composite structures
# ============================================================================

subtest 'simple key-value object' => sub {
    is(
        types_of('{"key": 42}'),
        [qw(LBRACE STRING COLON NUMBER RBRACE)],
        'key-value object token types'
    );
};

subtest 'array of mixed values' => sub {
    is(
        types_of('[1, "two", true, null]'),
        [qw(LBRACKET NUMBER COMMA STRING COMMA TRUE COMMA NULL RBRACKET)],
        'mixed array token types'
    );
};

subtest 'nested object' => sub {
    is(
        types_of('{"a": {"b": 2}}'),
        [qw(LBRACE STRING COLON LBRACE STRING COLON NUMBER RBRACE RBRACE)],
        'nested object token types'
    );
};

subtest 'nested array' => sub {
    is(
        types_of('[[1,2],[3]]'),
        [qw(LBRACKET LBRACKET NUMBER COMMA NUMBER RBRACKET COMMA
            LBRACKET NUMBER RBRACKET RBRACKET)],
        'nested array token types'
    );
};

subtest 'real-world JSON blob' => sub {
    my $src = <<'END_JSON';
{
  "name": "Alice",
  "age": 30,
  "active": true,
  "score": -1.5e2,
  "tags": ["lua", "json"],
  "meta": null
}
END_JSON

    my $tokens = CodingAdventures::JsonLexer->tokenize($src);

    ok( scalar(@$tokens) > 20, 'many tokens produced' );

    # Last real token before EOF is RBRACE
    my $last = $tokens->[-2];
    is( $last->{type}, 'RBRACE', 'last substantive token is RBRACE' );

    # EOF is the final token
    is( $tokens->[-1]{type}, 'EOF', 'final token is EOF' );

    # Spot-check: first STRING is "name"
    my ($first_str) = grep { $_->{type} eq 'STRING' } @$tokens;
    is( $first_str->{value}, '"name"', 'first string is "name"' );

    # Should have TRUE and NULL
    my @trues = grep { $_->{type} eq 'TRUE' } @$tokens;
    ok( scalar @trues >= 1, 'has TRUE token' );

    my @nulls = grep { $_->{type} eq 'NULL' } @$tokens;
    ok( scalar @nulls >= 1, 'has NULL token' );
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::JsonLexer->tokenize('1');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected character raises die' => sub {
    ok(
        dies { CodingAdventures::JsonLexer->tokenize('@') },
        'unexpected @ causes die'
    );
};

subtest 'bare identifier (not true/false/null) raises die' => sub {
    ok(
        dies { CodingAdventures::JsonLexer->tokenize('undefined') },
        'bare identifier causes die'
    );
};

done_testing;
