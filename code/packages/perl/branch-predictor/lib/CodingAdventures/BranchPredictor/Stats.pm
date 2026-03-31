package CodingAdventures::BranchPredictor::Stats;

# ============================================================================
# BranchPredictor::Stats — Accuracy tracking for branch predictors
# ============================================================================
#
# Every branch predictor needs a scorecard. The key metric is "accuracy":
# the percentage of branches predicted correctly. A 95% accurate predictor
# causes a pipeline flush on only 5% of branches — turning a potential
# 11-cycle penalty (on ARM Cortex-A78) into a ~0.55-cycle average cost.
#
# We track three counters:
#   predictions — total number of branches seen
#   correct     — correctly predicted
#   incorrect   — incorrectly predicted (each one = pipeline flush)
#
# From these we derive:
#   accuracy()            — correct/predictions × 100
#   misprediction_rate()  — incorrect/predictions × 100

use strict;
use warnings;
our $VERSION = '0.01';

# Create a new Stats object with all counters at zero.
sub new {
    my ($class) = @_;
    return bless {
        predictions => 0,
        correct     => 0,
        incorrect   => 0,
    }, $class;
}

# Record the outcome of one prediction.
# correct_guess: 1 (correct) or 0 (wrong)
# Returns a NEW Stats object (immutable style).
sub record {
    my ($self, $correct_guess) = @_;
    my $class = ref($self) || $self;
    my $new = $class->new();
    $new->{predictions} = $self->{predictions} + 1;
    if ($correct_guess) {
        $new->{correct}   = $self->{correct} + 1;
        $new->{incorrect} = $self->{incorrect};
    } else {
        $new->{correct}   = $self->{correct};
        $new->{incorrect} = $self->{incorrect} + 1;
    }
    return $new;
}

# Prediction accuracy as a percentage (0.0 to 100.0).
# Returns 0.0 if no predictions have been made.
sub accuracy {
    my ($self) = @_;
    return 0.0 if $self->{predictions} == 0;
    return $self->{correct} / $self->{predictions} * 100.0;
}

# Misprediction rate as a percentage (0.0 to 100.0).
sub misprediction_rate {
    my ($self) = @_;
    return 0.0 if $self->{predictions} == 0;
    return $self->{incorrect} / $self->{predictions} * 100.0;
}

# Reset all counters.
sub reset { return $_[0]->new() }

# Accessors
sub predictions { $_[0]->{predictions} }
sub correct     { $_[0]->{correct} }
sub incorrect   { $_[0]->{incorrect} }

1;
