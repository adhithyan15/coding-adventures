use strict;
use warnings;
use Test2::V0;

use lib 'lib';
use CodingAdventures::StarlarkAstToBytecodeCompiler;

# Shorter alias
my $C = 'CodingAdventures::StarlarkAstToBytecodeCompiler';
sub tok  { $C->can('token_node')->(@_) }
sub anode { $C->can('ast_node')->(@_) }

sub compile_tree {
    my ($tree) = @_;
    my $compiler = $C->create_compiler();
    return $compiler->compile($tree);
}

sub opcodes_of {
    my ($co) = @_;
    return [map { $_->{opcode} } @{$co->{instructions}}];
}

# ============================================================================
# Opcode Constants
# ============================================================================

subtest 'opcode constants — stack ops' => sub {
    is($C->can('OP_LOAD_CONST')->(), 0x01, 'OP_LOAD_CONST = 0x01');
    is($C->can('OP_POP')->(),        0x02, 'OP_POP = 0x02');
    is($C->can('OP_DUP')->(),        0x03, 'OP_DUP = 0x03');
    is($C->can('OP_LOAD_NONE')->(),  0x04, 'OP_LOAD_NONE = 0x04');
    is($C->can('OP_LOAD_TRUE')->(),  0x05, 'OP_LOAD_TRUE = 0x05');
    is($C->can('OP_LOAD_FALSE')->(), 0x06, 'OP_LOAD_FALSE = 0x06');
};

subtest 'opcode constants — variable ops' => sub {
    is($C->can('OP_STORE_NAME')->(),  0x10, 'OP_STORE_NAME');
    is($C->can('OP_LOAD_NAME')->(),   0x11, 'OP_LOAD_NAME');
    is($C->can('OP_STORE_LOCAL')->(), 0x12, 'OP_STORE_LOCAL');
    is($C->can('OP_LOAD_LOCAL')->(),  0x13, 'OP_LOAD_LOCAL');
};

subtest 'opcode constants — arithmetic ops' => sub {
    is($C->can('OP_ADD')->(),       0x20, 'OP_ADD');
    is($C->can('OP_SUB')->(),       0x21, 'OP_SUB');
    is($C->can('OP_MUL')->(),       0x22, 'OP_MUL');
    is($C->can('OP_DIV')->(),       0x23, 'OP_DIV');
    is($C->can('OP_FLOOR_DIV')->(), 0x24, 'OP_FLOOR_DIV');
    is($C->can('OP_MOD')->(),       0x25, 'OP_MOD');
    is($C->can('OP_POWER')->(),     0x26, 'OP_POWER');
    is($C->can('OP_NEGATE')->(),    0x27, 'OP_NEGATE');
};

subtest 'opcode constants — bitwise ops' => sub {
    is($C->can('OP_BIT_AND')->(), 0x28, 'OP_BIT_AND');
    is($C->can('OP_BIT_OR')->(),  0x29, 'OP_BIT_OR');
    is($C->can('OP_BIT_XOR')->(), 0x2A, 'OP_BIT_XOR');
    is($C->can('OP_BIT_NOT')->(), 0x2B, 'OP_BIT_NOT');
    is($C->can('OP_LSHIFT')->(),  0x2C, 'OP_LSHIFT');
    is($C->can('OP_RSHIFT')->(),  0x2D, 'OP_RSHIFT');
};

subtest 'opcode constants — comparison ops' => sub {
    is($C->can('OP_CMP_EQ')->(),      0x30, 'OP_CMP_EQ');
    is($C->can('OP_CMP_NE')->(),      0x31, 'OP_CMP_NE');
    is($C->can('OP_CMP_LT')->(),      0x32, 'OP_CMP_LT');
    is($C->can('OP_CMP_GT')->(),      0x33, 'OP_CMP_GT');
    is($C->can('OP_CMP_LE')->(),      0x34, 'OP_CMP_LE');
    is($C->can('OP_CMP_GE')->(),      0x35, 'OP_CMP_GE');
    is($C->can('OP_CMP_IN')->(),      0x36, 'OP_CMP_IN');
    is($C->can('OP_CMP_NOT_IN')->(),  0x37, 'OP_CMP_NOT_IN');
    is($C->can('OP_LOGICAL_NOT')->(), 0x38, 'OP_LOGICAL_NOT');
};

subtest 'opcode constants — control flow' => sub {
    is($C->can('OP_JUMP')->(),                  0x40, 'OP_JUMP');
    is($C->can('OP_JUMP_IF_FALSE')->(),         0x41, 'OP_JUMP_IF_FALSE');
    is($C->can('OP_JUMP_IF_TRUE')->(),          0x42, 'OP_JUMP_IF_TRUE');
    is($C->can('OP_JUMP_IF_FALSE_OR_POP')->(),  0x43, 'OP_JUMP_IF_FALSE_OR_POP');
    is($C->can('OP_JUMP_IF_TRUE_OR_POP')->(),   0x44, 'OP_JUMP_IF_TRUE_OR_POP');
};

subtest 'opcode constants — function + collection + misc ops' => sub {
    is($C->can('OP_MAKE_FUNCTION')->(),   0x50, 'OP_MAKE_FUNCTION');
    is($C->can('OP_CALL_FUNCTION')->(),   0x51, 'OP_CALL_FUNCTION');
    is($C->can('OP_RETURN')->(),          0x53, 'OP_RETURN');
    is($C->can('OP_BUILD_LIST')->(),      0x60, 'OP_BUILD_LIST');
    is($C->can('OP_BUILD_DICT')->(),      0x61, 'OP_BUILD_DICT');
    is($C->can('OP_BUILD_TUPLE')->(),     0x62, 'OP_BUILD_TUPLE');
    is($C->can('OP_LIST_APPEND')->(),     0x63, 'OP_LIST_APPEND');
    is($C->can('OP_DICT_SET')->(),        0x64, 'OP_DICT_SET');
    is($C->can('OP_LOAD_SUBSCRIPT')->(),  0x70, 'OP_LOAD_SUBSCRIPT');
    is($C->can('OP_LOAD_ATTR')->(),       0x72, 'OP_LOAD_ATTR');
    is($C->can('OP_GET_ITER')->(),        0x80, 'OP_GET_ITER');
    is($C->can('OP_FOR_ITER')->(),        0x81, 'OP_FOR_ITER');
    is($C->can('OP_LOAD_MODULE')->(),     0x90, 'OP_LOAD_MODULE');
    is($C->can('OP_IMPORT_FROM')->(),     0x91, 'OP_IMPORT_FROM');
    is($C->can('OP_PRINT')->(),           0xA0, 'OP_PRINT');
    is($C->can('OP_HALT')->(),            0xFF, 'OP_HALT');
};

# ============================================================================
# Operator Maps
# ============================================================================

subtest 'BINARY_OP_MAP' => sub {
    my $m = $C->binary_op_map();
    is($m->{'+'}, $C->can('OP_ADD')->(),       '+ → ADD');
    is($m->{'-'}, $C->can('OP_SUB')->(),       '- → SUB');
    is($m->{'*'}, $C->can('OP_MUL')->(),       '* → MUL');
    is($m->{'/'}, $C->can('OP_DIV')->(),       '/ → DIV');
    is($m->{'//'}, $C->can('OP_FLOOR_DIV')->(),'// → FLOOR_DIV');
    is($m->{'%'}, $C->can('OP_MOD')->(),       '% → MOD');
    is($m->{'**'}, $C->can('OP_POWER')->(),    '** → POWER');
    is($m->{'&'}, $C->can('OP_BIT_AND')->(),   '& → BIT_AND');
    is($m->{'|'}, $C->can('OP_BIT_OR')->(),    '| → BIT_OR');
    is($m->{'^'}, $C->can('OP_BIT_XOR')->(),   '^ → BIT_XOR');
    is($m->{'<<'}, $C->can('OP_LSHIFT')->(),   '<< → LSHIFT');
    is($m->{'>>'}, $C->can('OP_RSHIFT')->(),   '>> → RSHIFT');
};

subtest 'COMPARE_OP_MAP' => sub {
    my $m = $C->compare_op_map();
    is($m->{'=='}, $C->can('OP_CMP_EQ')->(),     '== → CMP_EQ');
    is($m->{'!='}, $C->can('OP_CMP_NE')->(),     '!= → CMP_NE');
    is($m->{'<'},  $C->can('OP_CMP_LT')->(),     '< → CMP_LT');
    is($m->{'>'}, $C->can('OP_CMP_GT')->(),      '> → CMP_GT');
    is($m->{'<='}, $C->can('OP_CMP_LE')->(),     '<= → CMP_LE');
    is($m->{'>='}, $C->can('OP_CMP_GE')->(),     '>= → CMP_GE');
    is($m->{'in'}, $C->can('OP_CMP_IN')->(),     'in → CMP_IN');
    is($m->{'not in'}, $C->can('OP_CMP_NOT_IN')->(), 'not in → CMP_NOT_IN');
};

# ============================================================================
# Atom literals
# ============================================================================

subtest 'integer literal emits LOAD_CONST' => sub {
    my $tree = anode('file', [
        anode('statement', [
            anode('simple_stmt', [
                anode('expression_stmt', [
                    anode('atom', [ tok('INT', '42') ])
                ])
            ])
        ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[0]{opcode}, $C->can('OP_LOAD_CONST')->(), 'first op is LOAD_CONST');
    is($co->{constants}[0], 42, 'constant pool has 42');
};

subtest 'string literal strips quotes and emits LOAD_CONST' => sub {
    my $tree = anode('file', [
        anode('statement', [
            anode('simple_stmt', [
                anode('expression_stmt', [
                    anode('atom', [ tok('STRING', '"hello"') ])
                ])
            ])
        ])
    ]);
    my $co = compile_tree($tree);
    is($co->{constants}[0], 'hello', 'constant is unquoted string');
};

subtest 'True emits LOAD_TRUE' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [ anode('atom', [ tok('NAME', 'True') ]) ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[0]{opcode}, $C->can('OP_LOAD_TRUE')->(), 'True → LOAD_TRUE');
};

subtest 'False emits LOAD_FALSE' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [ anode('atom', [ tok('NAME', 'False') ]) ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[0]{opcode}, $C->can('OP_LOAD_FALSE')->(), 'False → LOAD_FALSE');
};

subtest 'None emits LOAD_NONE' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [ anode('atom', [ tok('NAME', 'None') ]) ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[0]{opcode}, $C->can('OP_LOAD_NONE')->(), 'None → LOAD_NONE');
};

# ============================================================================
# Identifier
# ============================================================================

subtest 'identifier emits LOAD_NAME' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('identifier', [ tok('NAME', 'x') ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[0]{opcode}, $C->can('OP_LOAD_NAME')->(), 'LOAD_NAME');
    is($co->{names}[0], 'x', 'name pool has x');
};

# ============================================================================
# Assignment
# ============================================================================

subtest 'x = 42 emits LOAD_CONST + STORE_NAME + HALT' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('assign_stmt', [
                anode('identifier', [ tok('NAME', 'x') ]),
                tok('OP', '='),
                anode('atom', [ tok('INT', '42') ]),
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    my $ops = opcodes_of($co);
    is($ops->[0], $C->can('OP_LOAD_CONST')->(), 'LOAD_CONST');
    is($ops->[1], $C->can('OP_STORE_NAME')->(),  'STORE_NAME');
    is($ops->[2], $C->can('OP_HALT')->(),         'HALT');
    is($co->{constants}[0], 42, 'constant = 42');
    is($co->{names}[0], 'x', 'name = x');
};

# ============================================================================
# Arithmetic
# ============================================================================

subtest '1 + 2 emits LOAD_CONST LOAD_CONST ADD' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('arith', [
                    anode('atom', [ tok('INT', '1') ]),
                    tok('OP', '+'),
                    anode('atom', [ tok('INT', '2') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    my $ops = opcodes_of($co);
    is($ops->[0], $C->can('OP_LOAD_CONST')->(), 'LOAD_CONST 1');
    is($ops->[1], $C->can('OP_LOAD_CONST')->(), 'LOAD_CONST 2');
    is($ops->[2], $C->can('OP_ADD')->(),         'ADD');
};

subtest 'a - b emits SUB' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('arith', [
                    anode('atom', [ tok('INT', '5') ]),
                    tok('OP', '-'),
                    anode('atom', [ tok('INT', '3') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[2]{opcode}, $C->can('OP_SUB')->(), 'SUB');
};

subtest 'a * b emits MUL' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('term', [
                    anode('atom', [ tok('INT', '3') ]),
                    tok('OP', '*'),
                    anode('atom', [ tok('INT', '7') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[2]{opcode}, $C->can('OP_MUL')->(), 'MUL');
};

subtest 'a // b emits FLOOR_DIV' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('term', [
                    anode('atom', [ tok('INT', '7') ]),
                    tok('OP', '//'),
                    anode('atom', [ tok('INT', '2') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[2]{opcode}, $C->can('OP_FLOOR_DIV')->(), 'FLOOR_DIV');
};

subtest 'a ** b emits POWER' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('power_expr', [
                    anode('atom', [ tok('INT', '2') ]),
                    tok('OP', '**'),
                    anode('atom', [ tok('INT', '10') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[2]{opcode}, $C->can('OP_POWER')->(), 'POWER');
};

# ============================================================================
# Unary
# ============================================================================

subtest '-x emits LOAD_NAME + NEGATE' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('factor', [
                    tok('OP', '-'),
                    anode('atom', [ tok('NAME', 'x') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[0]{opcode}, $C->can('OP_LOAD_NAME')->(), 'LOAD_NAME');
    is($co->{instructions}[1]{opcode}, $C->can('OP_NEGATE')->(),    'NEGATE');
};

subtest '~x emits BIT_NOT' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('factor', [
                    tok('OP', '~'),
                    anode('atom', [ tok('INT', '5') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[1]{opcode}, $C->can('OP_BIT_NOT')->(), 'BIT_NOT');
};

# ============================================================================
# Comparison
# ============================================================================

subtest 'a == b emits CMP_EQ' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('comparison', [
                    anode('atom', [ tok('INT', '1') ]),
                    tok('OP', '=='),
                    anode('atom', [ tok('INT', '1') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[2]{opcode}, $C->can('OP_CMP_EQ')->(), 'CMP_EQ');
};

subtest 'a < b emits CMP_LT' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('comparison', [
                    anode('atom', [ tok('NAME', 'a') ]),
                    tok('OP', '<'),
                    anode('atom', [ tok('NAME', 'b') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[2]{opcode}, $C->can('OP_CMP_LT')->(), 'CMP_LT');
};

subtest 'not x emits LOGICAL_NOT' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('not_expr', [
                    tok('KW', 'not'),
                    anode('atom', [ tok('NAME', 'x') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[1]{opcode}, $C->can('OP_LOGICAL_NOT')->(), 'LOGICAL_NOT');
};

# ============================================================================
# Boolean short-circuit
# ============================================================================

subtest 'a or b uses JUMP_IF_TRUE_OR_POP' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('or_expr', [
                    anode('atom', [ tok('NAME', 'a') ]),
                    tok('KW', 'or'),
                    anode('atom', [ tok('NAME', 'b') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    my $found = grep { $_->{opcode} == $C->can('OP_JUMP_IF_TRUE_OR_POP')->() }
                @{$co->{instructions}};
    ok($found, 'or_expr emits JUMP_IF_TRUE_OR_POP');
};

subtest 'a and b uses JUMP_IF_FALSE_OR_POP' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('and_expr', [
                    anode('atom', [ tok('NAME', 'a') ]),
                    tok('KW', 'and'),
                    anode('atom', [ tok('NAME', 'b') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    my $found = grep { $_->{opcode} == $C->can('OP_JUMP_IF_FALSE_OR_POP')->() }
                @{$co->{instructions}};
    ok($found, 'and_expr emits JUMP_IF_FALSE_OR_POP');
};

# ============================================================================
# pass_stmt
# ============================================================================

subtest 'pass emits only HALT' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('pass_stmt', [ tok('KW', 'pass') ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is(scalar @{$co->{instructions}}, 1, '1 instruction');
    is($co->{instructions}[0]{opcode}, $C->can('OP_HALT')->(), 'HALT only');
};

# ============================================================================
# return_stmt
# ============================================================================

subtest 'return 42 emits LOAD_CONST + RETURN' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('return_stmt', [
                tok('KW', 'return'),
                anode('atom', [ tok('INT', '42') ]),
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[0]{opcode}, $C->can('OP_LOAD_CONST')->(), 'LOAD_CONST');
    is($co->{instructions}[1]{opcode}, $C->can('OP_RETURN')->(),     'RETURN');
    is($co->{constants}[0], 42, 'constant = 42');
};

subtest 'bare return emits LOAD_NONE + RETURN' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('return_stmt', [ tok('KW', 'return') ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[0]{opcode}, $C->can('OP_LOAD_NONE')->(), 'LOAD_NONE');
    is($co->{instructions}[1]{opcode}, $C->can('OP_RETURN')->(),    'RETURN');
};

# ============================================================================
# Collections
# ============================================================================

subtest 'empty list emits BUILD_LIST 0' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('list_expr', [])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[0]{opcode},  $C->can('OP_BUILD_LIST')->(), 'BUILD_LIST');
    is($co->{instructions}[0]{operand}, 0,                           'count = 0');
};

subtest 'list with 2 elements emits 2 loads + BUILD_LIST 2' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('list_expr', [
                    anode('atom', [ tok('INT', '1') ]),
                    anode('atom', [ tok('INT', '2') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[2]{opcode},  $C->can('OP_BUILD_LIST')->(), 'BUILD_LIST');
    is($co->{instructions}[2]{operand}, 2,                           'count = 2');
};

subtest 'empty dict emits BUILD_DICT 0' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('dict_expr', [])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[0]{opcode},  $C->can('OP_BUILD_DICT')->(), 'BUILD_DICT');
    is($co->{instructions}[0]{operand}, 0,                           'count = 0');
};

subtest 'dict with 1 entry emits key + value + BUILD_DICT 1' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('dict_expr', [
                    anode('dict_entry', [
                        anode('atom', [ tok('STRING', '"k"') ]),
                        tok('OP', ':'),
                        anode('atom', [ tok('INT', '1') ]),
                    ])
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[2]{opcode},  $C->can('OP_BUILD_DICT')->(), 'BUILD_DICT');
    is($co->{instructions}[2]{operand}, 1,                           'count = 1');
};

subtest 'tuple with 2 items emits BUILD_TUPLE 2' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('tuple_expr', [
                    anode('atom', [ tok('INT', '1') ]),
                    anode('atom', [ tok('INT', '2') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[2]{opcode},  $C->can('OP_BUILD_TUPLE')->(), 'BUILD_TUPLE');
    is($co->{instructions}[2]{operand}, 2,                            'count = 2');
};

# ============================================================================
# if_stmt
# ============================================================================

subtest 'if_stmt emits JUMP_IF_FALSE' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('compound_stmt', [
            anode('if_stmt', [
                anode('atom', [ tok('NAME', 'cond') ]),
                anode('suite', [
                    anode('statement', [ anode('simple_stmt', [
                        anode('pass_stmt', [ tok('KW', 'pass') ])
                    ]) ])
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    my $found = grep { $_->{opcode} == $C->can('OP_JUMP_IF_FALSE')->() }
                @{$co->{instructions}};
    ok($found, 'if_stmt emits JUMP_IF_FALSE');
};

# ============================================================================
# for_stmt
# ============================================================================

subtest 'for_stmt emits GET_ITER + FOR_ITER + JUMP' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('compound_stmt', [
            anode('for_stmt', [
                tok('KW', 'for'),
                anode('identifier', [ tok('NAME', 'x') ]),
                tok('KW', 'in'),
                anode('atom', [ tok('NAME', 'items') ]),
                tok('OP', ':'),
                anode('suite', [
                    anode('statement', [ anode('simple_stmt', [
                        anode('pass_stmt', [ tok('KW', 'pass') ])
                    ]) ])
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    my $has_get_iter = grep { $_->{opcode} == $C->can('OP_GET_ITER')->() } @{$co->{instructions}};
    my $has_for_iter = grep { $_->{opcode} == $C->can('OP_FOR_ITER')->() } @{$co->{instructions}};
    my $has_jump     = grep { $_->{opcode} == $C->can('OP_JUMP')->()     } @{$co->{instructions}};
    ok($has_get_iter, 'emits GET_ITER');
    ok($has_for_iter, 'emits FOR_ITER');
    ok($has_jump,     'emits JUMP back');
};

# ============================================================================
# def_stmt
# ============================================================================

subtest 'def f(): pass emits MAKE_FUNCTION + STORE_NAME' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('compound_stmt', [
            anode('def_stmt', [
                tok('KW', 'def'),
                tok('NAME', 'f'),
                tok('OP', '('),
                tok('OP', ')'),
                tok('OP', ':'),
                anode('suite', [
                    anode('statement', [ anode('simple_stmt', [
                        anode('pass_stmt', [ tok('KW', 'pass') ])
                    ]) ])
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    my $has_make_func  = grep { $_->{opcode} == $C->can('OP_MAKE_FUNCTION')->() } @{$co->{instructions}};
    my $has_store_name = grep { $_->{opcode} == $C->can('OP_STORE_NAME')->()    } @{$co->{instructions}};
    ok($has_make_func,  'emits MAKE_FUNCTION');
    ok($has_store_name, 'emits STORE_NAME');
    is($co->{names}[0], 'f', 'function name in name pool');
};

# ============================================================================
# call
# ============================================================================

subtest 'f() emits LOAD_NAME + CALL_FUNCTION 0' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('call', [
                    anode('atom', [ tok('NAME', 'f') ]),
                    tok('OP', '('),
                    tok('OP', ')'),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[0]{opcode},  $C->can('OP_LOAD_NAME')->(),     'LOAD_NAME');
    is($co->{instructions}[1]{opcode},  $C->can('OP_CALL_FUNCTION')->(), 'CALL_FUNCTION');
    is($co->{instructions}[1]{operand}, 0,                               '0 args');
};

subtest 'f(1, 2) emits CALL_FUNCTION 2' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('call', [
                    anode('atom', [ tok('NAME', 'f') ]),
                    tok('OP', '('),
                    anode('call_args', [
                        anode('argument', [ anode('atom', [ tok('INT', '1') ]) ]),
                        anode('argument', [ anode('atom', [ tok('INT', '2') ]) ]),
                    ]),
                    tok('OP', ')'),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[3]{opcode},  $C->can('OP_CALL_FUNCTION')->(), 'CALL_FUNCTION');
    is($co->{instructions}[3]{operand}, 2,                               '2 args');
};

# ============================================================================
# augmented_assign_stmt
# ============================================================================

subtest 'x += 1 emits LOAD_NAME + LOAD_CONST + ADD + STORE_NAME' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('augmented_assign_stmt', [
                anode('identifier', [ tok('NAME', 'x') ]),
                tok('OP', '+='),
                anode('atom', [ tok('INT', '1') ]),
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    my $ops = opcodes_of($co);
    is($ops->[0], $C->can('OP_LOAD_NAME')->(),  'LOAD_NAME x');
    is($ops->[1], $C->can('OP_LOAD_CONST')->(), 'LOAD_CONST 1');
    is($ops->[2], $C->can('OP_ADD')->(),         'ADD');
    is($ops->[3], $C->can('OP_STORE_NAME')->(),  'STORE_NAME x');
};

# ============================================================================
# Bitwise and shift
# ============================================================================

subtest 'a & b emits BIT_AND' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('bitwise_and', [
                    anode('atom', [ tok('NAME', 'a') ]),
                    tok('OP', '&'),
                    anode('atom', [ tok('NAME', 'b') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[2]{opcode}, $C->can('OP_BIT_AND')->(), 'BIT_AND');
};

subtest 'a | b emits BIT_OR' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('bitwise_or', [
                    anode('atom', [ tok('NAME', 'a') ]),
                    tok('OP', '|'),
                    anode('atom', [ tok('NAME', 'b') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[2]{opcode}, $C->can('OP_BIT_OR')->(), 'BIT_OR');
};

subtest 'a << b emits LSHIFT' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('shift', [
                    anode('atom', [ tok('INT', '1') ]),
                    tok('OP', '<<'),
                    anode('atom', [ tok('INT', '4') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    is($co->{instructions}[2]{opcode}, $C->can('OP_LSHIFT')->(), 'LSHIFT');
};

# ============================================================================
# Code object structure
# ============================================================================

subtest 'code_object has instructions, constants, names' => sub {
    my $tree = anode('file', []);
    my $co = compile_tree($tree);
    ok(ref $co->{instructions} eq 'ARRAY', 'instructions is arrayref');
    ok(ref $co->{constants}    eq 'ARRAY', 'constants is arrayref');
    ok(ref $co->{names}        eq 'ARRAY', 'names is arrayref');
};

subtest 'constants are deduplicated' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('expression_stmt', [
                anode('arith', [
                    anode('atom', [ tok('INT', '5') ]),
                    tok('OP', '+'),
                    anode('atom', [ tok('INT', '5') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    my $count = grep { $_ == 5 } @{$co->{constants}};
    is($count, 1, 'constant 5 appears only once');
};

# ============================================================================
# String stripping helper
# ============================================================================

subtest '_strip_quotes helper' => sub {
    is($C->can('_strip_quotes')->('"hello"'), 'hello', 'double quotes');
    is($C->can('_strip_quotes')->("'world'"), 'world', 'single quotes');
    is($C->can('_strip_quotes')->('"""tri"""'), 'tri',  'triple double');
    is($C->can('_strip_quotes')->("'''tri'''"), 'tri',  'triple single');
    is($C->can('_strip_quotes')->('bare'),       'bare', 'unquoted passes through');
    is($C->can('_strip_quotes')->(undef),         '',    'undef returns empty');
};

# ============================================================================
# Integration: x = 1 + 2
# ============================================================================

subtest 'integration: x = 1 + 2' => sub {
    my $tree = anode('file', [
        anode('statement', [ anode('simple_stmt', [
            anode('assign_stmt', [
                anode('identifier', [ tok('NAME', 'x') ]),
                tok('OP', '='),
                anode('arith', [
                    anode('atom', [ tok('INT', '1') ]),
                    tok('OP', '+'),
                    anode('atom', [ tok('INT', '2') ]),
                ])
            ])
        ]) ])
    ]);
    my $co = compile_tree($tree);
    my $ops = opcodes_of($co);
    is($ops->[0], $C->can('OP_LOAD_CONST')->(), 'LOAD_CONST 1');
    is($ops->[1], $C->can('OP_LOAD_CONST')->(), 'LOAD_CONST 2');
    is($ops->[2], $C->can('OP_ADD')->(),         'ADD');
    is($ops->[3], $C->can('OP_STORE_NAME')->(),  'STORE_NAME x');
    is($ops->[4], $C->can('OP_HALT')->(),         'HALT');
    is($co->{constants}[0], 1, 'constants[0] = 1');
    is($co->{constants}[1], 2, 'constants[1] = 2');
    is($co->{names}[0], 'x', 'names[0] = x');
};

# ============================================================================
# compile_ast() convenience API
# ============================================================================

subtest 'compile_ast() convenience API' => sub {
    my $tree = anode('file', []);
    my $co = $C->compile_ast($tree);
    ok(ref $co eq 'HASH', 'returns hashref');
    is($co->{instructions}[0]{opcode}, $C->can('OP_HALT')->(), 'empty file → HALT');
};

done_testing;
