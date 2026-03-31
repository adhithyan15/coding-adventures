package CodingAdventures::BranchPredictor::Static;

# ============================================================================
# BranchPredictor::Static — Static predictors (no learning)
# ============================================================================
#
# Static predictors make the same prediction every time, with no learning
# from branch history. Three strategies:
#
#   AlwaysTaken      ~60-70% accurate  (loops are usually taken)
#   AlwaysNotTaken   ~30-40% accurate  (baseline, the 8086 strategy)
#   BTFNT            ~65-75% accurate  (backward=taken, forward=not taken)
#
# Historical usage:
#   Intel 8086 (1978): implicitly AlwaysNotTaken (no branch predictor)
#   MIPS R4000 (1991): BTFNT as primary strategy
#   SPARC V8 (1992):   BTFNT with branch annulling

use strict;
use warnings;
our $VERSION = '0.01';

# ============================================================================
# AlwaysTaken
# ============================================================================

package CodingAdventures::BranchPredictor::Static::AlwaysTaken;

use CodingAdventures::BranchPredictor::Stats;
use CodingAdventures::BranchPredictor::Prediction;

sub new {
    my ($class) = @_;
    $class = ref($class) if ref($class);
    return bless { stats => CodingAdventures::BranchPredictor::Stats->new() }, $class;
}

# Always predict taken. PC is ignored.
# Returns ($prediction, $self) — self unchanged during prediction.
sub predict {
    my ($self, $pc) = @_;
    return (
        CodingAdventures::BranchPredictor::Prediction->new(predicted_taken => 1, confidence => 0.0),
        $self,
    );
}

# Record accuracy: correct when branch was actually taken.
sub update {
    my ($self, $pc, $taken, $target) = @_;
    return bless {
        stats => $self->{stats}->record($taken ? 1 : 0),
    }, ref($self);
}

sub get_stats { $_[0]->{stats} }
sub reset { return $_[0]->new() }

# ============================================================================
# AlwaysNotTaken
# ============================================================================

package CodingAdventures::BranchPredictor::Static::AlwaysNotTaken;

use CodingAdventures::BranchPredictor::Stats;
use CodingAdventures::BranchPredictor::Prediction;

sub new {
    my ($class) = @_;
    $class = ref($class) if ref($class);
    return bless { stats => CodingAdventures::BranchPredictor::Stats->new() }, $class;
}

sub predict {
    my ($self, $pc) = @_;
    return (
        CodingAdventures::BranchPredictor::Prediction->new(predicted_taken => 0, confidence => 0.0),
        $self,
    );
}

# Correct when branch was NOT taken.
sub update {
    my ($self, $pc, $taken, $target) = @_;
    return bless {
        stats => $self->{stats}->record($taken ? 0 : 1),
    }, ref($self);
}

sub get_stats { $_[0]->{stats} }
sub reset { return $_[0]->new() }

# ============================================================================
# BTFNT — Backward Taken, Forward Not Taken
# ============================================================================
#
# If the branch target is BEFORE the current PC (backward = loop back-edge),
# predict TAKEN. If the target is AFTER the PC (forward = if-else), predict
# NOT TAKEN.
#
# On cold start (target unknown), defaults to NOT TAKEN.

package CodingAdventures::BranchPredictor::Static::BTFNT;

use CodingAdventures::BranchPredictor::Stats;
use CodingAdventures::BranchPredictor::Prediction;

sub new {
    my ($class) = @_;
    $class = ref($class) if ref($class);
    return bless {
        targets => {},   # pc -> last known target
        stats   => CodingAdventures::BranchPredictor::Stats->new(),
    }, $class;
}

sub predict {
    my ($self, $pc) = @_;
    my $target = $self->{targets}{$pc};
    unless (defined $target) {
        # Cold start: default to not taken
        return (
            CodingAdventures::BranchPredictor::Prediction->new(predicted_taken => 0, confidence => 0.0),
            $self,
        );
    }
    # Backward (target <= pc): taken; forward (target > pc): not taken
    my $taken = ($target <= $pc) ? 1 : 0;
    return (
        CodingAdventures::BranchPredictor::Prediction->new(
            predicted_taken => $taken,
            confidence      => 0.5,
            address         => $target,
        ),
        $self,
    );
}

sub update {
    my ($self, $pc, $taken, $target) = @_;
    my %new_targets = %{ $self->{targets} };
    $new_targets{$pc} = $target if defined $target;

    my $known_target = $new_targets{$pc};
    my $predicted_taken = 0;
    if (defined $known_target) {
        $predicted_taken = ($known_target <= $pc) ? 1 : 0;
    }

    return bless {
        targets => \%new_targets,
        stats   => $self->{stats}->record($predicted_taken == ($taken ? 1 : 0)),
    }, ref($self);
}

sub get_stats { $_[0]->{stats} }
sub reset { return $_[0]->new() }

1;
