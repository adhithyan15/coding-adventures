package CodingAdventures::BranchPredictor::Prediction;

# ============================================================================
# BranchPredictor::Prediction — Prediction result type
# ============================================================================
#
# A Prediction is the output of a branch predictor for a single branch.
# It contains:
#   predicted_taken  — whether the branch is predicted taken (1/0)
#   confidence       — 0.0 (no idea) to 1.0 (certain)
#   address          — predicted target address (from BTB, may be undef)

use strict;
use warnings;
our $VERSION = '0.01';

sub new {
    my ($class, %args) = @_;
    return bless {
        predicted_taken => $args{predicted_taken} // 0,
        confidence      => $args{confidence}      // 0.5,
        address         => $args{address},
    }, $class;
}

sub predicted_taken { $_[0]->{predicted_taken} }
sub confidence      { $_[0]->{confidence} }
sub address         { $_[0]->{address} }

1;
