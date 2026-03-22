//! # Lisp Compiler -- transforms S-expression ASTs into bytecode.
//!
//! ## How Lisp Compilation Differs from Most Languages
//!
//! Most language compilers map grammar rules to compilation functions: one handler
//! for `if_statement`, another for `assignment`, another for `for_loop`, etc.
//!
//! Lisp is radically different. The grammar has only 6 rules, and they don't
//! distinguish between `(define x 1)`, `(+ 1 2)`, and `(lambda (n) n)`. They
//! are all just "list" nodes. The **compiler** inspects the first element of
//! each list to decide what to do:
//!
//! - First element is `define` -> compile as a definition
//! - First element is `lambda` -> compile as a function
//! - First element is `cond` -> compile as conditional
//! - First element is `+` -> compile as arithmetic
//! - Otherwise -> compile as a function call
//!
//! This is why Lisp is called "homoiconic" -- code and data share the same
//! structure. The compiler's job is to assign **meaning** to that structure.
//!
//! ## Special Forms vs. Function Calls
//!
//! "Special forms" are syntactic constructs that the compiler handles directly.
//! They cannot be implemented as regular functions because they control
//! evaluation order.
//!
//! For example, `(cond (p1 e1) (p2 e2))` should NOT evaluate all branches --
//! only the one whose predicate is true. A regular function call would evaluate
//! all arguments before calling. So `cond` must be a special form.
//!
//! ## Tail Call Optimization
//!
//! A call is in "tail position" when it's the last thing a function does before
//! returning. The compiler tracks tail position and emits `TAIL_CALL` instead of
//! `CALL_FUNCTION` for calls in tail position. This enables unbounded recursion
//! for tail-recursive functions like:
//!
//! ```text
//! (define countdown (lambda (n)
//!   (cond ((eq n 0) 0)
//!         (t (countdown (- n 1))))))  ;; tail call!
//! ```

use lisp_parser::{parse, SExpr, AtomKind, ParseError};
use std::fmt;
use std::collections::HashMap;

// ============================================================================
// Section 1: Opcodes
// ============================================================================
//
// Every bytecode instruction has an opcode -- a number that identifies what
// operation to perform. These are grouped by category using the high nibble:
//
//   0x0_ = Stack operations      (push constants, nil, true)
//   0x1_ = Variable operations   (store/load by name or slot)
//   0x2_ = Arithmetic            (add, sub, mul, div)
//   0x3_ = Comparison            (eq, lt, gt)
//   0x4_ = Control flow          (jump, branch)
//   0x5_ = Functions             (closures, call, tail call, return)
//   0x7_ = Lisp-specific         (cons cells, symbols, predicates)
//   0xA_ = I/O                   (print)
//   0xF_ = VM control            (halt)
// ============================================================================

/// Lisp bytecode opcodes.
///
/// Each value is a single byte (0x00-0xFF). The high nibble groups opcodes
/// by category. Handlers for each opcode are registered with the VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LispOp {
    // -- Stack Operations (0x0_) --

    /// Push a constant from the pool. Operand: pool index.
    LoadConst = 0x01,
    /// Discard top of stack.
    Pop = 0x02,
    /// Push the NIL sentinel (Lisp's empty list / false / nothing).
    LoadNil = 0x03,
    /// Push True (Lisp's `t`).
    LoadTrue = 0x04,

    // -- Variable Operations (0x1_) --

    /// Pop and store in a named (global) variable. Operand: name index.
    StoreName = 0x10,
    /// Push a named variable's value. Operand: name index.
    LoadName = 0x11,
    /// Pop and store in a local slot. Operand: slot index.
    StoreLocal = 0x12,
    /// Push a local slot's value. Operand: slot index.
    LoadLocal = 0x13,

    // -- Arithmetic (0x2_) --

    /// Add two numbers. `a b -> (a + b)`
    Add = 0x20,
    /// Subtract. `a b -> (a - b)`
    Sub = 0x21,
    /// Multiply. `a b -> (a * b)`
    Mul = 0x22,
    /// Integer divide. `a b -> (a / b)`
    Div = 0x23,

    // -- Comparison (0x3_) --

    /// Equality. `a b -> (1 if a == b else 0)`
    CmpEq = 0x30,
    /// Less than. `a b -> (1 if a < b else 0)`
    CmpLt = 0x31,
    /// Greater than. `a b -> (1 if a > b else 0)`
    CmpGt = 0x32,

    // -- Control Flow (0x4_) --

    /// Unconditional jump. Operand: target instruction index.
    Jump = 0x40,
    /// Jump if top is falsy (NIL, 0, false). Operand: target.
    JumpIfFalse = 0x41,
    /// Jump if top is truthy. Operand: target.
    JumpIfTrue = 0x42,

    // -- Functions (0x5_) --

    /// Create a closure from top-of-stack CodeObject. Operand: param count.
    MakeClosure = 0x50,
    /// Call a function. Operand: argc.
    CallFunction = 0x51,
    /// Tail call optimization. Operand: argc.
    TailCall = 0x52,
    /// Return from a function.
    Return = 0x53,

    // -- Lisp-Specific (0x7_) --

    /// Create a cons cell. `cdr car -> address`
    Cons = 0x70,
    /// Get the car (first element) of a cons cell.
    Car = 0x71,
    /// Get the cdr (second element) of a cons cell.
    Cdr = 0x72,
    /// Intern a symbol. Operand: constant pool index of name string.
    MakeSymbol = 0x73,
    /// Test if value is an atom (not a cons cell). `value -> (1 or 0)`
    IsAtom = 0x74,
    /// Test if value is NIL. `value -> (1 or 0)`
    IsNil = 0x75,

    // -- I/O (0xA_) --

    /// Print the top of stack.
    Print = 0xA0,

    // -- VM Control (0xF_) --

    /// Stop execution.
    Halt = 0xFF,
}

// ============================================================================
// Section 2: Core Data Types
// ============================================================================
//
// The compiler produces a CodeObject containing instructions, constants, and
// names. The VM executes these instructions to run the program.
// ============================================================================

/// A single bytecode instruction.
///
/// Each instruction has an opcode (what to do) and an optional operand
/// (additional data, like a constant pool index or jump target).
#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    /// The operation to perform.
    pub opcode: LispOp,
    /// Optional operand (constant index, name index, jump target, arg count).
    pub operand: Option<usize>,
}

impl Instruction {
    /// Create an instruction with no operand.
    pub fn new(opcode: LispOp) -> Self {
        Instruction { opcode, operand: None }
    }

    /// Create an instruction with an operand.
    pub fn with_operand(opcode: LispOp, operand: usize) -> Self {
        Instruction { opcode, operand: Some(operand) }
    }
}

/// A runtime value in the Lisp VM.
///
/// This enum represents all possible values that can live on the stack,
/// in the constant pool, or in variables. It uses Rust's enum to provide
/// type safety instead of Python's duck typing.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// An integer number.
    Integer(i64),
    /// A string value.
    String(std::string::String),
    /// A boolean value (Lisp's `t` is `Bool(true)`).
    Bool(bool),
    /// The NIL sentinel -- Lisp's empty list / false / nothing.
    Nil,
    /// A symbol (interned string with heap address).
    Symbol(std::string::String),
    /// A cons cell address (index into the heap).
    ConsAddr(usize),
    /// A closure address (index into the heap).
    ClosureAddr(usize),
    /// A nested code object (used as a constant for lambda bodies).
    Code(Box<CodeObject>),
}

impl Value {
    /// Check if this value is falsy in Lisp.
    ///
    /// Falsy values: Nil, Integer(0), Bool(false).
    /// Everything else is truthy.
    pub fn is_falsy(&self) -> bool {
        match self {
            Value::Nil => true,
            Value::Bool(false) => true,
            Value::Integer(0) => true,
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Integer(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(true) => write!(f, "t"),
            Value::Bool(false) => write!(f, "nil"),
            Value::Nil => write!(f, "nil"),
            Value::Symbol(s) => write!(f, "{}", s),
            Value::ConsAddr(addr) => write!(f, "<cons @{}>", addr),
            Value::ClosureAddr(addr) => write!(f, "<closure @{}>", addr),
            Value::Code(_) => write!(f, "<code>"),
        }
    }
}

/// A compiled unit of Lisp code.
///
/// Contains the bytecode instructions, a pool of constants (numbers, strings,
/// nested CodeObjects), and a pool of variable names. This is the output of
/// the compiler and the input to the VM.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeObject {
    /// The bytecode instructions to execute.
    pub instructions: Vec<Instruction>,
    /// Constants referenced by LOAD_CONST instructions (by index).
    pub constants: Vec<Value>,
    /// Variable names referenced by STORE_NAME/LOAD_NAME instructions (by index).
    pub names: Vec<std::string::String>,
}

impl CodeObject {
    /// Create a new empty CodeObject.
    pub fn new() -> Self {
        CodeObject {
            instructions: Vec::new(),
            constants: Vec::new(),
            names: Vec::new(),
        }
    }
}

impl Default for CodeObject {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Section 3: Error Type
// ============================================================================

/// An error that occurs during compilation.
#[derive(Debug, Clone, PartialEq)]
pub struct CompileError {
    pub message: std::string::String,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CompileError: {}", self.message)
    }
}

impl std::error::Error for CompileError {}

impl From<ParseError> for CompileError {
    fn from(err: ParseError) -> Self {
        CompileError {
            message: format!("Parse error: {}", err),
        }
    }
}

// ============================================================================
// Section 4: The Compiler
// ============================================================================
//
// The compiler walks the S-expression tree and emits bytecode instructions.
// It maintains:
//
// - An instruction buffer (the bytecode being built)
// - A constant pool (numbers, strings, nested CodeObjects)
// - A name pool (variable names)
// - A scope tracker (for local variables in lambda bodies)
// - Tail position tracking (for tail call optimization)
// ============================================================================

/// Maps arithmetic operator symbols to their opcodes.
fn arithmetic_op(sym: &str) -> Option<LispOp> {
    match sym {
        "+" => Some(LispOp::Add),
        "-" => Some(LispOp::Sub),
        "*" => Some(LispOp::Mul),
        "/" => Some(LispOp::Div),
        _ => None,
    }
}

/// Maps comparison operator symbols to their opcodes.
fn comparison_op(sym: &str) -> Option<LispOp> {
    match sym {
        "=" => Some(LispOp::CmpEq),
        "<" => Some(LispOp::CmpLt),
        ">" => Some(LispOp::CmpGt),
        _ => None,
    }
}

/// The Lisp compiler.
///
/// Transforms S-expression ASTs into bytecode. Handles special forms,
/// function calls, quoted data, and tail call optimization.
pub struct Compiler {
    /// The instruction buffer for the current compilation unit.
    instructions: Vec<Instruction>,
    /// The constant pool.
    constants: Vec<Value>,
    /// The name pool (variable names).
    names: Vec<std::string::String>,
    /// Whether we are currently in tail position.
    tail_position: bool,
    /// Whether we are inside a function body.
    in_function: bool,
    /// Local variable scopes (name -> slot index).
    scopes: Vec<HashMap<std::string::String, usize>>,
}

impl Compiler {
    /// Create a new compiler instance.
    pub fn new() -> Self {
        Compiler {
            instructions: Vec::new(),
            constants: Vec::new(),
            names: Vec::new(),
            tail_position: false,
            in_function: false,
            scopes: Vec::new(),
        }
    }

    // -----------------------------------------------------------------
    // Helper Methods
    // -----------------------------------------------------------------

    /// Add a constant to the pool and return its index.
    fn add_constant(&mut self, value: Value) -> usize {
        // Check if this constant already exists (for deduplication)
        for (i, existing) in self.constants.iter().enumerate() {
            if existing == &value {
                return i;
            }
        }
        let idx = self.constants.len();
        self.constants.push(value);
        idx
    }

    /// Add a name to the pool and return its index.
    fn add_name(&mut self, name: &str) -> usize {
        for (i, existing) in self.names.iter().enumerate() {
            if existing == name {
                return i;
            }
        }
        let idx = self.names.len();
        self.names.push(name.to_string());
        idx
    }

    /// Emit an instruction with no operand.
    fn emit(&mut self, opcode: LispOp) {
        self.instructions.push(Instruction::new(opcode));
    }

    /// Emit an instruction with an operand.
    fn emit_with(&mut self, opcode: LispOp, operand: usize) {
        self.instructions.push(Instruction::with_operand(opcode, operand));
    }

    /// Emit a jump instruction and return the index for later patching.
    fn emit_jump(&mut self, opcode: LispOp) -> usize {
        let idx = self.instructions.len();
        // Placeholder operand -- will be patched later.
        self.instructions.push(Instruction::with_operand(opcode, 0));
        idx
    }

    /// Patch a previously emitted jump instruction with the correct target.
    fn patch_jump(&mut self, jump_idx: usize) {
        let target = self.instructions.len();
        self.instructions[jump_idx].operand = Some(target);
    }

    /// Look up a local variable in the current scope stack.
    fn get_local(&self, name: &str) -> Option<usize> {
        // Only look in the CURRENT (topmost) scope.  Variables from outer
        // scopes are captured by the closure and accessed via LoadName
        // (which reads from the VM's `variables` HashMap), not LoadLocal
        // (which reads from the VM's `locals` Vec).
        //
        // Searching all scopes would incorrectly compile a reference to
        // an outer parameter as a LoadLocal with the outer slot index,
        // but at runtime the inner closure has different locals.
        if let Some(scope) = self.scopes.last() {
            if let Some(&slot) = scope.get(name) {
                return Some(slot);
            }
        }
        None
    }

    // -----------------------------------------------------------------
    // Compilation Entry Points
    // -----------------------------------------------------------------

    /// Compile a program (vector of top-level S-expressions).
    ///
    /// Each S-expression is compiled in order. Intermediate results are
    /// popped; only the last result stays on the stack.
    fn compile_program(&mut self, exprs: &[SExpr]) -> Result<(), CompileError> {
        for (i, expr) in exprs.iter().enumerate() {
            self.compile_sexpr(expr)?;
            // Pop intermediate results, keep the last one.
            if i < exprs.len() - 1 {
                self.emit(LispOp::Pop);
            }
        }
        Ok(())
    }

    /// Compile a single S-expression.
    fn compile_sexpr(&mut self, expr: &SExpr) -> Result<(), CompileError> {
        match expr {
            SExpr::Atom(kind, value) => self.compile_atom(kind, value),
            SExpr::List(elements) => self.compile_list(elements),
            SExpr::DottedPair(elements, last) => {
                // Dotted pairs in code context are unusual; treat as a list
                // with the dot value appended.
                let mut all = elements.clone();
                all.push((*last.clone()));
                self.compile_list(&all)
            }
            SExpr::Quoted(inner) => self.compile_quoted_datum(inner),
        }
    }

    // -----------------------------------------------------------------
    // Atom Compilation
    // -----------------------------------------------------------------

    /// Compile an atom -- a number, symbol, or string literal.
    ///
    /// - Numbers become `LOAD_CONST` with the integer value.
    /// - The special symbols `nil` and `t` become `LOAD_NIL` and `LOAD_TRUE`.
    /// - Other symbols become `LOAD_LOCAL` (if in scope) or `LOAD_NAME` (global).
    /// - Strings become `LOAD_CONST` with the string value.
    fn compile_atom(&mut self, kind: &AtomKind, value: &str) -> Result<(), CompileError> {
        match kind {
            AtomKind::Number => {
                let n: i64 = value.parse().map_err(|_| CompileError {
                    message: format!("Invalid number: {}", value),
                })?;
                let idx = self.add_constant(Value::Integer(n));
                self.emit_with(LispOp::LoadConst, idx);
            }
            AtomKind::String => {
                // Strip surrounding quotes if present
                let s = if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
                    &value[1..value.len() - 1]
                } else {
                    value
                };
                let idx = self.add_constant(Value::String(s.to_string()));
                self.emit_with(LispOp::LoadConst, idx);
            }
            AtomKind::Symbol => {
                match value {
                    "nil" => self.emit(LispOp::LoadNil),
                    "t" => self.emit(LispOp::LoadTrue),
                    _ => {
                        // Check local scope first, then global
                        if let Some(slot) = self.get_local(value) {
                            self.emit_with(LispOp::LoadLocal, slot);
                        } else {
                            let idx = self.add_name(value);
                            self.emit_with(LispOp::LoadName, idx);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------
    // List Compilation -- the workhorse
    // -----------------------------------------------------------------

    /// Compile a list -- dispatches to special forms or function calls.
    ///
    /// A list could be:
    /// - A special form: `(define ...)`, `(lambda ...)`, `(cond ...)`, etc.
    /// - An arithmetic op: `(+ 1 2)`, `(* 3 4)`
    /// - A comparison: `(eq x y)`, `(< a b)`
    /// - A function call: `(f x y)`
    /// - An empty list: `()`
    ///
    /// We inspect the first element to decide which case applies.
    fn compile_list(&mut self, elements: &[SExpr]) -> Result<(), CompileError> {
        if elements.is_empty() {
            // Empty list () -> NIL
            self.emit(LispOp::LoadNil);
            return Ok(());
        }

        // Check if the first element is a symbol (for special form dispatch)
        let first_sym = match &elements[0] {
            SExpr::Atom(AtomKind::Symbol, name) => Some(name.as_str()),
            _ => None,
        };

        if let Some(sym) = first_sym {
            // Dispatch to special form handlers
            match sym {
                "define" => return self.compile_define(elements),
                "lambda" => return self.compile_lambda(elements),
                "cond" => return self.compile_cond(elements),
                "quote" => return self.compile_quote_form(elements),
                "cons" => return self.compile_cons(elements),
                "car" => return self.compile_unary_op(elements, LispOp::Car),
                "cdr" => return self.compile_unary_op(elements, LispOp::Cdr),
                "atom" => return self.compile_unary_op(elements, LispOp::IsAtom),
                "eq" => return self.compile_binary_op(elements, LispOp::CmpEq),
                "print" => return self.compile_unary_op(elements, LispOp::Print),
                "is-nil" => return self.compile_unary_op(elements, LispOp::IsNil),
                _ => {
                    // Check arithmetic and comparison operators
                    if let Some(op) = arithmetic_op(sym) {
                        return self.compile_binary_op(elements, op);
                    }
                    if let Some(op) = comparison_op(sym) {
                        return self.compile_binary_op(elements, op);
                    }
                    // Not a special form -- compile as function call
                    return self.compile_call(elements);
                }
            }
        }

        // First element is not a symbol -- must be a computed call
        // e.g., ((lambda (x) x) 42)
        self.compile_call(elements)
    }

    // -----------------------------------------------------------------
    // Special Form Compilation
    // -----------------------------------------------------------------

    /// Compile `(define name expr)`.
    ///
    /// Evaluates expr and binds the result to name in the global scope.
    /// Define returns NIL (it's a statement-like form, not an expression).
    fn compile_define(&mut self, elements: &[SExpr]) -> Result<(), CompileError> {
        if elements.len() != 3 {
            return Err(CompileError {
                message: format!("define expects 2 arguments, got {}", elements.len() - 1),
            });
        }

        let name = match &elements[1] {
            SExpr::Atom(AtomKind::Symbol, name) => name.clone(),
            _ => return Err(CompileError {
                message: "define name must be a symbol".to_string(),
            }),
        };

        // Compile the value expression (never in tail position)
        let saved_tail = self.tail_position;
        self.tail_position = false;
        self.compile_sexpr(&elements[2])?;
        self.tail_position = saved_tail;

        // Store in global scope
        let idx = self.add_name(&name);
        self.emit_with(LispOp::StoreName, idx);

        // Define returns NIL
        self.emit(LispOp::LoadNil);
        Ok(())
    }

    /// Compile `(lambda (params...) body...)`.
    ///
    /// Creates a closure: compiles the body as a nested CodeObject, pushes it
    /// as a constant, then emits MAKE_CLOSURE.
    ///
    /// The body is compiled in tail position, enabling tail call optimization
    /// for the last expression in the function body.
    fn compile_lambda(&mut self, elements: &[SExpr]) -> Result<(), CompileError> {
        if elements.len() < 3 {
            return Err(CompileError {
                message: "lambda needs params and body".to_string(),
            });
        }

        // Extract parameter names from the parameter list
        let params = match &elements[1] {
            SExpr::List(param_exprs) => {
                let mut names = Vec::new();
                for p in param_exprs {
                    match p {
                        SExpr::Atom(AtomKind::Symbol, name) => names.push(name.clone()),
                        _ => return Err(CompileError {
                            message: "lambda parameter must be a symbol".to_string(),
                        }),
                    }
                }
                names
            }
            _ => return Err(CompileError {
                message: "lambda params must be a list".to_string(),
            }),
        };

        let body_nodes = &elements[2..];

        // Enter a new scope with parameter names
        let mut scope = HashMap::new();
        for (i, name) in params.iter().enumerate() {
            scope.insert(name.clone(), i);
        }
        self.scopes.push(scope);

        // Save current compiler state
        let saved_tail = self.tail_position;
        let saved_in_func = self.in_function;
        let saved_instructions = std::mem::take(&mut self.instructions);
        let saved_constants = std::mem::take(&mut self.constants);
        let saved_names = std::mem::take(&mut self.names);

        self.in_function = true;

        // Compile body expressions -- last one is in tail position
        for (i, body_expr) in body_nodes.iter().enumerate() {
            let is_last = i == body_nodes.len() - 1;
            self.tail_position = is_last;
            self.compile_sexpr(body_expr)?;
            if !is_last {
                self.emit(LispOp::Pop);
            }
        }

        // Emit RETURN at the end of the function body
        self.emit(LispOp::Return);

        // Store param names as a constant in the body (for closure binding)
        if !params.is_empty() {
            let param_names: Vec<Value> = params.iter()
                .map(|n| Value::String(n.clone()))
                .collect();
            // Store as individual string constants that MAKE_CLOSURE can extract
            // We use a convention: the last N constants are the param names
            // Actually, store the count as a marker
            // Simpler: just store each param name
            for name in &params {
                self.constants.push(Value::String(name.clone()));
            }
        }

        // Capture the compiled body
        let body_code = CodeObject {
            instructions: std::mem::take(&mut self.instructions),
            constants: std::mem::take(&mut self.constants),
            names: std::mem::take(&mut self.names),
        };

        // Restore compiler state
        self.instructions = saved_instructions;
        self.constants = saved_constants;
        self.names = saved_names;
        self.tail_position = saved_tail;
        self.in_function = saved_in_func;

        // Exit the scope
        self.scopes.pop();

        // Push the body CodeObject as a constant
        let idx = self.add_constant(Value::Code(Box::new(body_code)));
        self.emit_with(LispOp::LoadConst, idx);

        // Emit MAKE_CLOSURE with param count
        self.emit_with(LispOp::MakeClosure, params.len());

        Ok(())
    }

    /// Compile `(cond (pred1 expr1) (pred2 expr2) ...)`.
    ///
    /// Each clause is a list `(predicate expression)`. We compile:
    ///
    /// ```text
    ///     compile pred1
    ///     JUMP_IF_FALSE -> next_clause
    ///     compile expr1
    ///     JUMP -> end
    /// next_clause:
    ///     compile pred2
    ///     ...
    /// end:
    /// ```
    ///
    /// The special symbol `t` as a predicate means "always true" (the else clause).
    fn compile_cond(&mut self, elements: &[SExpr]) -> Result<(), CompileError> {
        let clauses = &elements[1..]; // Skip 'cond' itself
        let mut end_jumps: Vec<usize> = Vec::new();

        for clause in clauses {
            let clause_parts = match clause {
                SExpr::List(parts) => parts,
                _ => return Err(CompileError {
                    message: "cond clause must be a list".to_string(),
                }),
            };

            if clause_parts.len() < 2 {
                return Err(CompileError {
                    message: "cond clause needs predicate and expression".to_string(),
                });
            }

            let predicate = &clause_parts[0];
            let expression = &clause_parts[clause_parts.len() - 1];

            // Check if predicate is `t` (always true -- the else clause)
            let is_else = matches!(predicate, SExpr::Atom(AtomKind::Symbol, s) if s == "t");

            if is_else {
                // No conditional jump needed -- just compile the expression
                let saved_tail = self.tail_position;
                self.compile_sexpr(expression)?;
                self.tail_position = saved_tail;
            } else {
                // Compile predicate (never in tail position)
                let saved_tail = self.tail_position;
                self.tail_position = false;
                self.compile_sexpr(predicate)?;
                self.tail_position = saved_tail;

                // Jump past this clause if predicate is false
                let false_jump = self.emit_jump(LispOp::JumpIfFalse);

                // Compile expression (inherits tail position)
                let saved_tail2 = self.tail_position;
                self.compile_sexpr(expression)?;
                self.tail_position = saved_tail2;

                // Jump to end after executing this branch
                let end_jump = self.emit_jump(LispOp::Jump);
                end_jumps.push(end_jump);

                // Patch the false jump to here (next clause)
                self.patch_jump(false_jump);
            }
        }

        // If no else clause, push NIL as default
        let has_else = clauses.last().map_or(false, |last| {
            match last {
                SExpr::List(parts) if !parts.is_empty() => {
                    matches!(&parts[0], SExpr::Atom(AtomKind::Symbol, s) if s == "t")
                }
                _ => false,
            }
        });

        if clauses.is_empty() || !has_else {
            self.emit(LispOp::LoadNil);
        }

        // Patch all end jumps to here
        for j in end_jumps {
            self.patch_jump(j);
        }

        Ok(())
    }

    /// Compile `(quote datum)`.
    fn compile_quote_form(&mut self, elements: &[SExpr]) -> Result<(), CompileError> {
        if elements.len() != 2 {
            return Err(CompileError {
                message: "quote takes exactly 1 argument".to_string(),
            });
        }
        self.compile_quoted_datum(&elements[1])
    }

    /// Compile a datum as quoted data (not code).
    ///
    /// - Numbers -> `LOAD_CONST`
    /// - Symbols -> `MAKE_SYMBOL` (intern the symbol)
    /// - Lists -> build a cons chain from right to left
    /// - Quoted forms -> recursive
    ///
    /// For example, `(quote (1 2 3))` builds the cons chain:
    ///
    /// ```text
    /// (1 . (2 . (3 . NIL)))
    /// ```
    ///
    /// By emitting: `LOAD_NIL`, `LOAD_CONST 3`, `CONS`, `LOAD_CONST 2`, `CONS`,
    /// `LOAD_CONST 1`, `CONS`.
    fn compile_quoted_datum(&mut self, expr: &SExpr) -> Result<(), CompileError> {
        match expr {
            SExpr::Atom(AtomKind::Number, value) => {
                let n: i64 = value.parse().map_err(|_| CompileError {
                    message: format!("Invalid number in quote: {}", value),
                })?;
                let idx = self.add_constant(Value::Integer(n));
                self.emit_with(LispOp::LoadConst, idx);
            }
            SExpr::Atom(AtomKind::String, value) => {
                let s = if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
                    &value[1..value.len() - 1]
                } else {
                    value.as_str()
                };
                let idx = self.add_constant(Value::String(s.to_string()));
                self.emit_with(LispOp::LoadConst, idx);
            }
            SExpr::Atom(AtomKind::Symbol, name) => {
                if name == "nil" {
                    self.emit(LispOp::LoadNil);
                } else {
                    let idx = self.add_constant(Value::String(name.clone()));
                    self.emit_with(LispOp::MakeSymbol, idx);
                }
            }
            SExpr::List(elements) => {
                if elements.is_empty() {
                    self.emit(LispOp::LoadNil);
                } else {
                    // Build cons chain from right to left:
                    // (a b c) -> (a . (b . (c . NIL)))
                    // Push NIL, then c, CONS, then b, CONS, then a, CONS
                    self.emit(LispOp::LoadNil);
                    for elem in elements.iter().rev() {
                        self.compile_quoted_datum(elem)?;
                        self.emit(LispOp::Cons);
                    }
                }
            }
            SExpr::DottedPair(elements, last) => {
                // For dotted pairs in quoted data, build the pair directly
                self.compile_quoted_datum(last)?;
                for elem in elements.iter().rev() {
                    self.compile_quoted_datum(elem)?;
                    self.emit(LispOp::Cons);
                }
            }
            SExpr::Quoted(inner) => {
                // Nested quote: ''x
                self.compile_quoted_datum(inner)?;
            }
        }
        Ok(())
    }

    /// Compile `(cons car cdr)`.
    ///
    /// Evaluates both arguments and creates a cons cell.
    /// Stack effect: pushes cdr first, then car, then CONS pops both
    /// and pushes the heap address.
    fn compile_cons(&mut self, elements: &[SExpr]) -> Result<(), CompileError> {
        if elements.len() != 3 {
            return Err(CompileError {
                message: "cons takes exactly 2 arguments".to_string(),
            });
        }
        let saved_tail = self.tail_position;
        self.tail_position = false;

        // Push cdr first (it goes below car on the stack)
        self.compile_sexpr(&elements[2])?;
        // Then car
        self.compile_sexpr(&elements[1])?;

        self.tail_position = saved_tail;
        self.emit(LispOp::Cons);
        Ok(())
    }

    /// Compile a unary operation: `(op arg)`.
    ///
    /// Used for car, cdr, atom, is-nil, print.
    fn compile_unary_op(&mut self, elements: &[SExpr], opcode: LispOp) -> Result<(), CompileError> {
        if elements.len() != 2 {
            return Err(CompileError {
                message: format!("Unary op expects 1 argument, got {}", elements.len() - 1),
            });
        }
        let saved_tail = self.tail_position;
        self.tail_position = false;
        self.compile_sexpr(&elements[1])?;
        self.tail_position = saved_tail;
        self.emit(opcode);
        Ok(())
    }

    /// Compile a binary operation: `(op left right)`.
    ///
    /// Used for arithmetic (+, -, *, /), comparison (eq, <, >).
    fn compile_binary_op(&mut self, elements: &[SExpr], opcode: LispOp) -> Result<(), CompileError> {
        if elements.len() != 3 {
            return Err(CompileError {
                message: format!("Binary op expects 2 arguments, got {}", elements.len() - 1),
            });
        }
        let saved_tail = self.tail_position;
        self.tail_position = false;
        self.compile_sexpr(&elements[1])?;
        self.compile_sexpr(&elements[2])?;
        self.tail_position = saved_tail;
        self.emit(opcode);
        Ok(())
    }

    /// Compile a function call: `(func arg1 arg2 ...)`.
    ///
    /// Pushes arguments left-to-right, then the function, then emits
    /// `CALL_FUNCTION` (or `TAIL_CALL` if in tail position inside a function).
    fn compile_call(&mut self, elements: &[SExpr]) -> Result<(), CompileError> {
        let func_node = &elements[0];
        let arg_nodes = &elements[1..];

        // Compile arguments left-to-right (never in tail position)
        let saved_tail = self.tail_position;
        self.tail_position = false;
        for arg in arg_nodes {
            self.compile_sexpr(arg)?;
        }

        // Compile the function expression
        self.compile_sexpr(func_node)?;
        self.tail_position = saved_tail;

        // Emit CALL_FUNCTION or TAIL_CALL
        if self.tail_position && self.in_function {
            self.emit_with(LispOp::TailCall, arg_nodes.len());
        } else {
            self.emit_with(LispOp::CallFunction, arg_nodes.len());
        }

        Ok(())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Section 5: Public API
// ============================================================================

/// Compile Lisp source code into a CodeObject.
///
/// This is the main entry point. Parses the source, then compiles the AST
/// into bytecode.
///
/// # Examples
///
/// ```
/// use lisp_compiler::{compile, LispOp};
///
/// let code = compile("(+ 1 2)").unwrap();
/// assert!(code.instructions.iter().any(|i| i.opcode == LispOp::Add));
/// ```
pub fn compile(source: &str) -> Result<CodeObject, CompileError> {
    let program = parse(source)?;
    compile_ast(&program)
}

/// Compile a pre-parsed AST into a CodeObject.
pub fn compile_ast(program: &[SExpr]) -> Result<CodeObject, CompileError> {
    let mut compiler = Compiler::new();
    compiler.compile_program(program)?;

    // Emit HALT at the end
    compiler.emit(LispOp::Halt);

    Ok(CodeObject {
        instructions: compiler.instructions,
        constants: compiler.constants,
        names: compiler.names,
    })
}

// ============================================================================
// Section 6: Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: compile and return opcode list.
    fn opcodes(source: &str) -> Vec<LispOp> {
        compile(source)
            .unwrap()
            .instructions
            .iter()
            .map(|i| i.opcode)
            .collect()
    }

    /// Helper: compile and return constants.
    fn constants(source: &str) -> Vec<Value> {
        compile(source).unwrap().constants
    }

    /// Helper: compile and return names.
    fn names(source: &str) -> Vec<std::string::String> {
        compile(source).unwrap().names
    }

    // =====================================================================
    // Atom Compilation
    // =====================================================================

    #[test]
    fn test_number_literal() {
        let code = compile("42").unwrap();
        assert!(code.constants.contains(&Value::Integer(42)));
        assert!(opcodes("42").contains(&LispOp::LoadConst));
    }

    #[test]
    fn test_negative_number() {
        let code = compile("-7").unwrap();
        assert!(code.constants.contains(&Value::Integer(-7)));
    }

    #[test]
    fn test_nil_literal() {
        assert!(opcodes("nil").contains(&LispOp::LoadNil));
    }

    #[test]
    fn test_t_literal() {
        assert!(opcodes("t").contains(&LispOp::LoadTrue));
    }

    #[test]
    fn test_symbol_becomes_load_name() {
        let code = compile("x").unwrap();
        assert!(code.names.contains(&"x".to_string()));
        assert!(opcodes("x").contains(&LispOp::LoadName));
    }

    #[test]
    fn test_string_literal() {
        let code = compile("\"hello\"").unwrap();
        assert!(code.constants.contains(&Value::String("hello".to_string())));
    }

    // =====================================================================
    // Arithmetic Compilation
    // =====================================================================

    #[test]
    fn test_add() {
        let ops = opcodes("(+ 1 2)");
        assert!(ops.contains(&LispOp::LoadConst));
        assert!(ops.contains(&LispOp::Add));
    }

    #[test]
    fn test_subtract() {
        assert!(opcodes("(- 5 3)").contains(&LispOp::Sub));
    }

    #[test]
    fn test_multiply() {
        assert!(opcodes("(* 4 5)").contains(&LispOp::Mul));
    }

    #[test]
    fn test_divide() {
        assert!(opcodes("(/ 10 2)").contains(&LispOp::Div));
    }

    #[test]
    fn test_nested_arithmetic() {
        let ops = opcodes("(+ (* 2 3) 4)");
        assert!(ops.contains(&LispOp::Mul));
        assert!(ops.contains(&LispOp::Add));
    }

    #[test]
    fn test_arithmetic_constants() {
        let code = compile("(+ 1 2)").unwrap();
        assert!(code.constants.contains(&Value::Integer(1)));
        assert!(code.constants.contains(&Value::Integer(2)));
    }

    // =====================================================================
    // Comparison Compilation
    // =====================================================================

    #[test]
    fn test_eq() {
        assert!(opcodes("(eq 1 2)").contains(&LispOp::CmpEq));
    }

    #[test]
    fn test_less_than() {
        assert!(opcodes("(< 1 2)").contains(&LispOp::CmpLt));
    }

    #[test]
    fn test_greater_than() {
        assert!(opcodes("(> 3 2)").contains(&LispOp::CmpGt));
    }

    #[test]
    fn test_equals_sign() {
        assert!(opcodes("(= 1 1)").contains(&LispOp::CmpEq));
    }

    // =====================================================================
    // Define Compilation
    // =====================================================================

    #[test]
    fn test_define_number() {
        let code = compile("(define x 42)").unwrap();
        assert!(code.names.contains(&"x".to_string()));
        assert!(code.constants.contains(&Value::Integer(42)));
        assert!(opcodes("(define x 42)").contains(&LispOp::StoreName));
    }

    #[test]
    fn test_define_pushes_nil() {
        assert!(opcodes("(define x 42)").contains(&LispOp::LoadNil));
    }

    // =====================================================================
    // Cons Cell Compilation
    // =====================================================================

    #[test]
    fn test_cons() {
        assert!(opcodes("(cons 1 2)").contains(&LispOp::Cons));
    }

    #[test]
    fn test_car() {
        assert!(opcodes("(car x)").contains(&LispOp::Car));
    }

    #[test]
    fn test_cdr() {
        assert!(opcodes("(cdr x)").contains(&LispOp::Cdr));
    }

    // =====================================================================
    // Predicate Compilation
    // =====================================================================

    #[test]
    fn test_atom() {
        assert!(opcodes("(atom x)").contains(&LispOp::IsAtom));
    }

    #[test]
    fn test_is_nil() {
        assert!(opcodes("(is-nil x)").contains(&LispOp::IsNil));
    }

    // =====================================================================
    // Quote Compilation
    // =====================================================================

    #[test]
    fn test_quote_number() {
        assert!(opcodes("(quote 42)").contains(&LispOp::LoadConst));
    }

    #[test]
    fn test_quote_symbol() {
        let code = compile("(quote foo)").unwrap();
        assert!(code.constants.contains(&Value::String("foo".to_string())));
        assert!(opcodes("(quote foo)").contains(&LispOp::MakeSymbol));
    }

    #[test]
    fn test_quote_nil() {
        assert!(opcodes("(quote nil)").contains(&LispOp::LoadNil));
    }

    #[test]
    fn test_quote_list() {
        let ops = opcodes("(quote (1 2 3))");
        assert!(ops.contains(&LispOp::LoadNil));
        assert_eq!(ops.iter().filter(|&&op| op == LispOp::Cons).count(), 3);
    }

    #[test]
    fn test_quote_empty_list() {
        assert!(opcodes("(quote ())").contains(&LispOp::LoadNil));
    }

    #[test]
    fn test_shorthand_quote() {
        assert!(opcodes("'foo").contains(&LispOp::MakeSymbol));
    }

    #[test]
    fn test_shorthand_quote_list() {
        let ops = opcodes("'(1 2)");
        assert_eq!(ops.iter().filter(|&&op| op == LispOp::Cons).count(), 2);
    }

    // =====================================================================
    // Cond Compilation
    // =====================================================================

    #[test]
    fn test_cond_emits_jumps() {
        let ops = opcodes("(cond ((eq 1 1) 42) (t 0))");
        assert!(ops.contains(&LispOp::JumpIfFalse));
        assert!(ops.contains(&LispOp::Jump));
    }

    #[test]
    fn test_cond_with_else() {
        let code = compile("(cond (t 42))").unwrap();
        let ops: Vec<_> = code.instructions.iter().map(|i| i.opcode).collect();
        assert!(ops.contains(&LispOp::LoadConst));
    }

    // =====================================================================
    // Lambda Compilation
    // =====================================================================

    #[test]
    fn test_lambda_emits_make_closure() {
        let ops = opcodes("(lambda (x) x)");
        assert!(ops.contains(&LispOp::LoadConst));
        assert!(ops.contains(&LispOp::MakeClosure));
    }

    #[test]
    fn test_lambda_body_is_code_object() {
        let code = compile("(lambda (x) x)").unwrap();
        let body_codes: Vec<_> = code.constants.iter()
            .filter(|c| matches!(c, Value::Code(_)))
            .collect();
        assert_eq!(body_codes.len(), 1);
    }

    #[test]
    fn test_lambda_body_uses_load_local() {
        let code = compile("(lambda (x) x)").unwrap();
        let body = code.constants.iter()
            .find_map(|c| match c { Value::Code(co) => Some(co), _ => None })
            .unwrap();
        let body_ops: Vec<_> = body.instructions.iter().map(|i| i.opcode).collect();
        assert!(body_ops.contains(&LispOp::LoadLocal));
    }

    #[test]
    fn test_lambda_body_has_return() {
        let code = compile("(lambda (x) x)").unwrap();
        let body = code.constants.iter()
            .find_map(|c| match c { Value::Code(co) => Some(co), _ => None })
            .unwrap();
        let body_ops: Vec<_> = body.instructions.iter().map(|i| i.opcode).collect();
        assert!(body_ops.contains(&LispOp::Return));
    }

    #[test]
    fn test_lambda_param_count() {
        let code = compile("(lambda (a b c) a)").unwrap();
        let make_closure: Vec<_> = code.instructions.iter()
            .filter(|i| i.opcode == LispOp::MakeClosure)
            .collect();
        assert_eq!(make_closure.len(), 1);
        assert_eq!(make_closure[0].operand, Some(3));
    }

    // =====================================================================
    // Function Call Compilation
    // =====================================================================

    #[test]
    fn test_simple_call() {
        assert!(opcodes("(f 1 2)").contains(&LispOp::CallFunction));
    }

    #[test]
    fn test_call_arg_count() {
        let code = compile("(f 1 2 3)").unwrap();
        let call: Vec<_> = code.instructions.iter()
            .filter(|i| i.opcode == LispOp::CallFunction)
            .collect();
        assert_eq!(call.len(), 1);
        assert_eq!(call[0].operand, Some(3));
    }

    // =====================================================================
    // Tail Call Compilation
    // =====================================================================

    #[test]
    fn test_tail_call_in_lambda_body() {
        let code = compile("(lambda (n) (f n))").unwrap();
        let body = code.constants.iter()
            .find_map(|c| match c { Value::Code(co) => Some(co), _ => None })
            .unwrap();
        let body_ops: Vec<_> = body.instructions.iter().map(|i| i.opcode).collect();
        assert!(body_ops.contains(&LispOp::TailCall));
        assert!(!body_ops.contains(&LispOp::CallFunction));
    }

    #[test]
    fn test_no_tail_call_at_top_level() {
        let ops = opcodes("(f 1)");
        assert!(ops.contains(&LispOp::CallFunction));
        assert!(!ops.contains(&LispOp::TailCall));
    }

    #[test]
    fn test_no_tail_call_in_args() {
        let code = compile("(lambda (n) (g (f n)))").unwrap();
        let body = code.constants.iter()
            .find_map(|c| match c { Value::Code(co) => Some(co), _ => None })
            .unwrap();
        let body_ops: Vec<_> = body.instructions.iter().map(|i| i.opcode).collect();
        // Outer call (g ...) should be TAIL_CALL
        // Inner call (f n) should be CALL_FUNCTION
        assert!(body_ops.contains(&LispOp::TailCall));
        assert!(body_ops.contains(&LispOp::CallFunction));
    }

    #[test]
    fn test_tail_call_in_cond_branch() {
        let code = compile("(lambda (n) (cond ((eq n 0) 1) (t (f n))))").unwrap();
        let body = code.constants.iter()
            .find_map(|c| match c { Value::Code(co) => Some(co), _ => None })
            .unwrap();
        let body_ops: Vec<_> = body.instructions.iter().map(|i| i.opcode).collect();
        assert!(body_ops.contains(&LispOp::TailCall));
    }

    // =====================================================================
    // Program Compilation
    // =====================================================================

    #[test]
    fn test_multiple_expressions() {
        let ops = opcodes("1 2 3");
        assert_eq!(ops.iter().filter(|&&op| op == LispOp::Pop).count(), 2);
    }

    #[test]
    fn test_define_then_use() {
        let code = compile("(define x 5) x").unwrap();
        assert!(code.names.contains(&"x".to_string()));
        assert!(opcodes("(define x 5) x").contains(&LispOp::StoreName));
        assert!(opcodes("(define x 5) x").contains(&LispOp::LoadName));
    }

    #[test]
    fn test_empty_program() {
        let code = compile("").unwrap();
        assert_eq!(code.instructions.len(), 1);
        assert_eq!(code.instructions[0].opcode, LispOp::Halt);
    }

    #[test]
    fn test_empty_list() {
        assert!(opcodes("()").contains(&LispOp::LoadNil));
    }

    // =====================================================================
    // Print Compilation
    // =====================================================================

    #[test]
    fn test_print_compiles() {
        let ops = opcodes("(print 42)");
        assert!(ops.contains(&LispOp::Print));
        assert!(ops.contains(&LispOp::LoadConst));
    }
}
