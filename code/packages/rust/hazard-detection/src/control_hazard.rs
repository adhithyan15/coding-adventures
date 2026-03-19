//! Control hazard detection -- handling branch mispredictions.
//!
//! A control hazard occurs when the pipeline doesn't know which instruction
//! to fetch next because a branch hasn't been resolved yet. When a
//! misprediction is detected, the IF and ID stages must be flushed.

use crate::types::{HazardAction, HazardResult, PipelineSlot};

/// Detects control hazards from branch mispredictions.
///
/// Decision logic:
///   1. Is EX valid?         No  -> NONE
///   2. Is EX a branch?      No  -> NONE
///   3. predicted == actual?  Yes -> NONE
///   4. Otherwise             -> FLUSH (2 stages)
pub struct ControlHazardDetector;

impl ControlHazardDetector {
    pub fn new() -> Self {
        Self
    }

    /// Check if a branch in the EX stage was mispredicted.
    pub fn detect(&self, ex_stage: &PipelineSlot) -> HazardResult {
        if !ex_stage.valid {
            return HazardResult::new(HazardAction::None, "EX stage is empty (bubble)");
        }

        if !ex_stage.is_branch {
            return HazardResult::new(HazardAction::None, "EX stage instruction is not a branch");
        }

        if ex_stage.branch_predicted_taken == ex_stage.branch_taken {
            let taken_str = if ex_stage.branch_taken { "taken" } else { "not taken" };
            return HazardResult::new(
                HazardAction::None,
                &format!("branch at PC=0x{:04X} correctly predicted {}", ex_stage.pc, taken_str),
            );
        }

        // Misprediction detected!
        let direction = if ex_stage.branch_taken {
            "predicted not-taken, actually taken"
        } else {
            "predicted taken, actually not-taken"
        };

        HazardResult {
            action: HazardAction::Flush,
            flush_count: 2,
            reason: format!(
                "branch misprediction at PC=0x{:04X}: {} -- flushing IF and ID stages",
                ex_stage.pc, direction
            ),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_slot() -> PipelineSlot {
        PipelineSlot { valid: false, ..Default::default() }
    }

    #[test]
    fn no_hazard_when_ex_empty() {
        let c = ControlHazardDetector::new();
        let result = c.detect(&empty_slot());
        assert_eq!(result.action, HazardAction::None);
    }

    #[test]
    fn no_hazard_when_not_branch() {
        let c = ControlHazardDetector::new();
        let ex = PipelineSlot { valid: true, is_branch: false, ..Default::default() };
        let result = c.detect(&ex);
        assert_eq!(result.action, HazardAction::None);
    }

    #[test]
    fn correctly_predicted_taken() {
        let c = ControlHazardDetector::new();
        let ex = PipelineSlot {
            valid: true, is_branch: true,
            branch_taken: true, branch_predicted_taken: true,
            ..Default::default()
        };
        let result = c.detect(&ex);
        assert_eq!(result.action, HazardAction::None);
    }

    #[test]
    fn correctly_predicted_not_taken() {
        let c = ControlHazardDetector::new();
        let ex = PipelineSlot {
            valid: true, is_branch: true,
            branch_taken: false, branch_predicted_taken: false,
            ..Default::default()
        };
        let result = c.detect(&ex);
        assert_eq!(result.action, HazardAction::None);
    }

    #[test]
    fn misprediction_not_taken_but_taken() {
        let c = ControlHazardDetector::new();
        let ex = PipelineSlot {
            valid: true, is_branch: true, pc: 0x100,
            branch_taken: true, branch_predicted_taken: false,
            ..Default::default()
        };
        let result = c.detect(&ex);
        assert_eq!(result.action, HazardAction::Flush);
        assert_eq!(result.flush_count, 2);
        assert!(result.reason.contains("not-taken, actually taken"));
    }

    #[test]
    fn misprediction_taken_but_not_taken() {
        let c = ControlHazardDetector::new();
        let ex = PipelineSlot {
            valid: true, is_branch: true, pc: 0x200,
            branch_taken: false, branch_predicted_taken: true,
            ..Default::default()
        };
        let result = c.detect(&ex);
        assert_eq!(result.action, HazardAction::Flush);
        assert_eq!(result.flush_count, 2);
        assert!(result.reason.contains("taken, actually not-taken"));
    }
}
