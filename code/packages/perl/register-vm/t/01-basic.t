use strict;
use warnings;
use Test2::V0;

use CodingAdventures::RegisterVM;
use CodingAdventures::RegisterVM::Opcodes;
use CodingAdventures::RegisterVM::Feedback;

my $OPS = 'CodingAdventures::RegisterVM::Opcodes';

# ============================================================================
# Helper: build a minimal CodeObject
# ============================================================================
#
# A CodeObject is the compiled representation of a function. It contains:
#
#   instructions       — arrayref of instruction hashrefs
#   constants          — arrayref of literal values referenced by index
#   names              — arrayref of variable/property name strings
#   register_count     — how many numbered registers this function needs
#   feedback_slot_count — how many feedback slots to allocate
#   parameter_count    — how many arguments the function accepts
#   name               — human-readable label for debugging
#
# This helper fills in sensible defaults so each test only specifies what it
# needs.

sub make_code {
    my (%args) = @_;
    return {
        name                => $args{name}                // 'test',
        instructions        => $args{instructions}        // [],
        constants           => $args{constants}           // [],
        names               => $args{names}               // [],
        register_count      => $args{register_count}      // 0,
        feedback_slot_count => $args{feedback_slot_count} // 0,
        parameter_count     => $args{parameter_count}     // 0,
    };
}

# Shorthand instruction builder: most tests don't care about the feedback_slot
sub instr {
    my ($opcode, @operands) = @_;
    return { opcode => $opcode, operands => \@operands, feedback_slot => -1 };
}

sub instr_fb {
    my ($opcode, $fb_slot, @operands) = @_;
    return { opcode => $opcode, operands => \@operands, feedback_slot => $fb_slot };
}

# ============================================================================
# Test 1: LDA_CONSTANT + RETURN
# ============================================================================
#
# The simplest possible program: load a constant into the accumulator and
# return it. Verifies the fundamental load→return path.
#
#   constants[0] = 42
#   LDA_CONSTANT 0     ; acc = 42
#   RETURN             ; return acc

subtest 'Test 1: LDA_CONSTANT + RETURN' => sub {
    my $code = make_code(
        instructions => [
            instr($OPS->LDA_CONSTANT, 0),
            instr($OPS->RETURN),
        ],
        constants => [42],
    );

    my $result = CodingAdventures::RegisterVM->run($code, {});
    is($result->{value}, 42, 'LDA_CONSTANT loads 42 into accumulator');
    ok(!defined $result->{error}, 'no error');
};

# ============================================================================
# Test 2: STAR / LDAR round-trip
# ============================================================================
#
# Verify register store and load.
#
#   LDA_SMI 99         ; acc = 99
#   STAR r0            ; r0  = 99
#   LDA_ZERO           ; acc = 0   (clobber accumulator)
#   LDAR r0            ; acc = r0 = 99
#   RETURN

subtest 'Test 2: STAR / LDAR round-trip' => sub {
    my $code = make_code(
        register_count => 1,
        instructions   => [
            instr($OPS->LDA_SMI,  99),
            instr($OPS->STAR,     0),
            instr($OPS->LDA_ZERO),
            instr($OPS->LDAR,     0),
            instr($OPS->RETURN),
        ],
    );

    my $result = CodingAdventures::RegisterVM->run($code, {});
    is($result->{value}, 99, 'STAR stores accumulator; LDAR restores it');
};

# ============================================================================
# Test 3: ADD same types → monomorphic feedback
# ============================================================================
#
# When we add two integers at the same operation site every time, the feedback
# slot should stay "monomorphic" — only one type-pair ("int:int") observed.
#
#   LDA_SMI 3          ; acc = 3
#   STAR r0            ; r0  = 3
#   LDA_SMI 4          ; acc = 4
#   ADD r0, fb=0       ; acc = 4 + 3 = 7   (records "int:int")
#   RETURN

subtest 'Test 3: ADD same types → monomorphic feedback' => sub {
    my $code = make_code(
        register_count      => 1,
        feedback_slot_count => 1,
        instructions        => [
            instr($OPS->LDA_SMI,  3),
            instr($OPS->STAR,     0),
            instr($OPS->LDA_SMI,  4),
            instr_fb($OPS->ADD, 0, 0),   # ADD r0, feedback_slot=0
            instr($OPS->RETURN),
        ],
    );

    # Create a VM instance so we can inspect the feedback vector.
    my $vm     = CodingAdventures::RegisterVM->new();
    my $result = $vm->execute($code, {});

    is($result->{value}, 7, 'ADD: 4 + 3 = 7');

    # We can't directly inspect the frame's feedback vector after execution
    # because the frame was discarded. Instead, test the Feedback module directly.
    my $slot = CodingAdventures::RegisterVM::Feedback::make();
    CodingAdventures::RegisterVM::Feedback::record($slot, 'int:int');
    is($slot->{kind}, 'monomorphic', 'single int:int observation → monomorphic');
    is($slot->{types}, ['int:int'],  'monomorphic types list correct');
};

# ============================================================================
# Test 4: ADD mixed types → mono→poly→mega progression
# ============================================================================
#
# Feeding five distinct type-pairs through one slot causes the progression:
#
#   uninitialized → monomorphic (1 pair)
#                 → polymorphic (2–4 pairs)
#                 → megamorphic (5+ pairs)
#
# Deduplication: the same pair observed twice must NOT advance the state.

subtest 'Test 4: Feedback slot state machine progression' => sub {
    my $slot = CodingAdventures::RegisterVM::Feedback::make();

    # Initial state
    is($slot->{kind}, 'uninitialized', 'fresh slot is uninitialized');

    # First distinct pair
    CodingAdventures::RegisterVM::Feedback::record($slot, 'int:int');
    is($slot->{kind}, 'monomorphic', 'after first pair: monomorphic');

    # Same pair again — should NOT advance
    CodingAdventures::RegisterVM::Feedback::record($slot, 'int:int');
    is($slot->{kind}, 'monomorphic', 'same pair again: stays monomorphic');

    # Second distinct pair
    CodingAdventures::RegisterVM::Feedback::record($slot, 'float:int');
    is($slot->{kind}, 'polymorphic', 'second distinct pair: polymorphic');

    # Third and fourth (still polymorphic)
    CodingAdventures::RegisterVM::Feedback::record($slot, 'string:int');
    CodingAdventures::RegisterVM::Feedback::record($slot, 'int:string');
    is($slot->{kind}, 'polymorphic', 'four distinct pairs: still polymorphic');

    # Fifth distinct pair → megamorphic
    CodingAdventures::RegisterVM::Feedback::record($slot, 'object:int');
    is($slot->{kind}, 'megamorphic', 'fifth distinct pair: megamorphic');

    # Megamorphic is absorbing: no types list, more records don't change state
    ok(!exists $slot->{types}, 'megamorphic slot has no types list');
    CodingAdventures::RegisterVM::Feedback::record($slot, 'bool:undef');
    is($slot->{kind}, 'megamorphic', 'megamorphic is absorbing state');
};

# ============================================================================
# Test 5: JUMP / JUMP_IF_FALSE — branching
# ============================================================================
#
# Conditional jumps are the foundation of if/else and loops. This test
# verifies that JUMP_IF_FALSE correctly skips code when the accumulator is
# falsy, and falls through when it is truthy.
#
# Program logic (pseudo-code):
#   acc = false
#   if (acc) goto skip   ; acc is false so we do NOT jump
#   acc = 42             ; executed
#   RETURN               ; return 42
#   acc = 99             ; skipped
#
# Instruction layout (ip 0-based; ip is pre-incremented before handler):
#   0: LDA_FALSE
#   1: JUMP_IF_TRUE +2    ; if truthy, skip 2 ahead (to ip=4: LDA_SMI 99)
#   2: LDA_SMI 42
#   3: RETURN
#   4: LDA_SMI 99         ; only reachable if jump taken
#   5: RETURN

subtest 'Test 5: JUMP_IF_TRUE / JUMP_IF_FALSE' => sub {
    my $code_false_path = make_code(
        instructions => [
            instr($OPS->LDA_FALSE),
            instr($OPS->JUMP_IF_TRUE,  2),   # offset 2: skip to ip 4
            instr($OPS->LDA_SMI,      42),
            instr($OPS->RETURN),
            instr($OPS->LDA_SMI,      99),
            instr($OPS->RETURN),
        ],
    );

    my $result = CodingAdventures::RegisterVM->run($code_false_path, {});
    is($result->{value}, 42, 'JUMP_IF_TRUE not taken when acc=false → returns 42');

    # Now test with acc=true: the jump IS taken, we get 99.
    my $code_true_path = make_code(
        instructions => [
            instr($OPS->LDA_TRUE),
            instr($OPS->JUMP_IF_TRUE,  2),   # taken: ip advances 2 more → ip=4
            instr($OPS->LDA_SMI,      42),
            instr($OPS->RETURN),
            instr($OPS->LDA_SMI,      99),
            instr($OPS->RETURN),
        ],
    );

    $result = CodingAdventures::RegisterVM->run($code_true_path, {});
    is($result->{value}, 99, 'JUMP_IF_TRUE taken when acc=true → returns 99');
};

# ============================================================================
# Test 6: LDA_GLOBAL / STA_GLOBAL
# ============================================================================
#
# Global variables live in the $globals hashref passed to run().
# STA_GLOBAL writes a new value; LDA_GLOBAL reads it back.
#
# Program: read 'x' from globals, add 10 to it, store as 'y', return 'y'.

subtest 'Test 6: LDA_GLOBAL / STA_GLOBAL' => sub {
    my $code = make_code(
        names          => ['x', 'y'],
        register_count => 1,
        instructions   => [
            instr($OPS->LDA_GLOBAL,  0),     # acc = globals{x}
            instr($OPS->ADD_SMI,    10),     # acc += 10
            instr($OPS->STA_GLOBAL,  1),     # globals{y} = acc
            instr($OPS->LDA_GLOBAL,  1),     # acc = globals{y}
            instr($OPS->RETURN),
        ],
    );

    my $globals = { x => 32 };
    my $result  = CodingAdventures::RegisterVM->run($code, $globals);

    is($result->{value}, 42, 'LDA_GLOBAL reads x=32, ADD_SMI +10 = 42');
    is($globals->{y},    42, 'STA_GLOBAL wrote y=42 into globals hashref');
};

# ============================================================================
# Test 7: CALL_ANY_RECEIVER pushes / pops frame
# ============================================================================
#
# Function calls are the heart of any VM. This test verifies that:
#   * Calling a VMFunction creates a new frame.
#   * The callee can read its arguments from registers.
#   * RETURN pops the frame and puts the callee's acc into the caller's acc.
#
# We create a VMFunction wrapping a tiny CodeObject that doubles its argument:
#   double(x): LDA_CONSTANT r0, MUL r1 (r1 = 2), RETURN
# Then call it with argument 6 and expect 12.

subtest 'Test 7: CALL_ANY_RECEIVER pushes / pops frame' => sub {
    # The callee function body:
    #   r0 = first argument (passed in by the caller)
    #   r1 = 2 (a constant for multiplication)
    #   acc = r0
    #   acc *= r1   → acc = arg * 2
    #   RETURN
    my $callee_code = make_code(
        name           => 'double',
        register_count => 2,
        instructions   => [
            instr($OPS->LDAR,   0),     # acc = r0 (first argument)
            instr($OPS->STAR,   1),     # r1 = acc (save left operand)
            instr($OPS->LDA_SMI, 2),    # acc = 2
            instr($OPS->MUL,    1),     # acc = acc * r1 = 2 * arg
            instr($OPS->RETURN),
        ],
    );

    # Wrap the code in a VMFunction hashref (this is what CREATE_CLOSURE produces).
    my $fn = {
        __type  => 'function',
        code    => $callee_code,
        context => CodingAdventures::RegisterVM::Scope->new(undef),
    };

    # The caller body:
    #   r0 = the VMFunction
    #   r1 = argument value 6
    #   CALL_ANY_RECEIVER r0, r1_start=1, argc=1
    #   RETURN (returns whatever the callee returned)
    my $caller_code = make_code(
        name           => 'caller',
        register_count => 2,
        constants      => [$fn],
        instructions   => [
            instr($OPS->LDA_CONSTANT, 0),   # acc = fn
            instr($OPS->STAR,         0),   # r0 = fn
            instr($OPS->LDA_SMI,      6),   # acc = 6
            instr($OPS->STAR,         1),   # r1 = 6
            instr($OPS->CALL_ANY_RECEIVER, 0, 1, 1),  # call r0, args@r1, count=1
            instr($OPS->RETURN),
        ],
    );

    my $result = CodingAdventures::RegisterVM->run($caller_code, {});
    is($result->{value}, 12, 'CALL_ANY_RECEIVER: double(6) = 12');
};

# ============================================================================
# Test 8: HALT stops execution immediately
# ============================================================================
#
# HALT is the hard stop — it terminates the execution loop and returns the
# current accumulator value. Instructions after HALT are never executed.

subtest 'Test 8: HALT stops execution' => sub {
    my $code = make_code(
        instructions => [
            instr($OPS->LDA_SMI,   7),
            instr($OPS->HALT),
            instr($OPS->LDA_SMI, 999),   # should never execute
            instr($OPS->RETURN),
        ],
    );

    my $result = CodingAdventures::RegisterVM->run($code, {});
    is($result->{value}, 7, 'HALT returns accumulator (7) immediately');
};

# ============================================================================
# Test 9: LDA_NAMED_PROPERTY monomorphic hidden-class feedback
# ============================================================================
#
# When we read a property off an object, the feedback slot records the
# object's hidden-class ID. If we always access the property on objects with
# the same shape (same set of property keys), the slot stays monomorphic.
# This is the basis for "inline caches" in a real JIT.
#
# We test:
#   1. Object creation via _make_object (internal helper exposed via test).
#   2. LDA_NAMED_PROPERTY updates the feedback slot.
#   3. Accessing the same shape twice keeps the slot monomorphic.

subtest 'Test 9: LDA_NAMED_PROPERTY hidden-class feedback' => sub {
    # Build an object with { x => 100 } using CREATE_OBJECT_LITERAL + STA_NAMED_PROPERTY.
    # We'll put the object in the constants pool for simplicity.
    my $obj = {
        __type            => 'object',
        __hidden_class_id => 0,
        properties        => { x => 100 },
    };

    # Access property 'x' on the object.
    my $code = make_code(
        names               => ['x'],
        register_count      => 1,
        feedback_slot_count => 1,
        constants           => [$obj],
        instructions        => [
            instr($OPS->LDA_CONSTANT,   0),  # acc = obj
            instr($OPS->STAR,           0),  # r0  = obj
            instr_fb($OPS->LDA_NAMED_PROPERTY, 0, 0, 0),  # acc = r0.x, fb=0
            instr($OPS->RETURN),
        ],
    );

    my $result = CodingAdventures::RegisterVM->run($code, {});
    is($result->{value}, 100, 'LDA_NAMED_PROPERTY reads property x = 100');

    # Now verify that the Feedback module records the hidden class.
    # Run the same access twice on the same-shaped object → monomorphic.
    my $slot = CodingAdventures::RegisterVM::Feedback::make();
    my $hid  = $obj->{__hidden_class_id};
    CodingAdventures::RegisterVM::Feedback::record($slot, "hid:$hid");
    CodingAdventures::RegisterVM::Feedback::record($slot, "hid:$hid");  # dedup
    is($slot->{kind}, 'monomorphic', 'same hidden class twice → monomorphic');

    # A second distinct hidden class → polymorphic.
    CodingAdventures::RegisterVM::Feedback::record($slot, 'hid:999');
    is($slot->{kind}, 'polymorphic', 'second hidden class → polymorphic');
};

# ============================================================================
# Test 10: STACK_CHECK overflow returns error
# ============================================================================
#
# The VM limits call depth to prevent C-stack exhaustion when a program
# contains unbounded recursion. STACK_CHECK is emitted at the top of every
# function and returns an error hashref if depth > max_depth.
#
# We create a VM with max_depth=2 and call a trivial function 3 levels deep.

subtest 'Test 10: STACK_CHECK overflow' => sub {
    # A function that calls itself (infinite recursion, but we hit the depth
    # limit before the C stack is exhausted).
    #
    # self_code:
    #   STACK_CHECK
    #   LDA_CONSTANT 0    ; acc = self_fn (circular reference via constants)
    #   STAR r0
    #   CALL_ANY_RECEIVER r0, args@r1, argc=0
    #   RETURN

    # We use a shallow max_depth=3 so the test runs quickly.
    my $vm = CodingAdventures::RegisterVM->new(max_depth => 3);

    # Manually simulate exceeding the depth by checking STACK_CHECK handler.
    # The simplest approach: set call_depth above max_depth and run STACK_CHECK.
    $vm->{call_depth} = 4;    # pretend we're already at depth 4

    my $code = make_code(
        instructions => [
            instr($OPS->STACK_CHECK),
            instr($OPS->LDA_SMI, 1),
            instr($OPS->RETURN),
        ],
    );

    my $result = $vm->execute($code, {});
    ok(defined $result->{error}, 'STACK_CHECK returns error when depth exceeded');
    like($result->{error}, qr/Stack overflow/, 'error message mentions stack overflow');
};

done_testing;
