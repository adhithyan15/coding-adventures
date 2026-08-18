# AUTO-GENERATED FILE — DO NOT EDIT
# Source: dartmouth_basic.tokens
# Regenerate with: perl code/programs/perl/grammar-tools/grammar-tools.pl compile-tokens dartmouth_basic.tokens
#
# This file embeds a TokenGrammar as native Perl data structures.
# Call token_grammar() instead of reading and parsing the .tokens file.

package CodingAdventures::DartmouthBasicLexer::_Grammar;
use strict;
use warnings;

sub token_grammar {
    return bless {
        definitions => [
            bless({
                name => 'LE',
                pattern => '<=',
                is_regex => 0,
                line_number => 50,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'GE',
                pattern => '>=',
                is_regex => 0,
                line_number => 51,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'NE',
                pattern => '<>',
                is_regex => 0,
                line_number => 52,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'NUMBER',
                pattern => '[0-9]*\\.?[0-9]+([Ee][+-]?[0-9]+)?',
                is_regex => 1,
                line_number => 85,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'LINE_NUM',
                pattern => '[0-9]+',
                is_regex => 1,
                line_number => 86,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'STRING_BODY',
                pattern => '"[^"]*"',
                is_regex => 1,
                line_number => 112,
                alias => 'STRING',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'BUILTIN_FN',
                pattern => '(?:sin|cos|tan|atn|exp|log|abs|sqr|int|rnd|sgn)',
                is_regex => 1,
                line_number => 168,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'USER_FN',
                pattern => 'fn[a-z]',
                is_regex => 1,
                line_number => 169,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'NAME',
                pattern => '[a-z][a-z0-9]*\\$?',
                is_regex => 1,
                line_number => 204,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'PLUS',
                pattern => '+',
                is_regex => 0,
                line_number => 244,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'MINUS',
                pattern => '-',
                is_regex => 0,
                line_number => 245,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'STAR',
                pattern => '*',
                is_regex => 0,
                line_number => 246,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'SLASH',
                pattern => '/',
                is_regex => 0,
                line_number => 247,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'CARET',
                pattern => '^',
                is_regex => 0,
                line_number => 248,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'EQ',
                pattern => '=',
                is_regex => 0,
                line_number => 249,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'LT',
                pattern => '<',
                is_regex => 0,
                line_number => 250,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'GT',
                pattern => '>',
                is_regex => 0,
                line_number => 251,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'LPAREN',
                pattern => '(',
                is_regex => 0,
                line_number => 252,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'RPAREN',
                pattern => ')',
                is_regex => 0,
                line_number => 253,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'COMMA',
                pattern => ',',
                is_regex => 0,
                line_number => 254,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'SEMICOLON',
                pattern => ';',
                is_regex => 0,
                line_number => 255,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'NEWLINE',
                pattern => '\\r?\\n',
                is_regex => 1,
                line_number => 276,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
        ],
        keywords => [
            'LET',
            'PRINT',
            'INPUT',
            'IF',
            'THEN',
            'GOTO',
            'GOSUB',
            'RETURN',
            'FOR',
            'TO',
            'STEP',
            'NEXT',
            'END',
            'STOP',
            'REM',
            'READ',
            'DATA',
            'RESTORE',
            'DIM',
            'DEF',
        ],
        context_keywords => [],
        layout_keywords => [],
        soft_keywords => [],
        mode => '',
        escape_mode => '',
        skip_definitions => [
            bless({
                name => 'WHITESPACE',
                pattern => '[ \\t]+',
                is_regex => 1,
                line_number => 288,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
        ],
        error_definitions => [
            bless({
                name => 'UNKNOWN',
                pattern => '.',
                is_regex => 1,
                line_number => 304,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
        ],
        reserved_keywords => [],
        groups => {},
        start_mode => '',
        transitions => [],
    }, 'CodingAdventures::GrammarTools::TokenGrammar';
}

1;
