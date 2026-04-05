use strict;
use warnings;
use Test2::V0;

use CodingAdventures::MosaicLexer;

# ============================================================================
# Helpers
# ============================================================================

sub types_of {
    my ($source) = @_;
    my ($toks, $err) = CodingAdventures::MosaicLexer->tokenize($source);
    die "tokenize error: $err" if $err;
    return [ map { $_->{type} } grep { $_->{type} ne 'EOF' } @$toks ];
}

sub values_of {
    my ($source) = @_;
    my ($toks, $err) = CodingAdventures::MosaicLexer->tokenize($source);
    die "tokenize error: $err" if $err;
    return [ map { $_->{value} } grep { $_->{type} ne 'EOF' } @$toks ];
}

# ============================================================================
# Empty / whitespace
# ============================================================================

subtest 'empty string produces only EOF' => sub {
    my ($toks, $err) = CodingAdventures::MosaicLexer->tokenize('');
    is($err, undef, 'no error');
    is(scalar @$toks, 1, '1 token');
    is($toks->[0]{type}, 'EOF', 'token is EOF');
};

subtest 'whitespace-only produces only EOF' => sub {
    my ($toks, $err) = CodingAdventures::MosaicLexer->tokenize("   \t\r\n  ");
    is($err, undef, 'no error');
    is(scalar @$toks, 1, '1 token after skipping whitespace');
    is($toks->[0]{type}, 'EOF', 'token is EOF');
};

# ============================================================================
# Comments
# ============================================================================

subtest 'line comment is skipped' => sub {
    is(types_of("// this is a comment\nfoo"), ['NAME'], 'only NAME after comment');
};

subtest 'block comment is skipped' => sub {
    is(types_of("/* block */ foo"), ['NAME'], 'only NAME after block comment');
};

# ============================================================================
# Keywords
# ============================================================================

subtest 'component keyword' => sub {
    my ($toks) = CodingAdventures::MosaicLexer->tokenize('component');
    is($toks->[0]{type},  'KEYWORD',    'type is KEYWORD');
    is($toks->[0]{value}, 'component',  'value is component');
};

subtest 'all structural keywords' => sub {
    is(
        types_of('component slot when each as'),
        [qw(KEYWORD KEYWORD KEYWORD KEYWORD KEYWORD)],
        'structural keywords recognized'
    );
};

subtest 'type keywords' => sub {
    is(
        types_of('text number bool image color node list'),
        [qw(KEYWORD KEYWORD KEYWORD KEYWORD KEYWORD KEYWORD KEYWORD)],
        'type keywords recognized'
    );
};

subtest 'true and false are keywords' => sub {
    my @types = @{ types_of('true false') };
    is($types[0], 'KEYWORD', 'true is KEYWORD');
    is($types[1], 'KEYWORD', 'false is KEYWORD');
};

# ============================================================================
# Identifiers / NAME
# ============================================================================

subtest 'simple name' => sub {
    my ($toks) = CodingAdventures::MosaicLexer->tokenize('ProfileCard');
    is($toks->[0]{type},  'NAME',        'type is NAME');
    is($toks->[0]{value}, 'ProfileCard', 'value matches');
};

subtest 'hyphenated name' => sub {
    my ($toks) = CodingAdventures::MosaicLexer->tokenize('corner-radius');
    is($toks->[0]{type},  'NAME',          'type is NAME');
    is($toks->[0]{value}, 'corner-radius', 'hyphen preserved');
};

subtest 'underscore name' => sub {
    my ($toks) = CodingAdventures::MosaicLexer->tokenize('_private');
    is($toks->[0]{type},  'NAME',     'type is NAME');
    is($toks->[0]{value}, '_private', 'underscore preserved');
};

# ============================================================================
# String literals
# ============================================================================

subtest 'simple string' => sub {
    my ($toks) = CodingAdventures::MosaicLexer->tokenize('"hello"');
    is($toks->[0]{type},  'STRING',  'type is STRING');
    is($toks->[0]{value}, '"hello"', 'value with quotes');
};

subtest 'string with escape' => sub {
    my ($toks) = CodingAdventures::MosaicLexer->tokenize('"a\\nb"');
    is($toks->[0]{type}, 'STRING', 'type is STRING');
    like($toks->[0]{value}, qr/\\n/, 'escape sequence preserved');
};

# ============================================================================
# HEX_COLOR
# ============================================================================

subtest 'three-digit hex color' => sub {
    my ($toks) = CodingAdventures::MosaicLexer->tokenize('#fff');
    is($toks->[0]{type},  'HEX_COLOR', 'type is HEX_COLOR');
    is($toks->[0]{value}, '#fff',      'value matches');
};

subtest 'six-digit hex color' => sub {
    my ($toks) = CodingAdventures::MosaicLexer->tokenize('#2563eb');
    is($toks->[0]{type},  'HEX_COLOR', 'type is HEX_COLOR');
    is($toks->[0]{value}, '#2563eb',   'value matches');
};

subtest 'eight-digit hex color (with alpha)' => sub {
    my ($toks) = CodingAdventures::MosaicLexer->tokenize('#2563ebff');
    is($toks->[0]{type},  'HEX_COLOR',  'type is HEX_COLOR');
    is($toks->[0]{value}, '#2563ebff',  'value matches');
};

# ============================================================================
# DIMENSION and NUMBER
# ============================================================================

subtest 'dimension dp' => sub {
    my ($toks) = CodingAdventures::MosaicLexer->tokenize('16dp');
    is($toks->[0]{type},  'DIMENSION', 'type is DIMENSION');
    is($toks->[0]{value}, '16dp',      'value matches');
};

subtest 'dimension percent' => sub {
    my ($toks) = CodingAdventures::MosaicLexer->tokenize('50%');
    is($toks->[0]{type},  'DIMENSION', 'type is DIMENSION');
    is($toks->[0]{value}, '50%',       'value matches');
};

subtest 'plain number' => sub {
    my ($toks) = CodingAdventures::MosaicLexer->tokenize('42');
    is($toks->[0]{type},  'NUMBER', 'type is NUMBER');
    is($toks->[0]{value}, '42',     'value matches');
};

subtest 'negative number' => sub {
    my ($toks) = CodingAdventures::MosaicLexer->tokenize('-1.5');
    is($toks->[0]{type},  'NUMBER', 'type is NUMBER');
    is($toks->[0]{value}, '-1.5',   'value matches');
};

# ============================================================================
# Punctuation
# ============================================================================

subtest 'all punctuation tokens' => sub {
    is(
        types_of('{ } < > : ; @ , . ='),
        [qw(LBRACE RBRACE LANGLE RANGLE COLON SEMICOLON AT COMMA DOT EQUALS)],
        'all punctuation tokens recognized'
    );
};

# ============================================================================
# Position tracking
# ============================================================================

subtest 'column tracking' => sub {
    # 'foo bar' — foo at col 1, bar at col 5
    my ($toks) = CodingAdventures::MosaicLexer->tokenize('foo bar');
    is($toks->[0]{col}, 1, 'foo at col 1');
    is($toks->[1]{col}, 5, 'bar at col 5');
};

subtest 'line tracking' => sub {
    my ($toks) = CodingAdventures::MosaicLexer->tokenize("foo\nbar");
    is($toks->[0]{line}, 1, 'foo on line 1');
    is($toks->[1]{line}, 2, 'bar on line 2');
};

# ============================================================================
# A realistic Mosaic snippet
# ============================================================================

subtest 'realistic snippet' => sub {
    my $src = <<'END';
component Card {
  slot title: text;
  slot count: number = 0;
  Column {
    Text { content: @title; }
  }
}
END
    my ($toks, $err) = CodingAdventures::MosaicLexer->tokenize($src);
    is($err, undef, 'no error');
    ok(scalar(@$toks) > 20, 'many tokens produced');
    is($toks->[-1]{type}, 'EOF', 'last token is EOF');

    my @keywords = grep { $_->{type} eq 'KEYWORD' } @$toks;
    ok(scalar @keywords >= 5, 'several keywords recognized');

    my @names = grep { $_->{type} eq 'NAME' } @$toks;
    my $has_card = grep { $_->{value} eq 'Card' } @names;
    ok($has_card, 'Card name found');
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unknown character returns error' => sub {
    my ($toks, $err) = CodingAdventures::MosaicLexer->tokenize('~');
    is($toks, undef, 'no tokens returned');
    ok(defined $err, 'error string returned');
    like($err, qr/unexpected/, 'error mentions unexpected');
};

done_testing;
