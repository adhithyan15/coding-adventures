package CodingAdventures::RegisterVM;

# ============================================================================
# CodingAdventures::RegisterVM — Generic register-based VM with accumulator
# ============================================================================
#
# # Register-Based vs. Stack-Based VMs
#
# There are two dominant architectures for bytecode virtual machines:
#
# **Stack-based** (JVM, CPython, .NET CLR):
#   Instructions pop operands from a stack and push results.
#   Simple to implement, compact bytecode, but values move a lot.
#
#   Example (a + b):  LOAD a, LOAD b, ADD    ← result stays on stack
#
# **Register-based** (Lua 5, Dalvik/Android, our VM):
#   Instructions name their operands explicitly by register number.
#   Fewer instructions needed, values stay in place, easier to optimise.
#
#   Example (a + b):  ADD r0, r1             ← result in accumulator
#
# Our VM is an **accumulator-register hybrid** modeled on V8's Ignition:
#
#   * One implicit accumulator register (usually the operation target/result).
#   * N explicit numbered registers (r0..rN-1) per call frame.
#   * Most binary ops use:  acc = acc OP reg[n]
#
# This hybrid is a sweet spot: fewer instruction bytes than pure register
# (because one operand is always implicit), but less data movement than
# pure stack.
#
# # Call Frames
#
# Every function call creates a CallFrame:
#
#   {
#     code            => $code_obj,      # the CodeObject being executed
#     ip              => 0,              # instruction pointer (index into instructions)
#     accumulator     => undef,          # the implicit accumulator register
#     registers       => [...],          # explicit register file (N undef slots)
#     feedback_vector => [...],          # one FeedbackSlot per feedback_slot_count
#     context         => $scope,         # lexical scope chain
#     caller_frame    => $prev_frame,    # stack link (undef for top-level)
#   }
#
# # Feedback Vectors
#
# Each CodeObject declares how many feedback slots it needs. The VM allocates
# a fresh feedback vector (arrayref of slot hashrefs) when a function is
# called. Slots start in the 'uninitialized' state and transition toward
# 'megamorphic' as more type combinations are observed.
#
# A real JIT (V8's TurboFan) reads these vectors to decide which types to
# specialise for. In our interpreter we just record the data.
#
# # Hidden Classes
#
# JavaScript (and our VM) uses *hidden classes* (also called "shapes" or
# "maps") to group objects with the same property layout. Objects with the
# same properties in the same order share a hidden class ID.
#
# When properties are accessed repeatedly on objects of the same hidden class,
# the JIT can hard-code the property offset instead of doing a hash lookup.
# Our implementation uses HIDDEN_CLASS_REGISTRY to assign stable IDs.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.1.0';

use CodingAdventures::RegisterVM::Opcodes;
use CodingAdventures::RegisterVM::Feedback;
use CodingAdventures::RegisterVM::Scope;

# ============================================================================
# Hidden-Class Registry
# ============================================================================
#
# Maps a canonical string (sorted property keys joined by ',') to a stable
# integer ID. Every object is assigned a hidden_class_id when created.
# Objects with the same property set share an ID.

my $NEXT_HIDDEN_CLASS_ID = 0;
my %HIDDEN_CLASS_REGISTRY;    # "key1,key2,key3" => $id

# Assign or look up the hidden class ID for a given property keyset.
# @param @keys   list of property name strings
# @return integer ID
sub _hidden_class_id_for {
    my (@keys) = @_;
    my $canonical = join ',', sort @keys;
    unless (exists $HIDDEN_CLASS_REGISTRY{$canonical}) {
        $HIDDEN_CLASS_REGISTRY{$canonical} = $NEXT_HIDDEN_CLASS_ID++;
    }
    return $HIDDEN_CLASS_REGISTRY{$canonical};
}

# ----------------------------------------------------------------------------
# _make_object(%properties) → $vm_object
#
# Create a VM object hashref with our internal tagging convention:
#
#   __type            => 'object'
#   __hidden_class_id => N
#   properties        => { key => value, ... }
#
# The double-underscore prefix is a convention borrowed from Python's "dunder"
# attributes — it marks implementation-internal fields that user code should
# not collide with.
#
# @param %properties   key/value pairs for the object's properties
# @return hashref
# ----------------------------------------------------------------------------
sub _make_object {
    my (%properties) = @_;
    my $id = _hidden_class_id_for(keys %properties);
    return {
        __type            => 'object',
        __hidden_class_id => $id,
        properties        => {%properties},
    };
}

# ----------------------------------------------------------------------------
# _make_function($code_obj, $context) → $vm_function
#
# Wrap a CodeObject and its captured lexical context into a VMFunction.
# This is what CREATE_CLOSURE produces.
#
# @param $code_obj   CodeObject hashref
# @param $context    Scope object (the captured environment)
# @return hashref
# ----------------------------------------------------------------------------
sub _make_function {
    my ($code_obj, $context) = @_;
    return {
        __type   => 'function',
        code     => $code_obj,
        context  => $context,
    };
}

# ============================================================================
# Dispatch Table
# ============================================================================
#
# Instead of a giant if/elsif chain, we use a hash from opcode integer to
# coderef. This is the "dispatch table" pattern — common in interpreters
# because it compiles to an indirect call rather than a chain of comparisons.
#
# Each handler receives ($vm, $frame, $globals, \@operands, $feedback_slot_idx)
# and returns one of:
#
#   undef               — continue to next instruction
#   { done => 1 }       — HALT: stop the loop, return acc as result
#   { return => 1 }     — RETURN: pop frame, propagate acc to caller
#   { error => $msg }   — fatal error

my %HANDLERS;

# We build the dispatch table lazily at END of file after all handlers are
# defined (Perl processes the file top-to-bottom). The table is installed
# in a BEGIN-like block further down.

# ============================================================================
# Constructor
# ============================================================================

# ----------------------------------------------------------------------------
# new(%args) → $vm
#
# Create a new VM instance. You can configure max_depth to limit call
# stack depth (default 500, matching V8's stack overflow threshold).
#
# @param max_depth   maximum call stack depth before STACK_CHECK fires
# @return blessed hashref
# ----------------------------------------------------------------------------
sub new {
    my ($class, %args) = @_;
    return bless {
        call_depth => 0,
        max_depth  => $args{max_depth} // 500,
    }, $class;
}

# ============================================================================
# Public API
# ============================================================================

# ----------------------------------------------------------------------------
# run($class, $code, $globals) → $result
#
# Class method convenience wrapper. Creates a fresh VM and executes $code.
#
# @param $code     CodeObject hashref
# @param $globals  hashref of global variable name => value
# @return { value => $acc } on success, or { error => $msg } on failure
# ----------------------------------------------------------------------------
sub run {
    my ($class, $code, $globals) = @_;
    my $vm = $class->new();
    return $vm->execute($code, $globals // {});
}

# ----------------------------------------------------------------------------
# execute($self_or_class, $code, $globals) → $result
#
# Execute a CodeObject and return the final accumulator value.
# Can be called as either a class method (creates a fresh VM) or an instance
# method (uses the existing VM instance, preserving call_depth etc.).
#
# The main execution loop:
#   1. Build the initial CallFrame.
#   2. Fetch the current instruction.
#   3. Look up the handler in %HANDLERS.
#   4. Call it and inspect the return value.
#   5. Advance ip and repeat.
#
# @param $code     CodeObject hashref
# @param $globals  hashref
# @return hashref { value => ... } or { error => ... }
# ----------------------------------------------------------------------------
sub execute {
    my ($self_or_class, $code, $globals) = @_;

    # Allow both OO and class-method calling styles.
    my $vm = ref($self_or_class) ? $self_or_class : $self_or_class->new();
    $globals //= {};

    # Build the root call frame.
    my $root_context = CodingAdventures::RegisterVM::Scope->new(undef);
    my $frame = _make_frame($code, $root_context, undef);

    # Main execution loop.
    while (1) {
        my $instructions = $frame->{code}{instructions};
        my $ip           = $frame->{ip};

        # Guard against ip running off the end (shouldn't happen in well-formed
        # bytecode, but we handle it gracefully).
        if ($ip >= scalar @$instructions) {
            last;    # treat as implicit HALT
        }

        my $instr        = $instructions->[$ip];
        my $opcode       = $instr->{opcode};
        my $operands     = $instr->{operands} // [];
        my $fb_slot_idx  = $instr->{feedback_slot} // -1;

        my $handler = $HANDLERS{$opcode};
        unless (defined $handler) {
            return { error => "Unknown opcode: 0x" . sprintf('%02X', $opcode) };
        }

        # Advance ip BEFORE calling the handler so that relative jump offsets
        # are applied correctly: a JUMP with offset 0 would re-execute the
        # same instruction if we added the offset after ip was already
        # incremented to the next position. By pre-advancing, offset 0 skips
        # to the instruction AFTER the jump (a no-op jump), which matches
        # conventional assembler behaviour.
        $frame->{ip}++;

        my $signal = $handler->($vm, $frame, $globals, $operands, $fb_slot_idx);

        # HALT: return accumulator immediately.
        if (defined $signal && $signal->{done}) {
            return { value => $frame->{accumulator} };
        }

        # Error: propagate upward.
        if (defined $signal && $signal->{error}) {
            return $signal;
        }

        # RETURN: pop the current frame and give the accumulator to the caller.
        if (defined $signal && $signal->{return}) {
            my $result_acc = $frame->{accumulator};
            my $caller     = $frame->{caller_frame};
            $vm->{call_depth}--;

            if (!defined $caller) {
                # Returned from the top-level function — we're done.
                return { value => $result_acc };
            }

            # Restore the caller's frame and put our return value in its acc.
            $frame = $caller;
            $frame->{accumulator} = $result_acc;
            next;
        }

        # Frame switch: CALL_ANY_RECEIVER pushes a new frame via this signal.
        if (defined $signal && $signal->{push_frame}) {
            $frame = $signal->{push_frame};
            next;
        }

        # undef signal means "continue" — just loop.
    }

    return { value => $frame->{accumulator} };
}

# ============================================================================
# Internal helpers
# ============================================================================

# Build a CallFrame hashref from a CodeObject.
sub _make_frame {
    my ($code, $context, $caller_frame) = @_;

    # Allocate the register file: register_count slots, all undef.
    my @registers = (undef) x ($code->{register_count} // 0);

    # Allocate the feedback vector: feedback_slot_count fresh uninitialized slots.
    my @feedback_vector = map {
        CodingAdventures::RegisterVM::Feedback::make()
    } 1 .. ($code->{feedback_slot_count} // 0);

    return {
        code            => $code,
        ip              => 0,
        accumulator     => undef,
        registers       => \@registers,
        feedback_vector => \@feedback_vector,
        context         => $context,
        caller_frame    => $caller_frame,
    };
}

# Retrieve a feedback slot by index, or a no-op sentinel if idx is -1.
sub _get_slot {
    my ($frame, $idx) = @_;
    return undef if $idx < 0;
    return $frame->{feedback_vector}[$idx];
}

# ============================================================================
# Opcode Handlers
# ============================================================================
#
# Each handler is a coderef stored in %HANDLERS.
# Signature: ($vm, $frame, $globals, $operands, $fb_slot_idx) → signal or undef

# ------------------------------------------------------------------
# Load accumulator — immediate / special values
# ------------------------------------------------------------------

sub _lda_constant {
    my ($vm, $frame, $globals, $ops) = @_;
    # Load from the constants pool. This is the most common way to get a literal
    # value that doesn't fit in a Small Integer (strings, floats, large ints).
    $frame->{accumulator} = $frame->{code}{constants}[$ops->[0]];
    return undef;
}

sub _lda_zero {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{accumulator} = 0;
    return undef;
}

sub _lda_smi {
    my ($vm, $frame, $globals, $ops) = @_;
    # SMI = Small Integer. Stored directly in the instruction stream.
    $frame->{accumulator} = $ops->[0];
    return undef;
}

sub _lda_undefined {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{accumulator} = undef;
    return undef;
}

sub _lda_null {
    my ($vm, $frame, $globals, $ops) = @_;
    # In JavaScript, null and undefined are distinct. In our Perl runtime we
    # use undef for both, so they compare equal here. A more complete VM would
    # tag the value to distinguish them.
    $frame->{accumulator} = undef;
    return undef;
}

sub _lda_true {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{accumulator} = 1;    # Perl's canonical true
    return undef;
}

sub _lda_false {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{accumulator} = '';    # Perl's canonical false (empty string)
    return undef;
}

# ------------------------------------------------------------------
# Register moves
# ------------------------------------------------------------------

sub _ldar {
    my ($vm, $frame, $globals, $ops) = @_;
    # Load accumulator from register: acc = reg[ops[0]]
    $frame->{accumulator} = $frame->{registers}[$ops->[0]];
    return undef;
}

sub _star {
    my ($vm, $frame, $globals, $ops) = @_;
    # Store accumulator to register: reg[ops[0]] = acc
    $frame->{registers}[$ops->[0]] = $frame->{accumulator};
    return undef;
}

sub _mov {
    my ($vm, $frame, $globals, $ops) = @_;
    # Register-to-register copy without touching the accumulator.
    # ops[0] = destination, ops[1] = source
    $frame->{registers}[$ops->[0]] = $frame->{registers}[$ops->[1]];
    return undef;
}

# ------------------------------------------------------------------
# Global and context variable access
# ------------------------------------------------------------------

sub _lda_global {
    my ($vm, $frame, $globals, $ops) = @_;
    my $name = $frame->{code}{names}[$ops->[0]];
    $frame->{accumulator} = $globals->{$name};
    return undef;
}

sub _sta_global {
    my ($vm, $frame, $globals, $ops) = @_;
    my $name = $frame->{code}{names}[$ops->[0]];
    $globals->{$name} = $frame->{accumulator};
    return undef;
}

sub _lda_context_slot {
    my ($vm, $frame, $globals, $ops) = @_;
    # Walk `depth` parent scopes, read slot index.
    $frame->{accumulator} = CodingAdventures::RegisterVM::Scope::get(
        $frame->{context}, $ops->[0], $ops->[1]
    );
    return undef;
}

sub _sta_context_slot {
    my ($vm, $frame, $globals, $ops) = @_;
    CodingAdventures::RegisterVM::Scope::set(
        $frame->{context}, $ops->[0], $ops->[1], $frame->{accumulator}
    );
    return undef;
}

sub _lda_current_context_slot {
    my ($vm, $frame, $globals, $ops) = @_;
    # depth=0 shorthand
    $frame->{accumulator} = CodingAdventures::RegisterVM::Scope::get(
        $frame->{context}, 0, $ops->[0]
    );
    return undef;
}

sub _sta_current_context_slot {
    my ($vm, $frame, $globals, $ops) = @_;
    CodingAdventures::RegisterVM::Scope::set(
        $frame->{context}, 0, $ops->[0], $frame->{accumulator}
    );
    return undef;
}

# ------------------------------------------------------------------
# Arithmetic
# ------------------------------------------------------------------
#
# All binary ops: acc = acc OP reg[ops[0]]
# We record a type-pair into the feedback slot so a JIT could see what
# types are flowing through this operation.

sub _record_binary_feedback {
    my ($frame, $fb_idx, $left, $right) = @_;
    return unless defined $fb_idx && $fb_idx >= 0;
    my $slot = $frame->{feedback_vector}[$fb_idx];
    return unless defined $slot;
    my $pair = CodingAdventures::RegisterVM::Feedback::type_pair($left, $right);
    CodingAdventures::RegisterVM::Feedback::record($slot, $pair);
}

sub _add {
    my ($vm, $frame, $globals, $ops, $fb) = @_;
    my $right = $frame->{registers}[$ops->[0]];
    _record_binary_feedback($frame, $fb, $frame->{accumulator}, $right);
    $frame->{accumulator} = ($frame->{accumulator} // 0) + ($right // 0);
    return undef;
}

sub _sub {
    my ($vm, $frame, $globals, $ops, $fb) = @_;
    my $right = $frame->{registers}[$ops->[0]];
    _record_binary_feedback($frame, $fb, $frame->{accumulator}, $right);
    $frame->{accumulator} = ($frame->{accumulator} // 0) - ($right // 0);
    return undef;
}

sub _mul {
    my ($vm, $frame, $globals, $ops, $fb) = @_;
    my $right = $frame->{registers}[$ops->[0]];
    _record_binary_feedback($frame, $fb, $frame->{accumulator}, $right);
    $frame->{accumulator} = ($frame->{accumulator} // 0) * ($right // 0);
    return undef;
}

sub _div {
    my ($vm, $frame, $globals, $ops, $fb) = @_;
    my $right = $frame->{registers}[$ops->[0]] // 0;
    _record_binary_feedback($frame, $fb, $frame->{accumulator}, $right);
    return { error => 'Division by zero' } if $right == 0;
    $frame->{accumulator} = ($frame->{accumulator} // 0) / $right;
    return undef;
}

sub _mod {
    my ($vm, $frame, $globals, $ops, $fb) = @_;
    my $right = $frame->{registers}[$ops->[0]] // 0;
    _record_binary_feedback($frame, $fb, $frame->{accumulator}, $right);
    return { error => 'Modulo by zero' } if $right == 0;
    $frame->{accumulator} = ($frame->{accumulator} // 0) % $right;
    return undef;
}

sub _pow {
    my ($vm, $frame, $globals, $ops, $fb) = @_;
    my $right = $frame->{registers}[$ops->[0]];
    _record_binary_feedback($frame, $fb, $frame->{accumulator}, $right);
    $frame->{accumulator} = ($frame->{accumulator} // 0) ** ($right // 0);
    return undef;
}

sub _add_smi {
    my ($vm, $frame, $globals, $ops) = @_;
    # The immediate value comes from the instruction itself, not a register.
    $frame->{accumulator} = ($frame->{accumulator} // 0) + $ops->[0];
    return undef;
}

sub _sub_smi {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{accumulator} = ($frame->{accumulator} // 0) - $ops->[0];
    return undef;
}

sub _negate {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{accumulator} = -($frame->{accumulator} // 0);
    return undef;
}

sub _bitwise_and {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{accumulator} = int($frame->{accumulator} // 0)
                          & int($frame->{registers}[$ops->[0]] // 0);
    return undef;
}

sub _bitwise_or {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{accumulator} = int($frame->{accumulator} // 0)
                          | int($frame->{registers}[$ops->[0]] // 0);
    return undef;
}

sub _bitwise_xor {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{accumulator} = int($frame->{accumulator} // 0)
                          ^ int($frame->{registers}[$ops->[0]] // 0);
    return undef;
}

sub _bitwise_not {
    my ($vm, $frame, $globals, $ops) = @_;
    # Perl's ~ operator on integers gives bitwise complement.
    # We mask to 32-bit signed range to match JavaScript behaviour.
    my $v = int($frame->{accumulator} // 0);
    $frame->{accumulator} = ~$v & 0xFFFF_FFFF;
    return undef;
}

sub _shift_left {
    my ($vm, $frame, $globals, $ops) = @_;
    my $shift = int($frame->{registers}[$ops->[0]] // 0) & 0x1F;  # mod 32
    $frame->{accumulator} = int($frame->{accumulator} // 0) << $shift;
    return undef;
}

sub _shift_right {
    my ($vm, $frame, $globals, $ops) = @_;
    # Arithmetic (signed) right shift
    my $shift = int($frame->{registers}[$ops->[0]] // 0) & 0x1F;
    $frame->{accumulator} = int($frame->{accumulator} // 0) >> $shift;
    return undef;
}

sub _shift_right_logical {
    my ($vm, $frame, $globals, $ops) = @_;
    # Logical (unsigned) right shift — insert zeros from the left.
    my $shift = int($frame->{registers}[$ops->[0]] // 0) & 0x1F;
    my $val   = int($frame->{accumulator} // 0) & 0xFFFF_FFFF;
    $frame->{accumulator} = ($val >> $shift) & 0xFFFF_FFFF;
    return undef;
}

# ------------------------------------------------------------------
# Comparison tests
# ------------------------------------------------------------------

sub _test_equal {
    my ($vm, $frame, $globals, $ops) = @_;
    my $right = $frame->{registers}[$ops->[0]];
    # Loose equality: undef == undef, numbers numerically, strings stringwise.
    # We use Perl's eq/== and handle undef specially.
    my $acc = $frame->{accumulator};
    my $result;
    if (!defined $acc && !defined $right) {
        $result = 1;
    } elsif (!defined $acc || !defined $right) {
        $result = '';
    } else {
        $result = ($acc == $right) ? 1 : '';
    }
    $frame->{accumulator} = $result;
    return undef;
}

sub _test_not_equal {
    my ($vm, $frame, $globals, $ops) = @_;
    _test_equal(@_);
    $frame->{accumulator} = $frame->{accumulator} ? '' : 1;
    return undef;
}

sub _test_strict_equal {
    my ($vm, $frame, $globals, $ops) = @_;
    my $right = $frame->{registers}[$ops->[0]];
    my $acc   = $frame->{accumulator};
    my $result;
    if (!defined $acc && !defined $right) {
        $result = 1;
    } elsif (!defined $acc || !defined $right) {
        $result = '';
    } else {
        # Strict: same ref type OR same string value
        my $acc_ref   = ref($acc)   // '';
        my $right_ref = ref($right) // '';
        if ($acc_ref ne $right_ref) {
            $result = '';
        } elsif ($acc_ref) {
            # Both references: identity check
            $result = ($acc == $right) ? 1 : '';
        } else {
            # Both plain scalars: string equality
            $result = ("$acc" eq "$right") ? 1 : '';
        }
    }
    $frame->{accumulator} = $result;
    return undef;
}

sub _test_strict_not_equal {
    my ($vm, $frame, $globals, $ops) = @_;
    _test_strict_equal(@_);
    $frame->{accumulator} = $frame->{accumulator} ? '' : 1;
    return undef;
}

sub _test_less_than {
    my ($vm, $frame, $globals, $ops) = @_;
    my $right = $frame->{registers}[$ops->[0]] // 0;
    $frame->{accumulator} = (($frame->{accumulator} // 0) < $right) ? 1 : '';
    return undef;
}

sub _test_greater_than {
    my ($vm, $frame, $globals, $ops) = @_;
    my $right = $frame->{registers}[$ops->[0]] // 0;
    $frame->{accumulator} = (($frame->{accumulator} // 0) > $right) ? 1 : '';
    return undef;
}

sub _test_le {
    my ($vm, $frame, $globals, $ops) = @_;
    my $right = $frame->{registers}[$ops->[0]] // 0;
    $frame->{accumulator} = (($frame->{accumulator} // 0) <= $right) ? 1 : '';
    return undef;
}

sub _test_ge {
    my ($vm, $frame, $globals, $ops) = @_;
    my $right = $frame->{registers}[$ops->[0]] // 0;
    $frame->{accumulator} = (($frame->{accumulator} // 0) >= $right) ? 1 : '';
    return undef;
}

sub _test_in {
    my ($vm, $frame, $globals, $ops) = @_;
    # Check if accumulator string is a key in the object at reg[ops[0]].
    my $obj = $frame->{registers}[$ops->[0]];
    if (ref($obj) eq 'HASH' && defined $obj->{properties}) {
        $frame->{accumulator} = exists $obj->{properties}{$frame->{accumulator}} ? 1 : '';
    } else {
        $frame->{accumulator} = '';
    }
    return undef;
}

sub _test_instance_of {
    my ($vm, $frame, $globals, $ops) = @_;
    # Simplified: check if accumulator object's __type matches the constructor tag.
    my $constructor = $frame->{registers}[$ops->[0]];
    my $acc         = $frame->{accumulator};
    if (ref($acc) eq 'HASH' && ref($constructor) eq 'HASH') {
        $frame->{accumulator} = ($acc->{__type} eq ($constructor->{__type} // '')) ? 1 : '';
    } else {
        $frame->{accumulator} = '';
    }
    return undef;
}

sub _test_undetectable {
    my ($vm, $frame, $globals, $ops) = @_;
    # In JS, null and undefined are undetectable (typeof returns "undefined" for null too).
    $frame->{accumulator} = !defined($frame->{accumulator}) ? 1 : '';
    return undef;
}

sub _logical_not {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{accumulator} = $frame->{accumulator} ? '' : 1;
    return undef;
}

sub _type_of {
    my ($vm, $frame, $globals, $ops) = @_;
    my $acc = $frame->{accumulator};
    my $type;
    if (!defined $acc) {
        $type = 'undefined';
    } elsif (ref($acc) eq 'HASH') {
        my $t = $acc->{__type} // '';
        $type = $t eq 'function' ? 'function' : 'object';
    } elsif (ref($acc)) {
        $type = 'object';
    } elsif ($acc =~ /\A[+-]?(?:[0-9]+\.?[0-9]*|[0-9]*\.[0-9]+)\z/) {
        $type = 'number';
    } elsif ($acc eq '1' || $acc eq '') {
        $type = 'boolean';
    } else {
        $type = 'string';
    }
    $frame->{accumulator} = $type;
    return undef;
}

# ------------------------------------------------------------------
# Jumps
# ------------------------------------------------------------------
#
# Jump implementation note:
# The ip was already incremented by 1 in the main loop BEFORE the handler
# runs. So if we want to jump to absolute instruction N, we set ip = N.
# If we want a relative jump of `offset` from the current instruction,
# we set ip += offset (ip is already pointing one past the current instr).
#
# JUMP offset=0 means "go to the instruction after this one" — a no-op.
# JUMP offset=1 skips the next instruction.
# JUMP offset=-2 re-executes the current instruction (loop-back by 1 from next).

sub _jump {
    my ($vm, $frame, $globals, $ops) = @_;
    # Relative jump: add offset to the already-incremented ip.
    $frame->{ip} += $ops->[0];
    return undef;
}

sub _jump_if_true {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{ip} += $ops->[0] if $frame->{accumulator};
    return undef;
}

sub _jump_if_false {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{ip} += $ops->[0] unless $frame->{accumulator};
    return undef;
}

sub _jump_if_null {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{ip} += $ops->[0] unless defined $frame->{accumulator};
    return undef;
}

sub _jump_if_undefined {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{ip} += $ops->[0] unless defined $frame->{accumulator};
    return undef;
}

sub _jump_if_null_or_undefined {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{ip} += $ops->[0] unless defined $frame->{accumulator};
    return undef;
}

sub _jump_if_to_boolean_true {
    my ($vm, $frame, $globals, $ops) = @_;
    # ToBoolean coercion: 0, '', undef, '0' are falsy; everything else truthy.
    $frame->{ip} += $ops->[0] if $frame->{accumulator};
    return undef;
}

sub _jump_if_to_boolean_false {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{ip} += $ops->[0] unless $frame->{accumulator};
    return undef;
}

sub _jump_loop {
    my ($vm, $frame, $globals, $ops) = @_;
    # Same semantics as JUMP; the different opcode signals a loop back-edge
    # to profiling and optimisation tiers.
    $frame->{ip} += $ops->[0];
    return undef;
}

# ------------------------------------------------------------------
# Calls and returns
# ------------------------------------------------------------------

sub _call_any_receiver {
    my ($vm, $frame, $globals, $ops, $fb) = @_;
    # ops[0] = callee register index
    # ops[1] = first argument register index
    # ops[2] = argument count
    my ($callee_reg, $args_start, $arg_count) = @$ops;
    my $callee = $frame->{registers}[$callee_reg];

    unless (ref($callee) eq 'HASH' && ($callee->{__type} // '') eq 'function') {
        return { error => "CALL_ANY_RECEIVER: not a function" };
    }

    # Stack-overflow guard: check call depth before pushing a new frame.
    $vm->{call_depth}++;
    if ($vm->{call_depth} > $vm->{max_depth}) {
        $vm->{call_depth}--;
        return { error => "Stack overflow: max call depth ($vm->{max_depth}) exceeded" };
    }

    # Build the new frame. Arguments go into the first N registers of the
    # callee's frame (matching how most calling conventions work).
    my $callee_code    = $callee->{code};
    my $callee_context = $callee->{context};
    my $new_frame      = _make_frame($callee_code, $callee_context, $frame);

    # Copy argument values from caller's registers into callee's registers.
    for my $i (0 .. ($arg_count - 1)) {
        $new_frame->{registers}[$i] = $frame->{registers}[$args_start + $i];
    }

    # Signal the main loop to switch to the new frame.
    return { push_frame => $new_frame };
}

sub _call_undefined_receiver {
    my ($vm, $frame, $globals, $ops, $fb) = @_;
    # Same as CALL_ANY_RECEIVER; receiver is implicit undef.
    return _call_any_receiver($vm, $frame, $globals, $ops, $fb);
}

sub _call_property {
    my ($vm, $frame, $globals, $ops, $fb) = @_;
    # ops[0] = receiver register, ops[1] = name index,
    # ops[2] = first arg register, ops[3] = arg count
    my ($recv_reg, $name_idx, $args_start, $arg_count) = @$ops;
    my $receiver = $frame->{registers}[$recv_reg];
    my $name     = $frame->{code}{names}[$name_idx];

    my $method;
    if (ref($receiver) eq 'HASH' && defined $receiver->{properties}) {
        $method = $receiver->{properties}{$name};
    }

    unless (ref($method) eq 'HASH' && ($method->{__type} // '') eq 'function') {
        return { error => "CALL_PROPERTY: '$name' is not a function" };
    }

    $vm->{call_depth}++;
    if ($vm->{call_depth} > $vm->{max_depth}) {
        $vm->{call_depth}--;
        return { error => "Stack overflow" };
    }

    my $new_frame = _make_frame($method->{code}, $method->{context}, $frame);
    # Receiver goes into r0 (JavaScript 'this'), then args.
    $new_frame->{registers}[0] = $receiver;
    for my $i (0 .. ($arg_count - 1)) {
        $new_frame->{registers}[$i + 1] = $frame->{registers}[$args_start + $i];
    }

    return { push_frame => $new_frame };
}

sub _construct {
    my ($vm, $frame, $globals, $ops, $fb) = @_;
    # Create a new object, call the constructor, return the object.
    # ops[0] = constructor register, ops[1] = first arg reg, ops[2] = arg count
    my ($ctor_reg, $args_start, $arg_count) = @$ops;
    my $ctor = $frame->{registers}[$ctor_reg];

    unless (ref($ctor) eq 'HASH' && ($ctor->{__type} // '') eq 'function') {
        return { error => "CONSTRUCT: not a constructor function" };
    }

    # Build the new object. The constructor may add properties to it.
    my $new_obj = _make_object();

    $vm->{call_depth}++;
    if ($vm->{call_depth} > $vm->{max_depth}) {
        $vm->{call_depth}--;
        return { error => "Stack overflow" };
    }

    my $new_frame = _make_frame($ctor->{code}, $ctor->{context}, $frame);
    # r0 = 'this' (the newly constructed object), then args
    $new_frame->{registers}[0] = $new_obj;
    for my $i (0 .. ($arg_count - 1)) {
        $new_frame->{registers}[$i + 1] = $frame->{registers}[$args_start + $i];
    }

    # Mark the frame so RETURN knows to give back the new_obj, not the acc.
    $new_frame->{is_constructor} = $new_obj;

    return { push_frame => $new_frame };
}

sub _return {
    my ($vm, $frame, $globals, $ops) = @_;
    # If this was a constructor call, return the constructed object
    # (unless the constructor explicitly returned an object).
    if (defined $frame->{is_constructor}) {
        my $acc = $frame->{accumulator};
        unless (ref($acc) eq 'HASH' && ($acc->{__type} // '') eq 'object') {
            $frame->{accumulator} = $frame->{is_constructor};
        }
    }
    return { return => 1 };
}

sub _suspend_generator { return undef }    # stub
sub _resume_generator  { return undef }    # stub

# ------------------------------------------------------------------
# Property access
# ------------------------------------------------------------------

sub _lda_named_property {
    my ($vm, $frame, $globals, $ops, $fb) = @_;
    # ops[0] = object register, ops[1] = name index
    my $obj  = $frame->{registers}[$ops->[0]];
    my $name = $frame->{code}{names}[$ops->[1]];

    # Record the hidden-class ID into the feedback slot.
    # In a real JIT, this is how inline caches (ICs) remember which shapes
    # they've seen, so they can specialise the property lookup.
    if ($fb >= 0 && defined $frame->{feedback_vector}[$fb]) {
        my $slot = $frame->{feedback_vector}[$fb];
        my $hid  = (ref($obj) eq 'HASH') ? ($obj->{__hidden_class_id} // 'unknown') : 'primitive';
        CodingAdventures::RegisterVM::Feedback::record($slot, "hid:$hid");
    }

    if (ref($obj) eq 'HASH' && defined $obj->{properties}) {
        $frame->{accumulator} = $obj->{properties}{$name};
    } else {
        $frame->{accumulator} = undef;
    }
    return undef;
}

sub _sta_named_property {
    my ($vm, $frame, $globals, $ops, $fb) = @_;
    my $obj  = $frame->{registers}[$ops->[0]];
    my $name = $frame->{code}{names}[$ops->[1]];

    if (ref($obj) eq 'HASH' && defined $obj->{properties}) {
        $obj->{properties}{$name} = $frame->{accumulator};
        # Update the hidden class ID since the property set changed.
        $obj->{__hidden_class_id} = _hidden_class_id_for(keys %{ $obj->{properties} });
    }
    return undef;
}

sub _lda_keyed_property {
    my ($vm, $frame, $globals, $ops) = @_;
    my $obj = $frame->{registers}[$ops->[0]];
    my $key = $frame->{registers}[$ops->[1]];

    if (ref($obj) eq 'HASH' && defined $obj->{properties}) {
        $frame->{accumulator} = $obj->{properties}{$key};
    } elsif (ref($obj) eq 'ARRAY') {
        $frame->{accumulator} = $obj->[$key];
    } else {
        $frame->{accumulator} = undef;
    }
    return undef;
}

sub _sta_keyed_property {
    my ($vm, $frame, $globals, $ops) = @_;
    my $obj = $frame->{registers}[$ops->[0]];
    my $key = $frame->{registers}[$ops->[1]];

    if (ref($obj) eq 'HASH' && defined $obj->{properties}) {
        $obj->{properties}{$key} = $frame->{accumulator};
        $obj->{__hidden_class_id} = _hidden_class_id_for(keys %{ $obj->{properties} });
    } elsif (ref($obj) eq 'ARRAY') {
        $obj->[$key] = $frame->{accumulator};
    }
    return undef;
}

sub _lda_named_property_no_feedback {
    my ($vm, $frame, $globals, $ops) = @_;
    # Same as LDA_NAMED_PROPERTY but skips feedback recording.
    my $obj  = $frame->{registers}[$ops->[0]];
    my $name = $frame->{code}{names}[$ops->[1]];
    if (ref($obj) eq 'HASH' && defined $obj->{properties}) {
        $frame->{accumulator} = $obj->{properties}{$name};
    } else {
        $frame->{accumulator} = undef;
    }
    return undef;
}

sub _sta_named_property_no_feedback {
    my ($vm, $frame, $globals, $ops) = @_;
    my $obj  = $frame->{registers}[$ops->[0]];
    my $name = $frame->{code}{names}[$ops->[1]];
    if (ref($obj) eq 'HASH' && defined $obj->{properties}) {
        $obj->{properties}{$name} = $frame->{accumulator};
        $obj->{__hidden_class_id} = _hidden_class_id_for(keys %{ $obj->{properties} });
    }
    return undef;
}

sub _delete_property_strict {
    my ($vm, $frame, $globals, $ops) = @_;
    my $obj  = $frame->{registers}[$ops->[0]];
    my $name = $frame->{accumulator};
    if (ref($obj) eq 'HASH' && defined $obj->{properties}) {
        delete $obj->{properties}{$name};
        $obj->{__hidden_class_id} = _hidden_class_id_for(keys %{ $obj->{properties} });
        $frame->{accumulator} = 1;
    } else {
        $frame->{accumulator} = '';
    }
    return undef;
}

sub _delete_property_sloppy {
    # Same behaviour as strict in our simplified VM.
    return _delete_property_strict(@_);
}

# ------------------------------------------------------------------
# Object creation
# ------------------------------------------------------------------

sub _create_object_literal {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{accumulator} = _make_object();
    return undef;
}

sub _create_array_literal {
    my ($vm, $frame, $globals, $ops) = @_;
    # Arrays are plain Perl arrayrefs in our VM.
    $frame->{accumulator} = [];
    return undef;
}

sub _create_regexp_literal {
    my ($vm, $frame, $globals, $ops) = @_;
    # ops[0] = index into constants where the pattern string lives.
    my $pattern = $frame->{code}{constants}[$ops->[0]] // '';
    $frame->{accumulator} = eval { qr/$pattern/ } // undef;
    return undef;
}

sub _create_closure {
    my ($vm, $frame, $globals, $ops) = @_;
    # ops[0] = index into constants where the CodeObject lives.
    my $code = $frame->{code}{constants}[$ops->[0]];
    $frame->{accumulator} = _make_function($code, $frame->{context});
    return undef;
}

sub _create_context {
    my ($vm, $frame, $globals, $ops) = @_;
    # Push a new scope on top of the current one.
    $frame->{context} = CodingAdventures::RegisterVM::Scope->new($frame->{context});
    return undef;
}

sub _clone_object {
    my ($vm, $frame, $globals, $ops) = @_;
    my $src = $frame->{registers}[$ops->[0]];
    if (ref($src) eq 'HASH' && defined $src->{properties}) {
        $frame->{accumulator} = _make_object(%{ $src->{properties} });
    } else {
        $frame->{accumulator} = undef;
    }
    return undef;
}

# ------------------------------------------------------------------
# Iterator protocol (stubs — full coroutine support out of scope)
# ------------------------------------------------------------------

sub _get_iterator          { return undef }
sub _call_iterator_step    { return undef }
sub _get_iterator_done     { return undef }
sub _get_iterator_value    { return undef }

# ------------------------------------------------------------------
# Exception handling (simplified — no try/catch frame support)
# ------------------------------------------------------------------

sub _throw {
    my ($vm, $frame, $globals, $ops) = @_;
    return { error => "Exception: " . ($frame->{accumulator} // 'unknown') };
}

sub _rethrow {
    my ($vm, $frame, $globals, $ops) = @_;
    return { error => "Rethrow: " . ($frame->{accumulator} // 'unknown') };
}

# ------------------------------------------------------------------
# Context / module variables
# ------------------------------------------------------------------

sub _push_context {
    my ($vm, $frame, $globals, $ops) = @_;
    $frame->{context} = CodingAdventures::RegisterVM::Scope->new($frame->{context});
    return undef;
}

sub _pop_context {
    my ($vm, $frame, $globals, $ops) = @_;
    if (defined $frame->{context}{parent}) {
        $frame->{context} = $frame->{context}{parent};
    }
    return undef;
}

sub _lda_module_variable { return undef }    # stub
sub _sta_module_variable { return undef }    # stub

# ------------------------------------------------------------------
# Meta-instructions
# ------------------------------------------------------------------

sub _stack_check {
    my ($vm, $frame, $globals, $ops) = @_;
    # This opcode is emitted at the top of every function to detect runaway
    # recursion early (before the C stack overflows).
    if ($vm->{call_depth} > $vm->{max_depth}) {
        return { error => "Stack overflow: max call depth ($vm->{max_depth}) exceeded" };
    }
    return undef;
}

sub _debugger {
    # In a real VM, this would trap into an attached debugger.
    # Here we simply continue execution.
    return undef;
}

sub _halt {
    my ($vm, $frame, $globals, $ops) = @_;
    return { done => 1 };
}

# ============================================================================
# Install the dispatch table
# ============================================================================
#
# We define all handlers above as named subs, then reference them here.
# This separation makes the code readable (handlers grouped by category)
# while still giving us the performance of a direct dispatch table.

%HANDLERS = (
    CodingAdventures::RegisterVM::Opcodes::LDA_CONSTANT()      => \&_lda_constant,
    CodingAdventures::RegisterVM::Opcodes::LDA_ZERO()          => \&_lda_zero,
    CodingAdventures::RegisterVM::Opcodes::LDA_SMI()           => \&_lda_smi,
    CodingAdventures::RegisterVM::Opcodes::LDA_UNDEFINED()     => \&_lda_undefined,
    CodingAdventures::RegisterVM::Opcodes::LDA_NULL()          => \&_lda_null,
    CodingAdventures::RegisterVM::Opcodes::LDA_TRUE()          => \&_lda_true,
    CodingAdventures::RegisterVM::Opcodes::LDA_FALSE()         => \&_lda_false,

    CodingAdventures::RegisterVM::Opcodes::LDAR()              => \&_ldar,
    CodingAdventures::RegisterVM::Opcodes::STAR()              => \&_star,
    CodingAdventures::RegisterVM::Opcodes::MOV()               => \&_mov,

    CodingAdventures::RegisterVM::Opcodes::LDA_GLOBAL()        => \&_lda_global,
    CodingAdventures::RegisterVM::Opcodes::STA_GLOBAL()        => \&_sta_global,
    CodingAdventures::RegisterVM::Opcodes::LDA_CONTEXT_SLOT()  => \&_lda_context_slot,
    CodingAdventures::RegisterVM::Opcodes::STA_CONTEXT_SLOT()  => \&_sta_context_slot,
    CodingAdventures::RegisterVM::Opcodes::LDA_CURRENT_CONTEXT_SLOT() => \&_lda_current_context_slot,
    CodingAdventures::RegisterVM::Opcodes::STA_CURRENT_CONTEXT_SLOT() => \&_sta_current_context_slot,

    CodingAdventures::RegisterVM::Opcodes::ADD()               => \&_add,
    CodingAdventures::RegisterVM::Opcodes::SUB()               => \&_sub,
    CodingAdventures::RegisterVM::Opcodes::MUL()               => \&_mul,
    CodingAdventures::RegisterVM::Opcodes::DIV()               => \&_div,
    CodingAdventures::RegisterVM::Opcodes::MOD()               => \&_mod,
    CodingAdventures::RegisterVM::Opcodes::POW()               => \&_pow,
    CodingAdventures::RegisterVM::Opcodes::ADD_SMI()           => \&_add_smi,
    CodingAdventures::RegisterVM::Opcodes::SUB_SMI()           => \&_sub_smi,
    CodingAdventures::RegisterVM::Opcodes::NEGATE()            => \&_negate,
    CodingAdventures::RegisterVM::Opcodes::BITWISE_AND()       => \&_bitwise_and,
    CodingAdventures::RegisterVM::Opcodes::BITWISE_OR()        => \&_bitwise_or,
    CodingAdventures::RegisterVM::Opcodes::BITWISE_XOR()       => \&_bitwise_xor,
    CodingAdventures::RegisterVM::Opcodes::BITWISE_NOT()       => \&_bitwise_not,
    CodingAdventures::RegisterVM::Opcodes::SHIFT_LEFT()        => \&_shift_left,
    CodingAdventures::RegisterVM::Opcodes::SHIFT_RIGHT()       => \&_shift_right,
    CodingAdventures::RegisterVM::Opcodes::SHIFT_RIGHT_LOGICAL() => \&_shift_right_logical,

    CodingAdventures::RegisterVM::Opcodes::TEST_EQUAL()        => \&_test_equal,
    CodingAdventures::RegisterVM::Opcodes::TEST_NOT_EQUAL()    => \&_test_not_equal,
    CodingAdventures::RegisterVM::Opcodes::TEST_STRICT_EQUAL() => \&_test_strict_equal,
    CodingAdventures::RegisterVM::Opcodes::TEST_STRICT_NOT_EQUAL() => \&_test_strict_not_equal,
    CodingAdventures::RegisterVM::Opcodes::TEST_LESS_THAN()    => \&_test_less_than,
    CodingAdventures::RegisterVM::Opcodes::TEST_GREATER_THAN() => \&_test_greater_than,
    CodingAdventures::RegisterVM::Opcodes::TEST_LE()           => \&_test_le,
    CodingAdventures::RegisterVM::Opcodes::TEST_GE()           => \&_test_ge,
    CodingAdventures::RegisterVM::Opcodes::TEST_IN()           => \&_test_in,
    CodingAdventures::RegisterVM::Opcodes::TEST_INSTANCE_OF()  => \&_test_instance_of,
    CodingAdventures::RegisterVM::Opcodes::TEST_UNDETECTABLE() => \&_test_undetectable,
    CodingAdventures::RegisterVM::Opcodes::LOGICAL_NOT()       => \&_logical_not,
    CodingAdventures::RegisterVM::Opcodes::TYPE_OF()           => \&_type_of,

    CodingAdventures::RegisterVM::Opcodes::JUMP()              => \&_jump,
    CodingAdventures::RegisterVM::Opcodes::JUMP_IF_TRUE()      => \&_jump_if_true,
    CodingAdventures::RegisterVM::Opcodes::JUMP_IF_FALSE()     => \&_jump_if_false,
    CodingAdventures::RegisterVM::Opcodes::JUMP_IF_NULL()      => \&_jump_if_null,
    CodingAdventures::RegisterVM::Opcodes::JUMP_IF_UNDEFINED() => \&_jump_if_undefined,
    CodingAdventures::RegisterVM::Opcodes::JUMP_IF_NULL_OR_UNDEFINED() => \&_jump_if_null_or_undefined,
    CodingAdventures::RegisterVM::Opcodes::JUMP_IF_TO_BOOLEAN_TRUE()   => \&_jump_if_to_boolean_true,
    CodingAdventures::RegisterVM::Opcodes::JUMP_IF_TO_BOOLEAN_FALSE()  => \&_jump_if_to_boolean_false,
    CodingAdventures::RegisterVM::Opcodes::JUMP_LOOP()         => \&_jump_loop,

    CodingAdventures::RegisterVM::Opcodes::CALL_ANY_RECEIVER()       => \&_call_any_receiver,
    CodingAdventures::RegisterVM::Opcodes::CALL_PROPERTY()           => \&_call_property,
    CodingAdventures::RegisterVM::Opcodes::CALL_UNDEFINED_RECEIVER() => \&_call_undefined_receiver,
    CodingAdventures::RegisterVM::Opcodes::CONSTRUCT()               => \&_construct,
    CodingAdventures::RegisterVM::Opcodes::RETURN()                  => \&_return,
    CodingAdventures::RegisterVM::Opcodes::SUSPEND_GENERATOR()       => \&_suspend_generator,
    CodingAdventures::RegisterVM::Opcodes::RESUME_GENERATOR()        => \&_resume_generator,

    CodingAdventures::RegisterVM::Opcodes::LDA_NAMED_PROPERTY()      => \&_lda_named_property,
    CodingAdventures::RegisterVM::Opcodes::STA_NAMED_PROPERTY()      => \&_sta_named_property,
    CodingAdventures::RegisterVM::Opcodes::LDA_KEYED_PROPERTY()      => \&_lda_keyed_property,
    CodingAdventures::RegisterVM::Opcodes::STA_KEYED_PROPERTY()      => \&_sta_keyed_property,
    CodingAdventures::RegisterVM::Opcodes::LDA_NAMED_PROPERTY_NO_FEEDBACK() => \&_lda_named_property_no_feedback,
    CodingAdventures::RegisterVM::Opcodes::STA_NAMED_PROPERTY_NO_FEEDBACK() => \&_sta_named_property_no_feedback,
    CodingAdventures::RegisterVM::Opcodes::DELETE_PROPERTY_STRICT()  => \&_delete_property_strict,
    CodingAdventures::RegisterVM::Opcodes::DELETE_PROPERTY_SLOPPY()  => \&_delete_property_sloppy,

    CodingAdventures::RegisterVM::Opcodes::CREATE_OBJECT_LITERAL()   => \&_create_object_literal,
    CodingAdventures::RegisterVM::Opcodes::CREATE_ARRAY_LITERAL()    => \&_create_array_literal,
    CodingAdventures::RegisterVM::Opcodes::CREATE_REGEXP_LITERAL()   => \&_create_regexp_literal,
    CodingAdventures::RegisterVM::Opcodes::CREATE_CLOSURE()          => \&_create_closure,
    CodingAdventures::RegisterVM::Opcodes::CREATE_CONTEXT()          => \&_create_context,
    CodingAdventures::RegisterVM::Opcodes::CLONE_OBJECT()            => \&_clone_object,

    CodingAdventures::RegisterVM::Opcodes::GET_ITERATOR()            => \&_get_iterator,
    CodingAdventures::RegisterVM::Opcodes::CALL_ITERATOR_STEP()      => \&_call_iterator_step,
    CodingAdventures::RegisterVM::Opcodes::GET_ITERATOR_DONE()       => \&_get_iterator_done,
    CodingAdventures::RegisterVM::Opcodes::GET_ITERATOR_VALUE()      => \&_get_iterator_value,

    CodingAdventures::RegisterVM::Opcodes::THROW()                   => \&_throw,
    CodingAdventures::RegisterVM::Opcodes::RETHROW()                 => \&_rethrow,

    CodingAdventures::RegisterVM::Opcodes::PUSH_CONTEXT()            => \&_push_context,
    CodingAdventures::RegisterVM::Opcodes::POP_CONTEXT()             => \&_pop_context,
    CodingAdventures::RegisterVM::Opcodes::LDA_MODULE_VARIABLE()     => \&_lda_module_variable,
    CodingAdventures::RegisterVM::Opcodes::STA_MODULE_VARIABLE()     => \&_sta_module_variable,

    CodingAdventures::RegisterVM::Opcodes::STACK_CHECK()             => \&_stack_check,
    CodingAdventures::RegisterVM::Opcodes::DEBUGGER()                => \&_debugger,
    CodingAdventures::RegisterVM::Opcodes::HALT()                    => \&_halt,
);

1;

__END__

=head1 NAME

CodingAdventures::RegisterVM - Generic register-based VM with accumulator model

=head1 VERSION

0.1.0

=head1 SYNOPSIS

    use CodingAdventures::RegisterVM;
    use CodingAdventures::RegisterVM::Opcodes;

    my $OPS = 'CodingAdventures::RegisterVM::Opcodes';

    my $code = {
        name               => 'add_example',
        instructions       => [
            { opcode => $OPS->LDA_SMI,  operands => [3],  feedback_slot => -1 },
            { opcode => $OPS->STAR,     operands => [0],  feedback_slot => -1 },
            { opcode => $OPS->LDA_SMI,  operands => [4],  feedback_slot => -1 },
            { opcode => $OPS->ADD,      operands => [0],  feedback_slot =>  0 },
            { opcode => $OPS->RETURN,   operands => [],   feedback_slot => -1 },
        ],
        constants          => [],
        names              => [],
        register_count     => 1,
        feedback_slot_count => 1,
        parameter_count    => 0,
    };

    my $result = CodingAdventures::RegisterVM->run($code, {});
    print $result->{value};    # 7

=head1 DESCRIPTION

A register-based virtual machine with an accumulator model, inspired by V8's
Ignition bytecode interpreter. Features:

=over 4

=item * ~70 opcodes covering arithmetic, logic, property access, calls, jumps

=item * Feedback vectors for type-profiling (uninitialized → monomorphic → polymorphic → megamorphic)

=item * Hidden-class IDs for object shape tracking

=item * Lexical scope chain for closures

=item * Call-depth limit (configurable, default 500)

=back

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
