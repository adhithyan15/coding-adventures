# AUTO-GENERATED FILE — DO NOT EDIT
# Source: python.tokens
# Regenerate with: perl code/programs/perl/grammar-tools/grammar-tools.pl compile-tokens python.tokens
#
# This file embeds a TokenGrammar as native Perl data structures.
# Call token_grammar() instead of reading and parsing the .tokens file.

package CodingAdventures::PythonLexer::_Grammar_legacy;
use strict;
use warnings;

sub token_grammar {
    return bless {
        definitions => [
            bless({
                name => 'NAME',
                pattern => '[a-zA-Z_][a-zA-Z0-9_]*',
                is_regex => 1,
                line_number => 13,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'NUMBER',
                pattern => '[0-9]+',
                is_regex => 1,
                line_number => 14,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'STRING',
                pattern => '"([^"\\\\]|\\\\.)*"',
                is_regex => 1,
                line_number => 15,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'EQUALS_EQUALS',
                pattern => '==',
                is_regex => 0,
                line_number => 18,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'EQUALS',
                pattern => '=',
                is_regex => 0,
                line_number => 21,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'PLUS',
                pattern => '+',
                is_regex => 0,
                line_number => 22,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'MINUS',
                pattern => '-',
                is_regex => 0,
                line_number => 23,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'STAR',
                pattern => '*',
                is_regex => 0,
                line_number => 24,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'SLASH',
                pattern => '/',
                is_regex => 0,
                line_number => 25,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'LPAREN',
                pattern => '(',
                is_regex => 0,
                line_number => 28,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'RPAREN',
                pattern => ')',
                is_regex => 0,
                line_number => 29,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'COMMA',
                pattern => ',',
                is_regex => 0,
                line_number => 30,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'COLON',
                pattern => ':',
                is_regex => 0,
                line_number => 31,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
        ],
        keywords => [
            'if',
            'else',
            'elif',
            'while',
            'for',
            'def',
            'return',
            'class',
            'import',
            'from',
            'as',
            'True',
            'False',
            'None',
        ],
        context_keywords => [],
        layout_keywords => [],
        soft_keywords => [],
        mode => '',
        escape_mode => '',
        skip_definitions => [],
        error_definitions => [],
        reserved_keywords => [],
        groups => {},
        start_mode => '',
        transitions => [],
    }, 'CodingAdventures::GrammarTools::TokenGrammar';
}

1;
