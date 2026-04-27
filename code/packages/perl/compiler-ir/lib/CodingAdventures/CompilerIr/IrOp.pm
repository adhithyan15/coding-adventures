package CodingAdventures::CompilerIr::IrOp;

# ============================================================================
# CodingAdventures::CompilerIr::IrOp — IR opcode enumeration
# ============================================================================
#
# This module defines every instruction the IR can represent, grouped by
# category.  Each opcode is an integer constant so we can store opcodes in
# hashrefs and compare them cheaply with ==.
#
# ## Design Rules (from the spec)
#
#   1. Existing opcodes never change semantics — only new ones are appended.
#   2. A new opcode is added only when a frontend truly needs it.
#   3. All frontends and backends remain forward-compatible.
#
# ## Opcode Categories
#
#   Constants:    LOAD_IMM, LOAD_ADDR
#   Memory:       LOAD_BYTE, STORE_BYTE, LOAD_WORD, STORE_WORD
#   Arithmetic:   ADD, ADD_IMM, SUB, AND, AND_IMM
#   Comparison:   CMP_EQ, CMP_NE, CMP_LT, CMP_GT
#   Control Flow: LABEL, JUMP, BRANCH_Z, BRANCH_NZ, CALL, RET
#   System:       SYSCALL, HALT
#   Meta:         NOP, COMMENT
#
# ## Integer encoding
#
# Each opcode maps to a unique integer (0..24).  The integers match the Go
# iota sequence so IR text files round-trip identically across language
# implementations.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(op_name parse_op);

# ── Constants ──────────────────────────────────────────────────────────────
# Load an immediate integer value into a register.
#   LOAD_IMM  v0, 42    →  v0 = 42
use constant LOAD_IMM   => 0;

# Load the address of a data label into a register.
#   LOAD_ADDR v0, tape  →  v0 = &tape
use constant LOAD_ADDR  => 1;

# ── Memory ────────────────────────────────────────────────────────────────
# Load a byte from memory (zero-extended): dst = mem[base + offset] & 0xFF
#   LOAD_BYTE v2, v0, v1  →  v2 = mem[v0 + v1] & 0xFF
use constant LOAD_BYTE  => 2;

# Store a byte to memory: mem[base + offset] = src & 0xFF
#   STORE_BYTE v2, v0, v1  →  mem[v0 + v1] = v2 & 0xFF
use constant STORE_BYTE => 3;

# Load a machine word from memory: dst = *(word*)(base + offset)
#   LOAD_WORD v2, v0, v1  →  v2 = *(int*)(v0 + v1)
use constant LOAD_WORD  => 4;

# Store a machine word to memory: *(word*)(base + offset) = src
#   STORE_WORD v2, v0, v1  →  *(int*)(v0 + v1) = v2
use constant STORE_WORD => 5;

# ── Arithmetic ────────────────────────────────────────────────────────────
# Register-register addition: dst = lhs + rhs
#   ADD v3, v1, v2  →  v3 = v1 + v2
use constant ADD        => 6;

# Register-immediate addition: dst = src + immediate
#   ADD_IMM v1, v1, 1  →  v1 = v1 + 1
use constant ADD_IMM    => 7;

# Register-register subtraction: dst = lhs - rhs
#   SUB v3, v1, v2  →  v3 = v1 - v2
use constant SUB        => 8;

# Register-register bitwise AND: dst = lhs & rhs
#   AND v3, v1, v2  →  v3 = v1 & v2
use constant AND        => 9;

# Register-immediate bitwise AND: dst = src & immediate
#   AND_IMM v2, v2, 255  →  v2 = v2 & 0xFF
use constant AND_IMM    => 10;

# ── Comparison ────────────────────────────────────────────────────────────
# Set dst = 1 if lhs == rhs, else 0
#   CMP_EQ v4, v1, v2  →  v4 = (v1 == v2) ? 1 : 0
use constant CMP_EQ     => 11;

# Set dst = 1 if lhs != rhs, else 0
#   CMP_NE v4, v1, v2  →  v4 = (v1 != v2) ? 1 : 0
use constant CMP_NE     => 12;

# Set dst = 1 if lhs < rhs (signed), else 0
#   CMP_LT v4, v1, v2  →  v4 = (v1 < v2) ? 1 : 0
use constant CMP_LT     => 13;

# Set dst = 1 if lhs > rhs (signed), else 0
#   CMP_GT v4, v1, v2  →  v4 = (v1 > v2) ? 1 : 0
use constant CMP_GT     => 14;

# ── Control Flow ──────────────────────────────────────────────────────────
# Define a label at this point in the instruction stream.
# Labels produce no machine code — they just record an address.
#   LABEL loop_start
use constant LABEL      => 15;

# Unconditional jump to a label.
#   JUMP loop_start  →  PC = &loop_start
use constant JUMP       => 16;

# Conditional branch: jump to label if register == 0.
#   BRANCH_Z v2, loop_end  →  if v2 == 0 then PC = &loop_end
use constant BRANCH_Z   => 17;

# Conditional branch: jump to label if register != 0.
#   BRANCH_NZ v2, loop_end  →  if v2 != 0 then PC = &loop_end
use constant BRANCH_NZ  => 18;

# Call a subroutine at the given label. Pushes return address.
#   CALL my_func
use constant CALL       => 19;

# Return from a subroutine. Pops return address.
#   RET
use constant RET        => 20;

# ── System ────────────────────────────────────────────────────────────────
# Invoke a system call. Syscall number is an immediate operand.
# Arguments follow the platform's syscall ABI.
#   SYSCALL 1  →  ecall with a7=1 (write)
use constant SYSCALL    => 21;

# Halt execution. The program terminates.
#   HALT  →  ecall with a7=10 (exit)
use constant HALT       => 22;

# ── Meta ──────────────────────────────────────────────────────────────────
# No operation. Produces a single NOP instruction in the backend.
#   NOP
use constant NOP        => 23;

# A human-readable comment. Produces no machine code.
# Useful for debugging IR output.
#   COMMENT "load tape base address"
use constant COMMENT    => 24;

# ============================================================================
# Name tables — bidirectional mapping between integer codes and text names
# ============================================================================
#
# The text names are used by the IR printer and parser.  They must match
# exactly what the Go implementation uses (verified by round-trip tests).

my %OP_NAMES = (
    LOAD_IMM()   => 'LOAD_IMM',
    LOAD_ADDR()  => 'LOAD_ADDR',
    LOAD_BYTE()  => 'LOAD_BYTE',
    STORE_BYTE() => 'STORE_BYTE',
    LOAD_WORD()  => 'LOAD_WORD',
    STORE_WORD() => 'STORE_WORD',
    ADD()        => 'ADD',
    ADD_IMM()    => 'ADD_IMM',
    SUB()        => 'SUB',
    AND()        => 'AND',
    AND_IMM()    => 'AND_IMM',
    CMP_EQ()     => 'CMP_EQ',
    CMP_NE()     => 'CMP_NE',
    CMP_LT()     => 'CMP_LT',
    CMP_GT()     => 'CMP_GT',
    LABEL()      => 'LABEL',
    JUMP()       => 'JUMP',
    BRANCH_Z()   => 'BRANCH_Z',
    BRANCH_NZ()  => 'BRANCH_NZ',
    CALL()       => 'CALL',
    RET()        => 'RET',
    SYSCALL()    => 'SYSCALL',
    HALT()       => 'HALT',
    NOP()        => 'NOP',
    COMMENT()    => 'COMMENT',
);

my %NAME_TO_OP = reverse %OP_NAMES;

# op_name($opcode) — returns the canonical text name for an opcode integer.
#
# Example:
#   op_name(LOAD_IMM)  →  'LOAD_IMM'
#   op_name(99)        →  'UNKNOWN'
sub op_name {
    my ($op) = @_;
    return $OP_NAMES{$op} // 'UNKNOWN';
}

# parse_op($name) — converts a text opcode name to its integer value.
#
# Returns (integer, 1) on success, or (undef, 0) on unknown name.
# This is the inverse of op_name().
#
# Example:
#   parse_op('ADD_IMM')   →  (7, 1)
#   parse_op('FOOBAR')    →  (undef, 0)
sub parse_op {
    my ($name) = @_;
    if (exists $NAME_TO_OP{$name}) {
        return ($NAME_TO_OP{$name}, 1);
    }
    return (undef, 0);
}

1;

__END__

=head1 NAME

CodingAdventures::CompilerIr::IrOp - IR opcode constants and name table

=head1 SYNOPSIS

  use CodingAdventures::CompilerIr::IrOp qw(op_name parse_op);
  use CodingAdventures::CompilerIr::IrOp ':all';

  my $name = op_name(CodingAdventures::CompilerIr::IrOp::ADD_IMM);  # 'ADD_IMM'
  my ($code, $ok) = parse_op('HALT');  # (22, 1)

=head1 DESCRIPTION

Defines the 25 IR opcodes (0..24) as C<use constant> exports, plus two
helper functions for converting between integer codes and text names.

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
