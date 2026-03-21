//! # Brainfuck — a complete Brainfuck implementation on the GenericVM.
//!
//! ## What is Brainfuck?
//!
//! Brainfuck is one of the most famous **esoteric programming languages** — a
//! language designed to be minimal rather than practical. Created by Urban
//! Mueller in 1993, it has only **8 instructions**, yet it is Turing-complete
//! (meaning it can compute anything that any other programming language can).
//!
//! ## Why implement it?
//!
//! Brainfuck is the perfect first target for a virtual machine because:
//! 1. It has only 8 instructions — easy to implement completely
//! 2. It exercises all major VM features: sequential execution, loops, I/O
//! 3. Non-trivial programs exist (Hello World, addition, etc.)
//! 4. It proves the GenericVM framework works for a real language
//!
//! ## The Language
//!
//! Brainfuck operates on a **tape** of 30,000 byte cells, all initialized to
//! zero. A **data pointer** (dp) starts at cell 0. The 8 instructions are:
//!
//! | Char | Instruction  | Description                                    |
//! |------|-------------|------------------------------------------------|
//! | `>`  | RIGHT       | Move the data pointer one cell to the right    |
//! | `<`  | LEFT        | Move the data pointer one cell to the left     |
//! | `+`  | INC         | Increment the byte at the data pointer         |
//! | `-`  | DEC         | Decrement the byte at the data pointer         |
//! | `.`  | OUTPUT      | Output the byte at the data pointer as ASCII   |
//! | `,`  | INPUT       | Read one byte of input into the current cell   |
//! | `[`  | LOOP_START  | If current cell is 0, jump past matching `]`   |
//! | `]`  | LOOP_END    | If current cell is non-zero, jump back to `[`  |
//!
//! Any other character is treated as a comment and ignored.
//!
//! ## Architecture
//!
//! ```text
//! Source: "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>."
//!
//!   ┌──────────┐     ┌──────────────┐     ┌──────────────┐
//!   │ Translate │────►│  CodeObject   │────►│ BrainfuckVM  │
//!   │ (parse)  │     │ (bytecode)   │     │  (execute)   │
//!   └──────────┘     └──────────────┘     └──────────────┘
//!                                                │
//!                                         ┌──────┴───────┐
//!                                         │  Tape [0..N] │
//!                                         │  dp: index   │
//!                                         │  output: str │
//!                                         └──────────────┘
//! ```

use std::collections::HashMap;
use virtual_machine::*;

// ─────────────────────────────────────────────────────────────────────────────
// Section 1: Opcodes
// ─────────────────────────────────────────────────────────────────────────────

/// Move the data pointer one cell to the right.
pub const OP_RIGHT: OpCode = 0x01;
/// Move the data pointer one cell to the left.
pub const OP_LEFT: OpCode = 0x02;
/// Increment the byte at the data pointer (wrapping at 255 -> 0).
pub const OP_INC: OpCode = 0x03;
/// Decrement the byte at the data pointer (wrapping at 0 -> 255).
pub const OP_DEC: OpCode = 0x04;
/// Output the byte at the data pointer as an ASCII character.
pub const OP_OUTPUT: OpCode = 0x05;
/// Read one byte of input into the cell at the data pointer.
pub const OP_INPUT: OpCode = 0x06;
/// If the current cell is zero, jump to the instruction after the matching `]`.
pub const OP_LOOP_START: OpCode = 0x07;
/// If the current cell is non-zero, jump back to the matching `[`.
pub const OP_LOOP_END: OpCode = 0x08;
/// Stop execution.
pub const OP_HALT: OpCode = 0xFF;

/// The standard Brainfuck tape size: 30,000 cells.
///
/// This is the traditional size used by most Brainfuck implementations.
/// Each cell holds a single byte (0-255).
pub const TAPE_SIZE: usize = 30_000;

// ─────────────────────────────────────────────────────────────────────────────
// Section 2: Translator (Source -> Bytecode)
// ─────────────────────────────────────────────────────────────────────────────

/// Translate Brainfuck source code into a CodeObject.
///
/// This function performs two tasks:
/// 1. **Filtering**: Ignores all characters that aren't Brainfuck instructions.
///    This means comments and whitespace are automatically handled.
/// 2. **Bracket matching**: Pairs up `[` and `]` instructions so that each
///    LOOP_START knows where to jump on zero, and each LOOP_END knows where
///    to jump back on non-zero.
///
/// ## Bracket Matching Algorithm
///
/// We use a stack to match brackets, similar to how you'd validate parentheses:
///
/// ```text
/// Source: [ + [ - ] ]
/// Index:  0 1 2 3 4 5
///
/// Pass 1: Emit instructions, recording bracket positions
/// Pass 2: Use a stack to match pairs:
///   - See '[' at 0 → push 0
///   - See '[' at 2 → push 2
///   - See ']' at 4 → pop 2 → pair (2, 4)
///   - See ']' at 5 → pop 0 → pair (0, 5)
/// ```
///
/// ## Errors
///
/// Returns an error if brackets are unmatched:
/// - `[` without a matching `]`
/// - `]` without a matching `[`
pub fn translate(source: &str) -> Result<CodeObject, String> {
    // First pass: convert characters to instructions, collecting bracket positions.
    let mut instructions: Vec<Instruction> = Vec::new();
    let mut bracket_stack: Vec<usize> = Vec::new();
    // Map from instruction index -> its matching instruction index.
    let mut bracket_map: HashMap<usize, usize> = HashMap::new();

    for ch in source.chars() {
        let opcode = match ch {
            '>' => Some(OP_RIGHT),
            '<' => Some(OP_LEFT),
            '+' => Some(OP_INC),
            '-' => Some(OP_DEC),
            '.' => Some(OP_OUTPUT),
            ',' => Some(OP_INPUT),
            '[' => {
                let idx = instructions.len();
                bracket_stack.push(idx);
                Some(OP_LOOP_START)
            }
            ']' => {
                let start_idx = bracket_stack.pop().ok_or_else(|| {
                    "Unmatched ']' — found a closing bracket without a matching '['".to_string()
                })?;
                let end_idx = instructions.len();
                bracket_map.insert(start_idx, end_idx + 1); // LOOP_START jumps past LOOP_END
                bracket_map.insert(end_idx, start_idx);     // LOOP_END jumps back to LOOP_START
                Some(OP_LOOP_END)
            }
            _ => None, // All other characters are comments
        };

        if let Some(op) = opcode {
            instructions.push(Instruction {
                opcode: op,
                operand: None, // We'll fill in loop operands next
            });
        }
    }

    // Check for unmatched opening brackets
    if !bracket_stack.is_empty() {
        return Err(format!(
            "Unmatched '[' — found {} opening bracket(s) without matching ']'",
            bracket_stack.len()
        ));
    }

    // Second pass: fill in the jump targets for loop instructions.
    for (idx, target) in &bracket_map {
        instructions[*idx].operand = Some(Operand::Index(*target));
    }

    // Append a HALT instruction to stop execution cleanly.
    instructions.push(Instruction {
        opcode: OP_HALT,
        operand: None,
    });

    Ok(CodeObject {
        instructions,
        constants: vec![],
        names: vec![],
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Section 3: BrainfuckVM
// ─────────────────────────────────────────────────────────────────────────────

/// The BrainfuckVM: a virtual machine specialized for executing Brainfuck bytecode.
///
/// ## Why not use GenericVM's handler registration?
///
/// The GenericVM's handler signature is `fn(&mut GenericVM, &Instruction, &CodeObject)`.
/// Brainfuck handlers need access to the **tape**, **data pointer**, and **input buffer**,
/// which live on BrainfuckVM, not GenericVM. In Rust, we can't have a handler that
/// borrows both `&mut GenericVM` and `&mut BrainfuckVM` simultaneously (the borrow
/// checker would reject this because GenericVM is a field of BrainfuckVM).
///
/// The solution: BrainfuckVM implements its own execution loop with a `match` statement
/// on opcodes, giving it direct access to all its fields. This is the same approach
/// used by the Go implementation and is actually simpler and faster than dynamic
/// dispatch through a handler table.
///
/// ## State
///
/// ```text
/// ┌────────────────────────────────────────────────┐
/// │  BrainfuckVM                                    │
/// │                                                │
/// │  Tape: [0, 0, 72, 101, 0, 0, ...]             │
/// │                  ^                              │
/// │                  dp (data pointer)              │
/// │                                                │
/// │  Input: "ABC"   input_pos: 1                   │
/// │                                                │
/// │  VM state: pc=15, halted=false                 │
/// │  Output: ["H", "e", ...]                       │
/// └────────────────────────────────────────────────┘
/// ```
pub struct BrainfuckVM {
    /// The underlying GenericVM, used for its stack and general state.
    pub vm: GenericVM,
    /// The tape: 30,000 byte cells, all initialized to zero.
    pub tape: Vec<u8>,
    /// The data pointer: index into the tape.
    pub dp: usize,
    /// Pre-loaded input bytes (from the input string).
    pub input_buffer: Vec<u8>,
    /// Current position in the input buffer.
    pub input_pos: usize,
}

impl BrainfuckVM {
    /// Create a new BrainfuckVM with the given input data.
    ///
    /// The input data is a string whose bytes will be fed to `,` (INPUT)
    /// instructions one at a time.
    pub fn new(input_data: &str) -> Self {
        BrainfuckVM {
            vm: GenericVM::new(),
            tape: vec![0u8; TAPE_SIZE],
            dp: 0,
            input_buffer: input_data.bytes().collect(),
            input_pos: 0,
        }
    }

    /// Execute a CodeObject (produced by `translate`) to completion.
    ///
    /// Returns a vector of execution traces, one per instruction executed.
    /// Each trace captures the VM state before and after the instruction,
    /// which is invaluable for debugging and visualization.
    ///
    /// ## The Execution Loop
    ///
    /// ```text
    /// while not halted and pc < instructions.len():
    ///     instruction = instructions[pc]
    ///     match instruction.opcode:
    ///         OP_RIGHT => dp += 1
    ///         OP_LEFT  => dp -= 1
    ///         OP_INC   => tape[dp] += 1  (wrapping)
    ///         OP_DEC   => tape[dp] -= 1  (wrapping)
    ///         OP_OUTPUT => output tape[dp] as char
    ///         OP_INPUT  => tape[dp] = next input byte
    ///         OP_LOOP_START => if tape[dp] == 0, jump to target
    ///         OP_LOOP_END   => if tape[dp] != 0, jump to target
    ///         OP_HALT  => stop
    ///     advance pc (if no jump occurred)
    /// ```
    pub fn execute(&mut self, code: &CodeObject) -> VMResult<Vec<VMTrace>> {
        let mut traces = Vec::new();

        while !self.vm.halted && self.vm.pc < code.instructions.len() {
            let trace = self.step(code)?;
            traces.push(trace);
        }

        Ok(traces)
    }

    /// Execute a single instruction and return its trace.
    fn step(&mut self, code: &CodeObject) -> VMResult<VMTrace> {
        let pc_before = self.vm.pc;
        let stack_before = self.vm.stack.clone();

        let instr = code.instructions.get(self.vm.pc).ok_or_else(|| {
            VMError::InvalidOpcode(format!("PC {} out of bounds", self.vm.pc))
        })?;
        let instr_clone = instr.clone();
        let mut output_str: Option<String> = None;
        let mut jumped = false;

        let description = match instr.opcode {
            // ── OP_RIGHT: Move data pointer right ────────────────────────
            //
            // In Brainfuck, `>` moves the data pointer to the next cell.
            // We wrap around if we reach the end of the tape, though in
            // practice well-formed programs stay within bounds.
            OP_RIGHT => {
                self.dp = (self.dp + 1) % TAPE_SIZE;
                format!("RIGHT: dp is now {}", self.dp)
            }

            // ── OP_LEFT: Move data pointer left ─────────────────────────
            //
            // `<` moves the data pointer to the previous cell.
            // Wraps around from 0 to TAPE_SIZE-1.
            OP_LEFT => {
                if self.dp == 0 {
                    self.dp = TAPE_SIZE - 1;
                } else {
                    self.dp -= 1;
                }
                format!("LEFT: dp is now {}", self.dp)
            }

            // ── OP_INC: Increment current cell ──────────────────────────
            //
            // `+` adds 1 to the current cell. Uses wrapping arithmetic:
            // 255 + 1 = 0 (just like real 8-bit hardware).
            OP_INC => {
                self.tape[self.dp] = self.tape[self.dp].wrapping_add(1);
                format!("INC: tape[{}] is now {}", self.dp, self.tape[self.dp])
            }

            // ── OP_DEC: Decrement current cell ──────────────────────────
            //
            // `-` subtracts 1 from the current cell. Wrapping: 0 - 1 = 255.
            OP_DEC => {
                self.tape[self.dp] = self.tape[self.dp].wrapping_sub(1);
                format!("DEC: tape[{}] is now {}", self.dp, self.tape[self.dp])
            }

            // ── OP_OUTPUT: Output current cell as ASCII ─────────────────
            //
            // `.` outputs the current cell's value as an ASCII character.
            // For example, if tape[dp] == 72, this outputs 'H'.
            OP_OUTPUT => {
                let ch = self.tape[self.dp] as char;
                let s = ch.to_string();
                self.vm.output.push(s.clone());
                output_str = Some(s.clone());
                format!("OUTPUT: '{}' ({})", ch, self.tape[self.dp])
            }

            // ── OP_INPUT: Read one byte of input ────────────────────────
            //
            // `,` reads one byte from the input buffer into the current cell.
            // If the input is exhausted, stores 0 (EOF convention).
            OP_INPUT => {
                let byte = if self.input_pos < self.input_buffer.len() {
                    let b = self.input_buffer[self.input_pos];
                    self.input_pos += 1;
                    b
                } else {
                    0 // EOF
                };
                self.tape[self.dp] = byte;
                format!("INPUT: tape[{}] = {} ('{}')", self.dp, byte, byte as char)
            }

            // ── OP_LOOP_START: Enter loop (or skip) ─────────────────────
            //
            // `[` checks the current cell. If it's zero, the loop body
            // should be skipped entirely — jump to the instruction after
            // the matching `]`. If non-zero, proceed into the loop body.
            //
            // This is how Brainfuck implements conditional execution:
            //
            //   [        if tape[dp] == 0, jump past ]
            //   ...      loop body (only runs if tape[dp] != 0)
            //   ]        if tape[dp] != 0, jump back to [
            OP_LOOP_START => {
                if self.tape[self.dp] == 0 {
                    let target = match &instr.operand {
                        Some(Operand::Index(t)) => *t,
                        _ => {
                            return Err(VMError::InvalidOperand(
                                "LOOP_START requires a jump target".to_string(),
                            ))
                        }
                    };
                    self.vm.pc = target;
                    jumped = true;
                    format!("LOOP_START: cell is 0, jumping to {}", target)
                } else {
                    format!("LOOP_START: cell is {}, entering loop", self.tape[self.dp])
                }
            }

            // ── OP_LOOP_END: End loop (or repeat) ───────────────────────
            //
            // `]` checks the current cell. If it's non-zero, jump back
            // to the matching `[` to repeat the loop. If zero, fall through
            // (exit the loop).
            OP_LOOP_END => {
                if self.tape[self.dp] != 0 {
                    let target = match &instr.operand {
                        Some(Operand::Index(t)) => *t,
                        _ => {
                            return Err(VMError::InvalidOperand(
                                "LOOP_END requires a jump target".to_string(),
                            ))
                        }
                    };
                    self.vm.pc = target;
                    jumped = true;
                    format!(
                        "LOOP_END: cell is {}, jumping back to {}",
                        self.tape[self.dp], target
                    )
                } else {
                    format!("LOOP_END: cell is 0, exiting loop")
                }
            }

            // ── OP_HALT: Stop execution ─────────────────────────────────
            OP_HALT => {
                self.vm.halted = true;
                "HALT".to_string()
            }

            // ── Unknown opcode ──────────────────────────────────────────
            other => {
                return Err(VMError::InvalidOpcode(format!(
                    "Unknown Brainfuck opcode: 0x{:02X}",
                    other
                )));
            }
        };

        // Advance PC if we didn't jump
        if !jumped {
            self.vm.pc += 1;
        }

        Ok(VMTrace {
            pc: pc_before,
            instruction: instr_clone,
            stack_before,
            stack_after: self.vm.stack.clone(),
            variables: self.vm.variables.clone(),
            output: output_str,
            description,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Section 4: High-Level API
// ─────────────────────────────────────────────────────────────────────────────

/// The result of executing a Brainfuck program.
///
/// Bundles together all the interesting output from execution:
/// - The text output (from `.` instructions)
/// - The final tape state
/// - The final data pointer position
/// - Execution traces (for debugging/visualization)
/// - Total number of steps executed
pub struct BrainfuckResult {
    /// The concatenated output from all `.` instructions.
    pub output: String,
    /// The final state of the tape.
    pub tape: Vec<u8>,
    /// The final data pointer position.
    pub dp: usize,
    /// Execution traces (one per instruction executed).
    pub traces: Vec<VMTrace>,
    /// Total number of instructions executed.
    pub steps: usize,
}

/// Execute a Brainfuck program from source code.
///
/// This is the simplest way to run Brainfuck code. It handles the full
/// pipeline: translate source to bytecode, then execute it.
///
/// ## Parameters
///
/// - `source`: The Brainfuck source code (any non-instruction characters are comments)
/// - `input_data`: Input to feed to `,` instructions
///
/// ## Returns
///
/// A `BrainfuckResult` containing the output, final tape state, traces, etc.
///
/// ## Example
///
/// ```rust
/// use brainfuck::execute_brainfuck;
///
/// // Simple addition: 2 + 3 = 5, output as ASCII '5' (char code 53)
/// let result = execute_brainfuck("++>+++<[->+<]>.", "").unwrap();
/// assert_eq!(result.output.as_bytes()[0], 5);
/// ```
pub fn execute_brainfuck(source: &str, input_data: &str) -> Result<BrainfuckResult, String> {
    // Step 1: Translate source to bytecode
    let code = translate(source)?;

    // Step 2: Create a VM with the input data
    let mut bf_vm = BrainfuckVM::new(input_data);

    // Step 3: Execute the bytecode
    let traces = bf_vm
        .execute(&code)
        .map_err(|e| format!("Execution error: {}", e))?;

    // Step 4: Collect results
    let output = bf_vm.vm.output.join("");
    let steps = traces.len();

    Ok(BrainfuckResult {
        output,
        tape: bf_vm.tape,
        dp: bf_vm.dp,
        traces,
        steps,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Section 5: Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Translator tests ─────────────────────────────────────────────────

    #[test]
    fn test_translate_basic() {
        let code = translate("+-><.,").unwrap();
        // 6 instructions + HALT = 7
        assert_eq!(code.instructions.len(), 7);
        assert_eq!(code.instructions[0].opcode, OP_INC);
        assert_eq!(code.instructions[1].opcode, OP_DEC);
        assert_eq!(code.instructions[2].opcode, OP_RIGHT);
        assert_eq!(code.instructions[3].opcode, OP_LEFT);
        assert_eq!(code.instructions[4].opcode, OP_OUTPUT);
        assert_eq!(code.instructions[5].opcode, OP_INPUT);
        assert_eq!(code.instructions[6].opcode, OP_HALT);
    }

    #[test]
    fn test_translate_ignores_comments() {
        let code = translate("hello + world -").unwrap();
        // Only + and - are instructions
        assert_eq!(code.instructions.len(), 3); // +, -, HALT
        assert_eq!(code.instructions[0].opcode, OP_INC);
        assert_eq!(code.instructions[1].opcode, OP_DEC);
    }

    #[test]
    fn test_translate_bracket_matching() {
        let code = translate("[+-]").unwrap();
        // [, +, -, ], HALT = 5 instructions
        assert_eq!(code.instructions.len(), 5);

        // '[' at index 0 should jump to index 4 (past ']')
        match &code.instructions[0].operand {
            Some(Operand::Index(4)) => {}
            other => panic!("Expected LOOP_START target 4, got {:?}", other),
        }

        // ']' at index 3 should jump back to index 0
        match &code.instructions[3].operand {
            Some(Operand::Index(0)) => {}
            other => panic!("Expected LOOP_END target 0, got {:?}", other),
        }
    }

    #[test]
    fn test_translate_nested_brackets() {
        let code = translate("[[]]").unwrap();
        // [, [, ], ], HALT = 5 instructions

        // Outer '[' at 0 jumps past outer ']' at 3 → target 4
        match &code.instructions[0].operand {
            Some(Operand::Index(4)) => {}
            other => panic!("Expected outer LOOP_START target 4, got {:?}", other),
        }

        // Inner '[' at 1 jumps past inner ']' at 2 → target 3
        match &code.instructions[1].operand {
            Some(Operand::Index(3)) => {}
            other => panic!("Expected inner LOOP_START target 3, got {:?}", other),
        }

        // Inner ']' at 2 jumps back to inner '[' at 1
        match &code.instructions[2].operand {
            Some(Operand::Index(1)) => {}
            other => panic!("Expected inner LOOP_END target 1, got {:?}", other),
        }

        // Outer ']' at 3 jumps back to outer '[' at 0
        match &code.instructions[3].operand {
            Some(Operand::Index(0)) => {}
            other => panic!("Expected outer LOOP_END target 0, got {:?}", other),
        }
    }

    #[test]
    fn test_translate_unmatched_close() {
        let result = translate("]");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unmatched ']'"));
    }

    #[test]
    fn test_translate_unmatched_open() {
        let result = translate("[");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unmatched '['"));
    }

    #[test]
    fn test_translate_empty() {
        let code = translate("").unwrap();
        assert_eq!(code.instructions.len(), 1); // Just HALT
        assert_eq!(code.instructions[0].opcode, OP_HALT);
    }

    // ── Individual opcode execution tests ────────────────────────────────

    #[test]
    fn test_op_right() {
        let result = execute_brainfuck(">", "").unwrap();
        assert_eq!(result.dp, 1);
    }

    #[test]
    fn test_op_left_wraps() {
        // Starting at dp=0, moving left should wrap to TAPE_SIZE-1
        let result = execute_brainfuck("<", "").unwrap();
        assert_eq!(result.dp, TAPE_SIZE - 1);
    }

    #[test]
    fn test_op_inc() {
        let result = execute_brainfuck("+++", "").unwrap();
        assert_eq!(result.tape[0], 3);
    }

    #[test]
    fn test_op_dec() {
        let result = execute_brainfuck("+++-", "").unwrap();
        assert_eq!(result.tape[0], 2);
    }

    #[test]
    fn test_op_output() {
        // Set cell to 65 ('A') and output it
        // 65 = 64 + 1. We need 65 increments.
        let source = &"+".repeat(65);
        let source = format!("{}.", source);
        let result = execute_brainfuck(&source, "").unwrap();
        assert_eq!(result.output, "A");
    }

    #[test]
    fn test_op_input() {
        let result = execute_brainfuck(",.", "X").unwrap();
        assert_eq!(result.output, "X");
        assert_eq!(result.tape[0], b'X');
    }

    #[test]
    fn test_op_input_eof() {
        // Two input reads but only one byte available
        let result = execute_brainfuck(",,", "A").unwrap();
        assert_eq!(result.tape[0], 0); // second read gets 0 (EOF)
    }

    // ── Loop tests ───────────────────────────────────────────────────────

    #[test]
    fn test_simple_loop() {
        // Set cell 0 to 3, then loop: decrement cell 0, increment cell 1
        // Result: cell 0 = 0, cell 1 = 3
        let result = execute_brainfuck("+++[->+<]", "").unwrap();
        assert_eq!(result.tape[0], 0);
        assert_eq!(result.tape[1], 3);
    }

    #[test]
    fn test_loop_skip_on_zero() {
        // Cell 0 starts at 0, so the loop body should be skipped entirely
        let result = execute_brainfuck("[+++]", "").unwrap();
        assert_eq!(result.tape[0], 0); // loop body never ran
    }

    #[test]
    fn test_nested_loops() {
        // Multiply 3 * 4 = 12 using nested loops:
        // Cell 0 = 3 (outer counter)
        // Inner loop: add 4 to cell 1 each outer iteration
        let result = execute_brainfuck("+++[>++++<-]", "").unwrap();
        assert_eq!(result.tape[0], 0);
        assert_eq!(result.tape[1], 12);
    }

    // ── Wrapping tests ───────────────────────────────────────────────────

    #[test]
    fn test_inc_wraps_at_255() {
        // Set cell to 255, then increment — should wrap to 0
        let source = &"+".repeat(256);
        let result = execute_brainfuck(source, "").unwrap();
        assert_eq!(result.tape[0], 0);
    }

    #[test]
    fn test_dec_wraps_at_zero() {
        // Decrement from 0 should wrap to 255
        let result = execute_brainfuck("-", "").unwrap();
        assert_eq!(result.tape[0], 255);
    }

    // ── End-to-end tests ─────────────────────────────────────────────────

    #[test]
    fn test_empty_program() {
        let result = execute_brainfuck("", "").unwrap();
        assert_eq!(result.output, "");
        assert_eq!(result.dp, 0);
        assert_eq!(result.steps, 1); // just HALT
    }

    #[test]
    fn test_comments_only() {
        let result = execute_brainfuck("this is all comments", "").unwrap();
        assert_eq!(result.output, "");
        assert_eq!(result.steps, 1); // just HALT
    }

    #[test]
    fn test_addition() {
        // Add 2 + 3: put 2 in cell 0, 3 in cell 1, move cell 0 into cell 1
        let result = execute_brainfuck("++>+++<[->+<]", "").unwrap();
        assert_eq!(result.tape[0], 0);
        assert_eq!(result.tape[1], 5);
    }

    #[test]
    fn test_hello_world() {
        // The classic Brainfuck Hello World program.
        // This is a well-known program that outputs "Hello World!\n".
        let source = "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.";
        let result = execute_brainfuck(source, "").unwrap();
        assert_eq!(result.output, "Hello World!\n");
    }

    #[test]
    fn test_input_echo() {
        // Read 3 characters and echo them back
        let result = execute_brainfuck(",.,.,.", "ABC").unwrap();
        assert_eq!(result.output, "ABC");
    }

    #[test]
    fn test_cell_copy() {
        // Copy cell 0 to cell 2 using cell 1 as temp:
        // cell 0 = 5
        // loop: dec cell 0, inc cell 1, inc cell 2
        // loop: dec cell 1, inc cell 0 (restore cell 0)
        let result = execute_brainfuck("+++++[->+>+<<]>[-<+>]", "").unwrap();
        assert_eq!(result.tape[0], 5); // restored
        assert_eq!(result.tape[2], 5); // copy
    }

    #[test]
    fn test_right_wraps() {
        // Move right TAPE_SIZE times — should wrap back to 0
        let source = &">".repeat(TAPE_SIZE);
        let result = execute_brainfuck(source, "").unwrap();
        assert_eq!(result.dp, 0);
    }

    // ── BrainfuckVM direct usage tests ───────────────────────────────────

    #[test]
    fn test_vm_direct_usage() {
        let code = translate("+++.").unwrap();
        let mut bf_vm = BrainfuckVM::new("");
        let traces = bf_vm.execute(&code).unwrap();

        // 3 increments + 1 output + 1 halt = 5 traces
        assert_eq!(traces.len(), 5);
        assert_eq!(bf_vm.tape[0], 3);
        assert_eq!(bf_vm.vm.output.len(), 1);
    }

    #[test]
    fn test_vm_traces_contain_descriptions() {
        let code = translate("+>.").unwrap();
        let mut bf_vm = BrainfuckVM::new("");
        let traces = bf_vm.execute(&code).unwrap();

        assert!(traces[0].description.contains("INC"));
        assert!(traces[1].description.contains("RIGHT"));
        assert!(traces[2].description.contains("OUTPUT"));
    }

    #[test]
    fn test_execute_brainfuck_error_on_bad_brackets() {
        let result = execute_brainfuck("[", "");
        assert!(result.is_err());
    }

    // ── Stress / edge case tests ─────────────────────────────────────────

    #[test]
    fn test_many_nested_loops() {
        // Deeply nested but valid: [[[...]]]
        // Cell starts at 0, so all loops are skipped
        let result = execute_brainfuck("[[[[[[[]]]]]]]", "").unwrap();
        assert_eq!(result.output, "");
    }

    #[test]
    fn test_multiple_outputs() {
        // Output 3 different values
        let result = execute_brainfuck("+++.>+++++.>+++++++.", "").unwrap();
        assert_eq!(result.output.len(), 3);
        assert_eq!(result.output.as_bytes()[0], 3);
        assert_eq!(result.output.as_bytes()[1], 5);
        assert_eq!(result.output.as_bytes()[2], 7);
    }
}
