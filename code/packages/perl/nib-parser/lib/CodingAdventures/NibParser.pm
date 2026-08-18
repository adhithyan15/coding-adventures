package CodingAdventures::NibParser;

use strict;
use warnings;

our $VERSION = '0.02';

use CodingAdventures::GrammarTools;
use CodingAdventures::Parser;
use CodingAdventures::NibLexer;

my $_grammar;

# _grammar() loads the pre-compiled `nib.grammar` (cached for the lifetime
# of the process) instead of reading and parsing nib.grammar off disk.
# CodingAdventures::NibParser::_Grammar is a checked-in generated module
# produced ahead of time by
# `grammar-tools.pl compile-grammar code/grammars/nib/nib.grammar`; a real
# CPAN install of this package does not ship code/grammars/, so the old
# disk-read approach would fail outside this monorepo checkout.
#
# Note: unlike TokenGrammar (a blessed object with accessor methods),
# ParserGrammar is a plain hashref: { rules => [...], version => N },
# accessed via $grammar->{rules} / $grammar->{version} in the consuming
# grammar parser, not method calls.
sub _grammar {
    return $_grammar if $_grammar;

    require CodingAdventures::NibParser::_Grammar;
    $_grammar = CodingAdventures::NibParser::_Grammar::parser_grammar();

    return $_grammar;
}

sub parse {
    my ($class, $source) = @_;
    my $grammar = _grammar();

    my $tokens = CodingAdventures::NibLexer->tokenize($source);
    my $parser = CodingAdventures::Parser->new_grammar_parser($tokens, $grammar);
    return $parser->grammar_parse();
}

1;
