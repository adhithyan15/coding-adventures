# AUTO-GENERATED FILE — DO NOT EDIT
# Source: ruby.tokens
# Regenerate with: perl code/programs/perl/grammar-tools/grammar-tools.pl compile-tokens ruby.tokens
#
# This file embeds a TokenGrammar as native Perl data structures.
# Call token_grammar() instead of reading and parsing the .tokens file.

package CodingAdventures::RubyLexer::_Grammar;
use strict;
use warnings;

sub token_grammar {
    return bless {
        definitions => [
            bless({
                name => 'NAME',
                pattern => '[a-zA-Z_][a-zA-Z0-9_]*',
                is_regex => 1,
                line_number => 23,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'NUMBER',
                pattern => '[0-9]+',
                is_regex => 1,
                line_number => 24,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'STRING',
                pattern => '"([^"\\\\]|\\\\.)*"',
                is_regex => 1,
                line_number => 25,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'EQUALS_EQUALS',
                pattern => '==',
                is_regex => 0,
                line_number => 28,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'DOT_DOT',
                pattern => '..',
                is_regex => 0,
                line_number => 29,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'HASH_ROCKET',
                pattern => '=>',
                is_regex => 0,
                line_number => 30,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'NOT_EQUALS',
                pattern => '!=',
                is_regex => 0,
                line_number => 31,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'LESS_EQUALS',
                pattern => '<=',
                is_regex => 0,
                line_number => 32,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'GREATER_EQUALS',
                pattern => '>=',
                is_regex => 0,
                line_number => 33,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'EQUALS',
                pattern => '=',
                is_regex => 0,
                line_number => 36,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'PLUS',
                pattern => '+',
                is_regex => 0,
                line_number => 37,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'MINUS',
                pattern => '-',
                is_regex => 0,
                line_number => 38,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'STAR',
                pattern => '*',
                is_regex => 0,
                line_number => 39,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'SLASH',
                pattern => '/',
                is_regex => 0,
                line_number => 40,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'LESS_THAN',
                pattern => '<',
                is_regex => 0,
                line_number => 43,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'GREATER_THAN',
                pattern => '>',
                is_regex => 0,
                line_number => 44,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'LPAREN',
                pattern => '(',
                is_regex => 0,
                line_number => 47,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'RPAREN',
                pattern => ')',
                is_regex => 0,
                line_number => 48,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'COMMA',
                pattern => ',',
                is_regex => 0,
                line_number => 49,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'COLON',
                pattern => ':',
                is_regex => 0,
                line_number => 50,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
        ],
        keywords => [
            'if',
            'else',
            'elsif',
            'end',
            'while',
            'for',
            'do',
            'def',
            'return',
            'class',
            'module',
            'require',
            'puts',
            'true',
            'false',
            'nil',
            'and',
            'or',
            'not',
            'then',
            'unless',
            'until',
            'yield',
            'begin',
            'rescue',
            'ensure',
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
