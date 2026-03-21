//! cpu-pipeline -- Configurable N-stage CPU instruction pipeline.
//!
//! # The Pipeline: a CPU's Assembly Line
//!
//! A CPU pipeline is the central execution engine of a processor core. Instead
//! of completing one instruction fully before starting the next (like a
//! single-cycle CPU), a pipelined CPU overlaps instruction execution -- while
//! one instruction is being executed, the next is being decoded, and the one
//! after that is being fetched.
//!
//! This is the same principle as a factory assembly line:
//!
//! ```text
//! Single-cycle (no pipeline):
//! Instr 1: [IF][ID][EX][MEM][WB]
//! Instr 2:                       [IF][ID][EX][MEM][WB]
//! Instr 3:                                              [IF]...
//! Throughput: 1 instruction every 5 cycles
//!
//! Pipelined:
//! Instr 1: [IF][ID][EX][MEM][WB]
//! Instr 2:     [IF][ID][EX][MEM][WB]
//! Instr 3:         [IF][ID][EX][MEM][WB]
//! Instr 4:             [IF][ID][EX][MEM][WB]
//! Throughput: 1 instruction every 1 cycle (after filling)
//! ```
//!
//! # What This Crate Does
//!
//! This crate manages the FLOW of instructions through pipeline stages. It
//! does NOT interpret instructions -- that is the ISA decoder's job. The
//! pipeline moves "tokens" (representing instructions) through stages, handling:
//!
//!   - Normal advancement: tokens move one stage per clock cycle
//!   - Stalls: freeze earlier stages and insert a "bubble" (NOP)
//!   - Flushes: replace speculative instructions with bubbles
//!   - Statistics: track IPC, stall cycles, flush cycles
//!
//! # The Classic 5-Stage Pipeline
//!
//! ```text
//! Stage 1: IF  (Instruction Fetch)  -- read instruction from memory at PC
//! Stage 2: ID  (Instruction Decode) -- decode opcode, read registers
//! Stage 3: EX  (Execute)            -- ALU operation, branch resolution
//! Stage 4: MEM (Memory Access)      -- load/store data from/to memory
//! Stage 5: WB  (Write Back)         -- write result to register file
//! ```
//!
//! # Modules
//!
//! - [`token`]: Pipeline tokens, stages, and configuration
//! - [`pipeline`]: The pipeline engine with step/run
//! - [`snapshot`]: Pipeline snapshots and execution statistics

pub mod token;
pub mod pipeline;
pub mod snapshot;

// Re-export key types for convenience.
pub use token::{PipelineToken, PipelineStage, PipelineConfig, StageCategory};
pub use pipeline::{Pipeline, HazardAction, HazardResponse, FetchFn, DecodeFn, ExecuteFn, MemoryFn, WritebackFn, HazardFn, PredictFn};
pub use snapshot::{PipelineSnapshot, PipelineStats};
