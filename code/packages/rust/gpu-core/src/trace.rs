//! Execution traces -- making every instruction's journey visible.
//!
//! # Why Traces?
//!
//! A key principle of this project is educational transparency: every operation
//! should be observable. When a GPU core executes an instruction, the trace
//! records exactly what happened:
//!
//! ```text
//! Cycle 3 | PC=2 | FFMA R3, R0, R1, R2
//!     -> R3 = R0 * R1 + R2 = 2.0 * 3.0 + 1.0 = 7.0
//!     -> Registers changed: {R3: 7.0}
//!     -> Next PC: 3
//! ```
//!
//! This lets a student (or debugger) follow the execution step by step,
//! understanding not just *what* the GPU did but *why* -- which registers were
//! read, what computation was performed, and what state changed.
//!
//! # Trace vs Log
//!
//! A trace is more structured than a log message. Each field is typed and
//! accessible programmatically, which enables:
//!
//! - Automated testing (`assert_eq!(trace.registers_changed["R3"], 7.0)`)
//! - Visualization tools (render execution as a timeline)
//! - Performance analysis (count cycles, track register usage)

use std::collections::HashMap;
use std::fmt;

use crate::opcodes::Instruction;

/// A record of one instruction's execution on a GPU core.
///
/// Every call to `GPUCore::step()` returns one of these, providing full
/// visibility into what the instruction did.
///
/// # Fields
///
/// - `cycle`: The clock cycle number (1-indexed).
/// - `pc`: The program counter BEFORE this instruction executed.
/// - `instruction`: The instruction that was executed.
/// - `description`: Human-readable description of what happened.
///   Example: "R3 = R0 * R1 + R2 = 2.0 * 3.0 + 1.0 = 7.0"
/// - `registers_changed`: Which registers changed and their new values.
///   Example: {"R3": 7.0}
/// - `memory_changed`: Which memory addresses changed and their new values.
///   Example: {0: 3.14, 4: 2.71}
/// - `next_pc`: The program counter AFTER this instruction.
/// - `halted`: True if this instruction stopped execution.
#[derive(Debug, Clone)]
pub struct GPUCoreTrace {
    pub cycle: u64,
    pub pc: usize,
    pub instruction: Instruction,
    pub description: String,
    pub next_pc: usize,
    pub halted: bool,
    pub registers_changed: HashMap<String, f64>,
    pub memory_changed: HashMap<usize, f64>,
}

impl GPUCoreTrace {
    /// Pretty-print this trace record for educational display.
    ///
    /// Returns a multi-line string like:
    ///
    /// ```text
    /// [Cycle 3] PC=2: FFMA R3, R0, R1, R2
    ///   -> R3 = R0 * R1 + R2 = 2.0 * 3.0 + 1.0 = 7.0
    ///   -> Registers: {R3=7.0}
    ///   -> Next PC: 3
    /// ```
    pub fn format(&self) -> String {
        let mut lines = vec![format!(
            "[Cycle {}] PC={}: {}",
            self.cycle, self.pc, self.instruction
        )];
        lines.push(format!("  -> {}", self.description));

        if !self.registers_changed.is_empty() {
            let regs: Vec<String> = self
                .registers_changed
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            lines.push(format!("  -> Registers: {{{}}}", regs.join(", ")));
        }

        if !self.memory_changed.is_empty() {
            let mems: Vec<String> = self
                .memory_changed
                .iter()
                .map(|(k, v)| format!("0x{:04X}={}", k, v))
                .collect();
            lines.push(format!("  -> Memory: {{{}}}", mems.join(", ")));
        }

        if self.halted {
            lines.push("  -> HALTED".to_string());
        } else {
            lines.push(format!("  -> Next PC: {}", self.next_pc));
        }

        lines.join("\n")
    }
}

impl fmt::Display for GPUCoreTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcodes;

    #[test]
    fn test_trace_format_basic() {
        let trace = GPUCoreTrace {
            cycle: 1,
            pc: 0,
            instruction: opcodes::limm(0, 3.0),
            description: "R0 = 3".to_string(),
            next_pc: 1,
            halted: false,
            registers_changed: {
                let mut m = HashMap::new();
                m.insert("R0".to_string(), 3.0);
                m
            },
            memory_changed: HashMap::new(),
        };
        let formatted = trace.format();
        assert!(formatted.contains("[Cycle 1]"));
        assert!(formatted.contains("PC=0"));
        assert!(formatted.contains("R0"));
        assert!(formatted.contains("Next PC: 1"));
    }

    #[test]
    fn test_trace_format_halted() {
        let trace = GPUCoreTrace {
            cycle: 5,
            pc: 4,
            instruction: opcodes::halt(),
            description: "Halted".to_string(),
            next_pc: 4,
            halted: true,
            registers_changed: HashMap::new(),
            memory_changed: HashMap::new(),
        };
        let formatted = trace.format();
        assert!(formatted.contains("HALTED"));
        assert!(!formatted.contains("Next PC"));
    }

    #[test]
    fn test_trace_format_memory_changed() {
        let trace = GPUCoreTrace {
            cycle: 3,
            pc: 2,
            instruction: opcodes::store(0, 1, 0.0),
            description: "Mem[0] = R1 = 42.0".to_string(),
            next_pc: 3,
            halted: false,
            registers_changed: HashMap::new(),
            memory_changed: {
                let mut m = HashMap::new();
                m.insert(0, 42.0);
                m
            },
        };
        let formatted = trace.format();
        assert!(formatted.contains("Memory:"));
        assert!(formatted.contains("0x0000=42"));
    }

    #[test]
    fn test_trace_display() {
        let trace = GPUCoreTrace {
            cycle: 1,
            pc: 0,
            instruction: opcodes::nop(),
            description: "No operation".to_string(),
            next_pc: 1,
            halted: false,
            registers_changed: HashMap::new(),
            memory_changed: HashMap::new(),
        };
        let display = format!("{}", trace);
        assert!(display.contains("[Cycle 1]"));
    }
}
