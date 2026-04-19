use strict;
use warnings;
use Test2::V0;

# Load the module
ok( eval { require CodingAdventures::VirtualMachine; 1 }, 'VirtualMachine module loads' )
    or diag($@);

# Convenience aliases
my $VM     = 'CodingAdventures::VirtualMachine';
my $Code   = 'CodingAdventures::VirtualMachine::CodeObject';

# Helper: build a CodeObject
sub make_code {
    my (%args) = @_;
    return $Code->new(
        instructions => $args{instructions} || [],
        constants    => $args{constants}    || [],
        names        => $args{names}        || [],
    );
}

# Helper: instruction hashref
sub instr {
    my ($opcode, $operand) = @_;
    return { opcode => $opcode, operand => $operand };
}

# ===== Test 1: VM instantiation =====
{
    my $vm = $VM->new();
    ok( defined $vm,                    'new() returns a VM' );
    is( ref($vm), $VM,                  'new() returns correct class' );
    is( $vm->pc,       0,               'initial pc is 0' );
    is( $vm->halted,   0,               'initial halted is false' );
    is( scalar(@{ $vm->stack }),  0,    'initial stack is empty' );
    is( scalar(@{ $vm->output }), 0,    'initial output is empty' );
}

# ===== Test 2: LOAD_CONST and HALT =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x01, 0), instr(0xFF) ],
        constants    => [42],
    );
    $vm->execute($c);
    is( scalar(@{ $vm->stack }), 1,  'LOAD_CONST pushes onto stack' );
    is( $vm->stack->[0],         42, 'LOAD_CONST pushes correct value' );
    is( $vm->halted,             1,  'HALT sets halted flag' );
}

# ===== Test 3: POP =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x01, 0), instr(0x02), instr(0xFF) ],
        constants    => [99],
    );
    $vm->execute($c);
    is( scalar(@{ $vm->stack }), 0, 'POP removes value from stack' );
}

# ===== Test 4: DUP =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x01, 0), instr(0x03), instr(0xFF) ],
        constants    => [7],
    );
    $vm->execute($c);
    is( scalar(@{ $vm->stack }), 2, 'DUP results in two items on stack' );
    is( $vm->stack->[0], 7, 'DUP: original value preserved' );
    is( $vm->stack->[1], 7, 'DUP: duplicate is identical' );
}

# ===== Regression: execute_with_context follows code switches =====
{
    my $vm = $VM->new();
    my $caller = make_code(instructions => [ instr(0xAA) ]);
    my $callee = make_code(instructions => [ instr(0xAB) ]);
    my $ctx = {
        caller_hits => 0,
        callee_hits => 0,
        callee      => $callee,
    };

    $vm->register_context_opcode(0xAA, sub {
        my ($vm, $instr, $code, $context) = @_;
        $context->{caller_hits}++;
        die "stuck in caller loop\n" if $context->{caller_hits} > 1;
        $vm->{_program} = $context->{callee};
        $vm->{pc} = 0;
    });

    $vm->register_context_opcode(0xAB, sub {
        my ($vm, $instr, $code, $context) = @_;
        $context->{callee_hits}++;
        $vm->{halted} = 1;
    });

    ok(eval { $vm->execute_with_context($caller, $ctx); 1 }, 'context execution follows code switches')
        or diag($@);
    is($ctx->{caller_hits}, 1, 'caller executed once');
    is($ctx->{callee_hits}, 1, 'callee executed after the switch');
}

# ===== Test 5: ADD =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x01, 0), instr(0x01, 1), instr(0x20), instr(0xFF) ],
        constants    => [10, 20],
    );
    $vm->execute($c);
    is( $vm->stack->[0], 30, 'ADD: 10 + 20 = 30' );
}

# ===== Test 6: SUB =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x01, 0), instr(0x01, 1), instr(0x21), instr(0xFF) ],
        constants    => [50, 20],
    );
    $vm->execute($c);
    is( $vm->stack->[0], 30, 'SUB: 50 - 20 = 30' );
}

# ===== Test 7: MUL =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x01, 0), instr(0x01, 1), instr(0x22), instr(0xFF) ],
        constants    => [6, 7],
    );
    $vm->execute($c);
    is( $vm->stack->[0], 42, 'MUL: 6 * 7 = 42' );
}

# ===== Test 8: DIV =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x01, 0), instr(0x01, 1), instr(0x23), instr(0xFF) ],
        constants    => [15, 4],
    );
    $vm->execute($c);
    is( $vm->stack->[0], 3, 'DIV: int(15/4) = 3' );
}

# ===== Test 9: DIV by zero raises error =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x01, 0), instr(0x01, 1), instr(0x23), instr(0xFF) ],
        constants    => [10, 0],
    );
    my $err;
    eval { $vm->execute($c) };
    $err = $@;
    ok( ref($err) && $err->isa('CodingAdventures::VirtualMachine::DivisionByZeroError'),
        'DIV by zero raises DivisionByZeroError' );
}

# ===== Test 10: CMP_EQ (equal) =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x01, 0), instr(0x01, 0), instr(0x30), instr(0xFF) ],
        constants    => [5],
    );
    $vm->execute($c);
    is( $vm->stack->[0], 1, 'CMP_EQ: equal values returns 1' );
}

# ===== Test 11: CMP_EQ (unequal) =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x01, 0), instr(0x01, 1), instr(0x30), instr(0xFF) ],
        constants    => [5, 6],
    );
    $vm->execute($c);
    is( $vm->stack->[0], 0, 'CMP_EQ: unequal values returns 0' );
}

# ===== Test 12: CMP_LT =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x01, 0), instr(0x01, 1), instr(0x31), instr(0xFF) ],
        constants    => [3, 5],
    );
    $vm->execute($c);
    is( $vm->stack->[0], 1, 'CMP_LT: 3 < 5 returns 1' );
}

# ===== Test 13: CMP_GT =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x01, 0), instr(0x01, 1), instr(0x32), instr(0xFF) ],
        constants    => [10, 5],
    );
    $vm->execute($c);
    is( $vm->stack->[0], 1, 'CMP_GT: 10 > 5 returns 1' );
}

# ===== Test 14: STORE_NAME / LOAD_NAME =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [
            instr(0x01, 0),   # LOAD_CONST 100
            instr(0x10, 0),   # STORE_NAME 'x'
            instr(0x11, 0),   # LOAD_NAME  'x'
            instr(0xFF),      # HALT
        ],
        constants => [100],
        names     => ['x'],
    );
    $vm->execute($c);
    is( $vm->variables->{'x'}, 100, 'STORE_NAME stores value' );
    is( $vm->stack->[0],       100, 'LOAD_NAME pushes value onto stack' );
}

# ===== Test 15: STORE_LOCAL / LOAD_LOCAL =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [
            instr(0x01, 0),   # LOAD_CONST 77
            instr(0x12, 0),   # STORE_LOCAL slot 0
            instr(0x13, 0),   # LOAD_LOCAL  slot 0
            instr(0xFF),
        ],
        constants => [77],
    );
    $vm->execute($c);
    is( $vm->stack->[0], 77, 'STORE_LOCAL / LOAD_LOCAL round-trip' );
}

# ===== Test 16: JUMP unconditional =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [
            instr(0x40, 2),   # JUMP to index 2
            instr(0x01, 0),   # LOAD_CONST 999 (should be skipped)
            instr(0xFF),      # HALT
        ],
        constants => [999],
    );
    $vm->execute($c);
    is( scalar(@{ $vm->stack }), 0, 'JUMP skipped LOAD_CONST, stack is empty' );
}

# ===== Test 17: JUMP_IF_FALSE (condition is false) =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [
            instr(0x01, 0),   # index 0: LOAD_CONST 0  (falsy)
            instr(0x41, 3),   # index 1: JUMP_IF_FALSE to 3
            instr(0x01, 1),   # index 2: LOAD_CONST 99 (should be skipped)
            instr(0xFF),      # index 3: HALT
        ],
        constants => [0, 99],
    );
    $vm->execute($c);
    is( scalar(@{ $vm->stack }), 0, 'JUMP_IF_FALSE jumped when condition was 0' );
}

# ===== Test 18: JUMP_IF_TRUE (condition is true) =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [
            instr(0x01, 0),   # index 0: LOAD_CONST 1  (truthy)
            instr(0x42, 3),   # index 1: JUMP_IF_TRUE to 3
            instr(0x01, 1),   # index 2: LOAD_CONST 99 (skipped)
            instr(0xFF),      # index 3: HALT
        ],
        constants => [1, 99],
    );
    $vm->execute($c);
    is( scalar(@{ $vm->stack }), 0, 'JUMP_IF_TRUE jumped when condition was 1' );
}

# ===== Test 19: PRINT =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [
            instr(0x01, 0),   # LOAD_CONST 'hello'
            instr(0x60),      # PRINT
            instr(0xFF),
        ],
        constants => ['hello'],
    );
    $vm->execute($c);
    is( scalar(@{ $vm->output }), 1,       'PRINT produces one output entry' );
    is( $vm->output->[0],          'hello', 'PRINT captures correct value' );
    is( scalar(@{ $vm->stack }),   0,       'PRINT pops from stack' );
}

# ===== Test 20: Stack underflow raises error =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x02) ],  # POP on empty stack
    );
    my $err;
    eval { $vm->execute($c) };
    $err = $@;
    ok( ref($err) && $err->isa('CodingAdventures::VirtualMachine::StackUnderflowError'),
        'POP on empty stack raises StackUnderflowError' );
}

# ===== Test 21: Unknown opcode raises error =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0xAA) ],  # invalid opcode
    );
    my $err;
    eval { $vm->execute($c) };
    $err = $@;
    ok( ref($err) && $err->isa('CodingAdventures::VirtualMachine::InvalidOpcodeError'),
        'Unknown opcode raises InvalidOpcodeError' );
}

# ===== Test 22: LOAD_NAME undefined raises error =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x11, 0) ],
        names        => ['undefined_var'],
    );
    my $err;
    eval { $vm->execute($c) };
    $err = $@;
    ok( ref($err) && $err->isa('CodingAdventures::VirtualMachine::UndefinedNameError'),
        'LOAD_NAME undefined variable raises UndefinedNameError' );
}

# ===== Test 23: execute() returns traces =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x01, 0), instr(0xFF) ],
        constants    => [5],
    );
    my $traces = $vm->execute($c);
    ok( ref($traces) eq 'ARRAY', 'execute() returns arrayref' );
    is( scalar(@$traces), 2,     'execute() returns one trace per instruction' );
    is( $traces->[0]{pc}, 0,     'first trace has pc=0' );
}

# ===== Test 24: step() returns VMTrace =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x01, 0), instr(0xFF) ],
        constants    => [123],
    );
    my $trace = $vm->step($c);
    ok( ref($trace) && $trace->isa('CodingAdventures::VirtualMachine::VMTrace'),
        'step() returns a VMTrace object' );
    is( $trace->pc, 0,  'trace pc is 0 for first instruction' );
    ok( defined $trace->description, 'trace has a description' );
    is( scalar(@{ $trace->stack_before }), 0, 'stack_before is empty at start' );
    is( scalar(@{ $trace->stack_after }),  1, 'stack_after has one element' );
}

# ===== Test 25: load() and run() API =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [ instr(0x01, 0), instr(0x60), instr(0xFF) ],
        constants    => ['world'],
    );
    $vm->load($c);
    $vm->run();
    is( $vm->output->[0], 'world', 'load()+run() API works' );
}

# ===== Test 26: registers() returns summary =====
{
    my $vm = $VM->new();
    my $regs = $vm->registers();
    ok( defined $regs,          'registers() returns a value' );
    ok( exists $regs->{pc},     'registers() includes pc' );
    ok( exists $regs->{halted}, 'registers() includes halted' );
    ok( exists $regs->{stack},  'registers() includes stack' );
}

# ===== Test 27: Multiple PRINT outputs =====
{
    my $vm = $VM->new();
    my $c  = make_code(
        instructions => [
            instr(0x01, 0), instr(0x60),   # PRINT 1
            instr(0x01, 1), instr(0x60),   # PRINT 2
            instr(0xFF),
        ],
        constants => [1, 2],
    );
    $vm->execute($c);
    is( scalar(@{ $vm->output }), 2,   'two PRINT calls produce two outputs' );
    is( $vm->output->[0],         '1', 'first output is 1' );
    is( $vm->output->[1],         '2', 'second output is 2' );
}

# ===== Test 28: CodeObject constructor =====
{
    my $co = $Code->new(
        instructions => [],
        constants    => [1, 2, 3],
        names        => ['x', 'y'],
    );
    ok( defined $co,                    'CodeObject constructs' );
    is( scalar(@{ $co->constants }), 3, 'constants stored correctly' );
    is( scalar(@{ $co->names }),     2, 'names stored correctly' );
}

done_testing;
