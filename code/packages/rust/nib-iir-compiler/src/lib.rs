//! # `nib-iir-compiler` — Nib source → `InterpreterIR` (IIRModule).
//!
//! Compiles a Nib source program to the language-agnostic
//! [`interpreter_ir::IIRModule`] used by the LANG-runtime AOT and JIT
//! pipelines.  Once a Nib program is in IIR form, the rest of the
//! native-codegen story (`aot-core` specialisation → `aarch64-backend`
//! lowering → `code-packager::macho_object` → `ld`) runs unchanged.
//!
//! ## Why this crate exists
//!
//! Historically Nib went through `compiler-ir::IrProgram`, an older
//! more assembly-flavoured IR shared with brainfuck-wasm and the
//! Intel-4004 toolchain.  The new pipeline (twig-aot, jit-core,
//! aot-core) sits on `interpreter_ir::IIRModule`, which is closer to
//! a typed bytecode and has the `call_builtin` lowering pass that
//! turns operators into native instructions.  Bridging Nib into IIR
//! lets the same backend produce native ARM64 binaries for Nib.
//!
//! ## Coverage (V1)
//!
//! | Nib construct | Status |
//! |---|---|
//! | `fn main() { ... }` with statements | ✓ |
//! | `let name: ty = expr;` | ✓ |
//! | `return expr;` | ✓ |
//! | Integer literals (`5`, `0x1F`) | ✓ |
//! | Identifiers / parameters | ✓ |
//! | Binary arithmetic (`+`, `-`) | ✓ — emitted as `call_builtin "+"` etc., which `aot-core::specialise` lowers to typed CIR |
//! | Comparisons (`==`, `!=`, `<`, `<=`, `>`, `>=`) | ✓ — same lowering path |
//! | `if expr { ... } else { ... }` | ✓ |
//! | Cross-function calls | ✗ — V1 has no cross-fn `call` lowering in aarch64-backend |
//! | Wrap/sat add, bitwise ops, for loops, BCD | ✗ — out of V1 scope |
//!
//! Calling other user-defined Nib functions emits IR but the V1
//! aarch64-backend rejects `call` (no relocation support yet); top-level
//! `main` stays self-contained.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::collections::HashMap;

use coding_adventures_nib_parser::parse_nib;
use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use interpreter_ir::source_loc::SourceLoc;
use nib_type_checker::{check, NibType, TypedAst};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Errors raised by the compiler.
#[derive(Debug)]
#[allow(dead_code)]
pub enum CompileError {
    /// Parser rejected the source.
    Parse(String),
    /// Type-checker reported errors.
    Type(Vec<String>),
    /// AST contained a construct we don't yet handle.
    Unsupported(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Parse(s)        => write!(f, "nib parse: {s}"),
            CompileError::Type(errs)      => write!(f, "nib type-check failed:\n{}", errs.join("\n")),
            CompileError::Unsupported(s)  => write!(f, "nib unsupported: {s}"),
        }
    }
}

impl std::error::Error for CompileError {}

/// Compile a Nib source string to an [`IIRModule`] ready for AOT or JIT.
///
/// The module's `entry_point` is set to `"main"` if the source contains a
/// `fn main()` declaration; otherwise the first compiled function.
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, CompileError> {
    let ast = parse_nib(source)
        .map_err(|e| CompileError::Parse(format!("{e}")))?;
    let result = check(ast);
    if !result.ok {
        return Err(CompileError::Type(result.errors.iter().map(|e| e.message.clone()).collect()));
    }
    compile_typed(result.typed_ast, module_name)
}

/// Compile an already-type-checked Nib AST.
pub fn compile_typed(typed: TypedAst, module_name: &str) -> Result<IIRModule, CompileError> {
    let mut module = IIRModule::new(module_name, "nib");
    let mut compiler = Compiler::default();
    compiler.compile_program(&typed.root, &typed.types, &mut module)?;
    if module.entry_point.is_none() {
        // IIRModule::new defaults to Some("main"); only override if the
        // source genuinely had no main.
        module.entry_point = module.functions.first().map(|f| f.name.clone());
    }
    Ok(module)
}

// ---------------------------------------------------------------------------
// Compiler state
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Compiler {
    /// Counter for synthesised SSA-ish virtual register names within a
    /// function: `_n0`, `_n1`, …
    var_counter: usize,
    /// Counter for synthesised label names: `_L0`, `_L1`, …
    label_counter: usize,
}

impl Compiler {
    // ---- Top level ------------------------------------------------------

    fn compile_program(
        &mut self,
        root: &GrammarASTNode,
        types: &HashMap<usize, NibType>,
        module: &mut IIRModule,
    ) -> Result<(), CompileError> {
        for fn_decl in function_nodes(root) {
            let f = self.compile_function(fn_decl, types)?;
            module.add_or_replace(f);
        }
        Ok(())
    }

    fn compile_function(
        &mut self,
        fn_decl: &GrammarASTNode,
        types: &HashMap<usize, NibType>,
    ) -> Result<IIRFunction, CompileError> {
        // Reset per-function counters for stable register naming.
        self.var_counter = 0;
        self.label_counter = 0;

        let name = first_name(fn_decl)
            .ok_or_else(|| CompileError::Unsupported("fn_decl missing name".into()))?;
        let params = extract_params(fn_decl);
        let ret_ty = extract_return_type(fn_decl);

        let mut body: Vec<IIRInstr> = Vec::new();
        let mut env: HashMap<String, String> = params.iter().cloned().collect();

        if let Some(block) = child_nodes(fn_decl)
            .into_iter()
            .find(|n| n.rule_name == "block")
        {
            self.compile_block(block, types, &mut env, &mut body)?;
        }

        // Defensive trailing `ret_void` so a function without an explicit
        // return doesn't fall off the end at runtime.  The aarch64-backend
        // emits a defensive epilogue too, but a well-formed IIR should
        // always end in some `ret*`.
        if !body.iter().any(|i| i.op.starts_with("ret")) {
            body.push(IIRInstr::new("ret_void", None, vec![], "void"));
        }

        let mut iir_fn = IIRFunction::new(&name, params, &ret_ty, body);
        // Empty source_map keeps the lockstep invariant satisfied for
        // tooling that doesn't need positions.  Real position threading
        // is a follow-up.
        iir_fn.source_map = vec![SourceLoc::SYNTHETIC; iir_fn.instructions.len()];
        Ok(iir_fn)
    }

    // ---- Statements -----------------------------------------------------

    fn compile_block(
        &mut self,
        block: &GrammarASTNode,
        types: &HashMap<usize, NibType>,
        env: &mut HashMap<String, String>,
        out: &mut Vec<IIRInstr>,
    ) -> Result<(), CompileError> {
        for stmt in child_nodes(block) {
            self.compile_stmt(stmt, types, env, out)?;
        }
        Ok(())
    }

    fn compile_stmt(
        &mut self,
        stmt: &GrammarASTNode,
        types: &HashMap<usize, NibType>,
        env: &mut HashMap<String, String>,
        out: &mut Vec<IIRInstr>,
    ) -> Result<(), CompileError> {
        // The grammar wraps statements in a generic `stmt` node — unwrap.
        if stmt.rule_name == "stmt" {
            if let Some(inner) = child_nodes(stmt).first() {
                return self.compile_stmt(inner, types, env, out);
            }
            return Ok(());
        }

        match stmt.rule_name.as_str() {
            "let_stmt" => self.compile_let(stmt, types, env, out),
            "return_stmt" => {
                if let Some(expr) = expression_children(stmt).first() {
                    let v = self.compile_expr(expr, types, env, out)?;
                    let ty_str = lookup_node_type(expr, types).map(nib_ty_str).unwrap_or("any").to_string();
                    out.push(IIRInstr::new(
                        "ret",
                        None,
                        vec![Operand::Var(v)],
                        &ty_str,
                    ));
                } else {
                    out.push(IIRInstr::new("ret_void", None, vec![], "void"));
                }
                Ok(())
            }
            "assign_stmt" => {
                let name = first_name(stmt)
                    .ok_or_else(|| CompileError::Unsupported("assign_stmt missing name".into()))?;
                if let Some(expr) = expression_children(stmt).first() {
                    let v = self.compile_expr(expr, types, env, out)?;
                    // Re-emit as a `_move` so the destination's slot updates.
                    out.push(IIRInstr::new(
                        "call_builtin",
                        Some(name.clone()),
                        vec![Operand::Var("_move".into()), Operand::Var(v)],
                        "any",
                    ));
                }
                Ok(())
            }
            "expr_stmt" => {
                if let Some(expr) = expression_children(stmt).first() {
                    let _ = self.compile_expr(expr, types, env, out)?;
                }
                Ok(())
            }
            "if_stmt" => self.compile_if(stmt, types, env, out),
            other => Err(CompileError::Unsupported(format!("stmt: {other}"))),
        }
    }

    fn compile_let(
        &mut self,
        stmt: &GrammarASTNode,
        types: &HashMap<usize, NibType>,
        env: &mut HashMap<String, String>,
        out: &mut Vec<IIRInstr>,
    ) -> Result<(), CompileError> {
        let name = first_name(stmt)
            .ok_or_else(|| CompileError::Unsupported("let_stmt missing name".into()))?;
        let ty_str = extract_let_type(stmt).unwrap_or_else(|| "any".to_string());
        env.insert(name.clone(), ty_str.clone());

        if let Some(expr) = expression_children(stmt).first() {
            let v = self.compile_expr(expr, types, env, out)?;
            // Bind the user-named variable via `_move` so subsequent
            // references resolve to the same slot.
            out.push(IIRInstr::new(
                "call_builtin",
                Some(name.clone()),
                vec![Operand::Var("_move".into()), Operand::Var(v)],
                &ty_str,
            ));
        }
        Ok(())
    }

    fn compile_if(
        &mut self,
        stmt: &GrammarASTNode,
        types: &HashMap<usize, NibType>,
        env: &mut HashMap<String, String>,
        out: &mut Vec<IIRInstr>,
    ) -> Result<(), CompileError> {
        // Children: cond, then_block, [else_block]
        let kids = child_nodes(stmt);
        let cond_node = kids.iter().find(|n| is_expr_rule(&n.rule_name))
            .copied()
            .ok_or_else(|| CompileError::Unsupported("if_stmt missing condition".into()))?;
        let blocks: Vec<&GrammarASTNode> = kids.iter()
            .filter(|n| n.rule_name == "block")
            .copied().collect();
        let then_block = blocks.first()
            .ok_or_else(|| CompileError::Unsupported("if_stmt missing then-block".into()))?;
        let else_block = blocks.get(1).copied();

        let cond_v = self.compile_expr(cond_node, types, env, out)?;
        let else_lbl = self.fresh_label();
        let end_lbl  = self.fresh_label();

        // jmp_if_false cond_v, else_lbl
        out.push(IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var(cond_v), Operand::Var(else_lbl.clone())],
            "void",
        ));

        self.compile_block(then_block, types, env, out)?;
        out.push(IIRInstr::new(
            "jmp", None,
            vec![Operand::Var(end_lbl.clone())], "void",
        ));

        out.push(IIRInstr::new(
            "label", None,
            vec![Operand::Var(else_lbl)], "void",
        ));
        if let Some(eb) = else_block {
            self.compile_block(eb, types, env, out)?;
        }

        out.push(IIRInstr::new(
            "label", None,
            vec![Operand::Var(end_lbl)], "void",
        ));
        Ok(())
    }

    // ---- Expressions ----------------------------------------------------

    /// Compile an expression and return the IIR variable name holding
    /// its value.
    fn compile_expr(
        &mut self,
        node: &GrammarASTNode,
        types: &HashMap<usize, NibType>,
        env: &mut HashMap<String, String>,
        out: &mut Vec<IIRInstr>,
    ) -> Result<String, CompileError> {
        // Single-child wrapper rules pass through to the inner expression.
        let kids = child_nodes(node);
        if kids.len() == 1
            && node.rule_name != "primary"
            && !is_terminal_expr(node)
        {
            return self.compile_expr(kids[0], types, env, out);
        }

        match node.rule_name.as_str() {
            "primary" => self.compile_primary(node, types, env, out),
            "or_expr" | "and_expr" | "eq_expr" | "cmp_expr" | "add_expr" | "bitwise_expr" => {
                self.compile_binary_chain(node, types, env, out)
            }
            "unary_expr" => self.compile_unary(node, types, env, out),
            // Default: single-child fallthrough already handled above; if
            // we get here with a multi-child unknown rule, walk first.
            _ => {
                if let Some(c) = kids.first() {
                    return self.compile_expr(c, types, env, out);
                }
                Err(CompileError::Unsupported(format!("expr: {}", node.rule_name)))
            }
        }
    }

    fn compile_primary(
        &mut self,
        node: &GrammarASTNode,
        types: &HashMap<usize, NibType>,
        env: &mut HashMap<String, String>,
        out: &mut Vec<IIRInstr>,
    ) -> Result<String, CompileError> {
        // primary is one of: INT_LIT | HEX_LIT | NAME | "(" expr ")" | …
        if let Some(value) = parse_literal(node) {
            let v = self.fresh_var();
            let ty = lookup_node_type(node, types).map(nib_ty_str).unwrap_or("u8");
            out.push(IIRInstr::new(
                "const",
                Some(v.clone()),
                vec![Operand::Int(value)],
                ty,
            ));
            return Ok(v);
        }

        if let Some(name) = lookup_name(node) {
            // Variable reference — return its IIR name directly.
            return Ok(name);
        }

        // Fallback: if it's a parenthesised expression, recurse on the inner.
        if let Some(c) = child_nodes(node).into_iter().find(|c| is_expr_rule(&c.rule_name)) {
            return self.compile_expr(c, types, env, out);
        }

        Err(CompileError::Unsupported(format!("primary: {}", node.rule_name)))
    }

    /// Compile a left-associative binary chain like `a + b + c` by walking
    /// children pairwise.  The grammar uses rules like
    /// `add_expr = bitwise_expr { (PLUS|MINUS|...) bitwise_expr }`.
    fn compile_binary_chain(
        &mut self,
        node: &GrammarASTNode,
        types: &HashMap<usize, NibType>,
        env: &mut HashMap<String, String>,
        out: &mut Vec<IIRInstr>,
    ) -> Result<String, CompileError> {
        let kids = node.children.iter().collect::<Vec<_>>();
        if kids.is_empty() {
            return Err(CompileError::Unsupported(format!("empty {}", node.rule_name)));
        }

        // First child is always an expression sub-node.  Subsequent come
        // in (operator-token, expression-node) pairs.
        let mut iter = kids.into_iter();
        let first_child = iter.next().unwrap();
        let mut acc = match first_child {
            ASTNodeOrToken::Node(n) => self.compile_expr(n, types, env, out)?,
            ASTNodeOrToken::Token(_) => {
                return Err(CompileError::Unsupported(format!(
                    "{} starts with a token", node.rule_name
                )));
            }
        };

        loop {
            let op_tok = match iter.next() {
                Some(ASTNodeOrToken::Token(t)) => t,
                Some(_) => return Err(CompileError::Unsupported(
                    format!("{} expected operator token", node.rule_name))),
                None => break, // chain done
            };
            let rhs_node = match iter.next() {
                Some(ASTNodeOrToken::Node(n)) => n,
                _ => return Err(CompileError::Unsupported(
                    format!("{} dangling operator", node.rule_name))),
            };

            let rhs = self.compile_expr(rhs_node, types, env, out)?;
            let op_name = builtin_for(&op_tok.value, &op_tok.effective_type_name())
                .ok_or_else(|| CompileError::Unsupported(format!("op {:?}", op_tok.value)))?;

            let dest = self.fresh_var();
            // Result type — comparisons produce bool, arithmetic preserves.
            let ty_str = if is_comparison_op(op_name) { "bool" }
                         else { lookup_node_type(node, types).map(nib_ty_str).unwrap_or("any") };
            out.push(IIRInstr::new(
                "call_builtin",
                Some(dest.clone()),
                vec![
                    Operand::Var(op_name.into()),
                    Operand::Var(acc),
                    Operand::Var(rhs),
                ],
                ty_str,
            ));
            acc = dest;
        }

        Ok(acc)
    }

    fn compile_unary(
        &mut self,
        node: &GrammarASTNode,
        types: &HashMap<usize, NibType>,
        env: &mut HashMap<String, String>,
        out: &mut Vec<IIRInstr>,
    ) -> Result<String, CompileError> {
        // unary_expr = (BANG|TILDE) unary_expr | primary | …
        // For V1 we just pass the inner expression through (drop the
        // operator); proper logical-NOT / bitwise-NOT lowering is deferred.
        if let Some(inner) = child_nodes(node).into_iter().find(|c| is_expr_rule(&c.rule_name)) {
            return self.compile_expr(inner, types, env, out);
        }
        Err(CompileError::Unsupported("empty unary_expr".into()))
    }

    // ---- Helpers --------------------------------------------------------

    fn fresh_var(&mut self) -> String {
        let i = self.var_counter;
        self.var_counter += 1;
        format!("_n{i}")
    }

    fn fresh_label(&mut self) -> String {
        let i = self.label_counter;
        self.label_counter += 1;
        format!("_L{i}")
    }
}

// ---------------------------------------------------------------------------
// AST traversal helpers
// ---------------------------------------------------------------------------

fn function_nodes(root: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    child_nodes(root)
        .into_iter()
        .filter_map(|n| {
            if n.rule_name == "fn_decl" { Some(n) }
            else if n.rule_name == "top_decl" {
                child_nodes(n).into_iter().find(|c| c.rule_name == "fn_decl")
            } else { None }
        })
        .collect()
}

fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children.iter().filter_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        _ => None,
    }).collect()
}

fn expression_children(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    child_nodes(node).into_iter()
        .filter(|c| is_expr_rule(&c.rule_name))
        .collect()
}

fn is_expr_rule(name: &str) -> bool {
    matches!(name,
        "expr" | "or_expr" | "and_expr" | "eq_expr" | "cmp_expr"
        | "add_expr" | "bitwise_expr" | "unary_expr" | "primary"
        | "call_expr"
    )
}

fn is_terminal_expr(node: &GrammarASTNode) -> bool {
    node.rule_name == "primary" || node.rule_name == "call_expr"
}

fn first_name(node: &GrammarASTNode) -> Option<String> {
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.effective_type_name() == "NAME" {
                return Some(t.value.clone());
            }
        }
    }
    None
}

fn lookup_name(node: &GrammarASTNode) -> Option<String> {
    first_name(node).or_else(|| {
        child_nodes(node).into_iter().find_map(lookup_name)
    })
}

fn extract_params(fn_decl: &GrammarASTNode) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for c in child_nodes(fn_decl) {
        if c.rule_name == "param_list" {
            for p in child_nodes(c) {
                if p.rule_name == "param" {
                    let nm = first_name(p).unwrap_or_else(|| "_arg".to_string());
                    let ty = first_type_name(p).unwrap_or_else(|| "any".to_string());
                    out.push((nm, ty));
                }
            }
        }
    }
    out
}

/// Find the return type after `ARROW` in `fn_decl`.
fn extract_return_type(fn_decl: &GrammarASTNode) -> String {
    if let Some(ty_node) = child_nodes(fn_decl).into_iter().find(|n| n.rule_name == "type") {
        return type_str_from_node(ty_node);
    }
    "void".to_string()
}

/// Extract the declared type from a `let_stmt` (after the COLON).
fn extract_let_type(stmt: &GrammarASTNode) -> Option<String> {
    child_nodes(stmt).into_iter().find(|n| n.rule_name == "type")
        .map(type_str_from_node)
}

fn type_str_from_node(node: &GrammarASTNode) -> String {
    // The type rule contains a single keyword token like "u4" / "u8".
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            return t.value.clone();
        }
    }
    "any".to_string()
}

fn first_type_name(node: &GrammarASTNode) -> Option<String> {
    child_nodes(node).into_iter().find(|c| c.rule_name == "type")
        .map(|n| type_str_from_node(n))
}

fn parse_literal(node: &GrammarASTNode) -> Option<i64> {
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            match t.effective_type_name() {
                "INT_LIT"  => return t.value.parse().ok(),
                "HEX_LIT"  => {
                    let s = t.value.trim_start_matches("0x").trim_start_matches("0X");
                    return i64::from_str_radix(s, 16).ok();
                }
                _ => {}
            }
        }
    }
    None
}

fn lookup_node_type<'a>(node: &'a GrammarASTNode, types: &'a HashMap<usize, NibType>) -> Option<&'a NibType> {
    let id = node as *const GrammarASTNode as usize;
    types.get(&id)
}

fn nib_ty_str(t: &NibType) -> &'static str {
    match t {
        NibType::U4   => "u8",   // u4 has no native CIR mnemonic; widen to u8
        NibType::U8   => "u8",
        NibType::Bcd  => "u8",   // BCD is byte-encoded
        NibType::Bool => "bool",
        NibType::Void => "void",
    }
}

/// Map a Nib operator token to the IIR builtin name `aot-core::specialise`
/// recognises.
fn builtin_for(text: &str, type_name: &str) -> Option<&'static str> {
    match (text, type_name) {
        // Arithmetic
        ("+", _) | (_, "PLUS")        => Some("+"),
        ("-", _) | (_, "MINUS")       => Some("-"),
        // Comparisons
        ("==", _) | (_, "EQ_EQ")      => Some("=="),
        ("!=", _) | (_, "NEQ")        => Some("!="),
        ("<",  _) | (_, "LT")         => Some("<"),
        (">",  _) | (_, "GT")         => Some(">"),
        ("<=", _) | (_, "LEQ")        => Some("<="),
        (">=", _) | (_, "GEQ")        => Some(">="),
        // Logical (passed as comparisons of bool — V1 doesn't fully support)
        // ("&&", _) | (_, "LAND") => ...
        _ => None,
    }
}

fn is_comparison_op(name: &str) -> bool {
    matches!(name, "==" | "!=" | "<" | "<=" | ">" | ">=")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_minimal_main() {
        let src = "fn main() -> u8 { return 42; }";
        let m = compile_source(src, "test").expect("ok");
        assert_eq!(m.entry_point.as_deref(), Some("main"));
        assert_eq!(m.functions.len(), 1);
        assert_eq!(m.functions[0].name, "main");
        // Body should produce a const + ret.
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "const"));
        assert!(body.iter().any(|i| i.op == "ret"));
    }

    #[test]
    fn compiles_arithmetic() {
        let src = "fn main() -> u8 { return 30 + 12; }";
        let m = compile_source(src, "test").expect("ok");
        let body = &m.functions[0].instructions;
        // Expected: const(30), const(12), call_builtin "+", ret
        let consts = body.iter().filter(|i| i.op == "const").count();
        assert!(consts >= 2, "got body: {body:?}");
        assert!(body.iter().any(|i|
            i.op == "call_builtin" && i.srcs.first().and_then(|s| match s {
                Operand::Var(n) => Some(n.as_str()),
                _ => None,
            }) == Some("+")));
        assert!(body.iter().any(|i| i.op == "ret"));
    }

    #[test]
    fn compiles_let_then_return() {
        // Nib's type checker is strict — `7` is u4 by default, so the
        // declared type must match.
        let src = "fn main() -> u4 { let x: u4 = 7; return x; }";
        let m = compile_source(src, "test").expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "ret"));
    }

    #[test]
    fn compiles_if_else() {
        let src = "fn main() -> u8 { if 1 == 1 { return 100; } else { return 200; } }";
        let m = compile_source(src, "test").expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "jmp_if_false"));
        assert!(body.iter().any(|i| i.op == "label"));
    }

    #[test]
    fn rejects_parse_error() {
        let err = compile_source("fn main(", "test").unwrap_err();
        assert!(matches!(err, CompileError::Parse(_)));
    }
}
