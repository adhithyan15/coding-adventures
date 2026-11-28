//! Core -- a configurable processor core.
//!
//! Composes all D-series sub-components into a complete processor core:
//!
//!   - Pipeline (D04): manages instruction flow through stages
//!   - Branch Predictor (D02): speculative fetch direction
//!   - Hazard Unit (D03): data, control, and structural hazard detection
//!   - Cache Hierarchy (D01): L1I + L1D + optional L2
//!   - Register File: fast operand storage
//!   - Clock: cycle-accurate timing
//!   - Memory Controller: access to backing memory
//!
//! The Core wires these together by providing callback functions to the
//! pipeline. When the pipeline needs to fetch an instruction, it calls the
//! Core's fetch callback, which reads from the L1I cache. And so on.
//!
//! # ISA Independence
//!
//! The Core does not know what instructions mean. The ISADecoder provides
//! instruction semantics. The same Core can run ARM, RISC-V, or any custom
//! ISA by swapping the decoder.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use branch_predictor::{BranchPredictor, BranchTargetBuffer};
use cpu_cache::{Cache, CacheConfig, CacheHierarchy};
use clock::Clock;
use cpu_pipeline::{
    ForwardedOperand, HazardAction, HazardResponse, OperandForwardingStage, OperandSlot, Pipeline,
    PipelineConfig, PipelineSnapshot, PipelineToken, StageCategory,
};
use hazard_detection::hazard_unit::HazardUnit;
use hazard_detection::types::{HazardAction as HazardDetectionAction, PipelineSlot};

use crate::config::{create_branch_predictor, CoreConfig};
use crate::decoder::ISADecoder;
use crate::memory_controller::MemoryController;
use crate::register_file::RegisterFile;
use crate::stats::CoreStats;

/// Shared mutable state for the Core, wrapped in Rc<RefCell<...>> to allow
/// multiple closures (pipeline callbacks) to borrow it.
///
/// This is the Rust way to handle the "multiple closures need mutable access
/// to the same data" pattern that Go handles with method receivers. The
/// RefCell provides runtime borrow checking instead of compile-time checking,
/// which is necessary because the pipeline calls its callbacks in an order
/// determined at runtime.
struct CoreState {
    /// Register file for the core.
    reg_file: RegisterFile,

    /// Memory controller.
    mem_ctrl: MemoryController,

    /// Cache hierarchy (L1I, L1D, optional L2).
    cache_hierarchy: CacheHierarchy,

    /// Branch predictor.
    predictor: Box<dyn BranchPredictor>,

    /// Branch Target Buffer.
    btb: BranchTargetBuffer,

    /// Hazard detection unit.
    hazard_unit: HazardUnit,

    /// ISA decoder (trait object for runtime polymorphism).
    decoder: Box<dyn ISADecoder>,

    /// Current cycle number.
    cycle: i64,

    /// Total forwarding operations.
    forward_count: i64,

    /// Total stall cycles.
    stall_count: i64,

    /// Total pipeline flushes.
    flush_count: i64,

    /// Pipeline configuration (needed in hazard callback).
    pipeline_config: PipelineConfig,

    /// Whether the opt-in operand-indexed hazard boundary is active.
    operand_indexed_forwarding: bool,
}

/// A complete processor core that composes all D-series sub-components.
///
/// # Construction
///
/// ```text
///   let config = core::simple_config();
///   let decoder = Box::new(core::MockDecoder::new());
///   let c = core::Core::new(config, decoder).unwrap();
/// ```
///
/// # Execution
///
/// ```text
///   c.step();               // advance one clock cycle
///   let stats = c.run(1000); // run up to 1000 cycles
/// ```
pub struct Core {
    /// Core configuration.
    config: CoreConfig,

    /// Pipeline engine.
    pipeline: Pipeline,

    /// Shared mutable state accessed by pipeline callbacks.
    state: Rc<RefCell<CoreState>>,

    /// System clock.
    _clk: Clock,

    /// Whether a HALT instruction has completed.
    halted: bool,

    /// Current cycle number (mirrors state.cycle for public access).
    cycle: i64,

    /// Count of retired instructions.
    instructions_completed: i64,
}

impl Core {
    /// Creates a fully-wired processor core from the given configuration
    /// and ISA decoder.
    ///
    /// Returns an error if the pipeline configuration is invalid.
    pub fn new(config: CoreConfig, decoder: Box<dyn ISADecoder>) -> Result<Self, String> {
        Self::new_internal(config, decoder, false)
    }

    /// Creates a core with the opt-in operand-indexed EX/MEM bypass path.
    ///
    /// Unlike [`Core::new`], this path sends one named value per matching
    /// source to `ISADecoder::execute_with_forwarding`. It requires both
    /// forwarding and hazard detection in the configuration so a load-use
    /// dependency is held until its MEM value is available. The opt-in path
    /// currently supports only a scalar, ordered Fetch/Decode/Execute/Memory/
    /// Writeback pipeline; split or reordered stage layouts are rejected.
    pub fn new_with_operand_forwarding(
        config: CoreConfig,
        decoder: Box<dyn ISADecoder>,
    ) -> Result<Self, String> {
        if !config.forwarding || !config.hazard_detection {
            return Err(
                "operand-indexed forwarding requires forwarding and hazard detection".to_string(),
            );
        }
        if !supports_operand_forwarding_topology(&config) {
            return Err(
                "operand-indexed forwarding requires scalar Fetch/Decode/Execute/Memory/Writeback stages in order".to_string(),
            );
        }
        Self::new_internal(config, decoder, true)
    }

    fn new_internal(
        config: CoreConfig,
        decoder: Box<dyn ISADecoder>,
        operand_indexed_forwarding: bool,
    ) -> Result<Self, String> {
        // --- 1. Register File ---
        let reg_file = RegisterFile::new(config.register_file.as_ref());

        // --- 2. Memory ---
        let mem_size = if config.memory_size == 0 { 65536 } else { config.memory_size };
        let memory = vec![0u8; mem_size];
        let mem_latency = if config.memory_latency == 0 { 100 } else { config.memory_latency };
        let mem_ctrl = MemoryController::new(memory, mem_latency);

        // --- 3. Cache Hierarchy ---
        let cache_hierarchy = build_cache_hierarchy(&config, mem_latency);

        // --- 4. Branch Predictor + BTB ---
        let predictor = create_branch_predictor(
            &config.branch_predictor_type,
            config.branch_predictor_size,
        );
        let btb_size = if config.btb_size == 0 { 64 } else { config.btb_size };
        let btb = BranchTargetBuffer::new(btb_size);

        // --- 5. Hazard Unit ---
        let num_fp_units = if config.fp_unit.is_some() { 1 } else { 0 };
        let hazard_unit = HazardUnit::new(1, num_fp_units, true);

        // --- 6. Pipeline ---
        let pipeline_config = if config.pipeline.stages.is_empty() {
            PipelineConfig::classic_5_stage()
        } else {
            config.pipeline.clone()
        };

        let state = Rc::new(RefCell::new(CoreState {
            reg_file,
            mem_ctrl,
            cache_hierarchy,
            predictor,
            btb,
            hazard_unit,
            decoder,
            cycle: 0,
            forward_count: 0,
            stall_count: 0,
            flush_count: 0,
            pipeline_config: pipeline_config.clone(),
            operand_indexed_forwarding,
        }));

        // Create pipeline callbacks that reference the shared state.
        let fetch_state = Rc::clone(&state);
        let fetch_fn: cpu_pipeline::FetchFn = Box::new(move |pc: i64| -> i64 {
            let mut s = fetch_state.borrow_mut();
            let cycle = s.cycle;
            s.cache_hierarchy.read(pc as u64, true, cycle as u64);
            s.mem_ctrl.read_word(pc)
        });

        let decode_state = Rc::clone(&state);
        let decode_fn: cpu_pipeline::DecodeFn =
            Box::new(move |raw: i64, token: PipelineToken| -> PipelineToken {
                let s = decode_state.borrow();
                s.decoder.decode(raw, token)
            });

        let execute_state = Rc::clone(&state);
        let execute_fn: cpu_pipeline::ExecuteFn =
            Box::new(move |token: PipelineToken| -> PipelineToken {
                let mut s = execute_state.borrow_mut();
                execute_and_update_predictor(&mut s, token, None)
            });

        let memory_state = Rc::clone(&state);
        let memory_fn: cpu_pipeline::MemoryFn =
            Box::new(move |mut token: PipelineToken| -> PipelineToken {
                let mut s = memory_state.borrow_mut();
                let cycle = s.cycle;

                if token.mem_read {
                    // Load: read from data cache hierarchy.
                    s.cache_hierarchy
                        .read(token.alu_result as u64, false, cycle as u64);
                    token.mem_data = s.mem_ctrl.read_word(token.alu_result);
                    token.write_data = token.mem_data;
                } else if token.mem_write {
                    // Store: write to data cache hierarchy.
                    let data = vec![(token.write_data & 0xFF) as u8];
                    s.cache_hierarchy
                        .write(token.alu_result as u64, &data, cycle as u64);
                    s.mem_ctrl.write_word(token.alu_result, token.write_data);
                }
                token
            });

        let writeback_state = Rc::clone(&state);
        let writeback_fn: cpu_pipeline::WritebackFn = Box::new(move |token: &PipelineToken| {
            let mut s = writeback_state.borrow_mut();
            if token.reg_write && token.rd >= 0 {
                s.reg_file.write(token.rd, token.write_data);
            }
        });

        let mut pipeline = Pipeline::new(
            pipeline_config,
            fetch_fn,
            decode_fn,
            execute_fn,
            memory_fn,
            writeback_fn,
        )?;

        if operand_indexed_forwarding {
            let forwarding_state = Rc::clone(&state);
            let forwarding_fn: cpu_pipeline::OperandForwardingFn = Box::new(move |stages| {
                let s = forwarding_state.borrow();
                resolve_operand_forwarding(&s, stages)
            });
            let forwarded_execute_state = Rc::clone(&state);
            let forwarded_execute_fn: cpu_pipeline::ExecuteWithOperandsFn =
                Box::new(move |token, forwarded| {
                    let mut s = forwarded_execute_state.borrow_mut();
                    execute_and_update_predictor(&mut s, token, Some(forwarded))
                });
            pipeline.set_operand_forwarding(forwarding_fn, forwarded_execute_fn);
        }

        // Wire hazard callback.
        if config.hazard_detection {
            let hazard_state = Rc::clone(&state);
            let hazard_fn: cpu_pipeline::HazardFn =
                Box::new(move |stages: &[Option<PipelineToken>]| -> HazardResponse {
                    let mut s = hazard_state.borrow_mut();
                    hazard_callback(&mut s, stages)
                });
            pipeline.set_hazard_fn(hazard_fn);
        }

        // Wire predict callback.
        let predict_state = Rc::clone(&state);
        let predict_fn: cpu_pipeline::PredictFn = Box::new(move |pc: i64| -> i64 {
            let mut s = predict_state.borrow_mut();
            let prediction = s.predictor.predict(pc as u64);
            let instr_size = s.decoder.instruction_size();

            if prediction.taken {
                if let Some(target) = s.btb.lookup(pc as u64) {
                    return target as i64;
                }
            }

            // Default: sequential fetch.
            pc + instr_size
        });
        pipeline.set_predict_fn(predict_fn);

        // --- 7. Clock ---
        let clk = Clock::new(1_000_000_000); // 1 GHz nominal

        Ok(Core {
            config,
            pipeline,
            state,
            _clk: clk,
            halted: false,
            cycle: 0,
            instructions_completed: 0,
        })
    }

    /// Loads machine code into memory starting at the given address.
    ///
    /// The program bytes are written to main memory. The PC is set to
    /// `start_address` before calling Run() or Step().
    pub fn load_program(&mut self, program: &[u8], start_address: usize) {
        self.state
            .borrow_mut()
            .mem_ctrl
            .load_program(program, start_address);
        self.pipeline.set_pc(start_address as i64);
    }

    /// Executes one clock cycle.
    ///
    /// Advances the pipeline by one step, which:
    ///   - Checks for hazards (stalls, flushes, forwards)
    ///   - Moves tokens through pipeline stages
    ///   - Executes stage callbacks (fetch, decode, execute, memory, writeback)
    ///   - Updates statistics
    pub fn step(&mut self) -> PipelineSnapshot {
        if self.halted {
            return self.pipeline.snapshot();
        }

        self.cycle += 1;
        self.state.borrow_mut().cycle = self.cycle;

        let snap = self.pipeline.step();

        // Check if the pipeline halted this cycle.
        if self.pipeline.is_halted() {
            self.halted = true;
        }

        // Track completed instructions.
        self.instructions_completed = self.pipeline.stats().instructions_completed;

        snap
    }

    /// Executes the core until it halts or `max_cycles` is reached.
    ///
    /// Returns aggregate statistics for the entire run.
    pub fn run(&mut self, max_cycles: i64) -> CoreStats {
        while self.cycle < max_cycles && !self.halted {
            self.step();
        }
        self.stats()
    }

    /// Returns aggregate statistics from all sub-components.
    pub fn stats(&self) -> CoreStats {
        let p_stats = self.pipeline.stats();
        let s = self.state.borrow();

        let mut cache_stats_map = HashMap::new();
        if let Some(ref l1i) = s.cache_hierarchy.l1i {
            cache_stats_map.insert("L1I".to_string(), l1i.stats.clone());
        }
        if let Some(ref l1d) = s.cache_hierarchy.l1d {
            cache_stats_map.insert("L1D".to_string(), l1d.stats.clone());
        }
        if let Some(ref l2) = s.cache_hierarchy.l2 {
            cache_stats_map.insert("L2".to_string(), l2.stats.clone());
        }

        CoreStats {
            instructions_completed: p_stats.instructions_completed,
            total_cycles: p_stats.total_cycles,
            pipeline_stats: p_stats,
            predictor_stats: Some(s.predictor.stats().clone()),
            cache_stats: cache_stats_map,
            forward_count: s.forward_count,
            stall_count: s.stall_count,
            flush_count: s.flush_count,
        }
    }

    /// Returns true if a halt instruction has completed.
    pub fn is_halted(&self) -> bool {
        self.halted
    }

    /// Reads a general-purpose register.
    pub fn read_register(&self, index: i64) -> i64 {
        self.state.borrow().reg_file.read(index)
    }

    /// Writes a general-purpose register.
    pub fn write_register(&mut self, index: i64, value: i64) {
        self.state.borrow_mut().reg_file.write(index, value);
    }

    /// Returns the current cycle number.
    pub fn cycle(&self) -> i64 {
        self.cycle
    }

    /// Returns the core configuration.
    pub fn config(&self) -> &CoreConfig {
        &self.config
    }

    /// Returns a reference to the pipeline (for advanced inspection).
    pub fn pipeline(&self) -> &Pipeline {
        &self.pipeline
    }

    /// Returns a mutable reference to the pipeline.
    pub fn pipeline_mut(&mut self) -> &mut Pipeline {
        &mut self.pipeline
    }

}

/// The operand-indexed Core resolver models one producer position for each
/// pipeline category. Keep that opt-in assumption explicit until split-stage
/// age and readiness rules are defined; the general Core constructor remains
/// free to use any valid `PipelineConfig`.
fn supports_operand_forwarding_topology(config: &CoreConfig) -> bool {
    let pipeline = &config.pipeline;
    if pipeline.execution_width != 1 {
        return false;
    }
    if pipeline.stages.is_empty() {
        // An empty stage list is the existing Core shorthand for the classic
        // five-stage configuration.
        return true;
    }

    let categories: Vec<_> = pipeline.stages.iter().map(|stage| stage.category).collect();
    categories
        == [
            StageCategory::Fetch,
            StageCategory::Decode,
            StageCategory::Execute,
            StageCategory::Memory,
            StageCategory::Writeback,
        ]
}

// =========================================================================
// Helper functions
// =========================================================================

/// Runs the ISA execute callback and keeps predictor updates identical for
/// the legacy and operand-indexed paths.
fn execute_and_update_predictor(
    state: &mut CoreState,
    token: PipelineToken,
    forwarded: Option<&[ForwardedOperand]>,
) -> PipelineToken {
    let result = match forwarded {
        Some(operands) => state
            .decoder
            .execute_with_forwarding(token, &state.reg_file, operands),
        None => state.decoder.execute(token, &state.reg_file),
    };

    if result.is_branch {
        state.predictor.update(
            result.pc as u64,
            result.branch_taken,
            if result.branch_taken {
                Some(result.branch_target as u64)
            } else {
                None
            },
        );
        if result.branch_taken {
            state
                .btb
                .update(result.pc as u64, result.branch_target as u64, "conditional");
        }
    }

    result
}

/// Resolves each source independently, preferring the youngest EX producer.
/// A matching EX load deliberately yields no MEM fallback: the hazard unit
/// must stall this consumer until that load reaches MEM.
fn resolve_operand_forwarding(
    state: &CoreState,
    stages: &[Option<PipelineToken>],
) -> Vec<ForwardedOperand> {
    let mut decode_token: Option<&PipelineToken> = None;
    let mut execute_token: Option<&PipelineToken> = None;
    let mut memory_token: Option<&PipelineToken> = None;

    for (index, stage) in state.pipeline_config.stages.iter().enumerate() {
        let Some(token) = stages.get(index).and_then(Option::as_ref) else {
            continue;
        };
        if token.is_bubble {
            continue;
        }
        match stage.category {
            StageCategory::Decode => decode_token = Some(token),
            StageCategory::Execute if execute_token.is_none() => execute_token = Some(token),
            StageCategory::Memory if memory_token.is_none() => memory_token = Some(token),
            _ => {}
        }
    }

    let Some(consumer) = decode_token else {
        return Vec::new();
    };
    let register_config = state.reg_file.config();
    let mut forwarded = Vec::with_capacity(2);
    for (operand, register) in [
        (OperandSlot::Rs1, consumer.rs1),
        (OperandSlot::Rs2, consumer.rs2),
    ] {
        if register < 0 || (register_config.zero_register && register == 0) {
            continue;
        }
        if let Some(value) = resolve_one_operand(register, execute_token, memory_token) {
            forwarded.push(ForwardedOperand {
                operand,
                register,
                value,
                source_stage: if execute_token.is_some_and(|token| {
                    token.reg_write && token.rd == register && !token.mem_read
                }) {
                    OperandForwardingStage::Execute
                } else {
                    OperandForwardingStage::Memory
                },
            });
        }
    }
    forwarded
}

fn resolve_one_operand(
    register: i64,
    execute_token: Option<&PipelineToken>,
    memory_token: Option<&PipelineToken>,
) -> Option<i64> {
    if let Some(token) = execute_token.filter(|token| token.reg_write && token.rd == register) {
        return if token.mem_read {
            None
        } else {
            Some(token.write_data)
        };
    }
    memory_token
        .filter(|token| token.reg_write && token.rd == register)
        .map(|token| token.write_data)
}

/// Builds the L1I, L1D, and optional L2 cache hierarchy.
fn build_cache_hierarchy(config: &CoreConfig, mem_latency: usize) -> CacheHierarchy {
    // Default L1I: 4KB direct-mapped, 64B lines, 1-cycle latency.
    let l1i = config
        .l1i_cache
        .as_ref()
        .map(|cfg| Cache::new(cfg.clone()))
        .or_else(|| Some(Cache::new(CacheConfig::new("L1I", 4096, 64, 1, 1))));

    // Default L1D: 4KB direct-mapped, 64B lines, 1-cycle latency.
    let l1d = config
        .l1d_cache
        .as_ref()
        .map(|cfg| Cache::new(cfg.clone()))
        .or_else(|| Some(Cache::new(CacheConfig::new("L1D", 4096, 64, 1, 1))));

    // Optional L2.
    let l2 = config.l2_cache.as_ref().map(|cfg| Cache::new(cfg.clone()));

    CacheHierarchy::new(l1i, l1d, l2, None, mem_latency as u64)
}

/// Hazard callback -- translates pipeline tokens into hazard-detection slots.
fn hazard_callback(
    state: &mut CoreState,
    stages: &[Option<PipelineToken>],
) -> HazardResponse {
    let num_stages = stages.len();
    let pipeline_cfg = &state.pipeline_config;

    // Find the IF, ID, EX, MEM stages by category.
    let mut if_tok: Option<&PipelineToken> = None;
    let mut id_tok: Option<&PipelineToken> = None;
    let mut ex_tok: Option<&PipelineToken> = None;
    let mut mem_tok: Option<&PipelineToken> = None;

    for (i, stage) in pipeline_cfg.stages.iter().enumerate() {
        if i >= num_stages {
            break;
        }
        if let Some(ref tok) = stages[i] {
            match stage.category {
                StageCategory::Fetch => {
                    if if_tok.is_none() {
                        if_tok = Some(tok);
                    }
                }
                StageCategory::Decode => {
                    // Use the LAST decode stage (closest to EX).
                    id_tok = Some(tok);
                }
                StageCategory::Execute => {
                    if ex_tok.is_none() {
                        ex_tok = Some(tok);
                    }
                }
                StageCategory::Memory => {
                    if mem_tok.is_none() {
                        mem_tok = Some(tok);
                    }
                }
                StageCategory::Writeback => {}
            }
        }
    }

    // Convert PipelineTokens to PipelineSlots for the hazard unit.
    let if_slot = token_to_slot(if_tok);
    let mut id_slot = token_to_slot(id_tok);
    if state.operand_indexed_forwarding {
        // Decode identifies source registers but does not execute an ALU
        // operation. In this opt-in path, do not mistake ID/EX overlap for a
        // single-ALU collision that would discard a ready bypass consumer.
        id_slot.uses_alu = false;
    }
    let ex_slot = token_to_slot(ex_tok);
    let mem_slot = token_to_slot(mem_tok);

    // Run hazard detection.
    let result = state.hazard_unit.check(&if_slot, &id_slot, &ex_slot, &mem_slot);

    // Convert HazardResult to HazardResponse.
    let mut response = HazardResponse::default();

    match result.action {
        HazardDetectionAction::Stall => {
            response.action = HazardAction::Stall;
            response.stall_stages = if state.operand_indexed_forwarding {
                pipeline_cfg
                    .stages
                    .iter()
                    .position(|stage| stage.category == StageCategory::Execute)
                    .unwrap_or(1)
                    .max(1)
            } else {
                result.stall_cycles as usize
            };
            state.stall_count += 1;
        }
        HazardDetectionAction::Flush => {
            response.action = HazardAction::Flush;
            response.flush_count = result.flush_count as usize;
            // Redirect PC to the correct target.
            if let Some(ex) = ex_tok {
                if ex.is_branch {
                    if ex.branch_taken {
                        response.redirect_pc = ex.branch_target;
                    } else {
                        response.redirect_pc = ex.pc + state.decoder.instruction_size();
                    }
                }
            }
            state.flush_count += 1;
        }
        HazardDetectionAction::ForwardFromEX => {
            response.action = HazardAction::ForwardFromEX;
            if let Some(val) = result.forwarded_value {
                response.forward_value = val;
            }
            response.forward_source = result.forwarded_from.clone();
            state.forward_count += 1;
        }
        HazardDetectionAction::ForwardFromMEM => {
            response.action = HazardAction::ForwardFromMEM;
            if let Some(val) = result.forwarded_value {
                response.forward_value = val;
            }
            response.forward_source = result.forwarded_from.clone();
            state.forward_count += 1;
        }
        HazardDetectionAction::None => {
            // No action needed.
        }
    }

    response
}

/// Converts a PipelineToken to a hazard-detection PipelineSlot.
///
/// Bridges the gap between the pipeline package (PipelineToken) and the
/// hazard-detection package (PipelineSlot). The packages are deliberately
/// decoupled, so the Core must translate between the two.
fn token_to_slot(tok: Option<&PipelineToken>) -> PipelineSlot {
    let tok = match tok {
        Some(t) if !t.is_bubble => t,
        _ => return PipelineSlot { valid: false, ..Default::default() },
    };

    let mut slot = PipelineSlot {
        valid: true,
        pc: tok.pc as u32,
        is_branch: tok.is_branch,
        mem_read: tok.mem_read,
        mem_write: tok.mem_write,
        uses_alu: true, // Most instructions use the ALU
        ..Default::default()
    };

    // Source registers.
    if tok.rs1 >= 0 {
        slot.source_regs.push(tok.rs1 as u32);
    }
    if tok.rs2 >= 0 {
        slot.source_regs.push(tok.rs2 as u32);
    }

    // Destination register.
    if tok.rd >= 0 && tok.reg_write {
        slot.dest_reg = Some(tok.rd as u32);
        // Provide the computed value for forwarding.
        if tok.alu_result != 0 || tok.write_data != 0 {
            let val = if tok.write_data != 0 {
                tok.write_data
            } else {
                tok.alu_result
            };
            slot.dest_value = Some(val);
        }
    }

    // Branch prediction tracking.
    if tok.is_branch {
        slot.branch_taken = tok.branch_taken;
        slot.branch_predicted_taken = false; // Default assumption
    }

    slot
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{cortex_a78_like_config, simple_config};
    use crate::decoder::{
        encode_add, encode_addi, encode_halt, encode_load, encode_nop, encode_program,
        encode_store, encode_sub, MockDecoder,
    };
    use std::cell::RefCell as TestRefCell;
    use std::rc::Rc as TestRc;

    fn make_simple_core() -> Core {
        Core::new(simple_config(), Box::new(MockDecoder::new())).expect("failed to create core")
    }

    fn make_default_core() -> Core {
        Core::new(CoreConfig::default(), Box::new(MockDecoder::new()))
            .expect("failed to create core")
    }

    fn forwarding_token(pc: i64, rs1: i64, rs2: i64, rd: i64, value: i64) -> PipelineToken {
        let mut token = PipelineToken::new();
        token.pc = pc;
        token.rs1 = rs1;
        token.rs2 = rs2;
        token.rd = rd;
        token.write_data = value;
        token.reg_write = true;
        token
    }

    #[test]
    fn operand_forwarding_resolves_independent_ex_and_mem_sources() {
        let core = make_simple_core();
        let state = core.state.borrow();
        let consumer = forwarding_token(12, 2, 1, -1, 0);
        let execute_producer = forwarding_token(8, -1, -1, 2, 128);
        let memory_producer = forwarding_token(4, -1, -1, 1, 0);
        let stages = vec![
            None,
            Some(consumer),
            Some(execute_producer),
            Some(memory_producer),
            None,
        ];

        let forwarded = resolve_operand_forwarding(&state, &stages);

        assert_eq!(
            forwarded,
            vec![
                ForwardedOperand {
                    operand: OperandSlot::Rs1,
                    register: 2,
                    value: 128,
                    source_stage: OperandForwardingStage::Execute,
                },
                ForwardedOperand {
                    operand: OperandSlot::Rs2,
                    register: 1,
                    value: 0,
                    source_stage: OperandForwardingStage::Memory,
                },
            ]
        );
    }

    #[test]
    fn operand_forwarding_chooses_youngest_ex_producer_per_register() {
        let core = make_simple_core();
        let state = core.state.borrow();
        let stages = vec![
            None,
            Some(forwarding_token(12, 5, -1, -1, 0)),
            Some(forwarding_token(8, -1, -1, 5, 11)),
            Some(forwarding_token(4, -1, -1, 5, 7)),
            None,
        ];

        let forwarded = resolve_operand_forwarding(&state, &stages);

        assert_eq!(forwarded.len(), 1);
        assert_eq!(forwarded[0].operand, OperandSlot::Rs1);
        assert_eq!(forwarded[0].register, 5);
        assert_eq!(forwarded[0].value, 11);
        assert_eq!(forwarded[0].source_stage, OperandForwardingStage::Execute);
    }

    #[test]
    fn operand_forwarding_does_not_use_older_mem_value_around_ex_load() {
        let core = make_simple_core();
        let state = core.state.borrow();
        let mut load = forwarding_token(8, -1, -1, 5, 0);
        load.mem_read = true;
        let stages = vec![
            None,
            Some(forwarding_token(12, 5, -1, -1, 0)),
            Some(load),
            Some(forwarding_token(4, -1, -1, 5, 7)),
            None,
        ];

        assert!(resolve_operand_forwarding(&state, &stages).is_empty());
    }

    #[test]
    fn operand_forwarding_respects_zero_register_configuration() {
        let core = make_simple_core();
        let state = core.state.borrow();
        let stages = vec![
            None,
            Some(forwarding_token(12, 0, -1, -1, 0)),
            Some(forwarding_token(8, -1, -1, 0, 99)),
            None,
            None,
        ];
        assert!(resolve_operand_forwarding(&state, &stages).is_empty());

        drop(state);
        let mut config = simple_config();
        config.register_file.as_mut().unwrap().zero_register = false;
        let core = Core::new(config, Box::new(MockDecoder::new())).unwrap();
        let state = core.state.borrow();
        let forwarded = resolve_operand_forwarding(&state, &stages);
        assert_eq!(forwarded.len(), 1);
        assert_eq!(forwarded[0].register, 0);
        assert_eq!(forwarded[0].value, 99);
    }

    struct ForwardingProbeDecoder {
        observed: TestRc<TestRefCell<Vec<Vec<ForwardedOperand>>>>,
    }

    impl ISADecoder for ForwardingProbeDecoder {
        fn decode(&self, raw: i64, mut token: PipelineToken) -> PipelineToken {
            match raw {
                1 => {
                    token.opcode = "PRODUCE".to_string();
                    token.rd = 1;
                    token.reg_write = true;
                }
                2 => {
                    token.opcode = "CONSUME".to_string();
                    token.rs1 = 1;
                    token.rd = 2;
                    token.reg_write = true;
                }
                3 => {
                    token.opcode = "LOAD_ZERO".to_string();
                    token.rd = 3;
                    token.mem_read = true;
                    token.reg_write = true;
                }
                4 => {
                    token.opcode = "CONSUME_LOAD".to_string();
                    token.rs1 = 3;
                    token.rd = 4;
                    token.reg_write = true;
                }
                255 => {
                    token.opcode = "HALT".to_string();
                    token.is_halt = true;
                }
                _ => token.opcode = "NOP".to_string(),
            }
            token
        }

        fn execute(&self, mut token: PipelineToken, _reg_file: &RegisterFile) -> PipelineToken {
            if token.opcode == "PRODUCE" {
                token.write_data = 5;
            } else if token.opcode == "LOAD_ZERO" {
                token.alu_result = 128;
            } else if token.opcode == "CONSUME" {
                // Legacy execution intentionally ignores bypasses.
                token.write_data = -1;
            }
            token
        }

        fn execute_with_forwarding(
            &self,
            mut token: PipelineToken,
            reg_file: &RegisterFile,
            forwarded: &[ForwardedOperand],
        ) -> PipelineToken {
            self.observed.borrow_mut().push(forwarded.to_vec());
            if token.opcode == "CONSUME" || token.opcode == "CONSUME_LOAD" {
                let value = forwarded
                    .iter()
                    .find(|value| value.operand == OperandSlot::Rs1)
                    .map(|value| value.value)
                    .unwrap_or_else(|| reg_file.read(token.rs1));
                token.write_data = value + 1;
            } else if token.opcode == "PRODUCE" {
                token.write_data = 5;
            } else if token.opcode == "LOAD_ZERO" {
                token.alu_result = 128;
            }
            token
        }

        fn instruction_size(&self) -> i64 {
            4
        }
    }

    #[test]
    fn core_opt_in_delivers_named_forward_value_to_decoder() {
        let observed = TestRc::new(TestRefCell::new(Vec::new()));
        let decoder = ForwardingProbeDecoder {
            observed: observed.clone(),
        };
        let mut core = Core::new_with_operand_forwarding(simple_config(), Box::new(decoder))
            .expect("opt-in forwarding core should construct");
        let program = [1u32, 2, 255]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>();
        core.load_program(&program, 0);
        core.run(100);

        assert!(core.is_halted());
        assert_eq!(core.read_register(1), 5);
        assert_eq!(core.read_register(2), 6, "forwarded callbacks: {:?}", observed.borrow());
        assert!(observed.borrow().iter().any(|operands| {
            operands
                .iter()
                .any(|operand| operand.operand == OperandSlot::Rs1 && operand.value == 5)
        }));
    }

    #[test]
    fn core_opt_in_holds_load_use_then_forwards_the_mem_result() {
        let observed = TestRc::new(TestRefCell::new(Vec::new()));
        let decoder = ForwardingProbeDecoder {
            observed: observed.clone(),
        };
        let mut core = Core::new_with_operand_forwarding(simple_config(), Box::new(decoder))
            .expect("opt-in forwarding core should construct");
        let program = [3u32, 4, 255]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>();
        core.load_program(&program, 0);
        core.write_register(3, 99);
        core.run(100);

        assert!(core.is_halted());
        assert_eq!(core.read_register(3), 0);
        assert_eq!(core.read_register(4), 1);
        assert_eq!(core.stats().stall_count, 1);
        assert!(observed.borrow().iter().any(|operands| {
            operands.iter().any(|operand| {
                operand.operand == OperandSlot::Rs1
                    && operand.register == 3
                    && operand.value == 0
                    && operand.source_stage == OperandForwardingStage::Memory
            })
        }));
    }

    #[test]
    fn core_opt_in_requires_hazards_and_forwarding_enabled() {
        let mut config = simple_config();
        config.forwarding = false;
        assert!(Core::new_with_operand_forwarding(config, Box::new(MockDecoder::new())).is_err());

        let mut config = simple_config();
        config.hazard_detection = false;
        assert!(Core::new_with_operand_forwarding(config, Box::new(MockDecoder::new())).is_err());
    }

    #[test]
    fn core_opt_in_rejects_split_or_reordered_stage_topologies() {
        let mut deep = simple_config();
        deep.pipeline = PipelineConfig::deep_13_stage();
        let error = Core::new_with_operand_forwarding(deep, Box::new(MockDecoder::new()))
            .err()
            .expect("split stage topology has no defined forwarding age rule");
        assert!(error.contains("scalar Fetch/Decode/Execute/Memory/Writeback"));

        let mut reordered = simple_config();
        reordered.pipeline.stages.swap(1, 2);
        assert!(Core::new_with_operand_forwarding(reordered, Box::new(MockDecoder::new())).is_err());
    }

    #[test]
    fn core_opt_in_rejects_non_scalar_execution_width() {
        let mut config = simple_config();
        config.pipeline.execution_width = 2;
        assert!(Core::new_with_operand_forwarding(config, Box::new(MockDecoder::new())).is_err());
    }

    #[test]
    fn default_core_still_accepts_deep_pipeline_configuration() {
        let mut config = simple_config();
        config.pipeline = PipelineConfig::deep_13_stage();
        assert!(Core::new(config, Box::new(MockDecoder::new())).is_ok());
    }

    #[test]
    fn test_core_construction() {
        let c = make_simple_core();
        assert!(!c.is_halted());
        assert_eq!(c.cycle(), 0);
    }

    #[test]
    fn test_core_default_construction() {
        let c = make_default_core();
        assert!(!c.is_halted());
    }

    #[test]
    fn test_load_and_run_simple_program() {
        let mut c = make_simple_core();
        // ADDI R1, R0, 42 ; HALT
        let program = encode_program(&[encode_addi(1, 0, 42), encode_halt()]);
        c.load_program(&program, 0);
        let stats = c.run(100);
        assert!(c.is_halted());
        assert!(stats.total_cycles > 0);
        assert!(stats.instructions_completed > 0);
    }

    #[test]
    fn test_register_read_write() {
        let mut c = make_simple_core();
        c.write_register(1, 42);
        assert_eq!(c.read_register(1), 42);
    }

    #[test]
    fn test_step() {
        let mut c = make_simple_core();
        let program = encode_program(&[encode_halt()]);
        c.load_program(&program, 0);
        let _snap = c.step();
        assert_eq!(c.cycle(), 1);
    }

    #[test]
    fn test_config_accessor() {
        let c = make_simple_core();
        assert_eq!(c.config().name, "Simple");
    }

    #[test]
    fn test_stats_after_halt() {
        let mut c = make_simple_core();
        let program = encode_program(&[encode_addi(1, 0, 10), encode_halt()]);
        c.load_program(&program, 0);
        c.run(100);
        let stats = c.stats();
        assert!(stats.ipc() > 0.0);
        assert!(stats.cpi() > 0.0);
    }

    #[test]
    fn test_halted_step_is_no_op() {
        let mut c = make_simple_core();
        let program = encode_program(&[encode_halt()]);
        c.load_program(&program, 0);
        c.run(100);
        assert!(c.is_halted());
        let cycle_before = c.cycle();
        c.step();
        assert_eq!(c.cycle(), cycle_before, "step after halt should not advance cycle");
    }

    #[test]
    fn test_token_to_slot_bubble() {
        let bubble = PipelineToken::new_bubble();
        let slot = token_to_slot(Some(&bubble));
        assert!(!slot.valid);
    }

    #[test]
    fn test_token_to_slot_none() {
        let slot = token_to_slot(None);
        assert!(!slot.valid);
    }

    #[test]
    fn test_token_to_slot_add() {
        let mut tok = PipelineToken::new();
        tok.opcode = "ADD".to_string();
        tok.rd = 1;
        tok.rs1 = 2;
        tok.rs2 = 3;
        tok.reg_write = true;
        tok.alu_result = 42;
        let slot = token_to_slot(Some(&tok));
        assert!(slot.valid);
        assert_eq!(slot.source_regs, vec![2, 3]);
        assert_eq!(slot.dest_reg, Some(1));
        assert_eq!(slot.dest_value, Some(42));
    }

    #[test]
    fn test_token_to_slot_branch() {
        let mut tok = PipelineToken::new();
        tok.opcode = "BRANCH".to_string();
        tok.is_branch = true;
        tok.branch_taken = true;
        let slot = token_to_slot(Some(&tok));
        assert!(slot.is_branch);
        assert!(slot.branch_taken);
    }

    // =====================================================================
    // Integration tests -- multi-instruction programs
    // =====================================================================

    #[test]
    fn test_multi_instruction_program() {
        // R1 = 10, R2 = 20, R3 = R1 + R2 (= 30), HALT
        // NOPs inserted to avoid data hazards (values must writeback before read).
        let mut c = make_simple_core();
        let program = encode_program(&[
            encode_addi(1, 0, 10),
            encode_nop(),
            encode_nop(),
            encode_addi(2, 0, 20),
            encode_nop(),
            encode_nop(),
            encode_add(3, 1, 2),
            encode_halt(),
        ]);
        c.load_program(&program, 0);
        c.run(100);
        assert!(c.is_halted());
        assert_eq!(c.read_register(1), 10);
        assert_eq!(c.read_register(2), 20);
        assert_eq!(c.read_register(3), 30);
    }

    #[test]
    fn test_sub_instruction() {
        // R1 = 100, R2 = 30, R3 = R1 - R2 (= 70), HALT
        // NOPs inserted to avoid data hazards.
        let mut c = make_simple_core();
        let program = encode_program(&[
            encode_addi(1, 0, 100),
            encode_nop(),
            encode_nop(),
            encode_addi(2, 0, 30),
            encode_nop(),
            encode_nop(),
            encode_sub(3, 1, 2),
            encode_halt(),
        ]);
        c.load_program(&program, 0);
        c.run(100);
        assert!(c.is_halted());
        assert_eq!(c.read_register(3), 70);
    }

    #[test]
    fn test_store_and_load() {
        // R1 = 42, store R1 to memory[100], load from memory[100] into R2
        let mut c = make_simple_core();
        let program = encode_program(&[
            encode_addi(1, 0, 42),
            encode_nop(),
            encode_nop(),
            encode_store(0, 1, 100),  // store R1 at address 100
            encode_nop(),
            encode_nop(),
            encode_load(2, 0, 100),   // load from address 100 into R2
            encode_halt(),
        ]);
        c.load_program(&program, 0);
        c.run(200);
        assert!(c.is_halted());
        assert_eq!(c.read_register(1), 42);
        assert_eq!(c.read_register(2), 42);
    }

    #[test]
    fn test_nop_program() {
        let mut c = make_simple_core();
        let program = encode_program(&[
            encode_nop(),
            encode_nop(),
            encode_nop(),
            encode_halt(),
        ]);
        c.load_program(&program, 0);
        c.run(100);
        assert!(c.is_halted());
    }

    #[test]
    fn test_max_cycles_reached() {
        // Program with no halt -- run should stop at max_cycles.
        let mut c = make_simple_core();
        let program = encode_program(&[encode_nop(), encode_nop(), encode_nop()]);
        c.load_program(&program, 0);
        c.run(5);
        assert!(!c.is_halted());
        assert_eq!(c.cycle(), 5);
    }

    #[test]
    fn test_cortex_a78_config_core() {
        // Verify that the deep pipeline config can create a core.
        let c = Core::new(cortex_a78_like_config(), Box::new(MockDecoder::new()));
        assert!(c.is_ok());
        let mut c = c.unwrap();
        let program = encode_program(&[encode_addi(1, 0, 99), encode_halt()]);
        c.load_program(&program, 0);
        c.run(200);
        assert!(c.is_halted());
        assert_eq!(c.read_register(1), 99);
    }

    #[test]
    fn test_pipeline_accessor() {
        let c = make_simple_core();
        let _config = c.pipeline().config();
        // Should not panic -- just verify pipeline is accessible.
    }

    #[test]
    fn test_stats_has_cache_info() {
        let mut c = make_simple_core();
        let program = encode_program(&[encode_addi(1, 0, 42), encode_halt()]);
        c.load_program(&program, 0);
        c.run(100);
        let stats = c.stats();
        // Simple config has L1I and L1D.
        assert!(stats.cache_stats.contains_key("L1I"));
        assert!(stats.cache_stats.contains_key("L1D"));
    }

    #[test]
    fn test_stats_has_predictor_info() {
        let mut c = make_simple_core();
        let program = encode_program(&[encode_addi(1, 0, 42), encode_halt()]);
        c.load_program(&program, 0);
        c.run(100);
        let stats = c.stats();
        assert!(stats.predictor_stats.is_some());
    }

    #[test]
    fn test_zero_register_convention() {
        // Writing to R0 should have no effect when zero_register is enabled.
        let mut c = make_simple_core();
        c.write_register(0, 42);
        assert_eq!(c.read_register(0), 0);
    }

    #[test]
    fn test_token_to_slot_no_source_regs() {
        let mut tok = PipelineToken::new();
        tok.opcode = "HALT".to_string();
        // rs1 and rs2 are both -1 (unused).
        let slot = token_to_slot(Some(&tok));
        assert!(slot.valid);
        assert!(slot.source_regs.is_empty());
    }

    #[test]
    fn test_token_to_slot_write_data_preferred() {
        // When both write_data and alu_result are nonzero, write_data wins.
        let mut tok = PipelineToken::new();
        tok.rd = 1;
        tok.reg_write = true;
        tok.alu_result = 10;
        tok.write_data = 20;
        let slot = token_to_slot(Some(&tok));
        assert_eq!(slot.dest_value, Some(20));
    }
}
