# AUTO-GENERATED FILE — DO NOT EDIT
# Source: json.tokens
# Regenerate with: perl code/programs/perl/grammar-tools/grammar-tools.pl compile-tokens json.tokens
#
# This file embeds a TokenGrammar as native Perl data structures.
# Call token_grammar() instead of reading and parsing the .tokens file.

package CodingAdventures::JsonLexer::_Grammar;
use strict;
use warnings;

sub token_grammar {
    return bless {
        definitions => [
            bless({
                name => 'STRING',
                pattern => '"([^"\\\\]|\\\\["\\\\\\x2fbfnrt]|\\\\u[0-9a-fA-F]{4})*"',
                is_regex => 1,
                line_number => 30,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'NUMBER',
                pattern => '-?[0-9]+\\.?[0-9]*[eE]?[-+]?[0-9]*',
                is_regex => 1,
                line_number => 37,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'TRUE',
                pattern => 'true',
                is_regex => 0,
                line_number => 41,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'FALSE',
                pattern => 'false',
                is_regex => 0,
                line_number => 42,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'NULL',
                pattern => 'null',
                is_regex => 0,
                line_number => 43,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'LBRACE',
                pattern => '{',
                is_regex => 0,
                line_number => 49,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'RBRACE',
                pattern => '}',
                is_regex => 0,
                line_number => 50,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'LBRACKET',
                pattern => '[',
                is_regex => 0,
                line_number => 51,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'RBRACKET',
                pattern => ']',
                is_regex => 0,
                line_number => 52,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'COLON',
                pattern => ':',
                is_regex => 0,
                line_number => 53,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'COMMA',
                pattern => ',',
                is_regex => 0,
                line_number => 54,
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
                line_number => 65,
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
