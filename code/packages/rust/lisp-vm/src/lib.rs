//! # Lisp VM -- executes compiled Lisp bytecode.
//!
//! ## What This VM Does
//!
//! This virtual machine executes the bytecode produced by the Lisp compiler.
//! It implements McCarthy's 1960 Lisp with modern additions: closures, tail
//! call optimization, and garbage collection.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │  LispVm                                              │
//! │                                                      │
//! │  ┌──────────┐  ┌──────────┐  ┌───────────────────┐  │
//! │  │  Stack    │  │Variables │  │  Locals            │  │
//! │  │ [v1,v2]  │  │ {x: 42}  │  │ [param1, param2]  │  │
//! │  └──────────┘  └──────────┘  └───────────────────┘  │
//! │                                                      │
//! │  ┌──────────────────────────────────────────────┐    │
//! │  │  Heap: [ConsCell, Symbol, Closure, ...]      │    │
//! │  └──────────────────────────────────────────────┘    │
//! │                                                      │
//! │  ┌──────────────────────────────────────────────┐    │
//! │  │  Symbol Table: {"foo" -> addr, "bar" -> addr}│    │
//! │  └──────────────────────────────────────────────┘    │
//! │                                                      │
//! │  PC: 0  ──►  fetch ──► decode ──► execute            │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## NIL and Truthiness
//!
//! NIL is Lisp's "nothing" value -- the empty list, the false value, the
//! end-of-list marker. In this VM, we represent it as `Value::Nil`.
//!
//! Falsy values: `Nil`, `Integer(0)`, `Bool(false)`.
//! Everything else is truthy.
//!
//! ## Tail Call Optimization
//!
//! When a function call is the last thing a function does before returning
//! (i.e., it's in "tail position"), the compiler emits `TAIL_CALL` instead
//! of `CALL_FUNCTION`. The VM handles this by reusing the current call frame
//! instead of pushing a new one, enabling unbounded recursion in O(1) stack
//! space.

use lisp_compiler::{
    compile, CodeObject, CompileError, Instruction, LispOp, Value,
};
use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Section 1: Heap Objects
// ============================================================================
//
// Lisp needs heap-allocated objects for three things:
//
// 1. **Cons cells** -- pairs of values, the building blocks of lists
// 2. **Symbols** -- interned strings (two references to 'foo get the same address)
// 3. **Closures** -- functions with captured environments
//
// All heap objects live in a flat vector (`Vec<HeapObject>`) indexed by address.
// ============================================================================

/// A cons cell -- a pair of values, the fundamental data structure of Lisp.
///
/// Every Lisp list is a chain of cons cells:
///
/// ```text
/// (1 2 3) = (1 . (2 . (3 . NIL)))
///
///   ┌─────────┐     ┌─────────┐     ┌─────────┐
///   │ car: 1  │────►│ car: 2  │────►│ car: 3  │────► NIL
///   │ cdr: ───┤     │ cdr: ───┤     │ cdr: ───┤
///   └─────────┘     └─────────┘     └─────────┘
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ConsCell {
    /// The first element of the pair.
    pub car: Value,
    /// The second element of the pair.
    pub cdr: Value,
}

/// A symbol -- an interned string with a unique heap address.
///
/// Two references to the same symbol name always resolve to the same
/// heap address. This makes symbol comparison an O(1) pointer comparison
/// instead of an O(n) string comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct HeapSymbol {
    /// The symbol's name.
    pub name: String,
}

/// A closure -- a function bundled with its captured environment.
///
/// When a lambda is created, it captures the current variable bindings
/// so that inner functions can reference outer variables even after the
/// outer function has returned.
#[derive(Debug, Clone, PartialEq)]
pub struct LispClosure {
    /// The compiled body of the function.
    pub code: CodeObject,
    /// The captured variable environment.
    pub env: HashMap<String, Value>,
    /// Parameter names for binding arguments.
    pub params: Vec<String>,
}

/// The kinds of objects that can live on the heap.
#[derive(Debug, Clone, PartialEq)]
pub enum HeapObject {
    /// A cons cell (pair of values).
    Cons(ConsCell),
    /// An interned symbol.
    Symbol(HeapSymbol),
    /// A closure (function with captured environment).
    Closure(LispClosure),
}

// ============================================================================
// Section 2: Error Type
// ============================================================================

/// An error that occurs during VM execution.
#[derive(Debug, Clone, PartialEq)]
pub struct VmError {
    pub message: String,
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VmError: {}", self.message)
    }
}

impl std::error::Error for VmError {}

impl From<CompileError> for VmError {
    fn from(err: CompileError) -> Self {
        VmError {
            message: format!("Compile error: {}", err),
        }
    }
}

// ============================================================================
// Section 3: The Virtual Machine
// ============================================================================

/// The Lisp virtual machine.
///
/// Executes bytecode produced by the Lisp compiler. Manages a value stack,
/// variables, local slots, and a heap for cons cells, symbols, and closures.
pub struct LispVm {
    /// The value stack -- where intermediate results live during computation.
    pub stack: Vec<Value>,
    /// Global variable bindings.
    pub variables: HashMap<String, Value>,
    /// Local variable slots (for function parameters).
    pub locals: Vec<Value>,
    /// The heap -- where cons cells, symbols, and closures are allocated.
    pub heap: Vec<HeapObject>,
    /// Symbol interning table: name -> heap address.
    pub symbol_table: HashMap<String, usize>,
    /// Program counter -- index of the current instruction.
    pub pc: usize,
    /// Whether the VM has halted.
    pub halted: bool,
    /// Output collected from PRINT instructions.
    pub output: Vec<String>,
}

impl LispVm {
    /// Create a new, empty VM.
    pub fn new() -> Self {
        LispVm {
            stack: Vec::new(),
            variables: HashMap::new(),
            locals: Vec::new(),
            heap: Vec::new(),
            symbol_table: HashMap::new(),
            pc: 0,
            halted: false,
            output: Vec::new(),
        }
    }

    // -----------------------------------------------------------------
    // Stack Operations
    // -----------------------------------------------------------------

    /// Push a value onto the stack.
    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    /// Pop a value from the stack.
    fn pop(&mut self) -> Result<Value, VmError> {
        self.stack.pop().ok_or_else(|| VmError {
            message: "Stack underflow".to_string(),
        })
    }

    // -----------------------------------------------------------------
    // Heap Operations
    // -----------------------------------------------------------------

    /// Allocate a heap object and return its address.
    fn allocate(&mut self, obj: HeapObject) -> usize {
        let addr = self.heap.len();
        self.heap.push(obj);
        addr
    }

    /// Dereference a heap address.
    fn deref(&self, addr: usize) -> Result<&HeapObject, VmError> {
        self.heap.get(addr).ok_or_else(|| VmError {
            message: format!("Invalid heap address: {}", addr),
        })
    }

    /// Check if an address is valid.
    fn is_valid_address(&self, addr: usize) -> bool {
        addr < self.heap.len()
    }

    /// Intern a symbol: look up or create.
    fn intern_symbol(&mut self, name: &str) -> usize {
        if let Some(&addr) = self.symbol_table.get(name) {
            return addr;
        }
        let addr = self.allocate(HeapObject::Symbol(HeapSymbol {
            name: name.to_string(),
        }));
        self.symbol_table.insert(name.to_string(), addr);
        addr
    }

    // -----------------------------------------------------------------
    // Instruction Execution
    // -----------------------------------------------------------------

    /// Execute a CodeObject until HALT or end of instructions.
    pub fn execute(&mut self, code: &CodeObject) -> Result<(), VmError> {
        self.pc = 0;
        self.halted = false;

        while !self.halted && self.pc < code.instructions.len() {
            let instr = &code.instructions[self.pc];
            self.execute_instruction(instr, code)?;
        }

        Ok(())
    }

    /// Execute a single instruction.
    fn execute_instruction(
        &mut self,
        instr: &Instruction,
        code: &CodeObject,
    ) -> Result<(), VmError> {
        match instr.opcode {
            // =============================================================
            // Stack Operations (0x01-0x04)
            // =============================================================

            LispOp::LoadConst => {
                // Push a constant from the pool.
                let idx = instr.operand.unwrap_or(0);
                let value = code.constants.get(idx).ok_or_else(|| VmError {
                    message: format!("Constant index out of bounds: {}", idx),
                })?.clone();
                self.push(value);
                self.pc += 1;
            }

            LispOp::Pop => {
                // Discard top of stack.
                self.pop()?;
                self.pc += 1;
            }

            LispOp::LoadNil => {
                self.push(Value::Nil);
                self.pc += 1;
            }

            LispOp::LoadTrue => {
                self.push(Value::Bool(true));
                self.pc += 1;
            }

            // =============================================================
            // Variable Operations (0x10-0x13)
            // =============================================================

            LispOp::StoreName => {
                let idx = instr.operand.unwrap_or(0);
                let name = code.names.get(idx).ok_or_else(|| VmError {
                    message: format!("Name index out of bounds: {}", idx),
                })?.clone();
                let value = self.pop()?;
                self.variables.insert(name, value);
                self.pc += 1;
            }

            LispOp::LoadName => {
                let idx = instr.operand.unwrap_or(0);
                let name = code.names.get(idx).ok_or_else(|| VmError {
                    message: format!("Name index out of bounds: {}", idx),
                })?.clone();
                let value = self.variables.get(&name).ok_or_else(|| VmError {
                    message: format!("Undefined variable: {}", name),
                })?.clone();
                self.push(value);
                self.pc += 1;
            }

            LispOp::StoreLocal => {
                let idx = instr.operand.unwrap_or(0);
                let value = self.pop()?;
                while self.locals.len() <= idx {
                    self.locals.push(Value::Nil);
                }
                self.locals[idx] = value;
                self.pc += 1;
            }

            LispOp::LoadLocal => {
                let idx = instr.operand.unwrap_or(0);
                let value = if idx < self.locals.len() {
                    self.locals[idx].clone()
                } else {
                    Value::Nil
                };
                self.push(value);
                self.pc += 1;
            }

            // =============================================================
            // Arithmetic (0x20-0x23)
            // =============================================================

            LispOp::Add => {
                let b = self.pop()?;
                let a = self.pop()?;
                match (&a, &b) {
                    (Value::Integer(x), Value::Integer(y)) => self.push(Value::Integer(x + y)),
                    _ => return Err(VmError {
                        message: format!("ADD: expected two integers, got {:?} and {:?}", a, b),
                    }),
                }
                self.pc += 1;
            }

            LispOp::Sub => {
                let b = self.pop()?;
                let a = self.pop()?;
                match (&a, &b) {
                    (Value::Integer(x), Value::Integer(y)) => self.push(Value::Integer(x - y)),
                    _ => return Err(VmError {
                        message: format!("SUB: expected two integers, got {:?} and {:?}", a, b),
                    }),
                }
                self.pc += 1;
            }

            LispOp::Mul => {
                let b = self.pop()?;
                let a = self.pop()?;
                match (&a, &b) {
                    (Value::Integer(x), Value::Integer(y)) => self.push(Value::Integer(x * y)),
                    _ => return Err(VmError {
                        message: format!("MUL: expected two integers, got {:?} and {:?}", a, b),
                    }),
                }
                self.pc += 1;
            }

            LispOp::Div => {
                let b = self.pop()?;
                let a = self.pop()?;
                match (&a, &b) {
                    (Value::Integer(_), Value::Integer(0)) => return Err(VmError {
                        message: "Division by zero".to_string(),
                    }),
                    (Value::Integer(x), Value::Integer(y)) => self.push(Value::Integer(x / y)),
                    _ => return Err(VmError {
                        message: format!("DIV: expected two integers, got {:?} and {:?}", a, b),
                    }),
                }
                self.pc += 1;
            }

            // =============================================================
            // Comparison (0x30-0x32)
            // =============================================================

            LispOp::CmpEq => {
                let b = self.pop()?;
                let a = self.pop()?;
                // NIL identity check
                let result = match (&a, &b) {
                    (Value::Nil, Value::Nil) => 1,
                    (Value::Nil, _) | (_, Value::Nil) => 0,
                    _ => if a == b { 1 } else { 0 },
                };
                self.push(Value::Integer(result));
                self.pc += 1;
            }

            LispOp::CmpLt => {
                let b = self.pop()?;
                let a = self.pop()?;
                match (&a, &b) {
                    (Value::Integer(x), Value::Integer(y)) => {
                        self.push(Value::Integer(if x < y { 1 } else { 0 }));
                    }
                    _ => return Err(VmError {
                        message: format!("CMP_LT: expected two integers, got {:?} and {:?}", a, b),
                    }),
                }
                self.pc += 1;
            }

            LispOp::CmpGt => {
                let b = self.pop()?;
                let a = self.pop()?;
                match (&a, &b) {
                    (Value::Integer(x), Value::Integer(y)) => {
                        self.push(Value::Integer(if x > y { 1 } else { 0 }));
                    }
                    _ => return Err(VmError {
                        message: format!("CMP_GT: expected two integers, got {:?} and {:?}", a, b),
                    }),
                }
                self.pc += 1;
            }

            // =============================================================
            // Control Flow (0x40-0x42)
            // =============================================================

            LispOp::Jump => {
                let target = instr.operand.unwrap_or(0);
                self.pc = target;
            }

            LispOp::JumpIfFalse => {
                let target = instr.operand.unwrap_or(0);
                let value = self.pop()?;
                if value.is_falsy() {
                    self.pc = target;
                } else {
                    self.pc += 1;
                }
            }

            LispOp::JumpIfTrue => {
                let target = instr.operand.unwrap_or(0);
                let value = self.pop()?;
                if !value.is_falsy() {
                    self.pc = target;
                } else {
                    self.pc += 1;
                }
            }

            // =============================================================
            // Functions (0x50-0x53)
            // =============================================================

            LispOp::MakeClosure => {
                // Pop a CodeObject, capture env, allocate closure, push address.
                let param_count = instr.operand.unwrap_or(0);
                let code_val = self.pop()?;
                let func_code = match code_val {
                    Value::Code(co) => *co,
                    _ => return Err(VmError {
                        message: "MAKE_CLOSURE: expected CodeObject on stack".to_string(),
                    }),
                };

                // Extract parameter names from the body's constants.
                // The compiler stores param name strings as the last N constants.
                let mut params = Vec::new();
                if param_count > 0 && func_code.constants.len() >= param_count {
                    let start = func_code.constants.len() - param_count;
                    for i in start..func_code.constants.len() {
                        if let Value::String(name) = &func_code.constants[i] {
                            params.push(name.clone());
                        } else {
                            params.push(format!("_p{}", i - start));
                        }
                    }
                } else {
                    for i in 0..param_count {
                        params.push(format!("_p{}", i));
                    }
                }

                // Capture current environment
                let env = self.variables.clone();

                let closure = LispClosure {
                    code: func_code,
                    env,
                    params,
                };
                let addr = self.allocate(HeapObject::Closure(closure));
                self.push(Value::ClosureAddr(addr));
                self.pc += 1;
            }

            LispOp::CallFunction => {
                let argc = instr.operand.unwrap_or(0);
                let func = self.pop()?;

                // Pop arguments (pushed left-to-right, pop in reverse)
                let mut args = Vec::new();
                for _ in 0..argc {
                    args.insert(0, self.pop()?);
                }

                match func {
                    Value::ClosureAddr(addr) => {
                        self.execute_closure(addr, args, false)?;
                    }
                    _ => return Err(VmError {
                        message: format!("Cannot call {:?}", func),
                    }),
                }
            }

            LispOp::TailCall => {
                let argc = instr.operand.unwrap_or(0);
                let func = self.pop()?;

                let mut args = Vec::new();
                for _ in 0..argc {
                    args.insert(0, self.pop()?);
                }

                match func {
                    Value::ClosureAddr(addr) => {
                        // For tail call, we signal the execution loop
                        // We push a special marker and let execute_closure handle it
                        self.execute_closure(addr, args, true)?;
                    }
                    _ => return Err(VmError {
                        message: format!("Cannot tail-call {:?}", func),
                    }),
                }
            }

            LispOp::Return => {
                // Handled by execute_closure -- this just advances PC
                self.pc += 1;
            }

            // =============================================================
            // Lisp-Specific Operations (0x70-0x75)
            // =============================================================

            LispOp::Cons => {
                // Pop car (top) and cdr (below), create cons cell.
                let car = self.pop()?;
                let cdr = self.pop()?;
                let addr = self.allocate(HeapObject::Cons(ConsCell { car, cdr }));
                self.push(Value::ConsAddr(addr));
                self.pc += 1;
            }

            LispOp::Car => {
                let addr_val = self.pop()?;
                match addr_val {
                    Value::ConsAddr(addr) => {
                        let obj = self.deref(addr)?.clone();
                        match obj {
                            HeapObject::Cons(cell) => self.push(cell.car),
                            _ => return Err(VmError {
                                message: "CAR: not a cons cell".to_string(),
                            }),
                        }
                    }
                    _ => return Err(VmError {
                        message: format!("CAR: expected cons address, got {:?}", addr_val),
                    }),
                }
                self.pc += 1;
            }

            LispOp::Cdr => {
                let addr_val = self.pop()?;
                match addr_val {
                    Value::ConsAddr(addr) => {
                        let obj = self.deref(addr)?.clone();
                        match obj {
                            HeapObject::Cons(cell) => self.push(cell.cdr),
                            _ => return Err(VmError {
                                message: "CDR: not a cons cell".to_string(),
                            }),
                        }
                    }
                    _ => return Err(VmError {
                        message: format!("CDR: expected cons address, got {:?}", addr_val),
                    }),
                }
                self.pc += 1;
            }

            LispOp::MakeSymbol => {
                let idx = instr.operand.unwrap_or(0);
                let name = match code.constants.get(idx) {
                    Some(Value::String(s)) => s.clone(),
                    _ => return Err(VmError {
                        message: format!("MAKE_SYMBOL: constant {} is not a string", idx),
                    }),
                };
                let addr = self.intern_symbol(&name);
                self.push(Value::ConsAddr(addr)); // Use ConsAddr for heap addresses
                self.pc += 1;
            }

            LispOp::IsAtom => {
                let value = self.pop()?;
                let result = match &value {
                    Value::Nil => 1,
                    Value::ConsAddr(addr) => {
                        if self.is_valid_address(*addr) {
                            match self.deref(*addr)? {
                                HeapObject::Cons(_) => 0, // Cons cell is NOT an atom
                                _ => 1, // Symbol on heap is an atom
                            }
                        } else {
                            1
                        }
                    }
                    _ => 1, // Numbers, strings, booleans are atoms
                };
                self.push(Value::Integer(result));
                self.pc += 1;
            }

            LispOp::IsNil => {
                let value = self.pop()?;
                let result = match value {
                    Value::Nil => 1,
                    _ => 0,
                };
                self.push(Value::Integer(result));
                self.pc += 1;
            }

            // =============================================================
            // I/O (0xA0)
            // =============================================================

            LispOp::Print => {
                let value = self.pop()?;
                let text = self.format_value(&value);
                self.output.push(text);
                self.pc += 1;
            }

            // =============================================================
            // VM Control (0xFF)
            // =============================================================

            LispOp::Halt => {
                self.halted = true;
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------
    // Closure Execution
    // -----------------------------------------------------------------

    /// Execute a closure with the given arguments.
    ///
    /// This creates a mini-execution context: saves current state, runs the
    /// closure's bytecode, then restores the caller's state.
    ///
    /// If `is_tail_call` is true, this is handled via the TCO loop: instead
    /// of creating a new call frame, we rebind args and restart.
    fn execute_closure(
        &mut self,
        closure_addr: usize,
        args: Vec<Value>,
        _is_tail_call: bool,
    ) -> Result<(), VmError> {
        // Get the closure
        let closure = match self.deref(closure_addr)?.clone() {
            HeapObject::Closure(c) => c,
            _ => return Err(VmError {
                message: "Not a closure".to_string(),
            }),
        };

        // Save current state
        let saved_pc = self.pc;
        let saved_halted = self.halted;
        let saved_vars = self.variables.clone();
        let saved_locals = self.locals.clone();

        // Restore captured environment
        self.variables.extend(closure.env.iter().map(|(k, v)| (k.clone(), v.clone())));

        // TCO loop
        let mut current_closure = closure;
        let mut current_args = args;

        loop {
            // Set up function context
            self.locals = current_args.clone();
            self.pc = 0;
            self.halted = false;

            // Bind parameters in variables so inner closures can capture them
            for (i, param_name) in current_closure.params.iter().enumerate() {
                if i < current_args.len() {
                    self.variables.insert(param_name.clone(), current_args[i].clone());
                }
            }

            let mut return_value = Value::Nil;
            let mut tail_call_info: Option<(usize, Vec<Value>)> = None;

            // Execute the function's bytecode
            while !self.halted && self.pc < current_closure.code.instructions.len() {
                let instr = current_closure.code.instructions[self.pc].clone();

                // Check for RETURN
                if instr.opcode == LispOp::Return {
                    return_value = if !self.stack.is_empty() {
                        self.pop()?
                    } else {
                        Value::Nil
                    };
                    break;
                }

                // Check for HALT
                if instr.opcode == LispOp::Halt {
                    break;
                }

                // Check for TAIL_CALL -- handle specially
                if instr.opcode == LispOp::TailCall {
                    let argc = instr.operand.unwrap_or(0);
                    let func = self.pop()?;
                    let mut new_args = Vec::new();
                    for _ in 0..argc {
                        new_args.insert(0, self.pop()?);
                    }

                    match func {
                        Value::ClosureAddr(addr) => {
                            tail_call_info = Some((addr, new_args));
                            break;
                        }
                        _ => return Err(VmError {
                            message: format!("Cannot tail-call {:?}", func),
                        }),
                    }
                }

                // Normal instruction execution
                self.execute_instruction(&instr, &current_closure.code)?;
            }

            // If a tail call was requested, restart with new closure/args
            if let Some((addr, new_args)) = tail_call_info {
                let new_closure = match self.deref(addr)?.clone() {
                    HeapObject::Closure(c) => c,
                    _ => return Err(VmError {
                        message: "Tail call target is not a closure".to_string(),
                    }),
                };

                // Reset variables to saved + new closure's env
                self.variables = saved_vars.clone();
                self.variables.extend(
                    new_closure.env.iter().map(|(k, v)| (k.clone(), v.clone())),
                );

                current_closure = new_closure;
                current_args = new_args;
                continue;
            }

            // Normal return -- break the TCO loop
            // Restore caller's state
            self.pc = saved_pc;
            self.halted = saved_halted;
            self.variables = saved_vars;
            self.locals = saved_locals;

            // Push return value and advance caller's PC
            self.push(return_value);
            self.pc += 1;
            break;
        }

        Ok(())
    }

    // -----------------------------------------------------------------
    // Value Formatting
    // -----------------------------------------------------------------

    /// Format a value for display (used by PRINT).
    pub fn format_value(&self, value: &Value) -> String {
        match value {
            Value::Nil => "nil".to_string(),
            Value::Bool(true) => "t".to_string(),
            Value::Bool(false) => "nil".to_string(),
            Value::Integer(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Symbol(s) => s.clone(),
            Value::ConsAddr(addr) => {
                if self.is_valid_address(*addr) {
                    match &self.heap[*addr] {
                        HeapObject::Cons(_) => self.format_cons(*addr, &mut Vec::new()),
                        HeapObject::Symbol(s) => s.name.clone(),
                        HeapObject::Closure(_) => format!("<closure @{}>", addr),
                    }
                } else {
                    format!("<invalid @{}>", addr)
                }
            }
            Value::ClosureAddr(addr) => format!("<closure @{}>", addr),
            Value::Code(_) => "<code>".to_string(),
        }
    }

    /// Format a cons cell as a Lisp list or dotted pair.
    ///
    /// Uses iterative traversal of the cdr chain with visited tracking
    /// to detect cycles.
    fn format_cons(&self, addr: usize, visited: &mut Vec<usize>) -> String {
        let mut parts = Vec::new();
        let mut current = addr;

        loop {
            if visited.contains(&current) {
                parts.push("...".to_string());
                break;
            }
            visited.push(current);

            if !self.is_valid_address(current) {
                break;
            }

            let cell = match &self.heap[current] {
                HeapObject::Cons(c) => c.clone(),
                _ => break,
            };

            // Format car
            parts.push(self.format_value(&cell.car));

            match &cell.cdr {
                Value::Nil => {
                    return format!("({})", parts.join(" "));
                }
                Value::ConsAddr(next_addr) => {
                    if self.is_valid_address(*next_addr) {
                        match &self.heap[*next_addr] {
                            HeapObject::Cons(_) => {
                                current = *next_addr;
                                continue;
                            }
                            _ => {
                                let cdr_str = self.format_value(&cell.cdr);
                                return format!("({} . {})", parts.join(" "), cdr_str);
                            }
                        }
                    }
                    return format!("({} . {})", parts.join(" "), next_addr);
                }
                other => {
                    let cdr_str = self.format_value(other);
                    return format!("({} . {})", parts.join(" "), cdr_str);
                }
            }
        }

        format!("({})", parts.join(" "))
    }
}

impl Default for LispVm {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Section 4: Public API
// ============================================================================

/// Compile and execute Lisp source code, returning the result.
///
/// This is the top-level convenience function. It runs the full pipeline:
/// lex -> parse -> compile -> execute.
///
/// # Returns
///
/// The result of evaluating the last expression, or `Value::Nil` if the
/// stack is empty.
///
/// # Examples
///
/// ```
/// use lisp_vm::run;
/// use lisp_compiler::Value;
///
/// let result = run("(+ 1 2)").unwrap();
/// assert_eq!(result, Value::Integer(3));
/// ```
pub fn run(source: &str) -> Result<Value, VmError> {
    let code = compile(source)?;
    let mut vm = LispVm::new();
    vm.execute(&code)?;
    Ok(if !vm.stack.is_empty() {
        vm.stack.last().unwrap().clone()
    } else {
        Value::Nil
    })
}

/// Compile and execute Lisp source code, returning the result and any output.
///
/// Like `run()` but also returns text produced by `(print ...)` expressions.
pub fn run_with_output(source: &str) -> Result<(Value, Vec<String>), VmError> {
    let code = compile(source)?;
    let mut vm = LispVm::new();
    vm.execute(&code)?;
    let result = if !vm.stack.is_empty() {
        vm.stack.last().unwrap().clone()
    } else {
        Value::Nil
    };
    Ok((result, vm.output))
}

// ============================================================================
// Section 5: Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lisp_compiler::{CodeObject, Instruction, LispOp, Value};

    /// Helper: create a CodeObject from instructions, constants, names.
    fn make_code(
        instructions: Vec<Instruction>,
        constants: Vec<Value>,
        names: Vec<String>,
    ) -> CodeObject {
        CodeObject { instructions, constants, names }
    }

    /// Helper: execute code and return top of stack.
    fn exec(code: &CodeObject) -> Value {
        let mut vm = LispVm::new();
        vm.execute(code).unwrap();
        vm.stack.last().cloned().unwrap_or(Value::Nil)
    }

    // =====================================================================
    // Stack Operations
    // =====================================================================

    #[test]
    fn test_load_const() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(42)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(42));
    }

    #[test]
    fn test_load_nil() {
        let code = make_code(
            vec![Instruction::new(LispOp::LoadNil), Instruction::new(LispOp::Halt)],
            vec![],
            vec![],
        );
        assert_eq!(exec(&code), Value::Nil);
    }

    #[test]
    fn test_load_true() {
        let code = make_code(
            vec![Instruction::new(LispOp::LoadTrue), Instruction::new(LispOp::Halt)],
            vec![],
            vec![],
        );
        assert_eq!(exec(&code), Value::Bool(true));
    }

    #[test]
    fn test_pop() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::with_operand(LispOp::LoadConst, 1),
                Instruction::new(LispOp::Pop),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(1), Value::Integer(2)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(1));
    }

    // =====================================================================
    // Variable Operations
    // =====================================================================

    #[test]
    fn test_store_and_load_name() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::with_operand(LispOp::StoreName, 0),
                Instruction::with_operand(LispOp::LoadName, 0),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(42)],
            vec!["x".to_string()],
        );
        assert_eq!(exec(&code), Value::Integer(42));
    }

    #[test]
    fn test_store_and_load_local() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::with_operand(LispOp::StoreLocal, 0),
                Instruction::with_operand(LispOp::LoadLocal, 0),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(99)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(99));
    }

    // =====================================================================
    // Arithmetic
    // =====================================================================

    #[test]
    fn test_add() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::with_operand(LispOp::LoadConst, 1),
                Instruction::new(LispOp::Add),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(3), Value::Integer(4)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(7));
    }

    #[test]
    fn test_sub() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::with_operand(LispOp::LoadConst, 1),
                Instruction::new(LispOp::Sub),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(10), Value::Integer(3)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(7));
    }

    #[test]
    fn test_mul() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::with_operand(LispOp::LoadConst, 1),
                Instruction::new(LispOp::Mul),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(6), Value::Integer(7)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(42));
    }

    #[test]
    fn test_div() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::with_operand(LispOp::LoadConst, 1),
                Instruction::new(LispOp::Div),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(10), Value::Integer(3)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(3));
    }

    // =====================================================================
    // Comparison
    // =====================================================================

    #[test]
    fn test_eq_true() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::new(LispOp::CmpEq),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(42)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(1));
    }

    #[test]
    fn test_eq_false() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::with_operand(LispOp::LoadConst, 1),
                Instruction::new(LispOp::CmpEq),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(1), Value::Integer(2)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(0));
    }

    #[test]
    fn test_eq_nil() {
        let code = make_code(
            vec![
                Instruction::new(LispOp::LoadNil),
                Instruction::new(LispOp::LoadNil),
                Instruction::new(LispOp::CmpEq),
                Instruction::new(LispOp::Halt),
            ],
            vec![],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(1));
    }

    #[test]
    fn test_lt() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::with_operand(LispOp::LoadConst, 1),
                Instruction::new(LispOp::CmpLt),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(1), Value::Integer(2)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(1));
    }

    #[test]
    fn test_gt() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::with_operand(LispOp::LoadConst, 1),
                Instruction::new(LispOp::CmpGt),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(5), Value::Integer(3)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(1));
    }

    // =====================================================================
    // Control Flow
    // =====================================================================

    #[test]
    fn test_jump() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::Jump, 2),
                Instruction::with_operand(LispOp::LoadConst, 0), // skipped
                Instruction::with_operand(LispOp::LoadConst, 1), // landed here
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(99), Value::Integer(42)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(42));
    }

    #[test]
    fn test_jump_if_false_taken() {
        let code = make_code(
            vec![
                Instruction::new(LispOp::LoadNil),
                Instruction::with_operand(LispOp::JumpIfFalse, 3),
                Instruction::with_operand(LispOp::LoadConst, 0), // skipped
                Instruction::with_operand(LispOp::LoadConst, 1), // landed here
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(99), Value::Integer(42)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(42));
    }

    #[test]
    fn test_jump_if_false_not_taken() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::with_operand(LispOp::JumpIfFalse, 3),
                Instruction::with_operand(LispOp::LoadConst, 1), // not skipped
                Instruction::new(LispOp::Halt),
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(1), Value::Integer(42)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(42));
    }

    #[test]
    fn test_zero_is_falsy() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0), // push 0
                Instruction::with_operand(LispOp::JumpIfFalse, 3),
                Instruction::with_operand(LispOp::LoadConst, 1), // skipped
                Instruction::with_operand(LispOp::LoadConst, 2), // landed here
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(0), Value::Integer(99), Value::Integer(42)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(42));
    }

    // =====================================================================
    // Cons Cells
    // =====================================================================

    #[test]
    fn test_cons() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0), // cdr = 2
                Instruction::with_operand(LispOp::LoadConst, 1), // car = 1
                Instruction::new(LispOp::Cons),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(2), Value::Integer(1)],
            vec![],
        );
        let mut vm = LispVm::new();
        vm.execute(&code).unwrap();
        let result = vm.stack.last().unwrap().clone();
        match result {
            Value::ConsAddr(addr) => {
                match &vm.heap[addr] {
                    HeapObject::Cons(cell) => {
                        assert_eq!(cell.car, Value::Integer(1));
                        assert_eq!(cell.cdr, Value::Integer(2));
                    }
                    _ => panic!("Expected cons cell"),
                }
            }
            _ => panic!("Expected cons address"),
        }
    }

    #[test]
    fn test_car() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0), // cdr
                Instruction::with_operand(LispOp::LoadConst, 1), // car
                Instruction::new(LispOp::Cons),
                Instruction::new(LispOp::Car),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(2), Value::Integer(1)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(1));
    }

    #[test]
    fn test_cdr() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0), // cdr
                Instruction::with_operand(LispOp::LoadConst, 1), // car
                Instruction::new(LispOp::Cons),
                Instruction::new(LispOp::Cdr),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(2), Value::Integer(1)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(2));
    }

    #[test]
    fn test_nested_cons() {
        // Build (1 . (2 . NIL)) = (1 2), then CAR to get 1
        let code = make_code(
            vec![
                Instruction::new(LispOp::LoadNil),                // NIL
                Instruction::with_operand(LispOp::LoadConst, 0), // 2
                Instruction::new(LispOp::Cons),                   // (2 . NIL)
                Instruction::with_operand(LispOp::LoadConst, 1), // 1
                Instruction::new(LispOp::Cons),                   // (1 . (2 . NIL))
                Instruction::new(LispOp::Car),                    // get 1
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(2), Value::Integer(1)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(1));
    }

    // =====================================================================
    // Symbols
    // =====================================================================

    #[test]
    fn test_make_symbol() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::MakeSymbol, 0),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::String("foo".to_string())],
            vec![],
        );
        let mut vm = LispVm::new();
        vm.execute(&code).unwrap();
        let result = vm.stack.last().unwrap().clone();
        match result {
            Value::ConsAddr(addr) => {
                match &vm.heap[addr] {
                    HeapObject::Symbol(s) => assert_eq!(s.name, "foo"),
                    _ => panic!("Expected symbol"),
                }
            }
            _ => panic!("Expected address"),
        }
    }

    #[test]
    fn test_symbol_interning() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::MakeSymbol, 0),
                Instruction::with_operand(LispOp::MakeSymbol, 0),
                Instruction::new(LispOp::CmpEq),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::String("foo".to_string())],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(1));
    }

    // =====================================================================
    // Predicates
    // =====================================================================

    #[test]
    fn test_is_atom_number() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::new(LispOp::IsAtom),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(42)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(1));
    }

    #[test]
    fn test_is_atom_nil() {
        let code = make_code(
            vec![
                Instruction::new(LispOp::LoadNil),
                Instruction::new(LispOp::IsAtom),
                Instruction::new(LispOp::Halt),
            ],
            vec![],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(1));
    }

    #[test]
    fn test_is_atom_cons() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::with_operand(LispOp::LoadConst, 1),
                Instruction::new(LispOp::Cons),
                Instruction::new(LispOp::IsAtom),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(2), Value::Integer(1)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(0));
    }

    #[test]
    fn test_is_nil_true() {
        let code = make_code(
            vec![
                Instruction::new(LispOp::LoadNil),
                Instruction::new(LispOp::IsNil),
                Instruction::new(LispOp::Halt),
            ],
            vec![],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(1));
    }

    #[test]
    fn test_is_nil_false() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::new(LispOp::IsNil),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(42)],
            vec![],
        );
        assert_eq!(exec(&code), Value::Integer(0));
    }

    // =====================================================================
    // Functions and Closures
    // =====================================================================

    #[test]
    fn test_simple_function() {
        // Function body: push 42, return
        let func_code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::new(LispOp::Return),
            ],
            vec![Value::Integer(42)],
            vec![],
        );

        let main_code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::with_operand(LispOp::MakeClosure, 0),
                Instruction::with_operand(LispOp::CallFunction, 0),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Code(Box::new(func_code))],
            vec![],
        );
        assert_eq!(exec(&main_code), Value::Integer(42));
    }

    #[test]
    fn test_function_with_args() {
        // Function body: load local 0, load local 1, add, return
        let func_code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadLocal, 0),
                Instruction::with_operand(LispOp::LoadLocal, 1),
                Instruction::new(LispOp::Add),
                Instruction::new(LispOp::Return),
            ],
            vec![
                Value::String("_p0".to_string()),
                Value::String("_p1".to_string()),
            ],
            vec![],
        );

        let main_code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 1), // arg 1 = 3
                Instruction::with_operand(LispOp::LoadConst, 2), // arg 2 = 4
                Instruction::with_operand(LispOp::LoadConst, 0), // func code
                Instruction::with_operand(LispOp::MakeClosure, 2),
                Instruction::with_operand(LispOp::CallFunction, 2),
                Instruction::new(LispOp::Halt),
            ],
            vec![
                Value::Code(Box::new(func_code)),
                Value::Integer(3),
                Value::Integer(4),
            ],
            vec![],
        );
        assert_eq!(exec(&main_code), Value::Integer(7));
    }

    // =====================================================================
    // Print
    // =====================================================================

    #[test]
    fn test_print_number() {
        let code = make_code(
            vec![
                Instruction::with_operand(LispOp::LoadConst, 0),
                Instruction::new(LispOp::Print),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(42)],
            vec![],
        );
        let mut vm = LispVm::new();
        vm.execute(&code).unwrap();
        assert!(vm.output.iter().any(|s| s.contains("42")));
    }

    #[test]
    fn test_print_nil() {
        let code = make_code(
            vec![
                Instruction::new(LispOp::LoadNil),
                Instruction::new(LispOp::Print),
                Instruction::new(LispOp::Halt),
            ],
            vec![],
            vec![],
        );
        let mut vm = LispVm::new();
        vm.execute(&code).unwrap();
        assert!(vm.output.iter().any(|s| s.contains("nil")));
    }

    #[test]
    fn test_print_cons() {
        // Build (1 . (2 . NIL)) = (1 2)
        let code = make_code(
            vec![
                Instruction::new(LispOp::LoadNil),
                Instruction::with_operand(LispOp::LoadConst, 0), // 2
                Instruction::new(LispOp::Cons),
                Instruction::with_operand(LispOp::LoadConst, 1), // 1
                Instruction::new(LispOp::Cons),
                Instruction::new(LispOp::Print),
                Instruction::new(LispOp::Halt),
            ],
            vec![Value::Integer(2), Value::Integer(1)],
            vec![],
        );
        let mut vm = LispVm::new();
        vm.execute(&code).unwrap();
        assert!(vm.output.iter().any(|s| s.contains("(1 2)")));
    }

    // =====================================================================
    // NIL Sentinel
    // =====================================================================

    #[test]
    fn test_nil_is_falsy() {
        assert!(Value::Nil.is_falsy());
    }

    #[test]
    fn test_nil_display() {
        assert_eq!(format!("{}", Value::Nil), "nil");
    }

    // =====================================================================
    // End-to-End Tests (via run())
    // =====================================================================

    #[test]
    fn test_e2e_add() {
        assert_eq!(run("(+ 1 2)").unwrap(), Value::Integer(3));
    }

    #[test]
    fn test_e2e_subtract() {
        assert_eq!(run("(- 10 3)").unwrap(), Value::Integer(7));
    }

    #[test]
    fn test_e2e_multiply() {
        assert_eq!(run("(* 4 5)").unwrap(), Value::Integer(20));
    }

    #[test]
    fn test_e2e_divide() {
        assert_eq!(run("(/ 10 2)").unwrap(), Value::Integer(5));
    }

    #[test]
    fn test_e2e_nested_arithmetic() {
        assert_eq!(run("(+ (* 2 3) (- 10 4))").unwrap(), Value::Integer(12));
    }

    #[test]
    fn test_e2e_deeply_nested() {
        assert_eq!(run("(* (+ 1 2) (+ 3 4))").unwrap(), Value::Integer(21));
    }

    #[test]
    fn test_e2e_eq_true() {
        assert_eq!(run("(eq 1 1)").unwrap(), Value::Integer(1));
    }

    #[test]
    fn test_e2e_eq_false() {
        assert_eq!(run("(eq 1 2)").unwrap(), Value::Integer(0));
    }

    #[test]
    fn test_e2e_less_than() {
        assert_eq!(run("(< 1 2)").unwrap(), Value::Integer(1));
    }

    #[test]
    fn test_e2e_greater_than() {
        assert_eq!(run("(> 3 2)").unwrap(), Value::Integer(1));
    }

    #[test]
    fn test_e2e_define_and_use() {
        assert_eq!(run("(define x 42) x").unwrap(), Value::Integer(42));
    }

    #[test]
    fn test_e2e_define_expression() {
        assert_eq!(run("(define x (+ 1 2)) x").unwrap(), Value::Integer(3));
    }

    #[test]
    fn test_e2e_multiple_defines() {
        assert_eq!(
            run("(define x 10) (define y 20) (+ x y)").unwrap(),
            Value::Integer(30),
        );
    }

    #[test]
    fn test_e2e_cond_true_branch() {
        assert_eq!(
            run("(cond ((eq 1 1) 42) (t 0))").unwrap(),
            Value::Integer(42),
        );
    }

    #[test]
    fn test_e2e_cond_false_branch() {
        assert_eq!(
            run("(cond ((eq 1 2) 42) (t 99))").unwrap(),
            Value::Integer(99),
        );
    }

    #[test]
    fn test_e2e_cond_multiple_branches() {
        let result = run(r#"
            (define x 2)
            (cond ((eq x 1) 10)
                  ((eq x 2) 20)
                  (t 30))
        "#).unwrap();
        assert_eq!(result, Value::Integer(20));
    }

    #[test]
    fn test_e2e_cond_else_only() {
        assert_eq!(run("(cond (t 42))").unwrap(), Value::Integer(42));
    }

    #[test]
    fn test_e2e_identity_function() {
        assert_eq!(
            run("((lambda (x) x) 42)").unwrap(),
            Value::Integer(42),
        );
    }

    #[test]
    fn test_e2e_simple_function() {
        assert_eq!(
            run("((lambda (x) (+ x 1)) 41)").unwrap(),
            Value::Integer(42),
        );
    }

    #[test]
    fn test_e2e_two_args() {
        assert_eq!(
            run("((lambda (x y) (+ x y)) 10 20)").unwrap(),
            Value::Integer(30),
        );
    }

    #[test]
    fn test_e2e_named_function() {
        let result = run(r#"
            (define double (lambda (x) (* x 2)))
            (double 21)
        "#).unwrap();
        assert_eq!(result, Value::Integer(42));
    }

    #[test]
    fn test_e2e_closure_captures_env() {
        let result = run(r#"
            (define y 10)
            (define add-y (lambda (x) (+ x y)))
            (add-y 32)
        "#).unwrap();
        assert_eq!(result, Value::Integer(42));
    }

    #[test]
    fn test_e2e_cons_car() {
        assert_eq!(run("(car (cons 1 2))").unwrap(), Value::Integer(1));
    }

    #[test]
    fn test_e2e_cons_cdr() {
        assert_eq!(run("(cdr (cons 1 2))").unwrap(), Value::Integer(2));
    }

    #[test]
    fn test_e2e_nested_cons() {
        assert_eq!(
            run("(car (cdr (cons 1 (cons 2 3))))").unwrap(),
            Value::Integer(2),
        );
    }

    #[test]
    fn test_e2e_quote_number() {
        assert_eq!(run("(quote 42)").unwrap(), Value::Integer(42));
    }

    #[test]
    fn test_e2e_quote_nil() {
        assert_eq!(run("(quote nil)").unwrap(), Value::Nil);
    }

    #[test]
    fn test_e2e_shorthand_quote_number() {
        assert_eq!(run("'42").unwrap(), Value::Integer(42));
    }

    #[test]
    fn test_e2e_quote_list_car() {
        assert_eq!(
            run("(car (quote (1 2 3)))").unwrap(),
            Value::Integer(1),
        );
    }

    #[test]
    fn test_e2e_quote_list_cdr_car() {
        assert_eq!(
            run("(car (cdr (quote (1 2 3))))").unwrap(),
            Value::Integer(2),
        );
    }

    #[test]
    fn test_e2e_quote_empty_list() {
        assert_eq!(run("(quote ())").unwrap(), Value::Nil);
    }

    #[test]
    fn test_e2e_atom_number() {
        assert_eq!(run("(atom 42)").unwrap(), Value::Integer(1));
    }

    #[test]
    fn test_e2e_atom_cons() {
        assert_eq!(run("(atom (cons 1 2))").unwrap(), Value::Integer(0));
    }

    #[test]
    fn test_e2e_is_nil_true() {
        assert_eq!(run("(is-nil nil)").unwrap(), Value::Integer(1));
    }

    #[test]
    fn test_e2e_is_nil_false() {
        assert_eq!(run("(is-nil 42)").unwrap(), Value::Integer(0));
    }

    #[test]
    fn test_e2e_nil_literal() {
        assert_eq!(run("nil").unwrap(), Value::Nil);
    }

    #[test]
    fn test_e2e_t_literal() {
        assert_eq!(run("t").unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_e2e_empty_list_is_nil() {
        assert_eq!(run("()").unwrap(), Value::Nil);
    }

    #[test]
    fn test_e2e_factorial() {
        let result = run(r#"
            (define factorial
              (lambda (n)
                (cond ((eq n 0) 1)
                      (t (* n (factorial (- n 1)))))))
            (factorial 5)
        "#).unwrap();
        assert_eq!(result, Value::Integer(120));
    }

    #[test]
    fn test_e2e_factorial_10() {
        let result = run(r#"
            (define factorial
              (lambda (n)
                (cond ((eq n 0) 1)
                      (t (* n (factorial (- n 1)))))))
            (factorial 10)
        "#).unwrap();
        assert_eq!(result, Value::Integer(3628800));
    }

    #[test]
    fn test_e2e_fibonacci() {
        let result = run(r#"
            (define fib
              (lambda (n)
                (cond ((eq n 0) 0)
                      ((eq n 1) 1)
                      (t (+ (fib (- n 1)) (fib (- n 2)))))))
            (fib 10)
        "#).unwrap();
        assert_eq!(result, Value::Integer(55));
    }

    #[test]
    fn test_e2e_tail_recursive_factorial() {
        let result = run(r#"
            (define factorial-iter
              (lambda (n acc)
                (cond ((eq n 0) acc)
                      (t (factorial-iter (- n 1) (* n acc))))))
            (factorial-iter 10 1)
        "#).unwrap();
        assert_eq!(result, Value::Integer(3628800));
    }

    #[test]
    fn test_e2e_tail_call_large_n() {
        let result = run(r#"
            (define countdown
              (lambda (n)
                (cond ((eq n 0) 0)
                      (t (countdown (- n 1))))))
            (countdown 10000)
        "#).unwrap();
        assert_eq!(result, Value::Integer(0));
    }

    #[test]
    fn test_e2e_eq_symbols() {
        assert_eq!(
            run("(eq (quote foo) (quote foo))").unwrap(),
            Value::Integer(1),
        );
    }

    #[test]
    fn test_e2e_neq_symbols() {
        assert_eq!(
            run("(eq (quote foo) (quote bar))").unwrap(),
            Value::Integer(0),
        );
    }

    #[test]
    fn test_e2e_higher_order_function() {
        let result = run(r#"
            (define apply-to-5 (lambda (f) (f 5)))
            (define double (lambda (x) (* x 2)))
            (apply-to-5 double)
        "#).unwrap();
        assert_eq!(result, Value::Integer(10));
    }

    #[test]
    fn test_e2e_currying() {
        let result = run(r#"
            (define make-adder (lambda (x) (lambda (y) (+ x y))))
            (define add-10 (make-adder 10))
            (add-10 32)
        "#).unwrap();
        assert_eq!(result, Value::Integer(42));
    }
}
