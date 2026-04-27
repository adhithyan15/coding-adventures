//! # Virtual Machine — a generic, pluggable bytecode execution engine.
//!
//! A virtual machine (VM) is a software-based computer that executes instructions
//! just like a physical CPU, but entirely in software. Instead of transistors and
//! silicon, we use data structures (a stack, variables, a program counter) and
//! Rust code to simulate the fetch-decode-execute cycle.
//!
//! ## Why "generic"?
//!
//! Most VMs are hardcoded to one language's bytecode. The JVM runs Java bytecode,
//! CPython's VM runs Python bytecode, and so on. This VM takes a different approach:
//! it provides the **execution machinery** (stack, variables, program counter, call
//! stack) but lets you **plug in** your own opcodes and handlers. Think of it like
//! a CPU where you can define the instruction set.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │  GenericVM                                   │
//! │                                              │
//! │  ┌──────────┐  ┌──────────┐  ┌───────────┐  │
//! │  │  Stack    │  │ Variables│  │ Call Stack │  │
//! │  │ [v1,v2]  │  │ {x: 42}  │  │ [frame..]  │  │
//! │  └──────────┘  └──────────┘  └───────────┘  │
//! │                                              │
//! │  ┌──────────────────────────────────────┐    │
//! │  │  Handler Table (OpCode -> Function)  │    │
//! │  │  0x01 -> load_const_handler          │    │
//! │  │  0x02 -> pop_handler                 │    │
//! │  │  0x10 -> add_handler                 │    │
//! │  └──────────────────────────────────────┘    │
//! │                                              │
//! │  PC: 0  ──►  fetch ──► decode ──► execute    │
//! └─────────────────────────────────────────────┘
//! ```
//!
//! ## The Fetch-Decode-Execute Cycle
//!
//! Every step of execution follows the same pattern:
//! 1. **Fetch**: Read the instruction at the current program counter (PC)
//! 2. **Decode**: Look up the handler for this instruction's opcode
//! 3. **Execute**: Run the handler, which may modify the stack, variables, or PC
//! 4. **Advance**: Move PC to the next instruction (unless the handler jumped)
//!
//! This is exactly what a physical CPU does billions of times per second.

use std::collections::HashMap;
use std::any::Any;

// ─────────────────────────────────────────────────────────────────────────────
// Section 1: Core Types
// ─────────────────────────────────────────────────────────────────────────────

/// An opcode is just a byte — a number from 0 to 255 that identifies an
/// instruction. For example, 0x01 might mean "load a constant onto the stack"
/// and 0x10 might mean "add the top two stack values."
///
/// Using a simple `u8` keeps things fast and cache-friendly, just like real CPUs
/// where opcodes are fixed-width fields in the instruction encoding.
pub type OpCode = u8;

/// Standard opcodes that many language VMs share. These are the building blocks
/// that virtually every bytecode language needs:
///
/// | Category    | Opcodes                                  |
/// |-------------|------------------------------------------|
/// | Stack       | LOAD_CONST, POP, DUP                     |
/// | Variables   | STORE_NAME, LOAD_NAME, STORE_LOCAL, etc.  |
/// | Arithmetic  | ADD, SUBTRACT, MULTIPLY, DIVIDE, MODULO  |
/// | Comparison  | EQUAL, NOT_EQUAL, LESS_THAN, etc.         |
/// | Logic       | AND, OR, NOT                              |
/// | Control     | JUMP, JUMP_IF_TRUE, JUMP_IF_FALSE         |
/// | Functions   | CALL, RETURN                              |
/// | I/O         | PRINT                                     |
/// | System      | HALT                                      |
pub mod opcodes {
    use super::OpCode;

    // -- Stack manipulation --
    /// Push a constant from the constants pool onto the stack.
    pub const LOAD_CONST: OpCode = 0x01;
    /// Remove and discard the top value from the stack.
    pub const POP: OpCode = 0x02;
    /// Duplicate the top value on the stack.
    pub const DUP: OpCode = 0x03;

    // -- Variable access --
    /// Store the top-of-stack value into a named variable.
    pub const STORE_NAME: OpCode = 0x04;
    /// Load a named variable's value onto the stack.
    pub const LOAD_NAME: OpCode = 0x05;
    /// Store into a local variable slot (by index).
    pub const STORE_LOCAL: OpCode = 0x06;
    /// Load from a local variable slot (by index).
    pub const LOAD_LOCAL: OpCode = 0x07;

    // -- Arithmetic --
    /// Pop two values, push their sum.
    pub const ADD: OpCode = 0x10;
    /// Pop two values, push (second - first).
    pub const SUBTRACT: OpCode = 0x11;
    /// Pop two values, push their product.
    pub const MULTIPLY: OpCode = 0x12;
    /// Pop two values, push (second / first). Errors on division by zero.
    pub const DIVIDE: OpCode = 0x13;
    /// Pop two values, push (second % first).
    pub const MODULO: OpCode = 0x14;

    // -- Comparison --
    /// Pop two values, push Bool(second == first).
    pub const EQUAL: OpCode = 0x20;
    /// Pop two values, push Bool(second != first).
    pub const NOT_EQUAL: OpCode = 0x21;
    /// Pop two values, push Bool(second < first).
    pub const LESS_THAN: OpCode = 0x22;
    /// Pop two values, push Bool(second <= first).
    pub const LESS_EQUAL: OpCode = 0x23;
    /// Pop two values, push Bool(second > first).
    pub const GREATER_THAN: OpCode = 0x24;
    /// Pop two values, push Bool(second >= first).
    pub const GREATER_EQUAL: OpCode = 0x25;

    // -- Logical --
    /// Pop two booleans, push their logical AND.
    pub const AND: OpCode = 0x30;
    /// Pop two booleans, push their logical OR.
    pub const OR: OpCode = 0x31;
    /// Pop one boolean, push its logical NOT.
    pub const NOT: OpCode = 0x32;

    // -- Control flow --
    /// Unconditional jump to instruction at operand index.
    pub const JUMP: OpCode = 0x40;
    /// Pop a value; jump if it's truthy.
    pub const JUMP_IF_TRUE: OpCode = 0x41;
    /// Pop a value; jump if it's falsy.
    pub const JUMP_IF_FALSE: OpCode = 0x42;

    // -- Functions --
    /// Call a function with N arguments (operand = arg count).
    pub const CALL: OpCode = 0x50;
    /// Return from the current function.
    pub const RETURN: OpCode = 0x51;

    // -- I/O --
    /// Pop the top value and print it to output.
    pub const PRINT: OpCode = 0x60;

    // -- System --
    /// Stop execution.
    pub const HALT: OpCode = 0xFF;
}

/// A single bytecode instruction, consisting of an opcode and an optional operand.
///
/// Some instructions are self-contained (like POP — just remove the top of stack),
/// while others need additional data (like LOAD_CONST, which needs to know *which*
/// constant to load). The operand carries that additional data.
///
/// ## Example
///
/// ```text
/// Instruction { opcode: 0x01 (LOAD_CONST), operand: Some(Index(0)) }
///   → "Load constant #0 from the constants pool onto the stack"
///
/// Instruction { opcode: 0x02 (POP), operand: None }
///   → "Remove the top value from the stack"
/// ```
#[derive(Debug, Clone)]
pub struct Instruction {
    /// The operation to perform.
    pub opcode: OpCode,
    /// Additional data needed by the operation, if any.
    pub operand: Option<Operand>,
}

/// An operand provides additional data for an instruction. It can be either:
///
/// - **Index**: A numeric index into some array (constants pool, names list,
///   local variable slots, or a jump target).
/// - **Str**: A string value used directly (e.g., a variable name).
///
/// Most real VMs use only numeric operands and look up strings via indices,
/// but supporting both makes our generic VM more flexible.
#[derive(Debug, Clone)]
pub enum Operand {
    /// A numeric index (into constants, names, locals, or jump targets).
    Index(usize),
    /// A string value used directly.
    Str(String),
}

/// Values that can live on the VM's stack.
///
/// A stack-based VM manipulates values by pushing them onto and popping them
/// off a stack. These are the types of values our VM can handle. This is
/// deliberately kept simple — a real VM would have more types (lists, dicts,
/// objects), but these cover the fundamentals.
///
/// | Variant | Represents              | Example           |
/// |---------|-------------------------|-------------------|
/// | Int     | Whole numbers           | 42, -7            |
/// | Float   | Decimal numbers         | 3.14              |
/// | Str     | Text                    | "hello"           |
/// | Bool    | True/false              | true              |
/// | Code    | A compiled function     | CodeObject        |
/// | Null    | The absence of a value  | null/nil          |
/// | List    | Ordered sequence        | [1, 2, 3]         |
/// | Dict    | Key-value mapping       | {"a": 1, "b": 2}  |
#[derive(Debug, Clone)]
pub enum Value {
    /// A 64-bit signed integer.
    Int(i64),
    /// A 64-bit floating point number.
    Float(f64),
    /// A UTF-8 string.
    Str(String),
    /// A boolean value.
    Bool(bool),
    /// A compiled code object (used for functions/closures).
    Code(Box<CodeObject>),
    /// The null/nil/None value — represents "nothing."
    Null,
    /// An ordered sequence of values (mutable list).
    List(Vec<Value>),
    /// An ordered key-value mapping (dict).
    Dict(Vec<(Value, Value)>),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Str(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Code(_) => write!(f, "<code>"),
            Value::Null => write!(f, "null"),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Dict(pairs) => {
                write!(f, "{{")?;
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TypedVMValue — a value tagged with a type code, used by WASM and other
// typed bytecode engines that need finer type distinctions than Value provides.
// ─────────────────────────────────────────────────────────────────────────────

/// A typed VM value: a raw [`Value`] tagged with an integer type code.
///
/// While the base [`Value`] enum distinguishes ints from floats from strings,
/// some execution engines (e.g. WebAssembly) need to distinguish i32 from i64,
/// f32 from f64, etc.  The `value_type` field carries this extra type
/// information as an opaque `u8`.
///
/// ```text
/// TypedVMValue { value_type: 0x7F, value: Value::Int(42) }
///                ^                  ^
///                |                  The actual runtime value
///                WASM type code for i32
/// ```
///
/// The typed stack (`GenericVM::typed_stack`) stores these values, and the
/// `push_typed` / `pop_typed` / `peek_typed` methods operate on it.
#[derive(Debug, Clone)]
pub struct TypedVMValue {
    /// An opaque type code (e.g. 0x7F for WASM i32, 0x7E for i64, etc.).
    pub value_type: u8,
    /// The actual runtime value.
    pub value: Value,
}

/// A compiled unit of code — the output of a compiler, the input to the VM.
///
/// A CodeObject bundles together everything the VM needs to execute a piece of
/// code:
///
/// - **instructions**: The sequence of bytecode instructions to execute.
/// - **constants**: A pool of literal values referenced by LOAD_CONST instructions.
///   For example, the expression `x = 42` would have `42` in the constants pool.
/// - **names**: A pool of identifier strings referenced by STORE_NAME/LOAD_NAME.
///   For example, `x = 42` would have `"x"` in the names list.
///
/// Functions are represented as nested CodeObjects — a function definition compiles
/// to a CodeObject that gets stored as a `Value::Code` in the parent's constants pool.
#[derive(Debug, Clone)]
pub struct CodeObject {
    /// The bytecode instructions to execute, in order.
    pub instructions: Vec<Instruction>,
    /// Literal values (numbers, strings) referenced by index from instructions.
    pub constants: Vec<Value>,
    /// Variable/function names referenced by index from instructions.
    pub names: Vec<String>,
}

/// An execution trace captures a snapshot of the VM's state at each step.
///
/// Tracing is invaluable for debugging and education. Each trace records:
/// - What instruction was executed
/// - What the stack looked like before and after
/// - What variables exist
/// - Any output produced
/// - A human-readable description of what happened
///
/// By collecting traces, you can replay the entire execution of a program
/// step by step — like a flight recorder for your VM.
#[derive(Debug, Clone)]
pub struct VMTrace {
    /// The program counter value when this instruction was executed.
    pub pc: usize,
    /// The instruction that was executed.
    pub instruction: Instruction,
    /// The stack contents before execution.
    pub stack_before: Vec<Value>,
    /// The stack contents after execution.
    pub stack_after: Vec<Value>,
    /// The variable bindings after execution.
    pub variables: HashMap<String, Value>,
    /// Any output produced by this instruction (e.g., from PRINT).
    pub output: Option<String>,
    /// A human-readable description of what this instruction did.
    pub description: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Section 2: Error Handling
// ─────────────────────────────────────────────────────────────────────────────

/// Errors that can occur during VM execution.
///
/// Rather than panicking (crashing the program), the VM returns typed errors
/// that callers can inspect and handle. This is idiomatic Rust — errors are
/// values, not exceptions.
///
/// Each variant carries a descriptive message explaining what went wrong:
///
/// | Error            | When it occurs                                    |
/// |------------------|---------------------------------------------------|
/// | StackUnderflow   | Tried to pop from an empty stack                  |
/// | UndefinedName    | Referenced a variable that doesn't exist           |
/// | DivisionByZero   | Tried to divide by zero                            |
/// | InvalidOpcode    | Encountered an opcode with no registered handler   |
/// | InvalidOperand   | Instruction's operand was wrong type or missing    |
/// | TypeError        | Operation on incompatible types (e.g., "hi" + 42) |
/// | MaxRecursion     | Call stack exceeded the configured depth limit     |
/// | GenericError     | Catch-all for other errors                         |
#[derive(Debug, Clone)]
pub enum VMError {
    StackUnderflow(String),
    UndefinedName(String),
    DivisionByZero(String),
    InvalidOpcode(String),
    InvalidOperand(String),
    TypeError(String),
    MaxRecursion(String),
    GenericError(String),
}

impl std::fmt::Display for VMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VMError::StackUnderflow(msg) => write!(f, "StackUnderflow: {}", msg),
            VMError::UndefinedName(msg) => write!(f, "UndefinedName: {}", msg),
            VMError::DivisionByZero(msg) => write!(f, "DivisionByZero: {}", msg),
            VMError::InvalidOpcode(msg) => write!(f, "InvalidOpcode: {}", msg),
            VMError::InvalidOperand(msg) => write!(f, "InvalidOperand: {}", msg),
            VMError::TypeError(msg) => write!(f, "TypeError: {}", msg),
            VMError::MaxRecursion(msg) => write!(f, "MaxRecursion: {}", msg),
            VMError::GenericError(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for VMError {}

/// A convenience alias: most VM operations return `VMResult<T>`.
pub type VMResult<T> = Result<T, VMError>;

// ─────────────────────────────────────────────────────────────────────────────
// Section 3: GenericVM — the pluggable execution engine
// ─────────────────────────────────────────────────────────────────────────────

/// The type signature for an opcode handler function.
///
/// A handler receives:
/// - `vm`: Mutable reference to the VM (so it can push/pop the stack, etc.)
/// - `instr`: The instruction being executed (so it can read the operand)
/// - `code`: The CodeObject being executed (so it can access constants/names)
///
/// It returns:
/// - `Ok(Some(description))` — success, with a description for the trace
/// - `Ok(None)` — success, no special description needed
/// - `Err(VMError)` — something went wrong
pub type OpcodeHandler =
    fn(vm: &mut GenericVM, instr: &Instruction, code: &CodeObject) -> VMResult<Option<String>>;

/// A context-aware opcode handler receives an extra `&mut dyn Any` context
/// argument. WASM instruction handlers use this to access the
/// [`WasmExecutionContext`] (memory, tables, globals, locals, label stack).
///
/// The handler signature is:
/// ```text
/// fn(vm, instruction, code_object, context) -> Result<Option<description>, VMError>
/// ```
///
/// The context is type-erased (`&mut dyn Any`) so that GenericVM remains
/// agnostic to the concrete context type. Handlers downcast with
/// `ctx.downcast_mut::<MyContext>().unwrap()`.
pub type ContextOpcodeHandler = fn(
    vm: &mut GenericVM,
    instr: &Instruction,
    code: &CodeObject,
    ctx: &mut dyn Any,
) -> VMResult<Option<String>>;

/// A built-in function that can be called from bytecode.
///
/// Built-ins are functions provided by the VM runtime rather than defined in
/// user code. Common examples include `print`, `len`, `type`, etc. They take
/// a slice of Values as arguments and return a single Value.
pub struct BuiltinFunction {
    /// The name of the built-in (used for lookup and error messages).
    pub name: String,
    /// The Rust function that implements this built-in.
    pub implementation: fn(args: &[Value]) -> Value,
}

/// The GenericVM: a stack-based virtual machine with pluggable opcodes.
///
/// ## How it works
///
/// The VM maintains several pieces of state:
///
/// ```text
/// Stack:      [10, 20, 30]    ← values are pushed/popped here
///                        ^
///                   top of stack
///
/// Variables:  { "x": Int(42), "name": Str("hello") }
///
/// Locals:     [Int(1), Int(2)]  ← indexed local variable slots
///
/// PC:         3                  ← points to the next instruction
///
/// Call Stack: [frame0, frame1]   ← saved variable scopes for function calls
/// ```
///
/// The handler table maps opcodes to functions. When the VM encounters opcode
/// 0x01, it looks up the handler for 0x01 and calls it. This is how the VM
/// stays generic — you define what each opcode does.
///
/// ## Freezing
///
/// Once you've registered all your handlers, you can "freeze" the VM. A frozen
/// VM rejects new handler registrations. This prevents accidental modification
/// after setup is complete.
pub struct GenericVM {
    /// The operand stack — the heart of a stack-based VM.
    pub stack: Vec<Value>,
    /// Named variables (global scope).
    pub variables: HashMap<String, Value>,
    /// Local variable slots (indexed access, faster than hash lookup).
    pub locals: Vec<Value>,
    /// The program counter — index of the next instruction to execute.
    pub pc: usize,
    /// Whether the VM has halted (finished executing).
    pub halted: bool,
    /// Accumulated output (from PRINT instructions or similar).
    pub output: Vec<String>,
    /// The call stack — saved variable scopes for nested function calls.
    /// Each frame is a snapshot of the variables at the call site.
    pub call_stack: Vec<HashMap<String, Value>>,

    // ── Typed stack (for WASM and other typed bytecode engines) ──────────
    //
    // The typed stack carries TypedVMValue entries instead of plain Values.
    // This is used by WASM where every stack slot has an explicit type tag
    // (i32, i64, f32, f64).

    /// The typed operand stack for WASM-style execution.
    pub typed_stack: Vec<TypedVMValue>,

    /// The handler dispatch table: opcode -> handler function.
    handlers: HashMap<OpCode, OpcodeHandler>,
    /// Context-aware handlers: opcode -> context handler function.
    /// These are checked *before* the regular handlers during
    /// `execute_with_context` and receive the execution context.
    pub context_handlers: HashMap<OpCode, ContextOpcodeHandler>,
    /// Built-in functions available to bytecode.
    builtins: HashMap<String, BuiltinFunction>,
    /// Maximum call stack depth (None = unlimited).
    max_recursion_depth: Option<usize>,
    /// Whether the VM is frozen (no new handler registrations allowed).
    frozen: bool,
}

impl GenericVM {
    /// Create a new VM with empty state and no registered handlers.
    ///
    /// After creation, you should register opcode handlers and optionally
    /// built-in functions before calling `execute`.
    pub fn new() -> Self {
        GenericVM {
            stack: Vec::new(),
            variables: HashMap::new(),
            locals: Vec::new(),
            pc: 0,
            halted: false,
            output: Vec::new(),
            call_stack: Vec::new(),
            typed_stack: Vec::new(),
            handlers: HashMap::new(),
            context_handlers: HashMap::new(),
            builtins: HashMap::new(),
            max_recursion_depth: None,
            frozen: false,
        }
    }

    // ── Handler registration ─────────────────────────────────────────────

    /// Register a handler for a specific opcode.
    ///
    /// This is how you define what each instruction does. For example:
    ///
    /// ```text
    /// vm.register_opcode(0x01, load_const_handler);
    /// vm.register_opcode(0x10, add_handler);
    /// ```
    ///
    /// Panics if the VM is frozen.
    pub fn register_opcode(&mut self, opcode: OpCode, handler: OpcodeHandler) {
        assert!(
            !self.frozen,
            "Cannot register opcodes on a frozen VM"
        );
        self.handlers.insert(opcode, handler);
    }

    /// Register a built-in function that can be called from bytecode.
    pub fn register_builtin(&mut self, name: String, f: BuiltinFunction) {
        self.builtins.insert(name, f);
    }

    /// Look up a built-in function by name.
    pub fn get_builtin(&self, name: &str) -> Option<&BuiltinFunction> {
        self.builtins.get(name)
    }

    // ── Stack operations ─────────────────────────────────────────────────

    /// Push a value onto the stack.
    ///
    /// ```text
    /// Before: [10, 20]
    /// push(30)
    /// After:  [10, 20, 30]
    /// ```
    pub fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    /// Pop a value from the stack, returning an error if the stack is empty.
    ///
    /// ```text
    /// Before: [10, 20, 30]
    /// pop() -> Ok(30)
    /// After:  [10, 20]
    /// ```
    pub fn pop(&mut self) -> VMResult<Value> {
        self.stack
            .pop()
            .ok_or_else(|| VMError::StackUnderflow("Cannot pop from empty stack".to_string()))
    }

    /// Peek at the top value without removing it.
    pub fn peek(&self) -> VMResult<&Value> {
        self.stack
            .last()
            .ok_or_else(|| VMError::StackUnderflow("Cannot peek at empty stack".to_string()))
    }

    // ── Call stack operations ────────────────────────────────────────────

    /// Push a new frame onto the call stack (entering a function).
    ///
    /// If a max recursion depth is set, this will return `MaxRecursion`
    /// error if the limit would be exceeded.
    pub fn push_frame(&mut self, frame: HashMap<String, Value>) -> VMResult<()> {
        if let Some(max) = self.max_recursion_depth {
            if self.call_stack.len() >= max {
                return Err(VMError::MaxRecursion(format!(
                    "Maximum recursion depth of {} exceeded",
                    max
                )));
            }
        }
        self.call_stack.push(frame);
        Ok(())
    }

    /// Pop a frame from the call stack (returning from a function).
    pub fn pop_frame(&mut self) -> VMResult<HashMap<String, Value>> {
        self.call_stack.pop().ok_or_else(|| {
            VMError::GenericError("Cannot pop from empty call stack".to_string())
        })
    }

    // ── Program counter operations ───────────────────────────────────────

    /// Advance the program counter to the next instruction.
    pub fn advance_pc(&mut self) {
        self.pc += 1;
    }

    /// Jump to a specific instruction index.
    pub fn jump_to(&mut self, target: usize) {
        self.pc = target;
    }

    // ── Global injection ──────────────────────────────────────────────────

    /// Pre-seed named variables into the VM's global scope.
    ///
    /// These variables are available to the program as regular variables
    /// but are set before execution begins. Useful for build context,
    /// environment info, etc.
    ///
    /// Injected globals are merged into `variables` — they don't replace
    /// the map. If a key already exists, the injected value overwrites it.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use std::collections::HashMap;
    /// let mut globals = HashMap::new();
    /// globals.insert("_ctx".to_string(), Value::Dict(ctx_dict));
    /// vm.inject_globals(globals);
    /// ```
    pub fn inject_globals(&mut self, globals: HashMap<String, Value>) {
        for (key, value) in globals {
            self.variables.insert(key, value);
        }
    }

    // ── Configuration ────────────────────────────────────────────────────

    /// Set the maximum recursion depth. Pass `None` for unlimited.
    pub fn set_max_recursion_depth(&mut self, depth: Option<usize>) {
        self.max_recursion_depth = depth;
    }

    /// Get the current maximum recursion depth setting.
    pub fn max_recursion_depth(&self) -> Option<usize> {
        self.max_recursion_depth
    }

    /// Freeze the VM, preventing further handler registrations.
    pub fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
    }

    /// Check whether the VM is frozen.
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    // ── Execution ────────────────────────────────────────────────────────

    /// Execute a CodeObject to completion, returning a trace of every step.
    ///
    /// This is the main entry point for running bytecode. It repeatedly calls
    /// `step()` until the VM halts or runs out of instructions.
    ///
    /// ## The execution loop
    ///
    /// ```text
    /// while not halted and pc < instructions.len():
    ///     trace = step(code)
    ///     traces.push(trace)
    /// ```
    pub fn execute(&mut self, code: &CodeObject) -> VMResult<Vec<VMTrace>> {
        let mut traces = Vec::new();
        while !self.halted && self.pc < code.instructions.len() {
            let trace = self.step(code)?;
            traces.push(trace);
        }
        Ok(traces)
    }

    /// Execute a single instruction and return its trace.
    ///
    /// This is the core of the fetch-decode-execute cycle:
    /// 1. Save the current state (for the "before" snapshot)
    /// 2. Fetch the instruction at PC
    /// 3. Look up the handler for this opcode
    /// 4. Execute the handler
    /// 5. Advance PC (unless the handler already moved it)
    /// 6. Capture the "after" snapshot
    pub fn step(&mut self, code: &CodeObject) -> VMResult<VMTrace> {
        // 1. Capture "before" state
        let pc_before = self.pc;
        let stack_before = self.stack.clone();

        // 2. Fetch the instruction
        let instr = code.instructions.get(self.pc).ok_or_else(|| {
            VMError::InvalidOpcode(format!("PC {} is out of bounds", self.pc))
        })?;
        let instr_clone = instr.clone();

        // 3. Look up the handler
        let handler = self.handlers.get(&instr.opcode).copied().ok_or_else(|| {
            VMError::InvalidOpcode(format!("No handler for opcode 0x{:02X}", instr.opcode))
        })?;

        // 4. Execute the handler
        let description = handler(self, &instr_clone, code)?;

        // 5. Auto-advance PC if the handler didn't change it.
        //    (Handlers that jump will have already modified self.pc)
        if self.pc == pc_before {
            self.pc += 1;
        }

        // 6. Capture "after" state and build the trace
        let output = if self.output.len() > traces_output_start(&stack_before, &self.output) {
            // If new output was added during this step, capture it
            None
        } else {
            None
        };

        let trace = VMTrace {
            pc: pc_before,
            instruction: instr_clone,
            stack_before,
            stack_after: self.stack.clone(),
            variables: self.variables.clone(),
            output: description.clone().or(output),
            description: description.unwrap_or_else(|| {
                format!("Executed opcode 0x{:02X}", instr.opcode)
            }),
        };

        Ok(trace)
    }

    /// Reset the VM's execution state while preserving registered handlers
    /// and built-in functions.
    ///
    /// This is useful for running multiple programs on the same VM without
    /// re-registering all the handlers.
    pub fn reset(&mut self) {
        self.stack.clear();
        self.variables.clear();
        self.locals.clear();
        self.pc = 0;
        self.halted = false;
        self.output.clear();
        self.call_stack.clear();
        self.typed_stack.clear();
    }

    // ── Context-aware opcode registration ───────────────────────────────

    /// Register a context-aware handler for a specific opcode.
    ///
    /// Context handlers are dispatched during `execute_with_context` and
    /// receive an extra `&mut dyn Any` argument carrying the execution
    /// context (e.g. a `WasmExecutionContext` for WASM instruction handlers).
    ///
    /// Context handlers take priority over regular handlers when
    /// `execute_with_context` is used.
    ///
    /// Panics if the VM is frozen.
    pub fn register_context_opcode(
        &mut self,
        opcode: OpCode,
        handler: ContextOpcodeHandler,
    ) {
        assert!(
            !self.frozen,
            "Cannot register context opcodes on a frozen VM"
        );
        self.context_handlers.insert(opcode, handler);
    }

    // ── Typed stack operations ──────────────────────────────────────────

    /// Push a typed value onto the typed stack.
    ///
    /// Used by WASM and other typed bytecode engines where each stack
    /// slot carries an explicit type tag (e.g. i32, i64, f32, f64).
    pub fn push_typed(&mut self, value: TypedVMValue) {
        self.typed_stack.push(value);
    }

    /// Pop a typed value from the typed stack.
    ///
    /// Returns `VMError::StackUnderflow` if the typed stack is empty.
    pub fn pop_typed(&mut self) -> VMResult<TypedVMValue> {
        self.typed_stack.pop().ok_or_else(|| {
            VMError::StackUnderflow("Cannot pop from empty typed stack".to_string())
        })
    }

    /// Peek at the top typed value without removing it.
    ///
    /// Returns `VMError::StackUnderflow` if the typed stack is empty.
    pub fn peek_typed(&self) -> VMResult<TypedVMValue> {
        self.typed_stack.last().cloned().ok_or_else(|| {
            VMError::StackUnderflow("Cannot peek at empty typed stack".to_string())
        })
    }

    // ── Context-aware execution ─────────────────────────────────────────

    /// Execute a CodeObject with a context, using context-aware handlers.
    ///
    /// This is the WASM execution entry point. It runs the fetch-decode-execute
    /// cycle, dispatching to context handlers (which receive `ctx`) before
    /// falling back to regular handlers.
    ///
    /// The `ctx` parameter is type-erased (`&mut dyn Any`) so that GenericVM
    /// remains agnostic to the concrete context type. Handlers downcast
    /// with `ctx.downcast_mut::<MyContext>().unwrap()`.
    pub fn execute_with_context(
        &mut self,
        code: &CodeObject,
        ctx: &mut dyn Any,
    ) -> VMResult<()> {
        while !self.halted && self.pc < code.instructions.len() {
            let instr = code.instructions.get(self.pc).ok_or_else(|| {
                VMError::InvalidOpcode(format!("PC {} is out of bounds", self.pc))
            })?;
            let instr_clone = instr.clone();
            let pc_before = self.pc;

            // Try context handler first, then regular handler.
            if let Some(handler) = self.context_handlers.get(&instr.opcode).copied() {
                handler(self, &instr_clone, code, ctx)?;
            } else if let Some(handler) = self.handlers.get(&instr.opcode).copied() {
                handler(self, &instr_clone, code)?;
                // Auto-advance PC if the handler didn't change it.
                if self.pc == pc_before {
                    self.pc += 1;
                }
            } else {
                return Err(VMError::InvalidOpcode(format!(
                    "No handler for opcode 0x{:02X}",
                    instr.opcode
                )));
            }
        }
        Ok(())
    }
}

/// Helper function (currently a no-op placeholder for output tracking).
fn traces_output_start(_stack_before: &[Value], _output: &[String]) -> usize {
    0
}

// ─────────────────────────────────────────────────────────────────────────────
// Section 4: Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test handler functions ───────────────────────────────────────────
    //
    // These are simple handlers used to test the GenericVM's execution
    // machinery. In a real language implementation, handlers would be more
    // complex.

    /// Handler: push the constant at the operand index onto the stack.
    fn load_const_handler(
        vm: &mut GenericVM,
        instr: &Instruction,
        code: &CodeObject,
    ) -> VMResult<Option<String>> {
        let idx = match &instr.operand {
            Some(Operand::Index(i)) => *i,
            _ => {
                return Err(VMError::InvalidOperand(
                    "LOAD_CONST requires an Index operand".to_string(),
                ))
            }
        };
        let val = code.constants.get(idx).ok_or_else(|| {
            VMError::InvalidOperand(format!("Constant index {} out of bounds", idx))
        })?;
        vm.push(val.clone());
        Ok(Some(format!("Loaded constant {}", val)))
    }

    /// Handler: pop the top value and store it in a named variable.
    fn store_name_handler(
        vm: &mut GenericVM,
        instr: &Instruction,
        code: &CodeObject,
    ) -> VMResult<Option<String>> {
        let idx = match &instr.operand {
            Some(Operand::Index(i)) => *i,
            _ => {
                return Err(VMError::InvalidOperand(
                    "STORE_NAME requires an Index operand".to_string(),
                ))
            }
        };
        let name = code.names.get(idx).ok_or_else(|| {
            VMError::InvalidOperand(format!("Name index {} out of bounds", idx))
        })?;
        let val = vm.pop()?;
        vm.variables.insert(name.clone(), val);
        Ok(Some(format!("Stored into '{}'", name)))
    }

    /// Handler: load a named variable's value onto the stack.
    fn load_name_handler(
        vm: &mut GenericVM,
        instr: &Instruction,
        code: &CodeObject,
    ) -> VMResult<Option<String>> {
        let idx = match &instr.operand {
            Some(Operand::Index(i)) => *i,
            _ => {
                return Err(VMError::InvalidOperand(
                    "LOAD_NAME requires an Index operand".to_string(),
                ))
            }
        };
        let name = code.names.get(idx).ok_or_else(|| {
            VMError::InvalidOperand(format!("Name index {} out of bounds", idx))
        })?;
        let val = vm.variables.get(name).ok_or_else(|| {
            VMError::UndefinedName(format!("Variable '{}' is not defined", name))
        })?;
        vm.push(val.clone());
        Ok(Some(format!("Loaded '{}'", name)))
    }

    /// Handler: pop two integers, push their sum.
    fn add_handler(
        vm: &mut GenericVM,
        _instr: &Instruction,
        _code: &CodeObject,
    ) -> VMResult<Option<String>> {
        let b = vm.pop()?;
        let a = vm.pop()?;
        match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => {
                vm.push(Value::Int(x + y));
                Ok(Some(format!("{} + {} = {}", x, y, x + y)))
            }
            _ => Err(VMError::TypeError("ADD requires two integers".to_string())),
        }
    }

    /// Handler: pop the top value and store it as output.
    fn print_handler(
        vm: &mut GenericVM,
        _instr: &Instruction,
        _code: &CodeObject,
    ) -> VMResult<Option<String>> {
        let val = vm.pop()?;
        let s = format!("{}", val);
        vm.output.push(s.clone());
        Ok(Some(format!("Print: {}", s)))
    }

    /// Handler: halt the VM.
    fn halt_handler(
        vm: &mut GenericVM,
        _instr: &Instruction,
        _code: &CodeObject,
    ) -> VMResult<Option<String>> {
        vm.halted = true;
        Ok(Some("HALT".to_string()))
    }

    /// Handler: pop the top value (discard it).
    fn pop_handler(
        vm: &mut GenericVM,
        _instr: &Instruction,
        _code: &CodeObject,
    ) -> VMResult<Option<String>> {
        let val = vm.pop()?;
        Ok(Some(format!("Popped {}", val)))
    }

    /// Handler: unconditional jump.
    fn jump_handler(
        vm: &mut GenericVM,
        instr: &Instruction,
        _code: &CodeObject,
    ) -> VMResult<Option<String>> {
        let target = match &instr.operand {
            Some(Operand::Index(i)) => *i,
            _ => {
                return Err(VMError::InvalidOperand(
                    "JUMP requires an Index operand".to_string(),
                ))
            }
        };
        vm.jump_to(target);
        Ok(Some(format!("Jump to {}", target)))
    }

    /// Build a simple VM with the basic handlers registered.
    fn make_test_vm() -> GenericVM {
        let mut vm = GenericVM::new();
        vm.register_opcode(opcodes::LOAD_CONST, load_const_handler);
        vm.register_opcode(opcodes::STORE_NAME, store_name_handler);
        vm.register_opcode(opcodes::LOAD_NAME, load_name_handler);
        vm.register_opcode(opcodes::ADD, add_handler);
        vm.register_opcode(opcodes::PRINT, print_handler);
        vm.register_opcode(opcodes::HALT, halt_handler);
        vm.register_opcode(opcodes::POP, pop_handler);
        vm.register_opcode(opcodes::JUMP, jump_handler);
        vm
    }

    // ── Basic execution tests ────────────────────────────────────────────

    #[test]
    fn test_basic_execution() {
        // Program: push 10, push 20, add, halt
        // Expected: stack contains [30]
        let mut vm = make_test_vm();
        let code = CodeObject {
            instructions: vec![
                Instruction {
                    opcode: opcodes::LOAD_CONST,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: opcodes::LOAD_CONST,
                    operand: Some(Operand::Index(1)),
                },
                Instruction {
                    opcode: opcodes::ADD,
                    operand: None,
                },
                Instruction {
                    opcode: opcodes::HALT,
                    operand: None,
                },
            ],
            constants: vec![Value::Int(10), Value::Int(20)],
            names: vec![],
        };
        let traces = vm.execute(&code).unwrap();
        assert_eq!(traces.len(), 4);
        assert_eq!(vm.stack.len(), 1);
        match &vm.stack[0] {
            Value::Int(30) => {}
            other => panic!("Expected Int(30), got {:?}", other),
        }
    }

    #[test]
    fn test_print_output() {
        // Program: push 42, print, halt
        let mut vm = make_test_vm();
        let code = CodeObject {
            instructions: vec![
                Instruction {
                    opcode: opcodes::LOAD_CONST,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: opcodes::PRINT,
                    operand: None,
                },
                Instruction {
                    opcode: opcodes::HALT,
                    operand: None,
                },
            ],
            constants: vec![Value::Int(42)],
            names: vec![],
        };
        vm.execute(&code).unwrap();
        assert_eq!(vm.output, vec!["42"]);
    }

    // ── Stack operation tests ────────────────────────────────────────────

    #[test]
    fn test_push_pop() {
        let mut vm = GenericVM::new();
        vm.push(Value::Int(42));
        vm.push(Value::Str("hello".to_string()));
        assert_eq!(vm.stack.len(), 2);

        let top = vm.pop().unwrap();
        match top {
            Value::Str(s) => assert_eq!(s, "hello"),
            _ => panic!("Expected Str"),
        }

        let next = vm.pop().unwrap();
        match next {
            Value::Int(42) => {}
            _ => panic!("Expected Int(42)"),
        }
    }

    #[test]
    fn test_pop_empty_stack() {
        let mut vm = GenericVM::new();
        let result = vm.pop();
        assert!(result.is_err());
        match result.unwrap_err() {
            VMError::StackUnderflow(_) => {}
            other => panic!("Expected StackUnderflow, got {:?}", other),
        }
    }

    #[test]
    fn test_peek() {
        let mut vm = GenericVM::new();
        vm.push(Value::Int(99));
        let val = vm.peek().unwrap();
        match val {
            Value::Int(99) => {}
            _ => panic!("Expected Int(99)"),
        }
        // Stack should still have the value
        assert_eq!(vm.stack.len(), 1);
    }

    #[test]
    fn test_peek_empty_stack() {
        let vm = GenericVM::new();
        assert!(vm.peek().is_err());
    }

    // ── Call stack tests ─────────────────────────────────────────────────

    #[test]
    fn test_call_stack() {
        let mut vm = GenericVM::new();
        let mut frame = HashMap::new();
        frame.insert("x".to_string(), Value::Int(10));
        vm.push_frame(frame).unwrap();
        assert_eq!(vm.call_stack.len(), 1);

        let restored = vm.pop_frame().unwrap();
        match restored.get("x") {
            Some(Value::Int(10)) => {}
            _ => panic!("Expected x=10 in restored frame"),
        }
        assert_eq!(vm.call_stack.len(), 0);
    }

    #[test]
    fn test_max_recursion_depth() {
        let mut vm = GenericVM::new();
        vm.set_max_recursion_depth(Some(2));

        vm.push_frame(HashMap::new()).unwrap();
        vm.push_frame(HashMap::new()).unwrap();

        // Third frame should fail
        let result = vm.push_frame(HashMap::new());
        assert!(result.is_err());
        match result.unwrap_err() {
            VMError::MaxRecursion(_) => {}
            other => panic!("Expected MaxRecursion, got {:?}", other),
        }
    }

    #[test]
    fn test_pop_empty_call_stack() {
        let mut vm = GenericVM::new();
        assert!(vm.pop_frame().is_err());
    }

    // ── Program counter tests ────────────────────────────────────────────

    #[test]
    fn test_advance_pc() {
        let mut vm = GenericVM::new();
        assert_eq!(vm.pc, 0);
        vm.advance_pc();
        assert_eq!(vm.pc, 1);
        vm.advance_pc();
        assert_eq!(vm.pc, 2);
    }

    #[test]
    fn test_jump_to() {
        let mut vm = GenericVM::new();
        vm.jump_to(42);
        assert_eq!(vm.pc, 42);
    }

    #[test]
    fn test_jump_instruction() {
        // Program: push 10, jump to instruction 3 (halt), push 20 (skipped), halt
        let mut vm = make_test_vm();
        let code = CodeObject {
            instructions: vec![
                Instruction {
                    opcode: opcodes::LOAD_CONST,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: opcodes::JUMP,
                    operand: Some(Operand::Index(3)),
                },
                Instruction {
                    opcode: opcodes::LOAD_CONST,
                    operand: Some(Operand::Index(1)),
                },
                Instruction {
                    opcode: opcodes::HALT,
                    operand: None,
                },
            ],
            constants: vec![Value::Int(10), Value::Int(20)],
            names: vec![],
        };
        vm.execute(&code).unwrap();
        // Only 10 should be on the stack (20 was skipped)
        assert_eq!(vm.stack.len(), 1);
        match &vm.stack[0] {
            Value::Int(10) => {}
            other => panic!("Expected Int(10), got {:?}", other),
        }
    }

    // ── Built-in function tests ──────────────────────────────────────────

    #[test]
    fn test_builtins() {
        let mut vm = GenericVM::new();
        vm.register_builtin(
            "double".to_string(),
            BuiltinFunction {
                name: "double".to_string(),
                implementation: |args: &[Value]| match &args[0] {
                    Value::Int(n) => Value::Int(n * 2),
                    _ => Value::Null,
                },
            },
        );

        let builtin = vm.get_builtin("double").unwrap();
        let result = (builtin.implementation)(&[Value::Int(21)]);
        match result {
            Value::Int(42) => {}
            other => panic!("Expected Int(42), got {:?}", other),
        }
    }

    #[test]
    fn test_builtin_not_found() {
        let vm = GenericVM::new();
        assert!(vm.get_builtin("nonexistent").is_none());
    }

    // ── Configuration tests ──────────────────────────────────────────────

    #[test]
    fn test_frozen_vm() {
        let mut vm = GenericVM::new();
        vm.set_frozen(true);
        assert!(vm.is_frozen());
    }

    #[test]
    #[should_panic(expected = "Cannot register opcodes on a frozen VM")]
    fn test_frozen_vm_rejects_registration() {
        let mut vm = GenericVM::new();
        vm.set_frozen(true);
        vm.register_opcode(0x01, halt_handler);
    }

    #[test]
    fn test_max_recursion_depth_config() {
        let mut vm = GenericVM::new();
        assert_eq!(vm.max_recursion_depth(), None);
        vm.set_max_recursion_depth(Some(100));
        assert_eq!(vm.max_recursion_depth(), Some(100));
        vm.set_max_recursion_depth(None);
        assert_eq!(vm.max_recursion_depth(), None);
    }

    // ── Reset tests ──────────────────────────────────────────────────────

    #[test]
    fn test_reset_preserves_handlers() {
        let mut vm = make_test_vm();

        // Execute a program
        let code = CodeObject {
            instructions: vec![
                Instruction {
                    opcode: opcodes::LOAD_CONST,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: opcodes::HALT,
                    operand: None,
                },
            ],
            constants: vec![Value::Int(42)],
            names: vec![],
        };
        vm.execute(&code).unwrap();
        assert_eq!(vm.stack.len(), 1);
        assert!(vm.halted);

        // Reset
        vm.reset();
        assert_eq!(vm.stack.len(), 0);
        assert_eq!(vm.pc, 0);
        assert!(!vm.halted);
        assert!(vm.output.is_empty());

        // Handlers should still work
        let code2 = CodeObject {
            instructions: vec![
                Instruction {
                    opcode: opcodes::LOAD_CONST,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: opcodes::HALT,
                    operand: None,
                },
            ],
            constants: vec![Value::Int(99)],
            names: vec![],
        };
        vm.execute(&code2).unwrap();
        match &vm.stack[0] {
            Value::Int(99) => {}
            other => panic!("Expected Int(99), got {:?}", other),
        }
    }

    // ── Error tests ──────────────────────────────────────────────────────

    #[test]
    fn test_invalid_opcode() {
        let mut vm = GenericVM::new();
        vm.register_opcode(opcodes::HALT, halt_handler);
        let code = CodeObject {
            instructions: vec![Instruction {
                opcode: 0xAA, // unregistered
                operand: None,
            }],
            constants: vec![],
            names: vec![],
        };
        let result = vm.execute(&code);
        assert!(result.is_err());
        match result.unwrap_err() {
            VMError::InvalidOpcode(_) => {}
            other => panic!("Expected InvalidOpcode, got {:?}", other),
        }
    }

    #[test]
    fn test_invalid_operand() {
        let mut vm = make_test_vm();
        let code = CodeObject {
            instructions: vec![Instruction {
                opcode: opcodes::LOAD_CONST,
                operand: None, // missing required operand
            }],
            constants: vec![Value::Int(42)],
            names: vec![],
        };
        let result = vm.execute(&code);
        assert!(result.is_err());
    }

    #[test]
    fn test_undefined_name() {
        let mut vm = make_test_vm();
        let code = CodeObject {
            instructions: vec![Instruction {
                opcode: opcodes::LOAD_NAME,
                operand: Some(Operand::Index(0)),
            }],
            constants: vec![],
            names: vec!["nonexistent".to_string()],
        };
        let result = vm.execute(&code);
        assert!(result.is_err());
        match result.unwrap_err() {
            VMError::UndefinedName(_) => {}
            other => panic!("Expected UndefinedName, got {:?}", other),
        }
    }

    // ── Step-by-step execution tests ─────────────────────────────────────

    #[test]
    fn test_step_by_step() {
        let mut vm = make_test_vm();
        let code = CodeObject {
            instructions: vec![
                Instruction {
                    opcode: opcodes::LOAD_CONST,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: opcodes::LOAD_CONST,
                    operand: Some(Operand::Index(1)),
                },
                Instruction {
                    opcode: opcodes::ADD,
                    operand: None,
                },
                Instruction {
                    opcode: opcodes::HALT,
                    operand: None,
                },
            ],
            constants: vec![Value::Int(5), Value::Int(3)],
            names: vec![],
        };

        // Step 1: LOAD_CONST 5
        let trace1 = vm.step(&code).unwrap();
        assert_eq!(trace1.pc, 0);
        assert_eq!(vm.stack.len(), 1);

        // Step 2: LOAD_CONST 3
        let trace2 = vm.step(&code).unwrap();
        assert_eq!(trace2.pc, 1);
        assert_eq!(vm.stack.len(), 2);

        // Step 3: ADD
        let trace3 = vm.step(&code).unwrap();
        assert_eq!(trace3.pc, 2);
        assert_eq!(vm.stack.len(), 1);
        match &vm.stack[0] {
            Value::Int(8) => {}
            other => panic!("Expected Int(8), got {:?}", other),
        }

        // Step 4: HALT
        let _trace4 = vm.step(&code).unwrap();
        assert!(vm.halted);
    }

    // ── Variable tests ───────────────────────────────────────────────────

    #[test]
    fn test_store_and_load_name() {
        // Program: x = 42; print(x)
        let mut vm = make_test_vm();
        let code = CodeObject {
            instructions: vec![
                Instruction {
                    opcode: opcodes::LOAD_CONST,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: opcodes::STORE_NAME,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: opcodes::LOAD_NAME,
                    operand: Some(Operand::Index(0)),
                },
                Instruction {
                    opcode: opcodes::PRINT,
                    operand: None,
                },
                Instruction {
                    opcode: opcodes::HALT,
                    operand: None,
                },
            ],
            constants: vec![Value::Int(42)],
            names: vec!["x".to_string()],
        };
        vm.execute(&code).unwrap();
        assert_eq!(vm.output, vec!["42"]);
        match vm.variables.get("x") {
            Some(Value::Int(42)) => {}
            other => panic!("Expected x=42, got {:?}", other),
        }
    }

    #[test]
    fn test_inject_globals_merges_and_overwrites_variables() {
        let mut vm = GenericVM::new();
        vm.variables
            .insert("existing".to_string(), Value::Int(1));
        vm.variables.insert(
            "ctx_os".to_string(),
            Value::Str("linux".to_string()),
        );

        let mut globals = HashMap::new();
        globals.insert(
            "ctx_os".to_string(),
            Value::Str("darwin".to_string()),
        );
        globals.insert("answer".to_string(), Value::Int(42));
        vm.inject_globals(globals);

        match vm.variables.get("existing") {
            Some(Value::Int(1)) => {}
            other => panic!("Expected existing=1, got {:?}", other),
        }
        match vm.variables.get("ctx_os") {
            Some(Value::Str(value)) => assert_eq!(value, "darwin"),
            other => panic!("Expected ctx_os=darwin, got {:?}", other),
        }
        match vm.variables.get("answer") {
            Some(Value::Int(42)) => {}
            other => panic!("Expected answer=42, got {:?}", other),
        }
    }

    // ── VMError Display test ─────────────────────────────────────────────

    #[test]
    fn test_error_display() {
        let err = VMError::StackUnderflow("test".to_string());
        assert_eq!(format!("{}", err), "StackUnderflow: test");

        let err = VMError::DivisionByZero("div".to_string());
        assert_eq!(format!("{}", err), "DivisionByZero: div");

        let err = VMError::GenericError("oops".to_string());
        assert_eq!(format!("{}", err), "Error: oops");
    }

    // ── Value Display test ───────────────────────────────────────────────

    #[test]
    fn test_value_display() {
        assert_eq!(format!("{}", Value::Int(42)), "42");
        assert_eq!(format!("{}", Value::Float(3.14)), "3.14");
        assert_eq!(format!("{}", Value::Str("hi".to_string())), "hi");
        assert_eq!(format!("{}", Value::Bool(true)), "true");
        assert_eq!(format!("{}", Value::Null), "null");
    }

    // ── Empty program test ───────────────────────────────────────────────

    #[test]
    fn test_empty_program() {
        let mut vm = make_test_vm();
        let code = CodeObject {
            instructions: vec![],
            constants: vec![],
            names: vec![],
        };
        let traces = vm.execute(&code).unwrap();
        assert_eq!(traces.len(), 0);
    }
}
