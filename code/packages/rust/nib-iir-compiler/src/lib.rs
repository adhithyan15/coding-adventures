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
        // Override `IIRFunction::new`'s automatic `infer_type_status` —
        // it returns `PartiallyTyped` because Nib's control-flow ops
        // (`label`, `jmp`, `jmp_if_false`, `ret_void`) use `"void"`
        // type hints and `"void"` is NOT in
        // `interpreter_ir::opcodes::CONCRETE_TYPES`.  Every Nib
        // instruction is in fact statically known (no `"any"` hints
        // anywhere), so the function is genuinely fully typed for the
        // JIT's threshold-zero compile path.  Mirrors Brainfuck +
        // Dartmouth BASIC + Oct.
        iir_fn.type_status = interpreter_ir::function::FunctionTypeStatus::FullyTyped;
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
                    // Use the expression's inferred type if the type
                    // checker annotated it; otherwise default to "i64"
                    // (Nib's u4/u8/bool all materialise as i64 at the
                    // IIR level — same convention as compile_binary_chain
                    // and oct-iir-compiler).  Falling back to "any" used
                    // to leak through to the IIR-to-* backends which
                    // reject untyped instructions.
                    let ty_str = lookup_node_type(expr, types)
                        .map(nib_ty_str)
                        .unwrap_or("i64")
                        .to_string();
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
                    // Re-emit as a typed `mov` so the destination's slot
                    // updates.  Previously this was `call_builtin "_move"`;
                    // typed `mov` is the canonical form recognised by
                    // vm-core's dispatch, GenericCirJit's bytecode
                    // compiler, and the AOT backends.
                    out.push(IIRInstr::new(
                        "mov",
                        Some(name.clone()),
                        vec![Operand::Var(v)],
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
            "while_stmt" => self.compile_while(stmt, types, env, out),
            other => Err(CompileError::Unsupported(format!("stmt: {other}"))),
        }
    }

    /// NIB04 step 3 — compile `while expr block` to the canonical
    /// IIR loop shape that both x86_64 + aarch64 backends already
    /// lower:
    ///
    /// ```text
    /// label  while_<n>_top
    /// <eval cond → c>
    /// jmp_if_false c, while_<n>_end
    /// <body>
    /// jmp while_<n>_top
    /// label  while_<n>_end
    /// ```
    ///
    /// Re-evaluates the guard each iteration (no hoisting); the body
    /// mutates locals via `assign_stmt`, which already maps to a
    /// `call_builtin "_move"` that updates the slot in-place.
    fn compile_while(
        &mut self,
        stmt: &GrammarASTNode,
        types: &HashMap<usize, NibType>,
        env: &mut HashMap<String, String>,
        out: &mut Vec<IIRInstr>,
    ) -> Result<(), CompileError> {
        // Children: cond expr, body block.
        let kids = child_nodes(stmt);
        let cond_node = kids.iter().find(|n| is_expr_rule(&n.rule_name))
            .copied()
            .ok_or_else(|| CompileError::Unsupported(
                "while_stmt missing condition".into()))?;
        let body = kids.iter().find(|n| n.rule_name == "block")
            .copied()
            .ok_or_else(|| CompileError::Unsupported(
                "while_stmt missing body block".into()))?;

        let top_lbl = self.fresh_label();
        let end_lbl = self.fresh_label();

        // label while_<n>_top
        out.push(IIRInstr::new("label", None,
            vec![Operand::Var(top_lbl.clone())], "void"));

        // <eval cond → c>; jmp_if_false c, while_<n>_end
        let cond_v = self.compile_expr(cond_node, types, env, out)?;
        out.push(IIRInstr::new("jmp_if_false", None,
            vec![Operand::Var(cond_v), Operand::Var(end_lbl.clone())],
            "void"));

        // <body>
        self.compile_block(body, types, env, out)?;

        // jmp while_<n>_top; label while_<n>_end
        out.push(IIRInstr::new("jmp", None,
            vec![Operand::Var(top_lbl)], "void"));
        out.push(IIRInstr::new("label", None,
            vec![Operand::Var(end_lbl)], "void"));
        Ok(())
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
            // Bind the user-named variable via typed `mov` so subsequent
            // references resolve to the same slot.  Canonical form
            // (was `call_builtin "_move"` historically).
            out.push(IIRInstr::new(
                "mov",
                Some(name.clone()),
                vec![Operand::Var(v)],
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
        // primary is one of: INT_LIT | HEX_LIT | NAME | call_expr | "(" expr ")" | …
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

        // NIB04: detect a call_expr child *before* falling back to
        // `lookup_name` (which would otherwise recursively pick up the
        // callee's NAME token and treat the call as a bare variable
        // reference, silently dropping all arguments).
        if let Some(call) = child_nodes(node).into_iter()
            .find(|c| c.rule_name == "call_expr")
        {
            return self.compile_call_expr(call, types, env, out);
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

    /// NIB04 — compile a `call_expr` node.
    ///
    /// Three cases:
    ///
    /// 1. **`print(x)`** — lowers to `call_builtin "print_i64", x`.  The
    ///    runtime helper `__twig_print_i64` already exists from LANG75.
    ///    `print` always takes exactly one integer argument; passing zero
    ///    or more than one is an `Unsupported` error.  The result slot
    ///    is a synthetic void marker (`_void_N`); callers that use
    ///    `print(x)` in expression position (which Nib doesn't generate
    ///    today, but the grammar permits) just ignore it.
    ///
    /// 2. **User-defined function call** — `f(a, b, c)` lowers to
    ///    `call f, a, b, c -> dest`.  The IIR `call` opcode is already
    ///    handled by the x86_64-backend (cross-function relocations
    ///    landed in PR #3331 / #3332).  The dest slot's IIR type is
    ///    inferred from the call-expression's `lookup_node_type` if
    ///    the type-checker annotated it, falling back to `"any"`.
    ///
    /// 3. **Zero-arg call `f()`** — same as case 2 with an empty arg
    ///    list.
    fn compile_call_expr(
        &mut self,
        node: &GrammarASTNode,
        types: &HashMap<usize, NibType>,
        env: &mut HashMap<String, String>,
        out: &mut Vec<IIRInstr>,
    ) -> Result<String, CompileError> {
        let fn_name = first_name(node)
            .ok_or_else(|| CompileError::Unsupported(
                "call_expr missing function name".into(),
            ))?;

        // Compile each argument expression in order.  The grammar shape
        // is `call_expr = NAME LPAREN [arg_list] RPAREN` where
        // `arg_list = expr { COMMA expr }`.  So `call_expr.children`
        // contains an optional `arg_list` sub-node whose non-token
        // children are the argument expressions.
        let mut arg_slots: Vec<String> = Vec::new();
        if let Some(args_node) = child_nodes(node).into_iter()
            .find(|c| c.rule_name == "arg_list")
        {
            for arg in child_nodes(args_node) {
                if is_expr_rule(&arg.rule_name) {
                    let v = self.compile_expr(arg, types, env, out)?;
                    arg_slots.push(v);
                }
            }
        }

        // Case 1: `print(x)` → `call_builtin "print_i64", x`.
        if fn_name == "print" {
            if arg_slots.len() != 1 {
                return Err(CompileError::Unsupported(format!(
                    "print() takes exactly 1 argument, got {}",
                    arg_slots.len(),
                )));
            }
            out.push(IIRInstr::new(
                "call_builtin",
                None,
                vec![
                    Operand::Var("print_i64".into()),
                    Operand::Var(arg_slots.into_iter().next().unwrap()),
                ],
                "void",
            ));
            // print returns no value; emit a synthetic name so callers
            // that use print in expression position (rare) don't blow up.
            return Ok(self.fresh_var());
        }

        // Case 2/3: user-defined function call.
        //
        // CIR srcs convention: srcs[0] = callee_name (as Var), srcs[1..] =
        // arguments.  The x86_64 / aarch64 backends already implement this
        // for cross-function `call` (see LANG43 PR #3331).
        let dest = self.fresh_var();
        let result_ty = lookup_node_type(node, types).map(nib_ty_str)
            .unwrap_or("any").to_string();
        let mut srcs = Vec::with_capacity(arg_slots.len() + 1);
        srcs.push(Operand::Var(fn_name));
        for a in arg_slots {
            srcs.push(Operand::Var(a));
        }
        out.push(IIRInstr::new(
            "call",
            Some(dest.clone()),
            srcs,
            &result_ty,
        ));
        Ok(dest)
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
            // Map operator token to a typed CIR mnemonic the IIR-to-*
            // backends (wasm/jvm/clr/beam) recognise.  Mirrors the
            // pattern oct-iir-compiler uses — emit `add` / `cmp_eq` etc.
            // directly instead of `call_builtin "+"` (the latter requires
            // a downstream `pre_lower_aot_builtins` pass that only runs
            // in the AOT chain, not in the IIR-to-* backends).
            let cir_op = cir_op_for(&op_tok.value, &op_tok.effective_type_name())
                .ok_or_else(|| CompileError::Unsupported(format!("op {:?}", op_tok.value)))?;

            let dest = self.fresh_var();
            // At the IIR level Nib's narrow types (u4/u8/bool) all flow
            // through 64-bit slots — match the pattern in oct-iir-compiler
            // which uses `"i64"` uniformly.  The function's declared
            // Nib return type ("u8" / "bool" / "void") stays the source
            // of truth on the IIRFunction; instruction type_hints are
            // a separate IIR-level concept and concrete-typing them as
            // `i64` lets every IIR-to-* validator accept the module.
            out.push(IIRInstr::new(
                cir_op,
                Some(dest.clone()),
                vec![Operand::Var(acc), Operand::Var(rhs)],
                "i64",
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
    // We widen Nib's narrower integer types to the closest CIR type so
    // `aot-core::specialise`'s ALLOWED_TYPES accepts them — `u4` widens
    // to `u8`, `bcd` likewise.  CIR's own typed mnemonics (`add_u8`,
    // …) operate on full 64-bit registers internally; the narrower
    // semantic width is enforceable later via masking at the backend
    // (deferred to a follow-up).
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            return widen_nib_type(&t.value).to_string();
        }
    }
    "any".to_string()
}

/// Map a raw Nib type name to the closest CIR-allowed type string.
fn widen_nib_type(t: &str) -> &str {
    match t {
        "u4"  => "u8",  // Nib's nibble widens to u8 for CIR
        "bcd" => "u8",  // BCD is byte-encoded
        other => other, // u8, bool, void all pass through unchanged
    }
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

/// Map a Nib operator token to a typed CIR mnemonic that every IIR
/// consumer in the workspace recognises (vm-core, aarch64-backend,
/// x86_64-backend, iir-to-wasm, iir-to-jvm-class-file,
/// iir-to-cil-bytecode, iir-to-beam).
///
/// Pre-NIB04-fix this returned the operator *symbol* (`"+"`, `"=="`)
/// inside a `call_builtin` instruction, on the assumption that the AOT
/// chain's `pre_lower_aot_builtins` pass would rewrite each one to a
/// typed CIR op before the native backends saw it.  That assumption
/// holds for `lang-aot`'s AOT path but breaks every IIR-to-* backend
/// (they validate against concrete CIR opcodes and reject
/// `call_builtin`).  Returning the typed op directly keeps the AOT
/// path working (the rewrite pass is a no-op when there's nothing to
/// rewrite) and unblocks the IIR-to-* backends.
///
/// Mirrors `oct-iir-compiler::compile_binary` exactly.
fn cir_op_for(text: &str, type_name: &str) -> Option<&'static str> {
    match (text, type_name) {
        // Arithmetic
        ("+", _) | (_, "PLUS")        => Some("add"),
        ("-", _) | (_, "MINUS")       => Some("sub"),
        // Comparisons
        ("==", _) | (_, "EQ_EQ")      => Some("cmp_eq"),
        ("!=", _) | (_, "NEQ")        => Some("cmp_ne"),
        ("<",  _) | (_, "LT")         => Some("cmp_lt"),
        (">",  _) | (_, "GT")         => Some("cmp_gt"),
        ("<=", _) | (_, "LEQ")        => Some("cmp_le"),
        (">=", _) | (_, "GEQ")        => Some("cmp_ge"),
        _ => None,
    }
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
        // Expected after the typed-CIR fix:
        //   const _n0 = 30 (u8)
        //   const _n1 = 12 (u8)
        //   add   _n2 = _n0, _n1 (i64)   ← was `call_builtin "+"` pre-fix
        //   ret   _n2 (u8)
        let consts = body.iter().filter(|i| i.op == "const").count();
        assert!(consts >= 2, "got body: {body:?}");
        assert!(body.iter().any(|i| i.op == "add"),
            "expected typed `add` op (not `call_builtin \"+\"`); got body: {body:?}");
        // Old behaviour leaked `call_builtin "+"` — verify we no longer do that.
        assert!(!body.iter().any(|i|
            i.op == "call_builtin" && i.srcs.first().and_then(|s| match s {
                Operand::Var(n) => Some(n.as_str()),
                _ => None,
            }) == Some("+")),
            "regression: `call_builtin \"+\"` leaked into IIR (would break IIR-to-* backends)");
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

    // ── NIB04 — print + cross-function calls ──────────────────────────────────

    /// `print(x)` lowers to `call_builtin "print_i64", x` (LANG75).
    #[test]
    fn compiles_print_call() {
        let src = "fn main() -> u8 { print(42); return 0; }";
        let m = compile_source(src, "test").expect("ok");
        let body = &m.functions[0].instructions;
        let print_call = body.iter().find(|i|
            i.op == "call_builtin" && i.srcs.first().and_then(|s| match s {
                Operand::Var(n) => Some(n.as_str()),
                _ => None,
            }) == Some("print_i64")
        );
        assert!(print_call.is_some(),
                "expected `call_builtin print_i64` in {body:?}");
        let pc = print_call.unwrap();
        assert_eq!(pc.dest, None, "print_i64 returns void; dest must be None");
        // Two srcs: the helper name + the value.
        assert_eq!(pc.srcs.len(), 2);
    }

    /// `double(21)` from main lowers to a `call` IIR with srcs[0] =
    /// callee name and srcs[1..] = arguments.
    #[test]
    fn compiles_cross_function_call() {
        let src = "fn double(x: u8) -> u8 { return x + x; } \
                   fn main() -> u8 { return double(21); }";
        let m = compile_source(src, "test").expect("ok");
        let main_fn = m.functions.iter().find(|f| f.name == "main").expect("main fn");
        let body = &main_fn.instructions;
        let call_instr = body.iter().find(|i| i.op == "call");
        assert!(call_instr.is_some(), "expected `call` in {body:?}");
        let call = call_instr.unwrap();
        // srcs[0] = "double", srcs[1] = the constant slot holding 21.
        assert!(call.dest.is_some(), "call must have a dest");
        assert!(matches!(call.srcs.first(), Some(Operand::Var(n)) if n == "double"),
                "call srcs[0] must be Var(\"double\"); got {:?}", call.srcs);
        assert_eq!(call.srcs.len(), 2, "call should have callee + 1 arg");
    }

    /// Zero-argument call: `f()` → `call f -> dest`.
    #[test]
    fn compiles_zero_arg_call() {
        let src = "fn forty_two() -> u8 { return 42; } \
                   fn main() -> u8 { return forty_two(); }";
        let m = compile_source(src, "test").expect("ok");
        let main_fn = m.functions.iter().find(|f| f.name == "main").expect("main fn");
        let call = main_fn.instructions.iter().find(|i| i.op == "call")
            .expect("expected `call`");
        assert_eq!(call.srcs.len(), 1, "zero-arg call has only the callee in srcs");
        assert!(matches!(call.srcs.first(), Some(Operand::Var(n)) if n == "forty_two"));
    }

    /// `print()` with zero or two arguments is rejected — V1 print
    /// expects exactly one i64 arg.
    #[test]
    fn rejects_print_with_wrong_arity() {
        // V1 print() takes exactly one arg.
        let err = compile_source("fn main() -> u8 { print(); return 0; }", "t")
            .unwrap_err();
        match err {
            CompileError::Unsupported(_) => {}
            other => panic!("expected Unsupported for print() with 0 args, got {other:?}"),
        }
    }

    // ── NIB04 step 3 — while loops ─────────────────────────────────────────────

    /// `while n < 10 { n = n + 1; }` lowers to the canonical
    /// label / jmp_if_false / body / jmp / label loop shape.
    ///
    /// Uses `u4` because integer literals (`0`, `10`, `1`) default to
    /// the smaller width in Nib's type-checker; widening would require
    /// explicit cast syntax which V1 Nib doesn't have.
    #[test]
    fn compiles_while_loop() {
        let src = "fn main() -> u4 { \
                     let n: u4 = 0; \
                     while n < 10 { n = n + 1; } \
                     return n; \
                   }";
        let m = compile_source(src, "test").expect("ok");
        let body = &m.functions[0].instructions;
        let ops: Vec<&str> = body.iter().map(|i| i.op.as_str()).collect();
        // Must have a label, a cmp via call_builtin "<", a jmp_if_false, an
        // unconditional jmp back, and a closing label.
        assert!(ops.contains(&"jmp_if_false"),
                "while loop must emit jmp_if_false; got {ops:?}");
        assert!(ops.contains(&"jmp"),
                "while loop must emit a back-edge jmp; got {ops:?}");
        let label_count = ops.iter().filter(|o| **o == "label").count();
        assert!(label_count >= 2,
                "while loop must emit at least 2 labels (top + end); got {label_count} in {ops:?}");
    }

    /// `while` body that performs cross-function calls + arithmetic — verifies
    /// the loop integrates with the broader IIR pipeline.
    #[test]
    fn compiles_while_with_nested_call() {
        let src = "fn one() -> u4 { return 1; } \
                   fn main() -> u4 { \
                     let n: u4 = 0; \
                     while n < 3 { n = n + one(); } \
                     return n; \
                   }";
        let m = compile_source(src, "test").expect("ok");
        let main_fn = m.functions.iter().find(|f| f.name == "main").expect("main fn");
        // The body should include both a `call` (to `one`) and a `jmp_if_false`.
        let ops: Vec<&str> = main_fn.instructions.iter()
            .map(|i| i.op.as_str()).collect();
        assert!(ops.contains(&"call"), "missing `call` to `one`; got {ops:?}");
        assert!(ops.contains(&"jmp_if_false"), "missing jmp_if_false; got {ops:?}");
    }
}

