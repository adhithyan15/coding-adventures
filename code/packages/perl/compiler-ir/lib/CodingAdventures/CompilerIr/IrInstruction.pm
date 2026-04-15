package CodingAdventures::CompilerIr::IrInstruction;

# ============================================================================
# CodingAdventures::CompilerIr::IrInstruction — a single IR instruction
# ============================================================================
#
# Every instruction in the IR program has three fields:
#
#   opcode   — what operation to perform (an integer from IrOp)
#   operands — arrayref of arguments: IrRegister, IrImmediate, or IrLabel
#   id       — a unique monotonic integer assigned by IDGenerator
#
# ## The ID field and source mapping
#
# The ID field is the key that connects this instruction to the source map
# chain.  Each instruction gets a unique ID from IDGenerator, and that ID
# flows through all pipeline stages.  This allows the debugger to ask:
# "Which source character produced this machine code byte?"
#
# Special case: LABEL and COMMENT instructions get ID = -1 because they
# produce no machine code.
#
# ## Examples
#
#   { opcode => ADD_IMM, operands => [$v1, $v1, $imm1], id => 3 }
#   → printed as:  ADD_IMM     v1, v1, 1  ; #3
#
#   { opcode => BRANCH_Z, operands => [$v2, $loop_end], id => 7 }
#   → printed as:  BRANCH_Z    v2, loop_0_end  ; #7
#
#   { opcode => LABEL, operands => [$lbl], id => -1 }
#   → printed as:  loop_0_start:
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# new(%args) — create an instruction.
#
# Arguments:
#   opcode   => integer (required)
#   operands => arrayref of IrRegister/IrImmediate/IrLabel (default [])
#   id       => integer (default -1)
sub new {
    my ($class, %args) = @_;
    return bless {
        opcode   => $args{opcode},
        operands => $args{operands} // [],
        id       => defined($args{id}) ? $args{id} : -1,
    }, $class;
}

1;

__END__

=head1 NAME

CodingAdventures::CompilerIr::IrInstruction - a single IR instruction

=head1 SYNOPSIS

  use CodingAdventures::CompilerIr::IrOp;
  use CodingAdventures::CompilerIr::IrInstruction;
  use CodingAdventures::CompilerIr::IrRegister;
  use CodingAdventures::CompilerIr::IrImmediate;

  my $instr = CodingAdventures::CompilerIr::IrInstruction->new(
      opcode   => CodingAdventures::CompilerIr::IrOp::ADD_IMM,
      operands => [
          CodingAdventures::CompilerIr::IrRegister->new(1),
          CodingAdventures::CompilerIr::IrRegister->new(1),
          CodingAdventures::CompilerIr::IrImmediate->new(1),
      ],
      id => 3,
  );

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
