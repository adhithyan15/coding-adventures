# AUTO-GENERATED FILE — DO NOT EDIT
# Source: brainfuck.tokens
# Regenerate with: perl code/programs/perl/grammar-tools/grammar-tools.pl compile-tokens brainfuck.tokens
#
# This file embeds a TokenGrammar as native Perl data structures.
# Call token_grammar() instead of reading and parsing the .tokens file.

package CodingAdventures::Brainfuck::Lexer::_Grammar;
use strict;
use warnings;

sub token_grammar {
    return bless {
        definitions => [
            bless({
                name => 'RIGHT',
                pattern => '>',
                is_regex => 0,
                line_number => 23,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'LEFT',
                pattern => '<',
                is_regex => 0,
                line_number => 24,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'INC',
                pattern => '+',
                is_regex => 0,
                line_number => 29,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'DEC',
                pattern => '-',
                is_regex => 0,
                line_number => 30,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'OUTPUT',
                pattern => '.',
                is_regex => 0,
                line_number => 35,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'INPUT',
                pattern => ',',
                is_regex => 0,
                line_number => 36,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'LOOP_START',
                pattern => '[',
                is_regex => 0,
                line_number => 41,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'LOOP_END',
                pattern => ']',
                is_regex => 0,
                line_number => 42,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
        ],
        keywords => [],
        context_keywords => [],
        layout_keywords => [],
        soft_keywords => [],
        mode => '',
        escape_mode => '',
        skip_definitions => [
            bless({
                name => 'WHITESPACE',
                pattern => '[ \\t\\r\\n]+',
                is_regex => 1,
                line_number => 65,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'COMMENT',
                pattern => '[^><+\\-.,\\[\\] \\t\\r\\n]+',
                is_regex => 1,
                line_number => 66,
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
