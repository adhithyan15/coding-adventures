package CodingAdventures::BranchPredictor::BTB;

# ============================================================================
# BranchPredictor::BTB — Branch Target Buffer
# ============================================================================
#
# The BTB answers "WHERE does the branch go?" — the direction predictor answers
# "WILL it be taken?" You need both for zero-bubble branch prediction.
#
# Without a BTB, even a perfect direction predictor causes a 1-cycle bubble
# because the target address isn't available until the decode stage. The BTB
# makes the target available in the SAME cycle as the prediction.
#
# ## Organization (direct-mapped cache)
#
#   index = pc % size
#   Each entry: {tag => pc, target => addr, branch_type => type}
#
#   Hit:  index has an entry AND entry.tag == pc
#   Miss: no entry, or entry.tag != pc (aliasing conflict)
#
#   Eviction: new entry always replaces the old entry at same index.
#   This is the direct-mapped eviction policy.

use strict;
use warnings;
our $VERSION = '0.01';

sub new {
    my ($class, %args) = @_;
    return bless {
        size    => $args{size} // 256,
        entries => {},
        lookups => 0,
        hits    => 0,
        misses  => 0,
    }, $class;
}

sub _index { $_[1] % $_[0]->{size} }

# Look up target for a branch at PC.
# Returns ($target_or_undef, $new_btb)
sub lookup {
    my ($self, $pc) = @_;
    my $idx   = $self->_index($pc);
    my $entry = $self->{entries}{$idx};

    my $new = bless {
        size    => $self->{size},
        entries => $self->{entries},
        lookups => $self->{lookups} + 1,
        hits    => $self->{hits},
        misses  => $self->{misses},
    }, ref($self);

    if (defined $entry && $entry->{tag} == $pc) {
        $new->{hits} = $self->{hits} + 1;
        return ($entry->{target}, $new);
    }
    $new->{misses} = $self->{misses} + 1;
    return (undef, $new);
}

# Record a branch target. Evicts any conflicting entry at the same index.
sub update {
    my ($self, $pc, $target, $branch_type) = @_;
    $branch_type //= 'conditional';
    my $idx = $self->_index($pc);

    my %new_entries = %{ $self->{entries} };
    $new_entries{$idx} = { tag => $pc, target => $target, branch_type => $branch_type };

    return bless {
        size    => $self->{size},
        entries => \%new_entries,
        lookups => $self->{lookups},
        hits    => $self->{hits},
        misses  => $self->{misses},
    }, ref($self);
}

# Inspect entry (no stats update). Returns hashref or undef.
sub get_entry {
    my ($self, $pc) = @_;
    my $entry = $self->{entries}{ $self->_index($pc) };
    return undef unless defined $entry;
    return undef unless $entry->{tag} == $pc;
    return $entry;
}

# Hit rate as a percentage.
sub hit_rate {
    my ($self) = @_;
    return 0.0 unless $self->{lookups} > 0;
    return $self->{hits} / $self->{lookups} * 100.0;
}

sub reset { return $_[0]->new(size => $_[0]->{size}) }

# Accessors
sub size    { $_[0]->{size} }
sub lookups { $_[0]->{lookups} }
sub hits    { $_[0]->{hits} }
sub misses  { $_[0]->{misses} }

1;
