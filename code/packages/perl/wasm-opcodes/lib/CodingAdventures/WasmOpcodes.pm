package CodingAdventures::WasmOpcodes;

# ============================================================================
# CodingAdventures::WasmOpcodes — WebAssembly opcode definitions
# ============================================================================
#
# An "opcode" (operation code) is the numeric identifier for a single
# instruction in a virtual machine's instruction set. WebAssembly's binary
# format encodes each instruction starting with an opcode byte (0x00–0xFF).
#
# ## How WebAssembly Instructions Work
#
# A WebAssembly program is a sequence of instructions. In the binary format
# each instruction is encoded as:
#
#   [opcode_byte] [optional_immediate_bytes...]
#
# The opcode byte identifies WHICH instruction to execute. The immediate bytes
# provide any parameters the instruction needs (e.g., a local variable index,
# a memory offset, or a constant value).
#
# Most WebAssembly instructions use a single opcode byte (0x00–0xFB).
# Extended instruction sets (SIMD, GC, threads) use prefix bytes (0xFC–0xFF)
# followed by additional LEB128 bytes — those are NOT covered here.
#
# ## Instruction Categories
#
# CONTROL FLOW — instructions that change the program counter
#   unreachable (0x00): Always traps. Marks code that cannot be reached.
#   nop (0x01): No operation. Useful for padding or placeholders.
#   block/loop/if: Structured control instructions introducing new label scopes.
#   end (0x0b): Ends any structured control block.
#   br/br_if/br_table: Branch instructions (like goto, but only forward or to loop start).
#   return (0x0f): Return from the current function.
#   call (0x10): Call a function by its module-level index.
#   call_indirect (0x11): Call a function through a table (runtime type check).
#
# PARAMETRIC — stack-manipulation instructions
#   drop (0x1a): Discard the top value.
#   select (0x1b): Choose between two values based on a condition.
#
# VARIABLE — access to local and global variables
#   local.get/set/tee: Read/write local variables (including function params).
#   global.get/set: Read/write global variables.
#
# MEMORY — access to linear memory
#   Loads bring values FROM memory INTO the stack.
#   Stores take values FROM the stack and write THEM to memory.
#   memory.size/grow: Introspect and resize the linear memory.
#
# NUMERIC — arithmetic and comparisons on integer and floating-point types
#   const: Push a constant value.
#   Comparisons produce i32 results (0 = false, 1 = true).
#   Arithmetic wraps on integer overflow (unlike C, no undefined behavior).
#   Conversions are explicit (no implicit coercions in WebAssembly).
#
# ## This Module's Role
#
# This module is a reference/lookup layer. It provides:
#   1. A hash mapping opcode bytes to human-readable names and descriptions.
#   2. Functions to query opcode info: opcode_name, is_valid_opcode, get_opcode_info.
#
# Higher-level tools (disassemblers, validators, JIT compilers) use this data
# to produce error messages, debug output, and documentation.
#
# ## Usage
#
#   use CodingAdventures::WasmOpcodes qw(opcode_name is_valid_opcode get_opcode_info);
#
#   opcode_name(0x00)     # "unreachable"
#   opcode_name(0x6a)     # "i32.add"
#   opcode_name(0x99)     # "unknown_0x99"
#
#   is_valid_opcode(0x01) # 1
#   is_valid_opcode(0x99) # ''
#
#   my $info = get_opcode_info(0x28);
#   # { name => "i32.load", operands => "memarg(align:u32, offset:u32)" }
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(opcode_name is_valid_opcode get_opcode_info);

# ============================================================================
# %OPCODES — The Master Opcode Table
# ============================================================================
#
# Maps byte value → hashref with 'name' and 'operands' fields.
#
# 'name'     — the standard WebAssembly text format mnemonic
# 'operands' — a human-readable description of any immediate bytes that follow
#              the opcode in the binary encoding
#
# Operand notation:
#   "none"                        — no immediates
#   "blocktype"                   — one block type byte (0x40 or ValType)
#   "label:u32"                   — label index, unsigned LEB128
#   "func_idx:u32"                — function index, unsigned LEB128
#   "local_idx:u32"               — local variable index, unsigned LEB128
#   "global_idx:u32"              — global variable index, unsigned LEB128
#   "memarg(align:u32,offset:u32)" — memory alignment hint + byte offset
#   "i32:i32"                     — signed 32-bit LEB128 immediate
#   "i64:i64"                     — signed 64-bit LEB128 immediate
#   "f32:ieee754"                 — 4 bytes IEEE 754 single-precision
#   "f64:ieee754"                 — 8 bytes IEEE 754 double-precision
#   "vec(label:u32)+default:u32"  — br_table: vector of labels + default

our %OPCODES = (

    # ========================================================================
    # Control Flow Instructions
    # ========================================================================

    # unreachable: Unconditional trap. This instruction never completes
    # normally — executing it always raises a WebAssembly trap (runtime error).
    # Used after calls that the compiler knows will never return (e.g., panic).
    0x00 => { name => 'unreachable', operands => 'none' },

    # nop: No operation. Advances the program counter by one byte and does
    # nothing else. Consumes nothing from the value stack, produces nothing.
    0x01 => { name => 'nop', operands => 'none' },

    # block: Begin a "block" structured control instruction.
    # The blocktype tells whether the block produces a value when it exits.
    # A branch to a block label exits the block (forward jump).
    0x02 => { name => 'block', operands => 'blocktype' },

    # loop: Begin a "loop" structured control instruction.
    # Unlike block, the label of a loop points to its START. Branching to a
    # loop label re-runs the loop body. This is how while/for loops are encoded.
    0x03 => { name => 'loop', operands => 'blocktype' },

    # if: Conditional block. Pops an i32 condition. Non-zero executes the
    # "then" arm; zero skips to the else arm or end.
    0x04 => { name => 'if', operands => 'blocktype' },

    # else: Separates the "then" and "else" arms of an if instruction.
    # If the condition was zero, execution jumps here. Only valid inside if.
    0x05 => { name => 'else', operands => 'none' },

    # end: Terminates a block, loop, if, or else. Also implicitly terminates
    # the function body (each function is an implicit block).
    0x0b => { name => 'end', operands => 'none' },

    # br: Unconditional branch to a label. The label index is a depth value:
    #   0 = immediately enclosing block/loop
    #   1 = one level above that, etc.
    # For blocks: branches to the END. For loops: branches to the START.
    0x0c => { name => 'br', operands => 'label:u32' },

    # br_if: Conditional branch. Pops an i32 condition.
    # If non-zero, branches to the label. If zero, falls through to the next
    # instruction. Preserves the stack shape for the fall-through case.
    0x0d => { name => 'br_if', operands => 'label:u32' },

    # br_table: Indexed switch. Pops an i32 index $i.
    # Branches to labels[$i] if $i is within the vector, or to the default label.
    # Used to implement switch/match statements efficiently.
    0x0e => { name => 'br_table', operands => 'vec(label:u32)+default:u32' },

    # return: Return from the current function. Pops the function's result
    # values and transfers control to the caller.
    0x0f => { name => 'return', operands => 'none' },

    # call: Direct function call. Pops arguments, pushes return values.
    # The immediate is the function index in the module's function space.
    0x10 => { name => 'call', operands => 'func_idx:u32' },

    # call_indirect: Call through a function table. Pops a table element
    # index, performs a runtime type check against the given type index, then
    # calls the function. Traps if the type doesn't match or index is out of bounds.
    0x11 => { name => 'call_indirect', operands => 'type_idx:u32 table_idx:u32' },

    # ========================================================================
    # Parametric Instructions
    # ========================================================================

    # drop: Discard the top value of the value stack.
    # Useful when calling a function for its side effects but not its return value.
    0x1a => { name => 'drop', operands => 'none' },

    # select: Conditional selection.
    #   Before: [val1, val2, condition:i32]
    #   After:  [val1]  if condition != 0
    #           [val2]  if condition == 0
    # Both values must have the same type. This is the ternary operator of Wasm.
    0x1b => { name => 'select', operands => 'none' },

    # ========================================================================
    # Variable Instructions
    # ========================================================================

    # local.get: Push the value of local variable at the given index onto
    # the value stack. Local 0..n-1 are the function parameters; n..m are
    # the locally declared variables.
    0x20 => { name => 'local.get', operands => 'local_idx:u32' },

    # local.set: Pop the top of the value stack and store it in local variable
    # at the given index.
    0x21 => { name => 'local.set', operands => 'local_idx:u32' },

    # local.tee: Like local.set but also leaves the value on the stack.
    # ("Tee" — named after a T-pipe fitting that splits one stream into two.)
    0x22 => { name => 'local.tee', operands => 'local_idx:u32' },

    # global.get: Push the current value of the global variable at the given
    # index. Both mutable and immutable globals can be read.
    0x23 => { name => 'global.get', operands => 'global_idx:u32' },

    # global.set: Pop and store a value in the mutable global at the given
    # index. Traps (validation error) if the global is immutable.
    0x24 => { name => 'global.set', operands => 'global_idx:u32' },

    # ========================================================================
    # Memory Load Instructions
    # ========================================================================
    #
    # All load instructions take a "memory argument" — two unsigned LEB128 values:
    #   alignment: log2 of the expected alignment (e.g., 2 = 4-byte aligned)
    #   offset:    constant byte offset added to the runtime address
    #
    # The naming convention is:
    #   i32.load    — load 4 bytes, interpret as i32
    #   i32.load8_s — load 1 byte, sign-extend to i32
    #   i32.load8_u — load 1 byte, zero-extend to i32
    #   i32.load16_s — load 2 bytes, sign-extend to i32
    #   etc.

    0x28 => { name => 'i32.load',    operands => 'memarg(align:u32,offset:u32)' },
    0x29 => { name => 'i64.load',    operands => 'memarg(align:u32,offset:u32)' },
    0x2a => { name => 'f32.load',    operands => 'memarg(align:u32,offset:u32)' },
    0x2b => { name => 'f64.load',    operands => 'memarg(align:u32,offset:u32)' },
    0x2c => { name => 'i32.load8_s', operands => 'memarg(align:u32,offset:u32)' },
    0x2d => { name => 'i32.load8_u', operands => 'memarg(align:u32,offset:u32)' },
    0x2e => { name => 'i32.load16_s',operands => 'memarg(align:u32,offset:u32)' },
    0x2f => { name => 'i32.load16_u',operands => 'memarg(align:u32,offset:u32)' },
    0x30 => { name => 'i64.load8_s', operands => 'memarg(align:u32,offset:u32)' },
    0x31 => { name => 'i64.load8_u', operands => 'memarg(align:u32,offset:u32)' },

    # ========================================================================
    # Memory Store Instructions
    # ========================================================================

    0x36 => { name => 'i32.store',  operands => 'memarg(align:u32,offset:u32)' },
    0x3a => { name => 'i32.store8', operands => 'memarg(align:u32,offset:u32)' },
    0x3b => { name => 'i32.store16',operands => 'memarg(align:u32,offset:u32)' },

    # ========================================================================
    # Memory Size Instructions
    # ========================================================================

    # memory.size: Push current size of linear memory in pages (64 KiB each).
    # Immediate: 0x00 (reserved, must be zero in the MVP).
    0x3f => { name => 'memory.size', operands => 'reserved:u8' },

    # memory.grow: Attempt to grow linear memory by the given number of pages.
    # Pops: delta (number of pages to add).
    # Pushes: previous size (in pages) if successful, or -1 (as i32) if failed.
    0x40 => { name => 'memory.grow', operands => 'reserved:u8' },

    # ========================================================================
    # i32 Numeric Instructions
    # ========================================================================

    # i32.const: Push a 32-bit integer constant. Immediate: signed LEB128.
    0x41 => { name => 'i32.const', operands => 'i32:i32' },

    # i32.eqz: Pop one i32; push 1 if it equals zero, 0 otherwise.
    # The only unary comparison. (There is no "i32.nez" — use eqz + i32.eqz.)
    0x45 => { name => 'i32.eqz', operands => 'none' },

    # i32.eq: Pop two i32s; push 1 if equal, 0 otherwise.
    0x46 => { name => 'i32.eq', operands => 'none' },

    # i32.ne: Pop two i32s; push 1 if not equal, 0 otherwise.
    0x47 => { name => 'i32.ne', operands => 'none' },

    # i32.lt_s: Signed less-than comparison. Interprets both i32s as signed.
    0x48 => { name => 'i32.lt_s', operands => 'none' },

    # i32.add: Add two i32s. Wraps on overflow (modular 2^32 arithmetic).
    # This is deliberate — overflow is defined, not undefined behavior.
    0x6a => { name => 'i32.add', operands => 'none' },

    # i32.sub: Subtract two i32s. Wraps on underflow.
    0x6b => { name => 'i32.sub', operands => 'none' },

    # i32.mul: Multiply two i32s. Wraps on overflow.
    0x6c => { name => 'i32.mul', operands => 'none' },

    # i32.div_s: Signed division. Traps if divisor is zero or if
    # INT_MIN / -1 (signed overflow).
    0x6d => { name => 'i32.div_s', operands => 'none' },

    # i32.and: Bitwise AND.
    0x71 => { name => 'i32.and', operands => 'none' },

    # i32.or: Bitwise inclusive OR.
    0x72 => { name => 'i32.or', operands => 'none' },

    # i32.xor: Bitwise exclusive OR.
    0x73 => { name => 'i32.xor', operands => 'none' },

    # i32.shl: Left shift. Shift amount is taken modulo 32.
    0x74 => { name => 'i32.shl', operands => 'none' },

    # i32.shr_s: Arithmetic (sign-extending) right shift. The sign bit is
    # replicated into the vacated bits. Shift amount is modulo 32.
    0x75 => { name => 'i32.shr_s', operands => 'none' },

    # ========================================================================
    # i64 Numeric Instructions
    # ========================================================================

    # i64.const: Push a 64-bit integer constant. Immediate: signed 64-bit LEB128.
    0x42 => { name => 'i64.const', operands => 'i64:i64' },

    # i64.add: 64-bit wrapping addition.
    0x7c => { name => 'i64.add', operands => 'none' },

    # i64.sub: 64-bit wrapping subtraction.
    0x7d => { name => 'i64.sub', operands => 'none' },

    # i64.mul: 64-bit wrapping multiplication.
    0x7e => { name => 'i64.mul', operands => 'none' },

    # ========================================================================
    # f32 Numeric Instructions
    # ========================================================================

    # f32.const: Push a 32-bit float constant. Immediate: 4 bytes IEEE 754.
    0x43 => { name => 'f32.const', operands => 'f32:ieee754' },

    # f32.add: IEEE 754 single-precision addition.
    0x92 => { name => 'f32.add', operands => 'none' },

    # f32.sub: IEEE 754 single-precision subtraction.
    0x93 => { name => 'f32.sub', operands => 'none' },

    # f32.mul: IEEE 754 single-precision multiplication.
    0x94 => { name => 'f32.mul', operands => 'none' },

    # ========================================================================
    # f64 Numeric Instructions
    # ========================================================================

    # f64.const: Push a 64-bit double constant. Immediate: 8 bytes IEEE 754.
    0x44 => { name => 'f64.const', operands => 'f64:ieee754' },

    # f64.add: IEEE 754 double-precision addition.
    0xa0 => { name => 'f64.add', operands => 'none' },

    # f64.sub: IEEE 754 double-precision subtraction.
    0xa1 => { name => 'f64.sub', operands => 'none' },

    # f64.mul: IEEE 754 double-precision multiplication.
    0xa2 => { name => 'f64.mul', operands => 'none' },

    # ========================================================================
    # Conversion Instructions
    # ========================================================================
    #
    # WebAssembly requires EXPLICIT conversions — there are no implicit type
    # coercions anywhere in the language. This prevents a whole class of subtle
    # bugs common in languages with implicit coercion (JavaScript, C, etc.).

    # i32.wrap_i64: Discard the high 32 bits of an i64, keeping only the low 32.
    # The value wraps modulo 2^32.
    0xa7 => { name => 'i32.wrap_i64', operands => 'none' },

    # i32.trunc_f32_s: Convert f32 to i32 by truncating (rounding toward zero).
    # Signed interpretation. Traps if the value is NaN, infinite, or out of range.
    0xa8 => { name => 'i32.trunc_f32_s', operands => 'none' },

    # i64.extend_i32_s: Sign-extend an i32 to i64.
    # The high 32 bits of the result are filled with copies of i32's sign bit.
    0xac => { name => 'i64.extend_i32_s', operands => 'none' },

    # f32.demote_f64: Convert f64 to f32. May lose precision.
    # Values out of f32 range become ±Infinity. NaN is preserved (possibly with
    # a different payload).
    0xb6 => { name => 'f32.demote_f64', operands => 'none' },

    # f64.promote_f32: Convert f32 to f64. This is exact — every f32 value
    # is exactly representable as f64 (f32 has 24-bit mantissa, f64 has 53-bit).
    0xbb => { name => 'f64.promote_f32', operands => 'none' },
);

# ============================================================================
# opcode_name($byte) — Get the mnemonic name for an opcode byte
# ============================================================================

# Return the human-readable mnemonic for the given opcode byte.
# For known opcodes, returns the standard mnemonic (e.g., "i32.add").
# For unrecognized bytes, returns "unknown_0xXX".
#
# @param  $byte  Integer opcode byte (0x00–0xFF).
# @return        String mnemonic.

sub opcode_name {
    my $byte = ( @_ == 2 ) ? $_[1] : $_[0];
    my $entry = $OPCODES{$byte};
    return $entry ? $entry->{name} : sprintf('unknown_0x%02x', $byte);
}

# ============================================================================
# is_valid_opcode($byte) — Check if a byte is a recognized opcode
# ============================================================================

# Return true (1) if $byte maps to a known WebAssembly opcode.
# Return false ('') otherwise.
#
# @param  $byte  Integer to test.
# @return        1 or ''.

sub is_valid_opcode {
    my $byte = ( @_ == 2 ) ? $_[1] : $_[0];
    return exists $OPCODES{$byte} ? 1 : '';
}

# ============================================================================
# get_opcode_info($byte) — Retrieve full opcode metadata
# ============================================================================

# Return a hash reference with the opcode's name and operands description,
# or undef if the byte is not a recognized opcode.
#
# The returned hashref has keys:
#   name     — string mnemonic (e.g., "i32.add")
#   operands — string describing immediates (e.g., "none", "local_idx:u32")
#
# @param  $byte  Integer opcode byte.
# @return        Hashref {name, operands} or undef.

sub get_opcode_info {
    my $byte = ( @_ == 2 ) ? $_[1] : $_[0];
    return $OPCODES{$byte};
}

1;

__END__

=head1 NAME

CodingAdventures::WasmOpcodes - WebAssembly opcode definitions and lookup

=head1 SYNOPSIS

    use CodingAdventures::WasmOpcodes qw(opcode_name is_valid_opcode get_opcode_info);

    opcode_name(0x6a)      # "i32.add"
    is_valid_opcode(0x00)  # 1
    is_valid_opcode(0x99)  # ''

    my $info = get_opcode_info(0x28);
    # { name => "i32.load", operands => "memarg(align:u32,offset:u32)" }

=head1 DESCRIPTION

A reference table mapping WebAssembly MVP opcode bytes to names and operand
descriptions. Covers control flow, parametric, variable, memory, and numeric
instructions for i32, i64, f32, and f64 types, plus conversion instructions.

=head1 FUNCTIONS

=over 4

=item B<opcode_name($byte)>

Returns the mnemonic string for the opcode, or C<"unknown_0xXX">.

=item B<is_valid_opcode($byte)>

Returns 1 if $byte is a recognized opcode, '' otherwise.

=item B<get_opcode_info($byte)>

Returns a hashref C<{name =E<gt> ..., operands =E<gt> ...}> or undef.

=back

=head1 VARIABLES

=over 4

=item C<%CodingAdventures::WasmOpcodes::OPCODES>

The master opcode table. Keys are byte values (integers), values are hashrefs
with 'name' and 'operands' fields.

=back

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
