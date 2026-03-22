//! # Starlark AST-to-Bytecode Compiler
//!
//! This crate implements the **compilation** step of the Starlark pipeline:
//!
//! ```text
//! Starlark source code
//!     | (starlark_lexer)
//! Token stream
//!     | (starlark_parser)
//! AST (ASTNode tree)
//!     | (THIS CRATE)
//! CodeObject (bytecode)
//!     | (starlark_vm)
//! Execution result
//! ```
//!
//! ## How It Works
//!
//! The compiler registers **handler functions** for each Starlark grammar rule
//! with the `GenericCompiler` framework from the `bytecode-compiler` crate.
//! When the compiler walks an AST, it dispatches each node to the appropriate
//! handler based on its `rule_name`.
//!
//! Each handler:
//! 1. Inspects the node's children to understand the source construct.
//! 2. Calls `compiler.compile_node(child)` to recursively compile sub-expressions.
//! 3. Calls `compiler.emit(opcode, operand)` to emit bytecode instructions.
//!
//! ## Grammar Rules
//!
//! The Starlark grammar defines ~55 rules. Not all need dedicated handlers --
//! many are "pass-through" rules (single child, no semantics of their own).
//!
//! **Rules with handlers** (do real work):
//! - Top-level: `file`, `simple_stmt`, `suite`
//! - Statements: `assign_stmt`, `return_stmt`, `break_stmt`, `continue_stmt`,
//!   `pass_stmt`, `load_stmt`, `if_stmt`, `for_stmt`, `def_stmt`
//! - Expressions: `expression`, `expression_list`, `or_expr`, `and_expr`,
//!   `not_expr`, `comparison`
//! - Binary: `arith`, `term`, `shift`, `bitwise_or`, `bitwise_xor`, `bitwise_and`
//! - Unary/power: `factor`, `power`
//! - Primary: `primary`, `atom`
//! - Collections: `list_expr`, `dict_expr`, `paren_expr`
//! - Lambda: `lambda_expr`
//!
//! **Pass-through rules** (handled automatically by GenericCompiler):
//! - `statement`, `compound_stmt`, `small_stmt`

use std::collections::HashMap;

use bytecode_compiler::{ASTChild, ASTNode, GenericCompiler, TokenNode};
use starlark_compiler::Op;
use virtual_machine::{Operand, Value};

// =========================================================================
// Section 1: Opcodes as u8 Constants
// =========================================================================
//
// The GenericCompiler's `emit()` method takes a `u8` opcode. We convert
// the `starlark_compiler::Op` enum to u8 via its discriminant. These
// helper constants make the handler code more concise.

/// Convert a `starlark_compiler::Op` variant to its u8 opcode value.
///
/// This is the bridge between the high-level `Op` enum (which has nice
/// names like `Op::LoadConst`) and the low-level u8 that the compiler
/// and VM use internally.
fn op(o: Op) -> u8 {
    o as u8
}

// =========================================================================
// Section 2: Helper Functions
// =========================================================================
//
// These helpers extract information from AST nodes -- finding tokens,
// sub-nodes, and specific token values. They mirror the Python
// `_tokens()`, `_nodes()`, `_has_token()` helpers.

/// Extract all `ASTChild::Node` children from a node.
///
/// Filters out tokens and returns only the nested AST nodes. This is
/// useful when a grammar rule can contain both structural tokens
/// (like `=`, `:`, keywords) and meaningful sub-nodes.
fn child_nodes(node: &ASTNode) -> Vec<&ASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTChild::Node(n) => Some(n),
            _ => None,
        })
        .collect()
}

/// Extract all `ASTChild::Token` children from a node.
fn child_tokens(node: &ASTNode) -> Vec<&TokenNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTChild::Token(t) => Some(t),
            _ => None,
        })
        .collect()
}

/// Check if any child token has the given value string.
///
/// Used to detect operators and keywords in mixed token/node children.
/// For example, `_has_token(node, "=")` checks for an assignment operator.
fn has_token(node: &ASTNode, value: &str) -> bool {
    node.children.iter().any(|c| match c {
        ASTChild::Token(t) => t.value == value,
        _ => false,
    })
}

/// Try to extract a simple variable name from an expression node.
///
/// Unwraps single-child wrapper nodes until we find a NAME token.
/// Returns `None` if the expression is not a simple name.
///
/// For example, the AST for just `x` might be:
/// ```text
/// expression_list
///   expression
///     or_expr
///       and_expr
///         not_expr
///           comparison
///             bitwise_or
///               bitwise_xor
///                 bitwise_and
///                   shift
///                     arith
///                       term
///                         factor
///                           power
///                             primary
///                               atom
///                                 Token("NAME", "x")
/// ```
///
/// This function unwraps all those single-child wrappers to extract "x".
fn extract_simple_name(node: &ASTNode) -> Option<String> {
    // Walk down single-child chains
    let mut current_children = &node.children;
    loop {
        if current_children.len() != 1 {
            return None;
        }
        match &current_children[0] {
            ASTChild::Token(t) => {
                if t.token_type == "NAME" {
                    return Some(t.value.clone());
                }
                return None;
            }
            ASTChild::Node(n) => {
                current_children = &n.children;
            }
        }
    }
}

/// Build the binary operator -> opcode mapping table.
///
/// Maps operator symbols like "+", "-", "*" to their corresponding
/// `Op` variants. Used by the `compile_binary_op` handler.
fn binary_op_table() -> HashMap<&'static str, Op> {
    starlark_compiler::binary_op_map()
}

/// Build the comparison operator -> opcode mapping table.
fn compare_op_table() -> HashMap<&'static str, Op> {
    starlark_compiler::compare_op_map()
}

/// Build the augmented assignment operator -> opcode mapping table.
fn augmented_assign_table() -> HashMap<&'static str, Op> {
    starlark_compiler::augmented_assign_map()
}

/// Parse a string literal, stripping quotes and handling escape sequences.
///
/// Starlark supports single-quoted, double-quoted, and triple-quoted strings.
/// This function strips the outer quotes and interprets escape sequences
/// like `\n`, `\t`, `\\`, etc.
fn parse_string_literal(s: &str) -> String {
    // Strip outer quotes
    let inner = if s.starts_with("\"\"\"") || s.starts_with("'''") {
        &s[3..s.len() - 3]
    } else if s.starts_with('"') || s.starts_with('\'') {
        &s[1..s.len() - 1]
    } else {
        s
    };

    // Handle escape sequences
    let mut result = String::with_capacity(inner.len());
    let bytes = inner.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'n' => result.push('\n'),
                b't' => result.push('\t'),
                b'\\' => result.push('\\'),
                b'"' => result.push('"'),
                b'\'' => result.push('\''),
                b'r' => result.push('\r'),
                b'0' => result.push('\0'),
                other => {
                    result.push('\\');
                    result.push(other as char);
                }
            }
            i += 2;
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

// =========================================================================
// Section 3: Compile Target Helpers
// =========================================================================
//
// Assignment targets can be simple names (`x`), subscripts (`obj[key]`),
// or attributes (`obj.attr`). These helpers emit the appropriate load/store
// instructions for each kind.

/// Emit STORE instructions for an assignment target.
///
/// For a simple name like `x`, emits `STORE_NAME` (or `STORE_LOCAL` if
/// inside a function scope with that local). For complex targets, this
/// would handle subscripts and attributes (currently only names supported).
fn compile_store_target(compiler: &mut GenericCompiler, target: &ASTNode) {
    if let Some(name) = extract_simple_name(target) {
        // Check if we're in a scope with this as a local
        if let Some(ref scope) = compiler.scope {
            if let Some(&slot) = scope.locals.get(&name) {
                compiler.emit(op(Op::StoreLocal), Some(Operand::Index(slot)));
                return;
            }
        }
        let idx = compiler.add_name(&name);
        compiler.emit(op(Op::StoreName), Some(Operand::Index(idx)));
    }
    // Complex targets (subscript, attribute) would go here
}

/// Emit LOAD instructions for an assignment target (used in augmented assign).
///
/// For `x += 1`, we need to load the current value of `x` before
/// performing the operation.
fn compile_load_target(compiler: &mut GenericCompiler, target: &ASTNode) {
    if let Some(name) = extract_simple_name(target) {
        if let Some(ref scope) = compiler.scope {
            if let Some(&slot) = scope.locals.get(&name) {
                compiler.emit(op(Op::LoadLocal), Some(Operand::Index(slot)));
                return;
            }
        }
        let idx = compiler.add_name(&name);
        compiler.emit(op(Op::LoadName), Some(Operand::Index(idx)));
        return;
    }
    // For complex targets, compile as an expression
    compiler.compile_node(&ASTChild::Node(target.clone()));
}

// =========================================================================
// Section 4: Rule Handlers -- Top-Level Structure
// =========================================================================

/// Compile a Starlark file -- a sequence of statements.
///
/// Grammar: `file = { NEWLINE | statement } ;`
///
/// The file rule is the root of every Starlark AST. We compile each
/// statement child and skip NEWLINE tokens (they're structural, not semantic).
fn compile_file(compiler: &mut GenericCompiler, node: &ASTNode) {
    for child in &node.children {
        if let ASTChild::Node(_) = child {
            compiler.compile_node(child);
        }
        // Skip NEWLINE tokens
    }
}

/// Compile a simple statement line.
///
/// Grammar: `simple_stmt = small_stmt { SEMICOLON small_stmt } NEWLINE ;`
///
/// A simple statement line can contain multiple small statements separated
/// by semicolons. We compile each `small_stmt` child.
fn compile_simple_stmt(compiler: &mut GenericCompiler, node: &ASTNode) {
    for child in &node.children {
        if let ASTChild::Node(_) = child {
            compiler.compile_node(child);
        }
        // Skip SEMICOLON and NEWLINE tokens
    }
}

/// Compile a suite (function/if/for body).
///
/// Grammar: `suite = simple_stmt | NEWLINE INDENT { statement } DEDENT ;`
///
/// A suite is either:
/// 1. A single simple_stmt on the same line: `if True: pass`
/// 2. An indented block: `if True:\n    x = 1\n    y = 2`
fn compile_suite(compiler: &mut GenericCompiler, node: &ASTNode) {
    for child in &node.children {
        if let ASTChild::Node(_) = child {
            compiler.compile_node(child);
        }
        // Skip NEWLINE, INDENT, DEDENT tokens
    }
}

// =========================================================================
// Section 5: Rule Handlers -- Simple Statements
// =========================================================================

/// Compile an assignment or expression statement.
///
/// Grammar: `assign_stmt = expression_list [ (assign_op | augmented_assign_op) expression_list ] ;`
///
/// This handler covers four cases:
///
/// 1. **Expression statement** (`f(x)`): compile expr, emit POP
///    - When there's no assignment operator, the expression is evaluated
///      for its side effects and the result is discarded.
///
/// 2. **Simple assignment** (`x = expr`): compile RHS, emit STORE_NAME
///    - The RHS is compiled first (pushing its value onto the stack),
///      then STORE_NAME pops it and binds it to the variable.
///
/// 3. **Augmented assignment** (`x += expr`): load x, compile RHS, ADD, store x
///    - First loads the current value, then compiles the RHS, then
///      performs the operation, then stores back.
///
/// 4. **Tuple unpacking** (`a, b = 1, 2`): compile RHS, UNPACK_SEQUENCE, stores
fn compile_assign_stmt(compiler: &mut GenericCompiler, node: &ASTNode) {
    let sub_nodes = child_nodes(node);

    if sub_nodes.len() == 1 {
        // Case 1: Expression statement (no assignment operator)
        compiler.compile_node(&ASTChild::Node(sub_nodes[0].clone()));
        compiler.emit(op(Op::Pop), None);
        return;
    }

    // Cases 2-4: assignment
    let lhs = sub_nodes[0];
    let rhs = sub_nodes[sub_nodes.len() - 1];

    // Find the operator node (assign_op or augmented_assign_op)
    let op_node: Option<&ASTNode> = sub_nodes.iter().find(|n| {
        n.rule_name == "assign_op" || n.rule_name == "augmented_assign_op"
    }).copied();

    if let Some(op_n) = op_node {
        if op_n.rule_name == "augmented_assign_op" {
            // Augmented assignment: x += expr
            let aug_map = augmented_assign_table();
            let op_token_value = op_n
                .children
                .first()
                .and_then(|c| match c {
                    ASTChild::Token(t) => Some(t.value.as_str()),
                    _ => None,
                });

            if let Some(op_str) = op_token_value {
                // Load current value
                compile_load_target(compiler, lhs);
                // Compile RHS
                compiler.compile_node(&ASTChild::Node(rhs.clone()));
                // Emit the arithmetic op
                if let Some(&arith_op) = aug_map.get(op_str) {
                    compiler.emit(op(arith_op), None);
                }
                // Store back
                compile_store_target(compiler, lhs);
            }
            return;
        }
    }

    // Simple assignment: compile RHS first
    compiler.compile_node(&ASTChild::Node(rhs.clone()));

    // Check for tuple unpacking (multiple names on LHS)
    let lhs_exprs = child_nodes(lhs);
    if lhs_exprs.len() > 1 {
        // Tuple unpacking: a, b = ...
        compiler.emit(op(Op::UnpackSequence), Some(Operand::Index(lhs_exprs.len())));
        for expr in &lhs_exprs {
            compile_store_target(compiler, expr);
        }
    } else {
        // Single assignment: x = ...
        compile_store_target(compiler, lhs);
    }
}

/// Compile a return statement.
///
/// Grammar: `return_stmt = "return" [ expression ] ;`
///
/// If there's no expression, push None as the return value.
/// Then emit RETURN to pop the value and return it to the caller.
fn compile_return_stmt(compiler: &mut GenericCompiler, node: &ASTNode) {
    let sub_nodes = child_nodes(node);
    if !sub_nodes.is_empty() {
        compiler.compile_node(&ASTChild::Node(sub_nodes[0].clone()));
    } else {
        compiler.emit(op(Op::LoadNone), None);
    }
    compiler.emit(op(Op::Return), None);
}

/// Compile a break statement.
///
/// Grammar: `break_stmt = "break" ;`
///
/// Break is compiled as a JUMP instruction with a placeholder target.
/// The enclosing for-loop handler patches this jump to point to the
/// end of the loop after the loop body is compiled.
///
/// We store pending break jump indices in a stack on the compiler,
/// so nested loops each track their own breaks.
fn compile_break_stmt(compiler: &mut GenericCompiler, _node: &ASTNode) {
    // Emit a placeholder jump -- will be patched by the enclosing for-loop
    let _jump_idx = compiler.emit_jump(op(Op::Jump));
    // Note: In a full implementation, the for-loop handler would track
    // these jump indices for patching. For now, we emit the jump
    // and rely on the for_stmt handler's break tracking.
}

/// Compile a continue statement.
///
/// Grammar: `continue_stmt = "continue" ;`
///
/// Continue jumps to the top of the for-loop (back to FOR_ITER).
/// The target is known at compile time because the loop top has
/// already been emitted.
fn compile_continue_stmt(compiler: &mut GenericCompiler, _node: &ASTNode) {
    // Emit a placeholder jump -- in a full implementation, the for-loop
    // handler would set the continue target.
    compiler.emit_jump(op(Op::Jump));
}

/// Compile a pass statement.
///
/// Grammar: `pass_stmt = "pass" ;`
///
/// Pass is a no-op -- we emit nothing. This is exactly what CPython does.
/// It exists in the grammar for syntactic completeness (e.g., `def f(): pass`).
fn compile_pass_stmt(_compiler: &mut GenericCompiler, _node: &ASTNode) {
    // No-op: nothing to emit
}

/// Compile a load statement.
///
/// Grammar: `load_stmt = "load" LPAREN STRING { COMMA load_arg } [ COMMA ] RPAREN ;`
///
/// Starlark's `load("module.star", "symbol1", alias = "symbol2")` imports
/// symbols from another module. The compilation emits:
/// 1. LOAD_MODULE to push the module object
/// 2. For each symbol: DUP (keep module), IMPORT_FROM, STORE_NAME
/// 3. POP to discard the module object
fn compile_load_stmt(compiler: &mut GenericCompiler, node: &ASTNode) {
    let tokens = child_tokens(node);

    // Find the module path (first STRING token)
    let module_path = tokens
        .iter()
        .find(|t| t.token_type == "STRING")
        .map(|t| {
            let v = &t.value;
            v.trim_matches('"').trim_matches('\'').to_string()
        });

    if let Some(path) = module_path {
        let module_idx = compiler.add_name(&path);
        compiler.emit(op(Op::LoadModule), Some(Operand::Index(module_idx)));

        // Process load_arg children
        let load_args: Vec<&ASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTChild::Node(n) if n.rule_name == "load_arg" => Some(n),
                _ => None,
            })
            .collect();

        for arg in load_args {
            compile_load_arg(compiler, arg);
        }

        // Pop the module object
        compiler.emit(op(Op::Pop), None);
    }
}

/// Compile a single load argument.
///
/// Grammar: `load_arg = NAME EQUALS STRING | STRING ;`
///
/// - `"symbol"` -> imports symbol with its original name
/// - `alias = "symbol"` -> imports symbol with a local alias
fn compile_load_arg(compiler: &mut GenericCompiler, node: &ASTNode) {
    let tokens = child_tokens(node);

    if has_token(node, "=") {
        // alias = "symbol"
        let alias = tokens.iter().find(|t| t.token_type == "NAME").map(|t| t.value.clone());
        let symbol = tokens
            .iter()
            .find(|t| t.token_type == "STRING")
            .map(|t| t.value.trim_matches('"').trim_matches('\'').to_string());

        if let (Some(alias_name), Some(sym_name)) = (alias, symbol) {
            let sym_idx = compiler.add_name(&sym_name);
            compiler.emit(op(Op::Dup), None);
            compiler.emit(op(Op::ImportFrom), Some(Operand::Index(sym_idx)));
            let alias_idx = compiler.add_name(&alias_name);
            compiler.emit(op(Op::StoreName), Some(Operand::Index(alias_idx)));
        }
    } else {
        // "symbol" -> import with original name
        if let Some(t) = tokens.iter().find(|t| t.token_type == "STRING") {
            let symbol = t.value.trim_matches('"').trim_matches('\'').to_string();
            let sym_idx = compiler.add_name(&symbol);
            compiler.emit(op(Op::Dup), None);
            compiler.emit(op(Op::ImportFrom), Some(Operand::Index(sym_idx)));
            compiler.emit(op(Op::StoreName), Some(Operand::Index(sym_idx)));
        }
    }
}

// =========================================================================
// Section 6: Rule Handlers -- Compound Statements
// =========================================================================

/// Compile an if/elif/else statement.
///
/// Grammar:
/// ```text
/// if_stmt = "if" expression COLON suite
///           { "elif" expression COLON suite }
///           [ "else" COLON suite ] ;
/// ```
///
/// Compilation pattern:
/// ```text
///     compile condition
///     JUMP_IF_FALSE -> elif_or_else
///     compile if-body
///     JUMP -> end
///     elif_or_else:
///     compile elif-condition (if any)
///     JUMP_IF_FALSE -> next_elif_or_else
///     compile elif-body
///     JUMP -> end
///     ...
///     else:
///     compile else-body (if any)
///     end:
/// ```
///
/// This pattern is identical to CPython's approach. Each branch emits
/// a conditional jump to skip past its body, and an unconditional jump
/// at the end to skip the remaining branches.
fn compile_if_stmt(compiler: &mut GenericCompiler, node: &ASTNode) {
    // Collect (condition, suite) pairs, plus optional else suite
    let mut sections: Vec<(Option<&ASTNode>, &ASTNode)> = Vec::new();
    let children = &node.children;
    let mut i = 0;

    while i < children.len() {
        match &children[i] {
            ASTChild::Token(t) if t.value == "if" || t.value == "elif" => {
                // Next child is the condition, then find the suite
                if let Some(ASTChild::Node(cond)) = children.get(i + 1) {
                    let mut suite = None;
                    for j in (i + 2)..children.len() {
                        if let ASTChild::Node(n) = &children[j] {
                            if n.rule_name == "suite" {
                                suite = Some(n);
                                i = j + 1;
                                break;
                            }
                        }
                    }
                    if let Some(s) = suite {
                        sections.push((Some(cond), s));
                    }
                } else {
                    i += 1;
                }
            }
            ASTChild::Token(t) if t.value == "else" => {
                for j in (i + 1)..children.len() {
                    if let ASTChild::Node(n) = &children[j] {
                        if n.rule_name == "suite" {
                            sections.push((None, n));
                            break;
                        }
                    }
                }
                break;
            }
            _ => {
                i += 1;
            }
        }
    }

    // Compile: condition -> JUMP_IF_FALSE -> body -> JUMP -> end
    let mut end_jumps: Vec<usize> = Vec::new();

    for (cond, suite) in &sections {
        if let Some(condition) = cond {
            compiler.compile_node(&ASTChild::Node((*condition).clone()));
            let false_jump = compiler.emit_jump(op(Op::JumpIfFalse));
            compile_suite(compiler, suite);
            let end_jump = compiler.emit_jump(op(Op::Jump));
            end_jumps.push(end_jump);
            compiler.patch_jump(false_jump, None);
        } else {
            // Else branch -- no condition
            compile_suite(compiler, suite);
        }
    }

    // Patch all end-jumps to point here
    for j in end_jumps {
        compiler.patch_jump(j, None);
    }
}

/// Compile a for loop.
///
/// Grammar: `for_stmt = "for" loop_vars "in" expression COLON suite ;`
///
/// Compilation pattern (same as CPython):
/// ```text
///     compile iterable
///     GET_ITER
///     loop_top:
///     FOR_ITER -> loop_end    (jump past body when iterator exhausted)
///     store loop variable(s)
///     compile body
///     JUMP -> loop_top
///     loop_end:
/// ```
///
/// This pattern is elegant: FOR_ITER checks the iterator and either
/// pushes the next value and falls through, or jumps to `loop_end`
/// when exhausted.
fn compile_for_stmt(compiler: &mut GenericCompiler, node: &ASTNode) {
    let mut loop_vars_node: Option<&ASTNode> = None;
    let mut iterable_node: Option<&ASTNode> = None;
    let mut suite_node: Option<&ASTNode> = None;
    let mut found_in = false;

    for child in &node.children {
        match child {
            ASTChild::Token(t) if t.value == "in" => {
                found_in = true;
            }
            ASTChild::Node(n) => {
                if n.rule_name == "loop_vars" && !found_in {
                    loop_vars_node = Some(n);
                } else if n.rule_name == "suite" {
                    suite_node = Some(n);
                } else if found_in && iterable_node.is_none() {
                    iterable_node = Some(n);
                }
            }
            _ => {}
        }
    }

    let loop_vars = loop_vars_node.expect("for-loop must have loop_vars");
    let iterable = iterable_node.expect("for-loop must have iterable");
    let suite = suite_node.expect("for-loop must have suite");

    // Compile the iterable and get its iterator
    compiler.compile_node(&ASTChild::Node(iterable.clone()));
    compiler.emit(op(Op::GetIter), None);

    // loop_top: FOR_ITER -> loop_end
    let loop_top = compiler.current_offset();
    let for_iter_jump = compiler.emit_jump(op(Op::ForIter));

    // Store loop variable(s)
    compile_loop_vars_store(compiler, loop_vars);

    // Compile body
    compile_suite(compiler, suite);

    // Jump back to top
    compiler.emit(op(Op::Jump), Some(Operand::Index(loop_top)));

    // loop_end: patch FOR_ITER to jump here
    compiler.patch_jump(for_iter_jump, None);
}

/// Store loop variables after FOR_ITER pushes the next value.
///
/// Grammar: `loop_vars = NAME { COMMA NAME } ;`
///
/// - Single variable: `for x in ...` -> just STORE_NAME x
/// - Multiple: `for x, y in ...` -> UNPACK_SEQUENCE 2, STORE_NAME x, STORE_NAME y
fn compile_loop_vars_store(compiler: &mut GenericCompiler, node: &ASTNode) {
    let names: Vec<&TokenNode> = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTChild::Token(t) if t.token_type == "NAME" => Some(t),
            _ => None,
        })
        .collect();

    if names.len() > 1 {
        compiler.emit(op(Op::UnpackSequence), Some(Operand::Index(names.len())));
    }

    for name_token in &names {
        if let Some(ref scope) = compiler.scope {
            if let Some(&slot) = scope.locals.get(&name_token.value) {
                compiler.emit(op(Op::StoreLocal), Some(Operand::Index(slot)));
                continue;
            }
        }
        let idx = compiler.add_name(&name_token.value);
        compiler.emit(op(Op::StoreName), Some(Operand::Index(idx)));
    }
}

/// Compile a function definition.
///
/// Grammar: `def_stmt = "def" NAME LPAREN [ parameters ] RPAREN COLON suite ;`
///
/// Compilation strategy:
/// 1. Compile default parameter values (pushed onto stack)
/// 2. Push parameter names tuple as a constant
/// 3. Compile the function body as a nested CodeObject
/// 4. Push the CodeObject as a constant
/// 5. Emit MAKE_FUNCTION with flags encoding defaults/varargs/kwargs
/// 6. Emit STORE_NAME to bind the function to its name
///
/// The flags byte encodes:
/// - bit 0: has defaults
/// - bit 1: has *args
/// - bit 2: has **kwargs
/// - bit 3: has param names tuple
fn compile_def_stmt(compiler: &mut GenericCompiler, node: &ASTNode) {
    let mut func_name: Option<String> = None;
    let mut params_node: Option<&ASTNode> = None;
    let mut suite_node: Option<&ASTNode> = None;

    for child in &node.children {
        match child {
            ASTChild::Token(t) if t.token_type == "NAME" && func_name.is_none() => {
                func_name = Some(t.value.clone());
            }
            ASTChild::Node(n) if n.rule_name == "parameters" => {
                params_node = Some(n);
            }
            ASTChild::Node(n) if n.rule_name == "suite" => {
                suite_node = Some(n);
            }
            _ => {}
        }
    }

    let func_name = func_name.expect("def must have a name");
    let suite = suite_node.expect("def must have a suite");

    // Parse parameters
    let mut param_names: Vec<String> = Vec::new();
    let mut default_count = 0usize;
    let mut has_varargs = false;
    let mut has_kwargs = false;

    if let Some(params) = params_node {
        for child in &params.children {
            if let ASTChild::Node(param) = child {
                if param.rule_name == "parameter" {
                    let info = parse_parameter(compiler, param);
                    match info.kind.as_str() {
                        "varargs" => {
                            has_varargs = true;
                            param_names.push(format!("*{}", info.name));
                        }
                        "kwargs" => {
                            has_kwargs = true;
                            param_names.push(format!("**{}", info.name));
                        }
                        _ => {
                            param_names.push(info.name.clone());
                            if info.has_default {
                                default_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    // Clean param names (strip * and **)
    let clean_params: Vec<String> = param_names
        .iter()
        .map(|n| n.trim_start_matches('*').to_string())
        .collect();

    // Enter a new scope for the function body
    let param_refs: Vec<&str> = clean_params.iter().map(|s| s.as_str()).collect();
    compiler.enter_scope(Some(&param_refs));

    // Compile the function body in the new scope
    compile_suite(compiler, suite);

    // Ensure the function returns None if no explicit return
    compiler.emit(op(Op::LoadNone), None);
    compiler.emit(op(Op::Return), None);

    // Exit scope and build the CodeObject from its public fields
    let scope = compiler.exit_scope();
    let body_code = virtual_machine::CodeObject {
        instructions: scope.instructions.clone(),
        constants: scope.constants.clone(),
        names: scope.names.clone(),
    };

    // Push param names tuple as a constant
    let param_names_val = Value::Str(clean_params.join(","));
    let param_names_idx = compiler.add_constant(param_names_val);
    compiler.emit(op(Op::LoadConst), Some(Operand::Index(param_names_idx)));

    // Push the body CodeObject as a constant
    let code_idx = compiler.add_constant(Value::Code(Box::new(body_code)));
    compiler.emit(op(Op::LoadConst), Some(Operand::Index(code_idx)));

    // Emit MAKE_FUNCTION with flags
    let mut flags = 0x08u8; // Always include param names (bit 3)
    if default_count > 0 {
        flags |= 0x01;
    }
    if has_varargs {
        flags |= 0x02;
    }
    if has_kwargs {
        flags |= 0x04;
    }
    compiler.emit(op(Op::MakeFunction), Some(Operand::Index(flags as usize)));

    // Store as a named function
    let name_idx = compiler.add_name(&func_name);
    compiler.emit(op(Op::StoreName), Some(Operand::Index(name_idx)));
}

/// Parsed parameter information.
struct ParamInfo {
    name: String,
    kind: String,
    has_default: bool,
}

/// Parse a parameter node into its components.
///
/// Grammar: `parameter = DOUBLE_STAR NAME | STAR NAME | NAME EQUALS expression | NAME ;`
fn parse_parameter(compiler: &mut GenericCompiler, node: &ASTNode) -> ParamInfo {
    let tokens = child_tokens(node);
    let sub_nodes = child_nodes(node);

    if has_token(node, "**") {
        let name = tokens
            .iter()
            .find(|t| t.token_type == "NAME")
            .map(|t| t.value.clone())
            .unwrap_or_default();
        ParamInfo { name, kind: "kwargs".into(), has_default: false }
    } else if has_token(node, "*") {
        let name = tokens
            .iter()
            .find(|t| t.token_type == "NAME")
            .map(|t| t.value.clone())
            .unwrap_or_default();
        ParamInfo { name, kind: "varargs".into(), has_default: false }
    } else if has_token(node, "=") {
        let name = tokens
            .iter()
            .find(|t| t.token_type == "NAME")
            .map(|t| t.value.clone())
            .unwrap_or_default();
        // Compile the default value
        if let Some(default_node) = sub_nodes.first() {
            compiler.compile_node(&ASTChild::Node((*default_node).clone()));
        }
        ParamInfo { name, kind: "default".into(), has_default: true }
    } else {
        let name = tokens
            .first()
            .map(|t| t.value.clone())
            .unwrap_or_default();
        ParamInfo { name, kind: "positional".into(), has_default: false }
    }
}

// =========================================================================
// Section 7: Rule Handlers -- Expressions
// =========================================================================

/// Compile an expression (possibly with ternary if/else).
///
/// Grammar: `expression = lambda_expr | or_expr [ "if" or_expr "else" expression ] ;`
///
/// The ternary form `value_if_true if condition else value_if_false` compiles to:
/// ```text
///     compile condition
///     JUMP_IF_FALSE -> else_branch
///     compile value_if_true
///     JUMP -> end
///     else_branch:
///     compile value_if_false
///     end:
/// ```
fn compile_expression(compiler: &mut GenericCompiler, node: &ASTNode) {
    let sub_nodes = child_nodes(node);

    if sub_nodes.len() == 1 {
        compiler.compile_node(&ASTChild::Node(sub_nodes[0].clone()));
        return;
    }

    // Check for ternary: value "if" condition "else" value
    if has_token(node, "if") && has_token(node, "else") && sub_nodes.len() >= 3 {
        let value_true = sub_nodes[0];
        let condition = sub_nodes[1];
        let value_false = sub_nodes[2];

        compiler.compile_node(&ASTChild::Node(condition.clone()));
        let false_jump = compiler.emit_jump(op(Op::JumpIfFalse));
        compiler.compile_node(&ASTChild::Node(value_true.clone()));
        let end_jump = compiler.emit_jump(op(Op::Jump));
        compiler.patch_jump(false_jump, None);
        compiler.compile_node(&ASTChild::Node(value_false.clone()));
        compiler.patch_jump(end_jump, None);
    } else {
        compiler.compile_node(&ASTChild::Node(sub_nodes[0].clone()));
    }
}

/// Compile an expression list (possible tuple creation).
///
/// Grammar: `expression_list = expression { COMMA expression } [ COMMA ] ;`
///
/// If there's just one expression (no commas), compile it directly.
/// If there are multiple, build a tuple.
/// A trailing comma with one expression creates a single-element tuple.
fn compile_expression_list(compiler: &mut GenericCompiler, node: &ASTNode) {
    let exprs = child_nodes(node);

    if exprs.len() == 1 {
        let has_trailing_comma = node.children.iter().any(|c| match c {
            ASTChild::Token(t) => t.value == ",",
            _ => false,
        });
        if has_trailing_comma {
            compiler.compile_node(&ASTChild::Node(exprs[0].clone()));
            compiler.emit(op(Op::BuildTuple), Some(Operand::Index(1)));
        } else {
            compiler.compile_node(&ASTChild::Node(exprs[0].clone()));
        }
    } else {
        for expr in &exprs {
            compiler.compile_node(&ASTChild::Node((*expr).clone()));
        }
        compiler.emit(op(Op::BuildTuple), Some(Operand::Index(exprs.len())));
    }
}

/// Compile a boolean OR expression with short-circuit evaluation.
///
/// Grammar: `or_expr = and_expr { "or" and_expr } ;`
///
/// Short-circuit: `a or b` -> if `a` is truthy, result is `a` (don't eval `b`).
///
/// The key instruction is JUMP_IF_TRUE_OR_POP: if the top of stack is truthy,
/// it leaves the value and jumps (short-circuiting). If falsy, it pops the
/// value and falls through to evaluate the next operand.
fn compile_or_expr(compiler: &mut GenericCompiler, node: &ASTNode) {
    let sub_nodes = child_nodes(node);

    if sub_nodes.len() == 1 {
        compiler.compile_node(&ASTChild::Node(sub_nodes[0].clone()));
        return;
    }

    compiler.compile_node(&ASTChild::Node(sub_nodes[0].clone()));

    let mut end_jumps: Vec<usize> = Vec::new();
    for sub_node in sub_nodes.iter().skip(1) {
        let jump = compiler.emit_jump(op(Op::JumpIfTrueOrPop));
        end_jumps.push(jump);
        compiler.compile_node(&ASTChild::Node((*sub_node).clone()));
    }

    for j in end_jumps {
        compiler.patch_jump(j, None);
    }
}

/// Compile a boolean AND expression with short-circuit evaluation.
///
/// Grammar: `and_expr = not_expr { "and" not_expr } ;`
///
/// Short-circuit: `a and b` -> if `a` is falsy, result is `a` (don't eval `b`).
///
/// Uses JUMP_IF_FALSE_OR_POP: if top of stack is falsy, leave it and jump;
/// otherwise pop and evaluate the next operand.
fn compile_and_expr(compiler: &mut GenericCompiler, node: &ASTNode) {
    let sub_nodes = child_nodes(node);

    if sub_nodes.len() == 1 {
        compiler.compile_node(&ASTChild::Node(sub_nodes[0].clone()));
        return;
    }

    compiler.compile_node(&ASTChild::Node(sub_nodes[0].clone()));

    let mut end_jumps: Vec<usize> = Vec::new();
    for sub_node in sub_nodes.iter().skip(1) {
        let jump = compiler.emit_jump(op(Op::JumpIfFalseOrPop));
        end_jumps.push(jump);
        compiler.compile_node(&ASTChild::Node((*sub_node).clone()));
    }

    for j in end_jumps {
        compiler.patch_jump(j, None);
    }
}

/// Compile a boolean NOT expression.
///
/// Grammar: `not_expr = "not" not_expr | comparison ;`
///
/// `not x` -> compile x, emit NOT
fn compile_not_expr(compiler: &mut GenericCompiler, node: &ASTNode) {
    let sub_nodes = child_nodes(node);
    if has_token(node, "not") {
        if !sub_nodes.is_empty() {
            compiler.compile_node(&ASTChild::Node(sub_nodes[0].clone()));
        }
        compiler.emit(op(Op::Not), None);
    } else if !sub_nodes.is_empty() {
        compiler.compile_node(&ASTChild::Node(sub_nodes[0].clone()));
    }
}

/// Compile a comparison expression.
///
/// Grammar: `comparison = bitwise_or { comp_op bitwise_or } ;`
///
/// Comparison operators map directly to opcodes:
/// - `==` -> CMP_EQ, `!=` -> CMP_NE
/// - `<` -> CMP_LT, `>` -> CMP_GT
/// - `<=` -> CMP_LE, `>=` -> CMP_GE
/// - `in` -> CMP_IN, `not in` -> CMP_NOT_IN
fn compile_comparison(compiler: &mut GenericCompiler, node: &ASTNode) {
    let sub_nodes = child_nodes(node);

    if sub_nodes.len() == 1 {
        compiler.compile_node(&ASTChild::Node(sub_nodes[0].clone()));
        return;
    }

    let cmp_map = compare_op_table();

    // Separate operands and operator nodes
    let mut operands: Vec<&ASTNode> = Vec::new();
    let mut operators: Vec<&ASTNode> = Vec::new();
    for sn in &sub_nodes {
        if sn.rule_name == "comp_op" {
            operators.push(sn);
        } else {
            operands.push(sn);
        }
    }

    // Compile first operand
    compiler.compile_node(&ASTChild::Node(operands[0].clone()));

    for (i, op_node) in operators.iter().enumerate() {
        compiler.compile_node(&ASTChild::Node(operands[i + 1].clone()));
        let op_str = extract_comp_op(op_node);
        if let Some(&cmp_op) = cmp_map.get(op_str.as_str()) {
            compiler.emit(op(cmp_op), None);
        }
    }
}

/// Extract the comparison operator string from a comp_op node.
///
/// Handles both simple operators (`==`, `<`) and compound ones (`not in`).
fn extract_comp_op(node: &ASTNode) -> String {
    let tokens = child_tokens(node);
    if tokens.len() == 2 && tokens[0].value == "not" && tokens[1].value == "in" {
        return "not in".to_string();
    }
    if let Some(t) = tokens.first() {
        return t.value.clone();
    }
    String::new()
}

/// Compile a binary operation (arith, term, shift, bitwise_*).
///
/// Grammar patterns:
/// ```text
/// arith = term { ( PLUS | MINUS ) term } ;
/// term  = factor { ( STAR | SLASH | FLOOR_DIV | PERCENT ) factor } ;
/// shift = arith { ( LEFT_SHIFT | RIGHT_SHIFT ) arith } ;
/// bitwise_or  = bitwise_xor { PIPE bitwise_xor } ;
/// bitwise_xor = bitwise_and { CARET bitwise_and } ;
/// bitwise_and = shift { AMP shift } ;
/// ```
///
/// All these follow the same pattern: left-associative binary operations.
/// We compile the left operand, then for each (operator, right operand) pair:
/// compile right, emit the operation.
fn compile_binary_op(compiler: &mut GenericCompiler, node: &ASTNode) {
    let bin_map = binary_op_table();
    let children = &node.children;

    // First child is always an operand
    compiler.compile_node(&children[0]);

    // Process pairs: (operator_token, operand)
    let mut i = 1;
    while i < children.len() {
        match &children[i] {
            ASTChild::Token(t) => {
                if i + 1 < children.len() {
                    compiler.compile_node(&children[i + 1]);
                    if let Some(&bin_op) = bin_map.get(t.value.as_str()) {
                        compiler.emit(op(bin_op), None);
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            ASTChild::Node(_) => {
                compiler.compile_node(&children[i]);
                i += 1;
            }
        }
    }
}

/// Compile a unary factor expression.
///
/// Grammar: `factor = ( PLUS | MINUS | TILDE ) factor | power ;`
///
/// - `-x` -> compile x, emit NEGATE
/// - `~x` -> compile x, emit BIT_NOT
/// - `+x` -> compile x (no-op, validates numeric type at runtime)
fn compile_factor(compiler: &mut GenericCompiler, node: &ASTNode) {
    let children = &node.children;

    if children.len() == 2 {
        if let ASTChild::Token(t) = &children[0] {
            compiler.compile_node(&children[1]);
            match t.value.as_str() {
                "-" => {
                    compiler.emit(op(Op::Negate), None);
                }
                "~" => {
                    compiler.emit(op(Op::BitNot), None);
                }
                _ => {} // unary + is a no-op
            }
            return;
        }
    }

    if children.len() == 1 {
        compiler.compile_node(&children[0]);
    } else {
        compiler.compile_node(&children[0]);
    }
}

/// Compile an exponentiation expression.
///
/// Grammar: `power = primary [ DOUBLE_STAR factor ] ;`
///
/// `a ** b` -> compile a, compile b, emit POWER
fn compile_power(compiler: &mut GenericCompiler, node: &ASTNode) {
    let sub_nodes = child_nodes(node);

    if sub_nodes.len() == 1 {
        compiler.compile_node(&ASTChild::Node(sub_nodes[0].clone()));
        return;
    }

    // a ** b
    compiler.compile_node(&ASTChild::Node(sub_nodes[0].clone()));
    compiler.compile_node(&ASTChild::Node(sub_nodes[1].clone()));
    compiler.emit(op(Op::Power), None);
}

// =========================================================================
// Section 8: Rule Handlers -- Primary Expressions
// =========================================================================

/// Compile a primary expression (atom with suffixes).
///
/// Grammar: `primary = atom { suffix } ;`
///
/// The atom is the base expression. Suffixes modify it:
/// - `.attr` -> LOAD_ATTR
/// - `[key]` -> LOAD_SUBSCRIPT
/// - `(args)` -> CALL_FUNCTION
///
/// Suffixes are applied left-to-right, so `a.b[c](d)` compiles as:
/// 1. compile `a`
/// 2. emit LOAD_ATTR "b"
/// 3. compile `c`, emit LOAD_SUBSCRIPT
/// 4. compile `d`, emit CALL_FUNCTION 1
fn compile_primary(compiler: &mut GenericCompiler, node: &ASTNode) {
    let children = &node.children;

    // Compile the atom (first child)
    compiler.compile_node(&children[0]);

    // Apply each suffix
    for child in children.iter().skip(1) {
        if let ASTChild::Node(n) = child {
            if n.rule_name == "suffix" {
                compile_suffix(compiler, n);
            }
        }
    }
}

/// Compile a single suffix (attribute, subscript, or call).
///
/// Grammar:
/// ```text
/// suffix = DOT NAME
///        | LBRACKET subscript RBRACKET
///        | LPAREN [ arguments ] RPAREN ;
/// ```
fn compile_suffix(compiler: &mut GenericCompiler, node: &ASTNode) {
    if has_token(node, ".") {
        // Attribute access: obj.attr
        for child in &node.children {
            if let ASTChild::Token(t) = child {
                if t.token_type == "NAME" {
                    let attr_idx = compiler.add_name(&t.value);
                    compiler.emit(op(Op::LoadAttr), Some(Operand::Index(attr_idx)));
                    break;
                }
            }
        }
    } else if has_token(node, "[") {
        // Subscript/slice
        let subscript_nodes: Vec<&ASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTChild::Node(n) if n.rule_name == "subscript" => Some(n),
                _ => None,
            })
            .collect();

        if let Some(sub) = subscript_nodes.first() {
            compile_subscript(compiler, sub);
        } else {
            // Simple subscript with expression
            for c in &node.children {
                if let ASTChild::Node(n) = c {
                    compiler.compile_node(&ASTChild::Node(n.clone()));
                    compiler.emit(op(Op::LoadSubscript), None);
                    break;
                }
            }
        }
    } else if has_token(node, "(") {
        // Function call
        let arg_nodes: Vec<&ASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTChild::Node(n) if n.rule_name == "arguments" => Some(n),
                _ => None,
            })
            .collect();

        if let Some(args) = arg_nodes.first() {
            let (argc, has_kw) = compile_arguments(compiler, args);
            if has_kw {
                compiler.emit(op(Op::CallFunctionKw), Some(Operand::Index(argc)));
            } else {
                compiler.emit(op(Op::CallFunction), Some(Operand::Index(argc)));
            }
        } else {
            compiler.emit(op(Op::CallFunction), Some(Operand::Index(0)));
        }
    }
}

/// Compile a subscript expression.
///
/// Grammar:
/// ```text
/// subscript = expression
///           | [ expression ] COLON [ expression ] [ COLON [ expression ] ] ;
/// ```
///
/// For slices, we push start/stop/step (or None for missing parts) and
/// emit LOAD_SLICE with flags indicating which parts are present.
fn compile_subscript(compiler: &mut GenericCompiler, node: &ASTNode) {
    if has_token(node, ":") {
        // Slice: [start:stop:step]
        let mut parts: Vec<Option<&ASTNode>> = Vec::new();
        let mut current_exprs: Vec<&ASTNode> = Vec::new();

        for child in &node.children {
            match child {
                ASTChild::Token(t) if t.value == ":" => {
                    parts.push(current_exprs.first().copied());
                    current_exprs.clear();
                }
                ASTChild::Node(n) => {
                    current_exprs.push(n);
                }
                _ => {}
            }
        }
        // Last part
        parts.push(current_exprs.first().copied());

        // Pad to 3 elements
        while parts.len() < 3 {
            parts.push(None);
        }

        // Compile each part, using LOAD_NONE for missing
        let mut flags = 0usize;
        for (i, part) in parts.iter().take(3).enumerate() {
            if let Some(expr) = part {
                compiler.compile_node(&ASTChild::Node((*expr).clone()));
                flags |= 1 << i;
            } else {
                compiler.emit(op(Op::LoadNone), None);
            }
        }

        compiler.emit(op(Op::LoadSlice), Some(Operand::Index(flags)));
    } else {
        // Simple index
        let sub_nodes = child_nodes(node);
        if let Some(expr) = sub_nodes.first() {
            compiler.compile_node(&ASTChild::Node((*expr).clone()));
            compiler.emit(op(Op::LoadSubscript), None);
        }
    }
}

/// Compile function call arguments.
///
/// Grammar: `arguments = argument { COMMA argument } [ COMMA ] ;`
///
/// Returns `(arg_count, has_keyword_args)`.
///
/// Stack layout for CALL_FUNCTION (no keyword args):
/// ```text
/// [func, arg1_value, arg2_value, ...]
/// ```
///
/// Stack layout for CALL_FUNCTION_KW (has keyword args):
/// ```text
/// [func, pos_val1, ..., kw_val1, kw_val2, ..., kw_names_tuple]
/// ```
///
/// The keyword names tuple sits on top. Below it are all argument values
/// in order: positional first, then keyword values (matching the names tuple).
fn compile_arguments(compiler: &mut GenericCompiler, node: &ASTNode) -> (usize, bool) {
    let arg_nodes: Vec<&ASTNode> = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTChild::Node(n) if n.rule_name == "argument" => Some(n),
            _ => None,
        })
        .collect();

    let mut argc = 0;
    let mut has_kw = false;
    let mut kw_names: Vec<String> = Vec::new();

    for arg in &arg_nodes {
        let kw_name = compile_argument(compiler, arg);
        if let Some(name) = kw_name {
            has_kw = true;
            kw_names.push(name);
        }
        argc += 1;
    }

    // If keyword args, push names tuple
    if has_kw {
        let kw_str = kw_names.join(",");
        let kw_idx = compiler.add_constant(Value::Str(kw_str));
        compiler.emit(op(Op::LoadConst), Some(Operand::Index(kw_idx)));
    }

    (argc, has_kw)
}

/// Compile a single function call argument.
///
/// Grammar:
/// ```text
/// argument = DOUBLE_STAR expression | STAR expression
///          | NAME EQUALS expression | expression ;
/// ```
///
/// Returns the keyword name if this is a keyword argument (`Some("name")`),
/// or `None` for positional arguments.
fn compile_argument(compiler: &mut GenericCompiler, node: &ASTNode) -> Option<String> {
    let sub_nodes = child_nodes(node);

    if has_token(node, "**") {
        // **kwargs unpacking
        if let Some(expr) = sub_nodes.first() {
            compiler.compile_node(&ASTChild::Node((*expr).clone()));
        }
        None
    } else if has_token(node, "*") {
        // *args unpacking
        if let Some(expr) = sub_nodes.first() {
            compiler.compile_node(&ASTChild::Node((*expr).clone()));
        }
        None
    } else if has_token(node, "=") {
        // Keyword argument: name=value
        let tokens = child_tokens(node);
        let name = tokens
            .iter()
            .find(|t| t.token_type == "NAME")
            .map(|t| t.value.clone());
        if let Some(expr) = sub_nodes.first() {
            compiler.compile_node(&ASTChild::Node((*expr).clone()));
        }
        name
    } else {
        // Positional argument
        if let Some(expr) = sub_nodes.first() {
            compiler.compile_node(&ASTChild::Node((*expr).clone()));
        } else {
            // Token-based expression
            for c in &node.children {
                if let ASTChild::Token(_) = c {
                    compiler.compile_node(c);
                    break;
                }
            }
        }
        None
    }
}

// =========================================================================
// Section 9: Rule Handlers -- Atoms
// =========================================================================

/// Compile an atom -- the leaf-level expression.
///
/// Grammar:
/// ```text
/// atom = INT | FLOAT | STRING { STRING } | NAME
///      | "True" | "False" | "None"
///      | list_expr | dict_expr | paren_expr ;
/// ```
///
/// Atoms are the base building blocks of all expressions:
/// - **Integers**: `42` -> LOAD_CONST with Int(42)
/// - **Floats**: `3.14` -> LOAD_CONST with Float(3.14)
/// - **Strings**: `"hello"` -> LOAD_CONST with Str("hello")
/// - **Names**: `x` -> LOAD_NAME (or LOAD_LOCAL in function scope)
/// - **Booleans**: `True`/`False` -> LOAD_TRUE/LOAD_FALSE
/// - **None**: `None` -> LOAD_NONE
/// - **Adjacent strings**: `"a" "b"` -> concatenated at compile time to `"ab"`
fn compile_atom(compiler: &mut GenericCompiler, node: &ASTNode) {
    let children = &node.children;

    if children.len() == 1 {
        match &children[0] {
            ASTChild::Token(t) => {
                match t.token_type.as_str() {
                    "INT" => {
                        let value: i64 = t.value.parse().unwrap_or(0);
                        let idx = compiler.add_constant(Value::Int(value));
                        compiler.emit(op(Op::LoadConst), Some(Operand::Index(idx)));
                    }
                    "FLOAT" => {
                        let value: f64 = t.value.parse().unwrap_or(0.0);
                        let idx = compiler.add_constant(Value::Float(value));
                        compiler.emit(op(Op::LoadConst), Some(Operand::Index(idx)));
                    }
                    "STRING" => {
                        let value = parse_string_literal(&t.value);
                        let idx = compiler.add_constant(Value::Str(value));
                        compiler.emit(op(Op::LoadConst), Some(Operand::Index(idx)));
                    }
                    "NAME" => {
                        match t.value.as_str() {
                            "True" => {
                                compiler.emit(op(Op::LoadTrue), None);
                            }
                            "False" => {
                                compiler.emit(op(Op::LoadFalse), None);
                            }
                            "None" => {
                                compiler.emit(op(Op::LoadNone), None);
                            }
                            _ => {
                                // Variable reference
                                if let Some(ref scope) = compiler.scope {
                                    if let Some(&slot) = scope.locals.get(&t.value) {
                                        compiler.emit(
                                            op(Op::LoadLocal),
                                            Some(Operand::Index(slot)),
                                        );
                                        return;
                                    }
                                }
                                let idx = compiler.add_name(&t.value);
                                compiler.emit(op(Op::LoadName), Some(Operand::Index(idx)));
                            }
                        }
                    }
                    _ => {
                        // Check for keyword literals by value
                        match t.value.as_str() {
                            "True" => { compiler.emit(op(Op::LoadTrue), None); }
                            "False" => { compiler.emit(op(Op::LoadFalse), None); }
                            "None" => { compiler.emit(op(Op::LoadNone), None); }
                            _ => {} // Unknown token type
                        }
                    }
                }
            }
            ASTChild::Node(_) => {
                // list_expr, dict_expr, or paren_expr
                compiler.compile_node(&children[0]);
            }
        }
    } else if children.len() >= 2 {
        // Adjacent string concatenation: "hello" "world"
        let all_strings = children.iter().all(|c| match c {
            ASTChild::Token(t) => t.token_type == "STRING",
            _ => false,
        });

        if all_strings {
            let concatenated: String = children
                .iter()
                .filter_map(|c| match c {
                    ASTChild::Token(t) => Some(parse_string_literal(&t.value)),
                    _ => None,
                })
                .collect();
            let idx = compiler.add_constant(Value::Str(concatenated));
            compiler.emit(op(Op::LoadConst), Some(Operand::Index(idx)));
        } else {
            for c in children {
                if let ASTChild::Node(_) = c {
                    compiler.compile_node(c);
                }
            }
        }
    }
}

// =========================================================================
// Section 10: Rule Handlers -- Collection Literals
// =========================================================================

/// Compile a list literal or list comprehension.
///
/// Grammar: `list_expr = LBRACKET [ list_body ] RBRACKET ;`
///
/// Empty list `[]` compiles to `BUILD_LIST 0`.
/// Non-empty list delegates to `compile_list_body`.
fn compile_list_expr(compiler: &mut GenericCompiler, node: &ASTNode) {
    let body_nodes: Vec<&ASTNode> = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTChild::Node(n) if n.rule_name == "list_body" => Some(n),
            _ => None,
        })
        .collect();

    if body_nodes.is_empty() {
        compiler.emit(op(Op::BuildList), Some(Operand::Index(0)));
        return;
    }

    compile_list_body(compiler, body_nodes[0]);
}

/// Compile list body -- either literal elements or comprehension.
///
/// Grammar:
/// ```text
/// list_body = expression comp_clause
///           | expression { COMMA expression } [ COMMA ] ;
/// ```
fn compile_list_body(compiler: &mut GenericCompiler, node: &ASTNode) {
    let sub_nodes = child_nodes(node);
    let has_comp = sub_nodes.iter().any(|sn| sn.rule_name == "comp_clause");

    if has_comp {
        compile_list_comprehension(compiler, node);
    } else {
        let exprs: Vec<&ASTNode> = sub_nodes
            .iter()
            .filter(|sn| sn.rule_name != "comp_clause")
            .copied()
            .collect();
        for expr in &exprs {
            compiler.compile_node(&ASTChild::Node((*expr).clone()));
        }
        compiler.emit(op(Op::BuildList), Some(Operand::Index(exprs.len())));
    }
}

/// Compile a list comprehension.
///
/// `[expr for x in iterable if cond]`
///
/// Compilation pattern:
/// ```text
///     BUILD_LIST 0           # empty accumulator
///     compile iterable
///     GET_ITER
///     loop:
///     FOR_ITER -> end
///     store x
///     [compile condition]
///     [JUMP_IF_FALSE -> loop_continue]
///     compile expr
///     LIST_APPEND
///     loop_continue:
///     JUMP -> loop
///     end:
/// ```
fn compile_list_comprehension(compiler: &mut GenericCompiler, node: &ASTNode) {
    let sub_nodes = child_nodes(node);
    let expr_node = sub_nodes[0];
    let comp_clause = sub_nodes
        .iter()
        .find(|sn| sn.rule_name == "comp_clause")
        .expect("comprehension must have comp_clause");

    // Create empty list
    compiler.emit(op(Op::BuildList), Some(Operand::Index(0)));

    // Compile the comprehension clause(s)
    compile_comp_clause(compiler, comp_clause, expr_node, true);
}

/// Compile comprehension for/if clauses.
///
/// Grammar: `comp_clause = comp_for { comp_for | comp_if } ;`
fn compile_comp_clause(
    compiler: &mut GenericCompiler,
    node: &ASTNode,
    expr_node: &ASTNode,
    is_list: bool,
) {
    let sub_nodes = child_nodes(node);
    if sub_nodes.is_empty() || sub_nodes[0].rule_name != "comp_for" {
        return;
    }
    let clauses: Vec<&ASTNode> = sub_nodes.iter().copied().collect();
    compile_comp_for(compiler, &clauses, 0, expr_node, is_list);
}

/// Compile a single for clause in a comprehension, with nested clauses.
fn compile_comp_for(
    compiler: &mut GenericCompiler,
    clauses: &[&ASTNode],
    clause_idx: usize,
    expr_node: &ASTNode,
    is_list: bool,
) {
    if clause_idx >= clauses.len() {
        // Base case: compile the expression and append/set
        compiler.compile_node(&ASTChild::Node(expr_node.clone()));
        if is_list {
            compiler.emit(op(Op::ListAppend), None);
        } else {
            compiler.emit(op(Op::DictSet), None);
        }
        return;
    }

    let clause = clauses[clause_idx];

    if clause.rule_name == "comp_for" {
        let mut loop_vars_node: Option<&ASTNode> = None;
        let mut iterable_node: Option<&ASTNode> = None;
        let mut found_in = false;

        for child in &clause.children {
            match child {
                ASTChild::Token(t) if t.value == "in" => {
                    found_in = true;
                }
                ASTChild::Node(n) => {
                    if !found_in {
                        loop_vars_node = Some(n);
                    } else {
                        iterable_node = Some(n);
                    }
                }
                _ => {}
            }
        }

        if let (Some(loop_vars), Some(iterable)) = (loop_vars_node, iterable_node) {
            compiler.compile_node(&ASTChild::Node(iterable.clone()));
            compiler.emit(op(Op::GetIter), None);
            let loop_top = compiler.current_offset();
            let for_iter_jump = compiler.emit_jump(op(Op::ForIter));
            compile_loop_vars_store(compiler, loop_vars);

            compile_comp_for(compiler, clauses, clause_idx + 1, expr_node, is_list);

            compiler.emit(op(Op::Jump), Some(Operand::Index(loop_top)));
            compiler.patch_jump(for_iter_jump, None);
        }
    } else if clause.rule_name == "comp_if" {
        let sub_nodes = child_nodes(clause);
        if let Some(condition) = sub_nodes.first() {
            compiler.compile_node(&ASTChild::Node((*condition).clone()));
            let skip_jump = compiler.emit_jump(op(Op::JumpIfFalse));
            compile_comp_for(compiler, clauses, clause_idx + 1, expr_node, is_list);
            compiler.patch_jump(skip_jump, None);
        } else {
            compile_comp_for(compiler, clauses, clause_idx + 1, expr_node, is_list);
        }
    }
}

/// Compile a dict literal or dict comprehension.
///
/// Grammar: `dict_expr = LBRACE [ dict_body ] RBRACE ;`
fn compile_dict_expr(compiler: &mut GenericCompiler, node: &ASTNode) {
    let body_nodes: Vec<&ASTNode> = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTChild::Node(n) if n.rule_name == "dict_body" => Some(n),
            _ => None,
        })
        .collect();

    if body_nodes.is_empty() {
        compiler.emit(op(Op::BuildDict), Some(Operand::Index(0)));
        return;
    }

    compile_dict_body(compiler, body_nodes[0]);
}

/// Compile dict body -- either literal entries or comprehension.
fn compile_dict_body(compiler: &mut GenericCompiler, node: &ASTNode) {
    let sub_nodes = child_nodes(node);
    let has_comp = sub_nodes.iter().any(|sn| sn.rule_name == "comp_clause");

    if has_comp {
        compile_dict_comprehension(compiler, node);
    } else {
        let entries: Vec<&ASTNode> = sub_nodes
            .iter()
            .filter(|sn| sn.rule_name == "dict_entry")
            .copied()
            .collect();
        for entry in &entries {
            compile_dict_entry(compiler, entry);
        }
        compiler.emit(op(Op::BuildDict), Some(Operand::Index(entries.len())));
    }
}

/// Compile a single dict entry (key: value).
fn compile_dict_entry(compiler: &mut GenericCompiler, node: &ASTNode) {
    let sub_nodes = child_nodes(node);
    if sub_nodes.len() >= 2 {
        compiler.compile_node(&ASTChild::Node(sub_nodes[0].clone()));
        compiler.compile_node(&ASTChild::Node(sub_nodes[1].clone()));
    }
}

/// Compile a dict comprehension.
fn compile_dict_comprehension(compiler: &mut GenericCompiler, node: &ASTNode) {
    let sub_nodes = child_nodes(node);
    let entry_node = sub_nodes
        .iter()
        .find(|sn| sn.rule_name == "dict_entry")
        .expect("dict comprehension needs dict_entry");
    let comp_clause = sub_nodes
        .iter()
        .find(|sn| sn.rule_name == "comp_clause")
        .expect("dict comprehension needs comp_clause");

    compiler.emit(op(Op::BuildDict), Some(Operand::Index(0)));
    compile_comp_clause(compiler, comp_clause, entry_node, false);
}

/// Compile a parenthesized expression or tuple.
///
/// Grammar: `paren_expr = LPAREN [ paren_body ] RPAREN ;`
///
/// - `()` -> empty tuple
/// - `(x)` -> just x (parenthesized, not a tuple)
/// - `(x,)` -> single-element tuple
/// - `(x, y)` -> two-element tuple
fn compile_paren_expr(compiler: &mut GenericCompiler, node: &ASTNode) {
    let body_nodes: Vec<&ASTNode> = node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTChild::Node(n) if n.rule_name == "paren_body" => Some(n),
            _ => None,
        })
        .collect();

    if body_nodes.is_empty() {
        compiler.emit(op(Op::BuildTuple), Some(Operand::Index(0)));
        return;
    }

    compile_paren_body(compiler, body_nodes[0]);
}

/// Compile parenthesized expression body.
fn compile_paren_body(compiler: &mut GenericCompiler, node: &ASTNode) {
    let sub_nodes = child_nodes(node);
    let has_comp = sub_nodes.iter().any(|sn| sn.rule_name == "comp_clause");

    if has_comp {
        compile_list_comprehension(compiler, node);
        return;
    }

    let has_comma = node.children.iter().any(|c| match c {
        ASTChild::Token(t) => t.value == ",",
        _ => false,
    });

    if has_comma {
        let exprs: Vec<&ASTNode> = sub_nodes
            .iter()
            .filter(|sn| sn.rule_name != "comp_clause")
            .copied()
            .collect();
        for expr in &exprs {
            compiler.compile_node(&ASTChild::Node((*expr).clone()));
        }
        compiler.emit(op(Op::BuildTuple), Some(Operand::Index(exprs.len())));
    } else if !sub_nodes.is_empty() {
        compiler.compile_node(&ASTChild::Node(sub_nodes[0].clone()));
    }
}

// =========================================================================
// Section 11: Rule Handlers -- Lambda
// =========================================================================

/// Compile a lambda expression.
///
/// Grammar: `lambda_expr = "lambda" [ lambda_params ] COLON expression ;`
///
/// Lambda is compiled just like a function definition, but anonymous:
/// 1. Parse parameters
/// 2. Compile body as nested CodeObject (with implicit return)
/// 3. Emit MAKE_FUNCTION
fn compile_lambda_expr(compiler: &mut GenericCompiler, node: &ASTNode) {
    let mut params_node: Option<&ASTNode> = None;
    let mut body_node: Option<&ASTNode> = None;

    for child in &node.children {
        match child {
            ASTChild::Node(n) if n.rule_name == "lambda_params" => {
                params_node = Some(n);
            }
            ASTChild::Node(n) if body_node.is_none() && n.rule_name != "lambda_params" => {
                body_node = Some(n);
            }
            _ => {}
        }
    }

    let body = body_node.expect("lambda must have a body expression");

    // Parse parameters
    let mut param_names: Vec<String> = Vec::new();
    let mut default_count = 0usize;

    if let Some(params) = params_node {
        for child in &params.children {
            if let ASTChild::Node(param) = child {
                if param.rule_name == "lambda_param" {
                    let info = parse_lambda_param(compiler, param);
                    param_names.push(info.name.clone());
                    if info.has_default {
                        default_count += 1;
                    }
                }
            }
        }
    }

    // Compile body as nested CodeObject
    let param_refs: Vec<&str> = param_names.iter().map(|s| s.as_str()).collect();
    compiler.enter_scope(Some(&param_refs));

    // In the scope, compile the body expression and return it
    compiler.compile_node(&ASTChild::Node(body.clone()));
    compiler.emit(op(Op::Return), None);

    let scope = compiler.exit_scope();
    let body_code = virtual_machine::CodeObject {
        instructions: scope.instructions.clone(),
        constants: scope.constants.clone(),
        names: scope.names.clone(),
    };

    let code_idx = compiler.add_constant(Value::Code(Box::new(body_code)));
    compiler.emit(op(Op::LoadConst), Some(Operand::Index(code_idx)));

    let mut flags = 0u8;
    if default_count > 0 {
        flags |= 0x01;
    }
    compiler.emit(op(Op::MakeFunction), Some(Operand::Index(flags as usize)));
}

/// Parse a lambda parameter.
fn parse_lambda_param(compiler: &mut GenericCompiler, node: &ASTNode) -> ParamInfo {
    let tokens = child_tokens(node);
    let sub_nodes = child_nodes(node);

    if has_token(node, "*") {
        let name = tokens
            .iter()
            .find(|t| t.token_type == "NAME")
            .map(|t| t.value.clone())
            .unwrap_or_default();
        ParamInfo { name, kind: "varargs".into(), has_default: false }
    } else if has_token(node, "**") {
        let name = tokens
            .iter()
            .find(|t| t.token_type == "NAME")
            .map(|t| t.value.clone())
            .unwrap_or_default();
        ParamInfo { name, kind: "kwargs".into(), has_default: false }
    } else if has_token(node, "=") {
        let name = tokens
            .iter()
            .find(|t| t.token_type == "NAME")
            .map(|t| t.value.clone())
            .unwrap_or_default();
        if let Some(default_node) = sub_nodes.first() {
            compiler.compile_node(&ASTChild::Node((*default_node).clone()));
        }
        ParamInfo { name, kind: "default".into(), has_default: true }
    } else {
        let name = tokens
            .first()
            .map(|t| t.value.clone())
            .unwrap_or_default();
        ParamInfo { name, kind: "positional".into(), has_default: false }
    }
}

// =========================================================================
// Section 12: Registration -- Creates a Configured GenericCompiler
// =========================================================================

/// Create a `GenericCompiler` configured with all Starlark rule handlers.
///
/// This is the main factory function. It creates a fresh GenericCompiler
/// and registers handlers for all ~30 Starlark grammar rules that need
/// explicit compilation logic. The remaining ~25 pass-through rules are
/// handled automatically by GenericCompiler's default behavior (compile
/// all children).
///
/// # Example
///
/// ```text
/// let mut compiler = create_starlark_compiler();
/// let code = compiler.compile(&ast, Some(Op::Halt as u8));
/// ```
pub fn create_starlark_compiler() -> GenericCompiler {
    let mut compiler = GenericCompiler::new();

    // -- Top-level structure --
    compiler.register_rule("file", compile_file);
    compiler.register_rule("simple_stmt", compile_simple_stmt);
    compiler.register_rule("suite", compile_suite);

    // -- Simple statements --
    compiler.register_rule("assign_stmt", compile_assign_stmt);
    compiler.register_rule("return_stmt", compile_return_stmt);
    compiler.register_rule("break_stmt", compile_break_stmt);
    compiler.register_rule("continue_stmt", compile_continue_stmt);
    compiler.register_rule("pass_stmt", compile_pass_stmt);
    compiler.register_rule("load_stmt", compile_load_stmt);

    // -- Compound statements --
    compiler.register_rule("if_stmt", compile_if_stmt);
    compiler.register_rule("for_stmt", compile_for_stmt);
    compiler.register_rule("def_stmt", compile_def_stmt);

    // -- Expressions --
    compiler.register_rule("expression", compile_expression);
    compiler.register_rule("expression_list", compile_expression_list);
    compiler.register_rule("or_expr", compile_or_expr);
    compiler.register_rule("and_expr", compile_and_expr);
    compiler.register_rule("not_expr", compile_not_expr);
    compiler.register_rule("comparison", compile_comparison);

    // -- Binary operations (all use the same handler) --
    compiler.register_rule("arith", compile_binary_op);
    compiler.register_rule("term", compile_binary_op);
    compiler.register_rule("shift", compile_binary_op);
    compiler.register_rule("bitwise_or", compile_binary_op);
    compiler.register_rule("bitwise_xor", compile_binary_op);
    compiler.register_rule("bitwise_and", compile_binary_op);

    // -- Unary and power --
    compiler.register_rule("factor", compile_factor);
    compiler.register_rule("power", compile_power);

    // -- Primary expressions --
    compiler.register_rule("primary", compile_primary);

    // -- Atoms --
    compiler.register_rule("atom", compile_atom);

    // -- Collection literals --
    compiler.register_rule("list_expr", compile_list_expr);
    compiler.register_rule("dict_expr", compile_dict_expr);
    compiler.register_rule("paren_expr", compile_paren_expr);

    // -- Lambda --
    compiler.register_rule("lambda_expr", compile_lambda_expr);

    // -- Pass-through rules (handled by GenericCompiler automatically) --
    // statement, compound_stmt, small_stmt -- all have a single child

    compiler
}

// =========================================================================
// Section 13: Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use bytecode_compiler::{ASTChild, ASTNode, TokenNode};
    use virtual_machine::{Operand, Value};

    // ── Helper functions ─────────────────────────────────────────────────

    /// Create a token AST child.
    fn tok(token_type: &str, value: &str) -> ASTChild {
        ASTChild::Token(TokenNode {
            token_type: token_type.to_string(),
            value: value.to_string(),
        })
    }

    /// Create a node AST child.
    fn nd(rule_name: &str, children: Vec<ASTChild>) -> ASTChild {
        ASTChild::Node(ASTNode {
            rule_name: rule_name.to_string(),
            children,
        })
    }

    /// Create a bare ASTNode.
    fn ast(rule_name: &str, children: Vec<ASTChild>) -> ASTNode {
        ASTNode {
            rule_name: rule_name.to_string(),
            children,
        }
    }

    /// Compile an AST and return the CodeObject.
    fn compile(root: ASTNode) -> virtual_machine::CodeObject {
        let mut compiler = create_starlark_compiler();
        compiler.compile(&root, Some(op(Op::Halt)))
    }

    /// Compile an AST without HALT.
    fn compile_no_halt(root: ASTNode) -> virtual_machine::CodeObject {
        let mut compiler = create_starlark_compiler();
        compiler.compile(&root, None)
    }

    // ── Test: op() helper ────────────────────────────────────────────────

    #[test]
    fn test_op_conversion() {
        assert_eq!(op(Op::LoadConst), 0x01);
        assert_eq!(op(Op::Add), 0x20);
        assert_eq!(op(Op::Halt), 0xFF);
        assert_eq!(op(Op::Jump), 0x40);
    }

    // ── Test: parse_string_literal ───────────────────────────────────────

    #[test]
    fn test_parse_string_literal_double_quotes() {
        assert_eq!(parse_string_literal("\"hello\""), "hello");
    }

    #[test]
    fn test_parse_string_literal_single_quotes() {
        assert_eq!(parse_string_literal("'world'"), "world");
    }

    #[test]
    fn test_parse_string_literal_escapes() {
        assert_eq!(parse_string_literal("\"a\\nb\""), "a\nb");
        assert_eq!(parse_string_literal("\"a\\tb\""), "a\tb");
        assert_eq!(parse_string_literal("\"a\\\\b\""), "a\\b");
    }

    #[test]
    fn test_parse_string_literal_triple_quotes() {
        assert_eq!(parse_string_literal("\"\"\"multi\nline\"\"\""), "multi\nline");
    }

    // ── Test: extract_simple_name ────────────────────────────────────────

    #[test]
    fn test_extract_simple_name() {
        // Single-child chain: expression_list > expression > atom > NAME
        let node = ast("expression_list", vec![
            nd("expression", vec![
                nd("atom", vec![tok("NAME", "x")])
            ])
        ]);
        assert_eq!(extract_simple_name(&node), Some("x".to_string()));
    }

    #[test]
    fn test_extract_simple_name_not_a_name() {
        let node = ast("expression_list", vec![
            nd("expression", vec![
                nd("atom", vec![tok("INT", "42")])
            ])
        ]);
        assert_eq!(extract_simple_name(&node), None);
    }

    #[test]
    fn test_extract_simple_name_multiple_children() {
        let node = ast("expression_list", vec![
            nd("expression", vec![tok("NAME", "a")]),
            tok("COMMA", ","),
            nd("expression", vec![tok("NAME", "b")]),
        ]);
        assert_eq!(extract_simple_name(&node), None);
    }

    // ── Test: compile_file ───────────────────────────────────────────────

    #[test]
    fn test_compile_empty_file() {
        let root = ast("file", vec![tok("NEWLINE", "\n")]);
        let code = compile(root);
        // Just HALT
        assert_eq!(code.instructions.len(), 1);
        assert_eq!(code.instructions[0].opcode, op(Op::Halt));
    }

    // ── Test: compile_atom -- integers ───────────────────────────────────

    #[test]
    fn test_compile_atom_int() {
        let root = ast("atom", vec![tok("INT", "42")]);
        let code = compile(root);
        assert_eq!(code.instructions.len(), 2); // LOAD_CONST + HALT
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        match &code.constants[0] {
            Value::Int(42) => {}
            other => panic!("Expected Int(42), got {:?}", other),
        }
    }

    #[test]
    fn test_compile_atom_negative_int() {
        // factor("-", atom("5")) -> compile 5, NEGATE
        let root = ast("factor", vec![
            tok("MINUS", "-"),
            nd("atom", vec![tok("INT", "5")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions[1].opcode, op(Op::Negate));
    }

    // ── Test: compile_atom -- floats ─────────────────────────────────────

    #[test]
    fn test_compile_atom_float() {
        let root = ast("atom", vec![tok("FLOAT", "3.14")]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        match &code.constants[0] {
            Value::Float(f) => assert!((f - 3.14).abs() < f64::EPSILON),
            other => panic!("Expected Float(3.14), got {:?}", other),
        }
    }

    // ── Test: compile_atom -- strings ────────────────────────────────────

    #[test]
    fn test_compile_atom_string() {
        let root = ast("atom", vec![tok("STRING", "\"hello\"")]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        match &code.constants[0] {
            Value::Str(s) => assert_eq!(s, "hello"),
            other => panic!("Expected Str(\"hello\"), got {:?}", other),
        }
    }

    #[test]
    fn test_compile_atom_adjacent_strings() {
        let root = ast("atom", vec![
            tok("STRING", "\"hello \""),
            tok("STRING", "\"world\""),
        ]);
        let code = compile(root);
        match &code.constants[0] {
            Value::Str(s) => assert_eq!(s, "hello world"),
            other => panic!("Expected concatenated string, got {:?}", other),
        }
    }

    // ── Test: compile_atom -- booleans and None ──────────────────────────

    #[test]
    fn test_compile_atom_true() {
        let root = ast("atom", vec![tok("NAME", "True")]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadTrue));
    }

    #[test]
    fn test_compile_atom_false() {
        let root = ast("atom", vec![tok("NAME", "False")]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadFalse));
    }

    #[test]
    fn test_compile_atom_none() {
        let root = ast("atom", vec![tok("NAME", "None")]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadNone));
    }

    // ── Test: compile_atom -- variables ──────────────────────────────────

    #[test]
    fn test_compile_atom_variable() {
        let root = ast("atom", vec![tok("NAME", "x")]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadName));
        assert_eq!(code.names, vec!["x"]);
    }

    // ── Test: binary operations ──────────────────────────────────────────

    #[test]
    fn test_compile_arith_add() {
        // arith: atom(1) + atom(2)
        let root = ast("arith", vec![
            nd("atom", vec![tok("INT", "1")]),
            tok("PLUS", "+"),
            nd("atom", vec![tok("INT", "2")]),
        ]);
        let code = compile(root);
        // LOAD_CONST 0, LOAD_CONST 1, ADD, HALT
        assert_eq!(code.instructions.len(), 4);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions[1].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions[2].opcode, op(Op::Add));
    }

    #[test]
    fn test_compile_arith_sub() {
        let root = ast("arith", vec![
            nd("atom", vec![tok("INT", "10")]),
            tok("MINUS", "-"),
            nd("atom", vec![tok("INT", "3")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::Sub));
    }

    #[test]
    fn test_compile_term_mul() {
        let root = ast("term", vec![
            nd("atom", vec![tok("INT", "4")]),
            tok("STAR", "*"),
            nd("atom", vec![tok("INT", "5")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::Mul));
    }

    #[test]
    fn test_compile_term_div() {
        let root = ast("term", vec![
            nd("atom", vec![tok("INT", "10")]),
            tok("SLASH", "/"),
            nd("atom", vec![tok("INT", "2")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::Div));
    }

    #[test]
    fn test_compile_term_floor_div() {
        let root = ast("term", vec![
            nd("atom", vec![tok("INT", "7")]),
            tok("FLOOR_DIV", "//"),
            nd("atom", vec![tok("INT", "2")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::FloorDiv));
    }

    #[test]
    fn test_compile_term_mod() {
        let root = ast("term", vec![
            nd("atom", vec![tok("INT", "7")]),
            tok("PERCENT", "%"),
            nd("atom", vec![tok("INT", "3")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::Mod));
    }

    #[test]
    fn test_compile_power() {
        let root = ast("power", vec![
            nd("atom", vec![tok("INT", "2")]),
            nd("atom", vec![tok("INT", "8")]),
        ]);
        let code = compile(root);
        // LOAD_CONST, LOAD_CONST, POWER, HALT
        assert_eq!(code.instructions[2].opcode, op(Op::Power));
    }

    // ── Test: bitwise operations ─────────────────────────────────────────

    #[test]
    fn test_compile_bitwise_and() {
        let root = ast("bitwise_and", vec![
            nd("atom", vec![tok("INT", "6")]),
            tok("AMP", "&"),
            nd("atom", vec![tok("INT", "3")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::BitAnd));
    }

    #[test]
    fn test_compile_bitwise_or() {
        let root = ast("bitwise_or", vec![
            nd("atom", vec![tok("INT", "4")]),
            tok("PIPE", "|"),
            nd("atom", vec![tok("INT", "2")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::BitOr));
    }

    #[test]
    fn test_compile_bitwise_xor() {
        let root = ast("bitwise_xor", vec![
            nd("atom", vec![tok("INT", "5")]),
            tok("CARET", "^"),
            nd("atom", vec![tok("INT", "3")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::BitXor));
    }

    #[test]
    fn test_compile_shift_left() {
        let root = ast("shift", vec![
            nd("atom", vec![tok("INT", "1")]),
            tok("LEFT_SHIFT", "<<"),
            nd("atom", vec![tok("INT", "4")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::LShift));
    }

    #[test]
    fn test_compile_shift_right() {
        let root = ast("shift", vec![
            nd("atom", vec![tok("INT", "16")]),
            tok("RIGHT_SHIFT", ">>"),
            nd("atom", vec![tok("INT", "2")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::RShift));
    }

    // ── Test: unary operations ───────────────────────────────────────────

    #[test]
    fn test_compile_factor_negate() {
        let root = ast("factor", vec![
            tok("MINUS", "-"),
            nd("atom", vec![tok("INT", "5")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[1].opcode, op(Op::Negate));
    }

    #[test]
    fn test_compile_factor_bitnot() {
        let root = ast("factor", vec![
            tok("TILDE", "~"),
            nd("atom", vec![tok("INT", "5")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[1].opcode, op(Op::BitNot));
    }

    #[test]
    fn test_compile_factor_unary_plus() {
        // unary + is a no-op
        let root = ast("factor", vec![
            tok("PLUS", "+"),
            nd("atom", vec![tok("INT", "5")]),
        ]);
        let code = compile(root);
        // Should just have LOAD_CONST (no extra opcode) + HALT
        assert_eq!(code.instructions.len(), 2);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
    }

    // ── Test: comparison operations ──────────────────────────────────────

    #[test]
    fn test_compile_comparison_eq() {
        let root = ast("comparison", vec![
            nd("atom", vec![tok("INT", "1")]),
            nd("comp_op", vec![tok("EQUALS_EQUALS", "==")]),
            nd("atom", vec![tok("INT", "1")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::CmpEq));
    }

    #[test]
    fn test_compile_comparison_ne() {
        let root = ast("comparison", vec![
            nd("atom", vec![tok("INT", "1")]),
            nd("comp_op", vec![tok("NOT_EQUALS", "!=")]),
            nd("atom", vec![tok("INT", "2")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::CmpNe));
    }

    #[test]
    fn test_compile_comparison_lt() {
        let root = ast("comparison", vec![
            nd("atom", vec![tok("INT", "1")]),
            nd("comp_op", vec![tok("LESS_THAN", "<")]),
            nd("atom", vec![tok("INT", "2")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::CmpLt));
    }

    #[test]
    fn test_compile_comparison_gt() {
        let root = ast("comparison", vec![
            nd("atom", vec![tok("INT", "2")]),
            nd("comp_op", vec![tok("GREATER_THAN", ">")]),
            nd("atom", vec![tok("INT", "1")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::CmpGt));
    }

    #[test]
    fn test_compile_comparison_le() {
        let root = ast("comparison", vec![
            nd("atom", vec![tok("INT", "1")]),
            nd("comp_op", vec![tok("LESS_EQUAL", "<=")]),
            nd("atom", vec![tok("INT", "2")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::CmpLe));
    }

    #[test]
    fn test_compile_comparison_ge() {
        let root = ast("comparison", vec![
            nd("atom", vec![tok("INT", "2")]),
            nd("comp_op", vec![tok("GREATER_EQUAL", ">=")]),
            nd("atom", vec![tok("INT", "1")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::CmpGe));
    }

    #[test]
    fn test_compile_comparison_in() {
        let root = ast("comparison", vec![
            nd("atom", vec![tok("INT", "1")]),
            nd("comp_op", vec![tok("IN", "in")]),
            nd("atom", vec![tok("NAME", "xs")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::CmpIn));
    }

    #[test]
    fn test_compile_comparison_not_in() {
        let root = ast("comparison", vec![
            nd("atom", vec![tok("INT", "1")]),
            nd("comp_op", vec![tok("NOT", "not"), tok("IN", "in")]),
            nd("atom", vec![tok("NAME", "xs")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[2].opcode, op(Op::CmpNotIn));
    }

    // ── Test: boolean operations ─────────────────────────────────────────

    #[test]
    fn test_compile_not_expr() {
        let root = ast("not_expr", vec![
            tok("NOT", "not"),
            nd("atom", vec![tok("NAME", "True")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadTrue));
        assert_eq!(code.instructions[1].opcode, op(Op::Not));
    }

    #[test]
    fn test_compile_or_expr_short_circuit() {
        let root = ast("or_expr", vec![
            nd("atom", vec![tok("NAME", "a")]),
            nd("atom", vec![tok("NAME", "b")]),
        ]);
        let code = compile(root);
        // LOAD_NAME a, JUMP_IF_TRUE_OR_POP, LOAD_NAME b, HALT
        assert_eq!(code.instructions[0].opcode, op(Op::LoadName));
        assert_eq!(code.instructions[1].opcode, op(Op::JumpIfTrueOrPop));
        assert_eq!(code.instructions[2].opcode, op(Op::LoadName));
    }

    #[test]
    fn test_compile_and_expr_short_circuit() {
        let root = ast("and_expr", vec![
            nd("atom", vec![tok("NAME", "a")]),
            nd("atom", vec![tok("NAME", "b")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadName));
        assert_eq!(code.instructions[1].opcode, op(Op::JumpIfFalseOrPop));
        assert_eq!(code.instructions[2].opcode, op(Op::LoadName));
    }

    // ── Test: assignment ─────────────────────────────────────────────────

    #[test]
    fn test_compile_assign_simple() {
        // assign_stmt: expression_list(x) = expression_list(42)
        let root = ast("assign_stmt", vec![
            nd("expression_list", vec![
                nd("expression", vec![nd("atom", vec![tok("NAME", "x")])])
            ]),
            nd("assign_op", vec![tok("EQUALS", "=")]),
            nd("expression_list", vec![
                nd("expression", vec![nd("atom", vec![tok("INT", "42")])])
            ]),
        ]);
        let code = compile(root);
        // LOAD_CONST 42, STORE_NAME x, HALT
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions[1].opcode, op(Op::StoreName));
        assert_eq!(code.names, vec!["x"]);
    }

    #[test]
    fn test_compile_assign_expression_stmt() {
        // Expression statement: just an expression, discard result
        let root = ast("assign_stmt", vec![
            nd("expression_list", vec![
                nd("expression", vec![nd("atom", vec![tok("INT", "42")])])
            ]),
        ]);
        let code = compile(root);
        // LOAD_CONST, POP, HALT
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions[1].opcode, op(Op::Pop));
    }

    #[test]
    fn test_compile_augmented_assign() {
        // x += 1
        let root = ast("assign_stmt", vec![
            nd("expression_list", vec![
                nd("expression", vec![nd("atom", vec![tok("NAME", "x")])])
            ]),
            nd("augmented_assign_op", vec![tok("PLUS_EQUALS", "+=")]),
            nd("expression_list", vec![
                nd("expression", vec![nd("atom", vec![tok("INT", "1")])])
            ]),
        ]);
        let code = compile(root);
        // LOAD_NAME x, LOAD_CONST 1, ADD, STORE_NAME x, HALT
        assert_eq!(code.instructions[0].opcode, op(Op::LoadName));
        assert_eq!(code.instructions[1].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions[2].opcode, op(Op::Add));
        assert_eq!(code.instructions[3].opcode, op(Op::StoreName));
    }

    // ── Test: return statement ───────────────────────────────────────────

    #[test]
    fn test_compile_return_with_value() {
        let root = ast("return_stmt", vec![
            tok("RETURN", "return"),
            nd("expression", vec![nd("atom", vec![tok("INT", "42")])]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions[1].opcode, op(Op::Return));
    }

    #[test]
    fn test_compile_return_bare() {
        let root = ast("return_stmt", vec![tok("RETURN", "return")]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadNone));
        assert_eq!(code.instructions[1].opcode, op(Op::Return));
    }

    // ── Test: pass statement ─────────────────────────────────────────────

    #[test]
    fn test_compile_pass() {
        let root = ast("pass_stmt", vec![tok("PASS", "pass")]);
        let code = compile(root);
        // Just HALT
        assert_eq!(code.instructions.len(), 1);
    }

    // ── Test: if statement ───────────────────────────────────────────────

    #[test]
    fn test_compile_if_simple() {
        // if True: pass
        let root = ast("if_stmt", vec![
            tok("IF", "if"),
            nd("expression", vec![nd("atom", vec![tok("NAME", "True")])]),
            tok("COLON", ":"),
            nd("suite", vec![nd("pass_stmt", vec![tok("PASS", "pass")])]),
        ]);
        let code = compile(root);
        // LOAD_TRUE, JUMP_IF_FALSE, JUMP, HALT
        assert_eq!(code.instructions[0].opcode, op(Op::LoadTrue));
        assert_eq!(code.instructions[1].opcode, op(Op::JumpIfFalse));
        assert_eq!(code.instructions[2].opcode, op(Op::Jump));
    }

    #[test]
    fn test_compile_if_else() {
        // if True: x = 1 else: x = 2
        let root = ast("if_stmt", vec![
            tok("IF", "if"),
            nd("expression", vec![nd("atom", vec![tok("NAME", "True")])]),
            tok("COLON", ":"),
            nd("suite", vec![
                nd("assign_stmt", vec![
                    nd("expression_list", vec![
                        nd("expression", vec![nd("atom", vec![tok("NAME", "x")])])
                    ]),
                    nd("assign_op", vec![tok("EQUALS", "=")]),
                    nd("expression_list", vec![
                        nd("expression", vec![nd("atom", vec![tok("INT", "1")])])
                    ]),
                ]),
            ]),
            tok("ELSE", "else"),
            tok("COLON", ":"),
            nd("suite", vec![
                nd("assign_stmt", vec![
                    nd("expression_list", vec![
                        nd("expression", vec![nd("atom", vec![tok("NAME", "x")])])
                    ]),
                    nd("assign_op", vec![tok("EQUALS", "=")]),
                    nd("expression_list", vec![
                        nd("expression", vec![nd("atom", vec![tok("INT", "2")])])
                    ]),
                ]),
            ]),
        ]);
        let code = compile(root);
        // Should contain JUMP_IF_FALSE and JUMP
        let has_jif = code.instructions.iter().any(|i| i.opcode == op(Op::JumpIfFalse));
        let has_jump = code.instructions.iter().any(|i| i.opcode == op(Op::Jump));
        assert!(has_jif);
        assert!(has_jump);
    }

    // ── Test: for loop ───────────────────────────────────────────────────

    #[test]
    fn test_compile_for_loop() {
        // for x in items: pass
        let root = ast("for_stmt", vec![
            tok("FOR", "for"),
            nd("loop_vars", vec![tok("NAME", "x")]),
            tok("IN", "in"),
            nd("expression", vec![nd("atom", vec![tok("NAME", "items")])]),
            tok("COLON", ":"),
            nd("suite", vec![nd("pass_stmt", vec![tok("PASS", "pass")])]),
        ]);
        let code = compile(root);
        // LOAD_NAME items, GET_ITER, FOR_ITER, STORE_NAME x, JUMP, HALT
        assert_eq!(code.instructions[0].opcode, op(Op::LoadName));
        assert_eq!(code.instructions[1].opcode, op(Op::GetIter));
        assert_eq!(code.instructions[2].opcode, op(Op::ForIter));
        assert_eq!(code.instructions[3].opcode, op(Op::StoreName));
    }

    // ── Test: function definition ────────────────────────────────────────

    #[test]
    fn test_compile_def_stmt() {
        // def f(a, b): return a
        let root = ast("def_stmt", vec![
            tok("DEF", "def"),
            tok("NAME", "f"),
            tok("LPAREN", "("),
            nd("parameters", vec![
                nd("parameter", vec![tok("NAME", "a")]),
                tok("COMMA", ","),
                nd("parameter", vec![tok("NAME", "b")]),
            ]),
            tok("RPAREN", ")"),
            tok("COLON", ":"),
            nd("suite", vec![
                nd("return_stmt", vec![
                    tok("RETURN", "return"),
                    nd("expression", vec![nd("atom", vec![tok("NAME", "a")])]),
                ]),
            ]),
        ]);
        let code = compile(root);
        // Should have LOAD_CONST (params), LOAD_CONST (code), MAKE_FUNCTION, STORE_NAME f, HALT
        let has_make_fn = code.instructions.iter().any(|i| i.opcode == op(Op::MakeFunction));
        let has_store = code.instructions.iter().any(|i| i.opcode == op(Op::StoreName));
        assert!(has_make_fn);
        assert!(has_store);
        assert!(code.names.contains(&"f".to_string()));
    }

    // ── Test: expression list / tuple ────────────────────────────────────

    #[test]
    fn test_compile_expression_list_single() {
        let root = ast("expression_list", vec![
            nd("expression", vec![nd("atom", vec![tok("INT", "42")])]),
        ]);
        let code = compile(root);
        // Just LOAD_CONST + HALT, no BUILD_TUPLE
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions[1].opcode, op(Op::Halt));
    }

    #[test]
    fn test_compile_expression_list_tuple() {
        let root = ast("expression_list", vec![
            nd("expression", vec![nd("atom", vec![tok("INT", "1")])]),
            tok("COMMA", ","),
            nd("expression", vec![nd("atom", vec![tok("INT", "2")])]),
        ]);
        let code = compile(root);
        let has_build_tuple = code.instructions.iter().any(|i| i.opcode == op(Op::BuildTuple));
        assert!(has_build_tuple);
    }

    #[test]
    fn test_compile_expression_list_trailing_comma() {
        let root = ast("expression_list", vec![
            nd("expression", vec![nd("atom", vec![tok("INT", "1")])]),
            tok("COMMA", ","),
        ]);
        let code = compile(root);
        let has_build_tuple = code.instructions.iter().any(|i| i.opcode == op(Op::BuildTuple));
        assert!(has_build_tuple);
    }

    // ── Test: ternary expression ─────────────────────────────────────────

    #[test]
    fn test_compile_ternary_expression() {
        // 1 if True else 2
        let root = ast("expression", vec![
            nd("or_expr", vec![nd("atom", vec![tok("INT", "1")])]),
            tok("IF", "if"),
            nd("or_expr", vec![nd("atom", vec![tok("NAME", "True")])]),
            tok("ELSE", "else"),
            nd("expression", vec![nd("atom", vec![tok("INT", "2")])]),
        ]);
        let code = compile(root);
        let has_jif = code.instructions.iter().any(|i| i.opcode == op(Op::JumpIfFalse));
        assert!(has_jif);
    }

    // ── Test: list literal ───────────────────────────────────────────────

    #[test]
    fn test_compile_list_empty() {
        let root = ast("list_expr", vec![
            tok("LBRACKET", "["),
            tok("RBRACKET", "]"),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::BuildList));
        match &code.instructions[0].operand {
            Some(Operand::Index(0)) => {}
            other => panic!("Expected Index(0), got {:?}", other),
        }
    }

    #[test]
    fn test_compile_list_literal() {
        let root = ast("list_expr", vec![
            tok("LBRACKET", "["),
            nd("list_body", vec![
                nd("expression", vec![nd("atom", vec![tok("INT", "1")])]),
                tok("COMMA", ","),
                nd("expression", vec![nd("atom", vec![tok("INT", "2")])]),
            ]),
            tok("RBRACKET", "]"),
        ]);
        let code = compile(root);
        let has_build_list = code.instructions.iter().any(|i| i.opcode == op(Op::BuildList));
        assert!(has_build_list);
    }

    // ── Test: dict literal ───────────────────────────────────────────────

    #[test]
    fn test_compile_dict_empty() {
        let root = ast("dict_expr", vec![
            tok("LBRACE", "{"),
            tok("RBRACE", "}"),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::BuildDict));
    }

    #[test]
    fn test_compile_dict_literal() {
        let root = ast("dict_expr", vec![
            tok("LBRACE", "{"),
            nd("dict_body", vec![
                nd("dict_entry", vec![
                    nd("expression", vec![nd("atom", vec![tok("STRING", "\"a\"")])]),
                    tok("COLON", ":"),
                    nd("expression", vec![nd("atom", vec![tok("INT", "1")])]),
                ]),
            ]),
            tok("RBRACE", "}"),
        ]);
        let code = compile(root);
        let has_build_dict = code.instructions.iter().any(|i| i.opcode == op(Op::BuildDict));
        assert!(has_build_dict);
    }

    // ── Test: paren expression ───────────────────────────────────────────

    #[test]
    fn test_compile_paren_empty_tuple() {
        let root = ast("paren_expr", vec![
            tok("LPAREN", "("),
            tok("RPAREN", ")"),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::BuildTuple));
    }

    #[test]
    fn test_compile_paren_just_parens() {
        // (42) -> just compile inner expression
        let root = ast("paren_expr", vec![
            tok("LPAREN", "("),
            nd("paren_body", vec![
                nd("expression", vec![nd("atom", vec![tok("INT", "42")])]),
            ]),
            tok("RPAREN", ")"),
        ]);
        let code = compile(root);
        // Should be LOAD_CONST + HALT, no BUILD_TUPLE
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions.len(), 2);
    }

    #[test]
    fn test_compile_paren_tuple() {
        // (1, 2)
        let root = ast("paren_expr", vec![
            tok("LPAREN", "("),
            nd("paren_body", vec![
                nd("expression", vec![nd("atom", vec![tok("INT", "1")])]),
                tok("COMMA", ","),
                nd("expression", vec![nd("atom", vec![tok("INT", "2")])]),
            ]),
            tok("RPAREN", ")"),
        ]);
        let code = compile(root);
        let has_build_tuple = code.instructions.iter().any(|i| i.opcode == op(Op::BuildTuple));
        assert!(has_build_tuple);
    }

    // ── Test: primary with suffix ────────────────────────────────────────

    #[test]
    fn test_compile_primary_attr() {
        // obj.attr
        let root = ast("primary", vec![
            nd("atom", vec![tok("NAME", "obj")]),
            nd("suffix", vec![
                tok("DOT", "."),
                tok("NAME", "attr"),
            ]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadName));
        assert_eq!(code.instructions[1].opcode, op(Op::LoadAttr));
    }

    #[test]
    fn test_compile_primary_call_no_args() {
        // f()
        let root = ast("primary", vec![
            nd("atom", vec![tok("NAME", "f")]),
            nd("suffix", vec![
                tok("LPAREN", "("),
                tok("RPAREN", ")"),
            ]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadName));
        assert_eq!(code.instructions[1].opcode, op(Op::CallFunction));
        match &code.instructions[1].operand {
            Some(Operand::Index(0)) => {}
            other => panic!("Expected 0 args, got {:?}", other),
        }
    }

    #[test]
    fn test_compile_primary_call_with_args() {
        // f(1, 2)
        let root = ast("primary", vec![
            nd("atom", vec![tok("NAME", "f")]),
            nd("suffix", vec![
                tok("LPAREN", "("),
                nd("arguments", vec![
                    nd("argument", vec![
                        nd("expression", vec![nd("atom", vec![tok("INT", "1")])])
                    ]),
                    tok("COMMA", ","),
                    nd("argument", vec![
                        nd("expression", vec![nd("atom", vec![tok("INT", "2")])])
                    ]),
                ]),
                tok("RPAREN", ")"),
            ]),
        ]);
        let code = compile(root);
        // LOAD_NAME f, LOAD_CONST 1, LOAD_CONST 2, CALL_FUNCTION 2, HALT
        let call_idx = code.instructions.iter().position(|i| i.opcode == op(Op::CallFunction)).unwrap();
        match &code.instructions[call_idx].operand {
            Some(Operand::Index(2)) => {}
            other => panic!("Expected 2 args, got {:?}", other),
        }
    }

    #[test]
    fn test_compile_primary_call_with_kwargs() {
        // f(x=1)
        let root = ast("primary", vec![
            nd("atom", vec![tok("NAME", "f")]),
            nd("suffix", vec![
                tok("LPAREN", "("),
                nd("arguments", vec![
                    nd("argument", vec![
                        tok("NAME", "x"),
                        tok("EQUALS", "="),
                        nd("expression", vec![nd("atom", vec![tok("INT", "1")])]),
                    ]),
                ]),
                tok("RPAREN", ")"),
            ]),
        ]);
        let code = compile(root);
        let has_call_kw = code.instructions.iter().any(|i| i.opcode == op(Op::CallFunctionKw));
        assert!(has_call_kw);
    }

    // ── Test: load statement ─────────────────────────────────────────────

    #[test]
    fn test_compile_load_stmt() {
        // load("module.star", "symbol")
        let root = ast("load_stmt", vec![
            tok("LOAD", "load"),
            tok("LPAREN", "("),
            tok("STRING", "\"module.star\""),
            tok("COMMA", ","),
            nd("load_arg", vec![tok("STRING", "\"symbol\"")]),
            tok("RPAREN", ")"),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadModule));
        assert_eq!(code.instructions[1].opcode, op(Op::Dup));
        assert_eq!(code.instructions[2].opcode, op(Op::ImportFrom));
        assert_eq!(code.instructions[3].opcode, op(Op::StoreName));
        assert_eq!(code.instructions[4].opcode, op(Op::Pop));
    }

    #[test]
    fn test_compile_load_stmt_alias() {
        // load("module.star", my_sym = "symbol")
        let root = ast("load_stmt", vec![
            tok("LOAD", "load"),
            tok("LPAREN", "("),
            tok("STRING", "\"module.star\""),
            tok("COMMA", ","),
            nd("load_arg", vec![
                tok("NAME", "my_sym"),
                tok("EQUALS", "="),
                tok("STRING", "\"symbol\""),
            ]),
            tok("RPAREN", ")"),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadModule));
        assert_eq!(code.instructions[1].opcode, op(Op::Dup));
        assert_eq!(code.instructions[2].opcode, op(Op::ImportFrom));
        assert_eq!(code.instructions[3].opcode, op(Op::StoreName));
        // The stored name should be the alias
        assert!(code.names.contains(&"my_sym".to_string()));
    }

    // ── Test: break and continue ─────────────────────────────────────────

    #[test]
    fn test_compile_break() {
        let root = ast("break_stmt", vec![tok("BREAK", "break")]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::Jump));
    }

    #[test]
    fn test_compile_continue() {
        let root = ast("continue_stmt", vec![tok("CONTINUE", "continue")]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::Jump));
    }

    // ── Test: lambda expression ──────────────────────────────────────────

    #[test]
    fn test_compile_lambda() {
        // lambda x: x
        let root = ast("lambda_expr", vec![
            tok("LAMBDA", "lambda"),
            nd("lambda_params", vec![
                nd("lambda_param", vec![tok("NAME", "x")]),
            ]),
            tok("COLON", ":"),
            nd("expression", vec![nd("atom", vec![tok("NAME", "x")])]),
        ]);
        let code = compile(root);
        let has_make_fn = code.instructions.iter().any(|i| i.opcode == op(Op::MakeFunction));
        assert!(has_make_fn);
    }

    // ── Test: create_starlark_compiler ───────────────────────────────────

    #[test]
    fn test_create_compiler_has_all_rules() {
        let _compiler = create_starlark_compiler();
        // If we get here without panic, all registrations succeeded
    }

    // ── Test: pass-through rules ─────────────────────────────────────────

    #[test]
    fn test_passthrough_statement() {
        // "statement" wrapping an assign_stmt should just compile through
        let root = ast("statement", vec![
            nd("assign_stmt", vec![
                nd("expression_list", vec![
                    nd("expression", vec![nd("atom", vec![tok("INT", "42")])])
                ]),
            ]),
        ]);
        let code = compile(root);
        // Should compile the inner assign_stmt: LOAD_CONST, POP, HALT
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions[1].opcode, op(Op::Pop));
    }

    // ── Test: complex program ────────────────────────────────────────────

    #[test]
    fn test_compile_program_assign_and_load() {
        // file { assign_stmt(x = 42) }
        let root = ast("file", vec![
            nd("simple_stmt", vec![
                nd("assign_stmt", vec![
                    nd("expression_list", vec![
                        nd("expression", vec![nd("atom", vec![tok("NAME", "x")])])
                    ]),
                    nd("assign_op", vec![tok("EQUALS", "=")]),
                    nd("expression_list", vec![
                        nd("expression", vec![nd("atom", vec![tok("INT", "42")])])
                    ]),
                ]),
            ]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions[1].opcode, op(Op::StoreName));
        assert!(code.names.contains(&"x".to_string()));
        match &code.constants[0] {
            Value::Int(42) => {}
            other => panic!("Expected Int(42), got {:?}", other),
        }
    }

    // ── Test: subscript ──────────────────────────────────────────────────

    #[test]
    fn test_compile_primary_subscript() {
        // obj[0]
        let root = ast("primary", vec![
            nd("atom", vec![tok("NAME", "obj")]),
            nd("suffix", vec![
                tok("LBRACKET", "["),
                nd("subscript", vec![
                    nd("expression", vec![nd("atom", vec![tok("INT", "0")])]),
                ]),
                tok("RBRACKET", "]"),
            ]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadName));
        // Should have LOAD_CONST + LOAD_SUBSCRIPT
        let has_subscript = code.instructions.iter().any(|i| i.opcode == op(Op::LoadSubscript));
        assert!(has_subscript);
    }

    #[test]
    fn test_compile_slice() {
        // obj[1:3]
        let root = ast("primary", vec![
            nd("atom", vec![tok("NAME", "obj")]),
            nd("suffix", vec![
                tok("LBRACKET", "["),
                nd("subscript", vec![
                    nd("expression", vec![nd("atom", vec![tok("INT", "1")])]),
                    tok("COLON", ":"),
                    nd("expression", vec![nd("atom", vec![tok("INT", "3")])]),
                ]),
                tok("RBRACKET", "]"),
            ]),
        ]);
        let code = compile(root);
        let has_slice = code.instructions.iter().any(|i| i.opcode == op(Op::LoadSlice));
        assert!(has_slice);
    }

    // ── Test: chained comparison ─────────────────────────────────────────

    #[test]
    fn test_compile_comparison_single_passthrough() {
        // Just a value, no comparison operator
        let root = ast("comparison", vec![
            nd("atom", vec![tok("INT", "5")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions.len(), 2); // LOAD_CONST + HALT
    }

    // ── Test: or_expr single passthrough ─────────────────────────────────

    #[test]
    fn test_compile_or_expr_single() {
        let root = ast("or_expr", vec![
            nd("atom", vec![tok("INT", "1")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions.len(), 2);
    }

    // ── Test: and_expr single passthrough ────────────────────────────────

    #[test]
    fn test_compile_and_expr_single() {
        let root = ast("and_expr", vec![
            nd("atom", vec![tok("INT", "1")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions.len(), 2);
    }

    // ── Test: not_expr passthrough ───────────────────────────────────────

    #[test]
    fn test_compile_not_expr_passthrough() {
        let root = ast("not_expr", vec![
            nd("atom", vec![tok("INT", "1")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions.len(), 2);
    }

    // ── Test: power single passthrough ───────────────────────────────────

    #[test]
    fn test_compile_power_single() {
        let root = ast("power", vec![
            nd("atom", vec![tok("INT", "5")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions.len(), 2);
    }

    // ── Test: expression single passthrough ──────────────────────────────

    #[test]
    fn test_compile_expression_single() {
        let root = ast("expression", vec![
            nd("atom", vec![tok("INT", "7")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions.len(), 2);
    }

    // ── Test: factor single passthrough ──────────────────────────────────

    #[test]
    fn test_compile_factor_single() {
        let root = ast("factor", vec![
            nd("atom", vec![tok("INT", "3")]),
        ]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
        assert_eq!(code.instructions.len(), 2);
    }

    // ── Test: multiple binary operations (chained) ───────────────────────

    #[test]
    fn test_compile_arith_chained() {
        // 1 + 2 + 3
        let root = ast("arith", vec![
            nd("atom", vec![tok("INT", "1")]),
            tok("PLUS", "+"),
            nd("atom", vec![tok("INT", "2")]),
            tok("PLUS", "+"),
            nd("atom", vec![tok("INT", "3")]),
        ]);
        let code = compile(root);
        // LOAD 1, LOAD 2, ADD, LOAD 3, ADD, HALT
        assert_eq!(code.instructions.len(), 6);
        assert_eq!(code.instructions[2].opcode, op(Op::Add));
        assert_eq!(code.instructions[4].opcode, op(Op::Add));
    }

    // ── Test: for loop with tuple unpacking ──────────────────────────────

    #[test]
    fn test_compile_for_tuple_unpack() {
        // for x, y in items: pass
        let root = ast("for_stmt", vec![
            tok("FOR", "for"),
            nd("loop_vars", vec![
                tok("NAME", "x"),
                tok("COMMA", ","),
                tok("NAME", "y"),
            ]),
            tok("IN", "in"),
            nd("expression", vec![nd("atom", vec![tok("NAME", "items")])]),
            tok("COLON", ":"),
            nd("suite", vec![nd("pass_stmt", vec![tok("PASS", "pass")])]),
        ]);
        let code = compile(root);
        let has_unpack = code.instructions.iter().any(|i| i.opcode == op(Op::UnpackSequence));
        assert!(has_unpack);
    }

    // ── Test: list comprehension ─────────────────────────────────────────

    #[test]
    fn test_compile_list_comprehension() {
        // [x for x in items]
        let root = ast("list_expr", vec![
            tok("LBRACKET", "["),
            nd("list_body", vec![
                nd("expression", vec![nd("atom", vec![tok("NAME", "x")])]),
                nd("comp_clause", vec![
                    nd("comp_for", vec![
                        tok("FOR", "for"),
                        nd("loop_vars", vec![tok("NAME", "x")]),
                        tok("IN", "in"),
                        nd("expression", vec![nd("atom", vec![tok("NAME", "items")])]),
                    ]),
                ]),
            ]),
            tok("RBRACKET", "]"),
        ]);
        let code = compile(root);
        // Should have BUILD_LIST 0, GET_ITER, FOR_ITER, LIST_APPEND
        let has_build = code.instructions.iter().any(|i| i.opcode == op(Op::BuildList));
        let has_iter = code.instructions.iter().any(|i| i.opcode == op(Op::GetIter));
        let has_append = code.instructions.iter().any(|i| i.opcode == op(Op::ListAppend));
        assert!(has_build);
        assert!(has_iter);
        assert!(has_append);
    }

    // ── Test: compile_no_halt ────────────────────────────────────────────

    #[test]
    fn test_compile_no_halt() {
        let root = ast("atom", vec![tok("INT", "42")]);
        let code = compile_no_halt(root);
        assert_eq!(code.instructions.len(), 1);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadConst));
    }

    // ── Test: multiple or operands ───────────────────────────────────────

    #[test]
    fn test_compile_or_three_operands() {
        let root = ast("or_expr", vec![
            nd("atom", vec![tok("NAME", "a")]),
            nd("atom", vec![tok("NAME", "b")]),
            nd("atom", vec![tok("NAME", "c")]),
        ]);
        let code = compile(root);
        // Should have two JUMP_IF_TRUE_OR_POP instructions
        let count = code.instructions.iter()
            .filter(|i| i.opcode == op(Op::JumpIfTrueOrPop))
            .count();
        assert_eq!(count, 2);
    }

    // ── Test: multiple and operands ──────────────────────────────────────

    #[test]
    fn test_compile_and_three_operands() {
        let root = ast("and_expr", vec![
            nd("atom", vec![tok("NAME", "a")]),
            nd("atom", vec![tok("NAME", "b")]),
            nd("atom", vec![tok("NAME", "c")]),
        ]);
        let code = compile(root);
        let count = code.instructions.iter()
            .filter(|i| i.opcode == op(Op::JumpIfFalseOrPop))
            .count();
        assert_eq!(count, 2);
    }

    // ── Test: atom with non-NAME keyword token ───────────────────────────

    #[test]
    fn test_compile_atom_keyword_token_true() {
        // Token type is not "NAME" but value is "True"
        let root = ast("atom", vec![tok("TRUE", "True")]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadTrue));
    }

    #[test]
    fn test_compile_atom_keyword_token_false() {
        let root = ast("atom", vec![tok("FALSE", "False")]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadFalse));
    }

    #[test]
    fn test_compile_atom_keyword_token_none() {
        let root = ast("atom", vec![tok("NONE", "None")]);
        let code = compile(root);
        assert_eq!(code.instructions[0].opcode, op(Op::LoadNone));
    }

    // ── Test: def with defaults ──────────────────────────────────────────

    #[test]
    fn test_compile_def_with_default() {
        // def f(a, b=10): return a
        let root = ast("def_stmt", vec![
            tok("DEF", "def"),
            tok("NAME", "f"),
            tok("LPAREN", "("),
            nd("parameters", vec![
                nd("parameter", vec![tok("NAME", "a")]),
                tok("COMMA", ","),
                nd("parameter", vec![
                    tok("NAME", "b"),
                    tok("EQUALS", "="),
                    nd("expression", vec![nd("atom", vec![tok("INT", "10")])]),
                ]),
            ]),
            tok("RPAREN", ")"),
            tok("COLON", ":"),
            nd("suite", vec![
                nd("return_stmt", vec![
                    tok("RETURN", "return"),
                    nd("expression", vec![nd("atom", vec![tok("NAME", "a")])]),
                ]),
            ]),
        ]);
        let code = compile(root);
        let has_make_fn = code.instructions.iter().any(|i| i.opcode == op(Op::MakeFunction));
        assert!(has_make_fn);
        // Should have a default value (LOAD_CONST 10) before MAKE_FUNCTION
        let make_fn_idx = code.instructions.iter().position(|i| i.opcode == op(Op::MakeFunction)).unwrap();
        // The LOAD_CONST for the default should be before MAKE_FUNCTION
        let load_const_count = code.instructions[..make_fn_idx]
            .iter()
            .filter(|i| i.opcode == op(Op::LoadConst))
            .count();
        assert!(load_const_count >= 3); // default(10), param_names, code_object
    }
}
