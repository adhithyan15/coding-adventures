use strict;
use warnings;
use Test2::V0;

use CodingAdventures::GrammarTools;
use CodingAdventures::GrammarTools::Compiler;

# ============================================================================
# Tests for CodingAdventures::GrammarTools::Compiler
# ============================================================================
#
# compile_token_grammar / compile_parser_grammar generate Perl source code
# that reconstructs a TokenGrammar / ParserGrammar object as a literal data
# structure. These tests round-trip a parsed grammar through the compiler
# and `eval` the generated code, then assert the reconstructed object
# matches the original.

# ----------------------------------------------------------------------------
# Helper: eval a compiled `sub token_grammar { ... }` / `sub parser_grammar`
# body and call it, returning the resulting object.
# ----------------------------------------------------------------------------
my $_eval_counter = 0;

sub eval_token_grammar {
    my ($code) = @_;
    my $pkg = "CodingAdventures::GrammarTools::Compiler::_Test" . $_eval_counter++;
    my $wrapped = "package $pkg;\n"
        . "no strict 'refs';\n"
        . $code
        . "\n1;\n";
    my $ok = eval $wrapped;
    die "eval failed: $@" unless $ok;
    no strict 'refs';
    return &{"${pkg}::token_grammar"}();
}

sub eval_parser_grammar {
    my ($code) = @_;
    my $pkg = "CodingAdventures::GrammarTools::Compiler::_Test" . $_eval_counter++;
    my $wrapped = "package $pkg;\n"
        . "no strict 'refs';\n"
        . $code
        . "\n1;\n";
    my $ok = eval $wrapped;
    die "eval failed: $@" unless $ok;
    no strict 'refs';
    return &{"${pkg}::parser_grammar"}();
}

# ============================================================================
# compile_token_grammar
# ============================================================================

subtest 'compile_token_grammar: output structure' => sub {
    my ($grammar) = CodingAdventures::GrammarTools->parse_token_grammar("NUMBER = /[0-9]+/\n");
    my $code = CodingAdventures::GrammarTools::Compiler::compile_token_grammar($grammar, 'x.tokens');
    like($code, qr/AUTO-GENERATED FILE/, 'has DO NOT EDIT header');
    like($code, qr/Source: x\.tokens/, 'has source comment');
    like($code, qr/sub token_grammar/, 'defines token_grammar sub');
};

subtest 'compile_token_grammar: round-trips a simple definition' => sub {
    my ($original) = CodingAdventures::GrammarTools->parse_token_grammar("NUMBER = /[0-9]+/\n");
    my $code = CodingAdventures::GrammarTools::Compiler::compile_token_grammar($original);
    my $loaded = eval_token_grammar($code);

    is(ref($loaded), 'CodingAdventures::GrammarTools::TokenGrammar', 'reblessed correctly');
    is(scalar(@{ $loaded->definitions }), 1, 'one definition');
    is($loaded->definitions->[0]->name, 'NUMBER', 'name preserved');
    is($loaded->definitions->[0]->pattern, '[0-9]+', 'pattern preserved');
    is($loaded->definitions->[0]->is_regex, 1, 'is_regex preserved');
};

subtest 'compile_token_grammar: literal pattern with backslash round-trips' => sub {
    my ($original) = CodingAdventures::GrammarTools->parse_token_grammar('STRING = /"([^"\\\\]|\\\\.)*"/' . "\n");
    my $code = CodingAdventures::GrammarTools::Compiler::compile_token_grammar($original);
    my $loaded = eval_token_grammar($code);
    is($loaded->definitions->[0]->pattern, $original->definitions->[0]->pattern, 'backslash-heavy pattern preserved exactly');
};

subtest 'compile_token_grammar: keywords round-trip' => sub {
    my ($original) = CodingAdventures::GrammarTools->parse_token_grammar(
        "NAME = /[a-z]+/\nkeywords:\n  if\n  else\n"
    );
    my $code = CodingAdventures::GrammarTools::Compiler::compile_token_grammar($original);
    my $loaded = eval_token_grammar($code);
    is([sort @{ $loaded->keywords }], [sort qw(if else)], 'keywords preserved');
};

subtest 'compile_token_grammar: skip definitions round-trip' => sub {
    my ($original) = CodingAdventures::GrammarTools->parse_token_grammar(
        "NAME = /[a-z]+/\nskip:\n  WS = /[ \\t]+/\n"
    );
    my $code = CodingAdventures::GrammarTools::Compiler::compile_token_grammar($original);
    my $loaded = eval_token_grammar($code);
    is(scalar(@{ $loaded->skip_definitions }), 1, 'one skip definition');
    is($loaded->skip_definitions->[0]->name, 'WS', 'skip name preserved');
};

subtest 'compile_token_grammar: pattern groups round-trip' => sub {
    my ($original) = CodingAdventures::GrammarTools->parse_token_grammar(
        "TEXT = /[^<]+/\ngroup tag:\n  ATTR = /[a-z]+/\n  EQ = \"=\"\n"
    );
    my $code = CodingAdventures::GrammarTools::Compiler::compile_token_grammar($original);
    my $loaded = eval_token_grammar($code);
    ok(exists $loaded->groups->{tag}, 'tag group present');
    is(scalar(@{ $loaded->groups->{tag}->definitions }), 2, 'two definitions in tag group');
};

subtest 'compile_token_grammar: multiple pattern groups do not collide' => sub {
    # Regression test for the doubled-comma-style bug found in this
    # campaign's Python/Lua ports: a grammar with 2+ pattern groups must
    # not produce invalid or truncated Perl source.
    my ($original) = CodingAdventures::GrammarTools->parse_token_grammar(
        "TEXT = /[^<]+/\ngroup tag:\n  ATTR = /[a-z]+/\ngroup comment:\n  CTEXT = /[^-]+/\n"
    );
    my $code = CodingAdventures::GrammarTools::Compiler::compile_token_grammar($original);
    my $loaded = eval_token_grammar($code);
    ok(exists $loaded->groups->{tag}, 'tag group present');
    ok(exists $loaded->groups->{comment}, 'comment group present');
};

# ============================================================================
# compile_parser_grammar
# ============================================================================

subtest 'compile_parser_grammar: output structure' => sub {
    my ($grammar) = CodingAdventures::GrammarTools->parse_parser_grammar("start = NUMBER ;\n");
    my $code = CodingAdventures::GrammarTools::Compiler::compile_parser_grammar($grammar, 'x.grammar');
    like($code, qr/AUTO-GENERATED FILE/, 'has DO NOT EDIT header');
    like($code, qr/Source: x\.grammar/, 'has source comment');
    like($code, qr/sub parser_grammar/, 'defines parser_grammar sub');
};

subtest 'compile_parser_grammar: rule_reference round-trips' => sub {
    my ($original) = CodingAdventures::GrammarTools->parse_parser_grammar("start = NUMBER ;\n");
    my $code = CodingAdventures::GrammarTools::Compiler::compile_parser_grammar($original);
    my $loaded = eval_parser_grammar($code);
    is(scalar(@{ $loaded->{rules} }), 1, 'one rule');
    is($loaded->{rules}[0]{name}, 'start', 'rule name preserved');
    is($loaded->{rules}[0]{body}{type}, 'rule_reference', 'body type preserved');
    is($loaded->{rules}[0]{body}{name}, 'NUMBER', 'reference name preserved');
    is($loaded->{rules}[0]{body}{is_token}, 1, 'is_token preserved (uppercase = token)');
};

subtest 'compile_parser_grammar: sequence and alternation round-trip' => sub {
    my ($original) = CodingAdventures::GrammarTools->parse_parser_grammar(
        "expr = NUMBER PLUS NUMBER | NUMBER ;\n"
    );
    my $code = CodingAdventures::GrammarTools::Compiler::compile_parser_grammar($original);
    my $loaded = eval_parser_grammar($code);
    is($loaded->{rules}[0]{body}{type}, 'alternation', 'top-level alternation');
    is(scalar(@{ $loaded->{rules}[0]{body}{choices} }), 2, 'two alternatives');
    is($loaded->{rules}[0]{body}{choices}[0]{type}, 'sequence', 'first choice is a sequence');
};

subtest 'compile_parser_grammar: repetition and optional round-trip' => sub {
    my ($original) = CodingAdventures::GrammarTools->parse_parser_grammar(
        "stmts = { stmt } [ TAIL ] ;\n"
    );
    my $code = CodingAdventures::GrammarTools::Compiler::compile_parser_grammar($original);
    my $loaded = eval_parser_grammar($code);
    my $body = $loaded->{rules}[0]{body};
    is($body->{type}, 'sequence', 'top-level sequence');
    is($body->{elements}[0]{type}, 'repetition', 'repetition preserved');
    is($body->{elements}[1]{type}, 'optional', 'optional preserved');
};

subtest 'compile_parser_grammar: positive and negative lookahead round-trip' => sub {
    my ($original) = CodingAdventures::GrammarTools->parse_parser_grammar(
        'expr = &NUMBER value | !"end" stmt ;' . "\n"
    );
    my $code = CodingAdventures::GrammarTools::Compiler::compile_parser_grammar($original);
    my $loaded = eval_parser_grammar($code);
    my $choices = $loaded->{rules}[0]{body}{choices};
    is($choices->[0]{elements}[0]{type}, 'positive_lookahead', 'positive lookahead preserved');
    is($choices->[1]{elements}[0]{type}, 'negative_lookahead', 'negative lookahead preserved');
    is($choices->[1]{elements}[0]{element}{value}, 'end', 'negated literal value preserved');
};

subtest 'compile_parser_grammar: one-or-more and separated-repetition round-trip' => sub {
    my ($original) = CodingAdventures::GrammarTools->parse_parser_grammar(
        "stmts = { stmt }+ ;\nargs = { expr // COMMA }+ ;\n"
    );
    my $code = CodingAdventures::GrammarTools::Compiler::compile_parser_grammar($original);
    my $loaded = eval_parser_grammar($code);
    is($loaded->{rules}[0]{body}{type}, 'one_or_more', 'one_or_more preserved');
    my $sep_rule = $loaded->{rules}[1]{body};
    is($sep_rule->{type}, 'separated_repetition', 'separated_repetition preserved');
    is($sep_rule->{element}{name}, 'expr', 'element preserved');
    is($sep_rule->{separator}{name}, 'COMMA', 'separator preserved');
    is($sep_rule->{at_least_one}, 1, 'at_least_one preserved');
};

subtest 'compile_parser_grammar: group round-trips' => sub {
    my ($original) = CodingAdventures::GrammarTools->parse_parser_grammar("expr = ( A | B ) ;\n");
    my $code = CodingAdventures::GrammarTools::Compiler::compile_parser_grammar($original);
    my $loaded = eval_parser_grammar($code);
    is($loaded->{rules}[0]{body}{type}, 'group', 'group preserved');
};

subtest 'compile_parser_grammar: line_number preserved' => sub {
    my ($original) = CodingAdventures::GrammarTools->parse_parser_grammar("\n\nvalue = NUMBER ;\n");
    my $code = CodingAdventures::GrammarTools::Compiler::compile_parser_grammar($original);
    my $loaded = eval_parser_grammar($code);
    is($loaded->{rules}[0]{line_number}, 3, 'line_number preserved');
};

subtest 'compile_parser_grammar: JSON grammar full round-trip' => sub {
    my ($original) = CodingAdventures::GrammarTools->parse_parser_grammar(<<'GRAMMAR');
value = STRING | NUMBER | object | array | TRUE | FALSE | NULL ;
object = LBRACE [ member { COMMA member } ] RBRACE ;
member = STRING COLON value ;
array = LBRACKET [ value { COMMA value } ] RBRACKET ;
GRAMMAR
    my $code = CodingAdventures::GrammarTools::Compiler::compile_parser_grammar($original, 'json.grammar');
    my $loaded = eval_parser_grammar($code);
    is(scalar(@{ $loaded->{rules} }), 4, 'all 4 rules present');
    my %names = map { $_->{name} => 1 } @{ $loaded->{rules} };
    ok($names{$_}, "rule '$_' present") for qw(value object member array);
};

# ============================================================================
# package_name argument
# ============================================================================

subtest 'compile_token_grammar: package_name emits a package declaration' => sub {
    my ($original) = CodingAdventures::GrammarTools->parse_token_grammar("NUMBER = /[0-9]+/\n");
    my $code = CodingAdventures::GrammarTools::Compiler::compile_token_grammar(
        $original, 'x.tokens', 'CodingAdventures::Fixture::_Grammar'
    );
    like($code, qr/^package CodingAdventures::Fixture::_Grammar;/m, 'package line present');
    like($code, qr/use strict;/, 'use strict present');
    like($code, qr/use warnings;/, 'use warnings present');

    my $ok = eval $code;
    die "eval failed: $@" unless $ok;
    no strict 'refs';
    my $loaded = &{'CodingAdventures::Fixture::_Grammar::token_grammar'}();
    is(ref($loaded), 'CodingAdventures::GrammarTools::TokenGrammar', 'callable via qualified name');
};

subtest 'compile_token_grammar: omitted package_name has no package line' => sub {
    my ($original) = CodingAdventures::GrammarTools->parse_token_grammar("NUMBER = /[0-9]+/\n");
    my $code = CodingAdventures::GrammarTools::Compiler::compile_token_grammar($original, 'x.tokens');
    unlike($code, qr/^package /m, 'no package line when package_name omitted');
};

subtest 'compile_token_grammar: rejects a package_name that is not a bareword' => sub {
    # Regression test: package_name is spliced into generated source as
    # executable code (`package $package_name;`), unlike every grammar-
    # derived field, which goes through _perl_string(). A value containing
    # `;` would inject arbitrary Perl into the generated file.
    my ($original) = CodingAdventures::GrammarTools->parse_token_grammar("NUMBER = /[0-9]+/\n");
    for my $bad ('Foo; system("touch /tmp/pwned")', 'Foo::', '1Foo', 'Foo-Bar') {
        my $ok = eval {
            CodingAdventures::GrammarTools::Compiler::compile_token_grammar($original, 'x.tokens', $bad);
            1;
        };
        ok(!$ok, "rejects invalid package_name '$bad'");
    }
};

subtest 'compile_parser_grammar: package_name emits a package declaration' => sub {
    my ($original) = CodingAdventures::GrammarTools->parse_parser_grammar("start = NUMBER ;\n");
    my $code = CodingAdventures::GrammarTools::Compiler::compile_parser_grammar(
        $original, 'x.grammar', 'CodingAdventures::Fixture::_ParserGrammar'
    );
    like($code, qr/^package CodingAdventures::Fixture::_ParserGrammar;/m, 'package line present');

    my $ok = eval $code;
    die "eval failed: $@" unless $ok;
    no strict 'refs';
    my $loaded = &{'CodingAdventures::Fixture::_ParserGrammar::parser_grammar'}();
    is(ref($loaded), 'HASH', 'callable via qualified name');
};

done_testing;
