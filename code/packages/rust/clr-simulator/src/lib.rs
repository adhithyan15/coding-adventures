//! # CLR Simulator — Microsoft's Common Language Runtime.
//!
//! ## CLR vs JVM: Two philosophies of stack machines
//!
//! Both the JVM and CLR are stack-based virtual machines, but they take
//! different approaches to type information:
//!
//! ```text
//!     JVM approach — type in the opcode:
//!         iadd        <-- "i" means int32 addition
//!         ladd        <-- "l" means int64 addition
//!
//!     CLR approach — type inferred from the stack:
//!         add         <-- type inferred! works for int32, int64, float...
//! ```
//!
//! The CLR's approach is more flexible (one opcode handles multiple types)
//! but requires the runtime to track what types are on the stack.
//!
//! ## Nullable stack values
//!
//! Unlike the JVM (which only pushes concrete integers), the CLR supports
//! `ldnull` to push a null reference. Our simulator models this with
//! `Option<i32>` -- `None` represents null, `Some(val)` represents a value.
//!
//! ## Two-byte opcodes
//!
//! The CLR uses a prefix byte (0xFE) for extended opcodes like comparison
//! instructions (ceq, cgt, clt). This is different from the JVM where
//! all opcodes are single bytes.

// ===========================================================================
// Opcode constants
// ===========================================================================

pub const OP_NOP: u8 = 0x00;
pub const OP_LDNULL: u8 = 0x01;
pub const OP_LDLOC_0: u8 = 0x06;
pub const OP_LDLOC_3: u8 = 0x09;
pub const OP_STLOC_0: u8 = 0x0A;
pub const OP_STLOC_3: u8 = 0x0D;
pub const OP_LDLOC_S: u8 = 0x11;
pub const OP_STLOC_S: u8 = 0x13;
pub const OP_LDC_I4_0: u8 = 0x16;
pub const OP_LDC_I4_8: u8 = 0x1E;
pub const OP_LDC_I4_S: u8 = 0x1F;
pub const OP_LDC_I4: u8 = 0x20;
pub const OP_RET: u8 = 0x2A;
pub const OP_BR_S: u8 = 0x2B;
pub const OP_BRFALSE_S: u8 = 0x2C;
pub const OP_BRTRUE_S: u8 = 0x2D;
pub const OP_ADD: u8 = 0x58;
pub const OP_SUB: u8 = 0x59;
pub const OP_MUL: u8 = 0x5A;
pub const OP_DIV: u8 = 0x5B;
pub const OP_PREFIX_FE: u8 = 0xFE;

/// Second byte of two-byte comparison opcodes.
pub const CEQ_BYTE: u8 = 0x01;
pub const CGT_BYTE: u8 = 0x02;
pub const CLT_BYTE: u8 = 0x04;

// ===========================================================================
// Trace type
// ===========================================================================

/// A record of one CLR instruction's execution.
///
/// Stack and locals use `Option<i32>` to represent nullable values --
/// `None` corresponds to the CLR's `null` reference.
#[derive(Debug, Clone)]
pub struct CLRTrace {
    pub pc: usize,
    pub opcode: String,
    pub stack_before: Vec<Option<i32>>,
    pub stack_after: Vec<Option<i32>>,
    pub locals_snapshot: Vec<Option<i32>>,
    pub description: String,
}

// ===========================================================================
// Simulator
// ===========================================================================

/// The CLR simulator -- a type-inferring stack-based virtual machine.
pub struct CLRSimulator {
    pub stack: Vec<Option<i32>>,
    pub locals: Vec<Option<i32>>,
    pub pc: usize,
    pub bytecode: Vec<u8>,
    pub halted: bool,
}

impl CLRSimulator {
    pub fn new() -> Self {
        CLRSimulator {
            stack: Vec::new(),
            locals: vec![None; 16],
            pc: 0,
            bytecode: Vec::new(),
            halted: false,
        }
    }

    /// Load bytecode and configure local variable count.
    pub fn load(&mut self, bytecode: &[u8], num_locals: usize) {
        self.bytecode = bytecode.to_vec();
        self.stack.clear();
        self.locals = vec![None; num_locals];
        self.pc = 0;
        self.halted = false;
    }

    /// Execute one instruction and return its trace.
    pub fn step(&mut self) -> CLRTrace {
        assert!(!self.halted, "CLR simulator has halted");
        assert!(
            self.pc < self.bytecode.len(),
            "PC ({}) beyond bytecode length",
            self.pc
        );

        let pc = self.pc;
        let stack_before = self.stack.clone();
        let opcode_byte = self.bytecode[pc];

        // Two-byte opcode prefix.
        if opcode_byte == OP_PREFIX_FE {
            return self.execute_two_byte_opcode(stack_before);
        }

        if opcode_byte == OP_NOP {
            self.pc += 1;
            return CLRTrace {
                pc,
                opcode: "nop".to_string(),
                stack_before,
                stack_after: self.stack.clone(),
                locals_snapshot: self.locals.clone(),
                description: "no operation".to_string(),
            };
        }

        if opcode_byte == OP_LDNULL {
            self.stack.push(None);
            self.pc += 1;
            return CLRTrace {
                pc,
                opcode: "ldnull".to_string(),
                stack_before,
                stack_after: self.stack.clone(),
                locals_snapshot: self.locals.clone(),
                description: "push null".to_string(),
            };
        }

        // ldc.i4.N: push small integer constants 0-8.
        if opcode_byte >= OP_LDC_I4_0 && opcode_byte <= OP_LDC_I4_8 {
            let value = (opcode_byte - OP_LDC_I4_0) as i32;
            self.stack.push(Some(value));
            self.pc += 1;
            return CLRTrace {
                pc,
                opcode: format!("ldc.i4.{}", value),
                stack_before,
                stack_after: self.stack.clone(),
                locals_snapshot: self.locals.clone(),
                description: format!("push {}", value),
            };
        }

        if opcode_byte == OP_LDC_I4_S {
            let raw = self.bytecode[pc + 1] as i8;
            let val = raw as i32;
            self.stack.push(Some(val));
            self.pc += 2;
            return CLRTrace {
                pc,
                opcode: "ldc.i4.s".to_string(),
                stack_before,
                stack_after: self.stack.clone(),
                locals_snapshot: self.locals.clone(),
                description: format!("push {}", val),
            };
        }

        if opcode_byte == OP_LDC_I4 {
            let val = i32::from_le_bytes([
                self.bytecode[pc + 1],
                self.bytecode[pc + 2],
                self.bytecode[pc + 3],
                self.bytecode[pc + 4],
            ]);
            self.stack.push(Some(val));
            self.pc += 5;
            return CLRTrace {
                pc,
                opcode: "ldc.i4".to_string(),
                stack_before,
                stack_after: self.stack.clone(),
                locals_snapshot: self.locals.clone(),
                description: format!("push {}", val),
            };
        }

        // ldloc.N: push local variable 0-3.
        if opcode_byte >= OP_LDLOC_0 && opcode_byte <= OP_LDLOC_3 {
            let slot = (opcode_byte - OP_LDLOC_0) as usize;
            let val = self.locals[slot].expect(&format!("Local {} uninitialized", slot));
            self.stack.push(Some(val));
            self.pc += 1;
            return CLRTrace {
                pc,
                opcode: format!("ldloc.{}", slot),
                stack_before,
                stack_after: self.stack.clone(),
                locals_snapshot: self.locals.clone(),
                description: format!("push locals[{}] = {}", slot, val),
            };
        }

        // stloc.N: pop and store to local 0-3.
        if opcode_byte >= OP_STLOC_0 && opcode_byte <= OP_STLOC_3 {
            let slot = (opcode_byte - OP_STLOC_0) as usize;
            let val = self.stack.pop().expect("Stack underflow");
            self.locals[slot] = val;
            self.pc += 1;
            let desc = match val {
                Some(v) => format!("pop {}", v),
                None => "pop null".to_string(),
            };
            return CLRTrace {
                pc,
                opcode: format!("stloc.{}", slot),
                stack_before,
                stack_after: self.stack.clone(),
                locals_snapshot: self.locals.clone(),
                description: format!("{}, store in locals[{}]", desc, slot),
            };
        }

        if opcode_byte == OP_LDLOC_S {
            let slot = self.bytecode[pc + 1] as usize;
            let val = self.locals[slot].expect(&format!("Local {} uninitialized", slot));
            self.stack.push(Some(val));
            self.pc += 2;
            return CLRTrace {
                pc,
                opcode: "ldloc.s".to_string(),
                stack_before,
                stack_after: self.stack.clone(),
                locals_snapshot: self.locals.clone(),
                description: format!("push locals[{}] = {}", slot, val),
            };
        }

        if opcode_byte == OP_STLOC_S {
            let slot = self.bytecode[pc + 1] as usize;
            let val = self.stack.pop().expect("Stack underflow");
            self.locals[slot] = val;
            self.pc += 2;
            let desc = match val {
                Some(v) => format!("pop {}", v),
                None => "pop null".to_string(),
            };
            return CLRTrace {
                pc,
                opcode: "stloc.s".to_string(),
                stack_before,
                stack_after: self.stack.clone(),
                locals_snapshot: self.locals.clone(),
                description: format!("{}, store in locals[{}]", desc, slot),
            };
        }

        // Arithmetic operations.
        if opcode_byte == OP_ADD {
            return self.execute_arithmetic(stack_before, "add", |a, b| a.wrapping_add(b));
        }
        if opcode_byte == OP_SUB {
            return self.execute_arithmetic(stack_before, "sub", |a, b| a.wrapping_sub(b));
        }
        if opcode_byte == OP_MUL {
            return self.execute_arithmetic(stack_before, "mul", |a, b| a.wrapping_mul(b));
        }
        if opcode_byte == OP_DIV {
            let b = self.stack.last().expect("Stack underflow");
            let a_idx = self.stack.len() - 2;
            let a = self.stack[a_idx];
            assert!(b.is_some() && a.is_some(), "Math ops require valid ints");
            assert!(
                b.unwrap() != 0,
                "System.DivideByZeroException: division by zero"
            );
            let b_val = self.stack.pop().unwrap().unwrap();
            let a_val = self.stack.pop().unwrap().unwrap();
            let result = a_val.wrapping_div(b_val);
            self.stack.push(Some(result));
            self.pc += 1;
            return CLRTrace {
                pc,
                opcode: "div".to_string(),
                stack_before,
                stack_after: self.stack.clone(),
                locals_snapshot: self.locals.clone(),
                description: format!("pop {} and {}, push {}", b_val, a_val, result),
            };
        }

        if opcode_byte == OP_RET {
            self.pc += 1;
            self.halted = true;
            return CLRTrace {
                pc,
                opcode: "ret".to_string(),
                stack_before,
                stack_after: self.stack.clone(),
                locals_snapshot: self.locals.clone(),
                description: "return".to_string(),
            };
        }

        if opcode_byte == OP_BR_S {
            return self.execute_branch_s(stack_before, "br.s", true, false);
        }
        if opcode_byte == OP_BRFALSE_S {
            return self.execute_branch_s(stack_before, "brfalse.s", false, true);
        }
        if opcode_byte == OP_BRTRUE_S {
            return self.execute_branch_s(stack_before, "brtrue.s", false, false);
        }

        panic!(
            "Unknown CLR opcode: 0x{:02X} at PC={}",
            opcode_byte, pc
        );
    }

    fn execute_two_byte_opcode(&mut self, stack_before: Vec<Option<i32>>) -> CLRTrace {
        let pc = self.pc;
        let second_byte = self.bytecode[pc + 1];

        let b_opt = self.stack.pop().expect("Stack underflow");
        let a_opt = self.stack.pop().expect("Stack underflow");
        let b = b_opt.expect("Cannot compare nulls");
        let a = a_opt.expect("Cannot compare nulls");

        let (mnemonic, op_str, result) = match second_byte {
            CEQ_BYTE => ("ceq", "==", if a == b { 1 } else { 0 }),
            CGT_BYTE => ("cgt", ">", if a > b { 1 } else { 0 }),
            CLT_BYTE => ("clt", "<", if a < b { 1 } else { 0 }),
            _ => panic!("Unknown two-byte opcode: 0xFE 0x{:02X}", second_byte),
        };

        self.stack.push(Some(result));
        self.pc += 2;

        CLRTrace {
            pc,
            opcode: mnemonic.to_string(),
            stack_before,
            stack_after: self.stack.clone(),
            locals_snapshot: self.locals.clone(),
            description: format!(
                "pop {} and {}, push {} ({} {} {})",
                b, a, result, a, op_str, b
            ),
        }
    }

    fn execute_arithmetic<F>(
        &mut self,
        stack_before: Vec<Option<i32>>,
        mnemonic: &str,
        op: F,
    ) -> CLRTrace
    where
        F: Fn(i32, i32) -> i32,
    {
        let b_opt = self.stack.pop().expect("Stack underflow");
        let a_opt = self.stack.pop().expect("Stack underflow");
        let b = b_opt.expect("Math ops require valid ints");
        let a = a_opt.expect("Math ops require valid ints");
        let result = op(a, b);
        self.stack.push(Some(result));
        let save_pc = self.pc;
        self.pc += 1;
        CLRTrace {
            pc: save_pc,
            opcode: mnemonic.to_string(),
            stack_before,
            stack_after: self.stack.clone(),
            locals_snapshot: self.locals.clone(),
            description: format!("pop {} and {}, push {}", b, a, result),
        }
    }

    fn execute_branch_s(
        &mut self,
        stack_before: Vec<Option<i32>>,
        mnemonic: &str,
        always: bool,
        take_if_zero: bool,
    ) -> CLRTrace {
        let pc = self.pc;
        let raw = self.bytecode[pc + 1] as i8;
        let offset = raw as i32;
        let next_pc = pc + 2;
        let target = (next_pc as i32 + offset) as usize;

        if always {
            self.pc = target;
            return CLRTrace {
                pc,
                opcode: mnemonic.to_string(),
                stack_before,
                stack_after: self.stack.clone(),
                locals_snapshot: self.locals.clone(),
                description: format!("branch to PC={} (offset {})", target, offset),
            };
        }

        let val_opt = self.stack.pop().expect("Stack underflow");
        let val_conv = val_opt.unwrap_or(0);

        let should_branch = if take_if_zero {
            val_conv == 0
        } else {
            val_conv != 0
        };

        let desc_val = match val_opt {
            Some(v) => format!("{}", v),
            None => "null".to_string(),
        };

        if should_branch {
            self.pc = target;
            CLRTrace {
                pc,
                opcode: mnemonic.to_string(),
                stack_before,
                stack_after: self.stack.clone(),
                locals_snapshot: self.locals.clone(),
                description: format!("pop {}, branch taken to PC={}", desc_val, target),
            }
        } else {
            self.pc = next_pc;
            CLRTrace {
                pc,
                opcode: mnemonic.to_string(),
                stack_before,
                stack_after: self.stack.clone(),
                locals_snapshot: self.locals.clone(),
                description: format!("pop {}, branch not taken", desc_val),
            }
        }
    }

    /// Run until halt or max_steps reached.
    pub fn run(&mut self, max_steps: usize) -> Vec<CLRTrace> {
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

impl Default for CLRSimulator {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Encoding helpers
// ===========================================================================

/// Encode ldc.i4 with automatic compact form selection.
pub fn encode_ldc_i4(n: i32) -> Vec<u8> {
    if n >= 0 && n <= 8 {
        return vec![(OP_LDC_I4_0 as i32 + n) as u8];
    }
    if n >= -128 && n <= 127 {
        return vec![OP_LDC_I4_S, n as u8];
    }
    let mut res = vec![OP_LDC_I4];
    res.extend_from_slice(&(n as u32).to_le_bytes());
    res
}

/// Encode stloc with automatic compact form for slots 0-3.
pub fn encode_stloc(slot: u8) -> Vec<u8> {
    if slot <= 3 {
        return vec![OP_STLOC_0 + slot];
    }
    vec![OP_STLOC_S, slot]
}

/// Encode ldloc with automatic compact form for slots 0-3.
pub fn encode_ldloc(slot: u8) -> Vec<u8> {
    if slot <= 3 {
        return vec![OP_LDLOC_0 + slot];
    }
    vec![OP_LDLOC_S, slot]
}

/// Assemble a sequence of CLR bytecode fragments into flat bytecode.
pub fn assemble_clr(instructions: &[Vec<u8>]) -> Vec<u8> {
    instructions.iter().flatten().copied().collect()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test basic CLR math: x = 1 + 2 = 3.
    #[test]
    fn clr_simulator_math() {
        let mut sim = CLRSimulator::new();
        let prog = assemble_clr(&[
            encode_ldc_i4(1),
            encode_ldc_i4(2),
            vec![OP_ADD],
            encode_stloc(0),
            encode_ldloc(0),
            vec![OP_RET],
        ]);
        sim.load(&prog, 16);
        let traces = sim.run(100);
        assert_eq!(traces.len(), 6);
        assert_eq!(sim.locals[0], Some(3));
    }

    /// Division by zero should panic.
    #[test]
    #[should_panic(expected = "division by zero")]
    fn clr_div_by_zero() {
        let mut sim = CLRSimulator::new();
        let prog = assemble_clr(&[
            encode_ldc_i4(5),
            encode_ldc_i4(0),
            vec![OP_DIV],
        ]);
        sim.load(&prog, 16);
        sim.run(10);
    }

    /// Test two-byte comparison opcodes (ceq, cgt).
    #[test]
    fn clr_extended_opcodes() {
        let mut sim = CLRSimulator::new();
        let prog = assemble_clr(&[
            encode_ldc_i4(10),
            encode_ldc_i4(5),
            vec![OP_PREFIX_FE, CGT_BYTE], // 10 > 5 => push 1
            vec![OP_RET],
        ]);
        sim.load(&prog, 16);
        sim.run(10);
        assert_eq!(sim.stack[0], Some(1), "10 > 5 should push 1");
    }

    /// Test brfalse.s: branch when zero.
    #[test]
    fn clr_branching_zero() {
        let mut sim = CLRSimulator::new();
        let mut prog = assemble_clr(&[
            encode_ldc_i4(0),       // 1 byte
            vec![OP_BRFALSE_S, 2],  // 2 bytes, placeholder offset
            encode_ldc_i4(1000),    // 5 bytes (ldc.i4 with 4-byte payload)
            encode_ldc_i4(10),      // 1 byte (ldc.i4.s or compact)
            vec![OP_RET],
        ]);
        // Override offset to skip the 5-byte ldc.i4(1000) instruction.
        prog[2] = 5;

        sim.load(&prog, 16);
        let traces = sim.run(10);

        // Should have branched past ldc.i4(1000) and pushed 10 instead.
        let found_push10 = traces.iter().any(|trc| {
            trc.stack_after.iter().any(|v| *v == Some(10))
        });
        assert!(found_push10, "Should have pushed 10 after branching");
    }

    /// Halted simulator should panic on step.
    #[test]
    #[should_panic(expected = "CLR simulator has halted")]
    fn clr_halted_panics() {
        let mut sim = CLRSimulator::new();
        sim.halted = true;
        sim.step();
    }
}
