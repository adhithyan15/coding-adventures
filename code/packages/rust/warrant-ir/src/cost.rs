//! Operational step cost semantics for WIR complexity and bounds checking (VER00 §7.4.1).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExecutionCost {
    pub steps: u64,
    pub allocations: u64,
    pub allocated_bytes: u64,
    pub memory_reads: u64,
    pub memory_writes: u64,
    pub capability_calls: u64,
}

impl ExecutionCost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_steps(&mut self, n: u64) {
        self.steps = self.steps.saturating_add(n);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostSchedule {
    pub alu_step_cost: u64,
    pub branch_step_cost: u64,
    pub jump_step_cost: u64,
    pub memory_read_cost: u64,
    pub memory_write_cost: u64,
    pub alloc_cost: u64,
    pub free_cost: u64,
    pub call_cost: u64,
    pub return_cost: u64,
    pub capability_cost: u64,
}

impl Default for CostSchedule {
    fn default() -> Self {
        Self {
            alu_step_cost: 1,
            branch_step_cost: 1,
            jump_step_cost: 1,
            memory_read_cost: 2,
            memory_write_cost: 2,
            alloc_cost: 3,
            free_cost: 2,
            call_cost: 2,
            return_cost: 1,
            capability_cost: 5,
        }
    }
}
