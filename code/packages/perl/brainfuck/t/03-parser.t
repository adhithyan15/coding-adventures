use Test2::V0;

# ============================================================================
# t/03-parser.t — Tests for CodingAdventures::Brainfuck::Parser
# ============================================================================
#
# This test suite verifies that the Brainfuck recursive descent parser
# correctly builds AST nodes from tokenized Brainfuck source.
#
# The Brainfuck grammar has four rules:
#
#   program     = { instruction } ;
#   instruction = loop | command ;
#   loop        = LOOP_START { instruction } LOOP_END ;
#   command     = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT ;
#
# Test categories:
#   - Empty program
#   - Individual commands
#   - Simple loops
#   - Empty loops
#   - Nested loops
#   - Unmatched brackets die with an error
#   - Canonical program "++[>+<-]"
#   - Comment stripping (lexer integration)

use CodingAdventures::Brainfuck::Parser;

# ============================================================================
# Helper: count nodes of a given type in the AST (recursive).
# ============================================================================

sub count_type {
    my ($node, $target_type) = @_;
    my $count = ($node->{type} eq $target_type) ? 1 : 0;
    if ($node->{children}) {
        for my $child (@{ $node->{children} }) {
            $count += count_type($child, $target_type);
        }
    }
    return $count;
}

# ============================================================================
# Helper: find any node of a given type in the AST (recursive, returns bool).
# ============================================================================

sub has_type {
    my ($node, $target_type) = @_;
    return 1 if $node->{type} eq $target_type;
    if ($node->{children}) {
        for my $child (@{ $node->{children} }) {
            return 1 if has_type($child, $target_type);
        }
    }
    return 0;
}

# ============================================================================
# Test 1: Empty program
# ============================================================================
# An empty source string is a valid Brainfuck program.
# The grammar's { instruction } production allows zero iterations.

subtest 'empty program' => sub {
    my $ast = CodingAdventures::Brainfuck::Parser->parse('');
    is $ast->{type}, 'program', 'Root node type is "program"';
    is scalar(@{ $ast->{children} }), 0, 'Empty program has no instruction children';
};

# ============================================================================
# Test 2: Single command — INC
# ============================================================================

subtest 'single command inc' => sub {
    my $ast = CodingAdventures::Brainfuck::Parser->parse('+');
    is $ast->{type}, 'program', 'Root is program';
    is scalar(@{ $ast->{children} }), 1, 'One instruction';
    my $instr = $ast->{children}[0];
    is $instr->{type}, 'instruction', 'Child is instruction';
    my $cmd = $instr->{children}[0];
    is $cmd->{type}, 'command', 'Instruction contains a command';
    is $cmd->{token}{type}, 'INC', 'Command token type is INC';
};

# ============================================================================
# Test 3: All six command types parse
# ============================================================================

subtest 'all six command types' => sub {
    for my $pair ( ['>', 'RIGHT'], ['<', 'LEFT'], ['+', 'INC'],
                   ['-', 'DEC'],  ['.', 'OUTPUT'], [',', 'INPUT'] ) {
        my ($char, $expected_type) = @$pair;
        my $ast = CodingAdventures::Brainfuck::Parser->parse($char);
        is $ast->{type}, 'program', qq{"$char": root is program};
        ok has_type($ast, 'command'), qq{"$char": AST contains a command node};
        my $cmd = $ast->{children}[0]{children}[0];
        is $cmd->{token}{type}, $expected_type, qq{"$char": command token type is $expected_type};
    }
};

# ============================================================================
# Test 4: Multiple commands
# ============================================================================
# "++>" should produce 3 instruction nodes at the program level.

subtest 'multiple commands' => sub {
    my $ast = CodingAdventures::Brainfuck::Parser->parse('++>');
    is $ast->{type}, 'program', 'Root is program';
    is scalar(@{ $ast->{children} }), 3, '"++>" has 3 instruction children';
};

# ============================================================================
# Test 5: Simple loop "[+]"
# ============================================================================
# A loop containing one command.

subtest 'simple loop' => sub {
    my $ast = CodingAdventures::Brainfuck::Parser->parse('[+]');
    is $ast->{type}, 'program', 'Root is program';
    ok has_type($ast, 'loop'), 'AST contains a loop node';
    is count_type($ast, 'loop'), 1, 'Exactly one loop node';
};

# ============================================================================
# Test 6: Empty loop "[]"
# ============================================================================
# An empty loop is legal Brainfuck. It loops forever if cell != 0, or is a
# no-op if cell == 0. It's commonly used as the "clear cell" idiom [-].

subtest 'empty loop' => sub {
    my $ast = CodingAdventures::Brainfuck::Parser->parse('[]');
    is $ast->{type}, 'program', 'Root is program';
    ok has_type($ast, 'loop'), '[] contains a loop node';
    my $loop = $ast->{children}[0]{children}[0];
    is $loop->{type}, 'loop', 'The inner node is a loop';
    is scalar(@{ $loop->{children} }), 0, 'Empty loop has no instruction children';
};

# ============================================================================
# Test 7: Nested loops "[[+]]"
# ============================================================================

subtest 'nested loops' => sub {
    my $ast = CodingAdventures::Brainfuck::Parser->parse('[[+]]');
    is $ast->{type}, 'program', 'Root is program';
    is count_type($ast, 'loop'), 2, '"[[+]]" contains exactly 2 loop nodes';
};

# ============================================================================
# Test 8: Deeply nested loops "[[[+]]]"
# ============================================================================

subtest 'deeply nested loops' => sub {
    my $ast = CodingAdventures::Brainfuck::Parser->parse('[[[+]]]');
    is count_type($ast, 'loop'), 3, '"[[[+]]]" contains 3 loop nodes';
};

# ============================================================================
# Test 9: Canonical program "++[>+<-]"
# ============================================================================
# Moves 2 from cell 0 to cell 1 via a decrement loop.

subtest 'canonical ++[>+<-]' => sub {
    my $ast = CodingAdventures::Brainfuck::Parser->parse('++[>+<-]');
    is $ast->{type}, 'program', 'Root is program';
    ok has_type($ast, 'loop'), 'Contains a loop';
    ok has_type($ast, 'command'), 'Contains commands';
    # 3 top-level instructions: + + [...]
    is scalar(@{ $ast->{children} }), 3, '3 top-level instructions';
};

# ============================================================================
# Test 10: Unmatched "[" dies with error
# ============================================================================
# "[+" has an opening bracket with no matching ']'.

subtest 'unmatched open bracket dies' => sub {
    my $ok = eval {
        CodingAdventures::Brainfuck::Parser->parse('[+');
        1;
    };
    ok !$ok, 'Parser dies on "[+"';
    like $@, qr/without matching/, 'Error mentions "without matching"';
};

# ============================================================================
# Test 11: Unmatched "]" dies with error
# ============================================================================
# "+]" has a closing bracket with no matching '['.

subtest 'unmatched close bracket dies' => sub {
    my $ok = eval {
        CodingAdventures::Brainfuck::Parser->parse('+]');
        1;
    };
    ok !$ok, 'Parser dies on "+]"';
    like $@, qr/without matching|unexpected/, 'Error mentions unmatched bracket';
};

# ============================================================================
# Test 12: Comments are stripped by the lexer
# ============================================================================
# The parser relies on the lexer to remove comments. A commented program
# should produce the same AST structure as the uncommented version.

subtest 'comments stripped before parsing' => sub {
    my $commented = CodingAdventures::Brainfuck::Parser->parse(
        '++ two increments [loop body >+<-] done'
    );
    my $uncommented = CodingAdventures::Brainfuck::Parser->parse('++[>+<-]');

    is count_type($commented, 'instruction'),
       count_type($uncommented, 'instruction'),
       'Same number of instructions with/without comments';

    is count_type($commented, 'loop'),
       count_type($uncommented, 'loop'),
       'Same number of loops with/without comments';
};

# ============================================================================
# Test 13: Loop node has correct line/col from opening bracket
# ============================================================================

subtest 'loop node line col from bracket' => sub {
    my $ast = CodingAdventures::Brainfuck::Parser->parse('[+]');
    # First instruction child → loop child
    my $instr = $ast->{children}[0];
    my $loop  = $instr->{children}[0];
    is $loop->{type}, 'loop', 'Inner node is a loop';
    is $loop->{line}, 1, 'Loop starts on line 1';
    is $loop->{col},  1, 'Loop starts at col 1';
};

# ============================================================================
# Test 14: Complex multi-loop program
# ============================================================================

subtest 'complex multi-loop program' => sub {
    # +++++[->+<] sets cell 0=5 then moves it to cell 1
    my $ast = CodingAdventures::Brainfuck::Parser->parse('+++++[->+<]');
    is $ast->{type}, 'program', 'Root is program';
    is count_type($ast, 'loop'), 1, 'One loop in program';
    # 5 INC + 1 loop instruction = 6 top-level instructions
    is scalar(@{ $ast->{children} }), 6, '6 top-level instructions';
};

done_testing;
