//! # Bytecode Compiler — a generic, pluggable AST-to-bytecode compiler.
//!
//! ## What is a compiler?
//!
//! A compiler transforms code from one form to another. In our case, it takes
//! an **Abstract Syntax Tree** (AST) — a tree structure representing the
//! structure of a program — and produces **bytecode** — a flat sequence of
//! instructions that a virtual machine can execute.
//!
//! ## Why "generic"?
//!
//! Just like the `virtual-machine` crate provides a pluggable execution engine,
//! this crate provides a pluggable compilation engine. You register handlers
//! for different AST node types (rule names), and the compiler dispatches to
//! them as it walks the tree. This means the same compiler framework can target
//! different languages just by swapping out the handlers.
//!
//! ## Architecture
//!
//! ```text
//!       AST (tree)                    Bytecode (flat)
//!
//!     [Program]                    ┌─────────────────┐
//!      ├── [Assign]               │ LOAD_CONST  0    │
//!      │    ├── x                 │ STORE_NAME  0    │
//!      │    └── 42                │ LOAD_NAME   0    │
//!      └── [Print]    ──────►     │ PRINT            │
//!           └── [Var]             │ HALT             │
//!                └── x            └─────────────────┘
//! ```
//!
//! The compiler walks the AST top-down, and for each node, looks up the
//! registered handler for that node's rule name. The handler decides what
//! bytecode instructions to emit.
//!
//! ## Scopes
//!
//! When compiling functions, the compiler enters a new **scope**. Each scope
//! has its own instruction list, constants pool, and names list. When the
//! scope is exited, it produces a `CodeObject` that can be stored as a
//! constant in the parent scope. This is how nested functions work.

use std::collections::HashMap;
use virtual_machine::*;

// ─────────────────────────────────────────────────────────────────────────────
// Section 1: AST Types
// ─────────────────────────────────────────────────────────────────────────────

/// An AST node produced by a parser. Each node represents a syntactic construct
/// in the source language (e.g., an assignment, a function call, an if-statement).
///
/// The `rule_name` identifies what kind of construct this is (e.g., "assign",
/// "if_stmt", "function_def"). The `children` are the sub-components.
///
/// ## Example
///
/// The statement `x = 42` might produce:
///
/// ```text
/// ASTNode {
///     rule_name: "assign",
///     children: [
///         Token { token_type: "IDENTIFIER", value: "x" },
///         Token { token_type: "INTEGER", value: "42" },
///     ]
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ASTNode {
    /// The grammar rule that produced this node (e.g., "assign", "if_stmt").
    pub rule_name: String,
    /// The child nodes and tokens that make up this construct.
    pub children: Vec<ASTChild>,
}

/// A child of an AST node: either a sub-node or a terminal token.
///
/// The tree structure of an AST comes from nodes containing other nodes.
/// At the leaves, we find tokens — the actual words/symbols from the source code.
#[derive(Debug, Clone)]
pub enum ASTChild {
    /// A sub-node representing a nested syntactic construct.
    Node(ASTNode),
    /// A terminal token from the source code.
    Token(TokenNode),
}

/// A terminal token — a single meaningful unit from the source code.
///
/// Tokens are produced by the lexer and are the leaves of the AST.
/// For example, the source `x = 42` produces tokens:
/// - `{ token_type: "IDENTIFIER", value: "x" }`
/// - `{ token_type: "EQUALS", value: "=" }`
/// - `{ token_type: "INTEGER", value: "42" }`
#[derive(Debug, Clone)]
pub struct TokenNode {
    /// The category of this token (e.g., "IDENTIFIER", "INTEGER", "PLUS").
    pub token_type: String,
    /// The actual text from the source code.
    pub value: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Section 2: Compile Handler Type
// ─────────────────────────────────────────────────────────────────────────────

/// The type signature for a compilation handler function.
///
/// Each handler receives:
/// - `compiler`: Mutable reference to the compiler (so it can emit instructions)
/// - `node`: The AST node being compiled
///
/// The handler should emit whatever bytecode instructions are needed to
/// implement the semantics of this AST node type.
pub type CompileHandler = fn(compiler: &mut GenericCompiler, node: &ASTNode);

// ─────────────────────────────────────────────────────────────────────────────
// Section 3: Compiler Scope
// ─────────────────────────────────────────────────────────────────────────────

/// A compiler scope represents the compilation state for a single code unit
/// (a function, a class body, or the top-level module).
///
/// When the compiler enters a function definition, it pushes a new scope.
/// All instructions emitted within the function go into this scope's
/// instruction list. When the function definition ends, the scope is
/// converted into a CodeObject.
///
/// ## Why scopes?
///
/// Consider this code:
///
/// ```text
/// def add(a, b):
///     return a + b
/// result = add(1, 2)
/// ```
///
/// The function body `return a + b` needs its own instruction sequence,
/// constants pool, and names list — separate from the module-level code.
/// Scopes provide this isolation.
pub struct CompilerScope {
    /// Instructions emitted within this scope.
    pub instructions: Vec<Instruction>,
    /// Constants used within this scope.
    pub constants: Vec<Value>,
    /// Variable/function names used within this scope.
    pub names: Vec<String>,
    /// Parameter names (for function scopes).
    pub params: Vec<String>,
    /// Local variable names and their slot indices.
    pub locals: HashMap<String, usize>,
}

impl CompilerScope {
    /// Create a new empty scope, optionally with function parameters.
    fn new(params: Option<&[&str]>) -> Self {
        let mut scope = CompilerScope {
            instructions: Vec::new(),
            constants: Vec::new(),
            names: Vec::new(),
            params: Vec::new(),
            locals: HashMap::new(),
        };
        if let Some(param_names) = params {
            for &name in param_names {
                let idx = scope.locals.len();
                scope.locals.insert(name.to_string(), idx);
                scope.params.push(name.to_string());
            }
        }
        scope
    }

    /// Convert this scope into a CodeObject, suitable for execution by the VM.
    fn to_code_object(&self) -> CodeObject {
        CodeObject {
            instructions: self.instructions.clone(),
            constants: self.constants.clone(),
            names: self.names.clone(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Section 4: GenericCompiler
// ─────────────────────────────────────────────────────────────────────────────

/// The GenericCompiler: a pluggable AST-to-bytecode compiler.
///
/// ## How it works
///
/// 1. Register handlers for each AST rule name:
///    ```text
///    compiler.register_rule("assign", compile_assign);
///    compiler.register_rule("if_stmt", compile_if);
///    ```
///
/// 2. Call `compile()` with the root AST node:
///    ```text
///    let code = compiler.compile(&ast, Some(opcodes::HALT));
///    ```
///
/// 3. The compiler walks the tree, dispatching to handlers for each node.
///    Handlers use `emit()`, `add_constant()`, `add_name()`, etc. to
///    produce bytecode.
///
/// ## Jump patching
///
/// Control flow (if/else, loops) requires jump instructions whose targets
/// aren't known when they're first emitted. The compiler supports this via
/// `emit_jump()` (emits a placeholder) and `patch_jump()` (fills in the target).
///
/// ```text
/// let jump_idx = compiler.emit_jump(JUMP_IF_FALSE);  // placeholder target
/// // ... compile the "then" branch ...
/// compiler.patch_jump(jump_idx, None);                // patch to current position
/// ```
pub struct GenericCompiler {
    /// Instructions emitted at the current (top-level) scope.
    pub instructions: Vec<Instruction>,
    /// Constants at the current scope level.
    pub constants: Vec<Value>,
    /// Names at the current scope level.
    pub names: Vec<String>,
    /// The current nested scope (if inside a function definition).
    pub scope: Option<CompilerScope>,
    /// The dispatch table: rule name -> compile handler.
    dispatch: HashMap<String, CompileHandler>,
    /// Stack of saved scopes (for nested function definitions).
    scope_stack: Vec<CompilerScope>,
}

impl GenericCompiler {
    /// Create a new compiler with no registered handlers.
    pub fn new() -> Self {
        GenericCompiler {
            instructions: Vec::new(),
            constants: Vec::new(),
            names: Vec::new(),
            scope: None,
            dispatch: HashMap::new(),
            scope_stack: Vec::new(),
        }
    }

    /// Register a compile handler for a specific AST rule name.
    ///
    /// When the compiler encounters an ASTNode with this rule_name,
    /// it will call this handler to produce bytecode.
    pub fn register_rule(&mut self, rule_name: &str, handler: CompileHandler) {
        self.dispatch.insert(rule_name.to_string(), handler);
    }

    // ── Instruction emission ─────────────────────────────────────────────

    /// Emit a bytecode instruction, returning its index in the instruction list.
    ///
    /// If we're inside a scope (compiling a function), the instruction goes
    /// into the scope's list. Otherwise, it goes into the top-level list.
    pub fn emit(&mut self, opcode: OpCode, operand: Option<Operand>) -> usize {
        let instr = Instruction { opcode, operand };
        let instructions = if let Some(ref mut scope) = self.scope {
            &mut scope.instructions
        } else {
            &mut self.instructions
        };
        let idx = instructions.len();
        instructions.push(instr);
        idx
    }

    /// Emit a jump instruction with a placeholder operand (Index(0)).
    ///
    /// The actual target will be filled in later with `patch_jump()`.
    /// Returns the index of the jump instruction (needed for patching).
    pub fn emit_jump(&mut self, opcode: OpCode) -> usize {
        self.emit(opcode, Some(Operand::Index(0)))
    }

    /// Patch a previously emitted jump instruction with the actual target.
    ///
    /// If `target` is None, patches to the current instruction offset
    /// (i.e., the next instruction to be emitted).
    pub fn patch_jump(&mut self, index: usize, target: Option<usize>) {
        let actual_target = target.unwrap_or_else(|| self.current_offset());
        let instructions = if let Some(ref mut scope) = self.scope {
            &mut scope.instructions
        } else {
            &mut self.instructions
        };
        if let Some(instr) = instructions.get_mut(index) {
            instr.operand = Some(Operand::Index(actual_target));
        }
    }

    /// Get the current instruction offset (index of the next instruction
    /// to be emitted).
    pub fn current_offset(&self) -> usize {
        if let Some(ref scope) = self.scope {
            scope.instructions.len()
        } else {
            self.instructions.len()
        }
    }

    // ── Constants and names ──────────────────────────────────────────────

    /// Add a constant value to the pool, returning its index.
    ///
    /// Constants are deduplicated by position — if you add the same value
    /// twice, it gets two separate entries. (A production compiler would
    /// deduplicate, but simplicity wins here.)
    pub fn add_constant(&mut self, value: Value) -> usize {
        let constants = if let Some(ref mut scope) = self.scope {
            &mut scope.constants
        } else {
            &mut self.constants
        };
        let idx = constants.len();
        constants.push(value);
        idx
    }

    /// Add a name to the names list, returning its index.
    ///
    /// If the name already exists, returns the existing index.
    pub fn add_name(&mut self, name: &str) -> usize {
        let names = if let Some(ref mut scope) = self.scope {
            &mut scope.names
        } else {
            &mut self.names
        };
        // Check if name already exists
        if let Some(idx) = names.iter().position(|n| n == name) {
            return idx;
        }
        let idx = names.len();
        names.push(name.to_string());
        idx
    }

    // ── Scope management ─────────────────────────────────────────────────

    /// Enter a new compilation scope (e.g., for a function body).
    ///
    /// Saves the current state and starts fresh. Any instructions emitted
    /// will go into the new scope until `exit_scope()` is called.
    pub fn enter_scope(&mut self, params: Option<&[&str]>) -> &CompilerScope {
        // Save current state
        let saved = CompilerScope {
            instructions: std::mem::take(&mut self.instructions),
            constants: std::mem::take(&mut self.constants),
            names: std::mem::take(&mut self.names),
            params: Vec::new(),
            locals: HashMap::new(),
        };
        self.scope_stack.push(saved);

        // Create new scope
        self.scope = Some(CompilerScope::new(params));
        self.scope.as_ref().unwrap()
    }

    /// Exit the current scope, returning the scope as a CompilerScope.
    ///
    /// Restores the parent scope's state.
    pub fn exit_scope(&mut self) -> CompilerScope {
        let scope = self.scope.take().expect("Cannot exit scope: no active scope");

        // Restore parent state
        if let Some(parent) = self.scope_stack.pop() {
            self.instructions = parent.instructions;
            self.constants = parent.constants;
            self.names = parent.names;
        }

        scope
    }

    /// Compile an AST node in a fresh scope and return the resulting CodeObject.
    ///
    /// This is useful for compiling function bodies — the function's AST
    /// is compiled in isolation, producing a standalone CodeObject.
    pub fn compile_nested(&mut self, node: &ASTNode) -> CodeObject {
        self.enter_scope(None);
        self.compile_ast_node(node);
        let scope = self.exit_scope();
        scope.to_code_object()
    }

    // ── AST traversal ────────────────────────────────────────────────────

    /// Compile an AST child (either a node or a token).
    ///
    /// If it's a node, dispatch to the registered handler. If it's a token,
    /// call `compile_token` (which is a no-op by default — tokens are usually
    /// handled by their parent node's handler).
    pub fn compile_node(&mut self, child: &ASTChild) {
        match child {
            ASTChild::Node(node) => self.compile_ast_node(node),
            ASTChild::Token(token) => self.compile_token(token),
        }
    }

    /// Compile a token. This is a no-op by default.
    ///
    /// Tokens (like identifiers, numbers, operators) are typically handled
    /// by their parent node's compile handler, not compiled independently.
    /// For example, the "assign" handler reads the identifier token and
    /// the value token directly from its children — the tokens don't need
    /// their own compilation step.
    pub fn compile_token(&mut self, _token: &TokenNode) {
        // No-op: tokens are handled by their parent node's handler.
    }

    /// Internal: compile an AST node by dispatching to its handler.
    fn compile_ast_node(&mut self, node: &ASTNode) {
        if let Some(handler) = self.dispatch.get(&node.rule_name).copied() {
            handler(self, node);
        } else {
            // If no handler is registered for this rule, compile children.
            // This is a sensible default: for "pass-through" nodes that just
            // wrap their children, no explicit handler is needed.
            for child in &node.children {
                self.compile_node(child);
            }
        }
    }

    // ── Main entry point ─────────────────────────────────────────────────

    /// Compile an AST into a CodeObject.
    ///
    /// This is the main entry point. Pass the root AST node and optionally
    /// a halt opcode to append at the end of the program.
    ///
    /// ## Example
    ///
    /// ```text
    /// let code = compiler.compile(&ast, Some(opcodes::HALT));
    /// vm.execute(&code);
    /// ```
    pub fn compile(&mut self, ast: &ASTNode, halt_opcode: Option<OpCode>) -> CodeObject {
        // Compile the AST
        self.compile_ast_node(ast);

        // Optionally append a HALT instruction
        if let Some(halt) = halt_opcode {
            self.emit(halt, None);
        }

        // Build the CodeObject from the current state
        CodeObject {
            instructions: self.instructions.clone(),
            constants: self.constants.clone(),
            names: self.names.clone(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Section 5: Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test compile handlers ────────────────────────────────────────────

    /// Compile handler for "number" nodes: emit LOAD_CONST with the number value.
    fn compile_number(compiler: &mut GenericCompiler, node: &ASTNode) {
        if let Some(ASTChild::Token(token)) = node.children.first() {
            let val: i64 = token.value.parse().unwrap_or(0);
            let idx = compiler.add_constant(Value::Int(val));
            compiler.emit(opcodes::LOAD_CONST, Some(Operand::Index(idx)));
        }
    }

    /// Compile handler for "add" nodes: compile left, compile right, emit ADD.
    fn compile_add(compiler: &mut GenericCompiler, node: &ASTNode) {
        // Compile left child
        if let Some(child) = node.children.first() {
            compiler.compile_node(child);
        }
        // Compile right child
        if let Some(child) = node.children.get(1) {
            compiler.compile_node(child);
        }
        // Emit ADD
        compiler.emit(opcodes::ADD, None);
    }

    /// Compile handler for "assign" nodes: compile value, emit STORE_NAME.
    fn compile_assign(compiler: &mut GenericCompiler, node: &ASTNode) {
        // First child is the name (token), second is the value (node)
        let name = if let Some(ASTChild::Token(token)) = node.children.first() {
            token.value.clone()
        } else {
            return;
        };
        // Compile the value expression
        if let Some(child) = node.children.get(1) {
            compiler.compile_node(child);
        }
        // Emit STORE_NAME
        let name_idx = compiler.add_name(&name);
        compiler.emit(opcodes::STORE_NAME, Some(Operand::Index(name_idx)));
    }

    /// Compile handler for "print" nodes: compile the argument, emit PRINT.
    fn compile_print(compiler: &mut GenericCompiler, node: &ASTNode) {
        if let Some(child) = node.children.first() {
            compiler.compile_node(child);
        }
        compiler.emit(opcodes::PRINT, None);
    }

    /// Compile handler for "var" nodes: emit LOAD_NAME.
    fn compile_var(compiler: &mut GenericCompiler, node: &ASTNode) {
        if let Some(ASTChild::Token(token)) = node.children.first() {
            let name_idx = compiler.add_name(&token.value);
            compiler.emit(opcodes::LOAD_NAME, Some(Operand::Index(name_idx)));
        }
    }

    /// Compile handler for "program" nodes: compile all children.
    fn compile_program(compiler: &mut GenericCompiler, node: &ASTNode) {
        for child in &node.children {
            compiler.compile_node(child);
        }
    }

    /// Build a test compiler with handlers for our mini-language.
    fn make_test_compiler() -> GenericCompiler {
        let mut compiler = GenericCompiler::new();
        compiler.register_rule("program", compile_program);
        compiler.register_rule("number", compile_number);
        compiler.register_rule("add", compile_add);
        compiler.register_rule("assign", compile_assign);
        compiler.register_rule("print", compile_print);
        compiler.register_rule("var", compile_var);
        compiler
    }

    /// Helper to create a token AST child.
    fn token(token_type: &str, value: &str) -> ASTChild {
        ASTChild::Token(TokenNode {
            token_type: token_type.to_string(),
            value: value.to_string(),
        })
    }

    /// Helper to create a node AST child.
    fn node(rule_name: &str, children: Vec<ASTChild>) -> ASTChild {
        ASTChild::Node(ASTNode {
            rule_name: rule_name.to_string(),
            children,
        })
    }

    // ── Basic compilation tests ──────────────────────────────────────────

    #[test]
    fn test_compile_number() {
        let mut compiler = make_test_compiler();
        let ast = ASTNode {
            rule_name: "number".to_string(),
            children: vec![token("INTEGER", "42")],
        };
        let code = compiler.compile(&ast, Some(opcodes::HALT));
        assert_eq!(code.instructions.len(), 2); // LOAD_CONST + HALT
        assert_eq!(code.constants.len(), 1);
        match &code.constants[0] {
            Value::Int(42) => {}
            other => panic!("Expected Int(42), got {:?}", other),
        }
    }

    #[test]
    fn test_compile_addition() {
        // AST: add(number(10), number(20))
        let mut compiler = make_test_compiler();
        let ast = ASTNode {
            rule_name: "add".to_string(),
            children: vec![
                node("number", vec![token("INTEGER", "10")]),
                node("number", vec![token("INTEGER", "20")]),
            ],
        };
        let code = compiler.compile(&ast, Some(opcodes::HALT));
        // LOAD_CONST 0, LOAD_CONST 1, ADD, HALT
        assert_eq!(code.instructions.len(), 4);
        assert_eq!(code.constants.len(), 2);
    }

    #[test]
    fn test_compile_assignment() {
        // AST: assign("x", number(42))
        let mut compiler = make_test_compiler();
        let ast = ASTNode {
            rule_name: "assign".to_string(),
            children: vec![
                token("IDENTIFIER", "x"),
                node("number", vec![token("INTEGER", "42")]),
            ],
        };
        let code = compiler.compile(&ast, None);
        // LOAD_CONST 0, STORE_NAME 0
        assert_eq!(code.instructions.len(), 2);
        assert_eq!(code.names, vec!["x"]);
    }

    #[test]
    fn test_compile_program() {
        // program { assign("x", number(42)), print(var("x")) }
        let mut compiler = make_test_compiler();
        let ast = ASTNode {
            rule_name: "program".to_string(),
            children: vec![
                node(
                    "assign",
                    vec![
                        token("IDENTIFIER", "x"),
                        node("number", vec![token("INTEGER", "42")]),
                    ],
                ),
                node(
                    "print",
                    vec![node("var", vec![token("IDENTIFIER", "x")])],
                ),
            ],
        };
        let code = compiler.compile(&ast, Some(opcodes::HALT));
        // LOAD_CONST, STORE_NAME, LOAD_NAME, PRINT, HALT
        assert_eq!(code.instructions.len(), 5);
    }

    // ── Jump patching tests ──────────────────────────────────────────────

    #[test]
    fn test_emit_and_patch_jump() {
        let mut compiler = GenericCompiler::new();

        // Emit some instructions, then a jump
        compiler.emit(opcodes::LOAD_CONST, Some(Operand::Index(0)));
        let jump_idx = compiler.emit_jump(opcodes::JUMP_IF_FALSE);
        compiler.emit(opcodes::LOAD_CONST, Some(Operand::Index(1)));
        compiler.emit(opcodes::PRINT, None);

        // Patch the jump to point to the current offset (after PRINT)
        compiler.patch_jump(jump_idx, None);

        // The jump instruction should now point to index 4
        match &compiler.instructions[jump_idx].operand {
            Some(Operand::Index(4)) => {}
            other => panic!("Expected Index(4), got {:?}", other),
        }
    }

    #[test]
    fn test_emit_jump_with_explicit_target() {
        let mut compiler = GenericCompiler::new();
        let jump_idx = compiler.emit_jump(opcodes::JUMP);
        compiler.patch_jump(jump_idx, Some(10));

        match &compiler.instructions[jump_idx].operand {
            Some(Operand::Index(10)) => {}
            other => panic!("Expected Index(10), got {:?}", other),
        }
    }

    // ── Constants and names tests ────────────────────────────────────────

    #[test]
    fn test_add_constant() {
        let mut compiler = GenericCompiler::new();
        let idx0 = compiler.add_constant(Value::Int(42));
        let idx1 = compiler.add_constant(Value::Str("hello".to_string()));
        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);
        assert_eq!(compiler.constants.len(), 2);
    }

    #[test]
    fn test_add_name_dedup() {
        let mut compiler = GenericCompiler::new();
        let idx0 = compiler.add_name("x");
        let idx1 = compiler.add_name("y");
        let idx2 = compiler.add_name("x"); // should return existing index
        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);
        assert_eq!(idx2, 0); // deduplicated
        assert_eq!(compiler.names.len(), 2);
    }

    // ── Scope tests ──────────────────────────────────────────────────────

    #[test]
    fn test_enter_exit_scope() {
        let mut compiler = GenericCompiler::new();

        // Emit at top level
        compiler.emit(opcodes::LOAD_CONST, Some(Operand::Index(0)));
        assert_eq!(compiler.instructions.len(), 1);

        // Enter scope
        compiler.enter_scope(Some(&["a", "b"]));
        assert_eq!(compiler.instructions.len(), 0); // fresh scope

        // Emit inside scope
        compiler.emit(opcodes::LOAD_LOCAL, Some(Operand::Index(0)));
        compiler.emit(opcodes::LOAD_LOCAL, Some(Operand::Index(1)));
        compiler.emit(opcodes::ADD, None);

        // Exit scope
        let scope = compiler.exit_scope();
        assert_eq!(scope.instructions.len(), 3);
        assert_eq!(scope.params, vec!["a", "b"]);

        // Parent state restored
        assert_eq!(compiler.instructions.len(), 1);
    }

    #[test]
    fn test_compile_nested() {
        let mut compiler = make_test_compiler();

        // Emit something at top level first
        compiler.add_constant(Value::Int(0));

        // Compile a nested AST
        let nested_ast = ASTNode {
            rule_name: "add".to_string(),
            children: vec![
                node("number", vec![token("INTEGER", "5")]),
                node("number", vec![token("INTEGER", "3")]),
            ],
        };
        let nested_code = compiler.compile_nested(&nested_ast);
        assert_eq!(nested_code.instructions.len(), 3); // LOAD_CONST, LOAD_CONST, ADD
        assert_eq!(nested_code.constants.len(), 2);

        // Top-level state should be intact
        assert_eq!(compiler.constants.len(), 1);
    }

    // ── compile_token no-op test ─────────────────────────────────────────

    #[test]
    fn test_compile_token_is_noop() {
        let mut compiler = GenericCompiler::new();
        let token_node = TokenNode {
            token_type: "INTEGER".to_string(),
            value: "42".to_string(),
        };
        compiler.compile_token(&token_node);
        // Should not emit any instructions
        assert_eq!(compiler.instructions.len(), 0);
    }

    // ── Unregistered rule default behavior test ──────────────────────────

    #[test]
    fn test_unregistered_rule_compiles_children() {
        let mut compiler = make_test_compiler();
        // "wrapper" has no handler, so it should pass through to children
        let ast = ASTNode {
            rule_name: "wrapper".to_string(),
            children: vec![node("number", vec![token("INTEGER", "7")])],
        };
        let code = compiler.compile(&ast, None);
        // Should have compiled the number child
        assert_eq!(code.instructions.len(), 1);
        assert_eq!(code.constants.len(), 1);
    }

    // ── current_offset test ──────────────────────────────────────────────

    #[test]
    fn test_current_offset() {
        let mut compiler = GenericCompiler::new();
        assert_eq!(compiler.current_offset(), 0);
        compiler.emit(opcodes::LOAD_CONST, Some(Operand::Index(0)));
        assert_eq!(compiler.current_offset(), 1);
        compiler.emit(opcodes::ADD, None);
        assert_eq!(compiler.current_offset(), 2);
    }

    // ── Integration test: compile then execute ───────────────────────────

    #[test]
    fn test_compile_and_execute() {
        // This test demonstrates the full pipeline: AST -> bytecode -> execution.
        //
        // Program: x = 10 + 20; print(x)
        //
        // AST:
        //   program
        //     assign("x", add(number(10), number(20)))
        //     print(var("x"))

        let mut compiler = make_test_compiler();
        let ast = ASTNode {
            rule_name: "program".to_string(),
            children: vec![
                node(
                    "assign",
                    vec![
                        token("IDENTIFIER", "x"),
                        node(
                            "add",
                            vec![
                                node("number", vec![token("INTEGER", "10")]),
                                node("number", vec![token("INTEGER", "20")]),
                            ],
                        ),
                    ],
                ),
                node(
                    "print",
                    vec![node("var", vec![token("IDENTIFIER", "x")])],
                ),
            ],
        };
        let code = compiler.compile(&ast, Some(opcodes::HALT));

        // Now execute the compiled bytecode
        use virtual_machine::GenericVM;

        // We need handlers for the VM too
        fn vm_load_const(
            vm: &mut GenericVM,
            instr: &Instruction,
            code: &CodeObject,
        ) -> VMResult<Option<String>> {
            let idx = match &instr.operand {
                Some(Operand::Index(i)) => *i,
                _ => return Err(VMError::InvalidOperand("need index".into())),
            };
            vm.push(code.constants[idx].clone());
            Ok(None)
        }
        fn vm_store_name(
            vm: &mut GenericVM,
            instr: &Instruction,
            code: &CodeObject,
        ) -> VMResult<Option<String>> {
            let idx = match &instr.operand {
                Some(Operand::Index(i)) => *i,
                _ => return Err(VMError::InvalidOperand("need index".into())),
            };
            let val = vm.pop()?;
            vm.variables.insert(code.names[idx].clone(), val);
            Ok(None)
        }
        fn vm_load_name(
            vm: &mut GenericVM,
            instr: &Instruction,
            code: &CodeObject,
        ) -> VMResult<Option<String>> {
            let idx = match &instr.operand {
                Some(Operand::Index(i)) => *i,
                _ => return Err(VMError::InvalidOperand("need index".into())),
            };
            let val = vm
                .variables
                .get(&code.names[idx])
                .cloned()
                .ok_or_else(|| VMError::UndefinedName(code.names[idx].clone()))?;
            vm.push(val);
            Ok(None)
        }
        fn vm_add(
            vm: &mut GenericVM,
            _instr: &Instruction,
            _code: &CodeObject,
        ) -> VMResult<Option<String>> {
            let b = vm.pop()?;
            let a = vm.pop()?;
            match (a, b) {
                (Value::Int(x), Value::Int(y)) => vm.push(Value::Int(x + y)),
                _ => return Err(VMError::TypeError("ADD needs ints".into())),
            }
            Ok(None)
        }
        fn vm_print(
            vm: &mut GenericVM,
            _instr: &Instruction,
            _code: &CodeObject,
        ) -> VMResult<Option<String>> {
            let val = vm.pop()?;
            vm.output.push(format!("{}", val));
            Ok(None)
        }
        fn vm_halt(
            vm: &mut GenericVM,
            _instr: &Instruction,
            _code: &CodeObject,
        ) -> VMResult<Option<String>> {
            vm.halted = true;
            Ok(None)
        }

        let mut vm = GenericVM::new();
        vm.register_opcode(opcodes::LOAD_CONST, vm_load_const);
        vm.register_opcode(opcodes::STORE_NAME, vm_store_name);
        vm.register_opcode(opcodes::LOAD_NAME, vm_load_name);
        vm.register_opcode(opcodes::ADD, vm_add);
        vm.register_opcode(opcodes::PRINT, vm_print);
        vm.register_opcode(opcodes::HALT, vm_halt);

        vm.execute(&code).unwrap();
        assert_eq!(vm.output, vec!["30"]);
    }
}
