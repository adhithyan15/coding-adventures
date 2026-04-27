package CodingAdventures::CompilerSourceMap::IrToIr;

# ============================================================================
# IrToIr — Segment 3: IR instruction IDs → optimised IR instruction IDs
# ============================================================================
#
# One IrToIr segment is produced per optimiser pass.  The pass_name field
# identifies which pass produced this mapping (e.g., "identity",
# "contraction", "clear_loop", "dead_store").
#
# ## Three cases for each original instruction
#
#   1. Preserved:  original_id → [same_id]         (instruction unchanged)
#   2. Replaced:   original_id → [new_id_1, ...]   (split or transformed)
#   3. Deleted:    original_id is in the deleted set (optimised away)
#
# ## Example: contraction pass
#
# A contraction pass folds three ADD_IMM 1 instructions (IDs 7, 8, 9) into
# one ADD_IMM 3 (ID 100):
#   7 → [100], 8 → [100], 9 → [100]
#
# ## Fields
#
#   entries    — arrayref of { original_id => N, new_ids => [...] }
#   deleted    — hashref { id => 1 } for optimised-away instructions
#   pass_name  — string identifying which optimiser pass produced this segment
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# new($pass_name) — create an empty IrToIr segment for the named pass.
sub new {
    my ($class, $pass_name) = @_;
    return bless {
        entries   => [],
        deleted   => {},
        pass_name => $pass_name // '',
    }, $class;
}

# add_mapping($original_id, $new_ids_arrayref) — record a transformation.
#
# Records that original_id was replaced by the instructions in new_ids.
sub add_mapping {
    my ($self, $original_id, $new_ids) = @_;
    push @{ $self->{entries} }, {
        original_id => $original_id,
        new_ids     => $new_ids,
    };
}

# add_deletion($original_id) — record that an instruction was deleted.
#
# Deleted instructions have no replacement — they were optimised away.
sub add_deletion {
    my ($self, $original_id) = @_;
    $self->{deleted}{$original_id} = 1;
    push @{ $self->{entries} }, {
        original_id => $original_id,
        new_ids     => [],
    };
}

# lookup_by_original_id($original_id) — return new IDs for an original ID.
#
# Returns an arrayref of new IDs if found, or undef if deleted or not found.
sub lookup_by_original_id {
    my ($self, $original_id) = @_;
    return undef if $self->{deleted}{$original_id};
    for my $entry (@{ $self->{entries} }) {
        if ($entry->{original_id} == $original_id) {
            return $entry->{new_ids};
        }
    }
    return undef;
}

# lookup_by_new_id($new_id) — return the original ID that produced a new ID.
#
# Returns the original ID integer, or -1 if not found.
# When multiple originals map to the same new ID (contraction), this
# returns the first one found.
sub lookup_by_new_id {
    my ($self, $new_id) = @_;
    for my $entry (@{ $self->{entries} }) {
        for my $id (@{ $entry->{new_ids} }) {
            return $entry->{original_id} if $id == $new_id;
        }
    }
    return -1;
}

1;

__END__

=head1 NAME

CodingAdventures::CompilerSourceMap::IrToIr - Segment 3: IR to optimised IR mapping

=head1 SYNOPSIS

  my $pass = CodingAdventures::CompilerSourceMap::IrToIr->new('contraction');
  $pass->add_mapping(7, [100]);
  $pass->add_mapping(8, [100]);
  $pass->add_deletion(9);

  my $new_ids = $pass->lookup_by_original_id(7);  # [100]
  my $orig    = $pass->lookup_by_new_id(100);      # 7 (first match)

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
