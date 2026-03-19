//! Shared data types for pipeline hazard detection.
//!
//! === Why These Types Exist ===
//!
//! A CPU pipeline is like an assembly line: each stage works on a different
//! instruction simultaneously. But sometimes instructions interfere with each
//! other -- one instruction needs a result that another hasn't produced yet,
//! or two instructions fight over the same hardware resource.
//!
//! The hazard detection unit needs to know what each pipeline stage is doing
//! WITHOUT knowing the specifics of the instruction set.

/// The action the hazard unit tells the pipeline to take.
///
/// Think of these as traffic signals for the pipeline:
/// - `None` — Green light, pipeline flows normally
/// - `ForwardFromEX` — Grab value from the EX stage
/// - `ForwardFromMEM` — Grab value from the MEM stage
/// - `Stall` — Red light, pipeline must freeze (load-use hazard)
/// - `Flush` — Emergency stop, branch misprediction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardAction {
    None,
    ForwardFromMEM,
    ForwardFromEX,
    Stall,
    Flush,
}

impl HazardAction {
    /// Numeric priority (higher = more severe).
    pub fn priority(self) -> u8 {
        match self {
            HazardAction::None => 0,
            HazardAction::ForwardFromMEM => 1,
            HazardAction::ForwardFromEX => 2,
            HazardAction::Stall => 3,
            HazardAction::Flush => 4,
        }
    }
}

/// Information about an instruction occupying a pipeline stage.
///
/// This is ISA-independent. Whatever decoder is plugged in extracts this
/// info from raw instruction bits. The hazard unit only cares about register
/// numbers and resource usage, not opcodes.
///
/// # Example: Encoding "ADD R1, R2, R3"
///
/// ```
/// use hazard_detection::types::PipelineSlot;
/// let slot = PipelineSlot {
///     valid: true,
///     pc: 0x1000,
///     source_regs: vec![2, 3],
///     dest_reg: Some(1),
///     uses_alu: true,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct PipelineSlot {
    pub valid: bool,
    pub pc: u32,
    pub source_regs: Vec<u32>,
    pub dest_reg: Option<u32>,
    pub dest_value: Option<i64>,
    pub is_branch: bool,
    pub branch_taken: bool,
    pub branch_predicted_taken: bool,
    pub mem_read: bool,
    pub mem_write: bool,
    pub uses_alu: bool,
    pub uses_fp: bool,
}

/// Complete result from hazard detection.
///
/// Includes the action to take, forwarded value (if any), stall/flush
/// counts, and a human-readable reason for debugging.
#[derive(Debug, Clone)]
pub struct HazardResult {
    pub action: HazardAction,
    pub forwarded_value: Option<i64>,
    pub forwarded_from: String,
    pub stall_cycles: u32,
    pub flush_count: u32,
    pub reason: String,
}

impl Default for HazardResult {
    fn default() -> Self {
        Self {
            action: HazardAction::None,
            forwarded_value: None,
            forwarded_from: String::new(),
            stall_cycles: 0,
            flush_count: 0,
            reason: String::new(),
        }
    }
}

impl HazardResult {
    /// Create a new HazardResult with the given action and reason.
    pub fn new(action: HazardAction, reason: &str) -> Self {
        Self {
            action,
            reason: reason.to_string(),
            ..Default::default()
        }
    }
}
