//! Structural hazard detection -- when hardware resources collide.
//!
//! A structural hazard occurs when two instructions need the same hardware
//! resource in the same clock cycle.

use crate::types::{HazardAction, HazardResult, PipelineSlot};

/// Detects structural hazards -- two instructions competing for hardware.
///
/// Configurable with:
/// - `num_alus`: Number of integer ALU units
/// - `num_fp_units`: Number of floating-point units
/// - `split_caches`: Whether L1I and L1D are separate
pub struct StructuralHazardDetector {
    num_alus: u32,
    num_fp_units: u32,
    split_caches: bool,
}

impl StructuralHazardDetector {
    pub fn new(num_alus: u32, num_fp_units: u32, split_caches: bool) -> Self {
        Self {
            num_alus,
            num_fp_units,
            split_caches,
        }
    }

    /// Check for structural hazards between pipeline stages.
    pub fn detect(
        &self,
        id_stage: &PipelineSlot,
        ex_stage: &PipelineSlot,
        if_stage: Option<&PipelineSlot>,
        mem_stage: Option<&PipelineSlot>,
    ) -> HazardResult {
        let exec_result = self.check_execution_unit_conflict(id_stage, ex_stage);
        if exec_result.action != HazardAction::None {
            return exec_result;
        }

        if let (Some(if_s), Some(mem_s)) = (if_stage, mem_stage) {
            let mem_result = self.check_memory_port_conflict(if_s, mem_s);
            if mem_result.action != HazardAction::None {
                return mem_result;
            }
        }

        HazardResult::new(HazardAction::None, "no structural hazards -- all resources available")
    }

    /// Check if ID and EX need the same execution unit.
    fn check_execution_unit_conflict(
        &self,
        id_stage: &PipelineSlot,
        ex_stage: &PipelineSlot,
    ) -> HazardResult {
        if !id_stage.valid || !ex_stage.valid {
            return HazardResult::new(HazardAction::None, "one or both stages are empty (bubble)");
        }

        if id_stage.uses_alu && ex_stage.uses_alu && self.num_alus < 2 {
            return HazardResult {
                action: HazardAction::Stall,
                stall_cycles: 1,
                reason: format!(
                    "structural hazard: both ID (PC=0x{:04X}) and EX (PC=0x{:04X}) need the ALU, but only {} ALU available",
                    id_stage.pc, ex_stage.pc, self.num_alus
                ),
                ..Default::default()
            };
        }

        if id_stage.uses_fp && ex_stage.uses_fp && self.num_fp_units < 2 {
            return HazardResult {
                action: HazardAction::Stall,
                stall_cycles: 1,
                reason: format!(
                    "structural hazard: both ID (PC=0x{:04X}) and EX (PC=0x{:04X}) need the FP unit, but only {} FP unit available",
                    id_stage.pc, ex_stage.pc, self.num_fp_units
                ),
                ..Default::default()
            };
        }

        HazardResult::new(HazardAction::None, "no execution unit conflict")
    }

    /// Check if IF and MEM both need the memory bus (shared cache only).
    fn check_memory_port_conflict(
        &self,
        if_stage: &PipelineSlot,
        mem_stage: &PipelineSlot,
    ) -> HazardResult {
        if self.split_caches {
            return HazardResult::new(HazardAction::None, "split caches -- no memory port conflict");
        }

        if if_stage.valid && mem_stage.valid && (mem_stage.mem_read || mem_stage.mem_write) {
            let access_type = if mem_stage.mem_read { "load" } else { "store" };
            return HazardResult {
                action: HazardAction::Stall,
                stall_cycles: 1,
                reason: format!(
                    "structural hazard: IF (fetch at PC=0x{:04X}) and MEM ({} at PC=0x{:04X}) both need the shared memory bus",
                    if_stage.pc, access_type, mem_stage.pc
                ),
                ..Default::default()
            };
        }

        HazardResult::new(HazardAction::None, "no memory port conflict")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_slot() -> PipelineSlot {
        PipelineSlot { valid: false, ..Default::default() }
    }

    #[test]
    fn no_hazard_with_enough_alus() {
        let s = StructuralHazardDetector::new(2, 1, true);
        let id = PipelineSlot { valid: true, uses_alu: true, ..Default::default() };
        let ex = PipelineSlot { valid: true, uses_alu: true, ..Default::default() };
        let result = s.detect(&id, &ex, None, None);
        assert_eq!(result.action, HazardAction::None);
    }

    #[test]
    fn alu_conflict_with_one_alu() {
        let s = StructuralHazardDetector::new(1, 1, true);
        let id = PipelineSlot { valid: true, uses_alu: true, ..Default::default() };
        let ex = PipelineSlot { valid: true, uses_alu: true, ..Default::default() };
        let result = s.detect(&id, &ex, None, None);
        assert_eq!(result.action, HazardAction::Stall);
        assert_eq!(result.stall_cycles, 1);
    }

    #[test]
    fn fp_conflict_with_one_fp_unit() {
        let s = StructuralHazardDetector::new(1, 1, true);
        let id = PipelineSlot { valid: true, uses_alu: false, uses_fp: true, ..Default::default() };
        let ex = PipelineSlot { valid: true, uses_alu: false, uses_fp: true, ..Default::default() };
        let result = s.detect(&id, &ex, None, None);
        assert_eq!(result.action, HazardAction::Stall);
    }

    #[test]
    fn no_fp_conflict_with_two_fp_units() {
        let s = StructuralHazardDetector::new(1, 2, true);
        let id = PipelineSlot { valid: true, uses_alu: false, uses_fp: true, ..Default::default() };
        let ex = PipelineSlot { valid: true, uses_alu: false, uses_fp: true, ..Default::default() };
        let result = s.detect(&id, &ex, None, None);
        assert_eq!(result.action, HazardAction::None);
    }

    #[test]
    fn no_conflict_when_id_empty() {
        let s = StructuralHazardDetector::new(1, 1, true);
        let id = empty_slot();
        let ex = PipelineSlot { valid: true, uses_alu: true, ..Default::default() };
        let result = s.detect(&id, &ex, None, None);
        assert_eq!(result.action, HazardAction::None);
    }

    #[test]
    fn memory_port_conflict_shared_cache() {
        let s = StructuralHazardDetector::new(1, 1, false);
        let id = PipelineSlot { valid: true, uses_alu: false, ..Default::default() };
        let ex = PipelineSlot { valid: true, uses_alu: false, ..Default::default() };
        let if_stage = PipelineSlot { valid: true, pc: 0x10, ..Default::default() };
        let mem_stage = PipelineSlot { valid: true, pc: 0x04, mem_read: true, ..Default::default() };
        let result = s.detect(&id, &ex, Some(&if_stage), Some(&mem_stage));
        assert_eq!(result.action, HazardAction::Stall);
    }

    #[test]
    fn no_memory_conflict_split_cache() {
        let s = StructuralHazardDetector::new(1, 1, true);
        let id = PipelineSlot { valid: true, uses_alu: false, ..Default::default() };
        let ex = PipelineSlot { valid: true, uses_alu: false, ..Default::default() };
        let if_stage = PipelineSlot { valid: true, ..Default::default() };
        let mem_stage = PipelineSlot { valid: true, mem_read: true, ..Default::default() };
        let result = s.detect(&id, &ex, Some(&if_stage), Some(&mem_stage));
        assert_eq!(result.action, HazardAction::None);
    }

    #[test]
    fn memory_port_conflict_store() {
        let s = StructuralHazardDetector::new(1, 1, false);
        let id = PipelineSlot { valid: true, uses_alu: false, ..Default::default() };
        let ex = PipelineSlot { valid: true, uses_alu: false, ..Default::default() };
        let if_stage = PipelineSlot { valid: true, ..Default::default() };
        let mem_stage = PipelineSlot { valid: true, mem_write: true, ..Default::default() };
        let result = s.detect(&id, &ex, Some(&if_stage), Some(&mem_stage));
        assert_eq!(result.action, HazardAction::Stall);
    }

    #[test]
    fn no_memory_conflict_when_mem_not_accessing() {
        let s = StructuralHazardDetector::new(1, 1, false);
        let id = PipelineSlot { valid: true, uses_alu: false, ..Default::default() };
        let ex = PipelineSlot { valid: true, uses_alu: false, ..Default::default() };
        let if_stage = PipelineSlot { valid: true, ..Default::default() };
        let mem_stage = PipelineSlot { valid: true, ..Default::default() };
        let result = s.detect(&id, &ex, Some(&if_stage), Some(&mem_stage));
        assert_eq!(result.action, HazardAction::None);
    }
}
