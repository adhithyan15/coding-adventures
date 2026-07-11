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
//! ## The stack value model ([`Value`])
//!
//! A CLR evaluation-stack slot holds either a 32-bit integer or an **object
//! reference**. We model that with [`Value`]: `Int(i32)` for a number, and
//! `Ref(Option<usize>)` for a reference — `Ref(None)` is the CLR `null`, and
//! `Ref(Some(i))` indexes the simulator's object [`heap`](CLRSimulator::heap).
//! A stack/local slot is `Option<Value>`, where the outer `None` means an
//! *uninitialised* local (distinct from a `null` reference value).
//!
//! This lets the simulator execute **reference types** — most importantly the
//! `System.Object[]` arrays the IIR→CIL backend uses for McCarthy cons cells
//! (`newarr` / `stelem.ref` / `ldelem.ref`), with value-type `box` / `unbox.any`
//! treated as identity in this loose model (the `Int` flows through), the same
//! way the wasm engine treats `i31` box/unbox. (LANG77 / McCarthy W6b.)
//!
//! ## Two-byte opcodes
//!
//! The CLR uses a prefix byte (0xFE) for extended opcodes like comparison
//! instructions (ceq, cgt, clt). This is different from the JVM where
//! all opcodes are single bytes.

use std::fmt;

// ===========================================================================
// Opcode constants
// ===========================================================================

pub const OP_NOP: u8 = 0x00;
pub const OP_LDNULL: u8 = 0x14;
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
pub const OP_DUP: u8 = 0x25;
/// `ldarg.0`..`ldarg.3` (0x02–0x05) — McCarthy W8b (lambda). Push method
/// parameter N onto the stack (the CLR counterpart of the JVM `aload`/`iload`
/// of a parameter slot and the wasm `local.get` of a function param).
pub const OP_LDARG_0: u8 = 0x02;
pub const OP_LDARG_3: u8 = 0x05;
/// `ldarg.s <idx>` (0x0E + u8) — McCarthy W8b. Push parameter `idx` (4–255).
pub const OP_LDARG_S: u8 = 0x0E;
/// `call <methodTok>` (0x28 + 4-byte token) — McCarthy W8b (lambda). Invoke
/// another method: pop its arguments, push a call frame, transfer control.
pub const OP_CALL: u8 = 0x28;
pub const OP_RET: u8 = 0x2A;
pub const OP_BR_S: u8 = 0x2B;
pub const OP_BRFALSE_S: u8 = 0x2C;
pub const OP_BRTRUE_S: u8 = 0x2D;
pub const OP_ADD: u8 = 0x58;
pub const OP_SUB: u8 = 0x59;
pub const OP_MUL: u8 = 0x5A;
pub const OP_DIV: u8 = 0x5B;
/// `xor` (0x61) — McCarthy W7 logical `not` lowers to `x ^ 1`.
pub const OP_XOR: u8 = 0x61;
/// `box <typeTok>` — McCarthy W6b. Boxes a value type into an object reference.
/// In this loose model the `Int` already roundtrips through `object[]`, so `box`
/// is identity (skips the 4-byte type token). Mirrors the wasm `i31` box no-op.
pub const OP_BOX: u8 = 0x8C;
/// `newarr <elemTypeTok>` — McCarthy W6b. Allocates a 1-D array; pops the length,
/// pushes an object reference (skips the 4-byte element-type token).
pub const OP_NEWARR: u8 = 0x8D;
/// `ldelem.ref` — McCarthy W6b. Pops `array, index`; pushes `array[index]`.
pub const OP_LDELEM_REF: u8 = 0xA2;
/// `stelem.ref` — McCarthy W6b. Pops `array, index, value`; stores into the array.
pub const OP_STELEM_REF: u8 = 0xA4;
/// `unbox.any <typeTok>` — McCarthy W6b. The dual of `box`; identity here (skips
/// the 4-byte type token).
pub const OP_UNBOX_ANY: u8 = 0xA5;
/// `isinst <typeTok>` (0x75) — McCarthy W7 `pair?`. Pops a reference; pushes it
/// back if it is an `object[]` (a cons cell), else pushes `null`. The CLR
/// counterpart of the JVM `instanceof` and the wasm `ref.test`.
pub const OP_ISINST: u8 = 0x75;
pub const OP_PREFIX_FE: u8 = 0xFE;

/// DoS guard: the maximum `newarr` length the simulator will allocate. A single
/// `newarr` is not bounded by the `run` step budget, so without a cap an
/// adversarial length could request tens of GB and OOM. 1M elements is far above
/// any real McCarthy program (a cons cell is length 2) yet bounds the allocation.
pub const MAX_ARRAY_LEN: usize = 1 << 20;

/// DoS guard: the maximum call-frame depth (McCarthy W8b). McCarthy lambda makes
/// `call` recursion possible; an adversarial or runaway program could recurse
/// unboundedly and exhaust the host stack. The `run` step budget already bounds
/// total work, but a depth cap turns deep recursion into a controlled panic
/// rather than a host-stack overflow. 10_000 frames is far beyond any real
/// McCarthy evaluation yet safely below the host stack limit.
pub const MAX_CALL_DEPTH: usize = 10_000;

// Two-byte opcode second bytes (after the 0xFE prefix).
pub const CEQ_BYTE: u8 = 0x01;
pub const CGT_BYTE: u8 = 0x02;
pub const CLT_BYTE: u8 = 0x04;

// ===========================================================================
// Stack value model
// ===========================================================================

/// A CLR evaluation-stack value: a 32-bit integer, or an object reference
/// (`None` = `null`, `Some(i)` = index into [`CLRSimulator::heap`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    Int(i32),
    Ref(Option<usize>),
}

impl Value {
    /// The integer payload, or panic if this is a reference. Arithmetic and
    /// comparison opcodes require integers.
    fn as_int(self) -> i32 {
        match self {
            Value::Int(n) => n,
            Value::Ref(_) => panic!("expected an int on the CLR stack, found a reference"),
        }
    }

    /// Reference-aware integer projection for **comparison** opcodes only
    /// (`ceq`/`cgt`/`clt`). Unlike [`as_int`], this does not panic on a
    /// reference: `null` ranks 0 and any heap reference ranks 1, matching their
    /// truthiness. This lets `pair?`/`is_null` compare a reference against
    /// `ldnull` without the strict arithmetic guard firing.
    fn as_cmp_int(self) -> i32 {
        match self {
            Value::Int(n) => n,
            Value::Ref(None) => 0,
            Value::Ref(Some(_)) => 1,
        }
    }

    /// CLR truthiness for `brtrue`/`brfalse`: an int is true iff non-zero; a
    /// reference is true iff non-null.
    fn is_truthy(self) -> bool {
        match self {
            Value::Int(n) => n != 0,
            Value::Ref(r) => r.is_some(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{n}"),
            Value::Ref(None) => write!(f, "null"),
            Value::Ref(Some(i)) => write!(f, "obj#{i}"),
        }
    }
}

// ===========================================================================
// Trace type
// ===========================================================================

/// A record of one CLR instruction's execution.
#[derive(Debug, Clone)]
pub struct CLRTrace {
    pub pc: usize,
    pub opcode: String,
    pub stack_before: Vec<Option<Value>>,
    pub stack_after: Vec<Option<Value>>,
    pub locals_snapshot: Vec<Option<Value>>,
    pub description: String,
}

// ===========================================================================
// Simulator
// ===========================================================================

/// The CLR simulator -- a type-inferring stack-based virtual machine.
/// One method in the program's method table (McCarthy W8b). Indexed by its
/// `MethodDef` ordinal: the `call <0x0600_00NN>` token resolves to
/// `methods[NN - 1]`.
#[derive(Clone, Debug)]
pub struct MethodCode {
    pub body: Vec<u8>,
    pub num_locals: usize,
    pub num_args: usize,
}

/// A saved caller context, pushed by `call` and restored by `ret` (W8b). The
/// operand `stack` and `heap` are shared across frames (CIL passes args + the
/// return value on the shared operand stack); only the per-method registers
/// (`pc`, `bytecode`, `locals`, `args`, `cur_method`) are saved/restored.
struct Frame {
    return_pc: usize,
    return_method: usize,
    return_bytecode: Vec<u8>,
    return_locals: Vec<Option<Value>>,
    return_args: Vec<Option<Value>>,
}

pub struct CLRSimulator {
    pub stack: Vec<Option<Value>>,
    pub locals: Vec<Option<Value>>,
    /// The object heap: each entry is one allocated array (`object[]`). A
    /// `Value::Ref(Some(i))` references `heap[i]`.
    pub heap: Vec<Vec<Value>>,
    pub pc: usize,
    pub bytecode: Vec<u8>,
    pub halted: bool,
    /// Method parameters of the currently-executing method (W8b). `ldarg.N`
    /// reads `args[N]`; populated by `call`, restored by `ret`.
    pub args: Vec<Option<Value>>,
    /// The whole program's method table, indexed by `MethodDef` ordinal − 1
    /// (W8b). Empty/single-entry for legacy single-method programs.
    methods: Vec<MethodCode>,
    /// The index (into `methods`) of the currently-executing method (W8b).
    cur_method: usize,
    /// The call stack of saved caller contexts (W8b).
    frames: Vec<Frame>,
}

impl CLRSimulator {
    pub fn new() -> Self {
        CLRSimulator {
            stack: Vec::new(),
            locals: vec![None; 16],
            heap: Vec::new(),
            pc: 0,
            bytecode: Vec::new(),
            halted: false,
            args: Vec::new(),
            methods: Vec::new(),
            cur_method: 0,
            frames: Vec::new(),
        }
    }

    /// Load a single method's bytecode and configure local variable count. This
    /// is the legacy single-method entry point (no `call`s); it registers a
    /// one-method table so `ret` halts cleanly.
    pub fn load(&mut self, bytecode: &[u8], num_locals: usize) {
        self.load_program(
            vec![MethodCode { body: bytecode.to_vec(), num_locals, num_args: 0 }],
            0,
        );
    }

    /// Load a whole program (method table) and start executing at `entry`
    /// (McCarthy W8b — lambda). `call <0x0600_00NN>` dispatches to
    /// `methods[NN − 1]`; `ret` returns to the caller (or halts at the entry).
    pub fn load_program(&mut self, methods: Vec<MethodCode>, entry: usize) {
        assert!(entry < methods.len(), "entry method index out of range");
        self.stack.clear();
        self.heap.clear();
        self.frames.clear();
        self.cur_method = entry;
        self.bytecode = methods[entry].body.clone();
        self.locals = vec![None; methods[entry].num_locals];
        self.args = vec![None; methods[entry].num_args];
        self.methods = methods;
        self.pc = 0;
        self.halted = false;
    }

    fn pop(&mut self) -> Option<Value> {
        self.stack.pop().expect("Stack underflow")
    }

    fn pop_int(&mut self) -> i32 {
        self.pop().expect("Cannot operate on null").as_int()
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
            return self.trace(pc, "nop", stack_before, "no operation".to_string());
        }

        // ── McCarthy W8b (lambda): method parameters ──
        // ldarg.0..3 (0x02–0x05): push the method parameter at that index.
        if (OP_LDARG_0..=OP_LDARG_3).contains(&opcode_byte) {
            let idx = (opcode_byte - OP_LDARG_0) as usize;
            let val = self.args.get(idx).copied().flatten()
                .expect("ldarg: parameter index out of range or uninitialised");
            self.stack.push(Some(val));
            self.pc += 1;
            return self.trace(pc, &format!("ldarg.{idx}"), stack_before, format!("push arg[{idx}] = {val}"));
        }
        if opcode_byte == OP_LDARG_S {
            let idx = self.bytecode[pc + 1] as usize;
            let val = self.args.get(idx).copied().flatten()
                .expect("ldarg.s: parameter index out of range or uninitialised");
            self.stack.push(Some(val));
            self.pc += 2;
            return self.trace(pc, "ldarg.s", stack_before, format!("push arg[{idx}] = {val}"));
        }

        if opcode_byte == OP_LDNULL {
            self.stack.push(Some(Value::Ref(None)));
            self.pc += 1;
            return self.trace(pc, "ldnull", stack_before, "push null".to_string());
        }

        if opcode_byte == OP_DUP {
            let top = *self.stack.last().expect("Stack underflow on dup");
            self.stack.push(top);
            self.pc += 1;
            return self.trace(pc, "dup", stack_before, "duplicate top of stack".to_string());
        }

        // ldc.i4.N: push small integer constants 0-8.
        if (OP_LDC_I4_0..=OP_LDC_I4_8).contains(&opcode_byte) {
            let value = (opcode_byte - OP_LDC_I4_0) as i32;
            self.stack.push(Some(Value::Int(value)));
            self.pc += 1;
            return self.trace(pc, &format!("ldc.i4.{value}"), stack_before, format!("push {value}"));
        }

        if opcode_byte == OP_LDC_I4_S {
            let val = self.bytecode[pc + 1] as i8 as i32;
            self.stack.push(Some(Value::Int(val)));
            self.pc += 2;
            return self.trace(pc, "ldc.i4.s", stack_before, format!("push {val}"));
        }

        if opcode_byte == OP_LDC_I4 {
            let val = i32::from_le_bytes([
                self.bytecode[pc + 1],
                self.bytecode[pc + 2],
                self.bytecode[pc + 3],
                self.bytecode[pc + 4],
            ]);
            self.stack.push(Some(Value::Int(val)));
            self.pc += 5;
            return self.trace(pc, "ldc.i4", stack_before, format!("push {val}"));
        }

        // ldloc.N: push local variable 0-3.
        if (OP_LDLOC_0..=OP_LDLOC_3).contains(&opcode_byte) {
            let slot = (opcode_byte - OP_LDLOC_0) as usize;
            return self.do_ldloc(pc, slot, 1, stack_before);
        }
        if opcode_byte == OP_LDLOC_S {
            let slot = self.bytecode[pc + 1] as usize;
            return self.do_ldloc(pc, slot, 2, stack_before);
        }

        // stloc.N: pop and store to local 0-3.
        if (OP_STLOC_0..=OP_STLOC_3).contains(&opcode_byte) {
            let slot = (opcode_byte - OP_STLOC_0) as usize;
            return self.do_stloc(pc, slot, 1, stack_before);
        }
        if opcode_byte == OP_STLOC_S {
            let slot = self.bytecode[pc + 1] as usize;
            return self.do_stloc(pc, slot, 2, stack_before);
        }

        // ── McCarthy W6b: reference types ──
        if opcode_byte == OP_NEWARR {
            // newarr <elemTypeTok>: pop length, allocate object[length] of nulls.
            // DoS guard: a single `newarr` is not step-bounded, so an adversarial
            // length (up to i32::MAX × 16 bytes ≈ 32 GB) could OOM. Cap it — a
            // controlled panic beats an abort. McCarthy cons cells are length 2.
            let len = self.pop_int().max(0) as usize;
            assert!(
                len <= MAX_ARRAY_LEN,
                "newarr length {len} exceeds the simulator cap {MAX_ARRAY_LEN}"
            );
            let idx = self.heap.len();
            self.heap.push(vec![Value::Ref(None); len]);
            self.stack.push(Some(Value::Ref(Some(idx))));
            self.pc += 5; // opcode + 4-byte type token
            return self.trace(pc, "newarr", stack_before, format!("alloc object[{len}] → obj#{idx}"));
        }
        if opcode_byte == OP_STELEM_REF {
            // stelem.ref: pop value, index, array; array[index] = value.
            let value = self.pop().expect("stelem.ref value");
            let index = self.pop_int();
            let arr = self.pop().expect("stelem.ref array");
            self.heap_array_mut(arr)[index as usize] = value;
            self.pc += 1;
            return self.trace(pc, "stelem.ref", stack_before, format!("store {value} at [{index}]"));
        }
        if opcode_byte == OP_LDELEM_REF {
            // ldelem.ref: pop array, index; push array[index].
            let index = self.pop_int();
            let arr = self.pop().expect("ldelem.ref array");
            let val = self.heap_array(arr)[index as usize];
            self.stack.push(Some(val));
            self.pc += 1;
            return self.trace(pc, "ldelem.ref", stack_before, format!("load [{index}] = {val}"));
        }
        if opcode_byte == OP_BOX {
            // box <typeTok>: identity in the loose model (the Int roundtrips).
            self.pc += 5; // opcode + 4-byte type token
            return self.trace(pc, "box", stack_before, "box (identity)".to_string());
        }
        if opcode_byte == OP_UNBOX_ANY {
            // unbox.any <typeTok>: identity in the loose model.
            self.pc += 5; // opcode + 4-byte type token
            return self.trace(pc, "unbox.any", stack_before, "unbox.any (identity)".to_string());
        }
        if opcode_byte == OP_ISINST {
            // isinst <typeTok>: the McCarthy `pair?` type test. A cons cell is a
            // heap `object[]` (`Ref(Some)`); an atom is a boxed int (`Int`); nil
            // is `Ref(None)`. So "is this an object[]?" ≡ "is it a heap ref?":
            // keep `Ref(Some)`, otherwise push `null`. The token is ignored (the
            // loose model has exactly one reference kind — the cons array).
            let value = self.pop().expect("isinst operand");
            let result = match value {
                Value::Ref(Some(i)) => Value::Ref(Some(i)),
                _ => Value::Ref(None),
            };
            self.stack.push(Some(result));
            self.pc += 5; // opcode + 4-byte type token
            return self.trace(pc, "isinst", stack_before, format!("{value} isinst object[] → {result}"));
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
        if opcode_byte == OP_XOR {
            return self.execute_arithmetic(stack_before, "xor", |a, b| a ^ b);
        }
        if opcode_byte == OP_DIV {
            let b_val = self.pop_int();
            assert!(b_val != 0, "System.DivideByZeroException: division by zero");
            let a_val = self.pop_int();
            let result = a_val.wrapping_div(b_val);
            self.stack.push(Some(Value::Int(result)));
            self.pc += 1;
            return self.trace(pc, "div", stack_before, format!("pop {b_val} and {a_val}, push {result}"));
        }

        // ── call <methodTok> (0x28) — McCarthy W8b (lambda) ──
        // The 4-byte token is a MethodDef: 0x0600_00NN → methods[NN − 1]. Pop the
        // callee's N args off the (shared) operand stack — the LAST pushed is the
        // LAST arg — into a fresh `args` vector, save the caller's registers, and
        // transfer control to the callee. The return value comes back on the
        // shared stack at `ret`.
        if opcode_byte == OP_CALL {
            let token = u32::from_le_bytes([
                self.bytecode[pc + 1], self.bytecode[pc + 2],
                self.bytecode[pc + 3], self.bytecode[pc + 4],
            ]);
            // MethodDef tables are 0x06 in the high byte; ordinal is 1-based.
            let ordinal = (token & 0x00FF_FFFF) as usize;
            assert!(ordinal >= 1, "call: invalid MethodDef token 0x{token:08X}");
            let callee_idx = ordinal - 1;
            let callee = self.methods.get(callee_idx)
                .unwrap_or_else(|| panic!("call: no method for token 0x{token:08X}"))
                .clone();
            // DoS guard: bound recursion depth (turns runaway recursion into a
            // controlled panic instead of a host-stack overflow).
            assert!(
                self.frames.len() < MAX_CALL_DEPTH,
                "call depth exceeded the simulator cap {MAX_CALL_DEPTH} (runaway recursion?)"
            );
            // Pop args (in order: arg[0] was pushed first, so pop into reverse).
            let mut callee_args = vec![None; callee.num_args];
            for slot in callee_args.iter_mut().rev() {
                *slot = Some(self.pop().expect("call: not enough arguments on the stack"));
            }
            // Save the caller's context; `pc + 5` is the return address.
            self.frames.push(Frame {
                return_pc: pc + 5,
                return_method: self.cur_method,
                return_bytecode: std::mem::take(&mut self.bytecode),
                return_locals: std::mem::take(&mut self.locals),
                return_args: std::mem::take(&mut self.args),
            });
            // Enter the callee.
            self.cur_method = callee_idx;
            self.bytecode = callee.body;
            self.locals = vec![None; callee.num_locals];
            self.args = callee_args;
            self.pc = 0;
            return self.trace(pc, "call", stack_before, format!("call method #{callee_idx} (token 0x{token:08X})"));
        }

        if opcode_byte == OP_RET {
            // The return value (if any) is left on the shared operand stack. If
            // there is a caller frame, restore it and continue; otherwise this is
            // the entry method returning → halt.
            if let Some(frame) = self.frames.pop() {
                self.cur_method = frame.return_method;
                self.bytecode = frame.return_bytecode;
                self.locals = frame.return_locals;
                self.args = frame.return_args;
                self.pc = frame.return_pc;
                return self.trace(pc, "ret", stack_before, "return to caller".to_string());
            }
            self.pc += 1;
            self.halted = true;
            return self.trace(pc, "ret", stack_before, "return (halt)".to_string());
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

        panic!("Unknown CLR opcode: 0x{:02X} at PC={}", opcode_byte, pc);
    }

    /// Build a trace for the current step (the `stack_after`/`locals_snapshot`
    /// are captured from the post-mutation simulator state).
    fn trace(&self, pc: usize, opcode: &str, stack_before: Vec<Option<Value>>, description: String) -> CLRTrace {
        CLRTrace {
            pc,
            opcode: opcode.to_string(),
            stack_before,
            stack_after: self.stack.clone(),
            locals_snapshot: self.locals.clone(),
            description,
        }
    }

    /// Resolve a reference to a heap array (shared).
    fn heap_array(&self, r: Value) -> &Vec<Value> {
        match r {
            Value::Ref(Some(i)) => &self.heap[i],
            Value::Ref(None) => panic!("System.NullReferenceException"),
            Value::Int(_) => panic!("expected an array reference, found an int"),
        }
    }

    /// Resolve a reference to a heap array (mutable).
    fn heap_array_mut(&mut self, r: Value) -> &mut Vec<Value> {
        match r {
            Value::Ref(Some(i)) => &mut self.heap[i],
            Value::Ref(None) => panic!("System.NullReferenceException"),
            Value::Int(_) => panic!("expected an array reference, found an int"),
        }
    }

    fn do_ldloc(&mut self, pc: usize, slot: usize, width: usize, stack_before: Vec<Option<Value>>) -> CLRTrace {
        let val = self.locals[slot].unwrap_or_else(|| panic!("Local {slot} uninitialized"));
        self.stack.push(Some(val));
        self.pc += width;
        self.trace(pc, &format!("ldloc.{slot}"), stack_before, format!("push locals[{slot}] = {val}"))
    }

    fn do_stloc(&mut self, pc: usize, slot: usize, width: usize, stack_before: Vec<Option<Value>>) -> CLRTrace {
        let val = self.pop();
        self.locals[slot] = val;
        self.pc += width;
        let desc = match val {
            Some(v) => format!("pop {v}"),
            None => "pop (empty)".to_string(),
        };
        self.trace(pc, &format!("stloc.{slot}"), stack_before, format!("{desc}, store in locals[{slot}]"))
    }

    fn execute_two_byte_opcode(&mut self, stack_before: Vec<Option<Value>>) -> CLRTrace {
        let pc = self.pc;
        let second_byte = self.bytecode[pc + 1];
        // Comparisons are reference-aware: McCarthy `pair?`/`is_null` compare a
        // reference against `null` (`isinst …; ldnull; ceq`). Map a reference to
        // its truthiness rank for the compare — `null` → 0, a cons ref → 1 — so
        // `ceq` against `ldnull` (0) answers "is it null?" without panicking.
        // Atoms remain their integer value (this never collides: an atom that
        // happens to be 0/1 is only ever compared with `equal?`, which unboxes
        // both sides to genuine ints first).
        let b = self.pop().expect("Cannot compare null").as_cmp_int();
        let a = self.pop().expect("Cannot compare null").as_cmp_int();
        let (mnemonic, op_str, result) = match second_byte {
            CEQ_BYTE => ("ceq", "==", if a == b { 1 } else { 0 }),
            CGT_BYTE => ("cgt", ">", if a > b { 1 } else { 0 }),
            CLT_BYTE => ("clt", "<", if a < b { 1 } else { 0 }),
            _ => panic!("Unknown two-byte opcode: 0xFE 0x{:02X}", second_byte),
        };
        self.stack.push(Some(Value::Int(result)));
        self.pc += 2;
        self.trace(pc, mnemonic, stack_before, format!("pop {b} and {a}, push {result} ({a} {op_str} {b})"))
    }

    fn execute_arithmetic<F>(&mut self, stack_before: Vec<Option<Value>>, mnemonic: &str, op: F) -> CLRTrace
    where
        F: Fn(i32, i32) -> i32,
    {
        let b = self.pop_int();
        let a = self.pop_int();
        let result = op(a, b);
        self.stack.push(Some(Value::Int(result)));
        let pc = self.pc;
        self.pc += 1;
        self.trace(pc, mnemonic, stack_before, format!("pop {b} and {a}, push {result}"))
    }

    fn execute_branch_s(
        &mut self,
        stack_before: Vec<Option<Value>>,
        mnemonic: &str,
        always: bool,
        take_if_zero: bool,
    ) -> CLRTrace {
        let pc = self.pc;
        let raw = self.bytecode[pc + 1] as i8;
        let next_pc = pc + 2;
        let target = (next_pc as i32 + raw as i32) as usize;

        if always {
            self.pc = target;
            return self.trace(pc, mnemonic, stack_before, format!("branch to PC={target} (offset {})", raw as i32));
        }

        let val = self.pop().expect("Stack underflow on branch");
        let truthy = val.is_truthy();
        let should_branch = if take_if_zero { !truthy } else { truthy };
        let desc_val = format!("{val}");

        if should_branch {
            self.pc = target;
            self.trace(pc, mnemonic, stack_before, format!("pop {desc_val}, branch taken to PC={target}"))
        } else {
            self.pc = next_pc;
            self.trace(pc, mnemonic, stack_before, format!("pop {desc_val}, branch not taken"))
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
    if (0..=8).contains(&n) {
        return vec![(OP_LDC_I4_0 as i32 + n) as u8];
    }
    if (-128..=127).contains(&n) {
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
        assert_eq!(sim.locals[0], Some(Value::Int(3)));
    }

    /// Division by zero should panic.
    #[test]
    #[should_panic(expected = "division by zero")]
    fn clr_div_by_zero() {
        let mut sim = CLRSimulator::new();
        let prog = assemble_clr(&[encode_ldc_i4(5), encode_ldc_i4(0), vec![OP_DIV]]);
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
        assert_eq!(sim.stack[0], Some(Value::Int(1)), "10 > 5 should push 1");
    }

    /// Test brfalse.s: branch when zero.
    #[test]
    fn clr_branching_zero() {
        let mut sim = CLRSimulator::new();
        let mut prog = assemble_clr(&[
            encode_ldc_i4(0),      // 1 byte
            vec![OP_BRFALSE_S, 2], // 2 bytes, placeholder offset
            encode_ldc_i4(1000),   // 5 bytes (ldc.i4 with 4-byte payload)
            encode_ldc_i4(10),     // 1 byte
            vec![OP_RET],
        ]);
        prog[2] = 5; // skip the 5-byte ldc.i4(1000)
        sim.load(&prog, 16);
        let traces = sim.run(10);
        let found_push10 = traces
            .iter()
            .any(|trc| trc.stack_after.contains(&Some(Value::Int(10))));
        assert!(found_push10, "Should have pushed 10 after branching");
    }

    /// McCarthy W6b: a `System.Object[]` cons cell — newarr + stelem.ref +
    /// ldelem.ref, with box/unbox.any identity. Build `[7, 9]`, read back `7`.
    #[test]
    fn clr_object_array_cons_roundtrip() {
        let mut sim = CLRSimulator::new();
        let tok = [0u8; 4]; // type token (ignored by the simulator)
        let prog = assemble_clr(&[
            encode_ldc_i4(2),
            vec![OP_NEWARR], tok.to_vec(),     // arr = new object[2]
            encode_stloc(0),
            // arr[0] = box 7
            encode_ldloc(0), encode_ldc_i4(0), encode_ldc_i4(7), vec![OP_BOX], tok.to_vec(), vec![OP_STELEM_REF],
            // arr[1] = box 9
            encode_ldloc(0), encode_ldc_i4(1), encode_ldc_i4(9), vec![OP_BOX], tok.to_vec(), vec![OP_STELEM_REF],
            // return unbox.any arr[0]
            encode_ldloc(0), encode_ldc_i4(0), vec![OP_LDELEM_REF], vec![OP_UNBOX_ANY], tok.to_vec(),
            vec![OP_RET],
        ]);
        sim.load(&prog, 16);
        sim.run(100);
        assert_eq!(sim.stack.last(), Some(&Some(Value::Int(7))), "arr[0] should be 7");
    }

    /// `ldnull` pushes a null reference, and `brfalse.s` treats it as falsy.
    #[test]
    fn clr_null_is_falsy() {
        let mut sim = CLRSimulator::new();
        sim.load(&[OP_LDNULL], 0);
        sim.step();
        assert_eq!(sim.stack[0], Some(Value::Ref(None)), "ldnull pushes a null ref");
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
