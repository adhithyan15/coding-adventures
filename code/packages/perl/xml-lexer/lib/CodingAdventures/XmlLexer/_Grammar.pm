# AUTO-GENERATED FILE — DO NOT EDIT
# Source: xml.tokens
# Regenerate with: perl code/programs/perl/grammar-tools/grammar-tools.pl compile-tokens xml.tokens
#
# This file embeds a TokenGrammar as native Perl data structures.
# Call token_grammar() instead of reading and parsing the .tokens file.

package CodingAdventures::XmlLexer::_Grammar;
use strict;
use warnings;

sub token_grammar {
    return bless {
        definitions => [
            bless({
                name => 'TEXT',
                pattern => '[^<&]+',
                is_regex => 1,
                line_number => 77,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'ENTITY_REF',
                pattern => '&[a-zA-Z][a-zA-Z0-9]*;',
                is_regex => 1,
                line_number => 78,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'CHAR_REF_HEX',
                pattern => '&#x[0-9a-fA-F]+;',
                is_regex => 1,
                line_number => 85,
                alias => 'CHAR_REF',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'CHAR_REF_DEC',
                pattern => '&#[0-9]+;',
                is_regex => 1,
                line_number => 86,
                alias => 'CHAR_REF',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'COMMENT_START',
                pattern => '<!--',
                is_regex => 0,
                line_number => 88,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'CDATA_START',
                pattern => '<![CDATA[',
                is_regex => 0,
                line_number => 89,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'PI_START',
                pattern => '<?',
                is_regex => 0,
                line_number => 90,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'CLOSE_TAG_START',
                pattern => '</',
                is_regex => 0,
                line_number => 91,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
            bless({
                name => 'OPEN_TAG_START',
                pattern => '<',
                is_regex => 0,
                line_number => 92,
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
                line_number => 62,
                alias => '',
            }, 'CodingAdventures::GrammarTools::TokenDefinition'),
        ],
        error_definitions => [],
        reserved_keywords => [],
        groups => {
            'cdata' => bless({
                name => 'cdata',
                definitions => [
                        bless({
                            name => 'CDATA_END',
                            pattern => ']]>',
                            is_regex => 0,
                            line_number => 150,
                            alias => '',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                        bless({
                            name => 'CDATA_TEXT',
                            pattern => '[^\\]]+',
                            is_regex => 1,
                            line_number => 151,
                            alias => '',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                        bless({
                            name => 'CDATA_BRACK',
                            pattern => ']',
                            is_regex => 1,
                            line_number => 152,
                            alias => 'CDATA_TEXT',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                    ],
            }, 'CodingAdventures::GrammarTools::PatternGroup'),
            'comment' => bless({
                name => 'comment',
                definitions => [
                        bless({
                            name => 'COMMENT_END',
                            pattern => '-->',
                            is_regex => 0,
                            line_number => 133,
                            alias => '',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                        bless({
                            name => 'COMMENT_TEXT',
                            pattern => '[^-]+',
                            is_regex => 1,
                            line_number => 134,
                            alias => '',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                        bless({
                            name => 'COMMENT_DASH',
                            pattern => '-',
                            is_regex => 1,
                            line_number => 135,
                            alias => 'COMMENT_TEXT',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                    ],
            }, 'CodingAdventures::GrammarTools::PatternGroup'),
            'pi' => bless({
                name => 'pi',
                definitions => [
                        bless({
                            name => 'PI_END',
                            pattern => '?>',
                            is_regex => 0,
                            line_number => 184,
                            alias => '',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                        bless({
                            name => 'PI_TARGET',
                            pattern => '[a-zA-Z_][a-zA-Z0-9_:.-]*',
                            is_regex => 1,
                            line_number => 185,
                            alias => '',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                    ],
            }, 'CodingAdventures::GrammarTools::PatternGroup'),
            'pi_body' => bless({
                name => 'pi_body',
                definitions => [
                        bless({
                            name => 'PI_END',
                            pattern => '?>',
                            is_regex => 0,
                            line_number => 188,
                            alias => '',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                        bless({
                            name => 'PI_TEXT',
                            pattern => '[^?]+',
                            is_regex => 1,
                            line_number => 189,
                            alias => '',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                        bless({
                            name => 'PI_QMARK',
                            pattern => '\\?',
                            is_regex => 1,
                            line_number => 190,
                            alias => 'PI_TEXT',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                    ],
            }, 'CodingAdventures::GrammarTools::PatternGroup'),
            'tag' => bless({
                name => 'tag',
                definitions => [
                        bless({
                            name => 'TAG_NAME',
                            pattern => '[a-zA-Z_][a-zA-Z0-9_:.-]*',
                            is_regex => 1,
                            line_number => 107,
                            alias => '',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                        bless({
                            name => 'ATTR_EQUALS',
                            pattern => '=',
                            is_regex => 0,
                            line_number => 108,
                            alias => '',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                        bless({
                            name => 'ATTR_VALUE_DQ',
                            pattern => '"[^"]*"',
                            is_regex => 1,
                            line_number => 109,
                            alias => 'ATTR_VALUE',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                        bless({
                            name => 'ATTR_VALUE_SQ',
                            pattern => '\'[^\']*\'',
                            is_regex => 1,
                            line_number => 110,
                            alias => 'ATTR_VALUE',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                        bless({
                            name => 'TAG_CLOSE',
                            pattern => '>',
                            is_regex => 0,
                            line_number => 111,
                            alias => '',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                        bless({
                            name => 'SELF_CLOSE',
                            pattern => '/>',
                            is_regex => 0,
                            line_number => 112,
                            alias => '',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                        bless({
                            name => 'SLASH',
                            pattern => '/',
                            is_regex => 0,
                            line_number => 113,
                            alias => '',
                        }, 'CodingAdventures::GrammarTools::TokenDefinition'),
                    ],
            }, 'CodingAdventures::GrammarTools::PatternGroup'),
        },
        start_mode => '',
        transitions => [],
    }, 'CodingAdventures::GrammarTools::TokenGrammar';
}

1;
