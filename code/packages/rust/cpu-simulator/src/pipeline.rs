//! # Pipeline -- the fetch-decode-execute cycle that drives every CPU.
//!
//! ## What is a pipeline?
//!
//! Every CPU operates by repeating three steps over and over:
//!
//! ```text
//!     +---------+     +---------+     +---------+
//!     |  FETCH  | --> | DECODE  | --> | EXECUTE | --> (repeat)
//!     +---------+     +---------+     +---------+
//! ```
//!
//! 1. **FETCH:** Read the next instruction from memory at the address stored
//!    in the Program Counter (PC). The instruction is just a number
//!    -- a pattern of bits that encodes what operation to perform.
//!
//! 2. **DECODE:** Figure out what those bits mean. Which operation is it? (ADD?
//!    LOAD? BRANCH?) Which registers are involved? Is there an
//!    immediate value encoded in the instruction?
//!
//! 3. **EXECUTE:** Perform the operation. This might mean sending values through
//!    the ALU (for arithmetic), reading/writing memory (for loads
//!    and stores), or changing the PC (for branches/jumps).
//!
//! After execution, the PC is updated (usually PC += 4 for 32-bit instruction
//! sets) and the cycle repeats.
//!
//! ## Why is it called a "pipeline"?
//!
//! In simple CPUs (like ours), these three stages happen one after another
//! for each instruction. But in modern CPUs, they overlap -- while one
//! instruction is being executed, the next one is being decoded, and the one
//! after that is being fetched. This is called "pipelining" and it's how
//! CPUs achieve high throughput.
//!
//! Think of it like a laundry pipeline:
//!
//! - **Simple:** wash shirt 1, dry shirt 1, fold shirt 1, THEN wash shirt 2...
//! - **Pipelined:** while shirt 1 is drying, start washing shirt 2.
//!   While shirt 2 is drying and shirt 1 is being folded,
//!   start washing shirt 3.
//!
//! Our simulator starts with a simple non-pipelined design (one instruction
//! fully completes before the next begins) but exposes the pipeline stages
//! visibly so you can see what happens at each step.
//!
//! ## Pipeline hazards (future)
//!
//! Pipelining introduces problems called "hazards":
//!
//! - **Data hazard:** instruction 2 needs the result of instruction 1, but
//!   instruction 1 hasn't finished yet
//! - **Control hazard:** a branch instruction changes the PC, so the
//!   instructions we already fetched are wrong (pipeline "flush")
//! - **Structural hazard:** two instructions need the same hardware unit
//!   at the same time
//!
//! These are fascinating problems that we'll explore as we add pipelining.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Pipeline stage enum
// ---------------------------------------------------------------------------

/// The three stages of the fetch-decode-execute cycle.
///
/// Each instruction passes through these stages in order:
///
/// ```text
///     FETCH -> DECODE -> EXECUTE
/// ```
///
/// In our simple (non-pipelined) CPU, only one stage is active at a time.
/// In a pipelined CPU, up to three instructions can be in different stages
/// simultaneously.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    Fetch,
    Decode,
    Execute,
}

// ---------------------------------------------------------------------------
// Stage result types
// ---------------------------------------------------------------------------

/// What the FETCH stage produces.
///
/// The fetch stage reads raw bytes from memory at the current PC address.
/// It doesn't know what the bytes mean -- that's the decode stage's job.
///
/// ```text
///     +--------------------------------------+
///     | FETCH                                |
///     | PC: 0x00000004                       |
///     | Read 4 bytes -> 0x002081B3           |
///     +--------------------------------------+
/// ```
#[derive(Debug, Clone)]
pub struct FetchResult {
    /// Program Counter value when the fetch occurred.
    pub pc: usize,
    /// The raw 32-bit instruction word.
    pub raw_instruction: u32,
}

/// What the DECODE stage produces.
///
/// The decode stage takes the raw instruction bits and extracts the
/// meaningful fields: what operation, which registers, what immediate value.
///
/// This is ISA-specific -- RISC-V, ARM, WASM, and 4004 all decode
/// differently. The CPU simulator provides this as a generic container;
/// the ISA simulator fills in the details.
///
/// Example (RISC-V `add x3, x1, x2`):
///
/// ```text
///     mnemonic = "add"
///     fields   = { "rd": 3, "rs1": 1, "rs2": 2, "funct3": 0, "funct7": 0 }
/// ```
#[derive(Debug, Clone)]
pub struct DecodeResult {
    /// Human-readable instruction name (e.g., "add", "lw", "beq").
    pub mnemonic: String,
    /// Decoded fields (ISA-specific). Keys are field names like "rd", "rs1".
    pub fields: HashMap<String, i32>,
    /// The raw instruction (kept for display purposes).
    pub raw_instruction: u32,
}

/// What the EXECUTE stage produces.
///
/// The execute stage performs the actual operation and records what changed.
///
/// Example (`add x3, x1, x2` where x1=1, x2=2):
///
/// ```text
///     description       = "x3 = x1 + x2 = 1 + 2 = 3"
///     registers_changed = { "x3": 3 }
///     memory_changed    = {}
///     next_pc           = 12  (PC + 4, normal sequential execution)
///     halted            = false
/// ```
#[derive(Debug, Clone)]
pub struct ExecuteResult {
    /// Human-readable description of what happened.
    pub description: String,
    /// Which registers changed and to what values.
    pub registers_changed: HashMap<String, u32>,
    /// Which memory addresses changed (address -> byte value).
    pub memory_changed: HashMap<usize, u8>,
    /// The new program counter value.
    pub next_pc: usize,
    /// Did this instruction halt the CPU?
    pub halted: bool,
}

// ---------------------------------------------------------------------------
// Pipeline trace
// ---------------------------------------------------------------------------

/// A complete record of one instruction's journey through the pipeline.
///
/// This is the main data structure for visualization. It captures what
/// happened at each stage, allowing you to see the full pipeline:
///
/// ```text
///     +----------------------------------------------------------+
///     | Instruction #0                                           |
///     +--------------+------------------+-----------------------+
///     | FETCH        | DECODE           | EXECUTE               |
///     | PC: 0x0000   | addi x1, x0, 1  | x1 = 0 + 1 = 1       |
///     | -> 0x00100093| rd=1, rs1=0,     | Write x1 = 1          |
///     |              | imm=1            | PC -> 4               |
///     +--------------+------------------+-----------------------+
/// ```
#[derive(Debug, Clone)]
pub struct PipelineTrace {
    /// Which instruction number this is (0, 1, 2, ...).
    pub cycle: usize,
    /// Result of the fetch stage.
    pub fetch: FetchResult,
    /// Result of the decode stage.
    pub decode: DecodeResult,
    /// Result of the execute stage.
    pub execute: ExecuteResult,
    /// Snapshot of all register values after execution.
    pub register_snapshot: HashMap<String, u32>,
}

/// Format a pipeline trace as a visual pipeline diagram.
///
/// Returns a multi-line string showing all three stages side by side,
/// making it easy to follow what happened at each step.
///
/// # Example output
///
/// ```text
///     --- Cycle 0 ---
///       FETCH              | DECODE             | EXECUTE
///       PC: 0x0000         | addi               | x1 = 1
///       -> 0x00100093      | rd=1 rs1=0 imm=1   | PC -> 4
/// ```
pub fn format_pipeline(trace: &PipelineTrace) -> String {
    // Build the three columns of text -- one for each pipeline stage.

    let fetch_lines = vec![
        "FETCH".to_string(),
        format!("PC: 0x{:04X}", trace.fetch.pc),
        format!("-> 0x{:08X}", trace.fetch.raw_instruction),
    ];

    // Sort fields for deterministic output (HashMap iteration order varies).
    let mut field_pairs: Vec<_> = trace.decode.fields.iter().collect();
    field_pairs.sort_by_key(|(k, _)| (*k).clone());
    let fields_str: String = field_pairs
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join(" ");

    let decode_lines = vec![
        "DECODE".to_string(),
        trace.decode.mnemonic.clone(),
        fields_str,
    ];

    let execute_lines = vec![
        "EXECUTE".to_string(),
        trace.execute.description.clone(),
        format!("PC -> {}", trace.execute.next_pc),
    ];

    // Pad all columns to the same number of lines.
    let max_lines = fetch_lines
        .len()
        .max(decode_lines.len())
        .max(execute_lines.len());

    let pad = |lines: &[String], target: usize| -> Vec<String> {
        let mut result = lines.to_vec();
        while result.len() < target {
            result.push(String::new());
        }
        result
    };

    let fetch_lines = pad(&fetch_lines, max_lines);
    let decode_lines = pad(&decode_lines, max_lines);
    let execute_lines = pad(&execute_lines, max_lines);

    // Format as fixed-width columns separated by pipes.
    let col_width = 20;
    let mut result = vec![format!("--- Cycle {} ---", trace.cycle)];
    for i in 0..max_lines {
        let f = format!("{:<width$}", fetch_lines[i], width = col_width);
        let d = format!("{:<width$}", decode_lines[i], width = col_width);
        let e = format!("{:<width$}", execute_lines[i], width = col_width);
        result.push(format!("  {} | {} | {}", f, d, e));
    }

    result.join("\n")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_pipeline_contains_cycle_header() {
        let trace = PipelineTrace {
            cycle: 0,
            fetch: FetchResult {
                pc: 4,
                raw_instruction: 0x93,
            },
            decode: DecodeResult {
                mnemonic: "addi".to_string(),
                fields: HashMap::from([("rd".to_string(), 1)]),
                raw_instruction: 0x93,
            },
            execute: ExecuteResult {
                description: "x1 = 1".to_string(),
                registers_changed: HashMap::new(),
                memory_changed: HashMap::new(),
                next_pc: 8,
                halted: false,
            },
            register_snapshot: HashMap::new(),
        };

        let formatted = format_pipeline(&trace);
        assert!(
            formatted.contains("--- Cycle 0 ---"),
            "Should contain cycle header"
        );
        assert!(
            formatted.contains("addi"),
            "Should contain decode mnemonic"
        );
        assert!(
            formatted.contains("FETCH"),
            "Should contain FETCH label"
        );
        assert!(
            formatted.contains("EXECUTE"),
            "Should contain EXECUTE label"
        );
    }

    #[test]
    fn format_pipeline_shows_pc_in_hex() {
        let trace = PipelineTrace {
            cycle: 2,
            fetch: FetchResult {
                pc: 0x0010,
                raw_instruction: 0xABCD1234,
            },
            decode: DecodeResult {
                mnemonic: "sub".to_string(),
                fields: HashMap::new(),
                raw_instruction: 0xABCD1234,
            },
            execute: ExecuteResult {
                description: "nop".to_string(),
                registers_changed: HashMap::new(),
                memory_changed: HashMap::new(),
                next_pc: 0x14,
                halted: false,
            },
            register_snapshot: HashMap::new(),
        };

        let formatted = format_pipeline(&trace);
        assert!(formatted.contains("PC: 0x0010"), "PC should be in hex");
        assert!(
            formatted.contains("0xABCD1234"),
            "Raw instruction should be in hex"
        );
    }

    #[test]
    fn pipeline_stage_enum_values() {
        // Ensure the enum variants exist and are distinct.
        assert_ne!(PipelineStage::Fetch, PipelineStage::Decode);
        assert_ne!(PipelineStage::Decode, PipelineStage::Execute);
        assert_ne!(PipelineStage::Fetch, PipelineStage::Execute);
    }
}
