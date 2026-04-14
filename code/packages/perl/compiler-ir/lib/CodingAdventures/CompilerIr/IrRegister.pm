package CodingAdventures::CompilerIr::IrRegister;

# ============================================================================
# CodingAdventures::CompilerIr::IrRegister — a virtual register operand
# ============================================================================
#
# Virtual registers are named v0, v1, v2, ... (the "index" field).  There
# are infinitely many — the backend's register allocator maps them to
# physical hardware registers.
#
# ## Why virtual registers?
#
# A virtual register is just a name for a value.  By not reusing names
# (each computation gets its own vN), the IR stays in a form where the
# source and destination of every operation are explicit.  The backend can
# then decide whether two virtual registers share a physical register.
#
# ## String representation
#
#   IrRegister->new(0)  →  "v0"
#   IrRegister->new(5)  →  "v5"
#
# ## Usage
#
#   my $reg = CodingAdventures::CompilerIr::IrRegister->new(2);
#   print $reg->to_string;   # "v2"
#   print $reg->{index};     # 2
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# new($index) — create a virtual register with the given 0-based index.
#
# $index must be a non-negative integer.  The maximum is 65535 (v65535),
# which matches the Go implementation's limit.
sub new {
    my ($class, $index) = @_;
    return bless { index => $index }, $class;
}

# to_string() — return the canonical text representation ("vN").
sub to_string {
    my ($self) = @_;
    return 'v' . $self->{index};
}

# type_tag() — identifies this operand as a register.
# Used internally by the printer and parser to dispatch on operand kind.
sub type_tag { 'register' }

1;

__END__

=head1 NAME

CodingAdventures::CompilerIr::IrRegister - virtual register operand

=head1 SYNOPSIS

  my $r = CodingAdventures::CompilerIr::IrRegister->new(3);
  print $r->to_string;   # "v3"
  print $r->{index};     # 3

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
