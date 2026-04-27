package CodingAdventures::CompilerIr;

# ============================================================================
# CodingAdventures::CompilerIr — General-purpose IR for the AOT compiler
# ============================================================================
#
# This package provides the intermediate representation (IR) used by the
# coding-adventures AOT compiler pipeline.  It is a Perl port of the Go
# package at code/packages/go/compiler-ir/.
#
# ## What is an IR?
#
# An intermediate representation sits between the source language and the
# target machine code.  It is:
#
#   Source (Brainfuck)
#        ↓  [brainfuck-ir-compiler]
#      IR (this package)
#        ↓  [compiler-ir-optimizer]  ← optional passes
#      IR (optimised)
#        ↓  [codegen-riscv]
#   Machine code (ELF binary)
#
# By compiling to a common IR, we can support multiple frontends (Brainfuck,
# BASIC, ...) and multiple backends (RISC-V, ARM, x86, ...) without an
# N×M explosion of compiler passes.
#
# ## IR Properties
#
#   - Linear: no basic blocks, no SSA, no phi nodes
#   - Register-based: infinite virtual registers (v0, v1, ...)
#   - Target-independent: backends map IR to physical ISA
#   - Versioned: .version directive in text format (v1 = Brainfuck subset)
#
# ## Modules in this package
#
#   IrOp          — opcode constants (LOAD_IMM, ADD_IMM, HALT, ...)
#   IrRegister    — virtual register operand (v0, v1, ...)
#   IrImmediate   — literal integer operand (42, -1, 255, ...)
#   IrLabel       — named label operand (_start, loop_0_end, tape, ...)
#   IrInstruction — a single IR instruction (opcode + operands + ID)
#   IrDataDecl    — a data segment declaration (.data tape 30000 0)
#   IrProgram     — the complete IR program (instructions + data + entry)
#   IDGenerator   — monotonic unique instruction ID counter
#   Printer       — IrProgram → canonical text
#   Parser        — canonical text → IrProgram
#
# ## Quick start
#
#   use CodingAdventures::CompilerIr qw(print_ir parse_ir);
#   use CodingAdventures::CompilerIr::IrOp;
#   use CodingAdventures::CompilerIr::IrProgram;
#   use CodingAdventures::CompilerIr::IrInstruction;
#   use CodingAdventures::CompilerIr::IrRegister;
#   use CodingAdventures::CompilerIr::IrImmediate;
#
#   my $prog = CodingAdventures::CompilerIr::IrProgram->new('_start');
#   my $instr = CodingAdventures::CompilerIr::IrInstruction->new(
#       opcode   => CodingAdventures::CompilerIr::IrOp::HALT,
#       operands => [],
#       id       => 0,
#   );
#   $prog->add_instruction($instr);
#   print print_ir($prog);
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(print_ir parse_ir);

# Load all sub-modules so callers can use CodingAdventures::CompilerIr and
# then refer to the sub-module namespaces without additional use statements.
use CodingAdventures::CompilerIr::IrOp;
use CodingAdventures::CompilerIr::IrRegister;
use CodingAdventures::CompilerIr::IrImmediate;
use CodingAdventures::CompilerIr::IrLabel;
use CodingAdventures::CompilerIr::IrInstruction;
use CodingAdventures::CompilerIr::IrDataDecl;
use CodingAdventures::CompilerIr::IrProgram;
use CodingAdventures::CompilerIr::IDGenerator;
use CodingAdventures::CompilerIr::Printer qw(print_ir);
use CodingAdventures::CompilerIr::Parser  qw(parse_ir);

# Re-export the two most commonly used functions at the top level.
# Callers can do:
#   use CodingAdventures::CompilerIr qw(print_ir parse_ir);

1;

__END__

=head1 NAME

CodingAdventures::CompilerIr - General-purpose IR for the AOT compiler pipeline

=head1 VERSION

0.01

=head1 SYNOPSIS

  use CodingAdventures::CompilerIr qw(print_ir parse_ir);
  use CodingAdventures::CompilerIr::IrOp;
  use CodingAdventures::CompilerIr::IrProgram;
  use CodingAdventures::CompilerIr::IrInstruction;
  use CodingAdventures::CompilerIr::IrRegister;
  use CodingAdventures::CompilerIr::IrImmediate;
  use CodingAdventures::CompilerIr::IDGenerator;

  my $gen  = CodingAdventures::CompilerIr::IDGenerator->new;
  my $prog = CodingAdventures::CompilerIr::IrProgram->new('_start');

  $prog->add_instruction(
      CodingAdventures::CompilerIr::IrInstruction->new(
          opcode   => CodingAdventures::CompilerIr::IrOp::HALT,
          operands => [],
          id       => $gen->next,
      )
  );

  print print_ir($prog);
  # .version 1
  # .entry _start
  # _start:
  #   HALT          ; #0

=head1 DESCRIPTION

Perl port of the Go C<compiler-ir> package.  Provides the data types,
printer, and parser for the coding-adventures AOT compiler IR.

=head1 LICENSE

MIT

=cut
