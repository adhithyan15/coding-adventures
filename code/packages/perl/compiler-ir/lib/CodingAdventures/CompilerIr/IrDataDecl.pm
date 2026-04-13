package CodingAdventures::CompilerIr::IrDataDecl;

# ============================================================================
# CodingAdventures::CompilerIr::IrDataDecl — a data segment declaration
# ============================================================================
#
# A data declaration reserves a named region of memory with a given size
# and initial byte value.  For Brainfuck, this is the tape:
#
#   label => 'tape', size => 30000, init => 0
#
# The Init value is repeated for every byte in the region.  Init = 0 means
# zero-initialised (equivalent to .bss in ELF format).
#
# ## Text format
#
#   .data tape 30000 0
#   .data my_buf 1024 0
#
# ## Fields
#
#   label  — the symbol name (e.g., "tape")
#   size   — number of bytes to allocate
#   init   — initial byte value (0..255), usually 0
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# new(%args) — create a data declaration.
#
# Arguments:
#   label => string (required)
#   size  => integer > 0 (required)
#   init  => integer 0..255 (default 0)
sub new {
    my ($class, %args) = @_;
    return bless {
        label => $args{label},
        size  => $args{size},
        init  => defined($args{init}) ? $args{init} : 0,
    }, $class;
}

1;

__END__

=head1 NAME

CodingAdventures::CompilerIr::IrDataDecl - data segment declaration

=head1 SYNOPSIS

  my $decl = CodingAdventures::CompilerIr::IrDataDecl->new(
      label => 'tape',
      size  => 30000,
      init  => 0,
  );
  # Prints as: .data tape 30000 0

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
