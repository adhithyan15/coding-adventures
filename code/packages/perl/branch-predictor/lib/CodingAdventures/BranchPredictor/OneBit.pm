package CodingAdventures::BranchPredictor::OneBit;

# ============================================================================
# BranchPredictor::OneBit — 1-bit dynamic branch predictor
# ============================================================================
#
# The one-bit predictor learns from branch history. Each branch address maps
# to a single bit of state that records the last outcome:
#
#   bit = 0  →  predict NOT TAKEN
#   bit = 1  →  predict TAKEN
#
# "Predict whatever happened last time."
#
# ## State machine
#
#   +-------------+   taken    +-------------+
#   |  not_taken  | ---------> |    taken    |
#   |  (bit = 0)  | <--------- |  (bit = 1)  |
#   +-------------+  not_taken +-------------+
#
# ## The double-misprediction problem
#
# For a loop running N times: the 1-bit predictor mispredicts TWICE per
# invocation of the loop (once on entry, once on exit). The 2-bit predictor
# solves this with hysteresis.
#
# ## Table indexing and aliasing
#
# Table is indexed by: pc % table_size
# Two PCs with the same low bits ALIAS to the same entry — they interfere
# with each other's predictions. This is a known hardware limitation.

use strict;
use warnings;
use CodingAdventures::BranchPredictor::Stats;
use CodingAdventures::BranchPredictor::Prediction;

our $VERSION = '0.01';

sub new {
    my ($class, %args) = @_;
    $class = ref($class) if ref($class);
    my $table_size = $args{table_size} // 1024;
    return bless {
        table_size => $table_size,
        _table     => {},    # index -> 0/1
        stats      => CodingAdventures::BranchPredictor::Stats->new(),
    }, $class;
}

sub _index {
    my ($self, $pc) = @_;
    return $pc % $self->{table_size};
}

# Predict based on last outcome. Cold start = NOT TAKEN (0).
sub predict {
    my ($self, $pc) = @_;
    my $idx   = $self->_index($pc);
    my $taken = $self->{_table}{$idx} // 0;
    return (
        CodingAdventures::BranchPredictor::Prediction->new(
            predicted_taken => $taken,
            confidence      => 0.5,
        ),
        $self,
    );
}

# Update table and stats with actual outcome.
sub update {
    my ($self, $pc, $taken, $target) = @_;
    my $idx       = $self->_index($pc);
    my $predicted = $self->{_table}{$idx} // 0;
    my $correct   = ($predicted ? 1 : 0) == ($taken ? 1 : 0);

    my %new_table = %{ $self->{_table} };
    $new_table{$idx} = $taken ? 1 : 0;

    return bless {
        table_size => $self->{table_size},
        _table     => \%new_table,
        stats      => $self->{stats}->record($correct ? 1 : 0),
    }, ref($self);
}

sub get_stats { $_[0]->{stats} }

sub reset {
    my ($self) = @_;
    return __PACKAGE__->new(table_size => $self->{table_size});
}

1;
