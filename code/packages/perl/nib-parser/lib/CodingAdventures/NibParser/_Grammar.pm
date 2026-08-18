# AUTO-GENERATED FILE — DO NOT EDIT
# Source: nib.grammar
# Regenerate with: perl code/programs/perl/grammar-tools/grammar-tools.pl compile-grammar nib.grammar
#
# This file embeds a ParserGrammar as native Perl data structures.
# Call parser_grammar() instead of reading and parsing the .grammar file.

package CodingAdventures::NibParser::_Grammar;
use strict;
use warnings;

sub parser_grammar {
    return {
        rules => [
        {
            name => 'program',
            body => { type => 'repetition', element => { type => 'rule_reference', name => 'top_decl', is_token => 0 } },
            line_number => 42,
        },
        {
            name => 'top_decl',
            body => { type => 'alternation', choices => [
                { type => 'rule_reference', name => 'const_decl', is_token => 0 },
                { type => 'rule_reference', name => 'static_decl', is_token => 0 },
                { type => 'rule_reference', name => 'fn_decl', is_token => 0 },
            ] },
            line_number => 47,
        },
        {
            name => 'const_decl',
            body => { type => 'sequence', elements => [
                { type => 'literal', value => 'const' },
                { type => 'rule_reference', name => 'NAME', is_token => 1 },
                { type => 'rule_reference', name => 'COLON', is_token => 1 },
                { type => 'rule_reference', name => 'type', is_token => 0 },
                { type => 'rule_reference', name => 'EQ', is_token => 1 },
                { type => 'rule_reference', name => 'expr', is_token => 0 },
                { type => 'rule_reference', name => 'SEMICOLON', is_token => 1 },
            ] },
            line_number => 60,
        },
        {
            name => 'static_decl',
            body => { type => 'sequence', elements => [
                { type => 'literal', value => 'static' },
                { type => 'rule_reference', name => 'NAME', is_token => 1 },
                { type => 'rule_reference', name => 'COLON', is_token => 1 },
                { type => 'rule_reference', name => 'type', is_token => 0 },
                { type => 'rule_reference', name => 'EQ', is_token => 1 },
                { type => 'rule_reference', name => 'expr', is_token => 0 },
                { type => 'rule_reference', name => 'SEMICOLON', is_token => 1 },
            ] },
            line_number => 66,
        },
        {
            name => 'fn_decl',
            body => { type => 'sequence', elements => [
                { type => 'literal', value => 'fn' },
                { type => 'rule_reference', name => 'NAME', is_token => 1 },
                { type => 'rule_reference', name => 'LPAREN', is_token => 1 },
                { type => 'optional', element => { type => 'rule_reference', name => 'param_list', is_token => 0 } },
                { type => 'rule_reference', name => 'RPAREN', is_token => 1 },
                { type => 'optional', element => { type => 'sequence', elements => [
                        { type => 'rule_reference', name => 'ARROW', is_token => 1 },
                        { type => 'rule_reference', name => 'type', is_token => 0 },
                    ] } },
                { type => 'rule_reference', name => 'block', is_token => 0 },
            ] },
            line_number => 77,
        },
        {
            name => 'param_list',
            body => { type => 'sequence', elements => [
                { type => 'rule_reference', name => 'param', is_token => 0 },
                { type => 'repetition', element => { type => 'sequence', elements => [
                        { type => 'rule_reference', name => 'COMMA', is_token => 1 },
                        { type => 'rule_reference', name => 'param', is_token => 0 },
                    ] } },
            ] },
            line_number => 80,
        },
        {
            name => 'param',
            body => { type => 'sequence', elements => [
                { type => 'rule_reference', name => 'NAME', is_token => 1 },
                { type => 'rule_reference', name => 'COLON', is_token => 1 },
                { type => 'rule_reference', name => 'type', is_token => 0 },
            ] },
            line_number => 87,
        },
        {
            name => 'block',
            body => { type => 'sequence', elements => [
                { type => 'rule_reference', name => 'LBRACE', is_token => 1 },
                { type => 'repetition', element => { type => 'rule_reference', name => 'stmt', is_token => 0 } },
                { type => 'rule_reference', name => 'RBRACE', is_token => 1 },
            ] },
            line_number => 98,
        },
        {
            name => 'stmt',
            body => { type => 'alternation', choices => [
                { type => 'rule_reference', name => 'let_stmt', is_token => 0 },
                { type => 'rule_reference', name => 'assign_stmt', is_token => 0 },
                { type => 'rule_reference', name => 'return_stmt', is_token => 0 },
                { type => 'rule_reference', name => 'for_stmt', is_token => 0 },
                { type => 'rule_reference', name => 'while_stmt', is_token => 0 },
                { type => 'rule_reference', name => 'if_stmt', is_token => 0 },
                { type => 'rule_reference', name => 'expr_stmt', is_token => 0 },
            ] },
            line_number => 113,
        },
        {
            name => 'let_stmt',
            body => { type => 'sequence', elements => [
                { type => 'literal', value => 'let' },
                { type => 'rule_reference', name => 'NAME', is_token => 1 },
                { type => 'rule_reference', name => 'COLON', is_token => 1 },
                { type => 'rule_reference', name => 'type', is_token => 0 },
                { type => 'rule_reference', name => 'EQ', is_token => 1 },
                { type => 'rule_reference', name => 'expr', is_token => 0 },
                { type => 'rule_reference', name => 'SEMICOLON', is_token => 1 },
            ] },
            line_number => 126,
        },
        {
            name => 'assign_stmt',
            body => { type => 'sequence', elements => [
                { type => 'rule_reference', name => 'NAME', is_token => 1 },
                { type => 'rule_reference', name => 'EQ', is_token => 1 },
                { type => 'rule_reference', name => 'expr', is_token => 0 },
                { type => 'rule_reference', name => 'SEMICOLON', is_token => 1 },
            ] },
            line_number => 131,
        },
        {
            name => 'return_stmt',
            body => { type => 'sequence', elements => [
                { type => 'literal', value => 'return' },
                { type => 'rule_reference', name => 'expr', is_token => 0 },
                { type => 'rule_reference', name => 'SEMICOLON', is_token => 1 },
            ] },
            line_number => 136,
        },
        {
            name => 'for_stmt',
            body => { type => 'sequence', elements => [
                { type => 'literal', value => 'for' },
                { type => 'rule_reference', name => 'NAME', is_token => 1 },
                { type => 'rule_reference', name => 'COLON', is_token => 1 },
                { type => 'rule_reference', name => 'type', is_token => 0 },
                { type => 'literal', value => 'in' },
                { type => 'rule_reference', name => 'expr', is_token => 0 },
                { type => 'rule_reference', name => 'RANGE', is_token => 1 },
                { type => 'rule_reference', name => 'expr', is_token => 0 },
                { type => 'rule_reference', name => 'block', is_token => 0 },
            ] },
            line_number => 159,
        },
        {
            name => 'while_stmt',
            body => { type => 'sequence', elements => [
                { type => 'literal', value => 'while' },
                { type => 'rule_reference', name => 'expr', is_token => 0 },
                { type => 'rule_reference', name => 'block', is_token => 0 },
            ] },
            line_number => 170,
        },
        {
            name => 'if_stmt',
            body => { type => 'sequence', elements => [
                { type => 'literal', value => 'if' },
                { type => 'rule_reference', name => 'expr', is_token => 0 },
                { type => 'rule_reference', name => 'block', is_token => 0 },
                { type => 'optional', element => { type => 'sequence', elements => [
                        { type => 'literal', value => 'else' },
                        { type => 'rule_reference', name => 'block', is_token => 0 },
                    ] } },
            ] },
            line_number => 176,
        },
        {
            name => 'expr_stmt',
            body => { type => 'sequence', elements => [
                { type => 'rule_reference', name => 'expr', is_token => 0 },
                { type => 'rule_reference', name => 'SEMICOLON', is_token => 1 },
            ] },
            line_number => 183,
        },
        {
            name => 'type',
            body => { type => 'alternation', choices => [
                { type => 'literal', value => 'u4' },
                { type => 'literal', value => 'u8' },
                { type => 'literal', value => 'bcd' },
                { type => 'literal', value => 'bool' },
            ] },
            line_number => 218,
        },
        {
            name => 'expr',
            body => { type => 'rule_reference', name => 'or_expr', is_token => 0 },
            line_number => 259,
        },
        {
            name => 'or_expr',
            body => { type => 'sequence', elements => [
                { type => 'rule_reference', name => 'and_expr', is_token => 0 },
                { type => 'repetition', element => { type => 'sequence', elements => [
                        { type => 'rule_reference', name => 'LOR', is_token => 1 },
                        { type => 'rule_reference', name => 'and_expr', is_token => 0 },
                    ] } },
            ] },
            line_number => 265,
        },
        {
            name => 'and_expr',
            body => { type => 'sequence', elements => [
                { type => 'rule_reference', name => 'eq_expr', is_token => 0 },
                { type => 'repetition', element => { type => 'sequence', elements => [
                        { type => 'rule_reference', name => 'LAND', is_token => 1 },
                        { type => 'rule_reference', name => 'eq_expr', is_token => 0 },
                    ] } },
            ] },
            line_number => 269,
        },
        {
            name => 'eq_expr',
            body => { type => 'sequence', elements => [
                { type => 'rule_reference', name => 'cmp_expr', is_token => 0 },
                { type => 'repetition', element => { type => 'sequence', elements => [
                        { type => 'group', element => { type => 'alternation', choices => [
                                { type => 'rule_reference', name => 'EQ_EQ', is_token => 1 },
                                { type => 'rule_reference', name => 'NEQ', is_token => 1 },
                            ] } },
                        { type => 'rule_reference', name => 'cmp_expr', is_token => 0 },
                    ] } },
            ] },
            line_number => 274,
        },
        {
            name => 'cmp_expr',
            body => { type => 'sequence', elements => [
                { type => 'rule_reference', name => 'add_expr', is_token => 0 },
                { type => 'repetition', element => { type => 'sequence', elements => [
                        { type => 'group', element => { type => 'alternation', choices => [
                                { type => 'rule_reference', name => 'LT', is_token => 1 },
                                { type => 'rule_reference', name => 'GT', is_token => 1 },
                                { type => 'rule_reference', name => 'LEQ', is_token => 1 },
                                { type => 'rule_reference', name => 'GEQ', is_token => 1 },
                            ] } },
                        { type => 'rule_reference', name => 'add_expr', is_token => 0 },
                    ] } },
            ] },
            line_number => 280,
        },
        {
            name => 'add_expr',
            body => { type => 'sequence', elements => [
                { type => 'rule_reference', name => 'shift_expr', is_token => 0 },
                { type => 'repetition', element => { type => 'sequence', elements => [
                        { type => 'group', element => { type => 'alternation', choices => [
                                { type => 'rule_reference', name => 'PLUS', is_token => 1 },
                                { type => 'rule_reference', name => 'MINUS', is_token => 1 },
                                { type => 'rule_reference', name => 'WRAP_ADD', is_token => 1 },
                                { type => 'rule_reference', name => 'SAT_ADD', is_token => 1 },
                            ] } },
                        { type => 'rule_reference', name => 'shift_expr', is_token => 0 },
                    ] } },
            ] },
            line_number => 293,
        },
        {
            name => 'shift_expr',
            body => { type => 'sequence', elements => [
                { type => 'rule_reference', name => 'mul_expr', is_token => 0 },
                { type => 'repetition', element => { type => 'sequence', elements => [
                        { type => 'group', element => { type => 'alternation', choices => [
                                { type => 'rule_reference', name => 'SHL', is_token => 1 },
                                { type => 'rule_reference', name => 'SHR', is_token => 1 },
                            ] } },
                        { type => 'rule_reference', name => 'mul_expr', is_token => 0 },
                    ] } },
            ] },
            line_number => 298,
        },
        {
            name => 'mul_expr',
            body => { type => 'sequence', elements => [
                { type => 'rule_reference', name => 'bitwise_expr', is_token => 0 },
                { type => 'repetition', element => { type => 'sequence', elements => [
                        { type => 'group', element => { type => 'alternation', choices => [
                                { type => 'rule_reference', name => 'STAR', is_token => 1 },
                                { type => 'rule_reference', name => 'SLASH', is_token => 1 },
                                { type => 'rule_reference', name => 'PERCENT', is_token => 1 },
                            ] } },
                        { type => 'rule_reference', name => 'bitwise_expr', is_token => 0 },
                    ] } },
            ] },
            line_number => 308,
        },
        {
            name => 'bitwise_expr',
            body => { type => 'sequence', elements => [
                { type => 'rule_reference', name => 'unary_expr', is_token => 0 },
                { type => 'repetition', element => { type => 'sequence', elements => [
                        { type => 'group', element => { type => 'alternation', choices => [
                                { type => 'rule_reference', name => 'AMP', is_token => 1 },
                                { type => 'rule_reference', name => 'PIPE', is_token => 1 },
                                { type => 'rule_reference', name => 'CARET', is_token => 1 },
                            ] } },
                        { type => 'rule_reference', name => 'unary_expr', is_token => 0 },
                    ] } },
            ] },
            line_number => 314,
        },
        {
            name => 'unary_expr',
            body => { type => 'alternation', choices => [
                { type => 'sequence', elements => [
                    { type => 'group', element => { type => 'alternation', choices => [
                            { type => 'rule_reference', name => 'BANG', is_token => 1 },
                            { type => 'rule_reference', name => 'TILDE', is_token => 1 },
                        ] } },
                    { type => 'rule_reference', name => 'unary_expr', is_token => 0 },
                ] },
                { type => 'rule_reference', name => 'primary', is_token => 0 },
            ] },
            line_number => 322,
        },
        {
            name => 'primary',
            body => { type => 'alternation', choices => [
                { type => 'rule_reference', name => 'INT_LIT', is_token => 1 },
                { type => 'rule_reference', name => 'HEX_LIT', is_token => 1 },
                { type => 'literal', value => 'true' },
                { type => 'literal', value => 'false' },
                { type => 'rule_reference', name => 'call_expr', is_token => 0 },
                { type => 'rule_reference', name => 'NAME', is_token => 1 },
                { type => 'sequence', elements => [
                    { type => 'rule_reference', name => 'LPAREN', is_token => 1 },
                    { type => 'rule_reference', name => 'expr', is_token => 0 },
                    { type => 'rule_reference', name => 'RPAREN', is_token => 1 },
                ] },
            ] },
            line_number => 330,
        },
        {
            name => 'call_expr',
            body => { type => 'sequence', elements => [
                { type => 'rule_reference', name => 'NAME', is_token => 1 },
                { type => 'rule_reference', name => 'LPAREN', is_token => 1 },
                { type => 'optional', element => { type => 'rule_reference', name => 'arg_list', is_token => 0 } },
                { type => 'rule_reference', name => 'RPAREN', is_token => 1 },
            ] },
            line_number => 353,
        },
        {
            name => 'arg_list',
            body => { type => 'sequence', elements => [
                { type => 'rule_reference', name => 'expr', is_token => 0 },
                { type => 'repetition', element => { type => 'sequence', elements => [
                        { type => 'rule_reference', name => 'COMMA', is_token => 1 },
                        { type => 'rule_reference', name => 'expr', is_token => 0 },
                    ] } },
            ] },
            line_number => 356,
        },
    ],
        version => 0,
    };
}

1;
