use strict;
use warnings;
use Test2::V0;

use CodingAdventures::GrammarTools;

# ============================================================================
# Tests for CodingAdventures::GrammarTools
# ============================================================================
#
# Tests cover:
#   1. TokenDefinition: construction and accessors
#   2. TokenGrammar: construction and token_names
#   3. parse_token_grammar: various .tokens file formats
#   4. validate_token_grammar: semantic validation
#   5. is_nullable: nullable non-terminal detection
#   6. compute_first: FIRST set computation
#   7. compute_follow: FOLLOW set computation
#   8. build_parse_table: LL(1) parse table construction
#   9. is_ll1: LL(1) grammar check

# ============================================================================
# TokenDefinition
# ============================================================================

subtest 'TokenDefinition: construction' => sub {
    my $d = CodingAdventures::GrammarTools::TokenDefinition->new(
        name        => 'NUMBER',
        pattern     => '[0-9]+',
        is_regex    => 1,
        line_number => 1,
        alias       => '',
    );
    is($d->name,        'NUMBER',  'name');
    is($d->pattern,     '[0-9]+', 'pattern');
    is($d->is_regex,    1,        'is_regex');
    is($d->line_number, 1,        'line_number');
    is($d->alias,       '',       'alias empty');
};

subtest 'TokenDefinition: defaults' => sub {
    my $d = CodingAdventures::GrammarTools::TokenDefinition->new;
    is($d->name,     '', 'name defaults to empty');
    is($d->alias,    '', 'alias defaults to empty');
    is($d->is_regex, 0,  'is_regex defaults to 0');
};

# ============================================================================
# TokenGrammar
# ============================================================================

subtest 'TokenGrammar: construction' => sub {
    my $g = CodingAdventures::GrammarTools::TokenGrammar->new;
    is(ref($g->definitions), 'ARRAY', 'definitions is arrayref');
    is(ref($g->keywords),    'ARRAY', 'keywords is arrayref');
    is(ref($g->groups),      'HASH',  'groups is hashref');
    is($g->mode,         '', 'mode defaults to empty');
    is($g->escape_mode,  '', 'escape_mode defaults to empty');
};

subtest 'TokenGrammar: token_names includes aliases' => sub {
    my $g = CodingAdventures::GrammarTools::TokenGrammar->new;
    push @{ $g->{definitions} },
        CodingAdventures::GrammarTools::TokenDefinition->new(
            name => 'NUMBER', pattern => '[0-9]+', is_regex => 1,
            line_number => 1, alias => ''
        ),
        CodingAdventures::GrammarTools::TokenDefinition->new(
            name => 'STRING_DQ', pattern => '"[^"]*"', is_regex => 1,
            line_number => 2, alias => 'STRING'
        );
    my $names = $g->token_names;
    ok($names->{NUMBER},    'NUMBER in token_names');
    ok($names->{STRING_DQ}, 'STRING_DQ in token_names');
    ok($names->{STRING},    'alias STRING in token_names');
};

# ============================================================================
# parse_token_grammar: simple definitions
# ============================================================================

subtest 'parse_token_grammar: regex definition' => sub {
    my $src = "NUMBER = /[0-9]+/\n";
    my ($g, $err) = CodingAdventures::GrammarTools->parse_token_grammar($src);
    ok(!$err, 'no error');
    ok(defined $g, 'grammar returned');
    is(scalar @{ $g->definitions }, 1, 'one definition');
    is($g->definitions->[0]->name,     'NUMBER',  'name is NUMBER');
    is($g->definitions->[0]->is_regex, 1,         'is_regex == 1');
    is($g->definitions->[0]->pattern,  '[0-9]+',  'pattern extracted');
};

subtest 'parse_token_grammar: literal definition' => sub {
    my $src = "PLUS = \"+\"\n";
    my ($g, $err) = CodingAdventures::GrammarTools->parse_token_grammar($src);
    ok(!$err, 'no error');
    is($g->definitions->[0]->name,     'PLUS', 'name is PLUS');
    is($g->definitions->[0]->is_regex, 0,      'is_regex == 0');
    is($g->definitions->[0]->pattern,  '+',    'pattern is +');
};

subtest 'parse_token_grammar: alias syntax' => sub {
    my $src = "STRING_DQ = /\"[^\"]*\"/ -> STRING\n";
    my ($g, $err) = CodingAdventures::GrammarTools->parse_token_grammar($src);
    ok(!$err, 'no error');
    is($g->definitions->[0]->alias, 'STRING', 'alias parsed correctly');
};

subtest 'parse_token_grammar: comments and blank lines ignored' => sub {
    my $src = "# This is a comment\n\nNUMBER = /[0-9]+/\n";
    my ($g, $err) = CodingAdventures::GrammarTools->parse_token_grammar($src);
    ok(!$err, 'no error');
    is(scalar @{ $g->definitions }, 1, 'only one definition (comment ignored)');
};

subtest 'parse_token_grammar: mode directive' => sub {
    my $src = "mode: indentation\nNUMBER = /[0-9]+/\n";
    my ($g, $err) = CodingAdventures::GrammarTools->parse_token_grammar($src);
    ok(!$err, 'no error');
    is($g->mode, 'indentation', 'mode parsed');
};

subtest 'parse_token_grammar: layout keywords section' => sub {
    my $src = "mode: layout\nNAME = /[a-z]+/\nlayout_keywords:\n  let\n  where\n  do\n  of\n";
    my ($g, $err) = CodingAdventures::GrammarTools->parse_token_grammar($src);
    ok(!$err, 'no error');
    is($g->mode, 'layout', 'layout mode parsed');
    is($g->layout_keywords, ['let', 'where', 'do', 'of'], 'layout keywords parsed');
};

subtest 'parse_token_grammar: escapes directive' => sub {
    my $src = "escapes: none\nNUMBER = /[0-9]+/\n";
    my ($g, $err) = CodingAdventures::GrammarTools->parse_token_grammar($src);
    ok(!$err, 'no error');
    is($g->escape_mode, 'none', 'escape_mode parsed');
};

subtest 'parse_token_grammar: keywords section' => sub {
    my $src = "NAME = /[a-zA-Z]+/\nkeywords:\n  if\n  else\n";
    my ($g, $err) = CodingAdventures::GrammarTools->parse_token_grammar($src);
    ok(!$err, 'no error');
    is($g->keywords, ['if', 'else'], 'keywords parsed');
};

subtest 'parse_token_grammar: skip section' => sub {
    my $src = "NAME = /[a-zA-Z]+/\nskip:\n  WHITESPACE = /[ \\t]+/\n";
    my ($g, $err) = CodingAdventures::GrammarTools->parse_token_grammar($src);
    ok(!$err, 'no error');
    is(scalar @{ $g->skip_definitions }, 1, 'one skip definition');
    is($g->skip_definitions->[0]->name, 'WHITESPACE', 'skip name is WHITESPACE');
};

subtest 'parse_token_grammar: error on unclosed regex' => sub {
    my $src = "BAD = /unclosed\n";
    my ($g, $err) = CodingAdventures::GrammarTools->parse_token_grammar($src);
    ok(!defined $g, 'no grammar returned on error');
    ok(defined $err, 'error message returned');
    like($err, qr/Unclosed/i, 'error mentions unclosed');
};

# ============================================================================
# validate_token_grammar
# ============================================================================

subtest 'validate_token_grammar: clean grammar has no issues' => sub {
    my $src = "NUMBER = /[0-9]+/\nPLUS = \"+\"\n";
    my ($g, $err) = CodingAdventures::GrammarTools->parse_token_grammar($src);
    ok(!$err);
    my $issues = CodingAdventures::GrammarTools->validate_token_grammar($g);
    is(scalar @$issues, 0, 'no validation issues for clean grammar');
};

subtest 'validate_token_grammar: flags unknown mode' => sub {
    my $src = "NUMBER = /[0-9]+/\n";
    my ($g) = CodingAdventures::GrammarTools->parse_token_grammar($src);
    $g->{mode} = 'unknown_mode';
    my $issues = CodingAdventures::GrammarTools->validate_token_grammar($g);
    ok(scalar @$issues > 0, 'issue reported for unknown mode');
    like($issues->[0], qr/mode/i, 'issue mentions mode');
};

subtest 'validate_token_grammar: layout mode requires layout keywords' => sub {
    my $src = "NUMBER = /[0-9]+/\n";
    my ($g) = CodingAdventures::GrammarTools->parse_token_grammar($src);
    $g->{mode} = 'layout';
    my $issues = CodingAdventures::GrammarTools->validate_token_grammar($g);
    ok(grep(/layout_keywords/, @$issues), 'issue reported for missing layout keywords');
};

# ============================================================================
# LL(1) algorithms — is_nullable
# ============================================================================
#
# Grammar for testing (classic arithmetic with left recursion eliminated):
#
#   E  -> T E'
#   E' -> + T E' | ''
#   T  -> F T'
#   T' -> * F T' | ''
#   F  -> ( E ) | id
#
# Nullable: E', T' (they have epsilon productions)
# Non-nullable: E, T, F, id, +, *, (, )

my $arith_grammar = {
    "E"  => [["T", "E'"]],
    "E'" => [["+", "T", "E'"], [""]],
    "T"  => [["F", "T'"]],
    "T'" => [["*", "F", "T'"], [""]],
    "F"  => [["(", "E", ")"], ["id"]],
};

subtest 'is_nullable: E-prime is nullable (has epsilon production)' => sub {
    is(CodingAdventures::GrammarTools->is_nullable($arith_grammar, "E'"), 1,
        "E' is nullable");
};

subtest 'is_nullable: T-prime is nullable' => sub {
    is(CodingAdventures::GrammarTools->is_nullable($arith_grammar, "T'"), 1,
        "T' is nullable");
};

subtest 'is_nullable: E is NOT nullable' => sub {
    is(CodingAdventures::GrammarTools->is_nullable($arith_grammar, "E"), 0,
        "E is not nullable");
};

subtest 'is_nullable: F is NOT nullable' => sub {
    is(CodingAdventures::GrammarTools->is_nullable($arith_grammar, "F"), 0,
        "F is not nullable");
};

subtest 'is_nullable: terminal id is NOT nullable' => sub {
    is(CodingAdventures::GrammarTools->is_nullable($arith_grammar, "id"), 0,
        "terminal 'id' is not nullable");
};

# ============================================================================
# compute_first
# ============================================================================
#
# Expected FIRST sets for the arithmetic grammar:
#   FIRST(E)  = { (, id }
#   FIRST(E') = { +, '' }
#   FIRST(T)  = { (, id }
#   FIRST(T') = { *, '' }
#   FIRST(F)  = { (, id }

subtest 'compute_first: FIRST(E) = { (, id }' => sub {
    my $first = CodingAdventures::GrammarTools->compute_first($arith_grammar, "E");
    ok($first->{'('}, 'FIRST(E) contains (');
    ok($first->{'id'}, 'FIRST(E) contains id');
    ok(!$first->{'+'}, 'FIRST(E) does not contain +');
    ok(!$first->{''}, 'FIRST(E) does not contain epsilon');
};

subtest "compute_first: FIRST(E') = { +, '' }" => sub {
    my $first = CodingAdventures::GrammarTools->compute_first($arith_grammar, "E'");
    ok($first->{'+'}, "FIRST(E') contains +");
    ok($first->{''}, "FIRST(E') contains epsilon (nullable)");
};

subtest 'compute_first: FIRST(F) = { (, id }' => sub {
    my $first = CodingAdventures::GrammarTools->compute_first($arith_grammar, "F");
    ok($first->{'('}, 'FIRST(F) contains (');
    ok($first->{'id'}, 'FIRST(F) contains id');
};

subtest "compute_first: FIRST(T') = { *, '' }" => sub {
    my $first = CodingAdventures::GrammarTools->compute_first($arith_grammar, "T'");
    ok($first->{'*'}, "FIRST(T') contains *");
    ok($first->{''}, "FIRST(T') contains epsilon");
};

subtest 'compute_first: FIRST(terminal) = { terminal }' => sub {
    my $first = CodingAdventures::GrammarTools->compute_first($arith_grammar, '+');
    ok($first->{'+'}, "FIRST(+) = { + }");
    ok(!$first->{''}, "FIRST(+) does not contain epsilon");
};

# ============================================================================
# compute_follow
# ============================================================================
#
# Expected FOLLOW sets for arithmetic grammar (start = "E"):
#   FOLLOW(E)  = { ), $ }
#   FOLLOW(E') = { ), $ }
#   FOLLOW(T)  = { +, ), $ }
#   FOLLOW(T') = { +, ), $ }
#   FOLLOW(F)  = { *, +, ), $ }

subtest 'compute_follow: FOLLOW(E) contains $ (end of input)' => sub {
    my $follow = CodingAdventures::GrammarTools->compute_follow($arith_grammar, "E");
    ok($follow->{"E"}{'$'}, 'FOLLOW(E) contains $');
};

subtest 'compute_follow: FOLLOW(E) contains )' => sub {
    my $follow = CodingAdventures::GrammarTools->compute_follow($arith_grammar, "E");
    ok($follow->{"E"}{')'}, 'FOLLOW(E) contains )');
};

subtest 'compute_follow: FOLLOW(T) contains + and )' => sub {
    my $follow = CodingAdventures::GrammarTools->compute_follow($arith_grammar, "E");
    ok($follow->{"T"}{'+'}, 'FOLLOW(T) contains +');
    ok($follow->{"T"}{')'}, 'FOLLOW(T) contains )');
};

subtest "compute_follow: FOLLOW(F) contains *" => sub {
    my $follow = CodingAdventures::GrammarTools->compute_follow($arith_grammar, "E");
    ok($follow->{"F"}{'*'}, 'FOLLOW(F) contains *');
};

# ============================================================================
# build_parse_table and is_ll1
# ============================================================================

subtest 'build_parse_table: returns table for each non-terminal' => sub {
    my $table = CodingAdventures::GrammarTools->build_parse_table($arith_grammar, "E");
    for my $nt (qw(E E' T T' F)) {
        ok(exists $table->{$nt}, "table has entry for $nt");
    }
};

subtest 'is_ll1: arithmetic grammar is LL(1)' => sub {
    is(CodingAdventures::GrammarTools->is_ll1($arith_grammar, "E"), 1,
        'arithmetic grammar is LL(1)');
};

subtest 'is_ll1: ambiguous grammar is NOT LL(1)' => sub {
    # This ambiguous grammar has a First/First conflict on E -> E + E | E * E
    # We use a simpler ambiguous example: S -> a | a b
    my $ambig = {
        'S' => [['a'], ['a', 'b']],
    };
    is(CodingAdventures::GrammarTools->is_ll1($ambig, 'S'), 0,
        'ambiguous grammar S -> a | a b is not LL(1)');
};

done_testing;
