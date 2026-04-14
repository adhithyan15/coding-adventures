package CodingAdventures::NibParser;

use strict;
use warnings;

our $VERSION = '0.01';

use File::Basename qw(dirname);
use File::Spec;
use CodingAdventures::GrammarTools;
use CodingAdventures::Parser;
use CodingAdventures::NibLexer;

sub _grammars_dir {
    my $dir = File::Spec->rel2abs(dirname(__FILE__));
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

sub parse {
    my ($class, $source) = @_;
    my $grammar_path = File::Spec->catfile(_grammars_dir(), 'nib.grammar');
    open my $fh, '<', $grammar_path
        or die "CodingAdventures::NibParser: cannot open '$grammar_path': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_parser_grammar($content);
    die "CodingAdventures::NibParser: failed to parse nib.grammar: $err" unless $grammar;

    my $tokens = CodingAdventures::NibLexer->tokenize($source);
    my $parser = CodingAdventures::Parser->new_grammar_parser($tokens, $grammar);
    return $parser->grammar_parse();
}

1;
