package CodingAdventures::CompilerIr::IrLabel;

# ============================================================================
# CodingAdventures::CompilerIr::IrLabel — a named jump target or data label
# ============================================================================
#
# Labels are strings used in three contexts:
#
#   1. As jump/branch targets:  JUMP loop_0_start
#   2. As call targets:         CALL my_function
#   3. As data references:      LOAD_ADDR v0, tape
#
# Labels resolve to addresses during code generation.  At the IR level,
# they are just opaque strings — the backend resolves them.
#
# ## String representation
#
#   IrLabel->new('_start')       →  "_start"
#   IrLabel->new('loop_0_end')   →  "loop_0_end"
#   IrLabel->new('tape')         →  "tape"
#   IrLabel->new('__trap_oob')   →  "__trap_oob"
#
# Labels can contain letters, digits, underscores, and dots.  They must
# not start with 'v' followed by digits (that would be a register).
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# new($name) — create a label operand with the given name string.
sub new {
    my ($class, $name) = @_;
    return bless { name => $name }, $class;
}

# to_string() — return the label name as-is.
sub to_string {
    my ($self) = @_;
    return $self->{name};
}

# type_tag() — identifies this operand as a label.
sub type_tag { 'label' }

1;

__END__

=head1 NAME

CodingAdventures::CompilerIr::IrLabel - named jump target or data label operand

=head1 SYNOPSIS

  my $lbl = CodingAdventures::CompilerIr::IrLabel->new('loop_0_start');
  print $lbl->to_string;   # "loop_0_start"
  print $lbl->{name};      # "loop_0_start"

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
