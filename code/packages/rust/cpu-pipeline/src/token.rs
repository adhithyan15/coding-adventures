//! Pipeline tokens, stages, and configuration.
//!
//! # The Pipeline Token: a Tray on the Assembly Line
//!
//! A [`PipelineToken`] represents one instruction moving through the pipeline.
//! Think of it as a tray on a factory assembly line. The tray starts empty at
//! the IF stage, gets filled with decoded information at ID, gets computed
//! results at EX, gets memory data at MEM, and delivers results at WB.
//!
//! The token is ISA-independent. The ISA decoder fills in the fields via
//! callbacks. The pipeline itself never looks at instruction semantics --
//! it only moves tokens between stages and handles stalls/flushes.
//!
//! # Bubbles
//!
//! A "bubble" is a special token that represents NO instruction. Bubbles
//! are inserted when the pipeline stalls (to fill the gap left by frozen
//! stages) or when the pipeline flushes (to replace discarded speculative
//! instructions). A bubble flows through the pipeline like a normal token
//! but does nothing at each stage.
//!
//! In hardware, a bubble is a NOP (no-operation) instruction. In our
//! simulator, it is a token with `is_bubble = true`.

use std::collections::HashMap;
use std::fmt;

// =========================================================================
// StageCategory -- classifies pipeline stages by their function
// =========================================================================

/// Classifies pipeline stages by their function.
///
/// Every stage in a pipeline does one of these five jobs, regardless of
/// how many stages the pipeline has. A 5-stage pipeline has one stage per
/// category. A 13-stage pipeline might have 2 fetch stages, 2 decode
/// stages, 3 execute stages, etc.
///
/// This classification is used for:
///   - Determining which callback to invoke for each stage
///   - Knowing where to insert stall bubbles
///   - Knowing which stages to flush on a misprediction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageCategory {
    /// Stages that read instructions from the instruction cache.
    /// In a 5-stage pipeline, this is the IF stage.
    /// In deeper pipelines, this might be IF1 (TLB lookup) and IF2 (cache read).
    Fetch,

    /// Stages that decode the instruction and read registers.
    /// Extracts opcode, register numbers, immediate values from raw bits.
    Decode,

    /// Stages that perform computation (ALU, branch resolution).
    /// Some pipelines split this into EX1 (ALU), EX2 (shift/multiply), EX3 (result).
    Execute,

    /// Stages that access data memory (loads and stores).
    /// Some pipelines have separate stages for address calculation and data access.
    Memory,

    /// Stages that write results back to the register file.
    /// This is always the final stage -- the instruction is "retired" here.
    Writeback,
}

impl fmt::Display for StageCategory {
    /// Returns a human-readable name for the stage category.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StageCategory::Fetch => write!(f, "fetch"),
            StageCategory::Decode => write!(f, "decode"),
            StageCategory::Execute => write!(f, "execute"),
            StageCategory::Memory => write!(f, "memory"),
            StageCategory::Writeback => write!(f, "writeback"),
        }
    }
}

// =========================================================================
// PipelineStage -- definition of a single stage in the pipeline
// =========================================================================

/// Defines a single stage in the pipeline.
///
/// A stage has a short name (used in diagrams), a description (for humans),
/// and a category (for the pipeline to know what callback to invoke).
///
/// # Examples
///
/// ```
/// use cpu_pipeline::token::{PipelineStage, StageCategory};
///
/// let stage = PipelineStage {
///     name: "IF".to_string(),
///     description: "Instruction Fetch".to_string(),
///     category: StageCategory::Fetch,
/// };
/// assert_eq!(stage.to_string(), "IF");
/// ```
#[derive(Debug, Clone)]
pub struct PipelineStage {
    /// Short name like "IF", "ID", "EX1".
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// What kind of work this stage does.
    pub category: StageCategory,
}

impl fmt::Display for PipelineStage {
    /// Returns the stage name for display in diagrams.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// =========================================================================
// PipelineToken -- a unit of work flowing through the pipeline
// =========================================================================

/// Represents one instruction moving through the pipeline.
///
/// # Token Lifecycle
///
/// ```text
/// IF stage:  FetchFunc fills in pc and raw_instruction
/// ID stage:  DecodeFunc fills in opcode, registers, control signals
/// EX stage:  ExecuteFunc fills in alu_result, branch_taken, branch_target
/// MEM stage: MemoryFunc fills in mem_data (for loads)
/// WB stage:  WritebackFunc uses write_data to update register file
/// ```
///
/// # Bubbles
///
/// A "bubble" is a special token that represents NO instruction. Bubbles
/// are inserted when the pipeline stalls or flushes. A bubble flows through
/// the pipeline like a normal token but does nothing at each stage.
#[derive(Debug, Clone)]
pub struct PipelineToken {
    // --- Instruction identity ---

    /// The program counter -- the memory address of this instruction.
    /// Set by the IF stage when the instruction is fetched.
    pub pc: i64,

    /// The raw instruction bits as fetched from memory.
    /// Set by the IF stage via the fetch callback.
    pub raw_instruction: i64,

    /// The decoded instruction name (e.g., "ADD", "LDR", "BEQ").
    /// Set by the ID stage for debugging and tracing.
    pub opcode: String,

    // --- Decoded operands (set by ID stage callback) ---

    /// First source register number (-1 means unused).
    ///
    /// Example: in "ADD R1, R2, R3", rs1 = 2 (register R2).
    pub rs1: i64,

    /// Second source register number (-1 means unused).
    ///
    /// Example: in "ADD R1, R2, R3", rs2 = 3 (register R3).
    pub rs2: i64,

    /// Destination register number (-1 means unused).
    ///
    /// Example: in "ADD R1, R2, R3", rd = 1 (register R1).
    pub rd: i64,

    /// Sign-extended immediate value from the instruction.
    /// Used by I-type instructions like "ADDI R1, R2, #5" (immediate = 5).
    pub immediate: i64,

    // --- Control signals (set by ID stage callback) ---

    /// True if this instruction writes a register.
    ///
    /// Truth table:
    /// ```text
    ///   ADD  R1, R2, R3  -> reg_write = true  (writes R1)
    ///   STR  R1, [R2]    -> reg_write = false (only writes memory)
    ///   BEQ  R1, R2, L   -> reg_write = false (only changes PC)
    ///   LDR  R1, [R2]    -> reg_write = true  (writes R1)
    /// ```
    pub reg_write: bool,

    /// True if this instruction reads from data memory.
    /// Only load instructions (LDR, LW, etc.) set this.
    pub mem_read: bool,

    /// True if this instruction writes to data memory.
    /// Only store instructions (STR, SW, etc.) set this.
    pub mem_write: bool,

    /// True if this instruction is a branch (conditional or unconditional).
    pub is_branch: bool,

    /// True if this is a halt/stop instruction.
    /// When a halt token reaches the WB stage, the pipeline stops.
    pub is_halt: bool,

    // --- Computed values (filled during execution) ---

    /// Output of the ALU in the EX stage.
    ///
    /// For arithmetic: the computed value (e.g., R2 + R3 for ADD).
    /// For loads/stores: the computed memory address.
    /// For branches: the branch target address.
    pub alu_result: i64,

    /// Data read from memory in the MEM stage.
    /// Only meaningful for load instructions (mem_read = true).
    pub mem_data: i64,

    /// Final value to write to the destination register.
    /// Selected in the WB stage: either alu_result (ALU ops) or mem_data (loads).
    pub write_data: i64,

    /// True if the branch was actually taken (resolved in EX).
    pub branch_taken: bool,

    /// Actual branch target address (resolved in EX).
    pub branch_target: i64,

    // --- Pipeline metadata ---

    /// True if this token represents a NOP/bubble.
    ///
    /// Bubbles are inserted in two situations:
    ///   1. Stall: a bubble is inserted into the stage AFTER the stall point
    ///   2. Flush: bubbles replace all speculative instructions
    pub is_bubble: bool,

    /// Maps stage name to the cycle number when the token entered that stage.
    /// Used for tracing and debugging.
    ///
    /// Example: {"IF": 1, "ID": 2, "EX": 4, "MEM": 5, "WB": 6}
    /// (note the gap between ID and EX -- that was a stall cycle)
    pub stage_entered: HashMap<String, i64>,

    /// Records which stage provided a forwarded value, if forwarding was used.
    /// Empty string means no forwarding.
    pub forwarded_from: String,
}

impl PipelineToken {
    /// Creates a new empty token with default register values.
    ///
    /// The token starts with all register fields set to -1 (unused) and
    /// all control signals set to false. The fetch callback will fill in
    /// the PC and raw instruction; the decode callback fills in everything else.
    pub fn new() -> Self {
        PipelineToken {
            pc: 0,
            raw_instruction: 0,
            opcode: String::new(),
            rs1: -1,
            rs2: -1,
            rd: -1,
            immediate: 0,
            reg_write: false,
            mem_read: false,
            mem_write: false,
            is_branch: false,
            is_halt: false,
            alu_result: 0,
            mem_data: 0,
            write_data: 0,
            branch_taken: false,
            branch_target: 0,
            is_bubble: false,
            stage_entered: HashMap::new(),
            forwarded_from: String::new(),
        }
    }

    /// Creates a new bubble token.
    ///
    /// A bubble is a "do nothing" instruction that occupies a pipeline stage
    /// without performing any useful work. It is the pipeline equivalent of
    /// a "no-op" on an assembly line -- the stage runs through its motions
    /// but produces no output.
    pub fn new_bubble() -> Self {
        PipelineToken {
            is_bubble: true,
            rs1: -1,
            rs2: -1,
            rd: -1,
            ..PipelineToken::new()
        }
    }
}

impl Default for PipelineToken {
    fn default() -> Self {
        PipelineToken::new()
    }
}

impl fmt::Display for PipelineToken {
    /// Returns a human-readable representation of the token.
    ///
    /// For debugging and pipeline diagrams:
    ///   - Bubbles display as "---" (like empty slots on the assembly line)
    ///   - Normal tokens display their opcode and PC
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_bubble {
            write!(f, "---")
        } else if !self.opcode.is_empty() {
            write!(f, "{}@{}", self.opcode, self.pc)
        } else {
            write!(f, "instr@{}", self.pc)
        }
    }
}

// =========================================================================
// PipelineConfig -- configuration for the pipeline
// =========================================================================

/// Holds the configuration for a pipeline.
///
/// The key insight: a pipeline's behavior is determined entirely by its
/// stage configuration and execution width. Everything else (instruction
/// semantics, hazard handling) is injected via callbacks.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// The pipeline stages in order, from first to last.
    /// Tokens flow from stages[0] to stages[len-1].
    pub stages: Vec<PipelineStage>,

    /// Number of instructions the pipeline can process per cycle.
    /// Width 1 = scalar pipeline. Width > 1 = superscalar.
    /// (Superscalar is a future extension; for now we only support 1.)
    pub execution_width: i64,
}

impl PipelineConfig {
    /// Returns the number of stages in the pipeline.
    pub fn num_stages(&self) -> usize {
        self.stages.len()
    }

    /// Validates that the configuration is well-formed.
    ///
    /// Rules:
    ///   - Must have at least 2 stages (a 1-stage "pipeline" is not a pipeline)
    ///   - Execution width must be at least 1
    ///   - All stage names must be unique
    ///   - There must be at least one fetch stage and one writeback stage
    pub fn validate(&self) -> Result<(), String> {
        if self.stages.len() < 2 {
            return Err(format!(
                "pipeline must have at least 2 stages, got {}",
                self.stages.len()
            ));
        }
        if self.execution_width < 1 {
            return Err(format!(
                "execution width must be at least 1, got {}",
                self.execution_width
            ));
        }

        // Check for unique stage names.
        let mut seen = std::collections::HashSet::new();
        for s in &self.stages {
            if !seen.insert(&s.name) {
                return Err(format!("duplicate stage name: {:?}", s.name));
            }
        }

        // Check for required categories.
        let has_fetch = self.stages.iter().any(|s| s.category == StageCategory::Fetch);
        let has_writeback = self.stages.iter().any(|s| s.category == StageCategory::Writeback);

        if !has_fetch {
            return Err("pipeline must have at least one fetch stage".to_string());
        }
        if !has_writeback {
            return Err("pipeline must have at least one writeback stage".to_string());
        }

        Ok(())
    }

    /// Returns the standard 5-stage RISC pipeline configuration.
    ///
    /// This is the pipeline described in every computer architecture textbook:
    ///
    /// ```text
    /// IF -> ID -> EX -> MEM -> WB
    /// ```
    ///
    /// It matches the MIPS R2000 (1985) and is the foundation for understanding
    /// all modern CPU pipelines.
    pub fn classic_5_stage() -> Self {
        PipelineConfig {
            stages: vec![
                PipelineStage {
                    name: "IF".to_string(),
                    description: "Instruction Fetch".to_string(),
                    category: StageCategory::Fetch,
                },
                PipelineStage {
                    name: "ID".to_string(),
                    description: "Instruction Decode".to_string(),
                    category: StageCategory::Decode,
                },
                PipelineStage {
                    name: "EX".to_string(),
                    description: "Execute".to_string(),
                    category: StageCategory::Execute,
                },
                PipelineStage {
                    name: "MEM".to_string(),
                    description: "Memory Access".to_string(),
                    category: StageCategory::Memory,
                },
                PipelineStage {
                    name: "WB".to_string(),
                    description: "Write Back".to_string(),
                    category: StageCategory::Writeback,
                },
            ],
            execution_width: 1,
        }
    }

    /// Returns a 13-stage pipeline inspired by ARM Cortex-A78.
    ///
    /// Modern high-performance CPUs split the classic 5 stages into many
    /// sub-stages to enable higher clock frequencies. Each sub-stage does
    /// less work, so it completes faster, allowing a faster clock.
    ///
    /// The tradeoff: a branch misprediction now costs 10+ cycles instead of 2.
    pub fn deep_13_stage() -> Self {
        PipelineConfig {
            stages: vec![
                PipelineStage { name: "IF1".to_string(), description: "Fetch 1 - TLB lookup".to_string(), category: StageCategory::Fetch },
                PipelineStage { name: "IF2".to_string(), description: "Fetch 2 - cache read".to_string(), category: StageCategory::Fetch },
                PipelineStage { name: "IF3".to_string(), description: "Fetch 3 - align/buffer".to_string(), category: StageCategory::Fetch },
                PipelineStage { name: "ID1".to_string(), description: "Decode 1 - pre-decode".to_string(), category: StageCategory::Decode },
                PipelineStage { name: "ID2".to_string(), description: "Decode 2 - full decode".to_string(), category: StageCategory::Decode },
                PipelineStage { name: "ID3".to_string(), description: "Decode 3 - register read".to_string(), category: StageCategory::Decode },
                PipelineStage { name: "EX1".to_string(), description: "Execute 1 - ALU".to_string(), category: StageCategory::Execute },
                PipelineStage { name: "EX2".to_string(), description: "Execute 2 - shift/multiply".to_string(), category: StageCategory::Execute },
                PipelineStage { name: "EX3".to_string(), description: "Execute 3 - result select".to_string(), category: StageCategory::Execute },
                PipelineStage { name: "MEM1".to_string(), description: "Memory 1 - address calc".to_string(), category: StageCategory::Memory },
                PipelineStage { name: "MEM2".to_string(), description: "Memory 2 - cache access".to_string(), category: StageCategory::Memory },
                PipelineStage { name: "MEM3".to_string(), description: "Memory 3 - data align".to_string(), category: StageCategory::Memory },
                PipelineStage { name: "WB".to_string(), description: "Write Back".to_string(), category: StageCategory::Writeback },
            ],
            execution_width: 1,
        }
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_token() {
        let tok = PipelineToken::new();
        assert_eq!(tok.rs1, -1);
        assert_eq!(tok.rs2, -1);
        assert_eq!(tok.rd, -1);
        assert!(!tok.is_bubble);
        assert!(tok.stage_entered.is_empty());
    }

    #[test]
    fn test_new_bubble() {
        let b = PipelineToken::new_bubble();
        assert!(b.is_bubble);
        assert_eq!(b.to_string(), "---");
    }

    #[test]
    fn test_token_string() {
        let mut tok = PipelineToken::new();
        tok.opcode = "ADD".to_string();
        tok.pc = 100;
        assert_eq!(tok.to_string(), "ADD@100");

        let mut tok2 = PipelineToken::new();
        tok2.pc = 200;
        assert_eq!(tok2.to_string(), "instr@200");
    }

    #[test]
    fn test_token_clone() {
        let mut tok = PipelineToken::new();
        tok.pc = 100;
        tok.opcode = "ADD".to_string();
        tok.stage_entered.insert("IF".to_string(), 1);
        tok.stage_entered.insert("ID".to_string(), 2);

        let mut cloned = tok.clone();
        assert_eq!(cloned.pc, 100);
        assert_eq!(cloned.opcode, "ADD");

        // Mutating the clone should not affect the original.
        cloned.stage_entered.insert("EX".to_string(), 3);
        assert!(!tok.stage_entered.contains_key("EX"));
    }

    #[test]
    fn test_classic_5_stage() {
        let config = PipelineConfig::classic_5_stage();
        assert_eq!(config.num_stages(), 5);
        assert!(config.validate().is_ok());
        assert_eq!(config.stages[0].name, "IF");
        assert_eq!(config.stages[4].name, "WB");
    }

    #[test]
    fn test_deep_13_stage() {
        let config = PipelineConfig::deep_13_stage();
        assert_eq!(config.num_stages(), 13);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation() {
        // Too few stages.
        let cfg = PipelineConfig {
            stages: vec![PipelineStage {
                name: "IF".to_string(),
                description: "".to_string(),
                category: StageCategory::Fetch,
            }],
            execution_width: 1,
        };
        assert!(cfg.validate().is_err());

        // Zero execution width.
        let cfg2 = PipelineConfig {
            stages: vec![
                PipelineStage { name: "IF".to_string(), description: "".to_string(), category: StageCategory::Fetch },
                PipelineStage { name: "WB".to_string(), description: "".to_string(), category: StageCategory::Writeback },
            ],
            execution_width: 0,
        };
        assert!(cfg2.validate().is_err());

        // Duplicate stage names.
        let cfg3 = PipelineConfig {
            stages: vec![
                PipelineStage { name: "IF".to_string(), description: "".to_string(), category: StageCategory::Fetch },
                PipelineStage { name: "IF".to_string(), description: "".to_string(), category: StageCategory::Writeback },
            ],
            execution_width: 1,
        };
        assert!(cfg3.validate().is_err());

        // No fetch stage.
        let cfg4 = PipelineConfig {
            stages: vec![
                PipelineStage { name: "EX".to_string(), description: "".to_string(), category: StageCategory::Execute },
                PipelineStage { name: "WB".to_string(), description: "".to_string(), category: StageCategory::Writeback },
            ],
            execution_width: 1,
        };
        assert!(cfg4.validate().is_err());

        // No writeback stage.
        let cfg5 = PipelineConfig {
            stages: vec![
                PipelineStage { name: "IF".to_string(), description: "".to_string(), category: StageCategory::Fetch },
                PipelineStage { name: "EX".to_string(), description: "".to_string(), category: StageCategory::Execute },
            ],
            execution_width: 1,
        };
        assert!(cfg5.validate().is_err());

        // Valid 2-stage pipeline.
        let cfg6 = PipelineConfig {
            stages: vec![
                PipelineStage { name: "IF".to_string(), description: "".to_string(), category: StageCategory::Fetch },
                PipelineStage { name: "WB".to_string(), description: "".to_string(), category: StageCategory::Writeback },
            ],
            execution_width: 1,
        };
        assert!(cfg6.validate().is_ok());
    }

    #[test]
    fn test_stage_category_string() {
        assert_eq!(StageCategory::Fetch.to_string(), "fetch");
        assert_eq!(StageCategory::Decode.to_string(), "decode");
        assert_eq!(StageCategory::Execute.to_string(), "execute");
        assert_eq!(StageCategory::Memory.to_string(), "memory");
        assert_eq!(StageCategory::Writeback.to_string(), "writeback");
    }

    #[test]
    fn test_pipeline_stage_string() {
        let stage = PipelineStage {
            name: "IF".to_string(),
            description: "Instruction Fetch".to_string(),
            category: StageCategory::Fetch,
        };
        assert_eq!(stage.to_string(), "IF");
    }

    #[test]
    fn test_default_token() {
        let tok = PipelineToken::default();
        assert_eq!(tok.rs1, -1);
        assert_eq!(tok.rs2, -1);
        assert_eq!(tok.rd, -1);
    }
}
