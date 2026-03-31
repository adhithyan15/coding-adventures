package CodingAdventures::BranchPredictor::TwoBit;

# ============================================================================
# BranchPredictor::TwoBit — 2-bit saturating counter predictor
# ============================================================================
#
# Four states — a saturating counter that requires TWO consecutive
# mispredictions before changing the predicted direction:
#
#   SNT (Strongly Not Taken)   →  predict NOT TAKEN
#   WNT (Weakly Not Taken)     →  predict NOT TAKEN
#   WT  (Weakly Taken)         →  predict TAKEN
#   ST  (Strongly Taken)       →  predict TAKEN
#
# Transitions:
#   taken outcome:     SNT→WNT→WT→ST (increment, saturate at ST)
#   not-taken outcome: ST→WT→WNT→SNT (decrement, saturate at SNT)
#
# ## Why this beats 1-bit for loops
#
# After a loop exits (not taken): ST → WT (still predicts TAKEN).
# On the next loop entry: WT predicts TAKEN correctly — no extra misprediction.
# The 1-bit predictor would have predicted NOT TAKEN on re-entry.
#
# ## Historical usage
#   Alpha 21064: 2-bit counters, 2048 entries
#   Intel Pentium: 2-bit counters, 256 entries

use strict;
use warnings;
use CodingAdventures::BranchPredictor::Stats;
use CodingAdventures::BranchPredictor::Prediction;

our $VERSION = '0.01';

# State constants
use constant SNT => 'SNT';
use constant WNT => 'WNT';
use constant WT  => 'WT';
use constant ST  => 'ST';

# Saturating counter transitions
my %TAKEN_NEXT = (
    SNT, WNT,
    WNT, WT,
    WT,  ST,
    ST,  ST,   # saturate
);
my %NOT_TAKEN_NEXT = (
    ST,  WT,
    WT,  WNT,
    WNT, SNT,
    SNT, SNT,  # saturate
);

# States that predict TAKEN
my %PREDICTS_TAKEN = (WT, 1, ST, 1);

sub new {
    my ($class, %args) = @_;
    return bless {
        table_size    => $args{table_size}    // 1024,
        initial_state => $args{initial_state} // WNT,
        _table        => {},
        stats         => CodingAdventures::BranchPredictor::Stats->new(),
    }, $class;
}

sub _index { $_[1] % $_[0]->{table_size} }

sub _get_state {
    my ($self, $idx) = @_;
    return $self->{_table}{$idx} // $self->{initial_state};
}

# Predict: returns (prediction, self)
sub predict {
    my ($self, $pc) = @_;
    my $idx   = $pc % $self->{table_size};
    my $state = $self->_get_state($idx);
    my $taken = $PREDICTS_TAKEN{$state} ? 1 : 0;
    my $conf  = ($state eq ST || $state eq SNT) ? 1.0 : 0.5;
    return (
        CodingAdventures::BranchPredictor::Prediction->new(
            predicted_taken => $taken,
            confidence      => $conf,
        ),
        $self,
    );
}

# Update: returns new TwoBit object
sub update {
    my ($self, $pc, $taken, $target) = @_;
    my $idx       = $pc % $self->{table_size};
    my $state     = $self->_get_state($idx);
    my $predicted = $PREDICTS_TAKEN{$state} ? 1 : 0;
    my $correct   = $predicted == ($taken ? 1 : 0);

    my $next_state = $taken ? $TAKEN_NEXT{$state} : $NOT_TAKEN_NEXT{$state};

    my %new_table = %{ $self->{_table} };
    $new_table{$idx} = $next_state;

    return bless {
        table_size    => $self->{table_size},
        initial_state => $self->{initial_state},
        _table        => \%new_table,
        stats         => $self->{stats}->record($correct ? 1 : 0),
    }, ref($self);
}

# Get state string for a PC (for testing)
sub get_state_for_pc {
    my ($self, $pc) = @_;
    return $self->_get_state($pc % $self->{table_size});
}

sub get_stats { $_[0]->{stats} }

sub reset {
    my ($self) = @_;
    return __PACKAGE__->new(
        table_size    => $self->{table_size},
        initial_state => $self->{initial_state},
    );
}

# Export constants
sub SNT_STATE { SNT }
sub WNT_STATE { WNT }
sub WT_STATE  { WT  }
sub ST_STATE  { ST  }

1;
