package CodingAdventures::RegisterVM::Opcodes;

# ============================================================================
# CodingAdventures::RegisterVM::Opcodes — Opcode constant definitions
# ============================================================================
#
# # What is an Opcode?
#
# An "opcode" (operation code) is a numeric identifier that tells the VM
# which operation to perform. Real CPUs use the same idea: the ARM64 and
# x86-64 processors each have a fixed binary encoding for every instruction
# they support.
#
# Here we model the bytecode instruction set of a register-based VM inspired
# by V8's Ignition bytecode compiler (the engine that runs JavaScript in
# Chrome and Node.js). Ignition was chosen as inspiration because:
#
#   1. It is a clean accumulator-register hybrid (simpler than pure register).
#   2. It has feedback vectors that enable adaptive optimisation.
#   3. Its design is documented in public V8 blog posts and talks.
#
# # Opcode Grouping Convention
#
# We use the high nibble of the byte to group related opcodes:
#
#   0x00–0x0F  Load accumulator (immediate / special values)
#   0x10–0x1F  Register ↔ accumulator moves
#   0x20–0x2F  Global / context variable access
#   0x30–0x3F  Arithmetic and bitwise operations
#   0x40–0x4F  Comparison and logical tests
#   0x50–0x5F  Jumps and branches
#   0x60–0x6F  Calls, returns, and coroutines
#   0x70–0x7F  Property load/store
#   0x80–0x8F  Object/array/closure creation
#   0x90–0x9F  Iterator protocol
#   0xA0–0xAF  Exception handling
#   0xB0–0xBF  Context / module variable access
#   0xF0–0xFF  Meta-instructions (stack check, debugger, halt)
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.1.0';

use constant {

    # ------------------------------------------------------------------
    # 0x00–0x0F : Load Accumulator — put a value into the accumulator
    # ------------------------------------------------------------------
    #
    # The accumulator is a special implicit register. Almost all operations
    # read from or write to it. Think of it like the "A" register on a 6502:
    # most operations go through it.

    # LDA_CONSTANT idx — load constants[idx] into the accumulator
    LDA_CONSTANT => 0x00,

    # LDA_ZERO — load the integer 0 (saves a constants-pool slot for the
    #            most common literal)
    LDA_ZERO => 0x01,

    # LDA_SMI value — load a Small Integer directly from the instruction
    #                 stream (avoids a constants-pool lookup for small ints)
    LDA_SMI => 0x02,

    # LDA_UNDEFINED — load Perl undef (JavaScript's undefined)
    LDA_UNDEFINED => 0x03,

    # LDA_NULL — load undef (JavaScript's null, distinguished by type tag)
    LDA_NULL => 0x04,

    # LDA_TRUE / LDA_FALSE — load boolean 1 / empty-string ''
    LDA_TRUE  => 0x05,
    LDA_FALSE => 0x06,

    # ------------------------------------------------------------------
    # 0x10–0x1F : Register Moves
    # ------------------------------------------------------------------
    #
    # Registers are numbered 0..register_count-1 and stored in the
    # frame's 'registers' arrayref.

    # LDAR reg — Load Accumulator from Register: acc = reg[operand[0]]
    LDAR => 0x10,

    # STAR reg — Store Accumulator to Register: reg[operand[0]] = acc
    STAR => 0x11,

    # MOV dst, src — copy register-to-register without touching the accumulator
    MOV => 0x12,

    # ------------------------------------------------------------------
    # 0x20–0x2F : Global and Context Variables
    # ------------------------------------------------------------------
    #
    # "Globals" live in the $globals hashref passed to run().
    # "Context slots" live in a lexical scope chain (CodingAdventures::RegisterVM::Scope).

    # LDA_GLOBAL name_idx — acc = globals{names[name_idx]}
    LDA_GLOBAL => 0x20,

    # STA_GLOBAL name_idx — globals{names[name_idx]} = acc
    STA_GLOBAL => 0x21,

    # LDA_CONTEXT_SLOT depth, idx — walk `depth` parent scopes, read slot idx
    LDA_CONTEXT_SLOT => 0x22,

    # STA_CONTEXT_SLOT depth, idx — walk `depth` parent scopes, write slot idx
    STA_CONTEXT_SLOT => 0x23,

    # LDA_CURRENT_CONTEXT_SLOT idx — shorthand for depth=0
    LDA_CURRENT_CONTEXT_SLOT => 0x24,

    # STA_CURRENT_CONTEXT_SLOT idx — shorthand for depth=0
    STA_CURRENT_CONTEXT_SLOT => 0x25,

    # ------------------------------------------------------------------
    # 0x30–0x3F : Arithmetic and Bitwise Operations
    # ------------------------------------------------------------------
    #
    # All arithmetic ops use the accumulator as the LEFT operand and write
    # the result back to the accumulator. The RIGHT operand comes from the
    # register specified in operands[0].
    #
    # Pattern: acc = acc OP reg[operands[0]]
    #
    # This is the "accumulator model" — one side is always implicit.

    # ADD reg — acc = acc + reg[operands[0]]
    ADD => 0x30,
    # SUB reg — acc = acc - reg[operands[0]]
    SUB => 0x31,
    # MUL reg — acc = acc * reg[operands[0]]
    MUL => 0x32,
    # DIV reg — acc = acc / reg[operands[0]]
    DIV => 0x33,
    # MOD reg — acc = acc % reg[operands[0]]
    MOD => 0x34,
    # POW reg — acc = acc ** reg[operands[0]]
    POW => 0x35,

    # ADD_SMI imm — acc = acc + operands[0]  (avoids register lookup)
    ADD_SMI => 0x36,
    # SUB_SMI imm — acc = acc - operands[0]
    SUB_SMI => 0x37,

    # NEGATE — acc = -acc  (unary negation)
    NEGATE => 0x38,

    # Bitwise operations on integers
    BITWISE_AND         => 0x39,
    BITWISE_OR          => 0x3A,
    BITWISE_XOR         => 0x3B,
    BITWISE_NOT         => 0x3C,
    SHIFT_LEFT          => 0x3D,
    SHIFT_RIGHT         => 0x3E,
    SHIFT_RIGHT_LOGICAL => 0x3F,

    # ------------------------------------------------------------------
    # 0x40–0x4F : Comparison and Logical Tests
    # ------------------------------------------------------------------
    #
    # Each test compares acc against reg[operands[0]] and writes a
    # boolean (1 or '') back into acc.
    #
    # The "strict" variants require same type (like JavaScript ===).
    # Our Perl implementation checks ref() equality for strict tests.

    TEST_EQUAL            => 0x40,
    TEST_NOT_EQUAL        => 0x41,
    TEST_STRICT_EQUAL     => 0x42,
    TEST_STRICT_NOT_EQUAL => 0x43,
    TEST_LESS_THAN        => 0x44,
    TEST_GREATER_THAN     => 0x45,
    TEST_LE               => 0x46,
    TEST_GE               => 0x47,

    # TEST_IN — check if acc is a key in the object at reg[operands[0]]
    TEST_IN => 0x48,

    # TEST_INSTANCE_OF — check prototype chain (simplified: check __type tag)
    TEST_INSTANCE_OF => 0x49,

    # TEST_UNDETECTABLE — true for null/undefined (document.all quirk in JS)
    TEST_UNDETECTABLE => 0x4A,

    # LOGICAL_NOT — acc = !acc (boolean negation, writes 1 or '')
    LOGICAL_NOT => 0x4B,

    # TYPE_OF — acc = type string ('number', 'string', 'boolean', 'undefined',
    #                               'object', 'function')
    TYPE_OF => 0x4C,

    # ------------------------------------------------------------------
    # 0x50–0x5F : Jumps and Branches
    # ------------------------------------------------------------------
    #
    # All jumps use a RELATIVE offset stored in operands[0].
    # The instruction pointer (ip) is incremented by that offset AFTER the
    # current instruction executes.
    #
    # A positive offset jumps forward; a negative offset jumps backward
    # (enabling loops). JUMP_LOOP is semantically identical to JUMP but
    # signals to an optimising tier that this is a loop back-edge.

    # JUMP offset — unconditional relative jump
    JUMP => 0x50,

    # JUMP_IF_TRUE offset — jump if acc is truthy
    JUMP_IF_TRUE => 0x51,

    # JUMP_IF_FALSE offset — jump if acc is falsy
    JUMP_IF_FALSE => 0x52,

    # JUMP_IF_NULL — jump if acc is undef (null semantics)
    JUMP_IF_NULL => 0x53,

    # JUMP_IF_UNDEFINED — jump if acc is undef (undefined semantics)
    JUMP_IF_UNDEFINED => 0x54,

    # JUMP_IF_NULL_OR_UNDEFINED — combined null/undefined check
    JUMP_IF_NULL_OR_UNDEFINED => 0x55,

    # JUMP_IF_TO_BOOLEAN_TRUE — ToBoolean coercion then jump if truthy
    JUMP_IF_TO_BOOLEAN_TRUE => 0x56,

    # JUMP_IF_TO_BOOLEAN_FALSE — ToBoolean coercion then jump if falsy
    JUMP_IF_TO_BOOLEAN_FALSE => 0x57,

    # JUMP_LOOP offset — loop back-edge (same semantics as JUMP)
    JUMP_LOOP => 0x58,

    # ------------------------------------------------------------------
    # 0x60–0x6F : Calls, Returns, and Coroutines
    # ------------------------------------------------------------------

    # CALL_ANY_RECEIVER callee_reg, args_start_reg, arg_count
    #   Calls the function stored in reg[callee_reg] with arg_count arguments
    #   starting at reg[args_start_reg].
    CALL_ANY_RECEIVER => 0x60,

    # CALL_PROPERTY receiver_reg, name_idx, args_start_reg, arg_count
    #   Looks up a method on the receiver object and calls it.
    CALL_PROPERTY => 0x61,

    # CALL_UNDEFINED_RECEIVER callee_reg, args_start_reg, arg_count
    #   Same as CALL_ANY_RECEIVER but receiver is undefined (e.g., free functions)
    CALL_UNDEFINED_RECEIVER => 0x62,

    # CONSTRUCT callee_reg, args_start_reg, arg_count
    #   new-expression: creates object, calls constructor, returns object
    CONSTRUCT => 0x63,

    # RETURN — pop the current call frame, set parent acc to current acc
    RETURN => 0x64,

    # SUSPEND_GENERATOR / RESUME_GENERATOR — coroutine support
    SUSPEND_GENERATOR => 0x65,
    RESUME_GENERATOR  => 0x66,

    # ------------------------------------------------------------------
    # 0x70–0x7F : Property Access
    # ------------------------------------------------------------------
    #
    # Properties are accessed on object hashrefs by name (string key) or
    # by computed key (keyed access).
    #
    # The feedback slot records which hidden-class IDs have been seen at
    # this access site, enabling inline-cache optimisation in a real JIT.

    # LDA_NAMED_PROPERTY obj_reg, name_idx, feedback_slot
    #   acc = reg[obj_reg].properties{names[name_idx]}
    LDA_NAMED_PROPERTY => 0x70,

    # STA_NAMED_PROPERTY obj_reg, name_idx, feedback_slot
    #   reg[obj_reg].properties{names[name_idx]} = acc
    STA_NAMED_PROPERTY => 0x71,

    # LDA_KEYED_PROPERTY obj_reg, key_reg
    #   acc = reg[obj_reg]{reg[key_reg]}
    LDA_KEYED_PROPERTY => 0x72,

    # STA_KEYED_PROPERTY obj_reg, key_reg
    #   reg[obj_reg]{reg[key_reg]} = acc
    STA_KEYED_PROPERTY => 0x73,

    # *_NO_FEEDBACK variants skip feedback-slot recording (for cold paths)
    LDA_NAMED_PROPERTY_NO_FEEDBACK => 0x74,
    STA_NAMED_PROPERTY_NO_FEEDBACK => 0x75,

    # DELETE_PROPERTY_STRICT / _SLOPPY — remove a property from an object
    DELETE_PROPERTY_STRICT => 0x76,
    DELETE_PROPERTY_SLOPPY => 0x77,

    # ------------------------------------------------------------------
    # 0x80–0x8F : Object Creation
    # ------------------------------------------------------------------

    # CREATE_OBJECT_LITERAL — create a new empty object
    CREATE_OBJECT_LITERAL => 0x80,

    # CREATE_ARRAY_LITERAL — create a new empty array (arrayref)
    CREATE_ARRAY_LITERAL => 0x81,

    # CREATE_REGEXP_LITERAL — create a compiled qr// regex object
    CREATE_REGEXP_LITERAL => 0x82,

    # CREATE_CLOSURE code_idx — wrap a CodeObject into a VMFunction with
    #                           the current context (creates a closure)
    CREATE_CLOSURE => 0x83,

    # CREATE_CONTEXT — push a new lexical scope on top of the current one
    CREATE_CONTEXT => 0x84,

    # CLONE_OBJECT src_reg — shallow-copy the object at reg[src_reg]
    CLONE_OBJECT => 0x85,

    # ------------------------------------------------------------------
    # 0x90–0x9F : Iterator Protocol
    # ------------------------------------------------------------------

    # GET_ITERATOR — call [Symbol.iterator]() on acc; store iterator in acc
    GET_ITERATOR => 0x90,

    # CALL_ITERATOR_STEP iterator_reg — call .next() on the iterator
    CALL_ITERATOR_STEP => 0x91,

    # GET_ITERATOR_DONE — acc = iterator result .done property
    GET_ITERATOR_DONE => 0x92,

    # GET_ITERATOR_VALUE — acc = iterator result .value property
    GET_ITERATOR_VALUE => 0x93,

    # ------------------------------------------------------------------
    # 0xA0–0xAF : Exception Handling
    # ------------------------------------------------------------------

    # THROW — throw the value in acc as an exception
    THROW => 0xA0,

    # RETHROW — rethrow the current exception (from a catch block)
    RETHROW => 0xA1,

    # ------------------------------------------------------------------
    # 0xB0–0xBF : Context and Module Variables
    # ------------------------------------------------------------------

    PUSH_CONTEXT        => 0xB0,
    POP_CONTEXT         => 0xB1,
    LDA_MODULE_VARIABLE => 0xB2,
    STA_MODULE_VARIABLE => 0xB3,

    # ------------------------------------------------------------------
    # 0xF0–0xFF : Meta-Instructions
    # ------------------------------------------------------------------

    # STACK_CHECK — verify call depth hasn't exceeded max_depth
    STACK_CHECK => 0xF0,

    # DEBUGGER — pause execution (no-op in this implementation; a real VM
    #            would signal a debugger attachment here)
    DEBUGGER => 0xFE,

    # HALT — stop the execution loop immediately; acc is the final result
    HALT => 0xFF,
};

1;

__END__

=head1 NAME

CodingAdventures::RegisterVM::Opcodes - Opcode constants for the register VM

=head1 SYNOPSIS

    use CodingAdventures::RegisterVM::Opcodes;

    my $instr = {
        opcode       => CodingAdventures::RegisterVM::Opcodes::LDA_CONSTANT,
        operands     => [0],
        feedback_slot => -1,
    };

=head1 DESCRIPTION

Defines all ~70 opcode constants used by C<CodingAdventures::RegisterVM>.
Opcodes are grouped by their high nibble into functional categories.

=cut
