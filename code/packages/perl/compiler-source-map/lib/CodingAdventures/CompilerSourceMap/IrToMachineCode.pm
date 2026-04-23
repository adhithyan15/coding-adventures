package CodingAdventures::CompilerSourceMap::IrToMachineCode;

# ============================================================================
# IrToMachineCode — Segment 4: IR instruction IDs → machine code byte offsets
# ============================================================================
#
# Each entry maps one IR instruction to the machine code bytes it produced.
# For example, a LOAD_BYTE IR instruction might produce 8 bytes of RISC-V
# machine code starting at offset 0x14 in the .text section.
#
# ## Fields per entry
#
#   ir_id     — IR instruction ID
#   mc_offset — byte offset in the .text section
#   mc_length — number of bytes of machine code
#
# ## Lookups
#
#   lookup_by_ir_id($ir_id)       — returns (mc_offset, mc_length) or (-1, 0)
#   lookup_by_mc_offset($offset)  — returns ir_id or -1
#
# The "contains" relationship for lookup_by_mc_offset:
#   entry.mc_offset <= offset < entry.mc_offset + entry.mc_length
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# new() — create an empty IrToMachineCode segment.
sub new {
    my ($class) = @_;
    return bless { entries => [] }, $class;
}

# add($ir_id, $mc_offset, $mc_length) — record a mapping.
sub add {
    my ($self, $ir_id, $mc_offset, $mc_length) = @_;
    push @{ $self->{entries} }, {
        ir_id     => $ir_id,
        mc_offset => $mc_offset,
        mc_length => $mc_length,
    };
}

# lookup_by_ir_id($ir_id) — return (mc_offset, mc_length) for an IR instruction.
#
# Returns a list (offset, length) if found, or (-1, 0) if not found.
sub lookup_by_ir_id {
    my ($self, $ir_id) = @_;
    for my $entry (@{ $self->{entries} }) {
        if ($entry->{ir_id} == $ir_id) {
            return ($entry->{mc_offset}, $entry->{mc_length});
        }
    }
    return (-1, 0);
}

# lookup_by_mc_offset($offset) — return the IR ID whose machine code contains offset.
#
# An instruction's machine code "contains" $offset if:
#   entry.mc_offset <= $offset < entry.mc_offset + entry.mc_length
#
# Returns the ir_id integer, or -1 if not found.
sub lookup_by_mc_offset {
    my ($self, $offset) = @_;
    for my $entry (@{ $self->{entries} }) {
        if ($offset >= $entry->{mc_offset}
            && $offset < $entry->{mc_offset} + $entry->{mc_length})
        {
            return $entry->{ir_id};
        }
    }
    return -1;
}

1;

__END__

=head1 NAME

CodingAdventures::CompilerSourceMap::IrToMachineCode - Segment 4: IR IDs to machine code offsets

=head1 SYNOPSIS

  my $i2mc = CodingAdventures::CompilerSourceMap::IrToMachineCode->new;
  $i2mc->add(3, 0x14, 8);   # IR instruction 3 → 8 bytes at offset 0x14

  my ($off, $len) = $i2mc->lookup_by_ir_id(3);   # (0x14, 8)
  my $ir_id       = $i2mc->lookup_by_mc_offset(0x18);  # 3 (0x14 <= 0x18 < 0x1C)

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
