package CodingAdventures::CompilerIr::IrImmediate;

# ============================================================================
# CodingAdventures::CompilerIr::IrImmediate — a literal integer operand
# ============================================================================
#
# An immediate is a signed integer that appears directly in an instruction,
# rather than being loaded from a register.  Immediates are used for:
#
#   - Small constant values:  ADD_IMM v1, v1, 1    (delta = 1)
#   - Byte masks:             AND_IMM v2, v2, 255  (mask = 255)
#   - Syscall numbers:        SYSCALL 1             (write)
#   - Tape size:              LOAD_IMM v1, 0        (start offset)
#
# ## String representation
#
#   IrImmediate->new(42)   →  "42"
#   IrImmediate->new(-1)   →  "-1"
#   IrImmediate->new(255)  →  "255"
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# new($value) — create an immediate with the given integer value.
sub new {
    my ($class, $value) = @_;
    return bless { value => $value }, $class;
}

# to_string() — return the decimal string representation.
sub to_string {
    my ($self) = @_;
    return "$self->{value}";
}

# type_tag() — identifies this operand as an immediate.
sub type_tag { 'immediate' }

1;

__END__

=head1 NAME

CodingAdventures::CompilerIr::IrImmediate - literal integer operand

=head1 SYNOPSIS

  my $imm = CodingAdventures::CompilerIr::IrImmediate->new(255);
  print $imm->to_string;   # "255"
  print $imm->{value};     # 255

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
