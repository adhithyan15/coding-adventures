//! Data hazard detection -- the most common pipeline hazard.
//!
//! Detects Read After Write (RAW) data hazards and resolves them via
//! forwarding or stalling. Focuses on RAW hazards -- the only type
//! in a classic 5-stage in-order pipeline.

use crate::types::{HazardAction, HazardResult, PipelineSlot};

/// Detects RAW data hazards between the ID stage and EX/MEM stages.
///
/// For each source register of the ID-stage instruction:
///   1. Match EX dest_reg? Load -> STALL, else -> FORWARD from EX
///   2. Match MEM dest_reg? -> FORWARD from MEM
///   3. No match? -> No hazard.
pub struct DataHazardDetector;

impl DataHazardDetector {
    pub fn new() -> Self {
        Self
    }

    /// Check for data hazards between ID and later stages.
    pub fn detect(
        &self,
        id_stage: &PipelineSlot,
        ex_stage: &PipelineSlot,
        mem_stage: &PipelineSlot,
    ) -> HazardResult {
        if !id_stage.valid {
            return HazardResult::new(HazardAction::None, "ID stage is empty (bubble)");
        }

        if id_stage.source_regs.is_empty() {
            return HazardResult::new(HazardAction::None, "instruction has no source registers");
        }

        let mut worst = HazardResult::new(HazardAction::None, "no data dependencies detected");

        for &src_reg in &id_stage.source_regs {
            let result = self.check_single_register(src_reg, ex_stage, mem_stage);
            worst = pick_higher_priority(worst, result);
        }

        worst
    }

    /// Check one source register against EX and MEM destinations.
    fn check_single_register(
        &self,
        src_reg: u32,
        ex_stage: &PipelineSlot,
        mem_stage: &PipelineSlot,
    ) -> HazardResult {
        // --- Check EX stage first (higher priority -- newer instruction) ---
        if ex_stage.valid {
            if let Some(dest) = ex_stage.dest_reg {
                if dest == src_reg {
                    if ex_stage.mem_read {
                        return HazardResult {
                            action: HazardAction::Stall,
                            stall_cycles: 1,
                            reason: format!(
                                "load-use hazard: R{} is being loaded by instruction at PC=0x{:04X} -- must stall 1 cycle",
                                src_reg, ex_stage.pc
                            ),
                            ..Default::default()
                        };
                    }

                    return HazardResult {
                        action: HazardAction::ForwardFromEX,
                        forwarded_value: ex_stage.dest_value,
                        forwarded_from: "EX".to_string(),
                        reason: format!(
                            "RAW hazard on R{}: forwarding from EX stage (instruction at PC=0x{:04X})",
                            src_reg, ex_stage.pc
                        ),
                        ..Default::default()
                    };
                }
            }
        }

        // --- Check MEM stage (lower priority -- older instruction) ---
        if mem_stage.valid {
            if let Some(dest) = mem_stage.dest_reg {
                if dest == src_reg {
                    return HazardResult {
                        action: HazardAction::ForwardFromMEM,
                        forwarded_value: mem_stage.dest_value,
                        forwarded_from: "MEM".to_string(),
                        reason: format!(
                            "RAW hazard on R{}: forwarding from MEM stage (instruction at PC=0x{:04X})",
                            src_reg, mem_stage.pc
                        ),
                        ..Default::default()
                    };
                }
            }
        }

        HazardResult::new(
            HazardAction::None,
            &format!("R{} has no pending writes in EX or MEM", src_reg),
        )
    }
}

/// Return whichever hazard result is more severe.
pub fn pick_higher_priority(a: HazardResult, b: HazardResult) -> HazardResult {
    if b.action.priority() > a.action.priority() {
        b
    } else {
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_slot() -> PipelineSlot {
        PipelineSlot {
            valid: false,
            ..Default::default()
        }
    }

    #[test]
    fn no_hazard_when_id_empty() {
        let d = DataHazardDetector::new();
        let id = empty_slot();
        let ex = PipelineSlot { valid: true, dest_reg: Some(1), ..Default::default() };
        let mem = empty_slot();
        let result = d.detect(&id, &ex, &mem);
        assert_eq!(result.action, HazardAction::None);
    }

    #[test]
    fn no_hazard_when_no_source_regs() {
        let d = DataHazardDetector::new();
        let id = PipelineSlot { valid: true, source_regs: vec![], ..Default::default() };
        let ex = PipelineSlot { valid: true, dest_reg: Some(1), ..Default::default() };
        let mem = empty_slot();
        let result = d.detect(&id, &ex, &mem);
        assert_eq!(result.action, HazardAction::None);
    }

    #[test]
    fn no_hazard_when_no_dependency() {
        let d = DataHazardDetector::new();
        let id = PipelineSlot { valid: true, source_regs: vec![2, 3], ..Default::default() };
        let ex = PipelineSlot { valid: true, dest_reg: Some(5), ..Default::default() };
        let mem = PipelineSlot { valid: true, dest_reg: Some(6), ..Default::default() };
        let result = d.detect(&id, &ex, &mem);
        assert_eq!(result.action, HazardAction::None);
    }

    #[test]
    fn forward_from_ex() {
        let d = DataHazardDetector::new();
        let id = PipelineSlot { valid: true, source_regs: vec![1, 5], ..Default::default() };
        let ex = PipelineSlot { valid: true, dest_reg: Some(1), dest_value: Some(42), ..Default::default() };
        let mem = empty_slot();
        let result = d.detect(&id, &ex, &mem);
        assert_eq!(result.action, HazardAction::ForwardFromEX);
        assert_eq!(result.forwarded_value, Some(42));
        assert_eq!(result.forwarded_from, "EX");
    }

    #[test]
    fn forward_from_mem() {
        let d = DataHazardDetector::new();
        let id = PipelineSlot { valid: true, source_regs: vec![1], ..Default::default() };
        let ex = empty_slot();
        let mem = PipelineSlot { valid: true, dest_reg: Some(1), dest_value: Some(99), ..Default::default() };
        let result = d.detect(&id, &ex, &mem);
        assert_eq!(result.action, HazardAction::ForwardFromMEM);
        assert_eq!(result.forwarded_value, Some(99));
    }

    #[test]
    fn load_use_stall() {
        let d = DataHazardDetector::new();
        let id = PipelineSlot { valid: true, source_regs: vec![1], ..Default::default() };
        let ex = PipelineSlot { valid: true, dest_reg: Some(1), mem_read: true, ..Default::default() };
        let mem = empty_slot();
        let result = d.detect(&id, &ex, &mem);
        assert_eq!(result.action, HazardAction::Stall);
        assert_eq!(result.stall_cycles, 1);
    }

    #[test]
    fn ex_priority_over_mem() {
        let d = DataHazardDetector::new();
        let id = PipelineSlot { valid: true, source_regs: vec![1], ..Default::default() };
        let ex = PipelineSlot { valid: true, dest_reg: Some(1), dest_value: Some(10), ..Default::default() };
        let mem = PipelineSlot { valid: true, dest_reg: Some(1), dest_value: Some(20), ..Default::default() };
        let result = d.detect(&id, &ex, &mem);
        assert_eq!(result.action, HazardAction::ForwardFromEX);
        assert_eq!(result.forwarded_value, Some(10));
    }

    #[test]
    fn stall_beats_forward() {
        let d = DataHazardDetector::new();
        let id = PipelineSlot { valid: true, source_regs: vec![1, 2], ..Default::default() };
        let ex = PipelineSlot { valid: true, dest_reg: Some(1), mem_read: true, ..Default::default() };
        let mem = PipelineSlot { valid: true, dest_reg: Some(2), dest_value: Some(77), ..Default::default() };
        let result = d.detect(&id, &ex, &mem);
        assert_eq!(result.action, HazardAction::Stall);
    }

    #[test]
    fn no_hazard_when_ex_dest_reg_none() {
        let d = DataHazardDetector::new();
        let id = PipelineSlot { valid: true, source_regs: vec![1], ..Default::default() };
        let ex = PipelineSlot { valid: true, dest_reg: None, ..Default::default() };
        let mem = empty_slot();
        let result = d.detect(&id, &ex, &mem);
        assert_eq!(result.action, HazardAction::None);
    }

    #[test]
    fn no_hazard_when_ex_invalid() {
        let d = DataHazardDetector::new();
        let id = PipelineSlot { valid: true, source_regs: vec![1], ..Default::default() };
        let ex = PipelineSlot { valid: false, dest_reg: Some(1), ..Default::default() };
        let mem = empty_slot();
        let result = d.detect(&id, &ex, &mem);
        assert_eq!(result.action, HazardAction::None);
    }
}
