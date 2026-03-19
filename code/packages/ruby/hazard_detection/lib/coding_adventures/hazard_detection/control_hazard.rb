# frozen_string_literal: true

# Control hazard detection -- handling branch mispredictions.
#
# === What Is a Control Hazard? ===
#
# A control hazard occurs when the pipeline doesn't know which instruction
# to fetch next because a branch hasn't been resolved yet. Modern CPUs
# use branch predictors to GUESS the outcome, but sometimes they guess
# wrong. When that happens, instructions fetched based on the wrong guess
# must be thrown away (flushed).
#
# === Cost of Misprediction ===
#
# Each misprediction costs 2 cycles (the IF and ID stages are wasted).

module CodingAdventures
  module HazardDetection
    class ControlHazardDetector
      # Check if a branch in the EX stage was mispredicted.
      #
      # Decision logic:
      #   1. Is EX valid?         No  -> NONE
      #   2. Is EX a branch?      No  -> NONE
      #   3. predicted == actual?  Yes -> NONE (correct prediction!)
      #   4. Otherwise             -> FLUSH (misprediction!)
      def detect(ex_stage)
        unless ex_stage.valid
          return HazardResult.new(
            action: HazardAction::NONE,
            reason: "EX stage is empty (bubble)"
          )
        end

        unless ex_stage.is_branch
          return HazardResult.new(
            action: HazardAction::NONE,
            reason: "EX stage instruction is not a branch"
          )
        end

        # Branch prediction was correct -- no hazard!
        if ex_stage.branch_predicted_taken == ex_stage.branch_taken
          taken_str = ex_stage.branch_taken ? "taken" : "not taken"
          return HazardResult.new(
            action: HazardAction::NONE,
            reason: format(
              "branch at PC=0x%04X correctly predicted %s",
              ex_stage.pc, taken_str
            )
          )
        end

        # Misprediction detected! Flush IF and ID stages.
        direction = if ex_stage.branch_taken
          "predicted not-taken, actually taken"
        else
          "predicted taken, actually not-taken"
        end

        HazardResult.new(
          action: HazardAction::FLUSH,
          flush_count: 2,
          reason: format(
            "branch misprediction at PC=0x%04X: %s -- flushing IF and ID stages",
            ex_stage.pc, direction
          )
        )
      end
    end
  end
end
