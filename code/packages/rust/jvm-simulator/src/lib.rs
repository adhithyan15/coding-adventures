//! # JVM Simulator — a typed stack-based virtual machine.
//!
//! ## Stack machine with typed opcodes
//!
//! Like WASM, the JVM is a stack-based machine. But the JVM is *typed*:
//! where a generic VM might have a single `ADD` instruction, the JVM
//! forces type integrity through the opcode itself:
//!
//! ```text
//!     iadd    <-- integer add
//!     ladd    <-- long add
//!     fadd    <-- float add
//!     dadd    <-- double add
//! ```
//!
//! This type-in-the-opcode design means the JVM can verify type safety
//! at class-loading time without running the code.
//!
//! ## Local variables vs registers
//!
//! The JVM has "local variable slots" instead of registers. These are
//! numbered starting from 0 and accessed via `iload`/`istore` instructions.
//! The JVM spec even provides compact shortcut opcodes for slots 0-3:
//!
//! ```text
//!     iload_0   (1 byte)   vs   iload 0   (2 bytes)
//!     istore_2  (1 byte)   vs   istore 2  (2 bytes)
//! ```
//!
//! ## Supported instructions
//!
//! | Opcode | Mnemonic    | Description                      |
//! |--------|-------------|----------------------------------|
//! | 0x03-08| iconst_N    | Push integer constant 0-5        |
//! | 0x10   | bipush      | Push byte as integer             |
//! | 0x12   | ldc         | Push from constant pool          |
//! | 0x15   | iload       | Load int from local (2-byte)     |
//! | 0x1A-1D| iload_N     | Load int from local 0-3          |
//! | 0x36   | istore      | Store int to local (2-byte)      |
//! | 0x3B-3E| istore_N    | Store int to local 0-3           |
//! | 0x60   | iadd        | Integer add                      |
//! | 0x64   | isub        | Integer subtract                 |
//! | 0x68   | imul        | Integer multiply                 |
//! | 0x6C   | idiv        | Integer divide                   |
//! | 0x9F   | if_icmpeq   | Branch if ints equal             |
//! | 0xA3   | if_icmpgt   | Branch if int greater than       |
//! | 0xA7   | goto        | Unconditional branch             |
//! | 0xAC   | ireturn     | Return integer from method       |
//! | 0xB1   | return      | Return void from method          |

// ===========================================================================
// Opcode constants
// ===========================================================================

pub const OP_ICONST_0: u8 = 0x03;
pub const OP_ICONST_5: u8 = 0x08;
pub const OP_BIPUSH: u8 = 0x10;
pub const OP_LDC: u8 = 0x12;
pub const OP_ILOAD: u8 = 0x15;
pub const OP_ILOAD_0: u8 = 0x1A;
pub const OP_ILOAD_3: u8 = 0x1D;
pub const OP_ISTORE: u8 = 0x36;
pub const OP_ISTORE_0: u8 = 0x3B;
pub const OP_ISTORE_3: u8 = 0x3E;
pub const OP_IADD: u8 = 0x60;
pub const OP_ISUB: u8 = 0x64;
pub const OP_IMUL: u8 = 0x68;
pub const OP_IDIV: u8 = 0x6C;
pub const OP_IF_ICMPEQ: u8 = 0x9F;
pub const OP_IF_ICMPGT: u8 = 0xA3;
pub const OP_GOTO: u8 = 0xA7;
pub const OP_IRETURN: u8 = 0xAC;
pub const OP_RETURN: u8 = 0xB1;

// ===========================================================================
// Trace type
// ===========================================================================

/// A record of one JVM instruction's execution.
#[derive(Debug, Clone)]
pub struct JVMTrace {
    pub pc: usize,
    pub opcode: String,
    pub stack_before: Vec<i32>,
    pub stack_after: Vec<i32>,
    pub locals_snapshot: Vec<Option<i32>>,
    pub description: String,
}

// ===========================================================================
// Simulator
// ===========================================================================

/// The JVM simulator -- a typed stack-based virtual machine.
pub struct JVMSimulator {
    pub stack: Vec<i32>,
    pub locals: Vec<Option<i32>>,
    pub constants: Vec<i32>,
    pub pc: usize,
    pub halted: bool,
    pub return_value: Option<i32>,
    bytecode: Vec<u8>,
}

impl JVMSimulator {
    pub fn new() -> Self {
        JVMSimulator {
            stack: Vec::new(),
            locals: vec![None; 16],
            constants: Vec::new(),
            pc: 0,
            halted: false,
            return_value: None,
            bytecode: Vec::new(),
        }
    }

    /// Load bytecode, optional constant pool, and configure local variable count.
    pub fn load(&mut self, bytecode: &[u8], constants: &[i32], num_locals: usize) {
        self.bytecode = bytecode.to_vec();
        self.constants = constants.to_vec();
        self.stack.clear();
        self.locals = vec![None; num_locals];
        self.pc = 0;
        self.halted = false;
        self.return_value = None;
    }

    /// Execute one instruction and return its trace.
    pub fn step(&mut self) -> JVMTrace {
        assert!(!self.halted, "JVM simulator has halted");
        assert!(
            self.pc < self.bytecode.len(),
            "PC ({}) past end of bytecode",
            self.pc
        );

        let pc = self.pc;
        let stack_before = self.stack.clone();
        let opcode_byte = self.bytecode[pc];

        self.execute_opcode(opcode_byte, stack_before, pc)
    }

    /// Run until halt or max_steps reached.
    pub fn run(&mut self, max_steps: usize) -> Vec<JVMTrace> {
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.halted {
                break;
            }
            traces.push(self.step());
        }
        traces
    }

    fn copy_locals(&self) -> Vec<Option<i32>> {
        self.locals.clone()
    }

    /// Wrap a value to 32-bit integer range (simulating JVM i32 overflow).
    fn to_i32(val: i64) -> i32 {
        val as i32
    }

    fn execute_opcode(&mut self, opcode: u8, stack_before: Vec<i32>, pc: usize) -> JVMTrace {
        // iconst_N: push small integer constants 0-5.
        if opcode >= OP_ICONST_0 && opcode <= OP_ICONST_5 {
            let val = (opcode - OP_ICONST_0) as i32;
            self.stack.push(val);
            self.pc += 1;
            return JVMTrace {
                pc,
                opcode: format!("iconst_{}", val),
                stack_before,
                stack_after: self.stack.clone(),
                locals_snapshot: self.copy_locals(),
                description: format!("push {}", val),
            };
        }

        // iload_N: load from local variable slots 0-3 (compact form).
        if opcode >= OP_ILOAD_0 && opcode <= OP_ILOAD_3 {
            let slot = (opcode - OP_ILOAD_0) as usize;
            return self.do_iload(pc, slot, &format!("iload_{}", slot), stack_before, 1);
        }

        // istore_N: store to local variable slots 0-3 (compact form).
        if opcode >= OP_ISTORE_0 && opcode <= OP_ISTORE_3 {
            let slot = (opcode - OP_ISTORE_0) as usize;
            return self.do_istore(pc, slot, &format!("istore_{}", slot), stack_before, 1);
        }

        match opcode {
            OP_BIPUSH => {
                let raw = self.bytecode[pc + 1] as i8;
                let val = raw as i32;
                self.stack.push(val);
                self.pc += 2;
                JVMTrace {
                    pc,
                    opcode: "bipush".to_string(),
                    stack_before,
                    stack_after: self.stack.clone(),
                    locals_snapshot: self.copy_locals(),
                    description: format!("push {}", val),
                }
            }
            OP_LDC => {
                let idx = self.bytecode[pc + 1] as usize;
                assert!(
                    idx < self.constants.len(),
                    "Constant pool index {} out of range",
                    idx
                );
                let val = self.constants[idx];
                self.stack.push(val);
                self.pc += 2;
                JVMTrace {
                    pc,
                    opcode: "ldc".to_string(),
                    stack_before,
                    stack_after: self.stack.clone(),
                    locals_snapshot: self.copy_locals(),
                    description: format!("push constant[{}] = {}", idx, val),
                }
            }
            OP_ILOAD => {
                let slot = self.bytecode[pc + 1] as usize;
                self.do_iload(pc, slot, "iload", stack_before, 2)
            }
            OP_ISTORE => {
                let slot = self.bytecode[pc + 1] as usize;
                self.do_istore(pc, slot, "istore", stack_before, 2)
            }
            OP_IADD => self.do_binary_op(pc, "iadd", |a, b| a.wrapping_add(b), stack_before),
            OP_ISUB => self.do_binary_op(pc, "isub", |a, b| a.wrapping_sub(b), stack_before),
            OP_IMUL => self.do_binary_op(pc, "imul", |a, b| a.wrapping_mul(b), stack_before),
            OP_IDIV => {
                assert!(self.stack.len() >= 2, "Stack underflow");
                assert!(
                    *self.stack.last().unwrap() != 0,
                    "ArithmeticException: division by zero"
                );
                self.do_binary_op(pc, "idiv", |a, b| a.wrapping_div(b), stack_before)
            }
            OP_GOTO => {
                let b1 = self.bytecode[pc + 1];
                let b2 = self.bytecode[pc + 2];
                let raw = ((b1 as u16) << 8) | (b2 as u16);
                let offset = if raw >= 0x8000 {
                    raw as i32 - 0x10000
                } else {
                    raw as i32
                };
                let target = (pc as i32 + offset) as usize;
                self.pc = target;
                JVMTrace {
                    pc,
                    opcode: "goto".to_string(),
                    stack_before,
                    stack_after: self.stack.clone(),
                    locals_snapshot: self.copy_locals(),
                    description: format!("jump to PC={}", target),
                }
            }
            OP_IF_ICMPEQ => {
                self.do_if_icmp(pc, "if_icmpeq", stack_before, |a, b| a == b)
            }
            OP_IF_ICMPGT => {
                self.do_if_icmp(pc, "if_icmpgt", stack_before, |a, b| a > b)
            }
            OP_IRETURN => {
                assert!(!self.stack.is_empty(), "Stack underflow");
                let val = self.stack.pop().unwrap();
                self.return_value = Some(val);
                self.halted = true;
                self.pc += 1;
                JVMTrace {
                    pc,
                    opcode: "ireturn".to_string(),
                    stack_before,
                    stack_after: self.stack.clone(),
                    locals_snapshot: self.copy_locals(),
                    description: format!("return {}", val),
                }
            }
            OP_RETURN => {
                self.halted = true;
                self.pc += 1;
                JVMTrace {
                    pc,
                    opcode: "return".to_string(),
                    stack_before,
                    stack_after: self.stack.clone(),
                    locals_snapshot: self.copy_locals(),
                    description: "return void".to_string(),
                }
            }
            _ => panic!("Unimplemented opcode: 0x{:02X}", opcode),
        }
    }

    fn do_iload(
        &mut self,
        pc: usize,
        slot: usize,
        mnemonic: &str,
        stack_before: Vec<i32>,
        size: usize,
    ) -> JVMTrace {
        let val = self.locals[slot].expect("Local variable uninitialized");
        self.stack.push(val);
        self.pc += size;
        JVMTrace {
            pc,
            opcode: mnemonic.to_string(),
            stack_before,
            stack_after: self.stack.clone(),
            locals_snapshot: self.copy_locals(),
            description: format!("push locals[{}] = {}", slot, val),
        }
    }

    fn do_istore(
        &mut self,
        pc: usize,
        slot: usize,
        mnemonic: &str,
        stack_before: Vec<i32>,
        size: usize,
    ) -> JVMTrace {
        assert!(!self.stack.is_empty(), "Stack underflow");
        let val = self.stack.pop().unwrap();
        self.locals[slot] = Some(val);
        self.pc += size;
        JVMTrace {
            pc,
            opcode: mnemonic.to_string(),
            stack_before,
            stack_after: self.stack.clone(),
            locals_snapshot: self.copy_locals(),
            description: format!("pop {}, store in locals[{}]", val, slot),
        }
    }

    fn do_binary_op<F>(
        &mut self,
        pc: usize,
        mnemonic: &str,
        op: F,
        stack_before: Vec<i32>,
    ) -> JVMTrace
    where
        F: Fn(i32, i32) -> i32,
    {
        assert!(self.stack.len() >= 2, "Stack underflow");
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        let result = Self::to_i32(op(a, b) as i64);
        self.stack.push(result);
        self.pc += 1;
        JVMTrace {
            pc,
            opcode: mnemonic.to_string(),
            stack_before,
            stack_after: self.stack.clone(),
            locals_snapshot: self.copy_locals(),
            description: format!("pop {} and {}, push {}", b, a, result),
        }
    }

    fn do_if_icmp<F>(
        &mut self,
        pc: usize,
        mnemonic: &str,
        stack_before: Vec<i32>,
        op: F,
    ) -> JVMTrace
    where
        F: Fn(i32, i32) -> bool,
    {
        assert!(self.stack.len() >= 2, "Stack underflow");
        let b1 = self.bytecode[pc + 1];
        let b2 = self.bytecode[pc + 2];
        let raw = ((b1 as u16) << 8) | (b2 as u16);
        let offset = if raw >= 0x8000 {
            raw as i32 - 0x10000
        } else {
            raw as i32
        };
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        let taken = op(a, b);

        let description;
        if taken {
            let target = (pc as i32 + offset) as usize;
            self.pc = target;
            description = format!("pop {} and {}, true, jump to PC={}", b, a, target);
        } else {
            self.pc = pc + 3;
            description = format!("pop {} and {}, false, fall through", b, a);
        }

        JVMTrace {
            pc,
            opcode: mnemonic.to_string(),
            stack_before,
            stack_after: self.stack.clone(),
            locals_snapshot: self.copy_locals(),
            description,
        }
    }
}

impl Default for JVMSimulator {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Encoding helpers
// ===========================================================================

/// Encode a small integer constant. Uses iconst_N for 0-5, bipush for -128..127.
pub fn encode_iconst(n: i32) -> Vec<u8> {
    if n >= 0 && n <= 5 {
        return vec![(OP_ICONST_0 as i32 + n) as u8];
    }
    if n >= -128 && n <= 127 {
        return vec![OP_BIPUSH, n as u8];
    }
    panic!("Out of range for encode_iconst");
}

/// Encode istore with slot. Uses compact form for slots 0-3.
pub fn encode_istore(slot: u8) -> Vec<u8> {
    if slot <= 3 {
        return vec![OP_ISTORE_0 + slot];
    }
    vec![OP_ISTORE, slot]
}

/// Encode iload with slot. Uses compact form for slots 0-3.
pub fn encode_iload(slot: u8) -> Vec<u8> {
    if slot <= 3 {
        return vec![OP_ILOAD_0 + slot];
    }
    vec![OP_ILOAD, slot]
}

/// A bytecode instruction for the assembler.
pub struct Instr {
    pub opcode: u8,
    pub params: Vec<i32>,
}

/// Assemble a sequence of JVM instructions into flat bytecode.
pub fn assemble_jvm(instructions: &[Instr]) -> Vec<u8> {
    let mut res = Vec::new();
    for inst in instructions {
        res.push(inst.opcode);
        match inst.opcode {
            OP_BIPUSH | OP_ILOAD | OP_ISTORE | OP_LDC => {
                res.push(inst.params[0] as u8);
            }
            OP_GOTO | OP_IF_ICMPEQ | OP_IF_ICMPGT => {
                let off = inst.params[0] as u16;
                res.push(((off >> 8) & 0xFF) as u8);
                res.push((off & 0xFF) as u8);
            }
            _ => {}
        }
    }
    res
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test basic JVM program: x = 1 + 2, return 3.
    #[test]
    fn jvm_basic_program() {
        let mut sim = JVMSimulator::new();
        let prog = assemble_jvm(&[
            Instr { opcode: OP_ICONST_0 + 1, params: vec![] },  // iconst_1
            Instr { opcode: OP_ICONST_0 + 2, params: vec![] },  // iconst_2
            Instr { opcode: OP_IADD, params: vec![] },
            Instr { opcode: OP_ISTORE_0, params: vec![] },
            Instr { opcode: OP_ILOAD_0, params: vec![] },
            Instr { opcode: OP_IRETURN, params: vec![] },
        ]);
        sim.load(&prog, &[], 16);
        let traces = sim.run(100);
        assert_eq!(traces.len(), 6);
        assert_eq!(sim.return_value, Some(3));
    }

    /// Test ldc and bipush with negative values.
    #[test]
    fn jvm_ldc_and_bipush() {
        let mut sim = JVMSimulator::new();
        let prog = assemble_jvm(&[
            Instr { opcode: OP_BIPUSH, params: vec![-42i32] },
            Instr { opcode: OP_LDC, params: vec![0] },
            Instr { opcode: OP_ISUB, params: vec![] },
            Instr { opcode: OP_IRETURN, params: vec![] },
        ]);
        sim.load(&prog, &[100], 16);
        sim.run(100);
        assert_eq!(sim.return_value, Some(-142));
    }

    /// Test if_icmpeq branch taken.
    #[test]
    fn jvm_icmp_goto() {
        let mut sim = JVMSimulator::new();
        let prog = assemble_jvm(&[
            Instr { opcode: OP_ICONST_0 + 5, params: vec![] },  // push 5
            Instr { opcode: OP_ICONST_0 + 5, params: vec![] },  // push 5
            Instr { opcode: OP_IF_ICMPEQ, params: vec![4] },    // offset 4
            Instr { opcode: OP_ICONST_0 + 1, params: vec![] },  // skipped
            Instr { opcode: OP_ICONST_0 + 4, params: vec![] },  // target
            Instr { opcode: OP_IRETURN, params: vec![] },
        ]);
        sim.load(&prog, &[], 16);
        sim.run(10);
        assert_eq!(sim.return_value, Some(4));

        let traces = sim.run(10);
        // Re-run to check traces contain if_icmpeq
        sim.load(&prog, &[], 16);
        let traces = sim.run(10);
        let found = traces.iter().any(|t| t.opcode == "if_icmpeq");
        assert!(found, "if_icmpeq should appear in traces");
    }

    /// Division by zero should panic.
    #[test]
    #[should_panic(expected = "division by zero")]
    fn jvm_div_by_zero() {
        let mut sim = JVMSimulator::new();
        let prog = assemble_jvm(&[
            Instr { opcode: OP_ICONST_0 + 5, params: vec![] },
            Instr { opcode: OP_ICONST_0, params: vec![] },
            Instr { opcode: OP_IDIV, params: vec![] },
        ]);
        sim.load(&prog, &[], 16);
        sim.run(10);
    }

    /// Halted simulator should panic on step.
    #[test]
    #[should_panic(expected = "JVM simulator has halted")]
    fn jvm_halted_panics() {
        let mut sim = JVMSimulator::new();
        sim.halted = true;
        sim.step();
    }
}
