use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::Lexer; 1 }, 'module loads');

CodingAdventures::Lexer->import(qw(expression_rules default_rules));

# ============================================================================
# Construction
# ============================================================================

subtest 'new with default rules' => sub {
    my $lex = CodingAdventures::Lexer->new("x = 42");
    ok(defined $lex, 'lexer created');
    ok($lex->isa('CodingAdventures::Lexer'), 'is a Lexer');
};

subtest 'new with empty source' => sub {
    my $lex = CodingAdventures::Lexer->new('');
    my $tok = $lex->next_token();
    is($tok->{type}, 'EOF', 'empty source gives EOF');
};

# ============================================================================
# Single token types
# ============================================================================

subtest 'number tokens' => sub {
    my $lex = CodingAdventures::Lexer->new('42 3.14');
    my $t1  = $lex->next_token();
    is($t1->{type},  'NUMBER', 'integer type');
    is($t1->{value}, '42',     'integer value');
    $lex->next_token();  # whitespace
    my $t2 = $lex->next_token();
    is($t2->{type},  'NUMBER', 'float type');
    is($t2->{value}, '3.14',   'float value');
};

subtest 'string tokens' => sub {
    my $lex = CodingAdventures::Lexer->new('"hello world"');
    my $t   = $lex->next_token();
    is($t->{type},  'STRING',        'string type');
    is($t->{value}, '"hello world"', 'string value');
};

subtest 'identifier tokens' => sub {
    my $lex = CodingAdventures::Lexer->new('myVar _foo x123');
    my @toks = grep { $_->{type} ne 'WHITESPACE' } $lex->tokenize();
    pop @toks;  # remove EOF
    is(scalar @toks, 3, '3 identifiers');
    is($toks[0]{type}, 'IDENT', 'first is IDENT');
};

subtest 'keyword tokens' => sub {
    my $lex = CodingAdventures::Lexer->new('if while let');
    my @toks = grep { $_->{type} ne 'WHITESPACE' } $lex->tokenize();
    pop @toks;
    is($toks[0]{type},  'KEYWORD', 'if is keyword');
    is($toks[0]{value}, 'if',      'if value');
    is($toks[1]{value}, 'while',   'while keyword');
    is($toks[2]{value}, 'let',     'let keyword');
};

subtest 'symbol tokens' => sub {
    my $lex  = CodingAdventures::Lexer->new('+ - * / = == ( )');
    my @toks = grep { $_->{type} ne 'WHITESPACE' } $lex->tokenize();
    pop @toks;
    my @vals = map { $_->{value} } @toks;
    ok((grep { $_ eq '+' } @vals),  'has +');
    ok((grep { $_ eq '==' } @vals), 'has ==');
    ok((grep { $_ eq '(' } @vals),  'has (');
};

subtest 'newline token' => sub {
    my $lex = CodingAdventures::Lexer->new("a\nb");
    my @toks = $lex->tokenize();
    my @newlines = grep { $_->{type} eq 'NEWLINE' } @toks;
    is(scalar @newlines, 1, 'one newline token');
};

# ============================================================================
# Line and column tracking
# ============================================================================

subtest 'line and column tracking' => sub {
    my $lex = CodingAdventures::Lexer->new("abc\ndef");
    my $t1  = $lex->next_token();  # "abc"
    is($t1->{line}, 1, 'first token line 1');
    is($t1->{col},  1, 'first token col 1');

    $lex->next_token();  # newline
    my $t3 = $lex->next_token();  # "def"
    is($t3->{line}, 2, 'def on line 2');
};

# ============================================================================
# tokenize() method
# ============================================================================

subtest 'tokenize returns all tokens' => sub {
    my $lex  = CodingAdventures::Lexer->new("x = 42");
    my @toks = $lex->tokenize();
    ok(scalar(@toks) > 0, 'returns tokens');
    is($toks[-1]{type}, 'EOF', 'last token is EOF');
};

subtest 'tokenize_string class method' => sub {
    my @toks = CodingAdventures::Lexer->tokenize_string("1 + 2");
    ok(scalar(@toks) > 0, 'tokenize_string works');
    is($toks[-1]{type}, 'EOF', 'ends with EOF');
};

# ============================================================================
# expression_rules and default_rules
# ============================================================================

subtest 'expression_rules returns arrayref' => sub {
    my $rules = expression_rules();
    ok(ref($rules) eq 'ARRAY', 'expression_rules returns ARRAY');
    ok(scalar(@$rules) > 0, 'rules are non-empty');
    is(ref($rules->[0]), 'HASH', 'first rule is HASH');
    ok(exists $rules->[0]{type},    'rule has type');
    ok(exists $rules->[0]{pattern}, 'rule has pattern');
};

subtest 'default_rules same as expression_rules' => sub {
    my $dr = default_rules();
    my $er = expression_rules();
    is(scalar @$dr, scalar @$er, 'same number of rules');
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'error token for unrecognized chars' => sub {
    my $lex = CodingAdventures::Lexer->new('@');
    my $tok;
    my $found_error = 0;
    my $found_eof   = 0;
    for (1..3) {
        $tok = $lex->next_token();
        $found_error = 1 if $tok->{type} eq 'ERROR';
        $found_eof   = 1 if $tok->{type} eq 'EOF';
    }
    ok($found_error || $found_eof, 'unrecognized char handled');
};

# ============================================================================
# Complex expression
# ============================================================================

subtest 'complex expression tokenizes correctly' => sub {
    my $src  = 'let x = (42 + y) * 3';
    my $lex  = CodingAdventures::Lexer->new($src);
    my @toks = grep { $_->{type} ne 'WHITESPACE' } $lex->tokenize();
    pop @toks;  # remove EOF

    is($toks[0]{type},  'KEYWORD', 'let is keyword');
    is($toks[1]{type},  'IDENT',   'x is ident');
    is($toks[2]{value}, '=',       '= symbol');
    is($toks[3]{value}, '(',       '( symbol');
    is($toks[4]{value}, '42',      '42 number');
};

# ============================================================================
# Multiple EOF calls
# ============================================================================

subtest 'multiple EOF calls' => sub {
    my $lex = CodingAdventures::Lexer->new('');
    my $t1  = $lex->next_token();
    my $t2  = $lex->next_token();
    is($t1->{type}, 'EOF', 'first call is EOF');
    is($t2->{type}, 'EOF', 'second call is also EOF');
};

done_testing;
