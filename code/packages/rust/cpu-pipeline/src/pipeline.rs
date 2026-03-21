//! The pipeline engine -- moving instructions through stages.
//!
//! # The Pipeline: a CPU's Assembly Line
//!
//! A CPU pipeline is the central execution engine of a processor core. Instead
//! of completing one instruction fully before starting the next (like a
//! single-cycle CPU), a pipelined CPU overlaps instruction execution -- while
//! one instruction is being executed, the next is being decoded, and the one
//! after that is being fetched.
//!
//! ```text
//! Single-cycle (no pipeline):
//! Instr 1: [IF][ID][EX][MEM][WB]
//! Instr 2:                       [IF][ID][EX][MEM][WB]
//! Throughput: 1 instruction every 5 cycles
//!
//! Pipelined:
//! Instr 1: [IF][ID][EX][MEM][WB]
//! Instr 2:     [IF][ID][EX][MEM][WB]
//! Instr 3:         [IF][ID][EX][MEM][WB]
//! Throughput: 1 instruction every 1 cycle (after filling)
//! ```
//!
//! # Dependency Injection
//!
//! The pipeline uses callback functions instead of importing other packages
//! directly. This keeps the pipeline decoupled from specific implementations
//! of caches, hazard detectors, and branch predictors.

use std::collections::HashMap;

use crate::snapshot::{PipelineSnapshot, PipelineStats};
use crate::token::{PipelineConfig, PipelineToken, StageCategory};

// =========================================================================
// Callback function types
// =========================================================================
//
// The pipeline accepts callback functions that perform the actual work of
// each stage. This decouples the pipeline from specific ISA implementations.
//
// Analogy: the pipeline is like a conveyor belt. It does not care what
// is ON the belt (that is the callbacks' job). It only cares about
// MOVING items along the belt and handling stalls/flushes.

/// Fetches raw instruction bits at the given program counter.
/// In a real CPU, this reads from the instruction cache (L1I).
pub type FetchFn = Box<dyn FnMut(i64) -> i64>;

/// Decodes a raw instruction and fills in the token's fields.
/// Receives raw instruction bits and a token, returns the decoded token.
pub type DecodeFn = Box<dyn FnMut(i64, PipelineToken) -> PipelineToken>;

/// Performs the ALU operation for the instruction.
/// Receives a decoded token and returns it with ALU results.
pub type ExecuteFn = Box<dyn FnMut(PipelineToken) -> PipelineToken>;

/// Performs memory access (load/store) for the instruction.
/// For loads: fills in mem_data. For stores: writes to data cache.
pub type MemoryFn = Box<dyn FnMut(PipelineToken) -> PipelineToken>;

/// Writes the instruction's result to the register file.
/// This is the final stage -- the instruction is now complete.
pub type WritebackFn = Box<dyn FnMut(&PipelineToken)>;

// =========================================================================
// HazardAction -- what the hazard detector tells the pipeline to do
// =========================================================================

/// Represents the action the hazard unit tells the pipeline to take.
///
/// These are "traffic signals" for the pipeline:
///
/// ```text
/// NONE:             Green light -- pipeline flows normally
/// STALL:            Red light -- freeze earlier stages, insert bubble
/// FLUSH:            Emergency stop -- discard speculative instructions
/// FORWARD_FROM_EX:  Shortcut -- grab value from EX stage output
/// FORWARD_FROM_MEM: Shortcut -- grab value from MEM stage output
/// ```
///
/// Priority: FLUSH > STALL > FORWARD > NONE
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardAction {
    /// No hazard -- proceed normally.
    None,
    /// Forward value from EX stage.
    ForwardFromEX,
    /// Forward value from MEM stage.
    ForwardFromMEM,
    /// Stall the pipeline (insert bubble).
    Stall,
    /// Flush speculative stages.
    Flush,
}

impl std::fmt::Display for HazardAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HazardAction::None => write!(f, "NONE"),
            HazardAction::ForwardFromEX => write!(f, "FORWARD_FROM_EX"),
            HazardAction::ForwardFromMEM => write!(f, "FORWARD_FROM_MEM"),
            HazardAction::Stall => write!(f, "STALL"),
            HazardAction::Flush => write!(f, "FLUSH"),
        }
    }
}

/// The full response from the hazard detection callback.
///
/// Tells the pipeline what to do and provides additional context
/// (forwarded values, stall duration, flush target).
#[derive(Debug, Clone)]
pub struct HazardResponse {
    /// The hazard action to take.
    pub action: HazardAction,
    /// Value to forward (only used for FORWARD actions).
    pub forward_value: i64,
    /// Stage that provided the forwarded value.
    pub forward_source: String,
    /// Number of stages to stall (typically 1).
    pub stall_stages: usize,
    /// Number of stages to flush on a misprediction.
    pub flush_count: usize,
    /// Correct PC to fetch from after a flush.
    pub redirect_pc: i64,
}

impl Default for HazardResponse {
    fn default() -> Self {
        HazardResponse {
            action: HazardAction::None,
            forward_value: 0,
            forward_source: String::new(),
            stall_stages: 0,
            flush_count: 0,
            redirect_pc: 0,
        }
    }
}

/// Checks for hazards given the current pipeline stage contents.
/// The stages slice is ordered from first stage (IF) to last stage (WB).
pub type HazardFn = Box<dyn FnMut(&[Option<PipelineToken>]) -> HazardResponse>;

/// Predicts the next PC given the current PC.
/// Used by the IF stage for speculative fetch.
pub type PredictFn = Box<dyn FnMut(i64) -> i64>;

// =========================================================================
// Pipeline -- the main pipeline struct
// =========================================================================

/// A configurable N-stage instruction pipeline.
///
/// # How it Works
///
/// The pipeline is a slice of "slots", one per stage. Each slot holds an
/// optional [`PipelineToken`]. On each clock cycle (call to [`step()`]):
///
///  1. Check for hazards (via HazardFn callback)
///  2. If stalled: freeze stages before the stall point, insert bubble
///  3. If flushing: replace speculative stages with bubbles
///  4. Otherwise: shift all tokens one stage forward
///  5. Execute stage callbacks (fetch, decode, execute, memory, writeback)
///  6. Record a snapshot for tracing
///
/// All transitions happen "simultaneously" -- we compute the next state
/// from the current state, then swap.
///
/// # Example: 5-cycle execution of ADD instruction
///
/// ```text
/// Cycle 1: IF  -- fetch instruction at PC, ask branch predictor for next PC
/// Cycle 2: ID  -- decode: extract opcode=ADD, Rd=1, Rs1=2, Rs2=3
/// Cycle 3: EX  -- execute: ALUResult = Reg[2] + Reg[3]
/// Cycle 4: MEM -- memory: pass through (ADD doesn't access memory)
/// Cycle 5: WB  -- writeback: Reg[1] = ALUResult
/// ```
pub struct Pipeline {
    /// The pipeline configuration (stages, width).
    config: PipelineConfig,

    /// Current token in each pipeline stage.
    /// stages[0] is the first stage (IF), stages[N-1] is the last (WB).
    /// None means the stage is empty.
    stages: Vec<Option<PipelineToken>>,

    /// Current program counter (address of next instruction to fetch).
    pc: i64,

    /// Current clock cycle number.
    cycle: i64,

    /// True if a halt instruction has reached the last stage.
    halted: bool,

    /// Execution statistics.
    stats: PipelineStats,

    /// Snapshot history for tracing.
    history: Vec<PipelineSnapshot>,

    // --- Callbacks ---
    fetch_fn: FetchFn,
    decode_fn: DecodeFn,
    execute_fn: ExecuteFn,
    memory_fn: MemoryFn,
    writeback_fn: WritebackFn,

    // --- Optional callbacks ---
    hazard_fn: Option<HazardFn>,
    predict_fn: Option<PredictFn>,
}

impl Pipeline {
    /// Creates a new pipeline with the given configuration and callbacks.
    ///
    /// The configuration is validated before use. All five stage callbacks are
    /// required; hazard and predict callbacks are optional (set via setters).
    ///
    /// Returns an error if the configuration is invalid.
    pub fn new(
        config: PipelineConfig,
        fetch_fn: FetchFn,
        decode_fn: DecodeFn,
        execute_fn: ExecuteFn,
        memory_fn: MemoryFn,
        writeback_fn: WritebackFn,
    ) -> Result<Self, String> {
        config.validate()?;

        let num_stages = config.num_stages();
        Ok(Pipeline {
            config,
            stages: vec![None; num_stages],
            pc: 0,
            cycle: 0,
            halted: false,
            stats: PipelineStats::default(),
            history: Vec::new(),
            fetch_fn,
            decode_fn,
            execute_fn,
            memory_fn,
            writeback_fn,
            hazard_fn: None,
            predict_fn: None,
        })
    }

    /// Sets the optional hazard detection callback.
    pub fn set_hazard_fn(&mut self, f: HazardFn) {
        self.hazard_fn = Some(f);
    }

    /// Sets the optional branch prediction callback.
    pub fn set_predict_fn(&mut self, f: PredictFn) {
        self.predict_fn = Some(f);
    }

    /// Sets the program counter.
    pub fn set_pc(&mut self, pc: i64) {
        self.pc = pc;
    }

    /// Returns the current program counter.
    pub fn pc(&self) -> i64 {
        self.pc
    }

    /// Returns the current cycle number.
    pub fn cycle(&self) -> i64 {
        self.cycle
    }

    /// Returns true if a halt instruction has reached the last stage.
    pub fn is_halted(&self) -> bool {
        self.halted
    }

    /// Returns a copy of the current execution statistics.
    pub fn stats(&self) -> PipelineStats {
        self.stats.clone()
    }

    /// Returns the pipeline configuration.
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    /// Returns the token currently occupying the given stage.
    /// Returns None if the stage is empty or the stage name is invalid.
    pub fn stage_contents(&self, stage_name: &str) -> Option<&PipelineToken> {
        for (i, s) in self.config.stages.iter().enumerate() {
            if s.name == stage_name {
                return self.stages[i].as_ref();
            }
        }
        None
    }

    /// Returns the complete history of pipeline snapshots.
    pub fn trace(&self) -> Vec<PipelineSnapshot> {
        self.history.clone()
    }

    /// Returns a snapshot of the current pipeline state without advancing.
    pub fn snapshot(&self) -> PipelineSnapshot {
        self.take_snapshot()
    }

    /// Advances the pipeline by one clock cycle.
    ///
    /// This is the heart of the pipeline simulator. Each call to step()
    /// corresponds to one rising clock edge in hardware.
    ///
    /// # Step Algorithm
    ///
    ///  1. If halted, return the current snapshot (do nothing).
    ///  2. Increment the cycle counter.
    ///  3. Check for hazards by calling hazard_fn (if set).
    ///  4. Handle the hazard response (flush, stall, forward, or none).
    ///  5. Advance tokens through stages.
    ///  6. Execute stage callbacks on each token.
    ///  7. Update statistics.
    ///  8. Record a snapshot and return it.
    pub fn step(&mut self) -> PipelineSnapshot {
        if self.halted {
            return self.take_snapshot();
        }

        self.cycle += 1;
        self.stats.total_cycles += 1;
        let num_stages = self.config.num_stages();

        // --- Phase 1: Check for hazards ---
        //
        // The hazard function examines the CURRENT pipeline state (before any
        // advancement) and returns a verdict: stall, flush, forward, or proceed.
        let hazard = if let Some(ref mut hazard_fn) = self.hazard_fn {
            let stages_copy: Vec<Option<PipelineToken>> = self.stages.clone();
            hazard_fn(&stages_copy)
        } else {
            HazardResponse::default()
        };

        // --- Phase 2: Compute next state ---
        //
        // We build the next state in a new vec, then swap it in at the end.
        // This ensures all transitions are "simultaneous".
        let mut next_stages: Vec<Option<PipelineToken>> = vec![None; num_stages];
        let mut stalled = false;
        let mut flushing = false;

        match hazard.action {
            HazardAction::Flush => {
                // FLUSH: Replace speculative stages with bubbles.
                //
                // A flush happens when a branch misprediction is detected.
                // Everything before the stage that detected the misprediction
                // must be discarded.
                flushing = true;
                self.stats.flush_cycles += 1;

                // Determine how many stages to flush (from the front).
                let mut flush_count = hazard.flush_count;
                if flush_count == 0 {
                    for (i, s) in self.config.stages.iter().enumerate() {
                        if s.category == StageCategory::Execute {
                            flush_count = i;
                            break;
                        }
                    }
                    if flush_count == 0 {
                        flush_count = 1;
                    }
                }
                if flush_count > num_stages {
                    flush_count = num_stages;
                }

                // Shift non-flushed stages forward (from back to front).
                for i in (flush_count..num_stages).rev() {
                    if i > 0 && i - 1 >= flush_count {
                        next_stages[i] = self.stages[i - 1].clone();
                    } else if i > 0 {
                        let mut bubble = PipelineToken::new_bubble();
                        bubble.stage_entered.insert(
                            self.config.stages[i].name.clone(),
                            self.cycle,
                        );
                        next_stages[i] = Some(bubble);
                    } else {
                        next_stages[i] = self.stages[i].clone();
                    }
                }

                // Replace flushed stages with bubbles.
                for i in 0..flush_count {
                    let mut bubble = PipelineToken::new_bubble();
                    bubble.stage_entered.insert(
                        self.config.stages[i].name.clone(),
                        self.cycle,
                    );
                    next_stages[i] = Some(bubble);
                }

                // Redirect PC and fetch from the correct target.
                self.pc = hazard.redirect_pc;
                let tok = self.fetch_new_instruction();
                next_stages[0] = Some(tok);
            }

            HazardAction::Stall => {
                // STALL: Freeze earlier stages and insert a bubble.
                //
                // A stall happens when a data hazard cannot be resolved by
                // forwarding -- typically a load-use hazard.
                stalled = true;
                self.stats.stall_cycles += 1;

                // Find the stall insertion point.
                let mut stall_point = hazard.stall_stages;
                if stall_point == 0 {
                    for (i, s) in self.config.stages.iter().enumerate() {
                        if s.category == StageCategory::Execute {
                            stall_point = i;
                            break;
                        }
                    }
                    if stall_point == 0 {
                        stall_point = 1;
                    }
                }
                if stall_point >= num_stages {
                    stall_point = num_stages - 1;
                }

                // Stages AFTER the stall point advance normally.
                for i in (stall_point + 1..num_stages).rev() {
                    next_stages[i] = self.stages[i - 1].clone();
                }

                // Insert bubble at the stall point.
                let mut bubble = PipelineToken::new_bubble();
                bubble.stage_entered.insert(
                    self.config.stages[stall_point].name.clone(),
                    self.cycle,
                );
                next_stages[stall_point] = Some(bubble);

                // Stages BEFORE the stall point are frozen.
                for i in 0..stall_point {
                    next_stages[i] = self.stages[i].clone();
                }

                // PC does NOT advance during a stall.
            }

            _ => {
                // NONE or FORWARD: Normal advancement.
                //
                // Every token moves one stage forward. A new token is fetched
                // into the first stage.

                // Handle forwarding if needed.
                if hazard.action == HazardAction::ForwardFromEX
                    || hazard.action == HazardAction::ForwardFromMEM
                {
                    for (i, s) in self.config.stages.iter().enumerate() {
                        if s.category == StageCategory::Decode {
                            if let Some(ref mut tok) = self.stages[i] {
                                if !tok.is_bubble {
                                    tok.alu_result = hazard.forward_value;
                                    tok.forwarded_from = hazard.forward_source.clone();
                                    break;
                                }
                            }
                        }
                    }
                }

                // Shift tokens forward (from back to front).
                for i in (1..num_stages).rev() {
                    next_stages[i] = self.stages[i - 1].clone();
                }

                // Fetch new instruction into IF stage.
                let tok = self.fetch_new_instruction();
                next_stages[0] = Some(tok);
            }
        }

        // --- Phase 3: Commit the new state ---
        self.stages = next_stages;

        // --- Phase 4: Execute stage callbacks ---
        //
        // Now that all tokens are in their new positions, run the
        // stage-specific callbacks. We iterate from LAST to FIRST.
        for i in (0..num_stages).rev() {
            let should_process = if let Some(ref tok) = self.stages[i] {
                !tok.is_bubble
            } else {
                false
            };

            if !should_process {
                continue;
            }

            let stage_category = self.config.stages[i].category;
            let stage_name = self.config.stages[i].name.clone();
            let cycle = self.cycle;

            // Record when this token entered this stage.
            if let Some(ref mut tok) = self.stages[i] {
                tok.stage_entered.entry(stage_name.clone()).or_insert(cycle);
            }

            match stage_category {
                StageCategory::Fetch => {
                    // Already handled by fetch_new_instruction().
                }
                StageCategory::Decode => {
                    let should_decode = self.stages[i]
                        .as_ref()
                        .map(|t| t.opcode.is_empty())
                        .unwrap_or(false);
                    if should_decode {
                        if let Some(tok) = self.stages[i].take() {
                            let raw = tok.raw_instruction;
                            let decoded = (self.decode_fn)(raw, tok);
                            self.stages[i] = Some(decoded);
                        }
                    }
                }
                StageCategory::Execute => {
                    let entered_this_cycle = self.stages[i]
                        .as_ref()
                        .map(|t| t.stage_entered.get(&stage_name) == Some(&cycle))
                        .unwrap_or(false);
                    if entered_this_cycle {
                        if let Some(tok) = self.stages[i].take() {
                            let executed = (self.execute_fn)(tok);
                            self.stages[i] = Some(executed);
                        }
                    }
                }
                StageCategory::Memory => {
                    let entered_this_cycle = self.stages[i]
                        .as_ref()
                        .map(|t| t.stage_entered.get(&stage_name) == Some(&cycle))
                        .unwrap_or(false);
                    if entered_this_cycle {
                        if let Some(tok) = self.stages[i].take() {
                            let result = (self.memory_fn)(tok);
                            self.stages[i] = Some(result);
                        }
                    }
                }
                StageCategory::Writeback => {
                    // Writeback is handled in Phase 5 (retirement).
                }
            }
        }

        // --- Phase 5: Retire the instruction in the last stage ---
        //
        // The token that is NOW in the last stage (after advancement) gets
        // its writeback callback called. This is the "retirement" of the
        // instruction.
        let should_retire = self.stages[num_stages - 1]
            .as_ref()
            .map(|t| !t.is_bubble)
            .unwrap_or(false);

        if should_retire {
            if let Some(ref tok) = self.stages[num_stages - 1] {
                (self.writeback_fn)(tok);
                self.stats.instructions_completed += 1;
                if tok.is_halt {
                    self.halted = true;
                }
            }
        }

        // Count bubbles across all stages.
        for tok in &self.stages {
            if let Some(ref t) = tok {
                if t.is_bubble {
                    self.stats.bubble_cycles += 1;
                }
            }
        }

        // --- Phase 6: Take snapshot ---
        let mut snap = PipelineSnapshot {
            cycle: self.cycle,
            stages: HashMap::new(),
            stalled,
            flushing,
            pc: self.pc,
        };
        for (i, stage) in self.config.stages.iter().enumerate() {
            if let Some(ref tok) = self.stages[i] {
                snap.stages.insert(stage.name.clone(), tok.clone());
            }
        }
        self.history.push(snap.clone());

        snap
    }

    /// Creates a new token by calling the fetch callback.
    ///
    /// This is called at the start of each cycle to fetch the instruction
    /// at the current PC. The PC is then advanced (either by the branch
    /// predictor or by the default PC+4).
    fn fetch_new_instruction(&mut self) -> PipelineToken {
        let mut tok = PipelineToken::new();
        tok.pc = self.pc;
        tok.raw_instruction = (self.fetch_fn)(self.pc);
        tok.stage_entered
            .insert(self.config.stages[0].name.clone(), self.cycle);

        // Advance PC: use branch predictor if available, otherwise PC+4.
        if let Some(ref mut predict_fn) = self.predict_fn {
            self.pc = predict_fn(self.pc);
        } else {
            self.pc += 4;
        }

        tok
    }

    /// Executes the pipeline until a halt instruction is encountered or
    /// the maximum cycle count is reached.
    ///
    /// Returns the final execution statistics.
    pub fn run(&mut self, max_cycles: i64) -> PipelineStats {
        while self.cycle < max_cycles && !self.halted {
            self.step();
        }
        self.stats.clone()
    }

    /// Takes a snapshot of the current pipeline state.
    fn take_snapshot(&self) -> PipelineSnapshot {
        let mut stages_map = HashMap::new();
        for (i, stage) in self.config.stages.iter().enumerate() {
            if let Some(ref tok) = self.stages[i] {
                stages_map.insert(stage.name.clone(), tok.clone());
            }
        }
        PipelineSnapshot {
            cycle: self.cycle,
            stages: stages_map,
            stalled: false,
            flushing: false,
            pc: self.pc,
        }
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::PipelineConfig;
    use std::cell::RefCell;
    use std::rc::Rc;

    // =====================================================================
    // Test helpers -- simple instruction memory and callbacks
    // =====================================================================
    //
    // For testing, we create a tiny "instruction memory" -- just a vec of
    // integers. Each integer represents one instruction's raw bits.
    //
    // Encoding: raw = (opcode << 24) | (rd << 16) | (rs1 << 8) | rs2
    //
    // Opcodes:
    //   0x00 = NOP
    //   0x01 = ADD (register write)
    //   0x02 = LDR (load from memory)
    //   0x03 = STR (store to memory)
    //   0x04 = BEQ (branch if equal)
    //   0xFF = HALT

    const OP_NOP: i64 = 0x00;
    const OP_ADD: i64 = 0x01;
    const OP_LDR: i64 = 0x02;
    #[allow(dead_code)]
    const OP_STR: i64 = 0x03;
    const OP_BEQ: i64 = 0x04;
    const OP_HALT: i64 = 0xFF;

    fn make_instruction(opcode: i64, rd: i64, rs1: i64, rs2: i64) -> i64 {
        (opcode << 24) | (rd << 16) | (rs1 << 8) | rs2
    }

    fn simple_fetch(instrs: Vec<i64>) -> FetchFn {
        Box::new(move |pc: i64| -> i64 {
            let index = (pc / 4) as usize;
            if index < instrs.len() {
                instrs[index]
            } else {
                0 // NOP
            }
        })
    }

    fn simple_decode() -> DecodeFn {
        Box::new(|raw: i64, mut tok: PipelineToken| -> PipelineToken {
            let opcode = (raw >> 24) & 0xFF;
            let rd = (raw >> 16) & 0xFF;
            let rs1 = (raw >> 8) & 0xFF;
            let rs2 = raw & 0xFF;

            match opcode {
                0x01 => {
                    // ADD
                    tok.opcode = "ADD".to_string();
                    tok.rd = rd;
                    tok.rs1 = rs1;
                    tok.rs2 = rs2;
                    tok.reg_write = true;
                }
                0x02 => {
                    // LDR
                    tok.opcode = "LDR".to_string();
                    tok.rd = rd;
                    tok.rs1 = rs1;
                    tok.mem_read = true;
                    tok.reg_write = true;
                }
                0x03 => {
                    // STR
                    tok.opcode = "STR".to_string();
                    tok.rs1 = rs1;
                    tok.rs2 = rs2;
                    tok.mem_write = true;
                }
                0x04 => {
                    // BEQ
                    tok.opcode = "BEQ".to_string();
                    tok.rs1 = rs1;
                    tok.rs2 = rs2;
                    tok.is_branch = true;
                }
                0xFF => {
                    // HALT
                    tok.opcode = "HALT".to_string();
                    tok.is_halt = true;
                }
                _ => {
                    tok.opcode = "NOP".to_string();
                }
            }
            tok
        })
    }

    fn simple_execute() -> ExecuteFn {
        Box::new(|mut tok: PipelineToken| -> PipelineToken {
            match tok.opcode.as_str() {
                "ADD" => {
                    tok.alu_result = tok.rs1 + tok.rs2;
                }
                "LDR" => {
                    tok.alu_result = tok.rs1 + tok.immediate;
                }
                "STR" => {
                    tok.alu_result = tok.rs1 + tok.immediate;
                }
                "BEQ" => {
                    tok.branch_target = tok.pc + tok.immediate;
                }
                _ => {}
            }
            tok
        })
    }

    fn simple_memory() -> MemoryFn {
        Box::new(|mut tok: PipelineToken| -> PipelineToken {
            if tok.mem_read {
                tok.mem_data = 42;
                tok.write_data = tok.mem_data;
            } else {
                tok.write_data = tok.alu_result;
            }
            tok
        })
    }

    fn simple_writeback(completed: Rc<RefCell<Vec<i64>>>) -> WritebackFn {
        Box::new(move |tok: &PipelineToken| {
            completed.borrow_mut().push(tok.pc);
        })
    }

    fn simple_writeback_noop() -> WritebackFn {
        Box::new(|_tok: &PipelineToken| {})
    }

    fn new_test_pipeline(
        instrs: Vec<i64>,
        completed: Option<Rc<RefCell<Vec<i64>>>>,
    ) -> Pipeline {
        let config = PipelineConfig::classic_5_stage();
        let wb: WritebackFn = match completed {
            Some(c) => simple_writeback(c),
            None => simple_writeback_noop(),
        };
        Pipeline::new(
            config,
            simple_fetch(instrs),
            simple_decode(),
            simple_execute(),
            simple_memory(),
            wb,
        )
        .expect("failed to create test pipeline")
    }

    // =====================================================================
    // Token tests (already in token.rs, but pipeline-specific here)
    // =====================================================================

    // =====================================================================
    // Basic Pipeline tests
    // =====================================================================

    #[test]
    fn test_new_pipeline() {
        let instrs = vec![make_instruction(OP_ADD, 1, 2, 3)];
        let p = new_test_pipeline(instrs, None);

        assert!(!p.is_halted());
        assert_eq!(p.cycle(), 0);
        assert_eq!(p.pc(), 0);
    }

    #[test]
    fn test_new_pipeline_invalid_config() {
        let cfg = PipelineConfig {
            stages: vec![crate::token::PipelineStage {
                name: "IF".to_string(),
                description: "".to_string(),
                category: StageCategory::Fetch,
            }],
            execution_width: 1,
        };
        let result = Pipeline::new(
            cfg,
            Box::new(|_| 0),
            Box::new(|_, t| t),
            Box::new(|t| t),
            Box::new(|t| t),
            Box::new(|_| {}),
        );
        assert!(result.is_err());
    }

    /// Verifies that a single instruction progresses through all 5 stages
    /// in 5 cycles.
    ///
    /// ```text
    /// Cycle 1: ADD enters IF
    /// Cycle 2: ADD enters ID
    /// Cycle 3: ADD enters EX
    /// Cycle 4: ADD enters MEM
    /// Cycle 5: ADD enters WB and retires
    /// ```
    #[test]
    fn test_single_instruction_flows_through_5_stages() {
        let instrs = vec![
            make_instruction(OP_ADD, 1, 2, 3),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
        ];

        let completed = Rc::new(RefCell::new(Vec::new()));
        let mut p = new_test_pipeline(instrs, Some(completed.clone()));

        for _ in 0..5 {
            p.step();
        }

        assert!(
            !completed.borrow().is_empty(),
            "expected at least one instruction to complete after 5 cycles"
        );
        assert_eq!(completed.borrow()[0], 0, "first completed should be at PC=0");
    }

    /// Verifies pipeline fill timing: first instruction completes at cycle 5,
    /// subsequent instructions complete one per cycle.
    ///
    /// ```text
    /// Cycle:  1    2    3    4    5    6    7
    /// IF:    I1   I2   I3   I4   I5   I6   I7
    /// ID:    --   I1   I2   I3   I4   I5   I6
    /// EX:    --   --   I1   I2   I3   I4   I5
    /// MEM:   --   --   --   I1   I2   I3   I4
    /// WB:    --   --   --   --   I1   I2   I3
    ///                           ^1st  ^2nd  ^3rd
    /// ```
    #[test]
    fn test_pipeline_fill_timing() {
        let instrs: Vec<i64> = (0..20).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let completed = Rc::new(RefCell::new(Vec::new()));
        let mut p = new_test_pipeline(instrs, Some(completed.clone()));

        // After 4 cycles, nothing should have completed yet.
        for _ in 0..4 {
            p.step();
        }
        assert_eq!(completed.borrow().len(), 0, "0 completions after 4 cycles");

        // After cycle 5, exactly 1 instruction.
        p.step();
        assert_eq!(completed.borrow().len(), 1, "1 completion after 5 cycles");

        // After cycle 6, 2 completions.
        p.step();
        assert_eq!(completed.borrow().len(), 2, "2 completions after 6 cycles");

        // After cycle 7, 3 completions.
        p.step();
        assert_eq!(completed.borrow().len(), 3, "3 completions after 7 cycles");
    }

    /// Verifies that after the pipeline fills, IPC approaches 1.0.
    #[test]
    fn test_steady_state_ipc() {
        let instrs: Vec<i64> = (0..100).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let mut p = new_test_pipeline(instrs, None);

        for _ in 0..50 {
            p.step();
        }

        let stats = p.stats();
        let expected_completed = 50 - 5 + 1; // 46
        assert_eq!(
            stats.instructions_completed, expected_completed as i64,
            "expected {} completions after 50 cycles",
            expected_completed
        );

        let ipc = stats.ipc();
        assert!(
            ipc > 0.85 && ipc < 1.01,
            "expected IPC near 1.0, got {:.3}",
            ipc
        );
    }

    /// Verifies that a HALT instruction reaches WB and stops the pipeline.
    #[test]
    fn test_halt_propagation() {
        let instrs = vec![
            make_instruction(OP_ADD, 1, 2, 3),
            make_instruction(OP_ADD, 4, 5, 6),
            make_instruction(OP_HALT, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
        ];

        let completed = Rc::new(RefCell::new(Vec::new()));
        let mut p = new_test_pipeline(instrs, Some(completed.clone()));

        let stats = p.run(100);

        assert!(p.is_halted(), "pipeline should be halted");
        assert_eq!(p.cycle(), 7, "expected halt at cycle 7");
        assert_eq!(
            stats.instructions_completed, 3,
            "expected 3 completions (2 ADD + 1 HALT)"
        );
    }

    /// Verifies that stepping an empty pipeline works without panic.
    #[test]
    fn test_empty_pipeline() {
        let instrs: Vec<i64> = vec![];
        let mut p = new_test_pipeline(instrs, None);

        let snap = p.step();
        assert_eq!(snap.cycle, 1);
    }

    // =====================================================================
    // Stall tests
    // =====================================================================

    /// Verifies that during a stall, IF and ID are frozen and a bubble
    /// is inserted at EX.
    #[test]
    fn test_stall_freezes_earlier_stages() {
        let instrs = vec![
            make_instruction(OP_LDR, 1, 2, 0),
            make_instruction(OP_ADD, 3, 1, 4),
            make_instruction(OP_ADD, 5, 6, 7),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
        ];

        let completed = Rc::new(RefCell::new(Vec::new()));
        let mut p = new_test_pipeline(instrs, Some(completed.clone()));

        let stall_injected = Rc::new(RefCell::new(false));
        let stall_injected_clone = stall_injected.clone();

        p.set_hazard_fn(Box::new(
            move |stages: &[Option<PipelineToken>]| -> HazardResponse {
                if !*stall_injected_clone.borrow() && stages.len() >= 3 {
                    if let (Some(ex_tok), Some(id_tok)) = (&stages[2], &stages[1]) {
                        if !ex_tok.is_bubble
                            && ex_tok.opcode == "LDR"
                            && !id_tok.is_bubble
                            && id_tok.opcode == "ADD"
                        {
                            *stall_injected_clone.borrow_mut() = true;
                            return HazardResponse {
                                action: HazardAction::Stall,
                                stall_stages: 2,
                                ..Default::default()
                            };
                        }
                    }
                }
                HazardResponse::default()
            },
        ));

        p.step(); // cycle 1
        p.step(); // cycle 2
        p.step(); // cycle 3

        // At cycle 4, LDR should be in EX and ADD in ID -> STALL
        let snap = p.step(); // cycle 4

        assert!(snap.stalled, "expected stall at cycle 4");

        // EX should have a bubble.
        let ex_tok = p.stage_contents("EX");
        assert!(
            ex_tok.is_some() && ex_tok.unwrap().is_bubble,
            "expected bubble in EX after stall"
        );

        // ID should still contain ADD (frozen).
        let id_tok = p.stage_contents("ID");
        assert!(
            id_tok.is_some() && id_tok.unwrap().opcode == "ADD",
            "expected ADD to remain in ID (frozen)"
        );

        assert_eq!(p.stats().stall_cycles, 1, "expected 1 stall cycle");
    }

    /// Verifies bubble insertion at the correct stage during a stall.
    #[test]
    fn test_stall_bubble_insertion() {
        let instrs: Vec<i64> = (0..10).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let mut p = new_test_pipeline(instrs, None);

        let stall_count = Rc::new(RefCell::new(0));
        let stall_count_clone = stall_count.clone();

        p.set_hazard_fn(Box::new(
            move |_stages: &[Option<PipelineToken>]| -> HazardResponse {
                *stall_count_clone.borrow_mut() += 1;
                if *stall_count_clone.borrow() == 3 {
                    return HazardResponse {
                        action: HazardAction::Stall,
                        stall_stages: 2,
                        ..Default::default()
                    };
                }
                HazardResponse::default()
            },
        ));

        for _ in 0..3 {
            p.step();
        }

        let ex_tok = p.stage_contents("EX");
        assert!(
            ex_tok.is_some() && ex_tok.unwrap().is_bubble,
            "expected bubble in EX after stall"
        );
    }

    // =====================================================================
    // Flush tests
    // =====================================================================

    /// Verifies that a flush replaces speculative stages with bubbles
    /// and redirects the PC.
    #[test]
    fn test_flush_replaces_with_bubbles() {
        let instrs = vec![
            make_instruction(OP_BEQ, 0, 1, 2),
            make_instruction(OP_ADD, 1, 2, 3),
            make_instruction(OP_ADD, 4, 5, 6),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_ADD, 7, 8, 9), // PC=20
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
        ];

        let mut p = new_test_pipeline(instrs, None);

        let flushed = Rc::new(RefCell::new(false));
        let flushed_clone = flushed.clone();

        p.set_hazard_fn(Box::new(
            move |stages: &[Option<PipelineToken>]| -> HazardResponse {
                if !*flushed_clone.borrow() && stages.len() >= 3 {
                    if let Some(ref ex_tok) = stages[2] {
                        if !ex_tok.is_bubble && ex_tok.is_branch {
                            *flushed_clone.borrow_mut() = true;
                            return HazardResponse {
                                action: HazardAction::Flush,
                                flush_count: 2,
                                redirect_pc: 20,
                                ..Default::default()
                            };
                        }
                    }
                }
                HazardResponse::default()
            },
        ));

        p.step(); // cycle 1
        p.step(); // cycle 2
        p.step(); // cycle 3

        let snap = p.step(); // cycle 4 -- flush
        assert!(snap.flushing, "expected flush at cycle 4");

        // After flush, PC should be 24 (20 + 4).
        assert_eq!(p.pc(), 24, "expected PC=24 after flush");

        assert_eq!(p.stats().flush_cycles, 1, "expected 1 flush cycle");
    }

    // =====================================================================
    // Forwarding tests
    // =====================================================================

    /// Verifies that forwarding callback updates the token.
    #[test]
    fn test_forwarding_applied() {
        let instrs: Vec<i64> = (0..10).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let mut p = new_test_pipeline(instrs, None);

        let forward_cycle = Rc::new(RefCell::new(0));
        let forward_cycle_clone = forward_cycle.clone();

        p.set_hazard_fn(Box::new(
            move |_stages: &[Option<PipelineToken>]| -> HazardResponse {
                *forward_cycle_clone.borrow_mut() += 1;
                if *forward_cycle_clone.borrow() == 4 {
                    return HazardResponse {
                        action: HazardAction::ForwardFromEX,
                        forward_value: 99,
                        forward_source: "EX".to_string(),
                        ..Default::default()
                    };
                }
                HazardResponse::default()
            },
        ));

        for _ in 0..4 {
            p.step();
        }

        let ex_tok = p.stage_contents("EX").expect("expected token in EX");
        assert_eq!(
            ex_tok.forwarded_from, "EX",
            "expected ForwardedFrom='EX'"
        );
    }

    /// Verifies forwarding from MEM stage.
    #[test]
    fn test_forward_from_mem() {
        let instrs: Vec<i64> = (0..10).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let mut p = new_test_pipeline(instrs, None);

        let forward_cycle = Rc::new(RefCell::new(0));
        let forward_cycle_clone = forward_cycle.clone();

        p.set_hazard_fn(Box::new(
            move |_stages: &[Option<PipelineToken>]| -> HazardResponse {
                *forward_cycle_clone.borrow_mut() += 1;
                if *forward_cycle_clone.borrow() == 4 {
                    return HazardResponse {
                        action: HazardAction::ForwardFromMEM,
                        forward_value: 77,
                        forward_source: "MEM".to_string(),
                        ..Default::default()
                    };
                }
                HazardResponse::default()
            },
        ));

        for _ in 0..4 {
            p.step();
        }

        let ex_tok = p.stage_contents("EX").expect("expected token in EX");
        assert_eq!(ex_tok.forwarded_from, "MEM");
    }

    // =====================================================================
    // Statistics tests
    // =====================================================================

    /// Verifies that stalls reduce IPC below 1.0.
    #[test]
    fn test_stall_reduces_ipc() {
        let instrs: Vec<i64> = (0..50).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let mut p = new_test_pipeline(instrs, None);

        let cycle_count = Rc::new(RefCell::new(0));
        let cycle_count_clone = cycle_count.clone();

        p.set_hazard_fn(Box::new(
            move |_stages: &[Option<PipelineToken>]| -> HazardResponse {
                *cycle_count_clone.borrow_mut() += 1;
                if *cycle_count_clone.borrow() % 5 == 0 {
                    return HazardResponse {
                        action: HazardAction::Stall,
                        stall_stages: 2,
                        ..Default::default()
                    };
                }
                HazardResponse::default()
            },
        ));

        for _ in 0..30 {
            p.step();
        }

        let stats = p.stats();
        assert!(stats.ipc() < 1.0, "expected IPC < 1.0 with stalls");
        assert!(stats.stall_cycles > 0, "expected nonzero stall cycles");
    }

    // =====================================================================
    // Trace and Snapshot tests
    // =====================================================================

    /// Verifies snapshots correctly reflect pipeline contents.
    #[test]
    fn test_snapshot_accuracy() {
        let instrs = vec![
            make_instruction(OP_ADD, 1, 2, 3),
            make_instruction(OP_ADD, 4, 5, 6),
            make_instruction(OP_NOP, 0, 0, 0),
        ];

        let mut p = new_test_pipeline(instrs, None);

        let snap1 = p.step();
        assert_eq!(snap1.cycle, 1);
        let if_tok = snap1.stages.get("IF").expect("token in IF at cycle 1");
        assert_eq!(if_tok.pc, 0);

        let snap2 = p.step();
        assert_eq!(snap2.cycle, 2);
        let id_tok = snap2.stages.get("ID").expect("token in ID at cycle 2");
        assert_eq!(id_tok.pc, 0);
    }

    /// Verifies that trace records every cycle's state.
    #[test]
    fn test_trace_completeness() {
        let instrs: Vec<i64> = (0..10).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let mut p = new_test_pipeline(instrs, None);

        for _ in 0..7 {
            p.step();
        }

        let trace = p.trace();
        assert_eq!(trace.len(), 7, "expected 7 trace entries");

        for (i, snap) in trace.iter().enumerate() {
            let expected = (i + 1) as i64;
            assert_eq!(snap.cycle, expected, "trace[{}] cycle mismatch", i);
        }
    }

    /// Verifies snapshot does not advance the pipeline.
    #[test]
    fn test_snapshot_does_not_advance() {
        let instrs = vec![make_instruction(OP_ADD, 1, 2, 3)];
        let mut p = new_test_pipeline(instrs, None);

        p.step();
        let snap1 = p.snapshot();
        let snap2 = p.snapshot();

        assert_eq!(snap1.cycle, snap2.cycle, "snapshot should not advance");
    }

    // =====================================================================
    // Configuration preset tests
    // =====================================================================

    /// Verifies that a deeper pipeline takes more cycles to fill.
    #[test]
    fn test_deep_pipeline_longer_fill_time() {
        let config = PipelineConfig::deep_13_stage();
        let instrs: Vec<i64> = (0..30).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let mut p = Pipeline::new(
            config,
            simple_fetch(instrs),
            simple_decode(),
            simple_execute(),
            simple_memory(),
            simple_writeback_noop(),
        )
        .unwrap();

        for _ in 0..12 {
            p.step();
        }
        assert_eq!(
            p.stats().instructions_completed, 0,
            "0 completions after 12 cycles in 13-stage"
        );

        p.step();
        assert_eq!(
            p.stats().instructions_completed, 1,
            "1 completion after 13 cycles in 13-stage"
        );
    }

    /// Verifies custom stage configurations work correctly.
    #[test]
    fn test_custom_stage_configuration() {
        let config = PipelineConfig {
            stages: vec![
                crate::token::PipelineStage {
                    name: "IF".to_string(),
                    description: "Fetch".to_string(),
                    category: StageCategory::Fetch,
                },
                crate::token::PipelineStage {
                    name: "EX".to_string(),
                    description: "Execute".to_string(),
                    category: StageCategory::Execute,
                },
                crate::token::PipelineStage {
                    name: "WB".to_string(),
                    description: "Writeback".to_string(),
                    category: StageCategory::Writeback,
                },
            ],
            execution_width: 1,
        };

        let instrs: Vec<i64> = (0..10).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let completed = Rc::new(RefCell::new(Vec::new()));
        let completed_clone = completed.clone();

        let mut p = Pipeline::new(
            config,
            simple_fetch(instrs),
            simple_decode(),
            simple_execute(),
            simple_memory(),
            simple_writeback(completed_clone),
        )
        .unwrap();

        // 3-stage: first completion at cycle 3.
        for _ in 0..2 {
            p.step();
        }
        assert_eq!(completed.borrow().len(), 0, "0 completions after 2 cycles");

        p.step();
        assert_eq!(completed.borrow().len(), 1, "1 completion after 3 cycles");
    }

    // =====================================================================
    // Branch prediction tests
    // =====================================================================

    /// Verifies the predict callback determines the next PC.
    #[test]
    fn test_branch_predictor_integration() {
        let instrs: Vec<i64> = (0..100).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let mut p = new_test_pipeline(instrs, None);

        p.set_predict_fn(Box::new(|pc: i64| -> i64 { pc + 8 }));

        p.step(); // cycle 1: fetches PC=0, predicts next=8
        assert_eq!(p.pc(), 8, "expected PC=8 after prediction");

        p.step(); // cycle 2: fetches PC=8, predicts next=16
        assert_eq!(p.pc(), 16, "expected PC=16 after second prediction");
    }

    // =====================================================================
    // SetPC test
    // =====================================================================

    #[test]
    fn test_set_pc() {
        let instrs: Vec<i64> = (0..10).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();
        let mut p = new_test_pipeline(instrs, None);
        p.set_pc(100);
        assert_eq!(p.pc(), 100);
    }

    // =====================================================================
    // Halted pipeline test
    // =====================================================================

    /// Verifies stepping a halted pipeline does not change state.
    #[test]
    fn test_halted_pipeline_does_not_advance() {
        let instrs = vec![
            make_instruction(OP_HALT, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
        ];

        let mut p = new_test_pipeline(instrs, None);
        p.run(100);

        let cycle_at_halt = p.cycle();
        let completed_at_halt = p.stats().instructions_completed;

        p.step();
        p.step();

        assert_eq!(p.cycle(), cycle_at_halt, "cycle should not change after halt");
        assert_eq!(
            p.stats().instructions_completed,
            completed_at_halt,
            "completions should not change after halt"
        );
    }

    // =====================================================================
    // HazardAction tests
    // =====================================================================

    #[test]
    fn test_hazard_action_string() {
        assert_eq!(HazardAction::None.to_string(), "NONE");
        assert_eq!(HazardAction::ForwardFromEX.to_string(), "FORWARD_FROM_EX");
        assert_eq!(HazardAction::ForwardFromMEM.to_string(), "FORWARD_FROM_MEM");
        assert_eq!(HazardAction::Stall.to_string(), "STALL");
        assert_eq!(HazardAction::Flush.to_string(), "FLUSH");
    }

    // =====================================================================
    // Config returns test
    // =====================================================================

    #[test]
    fn test_pipeline_config() {
        let instrs = vec![make_instruction(OP_NOP, 0, 0, 0)];
        let p = new_test_pipeline(instrs, None);
        assert_eq!(p.config().num_stages(), 5);
    }

    // =====================================================================
    // Multiple stalls and flushes
    // =====================================================================

    #[test]
    fn test_multiple_stalls_and_flushes() {
        let instrs: Vec<i64> = (0..50).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let mut p = new_test_pipeline(instrs, None);

        let cycle_counter = Rc::new(RefCell::new(0));
        let cycle_counter_clone = cycle_counter.clone();

        p.set_hazard_fn(Box::new(
            move |_stages: &[Option<PipelineToken>]| -> HazardResponse {
                *cycle_counter_clone.borrow_mut() += 1;
                let c = *cycle_counter_clone.borrow();
                if c == 5 || c == 10 {
                    return HazardResponse {
                        action: HazardAction::Stall,
                        stall_stages: 2,
                        ..Default::default()
                    };
                }
                if c == 15 {
                    return HazardResponse {
                        action: HazardAction::Flush,
                        flush_count: 2,
                        redirect_pc: 0,
                        ..Default::default()
                    };
                }
                HazardResponse::default()
            },
        ));

        for _ in 0..20 {
            p.step();
        }

        let stats = p.stats();
        assert_eq!(stats.stall_cycles, 2, "expected 2 stall cycles");
        assert_eq!(stats.flush_cycles, 1, "expected 1 flush cycle");
    }

    // =====================================================================
    // Run max cycles test
    // =====================================================================

    #[test]
    fn test_run_max_cycles() {
        let instrs: Vec<i64> = (0..100).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let mut p = new_test_pipeline(instrs, None);
        let stats = p.run(10);

        assert_eq!(stats.total_cycles, 10, "expected 10 total cycles");
        assert!(!p.is_halted(), "should not be halted");
    }

    // =====================================================================
    // StageContents test
    // =====================================================================

    #[test]
    fn test_stage_contents_invalid_name() {
        let instrs = vec![make_instruction(OP_NOP, 0, 0, 0)];
        let mut p = new_test_pipeline(instrs, None);
        p.step();

        assert!(
            p.stage_contents("NONEXISTENT").is_none(),
            "expected None for nonexistent stage"
        );
    }

    // =====================================================================
    // Flush with default flush count test
    // =====================================================================

    #[test]
    fn test_flush_default_flush_count() {
        let instrs: Vec<i64> = (0..20).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let mut p = new_test_pipeline(instrs, None);

        let flushed = Rc::new(RefCell::new(false));
        let flushed_clone = flushed.clone();

        p.set_hazard_fn(Box::new(
            move |stages: &[Option<PipelineToken>]| -> HazardResponse {
                if !*flushed_clone.borrow() {
                    if stages.len() >= 3 {
                        if let Some(ref tok) = stages[2] {
                            if !tok.is_bubble {
                                *flushed_clone.borrow_mut() = true;
                                return HazardResponse {
                                    action: HazardAction::Flush,
                                    flush_count: 0, // Use default
                                    redirect_pc: 100,
                                    ..Default::default()
                                };
                            }
                        }
                    }
                }
                HazardResponse::default()
            },
        ));

        for _ in 0..5 {
            p.step();
        }

        assert_eq!(p.stats().flush_cycles, 1, "expected 1 flush cycle");
    }

    // =====================================================================
    // Stall with default stall point test
    // =====================================================================

    #[test]
    fn test_stall_default_stall_point() {
        let instrs: Vec<i64> = (0..20).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let mut p = new_test_pipeline(instrs, None);

        let stall_count = Rc::new(RefCell::new(0));
        let stall_count_clone = stall_count.clone();

        p.set_hazard_fn(Box::new(
            move |_stages: &[Option<PipelineToken>]| -> HazardResponse {
                *stall_count_clone.borrow_mut() += 1;
                if *stall_count_clone.borrow() == 3 {
                    return HazardResponse {
                        action: HazardAction::Stall,
                        stall_stages: 0, // Use default
                        ..Default::default()
                    };
                }
                HazardResponse::default()
            },
        ));

        for _ in 0..5 {
            p.step();
        }

        assert_eq!(p.stats().stall_cycles, 1, "expected 1 stall cycle");
    }

    // =====================================================================
    // Edge cases: flush/stall count larger than pipeline
    // =====================================================================

    #[test]
    fn test_flush_count_larger_than_pipeline() {
        let instrs: Vec<i64> = (0..20).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let mut p = new_test_pipeline(instrs, None);

        let flushed = Rc::new(RefCell::new(false));
        let flushed_clone = flushed.clone();

        p.set_hazard_fn(Box::new(
            move |stages: &[Option<PipelineToken>]| -> HazardResponse {
                if !*flushed_clone.borrow() {
                    if let Some(ref tok) = stages[2] {
                        if !tok.is_bubble {
                            *flushed_clone.borrow_mut() = true;
                            return HazardResponse {
                                action: HazardAction::Flush,
                                flush_count: 100, // Way too many -- should be clamped
                                redirect_pc: 0,
                                ..Default::default()
                            };
                        }
                    }
                }
                HazardResponse::default()
            },
        ));

        // Should not panic.
        for _ in 0..10 {
            p.step();
        }
    }

    #[test]
    fn test_stall_point_larger_than_pipeline() {
        let instrs: Vec<i64> = (0..20).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let mut p = new_test_pipeline(instrs, None);

        let stall_count = Rc::new(RefCell::new(0));
        let stall_count_clone = stall_count.clone();

        p.set_hazard_fn(Box::new(
            move |_stages: &[Option<PipelineToken>]| -> HazardResponse {
                *stall_count_clone.borrow_mut() += 1;
                if *stall_count_clone.borrow() == 3 {
                    return HazardResponse {
                        action: HazardAction::Stall,
                        stall_stages: 100, // Way too large -- should be clamped
                        ..Default::default()
                    };
                }
                HazardResponse::default()
            },
        ));

        // Should not panic.
        for _ in 0..10 {
            p.step();
        }
    }

    // =====================================================================
    // No hazard func test
    // =====================================================================

    #[test]
    fn test_pipeline_with_no_hazard_func() {
        let instrs: Vec<i64> = (0..20).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let mut p = new_test_pipeline(instrs, None);

        for _ in 0..10 {
            p.step();
        }

        let stats = p.stats();
        assert_eq!(stats.stall_cycles, 0, "expected 0 stall cycles");
        assert_eq!(stats.flush_cycles, 0, "expected 0 flush cycles");
    }

    // =====================================================================
    // Two-stage pipeline test
    // =====================================================================

    #[test]
    fn test_two_stage_pipeline() {
        let config = PipelineConfig {
            stages: vec![
                crate::token::PipelineStage {
                    name: "IF".to_string(),
                    description: "Fetch".to_string(),
                    category: StageCategory::Fetch,
                },
                crate::token::PipelineStage {
                    name: "WB".to_string(),
                    description: "Writeback".to_string(),
                    category: StageCategory::Writeback,
                },
            ],
            execution_width: 1,
        };

        let instrs: Vec<i64> = (0..10).map(|_| make_instruction(OP_ADD, 1, 2, 3)).collect();

        let completed = Rc::new(RefCell::new(Vec::new()));
        let completed_clone = completed.clone();

        let mut p = Pipeline::new(
            config,
            simple_fetch(instrs),
            simple_decode(),
            simple_execute(),
            simple_memory(),
            simple_writeback(completed_clone),
        )
        .unwrap();

        p.step(); // cycle 1
        assert_eq!(completed.borrow().len(), 0, "0 completions after 1 cycle");

        p.step(); // cycle 2
        assert_eq!(completed.borrow().len(), 1, "1 completion after 2 cycles");
    }

    // =====================================================================
    // Decode stage test
    // =====================================================================

    #[test]
    fn test_decode_stage() {
        let instrs = vec![
            make_instruction(OP_LDR, 5, 3, 0),
            make_instruction(OP_NOP, 0, 0, 0),
        ];

        let mut p = new_test_pipeline(instrs, None);

        p.step(); // cycle 1: LDR enters IF
        p.step(); // cycle 2: LDR moves to ID, gets decoded

        let id_tok = p.stage_contents("ID").expect("token in ID");
        assert_eq!(id_tok.opcode, "LDR");
        assert_eq!(id_tok.rd, 5);
        assert!(id_tok.mem_read);
        assert!(id_tok.reg_write);
    }

    // =====================================================================
    // Instruction count verification
    // =====================================================================

    #[test]
    fn test_instruction_count_matches_completions() {
        let instrs = vec![
            make_instruction(OP_ADD, 1, 2, 3),
            make_instruction(OP_ADD, 4, 5, 6),
            make_instruction(OP_ADD, 7, 8, 9),
            make_instruction(OP_HALT, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
            make_instruction(OP_NOP, 0, 0, 0),
        ];

        let completed = Rc::new(RefCell::new(Vec::new()));
        let mut p = new_test_pipeline(instrs, Some(completed.clone()));
        let stats = p.run(100);

        assert_eq!(
            stats.instructions_completed,
            completed.borrow().len() as i64,
            "stats vs callback count mismatch"
        );
        assert_eq!(
            stats.instructions_completed, 4,
            "expected 4 completed (3 ADD + 1 HALT)"
        );
    }
}
