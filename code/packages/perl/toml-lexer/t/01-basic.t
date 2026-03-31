use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::TomlLexer; 1 }, 'module loads' );

# ============================================================================
# Helpers
# ============================================================================

# Return token types (excluding EOF and optionally NEWLINE) from a source.
sub types_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::TomlLexer->tokenize($source);
    return [ map { $_->{type} }
             grep { $_->{type} ne 'EOF' }
             @$tokens ];
}

# Return token types ignoring both EOF and NEWLINE tokens.
# Useful for testing value sequences without caring about line endings.
sub types_no_nl {
    my ($source) = @_;
    my $tokens = CodingAdventures::TomlLexer->tokenize($source);
    return [ map { $_->{type} }
             grep { $_->{type} ne 'EOF' && $_->{type} ne 'NEWLINE' }
             @$tokens ];
}

# Return token values (excluding EOF).
sub values_of {
    my ($source) = @_;
    my $tokens = CodingAdventures::TomlLexer->tokenize($source);
    return [ map { $_->{value} }
             grep { $_->{type} ne 'EOF' }
             @$tokens ];
}

# Count tokens of a given type.
sub count_type {
    my ($tokens, $type) = @_;
    return scalar grep { $_->{type} eq $type } @$tokens;
}

# Return the first token of a given type.
sub first_of_type {
    my ($tokens, $type) = @_;
    my ($tok) = grep { $_->{type} eq $type } @$tokens;
    return $tok;
}

# ============================================================================
# Empty / trivial inputs
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'that token is EOF' );
};

subtest 'whitespace-only (spaces and tabs) produces only EOF' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize("   \t   ");
    is( scalar @$tokens, 1,     '1 token after skipping whitespace' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

subtest 'comment-only line produces only EOF' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('# this is a comment');
    is( scalar @$tokens, 1,     '1 token' );
    is( $tokens->[0]{type}, 'EOF', 'token is EOF' );
};

# ============================================================================
# Key-value pairs
# ============================================================================

subtest 'key = "value" produces BARE_KEY EQUALS BASIC_STRING' => sub {
    is(
        types_no_nl('key = "value"'),
        [qw(BARE_KEY EQUALS BASIC_STRING)],
        'token types for simple key-value pair'
    );
};

subtest 'bare key and string values are correct' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('key = "value"');
    is( $tokens->[0]{value}, 'key',     'bare key value' );
    is( $tokens->[1]{value}, '=',       'equals value' );
    is( $tokens->[2]{value}, '"value"', 'string value' );
};

subtest 'dotted key: a.b = 1' => sub {
    is(
        types_no_nl('a.b = 1'),
        [qw(BARE_KEY DOT BARE_KEY EQUALS INTEGER)],
        'dotted key token types'
    );
};

subtest 'multiple key-value pairs across lines' => sub {
    my $src = "name = \"Alice\"\nage = 30\n";
    is(
        types_no_nl($src),
        [qw(BARE_KEY EQUALS BASIC_STRING BARE_KEY EQUALS INTEGER)],
        'multi-line key-value token types'
    );
};

# ============================================================================
# Table headers
# ============================================================================

subtest '[section] header' => sub {
    is(
        types_no_nl('[section]'),
        [qw(LBRACKET BARE_KEY RBRACKET)],
        'table header token types'
    );
};

subtest '[[array-of-tables]] header' => sub {
    is(
        types_no_nl('[[products]]'),
        [qw(LBRACKET LBRACKET BARE_KEY RBRACKET RBRACKET)],
        'array-of-tables header token types'
    );
};

subtest '[a.b] dotted table header' => sub {
    is(
        types_no_nl('[a.b]'),
        [qw(LBRACKET BARE_KEY DOT BARE_KEY RBRACKET)],
        'dotted table header token types'
    );
};

# ============================================================================
# String types
# ============================================================================

subtest 'basic string (double-quoted)' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('"hello"');
    is( $tokens->[0]{type},  'BASIC_STRING', 'type is BASIC_STRING' );
    is( $tokens->[0]{value}, '"hello"',      'value preserved with quotes' );
};

subtest 'empty basic string' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('""');
    is( $tokens->[0]{type},  'BASIC_STRING', 'type is BASIC_STRING' );
    is( $tokens->[0]{value}, '""',           'empty string value' );
};

subtest 'basic string with escape sequence' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('"a\\nb"');
    is( $tokens->[0]{type}, 'BASIC_STRING', 'type is BASIC_STRING' );
    like( $tokens->[0]{value}, qr/\\n/, 'escape sequence preserved' );
};

subtest 'literal string (single-quoted)' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize("'hello'");
    is( $tokens->[0]{type},  'LITERAL_STRING', 'type is LITERAL_STRING' );
    is( $tokens->[0]{value}, "'hello'",         'value preserved' );
};

subtest 'multi-line basic string' => sub {
    my $src = "\"\"\"hello\nworld\"\"\"";
    my $tokens = CodingAdventures::TomlLexer->tokenize($src);
    is( $tokens->[0]{type},  'ML_BASIC_STRING', 'type is ML_BASIC_STRING' );
    is( $tokens->[0]{value}, $src,              'full value preserved' );
};

subtest 'multi-line literal string' => sub {
    my $src = "'''hello\nworld'''";
    my $tokens = CodingAdventures::TomlLexer->tokenize($src);
    is( $tokens->[0]{type},  'ML_LITERAL_STRING', 'type is ML_LITERAL_STRING' );
    is( $tokens->[0]{value}, $src,                'full value preserved' );
};

# ============================================================================
# Integer literals
# ============================================================================

subtest 'decimal integer' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('42');
    is( $tokens->[0]{type},  'INTEGER', 'type is INTEGER' );
    is( $tokens->[0]{value}, '42',      'value is 42' );
};

subtest 'negative integer' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('-17');
    is( $tokens->[0]{type},  'INTEGER', 'type is INTEGER' );
    is( $tokens->[0]{value}, '-17',     'value is -17' );
};

subtest 'positive integer with sign' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('+99');
    is( $tokens->[0]{type},  'INTEGER', 'type is INTEGER' );
    is( $tokens->[0]{value}, '+99',     'value is +99' );
};

subtest 'hex integer aliased to INTEGER' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('0xFF');
    is( $tokens->[0]{type},  'INTEGER', 'type is INTEGER (aliased from HEX_INTEGER)' );
    is( $tokens->[0]{value}, '0xFF',    'value is 0xFF' );
};

subtest 'octal integer aliased to INTEGER' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('0o755');
    is( $tokens->[0]{type},  'INTEGER', 'type is INTEGER (aliased from OCT_INTEGER)' );
    is( $tokens->[0]{value}, '0o755',   'value is 0o755' );
};

subtest 'binary integer aliased to INTEGER' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('0b1010');
    is( $tokens->[0]{type},  'INTEGER', 'type is INTEGER (aliased from BIN_INTEGER)' );
    is( $tokens->[0]{value}, '0b1010',  'value is 0b1010' );
};

subtest 'underscore-separated integer' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('1_000_000');
    is( $tokens->[0]{type},  'INTEGER',    'type is INTEGER' );
    is( $tokens->[0]{value}, '1_000_000', 'value is 1_000_000' );
};

# ============================================================================
# Float literals
# ============================================================================

subtest 'decimal float' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('3.14');
    is( $tokens->[0]{type},  'FLOAT', 'type is FLOAT' );
    is( $tokens->[0]{value}, '3.14',  'value is 3.14' );
};

subtest 'negative float' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('-0.5');
    is( $tokens->[0]{type},  'FLOAT', 'type is FLOAT' );
    is( $tokens->[0]{value}, '-0.5',  'value is -0.5' );
};

subtest 'scientific notation float' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('5e22');
    is( $tokens->[0]{type}, 'FLOAT', 'type is FLOAT' );
};

subtest 'positive infinity' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('inf');
    is( $tokens->[0]{type},  'FLOAT', 'type is FLOAT' );
    is( $tokens->[0]{value}, 'inf',   'value is inf' );
};

subtest 'negative infinity' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('-inf');
    is( $tokens->[0]{type},  'FLOAT', 'type is FLOAT' );
    is( $tokens->[0]{value}, '-inf',  'value is -inf' );
};

subtest 'not-a-number' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('nan');
    is( $tokens->[0]{type},  'FLOAT', 'type is FLOAT' );
    is( $tokens->[0]{value}, 'nan',   'value is nan' );
};

# ============================================================================
# Boolean literals
# ============================================================================

subtest 'true literal' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('true');
    is( $tokens->[0]{type},  'TRUE', 'type is TRUE' );
    is( $tokens->[0]{value}, 'true', 'value is true' );
};

subtest 'false literal' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('false');
    is( $tokens->[0]{type},  'FALSE', 'type is FALSE' );
    is( $tokens->[0]{value}, 'false', 'value is false' );
};

# ============================================================================
# Date/time literals
# ============================================================================

subtest 'offset datetime with Z' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('1979-05-27T07:32:00Z');
    is( $tokens->[0]{type},  'OFFSET_DATETIME',       'type is OFFSET_DATETIME' );
    is( $tokens->[0]{value}, '1979-05-27T07:32:00Z',  'value preserved' );
};

subtest 'offset datetime with timezone offset' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('1979-05-27T00:32:00+09:00');
    is( $tokens->[0]{type}, 'OFFSET_DATETIME', 'type is OFFSET_DATETIME' );
};

subtest 'local datetime' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('1979-05-27T07:32:00');
    is( $tokens->[0]{type},  'LOCAL_DATETIME',      'type is LOCAL_DATETIME' );
    is( $tokens->[0]{value}, '1979-05-27T07:32:00', 'value preserved' );
};

subtest 'local date' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('1979-05-27');
    is( $tokens->[0]{type},  'LOCAL_DATE',  'type is LOCAL_DATE' );
    is( $tokens->[0]{value}, '1979-05-27',  'value preserved' );
};

subtest 'local time' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('07:32:00');
    is( $tokens->[0]{type},  'LOCAL_TIME', 'type is LOCAL_TIME' );
    is( $tokens->[0]{value}, '07:32:00',   'value preserved' );
};

subtest 'local time with fractional seconds' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('07:32:00.999');
    is( $tokens->[0]{type},  'LOCAL_TIME',   'type is LOCAL_TIME' );
    is( $tokens->[0]{value}, '07:32:00.999', 'value preserved' );
};

# ============================================================================
# Inline tables
# ============================================================================

subtest 'inline table { key = "val" }' => sub {
    is(
        types_no_nl('{ key = "val" }'),
        [qw(LBRACE BARE_KEY EQUALS BASIC_STRING RBRACE)],
        'inline table token types'
    );
};

subtest 'multi-key inline table' => sub {
    is(
        types_no_nl('{ x = 1, y = 2 }'),
        [qw(LBRACE BARE_KEY EQUALS INTEGER COMMA BARE_KEY EQUALS INTEGER RBRACE)],
        'multi-key inline table token types'
    );
};

# ============================================================================
# Arrays
# ============================================================================

subtest 'array [1, 2, 3]' => sub {
    is(
        types_no_nl('[1, 2, 3]'),
        [qw(LBRACKET INTEGER COMMA INTEGER COMMA INTEGER RBRACKET)],
        'integer array token types'
    );
};

subtest 'array of strings' => sub {
    is(
        types_no_nl('["a", "b"]'),
        [qw(LBRACKET BASIC_STRING COMMA BASIC_STRING RBRACKET)],
        'string array token types'
    );
};

subtest 'empty array []' => sub {
    is(
        types_no_nl('[]'),
        [qw(LBRACKET RBRACKET)],
        'empty array token types'
    );
};

# ============================================================================
# Whitespace and comment handling
# ============================================================================

subtest 'spaces and tabs between tokens stripped' => sub {
    is(
        types_no_nl('key  =  "val"'),
        [qw(BARE_KEY EQUALS BASIC_STRING)],
        'no whitespace tokens in output'
    );
};

subtest 'comment after value consumed silently' => sub {
    is(
        types_no_nl('key = 42 # the answer'),
        [qw(BARE_KEY EQUALS INTEGER)],
        'comment stripped, value tokens remain'
    );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking on single line' => sub {
    # Input: key=1
    # col:   12345
    my $tokens = CodingAdventures::TomlLexer->tokenize('key=1');
    is( $tokens->[0]{col}, 1, 'key at col 1' );
    is( $tokens->[1]{col}, 4, '= at col 4' );
    is( $tokens->[2]{col}, 5, '1 at col 5' );
};

subtest 'all tokens on line 1 for single-line input' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('key = "val"');
    for my $tok (@$tokens) {
        is( $tok->{line}, 1, "$tok->{type} is on line 1" );
    }
};

# ============================================================================
# Composite TOML structures
# ============================================================================

subtest 'full TOML document' => sub {
    my $src = "[server]\nhost = \"localhost\"\nport = 8080\ndebug = true\n";
    my $tokens = CodingAdventures::TomlLexer->tokenize($src);

    ok( scalar(@$tokens) > 10, 'many tokens produced' );

    my $first_key = first_of_type($tokens, 'BARE_KEY');
    is( $first_key->{value}, 'server', 'first BARE_KEY is server' );

    ok( count_type($tokens, 'BARE_KEY') >= 4,  'at least 4 BARE_KEY tokens' );
    ok( count_type($tokens, 'EQUALS')   >= 3,  'at least 3 EQUALS tokens' );

    my $t_tok = first_of_type($tokens, 'TRUE');
    ok( defined $t_tok, 'TRUE token present' );

    is( $tokens->[-1]{type}, 'EOF', 'last token is EOF' );
};

subtest 'integer, float, boolean sequence' => sub {
    is(
        types_no_nl('42 3.14 true false'),
        [qw(INTEGER FLOAT TRUE FALSE)],
        'mixed literal token types'
    );
};

# ============================================================================
# EOF token
# ============================================================================

subtest 'EOF is always last' => sub {
    my $tokens = CodingAdventures::TomlLexer->tokenize('key = 1');
    is( $tokens->[-1]{type},  'EOF', 'last token is EOF' );
    is( $tokens->[-1]{value}, '',    'EOF has empty value' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unexpected character raises die' => sub {
    ok(
        dies { CodingAdventures::TomlLexer->tokenize('`invalid`') },
        'unexpected backtick causes die'
    );
};

done_testing;
