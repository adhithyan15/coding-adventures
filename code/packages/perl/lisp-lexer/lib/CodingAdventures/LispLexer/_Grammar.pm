# AUTO-GENERATED FILE — DO NOT EDIT
# Source: lisp.tokens
# Regenerate with: perl code/programs/perl/grammar-tools/grammar-tools.pl compile-tokens lisp.tokens
#
# This file embeds a TokenGrammar as native Perl data structures.
# Call token_grammar() instead of reading and parsing the .tokens file.

package CodingAdventures::LispLexer::_Grammar;
use strict;
use warnings;

sub token_grammar {
    return bless {
        definitions => [
            bless({
                name => 'NUMBER',
                pattern => '-?[0-9]+',
                is_regex => 1,
                line_number => 11,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'SYMBOL',
                pattern => '[a-zA-Z_+\\-*\\/=<>!?&][a-zA-Z0-9_+\\-*\\/=<>!?&]*',
                is_regex => 1,
                line_number => 12,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'STRING',
                pattern => '"([^"\\\\]|\\\\.)*"',
                is_regex => 1,
                line_number => 13,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'LPAREN',
                pattern => '(',
                is_regex => 0,
                line_number => 14,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'RPAREN',
                pattern => ')',
                is_regex => 0,
                line_number => 15,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'QUOTE',
                pattern => '\'',
                is_regex => 0,
                line_number => 16,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'DOT',
                pattern => '.',
                is_regex => 0,
                line_number => 17,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
        ],
        keywords => [],
        context_keywords => [],
        layout_keywords => [],
        soft_keywords => [],
        mode => '',
        escape_mode => 'none',
        skip_definitions => [
            bless({
                name => 'WHITESPACE',
                pattern => '[ \\t\\r\\n]+',
                is_regex => 1,
                line_number => 8,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'COMMENT',
                pattern => ';[^\\n]*',
                is_regex => 1,
                line_number => 9,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
        ],
        error_definitions => [],
        reserved_keywords => [],
        groups => {},
        start_mode => '',
        transitions => [],
    }, 'CodingAdventures::GrammarTools::TokenGrammar';
}

1;
