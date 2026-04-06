//! # Core types for the register VM
//!
//! This module defines the foundational data types that every other module
//! depends on:
//!
//! * [`VMValue`] — the dynamic value type (integer, float, string, object, …)
//! * [`VMObject`] — a JS-style property bag with hidden-class tracking
//! * [`CodeObject`] — compiled bytecode together with its constant pool
//! * [`RegisterInstruction`] — a single decoded instruction
//! * [`CallFrame`] — the execution state of one function invocation
//! * [`VMResult`] — the observable outcome of running a [`CodeObject`]
//! * [`VMError`] — a runtime error with location information
//!
//! ## Value model
//!
//! The VM uses a **boxed union** value model (Rust `enum`) rather than NaN
//! tagging or pointer tagging.  This keeps the implementation portable and
//! easy to understand at the cost of some heap allocation overhead.
//!
//! ```text
//! VMValue
//! ├── Integer(i64)          — small integers, index values
//! ├── Float(f64)            — floating-point arithmetic result
//! ├── Str(String)           — heap-allocated string
//! ├── Bool(bool)            — true / false
//! ├── Null                  — explicit absence of value
//! ├── Undefined             — uninitialised variable
//! ├── Object(Rc<RefCell>)   — key-value property bag
//! ├── Array(Rc<RefCell>)    — ordered list of VMValues
//! └── Function(CodeObject, Context?)  — first-class function / closure
//! ```

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use crate::scope::Context;

/// Undefined sentinel — distinct from `Null`.
///
/// In JavaScript semantics, `undefined` is the default value of a declared
/// but unassigned variable, while `null` is an explicit empty reference.
/// We represent both as unit-like enum variants in [`VMValue`].
#[derive(Debug, Clone, PartialEq)]
pub struct Undefined;

/// The singleton undefined value, available as a named constant.
pub const UNDEFINED: Undefined = Undefined;

/// `VMValue` — any value the register VM can hold.
///
/// Modelled on the ECMAScript value space so that we can faithfully simulate
/// V8 Ignition semantics. The `Rc<RefCell<…>>` wrappers give us shared
/// mutable ownership without `unsafe` code — multiple registers (or closure
/// environments) can hold references to the same object.
///
/// ## Truthiness
///
/// JavaScript has seven "falsy" values.  [`VMValue::is_truthy`] implements
/// the same rules:
///
/// | Value              | Truthy? |
/// |--------------------|---------|
/// | `false`            | no      |
/// | `null`             | no      |
/// | `undefined`        | no      |
/// | `0` (integer)      | no      |
/// | `0.0` (float)      | no      |
/// | `""` (empty str)   | no      |
/// | everything else    | yes     |
#[derive(Debug, Clone)]
pub enum VMValue {
    /// A 64-bit signed integer.  V8 calls values that fit in 31 bits "SMIs"
    /// (small integers) and stores them unboxed in pointer bits.
    Integer(i64),

    /// A 64-bit IEEE-754 double.  Used for non-integer arithmetic results.
    Float(f64),

    /// A heap-allocated UTF-8 string.
    Str(String),

    /// A boolean.
    Bool(bool),

    /// Explicit null reference (like `null` in JS or `None` in Python).
    Null,

    /// Uninitialised / missing value (like `undefined` in JS).
    Undefined,

    /// A key-value object with hidden-class tracking.
    /// Wrapped in `Rc<RefCell>` so it can be shared across registers and
    /// closure environments without unsafe aliasing.
    Object(Rc<RefCell<VMObject>>),

    /// An ordered, growable sequence of values.
    Array(Rc<RefCell<Vec<VMValue>>>),

    /// A first-class function.  Contains the compiled [`CodeObject`] and an
    /// optional captured lexical environment ([`Context`]).
    Function(Rc<CodeObject>, Option<Rc<RefCell<Context>>>),
}

impl VMValue {
    /// Returns `true` if this value is truthy according to JavaScript semantics.
    ///
    /// ```
    /// use register_vm::types::VMValue;
    ///
    /// assert!( VMValue::Integer(1).is_truthy());
    /// assert!(!VMValue::Integer(0).is_truthy());
    /// assert!(!VMValue::Bool(false).is_truthy());
    /// assert!(!VMValue::Null.is_truthy());
    /// assert!(!VMValue::Undefined.is_truthy());
    /// assert!(!VMValue::Str(String::new()).is_truthy());
    /// assert!( VMValue::Str("hi".into()).is_truthy());
    /// ```
    pub fn is_truthy(&self) -> bool {
        match self {
            VMValue::Bool(false) => false,
            VMValue::Null | VMValue::Undefined => false,
            VMValue::Integer(0) => false,
            VMValue::Float(f) if *f == 0.0 => false,
            VMValue::Str(s) if s.is_empty() => false,
            _ => true,
        }
    }

    /// Returns the type name string as JavaScript's `typeof` operator would.
    pub fn type_name(&self) -> &'static str {
        match self {
            VMValue::Integer(_) | VMValue::Float(_) => "number",
            VMValue::Str(_) => "string",
            VMValue::Bool(_) => "boolean",
            VMValue::Null => "object",       // JS quirk: typeof null === "object"
            VMValue::Undefined => "undefined",
            VMValue::Object(_) => "object",
            VMValue::Array(_) => "object",
            VMValue::Function(_, _) => "function",
        }
    }
}

impl std::fmt::Display for VMValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VMValue::Integer(n) => write!(f, "{}", n),
            VMValue::Float(n) => write!(f, "{}", n),
            VMValue::Str(s) => write!(f, "{}", s),
            VMValue::Bool(b) => write!(f, "{}", b),
            VMValue::Null => write!(f, "null"),
            VMValue::Undefined => write!(f, "undefined"),
            VMValue::Object(_) => write!(f, "[object Object]"),
            VMValue::Array(arr) => {
                let arr = arr.borrow();
                let parts: Vec<String> = arr.iter().map(|v| format!("{}", v)).collect();
                write!(f, "[{}]", parts.join(", "))
            }
            VMValue::Function(co, _) => write!(f, "[Function: {}]", co.name),
        }
    }
}

impl PartialEq for VMValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (VMValue::Integer(a), VMValue::Integer(b)) => a == b,
            (VMValue::Float(a), VMValue::Float(b)) => a == b,
            (VMValue::Integer(a), VMValue::Float(b)) => (*a as f64) == *b,
            (VMValue::Float(a), VMValue::Integer(b)) => *a == (*b as f64),
            (VMValue::Str(a), VMValue::Str(b)) => a == b,
            (VMValue::Bool(a), VMValue::Bool(b)) => a == b,
            (VMValue::Null, VMValue::Null) => true,
            (VMValue::Undefined, VMValue::Undefined) => true,
            // Objects and arrays: identity equality (same Rc pointer)
            (VMValue::Object(a), VMValue::Object(b)) => Rc::ptr_eq(a, b),
            (VMValue::Array(a), VMValue::Array(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

/// `VMObject` — a JavaScript-style object with hidden-class tracking.
///
/// ## Hidden Classes
///
/// In V8, every object is associated with a *hidden class* (also called a
/// *map*) that records the object's shape — which properties it has and in
/// what order.  Objects with the same shape share a hidden class, which lets
/// V8 represent property offsets as integer indices and avoid a hash-map
/// lookup on every property access.
///
/// We model this here with a simple integer `hidden_class_id`.  When a new
/// property is added, [`crate::feedback`] assigns a fresh id.  The feedback
/// vector records which hidden class was observed at each property-load site;
/// if the same class keeps appearing, the load is *monomorphic* and could be
/// optimised.
#[derive(Debug, Clone)]
pub struct VMObject {
    /// Numeric ID of the hidden class currently describing this object's shape.
    pub hidden_class_id: usize,
    /// The actual properties stored in a hash map.
    pub properties: HashMap<String, VMValue>,
}

impl VMObject {
    /// Creates a new, empty object and allocates it a fresh hidden-class id.
    pub fn new() -> Self {
        VMObject {
            hidden_class_id: crate::feedback::next_hidden_class_id(),
            properties: HashMap::new(),
        }
    }

    /// Inserts or updates a property, bumping the hidden class id to reflect
    /// the shape change (adding a new key changes the shape).
    pub fn set_property(&mut self, key: String, value: VMValue) {
        let is_new = !self.properties.contains_key(&key);
        self.properties.insert(key, value);
        if is_new {
            self.hidden_class_id = crate::feedback::next_hidden_class_id();
        }
    }
}

/// `CodeObject` — compiled function bytecode together with its metadata.
///
/// A `CodeObject` is immutable once created: it is the "compiled form" of a
/// function, analogous to a Python code object or a JVM class file's method.
///
/// ```text
/// CodeObject
/// ├── name               — human-readable label (for stack traces)
/// ├── instructions[]     — the bytecode
/// ├── constants[]        — literal values referenced by LDA_CONSTANT
/// ├── names[]            — string names referenced by property/global ops
/// ├── register_count     — how many local registers to allocate per call
/// ├── feedback_slot_count — how many type-feedback slots to allocate
/// └── parameter_count    — how many call arguments this function expects
/// ```
#[derive(Debug, Clone)]
pub struct CodeObject {
    /// Human-readable name of the function (e.g. `"add"`, `"<main>"`).
    pub name: String,

    /// The bytecode instructions making up the function body.
    pub instructions: Vec<RegisterInstruction>,

    /// Constant pool.  `LDA_CONSTANT` indexes into this.
    pub constants: Vec<VMValue>,

    /// Name table.  Property-access and global opcodes index into this.
    pub names: Vec<String>,

    /// Number of general-purpose registers allocated per call frame.
    pub register_count: usize,

    /// Number of type-feedback slots allocated in each call frame's feedback
    /// vector.
    pub feedback_slot_count: usize,

    /// Number of formal parameters this function declares.
    pub parameter_count: usize,
}

/// `RegisterInstruction` — a single decoded bytecode instruction.
///
/// Rather than packing operands into the raw byte stream (as a real bytecode
/// does for compactness), we keep them in a `Vec<i64>` for simplicity.  The
/// compiler-under-test (if any) handles encoding; the VM just reads the
/// pre-decoded form.
#[derive(Debug, Clone)]
pub struct RegisterInstruction {
    /// The opcode byte — one of the constants in [`crate::opcodes`].
    pub opcode: u8,

    /// Variable-length list of operands.  Semantics depend on the opcode.
    pub operands: Vec<i64>,

    /// Optional index into the call frame's feedback vector for type recording.
    pub feedback_slot: Option<usize>,
}

/// `CallFrame` — execution state for one active function invocation.
///
/// A new `CallFrame` is pushed onto the conceptual call stack each time a
/// function is called, and popped when it returns.  We represent the stack
/// as a linked chain of `Box<CallFrame>` references rather than a separate
/// `Vec` so that frames are naturally owned by the VM's execution loop.
///
/// ```text
/// CallFrame (inner function)
///   └── caller_frame: Box<CallFrame (outer function)>
///                         └── caller_frame: None   ← main frame
/// ```
pub struct CallFrame {
    /// The bytecode being executed.
    pub code: Rc<CodeObject>,

    /// Instruction pointer — index of the *next* instruction to execute.
    pub ip: usize,

    /// The *accumulator* register.  Most arithmetic and load operations read
    /// from / write to this single special register, keeping instruction
    /// encoding compact.
    pub accumulator: VMValue,

    /// General-purpose registers, indexed by register number.
    /// Pre-allocated to `code.register_count` entries.
    pub registers: Vec<VMValue>,

    /// Per-site type-feedback slots.  Tracks which types flowed through each
    /// instrumented operation (arithmetic, property loads, call sites).
    pub feedback_vector: Vec<crate::feedback::FeedbackSlot>,

    /// The lexical scope context for this invocation (used by closure ops).
    pub context: Option<Rc<RefCell<Context>>>,

    /// Linked reference to the caller's frame (for stack unwinding / return).
    pub caller_frame: Option<Box<CallFrame>>,
}

/// `VMResult` — the complete observable outcome of executing a [`CodeObject`].
///
/// Rather than having `execute` return only the return value, `VMResult`
/// captures all side effects so tests can verify them without intercepting
/// I/O streams.
pub struct VMResult {
    /// The value the program ultimately returned (accumulator at HALT/RETURN).
    pub return_value: VMValue,

    /// Lines printed by `INTRINSIC_PRINT` during execution.
    pub output: Vec<String>,

    /// If execution terminated due to a runtime error, this holds the error.
    /// `return_value` will be `VMValue::Undefined` in that case.
    pub error: Option<VMError>,
}

/// `VMError` — a runtime error with precise location information.
///
/// When the VM encounters an unrecoverable error (division by zero, type
/// mismatch, stack overflow, unknown opcode), it returns a `VMError` instead
/// of panicking.  This lets the caller display a readable message and decide
/// how to recover.
#[derive(Debug, Clone)]
pub struct VMError {
    /// Human-readable description of what went wrong.
    pub message: String,

    /// The instruction index (`frame.ip - 1`) at which the error occurred.
    pub instruction_index: usize,

    /// The opcode byte of the failing instruction.
    pub opcode: u8,
}

impl std::fmt::Display for VMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VMError at instruction {}: {} (opcode 0x{:02X})",
            self.instruction_index, self.message, self.opcode
        )
    }
}

impl std::error::Error for VMError {}
