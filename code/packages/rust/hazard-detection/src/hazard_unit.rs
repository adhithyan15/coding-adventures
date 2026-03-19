//! Combined hazard detection unit -- the pipeline's traffic controller.
//!
//! Runs all three detectors every clock cycle and returns the highest-priority
//! action. Also tracks statistics for performance analysis.
//!
//! Priority: FLUSH > STALL > FORWARD > NONE

use crate::control_hazard::ControlHazardDetector;
use crate::data_hazard::{pick_higher_priority, DataHazardDetector};
use crate::structural_hazard::StructuralHazardDetector;
use crate::types::{HazardAction, HazardResult, PipelineSlot};

/// Combined hazard detection unit.
pub struct HazardUnit {
    data_detector: DataHazardDetector,
    control_detector: ControlHazardDetector,
    structural_detector: StructuralHazardDetector,
    history: Vec<HazardResult>,
}

impl HazardUnit {
    /// Create a hazard unit with configurable hardware resources.
    pub fn new(num_alus: u32, num_fp_units: u32, split_caches: bool) -> Self {
        Self {
            data_detector: DataHazardDetector::new(),
            control_detector: ControlHazardDetector::new(),
            structural_detector: StructuralHazardDetector::new(num_alus, num_fp_units, split_caches),
            history: Vec::new(),
        }
    }

    /// Run all hazard detectors and return the highest-priority action.
    pub fn check(
        &mut self,
        if_stage: &PipelineSlot,
        id_stage: &PipelineSlot,
        ex_stage: &PipelineSlot,
        mem_stage: &PipelineSlot,
    ) -> HazardResult {
        let control_result = self.control_detector.detect(ex_stage);
        let data_result = self.data_detector.detect(id_stage, ex_stage, mem_stage);
        let structural_result = self.structural_detector.detect(
            id_stage,
            ex_stage,
            Some(if_stage),
            Some(mem_stage),
        );

        let final_result = pick_highest_priority(control_result, data_result, structural_result);
        self.history.push(final_result.clone());
        final_result
    }

    /// Complete history of hazard results.
    pub fn history(&self) -> &[HazardResult] {
        &self.history
    }

    /// Total stall cycles across all checks.
    pub fn stall_count(&self) -> u32 {
        self.history.iter().map(|r| r.stall_cycles).sum()
    }

    /// Total pipeline flushes.
    pub fn flush_count(&self) -> u32 {
        self.history
            .iter()
            .filter(|r| r.action == HazardAction::Flush)
            .count() as u32
    }

    /// Total forwarding operations.
    pub fn forward_count(&self) -> u32 {
        self.history
            .iter()
            .filter(|r| {
                r.action == HazardAction::ForwardFromEX
                    || r.action == HazardAction::ForwardFromMEM
            })
            .count() as u32
    }
}

/// Pick the highest-priority result from three candidates.
fn pick_highest_priority(a: HazardResult, b: HazardResult, c: HazardResult) -> HazardResult {
    let best = pick_higher_priority(a, b);
    pick_higher_priority(best, c)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_slot() -> PipelineSlot {
        PipelineSlot { valid: false, ..Default::default() }
    }

    #[test]
    fn no_hazard() {
        let mut unit = HazardUnit::new(2, 1, true);
        let if_s = PipelineSlot { valid: true, ..Default::default() };
        let id = PipelineSlot { valid: true, source_regs: vec![2], ..Default::default() };
        let ex = PipelineSlot { valid: true, dest_reg: Some(5), ..Default::default() };
        let mem = empty_slot();
        let result = unit.check(&if_s, &id, &ex, &mem);
        assert_eq!(result.action, HazardAction::None);
    }

    #[test]
    fn data_forwarding() {
        let mut unit = HazardUnit::new(2, 1, true);
        let if_s = PipelineSlot { valid: true, ..Default::default() };
        let id = PipelineSlot { valid: true, source_regs: vec![1], ..Default::default() };
        let ex = PipelineSlot { valid: true, dest_reg: Some(1), dest_value: Some(42), ..Default::default() };
        let mem = empty_slot();
        let result = unit.check(&if_s, &id, &ex, &mem);
        assert_eq!(result.action, HazardAction::ForwardFromEX);
        assert_eq!(result.forwarded_value, Some(42));
    }

    #[test]
    fn flush_beats_forward() {
        let mut unit = HazardUnit::new(2, 1, true);
        let if_s = PipelineSlot { valid: true, ..Default::default() };
        let id = PipelineSlot { valid: true, source_regs: vec![1], ..Default::default() };
        let ex = PipelineSlot {
            valid: true, dest_reg: Some(1), dest_value: Some(42),
            is_branch: true, branch_taken: true, branch_predicted_taken: false,
            ..Default::default()
        };
        let mem = empty_slot();
        let result = unit.check(&if_s, &id, &ex, &mem);
        assert_eq!(result.action, HazardAction::Flush);
    }

    #[test]
    fn stall_beats_forward() {
        let mut unit = HazardUnit::new(2, 1, true);
        let if_s = PipelineSlot { valid: true, ..Default::default() };
        let id = PipelineSlot { valid: true, source_regs: vec![1], ..Default::default() };
        let ex = PipelineSlot { valid: true, dest_reg: Some(1), mem_read: true, ..Default::default() };
        let mem = empty_slot();
        let result = unit.check(&if_s, &id, &ex, &mem);
        assert_eq!(result.action, HazardAction::Stall);
    }

    #[test]
    fn statistics_tracking() {
        let mut unit = HazardUnit::new(2, 1, true);
        let empty = empty_slot();
        let if_s = PipelineSlot { valid: true, ..Default::default() };

        // Cycle 1: no hazard
        let id1 = PipelineSlot { valid: true, source_regs: vec![2], ..Default::default() };
        let ex1 = PipelineSlot { valid: true, dest_reg: Some(5), ..Default::default() };
        unit.check(&if_s, &id1, &ex1, &empty);

        // Cycle 2: forward from EX
        let id2 = PipelineSlot { valid: true, source_regs: vec![1], ..Default::default() };
        let ex2 = PipelineSlot { valid: true, dest_reg: Some(1), dest_value: Some(10), ..Default::default() };
        unit.check(&if_s, &id2, &ex2, &empty);

        // Cycle 3: flush
        let ex3 = PipelineSlot {
            valid: true, is_branch: true,
            branch_taken: true, branch_predicted_taken: false,
            ..Default::default()
        };
        unit.check(&if_s, &empty, &ex3, &empty);

        assert_eq!(unit.history().len(), 3);
        assert_eq!(unit.stall_count(), 0);
        assert_eq!(unit.flush_count(), 1);
        assert_eq!(unit.forward_count(), 1);
    }

    #[test]
    fn structural_stall_with_one_alu() {
        let mut unit = HazardUnit::new(1, 1, true);
        let if_s = PipelineSlot { valid: true, ..Default::default() };
        let id = PipelineSlot { valid: true, source_regs: vec![], uses_alu: true, ..Default::default() };
        let ex = PipelineSlot { valid: true, dest_reg: Some(5), uses_alu: true, ..Default::default() };
        let mem = empty_slot();
        let result = unit.check(&if_s, &id, &ex, &mem);
        assert_eq!(result.action, HazardAction::Stall);
    }

    #[test]
    fn forward_from_mem() {
        let mut unit = HazardUnit::new(2, 1, true);
        let if_s = PipelineSlot { valid: true, ..Default::default() };
        let id = PipelineSlot { valid: true, source_regs: vec![3], ..Default::default() };
        let ex = empty_slot();
        let mem = PipelineSlot { valid: true, dest_reg: Some(3), dest_value: Some(88), ..Default::default() };
        let result = unit.check(&if_s, &id, &ex, &mem);
        assert_eq!(result.action, HazardAction::ForwardFromMEM);
        assert_eq!(result.forwarded_value, Some(88));
    }

    #[test]
    fn all_empty_stages() {
        let mut unit = HazardUnit::new(1, 1, true);
        let empty = empty_slot();
        let result = unit.check(&empty, &empty, &empty, &empty);
        assert_eq!(result.action, HazardAction::None);
    }
}
