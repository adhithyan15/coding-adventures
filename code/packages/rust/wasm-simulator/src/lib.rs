//! # WebAssembly Simulator — a stack-based virtual machine.
//!
//! ## Stack machines vs register machines
//!
//! Previous simulators (RISC-V, ARM) are *register machines*: instructions name
//! specific registers as explicit operands (e.g., `add R2, R1, R0`).
//!
//! WASM is a *stack machine*: instructions don't name their operands. Instead,
//! operands live on an implicit *operand stack*. Push values onto the stack,
//! then invoke an operation -- it autonomously pops its inputs and pushes the result.
//!
//! ```text
//!     Register machine:           Stack machine:
//!     add R2, R1, R0              i32.const 10
//!     (explicit: "add R1 and R0,  i32.const 20
//!      put result in R2")         i32.add
//!                                 (implicit: pop 20 and 10,
//!                                  push 30)
//! ```
//!
//! ## Variable-length encoding
//!
//! Unlike RISC-V (fixed 32-bit instructions), WASM uses variable-length
//! bytecode. Each instruction is 1 byte (the opcode), optionally followed
//! by operand bytes. This is more compact but harder to decode.
//!
//! ## Supported instructions
//!
//! | Opcode | Mnemonic      | Size  | Description                    |
//! |--------|---------------|-------|--------------------------------|
//! | 0x0B   | `end`         | 1     | Halt execution                 |
//! | 0x20   | `local.get`   | 2     | Push local variable to stack   |
//! | 0x21   | `local.set`   | 2     | Pop stack into local variable  |
//! | 0x41   | `i32.const`   | 5     | Push 32-bit constant           |
//! | 0x6A   | `i32.add`     | 1     | Pop two, push sum              |
//! | 0x6B   | `i32.sub`     | 1     | Pop two, push difference       |

// ===========================================================================
// Opcode constants
// ===========================================================================

/// Standard WASM instruction bytecodes.
const OP_END: u8 = 0x0B;
const OP_LOCAL_GET: u8 = 0x20;
const OP_LOCAL_SET: u8 = 0x21;
const OP_I32_CONST: u8 = 0x41;
const OP_I32_ADD: u8 = 0x6A;
const OP_I32_SUB: u8 = 0x6B;

// ===========================================================================
// Instruction type
// ===========================================================================

/// A decoded WASM instruction with its opcode, mnemonic, optional operand,
/// and byte size (how far to advance the PC).
#[derive(Debug, Clone)]
pub struct WasmInstruction {
    pub opcode: u8,
    pub mnemonic: String,
    /// Some instructions carry an operand (e.g., `i32.const 42` or `local.get 0`).
    pub operand: Option<i32>,
    /// Total byte length of this instruction (opcode + operand bytes).
    pub size: usize,
}

// ===========================================================================
// Step trace
// ===========================================================================

/// A complete record of one WASM instruction's execution.
///
/// Captures the stack state before and after, local variable snapshot,
/// and a human-readable description. This is the WASM equivalent of
/// the CPU pipeline trace.
#[derive(Debug, Clone)]
pub struct WasmStepTrace {
    pub pc: usize,
    pub instruction: WasmInstruction,
    pub stack_before: Vec<i32>,
    pub stack_after: Vec<i32>,
    pub locals_snapshot: Vec<i32>,
    pub description: String,
    pub halted: bool,
}

// ===========================================================================
// Decoder
// ===========================================================================

/// Decodes variable-length WASM bytecode into structured instructions.
pub struct WasmDecoder;

impl WasmDecoder {
    /// Decode one instruction at the given PC offset.
    ///
    /// Because WASM uses variable-length encoding, we must inspect the
    /// opcode byte to know how many additional bytes to consume.
    pub fn decode(&self, bytecode: &[u8], pc: usize) -> WasmInstruction {
        let opcode = bytecode[pc];
        match opcode {
            OP_I32_CONST => {
                // 5 bytes: opcode + 4 bytes little-endian i32.
                let val = i32::from_le_bytes([
                    bytecode[pc + 1],
                    bytecode[pc + 2],
                    bytecode[pc + 3],
                    bytecode[pc + 4],
                ]);
                WasmInstruction {
                    opcode,
                    mnemonic: "i32.const".to_string(),
                    operand: Some(val),
                    size: 5,
                }
            }
            OP_I32_ADD => WasmInstruction {
                opcode,
                mnemonic: "i32.add".to_string(),
                operand: None,
                size: 1,
            },
            OP_I32_SUB => WasmInstruction {
                opcode,
                mnemonic: "i32.sub".to_string(),
                operand: None,
                size: 1,
            },
            OP_LOCAL_GET => {
                let idx = bytecode[pc + 1] as i32;
                WasmInstruction {
                    opcode,
                    mnemonic: "local.get".to_string(),
                    operand: Some(idx),
                    size: 2,
                }
            }
            OP_LOCAL_SET => {
                let idx = bytecode[pc + 1] as i32;
                WasmInstruction {
                    opcode,
                    mnemonic: "local.set".to_string(),
                    operand: Some(idx),
                    size: 2,
                }
            }
            OP_END => WasmInstruction {
                opcode,
                mnemonic: "end".to_string(),
                operand: None,
                size: 1,
            },
            _ => panic!("Unknown WASM opcode: 0x{:02X} at PC={}", opcode, pc),
        }
    }
}

// ===========================================================================
// Executor
// ===========================================================================

/// Executes WASM instructions by mutating the stack and local variables.
pub struct WasmExecutor;

impl WasmExecutor {
    /// Execute one instruction, modifying the stack and locals in place.
    pub fn execute(
        &self,
        instruction: &WasmInstruction,
        stack: &mut Vec<i32>,
        locals: &mut [i32],
        pc: usize,
    ) -> WasmStepTrace {
        let stack_before = stack.clone();

        match instruction.mnemonic.as_str() {
            "i32.const" => {
                let val = instruction.operand.unwrap();
                stack.push(val);
                WasmStepTrace {
                    pc,
                    instruction: instruction.clone(),
                    stack_before,
                    stack_after: stack.clone(),
                    locals_snapshot: locals.to_vec(),
                    description: format!("push {}", val),
                    halted: false,
                }
            }
            "i32.add" => {
                let b = stack.pop().expect("Stack underflow");
                let a = stack.pop().expect("Stack underflow");
                // WASM i32.add wraps to 32 bits (unsigned interpretation).
                let res = (a.wrapping_add(b)) as u32 as i32;
                stack.push(res);
                WasmStepTrace {
                    pc,
                    instruction: instruction.clone(),
                    stack_before,
                    stack_after: stack.clone(),
                    locals_snapshot: locals.to_vec(),
                    description: format!("pop {} and {}, push {}", b, a, res),
                    halted: false,
                }
            }
            "i32.sub" => {
                let b = stack.pop().expect("Stack underflow");
                let a = stack.pop().expect("Stack underflow");
                // Subtraction also wraps to 32 bits unsigned.
                let res = (a.wrapping_sub(b)) as u32 as i32;
                stack.push(res);
                WasmStepTrace {
                    pc,
                    instruction: instruction.clone(),
                    stack_before,
                    stack_after: stack.clone(),
                    locals_snapshot: locals.to_vec(),
                    description: format!("pop {} and {}, push {}", b, a, res),
                    halted: false,
                }
            }
            "local.get" => {
                let idx = instruction.operand.unwrap() as usize;
                let val = locals[idx];
                stack.push(val);
                WasmStepTrace {
                    pc,
                    instruction: instruction.clone(),
                    stack_before,
                    stack_after: stack.clone(),
                    locals_snapshot: locals.to_vec(),
                    description: format!("push locals[{}] = {}", idx, val),
                    halted: false,
                }
            }
            "local.set" => {
                let idx = instruction.operand.unwrap() as usize;
                let val = stack.pop().expect("Stack underflow");
                locals[idx] = val;
                WasmStepTrace {
                    pc,
                    instruction: instruction.clone(),
                    stack_before,
                    stack_after: stack.clone(),
                    locals_snapshot: locals.to_vec(),
                    description: format!("pop {}, store in locals[{}]", val, idx),
                    halted: false,
                }
            }
            "end" => WasmStepTrace {
                pc,
                instruction: instruction.clone(),
                stack_before,
                stack_after: stack.clone(),
                locals_snapshot: locals.to_vec(),
                description: "halt".to_string(),
                halted: true,
            },
            other => panic!("Cannot execute: {}", other),
        }
    }
}

// ===========================================================================
// Simulator
// ===========================================================================

/// The full WASM simulation environment.
///
/// Unlike the register-based ISA simulators that use the generic CPU framework,
/// the WASM simulator manages its own stack-based execution loop. The generic
/// CPU framework is built around 32-bit register machines and doesn't fit
/// a stack machine's architecture.
pub struct WasmSimulator {
    pub stack: Vec<i32>,
    pub locals: Vec<i32>,
    pub pc: usize,
    pub bytecode: Vec<u8>,
    pub halted: bool,
    pub cycle: usize,
    decoder: WasmDecoder,
    executor: WasmExecutor,
}

impl WasmSimulator {
    /// Create a new WASM simulator with the given number of local variables.
    pub fn new(num_locals: usize) -> Self {
        WasmSimulator {
            stack: Vec::new(),
            locals: vec![0; num_locals],
            pc: 0,
            bytecode: Vec::new(),
            halted: false,
            cycle: 0,
            decoder: WasmDecoder,
            executor: WasmExecutor,
        }
    }

    /// Load bytecode into the simulator, resetting all state.
    pub fn load(&mut self, bytecode: &[u8]) {
        self.bytecode = bytecode.to_vec();
        self.pc = 0;
        self.halted = false;
        self.cycle = 0;
        self.stack.clear();
        for local in self.locals.iter_mut() {
            *local = 0;
        }
    }

    /// Execute one instruction and return its trace.
    ///
    /// # Panics
    ///
    /// Panics if the simulator has already halted.
    pub fn step(&mut self) -> WasmStepTrace {
        assert!(!self.halted, "WASM simulator has halted -- no more instructions to execute");
        let instruction = self.decoder.decode(&self.bytecode, self.pc);
        let trace = self.executor.execute(&instruction, &mut self.stack, &mut self.locals, self.pc);
        self.pc += instruction.size;
        self.halted = trace.halted;
        self.cycle += 1;
        trace
    }

    /// Run a program to completion or until max_steps is reached.
    pub fn run(&mut self, program: &[u8], max_steps: usize) -> Vec<WasmStepTrace> {
        self.load(program);
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.halted {
                break;
            }
            traces.push(self.step());
        }
        traces
    }
}

// ===========================================================================
// Encoding helpers
// ===========================================================================

/// Encode `i32.const val` as 5 bytes: opcode + little-endian i32.
pub fn encode_i32_const(val: i32) -> Vec<u8> {
    let bytes = val.to_le_bytes();
    vec![OP_I32_CONST, bytes[0], bytes[1], bytes[2], bytes[3]]
}

/// Encode `i32.add` as a single byte.
pub fn encode_i32_add() -> Vec<u8> {
    vec![OP_I32_ADD]
}

/// Encode `i32.sub` as a single byte.
pub fn encode_i32_sub() -> Vec<u8> {
    vec![OP_I32_SUB]
}

/// Encode `local.get idx` as 2 bytes.
pub fn encode_local_get(idx: u8) -> Vec<u8> {
    vec![OP_LOCAL_GET, idx]
}

/// Encode `local.set idx` as 2 bytes.
pub fn encode_local_set(idx: u8) -> Vec<u8> {
    vec![OP_LOCAL_SET, idx]
}

/// Encode `end` as a single byte.
pub fn encode_end() -> Vec<u8> {
    vec![OP_END]
}

/// Assemble a sequence of encoded instructions into a flat bytecode array.
pub fn assemble_wasm(instructions: &[Vec<u8>]) -> Vec<u8> {
    instructions.iter().flatten().copied().collect()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Full program test: push 1, push 2, add, store, load, push 5, sub, end.
    #[test]
    fn wasm_simulator_full_program() {
        let mut sim = WasmSimulator::new(4);
        let program = assemble_wasm(&[
            encode_i32_const(1),
            encode_i32_const(2),
            encode_i32_add(),
            encode_local_set(0),
            encode_local_get(0),
            encode_i32_const(5),
            encode_i32_sub(),
            encode_end(),
        ]);

        let traces = sim.run(&program, 1000);
        assert_eq!(traces.len(), 8);

        assert_eq!(sim.locals[0], 3);

        // 3 - 5 = -2, but wrapping to u32 gives 4294967294
        // In Rust i32: wrapping_sub gives -2 as i32, then cast to u32 and back
        // The Go version stores as int and checks against 4294967294
        // In our Rust version, (3i32 - 5i32) as u32 as i32 = -2 as u32 as i32
        // = 4294967294 as i32 = -2
        // But the Go code stores the result via uint32() cast, which gives 4294967294.
        // Our Rust stores i32, so -2. Let's match the semantics.
        assert_eq!(sim.stack.len(), 1);
        // The Go test checks: sim.Stack[0] != 4294967294
        // That's -2 interpreted as unsigned. Our i32 result is -2.
        // (3 - 5) as u32 as i32 = 4294967294u32 as i32 = -2
        // We store the u32->i32 cast result, which is -2 in i32.
        // But we do `(a.wrapping_sub(b)) as u32 as i32` which is: (-2i32 as u32) as i32 = -2
        assert_eq!(sim.stack[0] as u32, 4294967294);
    }

    /// Stepping a halted simulator should panic.
    #[test]
    #[should_panic(expected = "WASM simulator has halted")]
    fn halted_step_panics() {
        let mut sim = WasmSimulator::new(1);
        let program = assemble_wasm(&[encode_end()]);
        sim.run(&program, 10);
        sim.step(); // should panic
    }

    /// Unknown opcodes should panic with a descriptive message.
    #[test]
    #[should_panic(expected = "Unknown WASM opcode")]
    fn unknown_opcode_panics() {
        let mut sim = WasmSimulator::new(1);
        let program = vec![0xFF]; // invalid opcode
        sim.run(&program, 10);
    }

    /// The executor should panic on unknown mnemonic.
    #[test]
    #[should_panic(expected = "Cannot execute")]
    fn executor_unknown_mnemonic_panics() {
        let executor = WasmExecutor;
        let instruction = WasmInstruction {
            opcode: 0xFF,
            mnemonic: "unknown.command".to_string(),
            operand: None,
            size: 1,
        };
        let mut stack = vec![1, 2];
        let mut locals = vec![0, 0];
        executor.execute(&instruction, &mut stack, &mut locals, 0);
    }
}
