package CodingAdventures::BranchPredictor;

# ============================================================================
# CodingAdventures::BranchPredictor — Branch Prediction Algorithms in Perl
# ============================================================================
#
# A branch predictor guesses the outcome of a branch instruction (if/else,
# loop, function return) BEFORE the branch condition is evaluated.
#
# ## Why does it matter?
#
# In a pipelined CPU, a wrong branch prediction causes a "pipeline flush":
# several cycles of work are thrown away because the wrong instructions
# were fetched and partially executed. In a 13-stage pipeline (ARM Cortex-A78),
# a misprediction wastes ~11 cycles. At 3 GHz with 20% branch frequency,
# even a 5% misprediction rate costs ~33 million wasted cycles per second.
#
# ## Predictor hierarchy (accuracy vs complexity)
#
#   AlwaysNotTaken  ~40%   no state, no logic
#   AlwaysTaken     ~60%   no state, no logic
#   BTFNT           ~70%   1 memory word per branch (target cache)
#   OneBit          ~80%   1 bit per branch in a table
#   TwoBit          ~90%   2 bits per branch in a table (the industry standard)
#
# ## Usage
#
#   use CodingAdventures::BranchPredictor;
#
#   my $p = CodingAdventures::BranchPredictor::TwoBit->new();
#   my ($pred, $p) = $p->predict(0x100);
#   print $pred->predicted_taken ? "taken\n" : "not taken\n";
#   $p = $p->update(0x100, 1);  # branch was actually taken
#   printf "Accuracy: %.1f%%\n", $p->get_stats->accuracy;

use strict;
use warnings;
our $VERSION = '0.01';

# Re-export sub-module classes for convenience
use CodingAdventures::BranchPredictor::Stats;
use CodingAdventures::BranchPredictor::Prediction;
use CodingAdventures::BranchPredictor::Static;
use CodingAdventures::BranchPredictor::OneBit;
use CodingAdventures::BranchPredictor::TwoBit;
use CodingAdventures::BranchPredictor::BTB;

1;
