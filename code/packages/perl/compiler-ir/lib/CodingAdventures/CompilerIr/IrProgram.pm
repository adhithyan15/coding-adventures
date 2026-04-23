package CodingAdventures::CompilerIr::IrProgram;

# ============================================================================
# CodingAdventures::CompilerIr::IrProgram — a complete IR program
# ============================================================================
#
# An IrProgram is the top-level container for everything the compiler
# produces.  It holds:
#
#   instructions  — the linear sequence of IrInstruction objects
#   data          — data segment declarations (IrDataDecl objects)
#   entry_label   — the label where execution begins (e.g., "_start")
#   version       — IR version number (1 = Brainfuck subset)
#
# ## Linear IR
#
# The Instructions array is ordered — execution flows from index 0 to
# len-1, with JUMP/BRANCH instructions altering the flow.  There are no
# basic blocks or SSA form in v1.  This simplicity makes the IR easy to
# generate and easy to print/parse.
#
# ## Usage
#
#   my $prog = CodingAdventures::CompilerIr::IrProgram->new('_start');
#   $prog->add_instruction($instr);
#   $prog->add_data($decl);
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# new($entry_label) — create a new program with the given entry point.
#
# The version is always 1 (v1 = Brainfuck subset).
sub new {
    my ($class, $entry_label) = @_;
    return bless {
        instructions => [],
        data         => [],
        entry_label  => $entry_label,
        version      => 1,
    }, $class;
}

# add_instruction($instr) — append an IrInstruction to the program.
sub add_instruction {
    my ($self, $instr) = @_;
    push @{ $self->{instructions} }, $instr;
}

# add_data($decl) — append an IrDataDecl to the program.
sub add_data {
    my ($self, $decl) = @_;
    push @{ $self->{data} }, $decl;
}

1;

__END__

=head1 NAME

CodingAdventures::CompilerIr::IrProgram - a complete IR program container

=head1 SYNOPSIS

  my $prog = CodingAdventures::CompilerIr::IrProgram->new('_start');
  $prog->add_data(CodingAdventures::CompilerIr::IrDataDecl->new(
      label => 'tape', size => 30000, init => 0,
  ));
  $prog->add_instruction($some_instr);

  print $prog->{version};      # 1
  print $prog->{entry_label};  # '_start'
  print scalar @{ $prog->{instructions} };  # 1

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
