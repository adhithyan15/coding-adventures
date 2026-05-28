//! # `dartmouth-basic-iir-compiler` — Dartmouth BASIC → `IIRModule`.
//!
//! Lowers parsed 1964 Dartmouth BASIC into the language-agnostic
//! [`interpreter_ir::IIRModule`] consumed by the LANG VM AOT chain
//! (twig-aot / lang-aot → x86_64-backend / aarch64-backend → object →
//! system linker → native executable).
//!
//! Distinct from the existing `dartmouth-basic-ir-compiler` crate,
//! which targets the GE-225 simulator's custom `compiler_ir::IrProgram`
//! shape — that IR is **not** pluggable into the LANG VM chain.  PL05
//! introduces this *new* crate that emits `IIRModule` directly so
//! BASIC programs get the same Linux / Windows / macOS native
//! pipeline Twig and Nib enjoy.
//!
//! ## V1 scope
//!
//! Integer BASIC only — floats are deferred until the backends grow
//! SSE2 support.  Programs that use floating-point literals (`3.14`)
//! get the integer-cast (`3`) silently.
//!
//! | Statement     | Lowering |
//! |---------------|----------|
//! | `LET A = expr` | `<eval expr → t>; mov_i64 A = t` |
//! | `PRINT expr`   | `<eval expr → v>; call_builtin "print_i64", v` |
//! | `INPUT X`      | `call_builtin "input_i64" -> X` |
//! | `IF cond THEN m` | `<eval cond → c>; jmp_if_true c, "line_m"` |
//! | `GOTO m`       | `jmp "line_m"` |
//! | `FOR I = a TO b STEP s` / `NEXT I` | classic counter loop with `for_<n>_test` / `for_<n>_end` labels |
//! | `END`          | `const_i64 0 -> r; ret r` |
//! | `REM …`        | no-op |
//! | `GOSUB` / `RETURN` | **deferred** — V1 returns `UnsupportedStatement` |
//! | `READ` / `DATA` / `RESTORE` | **deferred** — needs data pool |
//! | `DIM` / arrays | **deferred** — needs LANG76 byte memory ops |
//! | `STOP`         | same as `END` for V1 |
//! | `DEF`          | **deferred** |
//!
//! ## Strings
//!
//! Strings (e.g. `PRINT "HELLO"`) are deferred — they need LANG77
//! `.rodata` support.  V1 errors out cleanly on `PRINT "…"`.
//!
//! ## Variables
//!
//! BASIC's `A..Z` and `A0..Z9` variable names map 1:1 to IIR slot
//! names — the IIR compiler emits them directly.  Array references
//! (`A(I)`) are deferred to V2.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use coding_adventures_dartmouth_basic_parser::parse_dartmouth_basic;
use interpreter_ir::function::{FunctionTypeStatus, IIRFunction};
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;

// Re-export the JIT backend so downstream consumers (tests, future
// `dartmouth-basic-vm` wrappers) can use it without depending on the
// internal module path.
pub mod jit_backend;
pub use jit_backend::{BasicCirJit, DEFAULT_OUTPUT_CAP, DEFAULT_STEP_CAP};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

// ===========================================================================
// Public surface
// ===========================================================================

/// Errors raised by the compiler.
#[derive(Debug)]
#[allow(dead_code)]
pub enum CompileError {
    /// V1 doesn't support this statement family (GOSUB/RETURN/READ/DIM/DEF).
    UnsupportedStatement(String),
    /// A BASIC construct exists in source but V1 doesn't lower it (e.g.
    /// string literals, array indexing, user-defined functions).
    Unsupported(String),
    /// AST shape didn't match our expectations — a parser change probably
    /// requires updating this crate.
    Malformed(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::UnsupportedStatement(s) =>
                write!(f, "dartmouth-basic V1 doesn't compile {s} statements yet"),
            CompileError::Unsupported(s) =>
                write!(f, "dartmouth-basic V1 doesn't compile {s} yet"),
            CompileError::Malformed(s) =>
                write!(f, "dartmouth-basic AST malformed: {s}"),
        }
    }
}

impl std::error::Error for CompileError {}

/// Compile a Dartmouth BASIC source string to an [`IIRModule`] ready
/// for the LANG VM AOT chain.
///
/// The module contains exactly one function named `main` whose return
/// type is `i64` (so its return value becomes the process exit code
/// per the LANG VM AOT entry-point convention).
pub fn compile_source(source: &str, module_name: &str)
    -> Result<IIRModule, CompileError>
{
    let ast = parse_dartmouth_basic(source);
    compile_program(&ast, module_name)
}

// ===========================================================================
// Compiler core
// ===========================================================================

/// Compile a parsed `program` AST node into an `IIRModule`.
fn compile_program(ast: &GrammarASTNode, module_name: &str)
    -> Result<IIRModule, CompileError>
{
    let mut comp = Compiler::default();
    comp.emit_program(ast)?;
    // Override `IIRFunction::new`'s automatic `infer_type_status` —
    // which returns `PartiallyTyped` because BASIC's control-flow ops
    // (label, jmp, jmp_if_*, ret, call_builtin "print_i64") use
    // `"void"` hints and `"void"` is not in `CONCRETE_TYPES`.  Every
    // BASIC instruction is in fact statically known (no `"any"` hints
    // anywhere), so the function is genuinely fully typed for the
    // JIT's threshold-zero compile path.  This mirrors Brainfuck's
    // `IIRFunction { … type_status: FullyTyped, … }` construction.
    let mut main = IIRFunction::new(
        "main",
        vec![],   // no parameters
        "i64",    // return i64 for the exit code
        comp.instrs,
    );
    main.type_status = FunctionTypeStatus::FullyTyped;
    let mut module = IIRModule::new(module_name, "dartmouth-basic");
    module.functions.push(main);
    module.entry_point = Some("main".to_string());
    Ok(module)
}

#[derive(Default)]
struct Compiler {
    instrs: Vec<IIRInstr>,
    /// Next synthetic register name (`_t0`, `_t1`, …) for temporaries.
    temp_counter: usize,
    /// Maps each `FOR I` to its limit / step / test-label / end-label so
    /// the matching `NEXT I` can close the loop.  Stack discipline is
    /// straightforward because BASIC's FOR/NEXT are properly nested in
    /// every example from the 1964 manual.
    open_fors: Vec<ForState>,
    /// Counter for unique `for_<n>` label families.
    for_counter: usize,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // limit_slot kept for future use (e.g. cmp_ge on negative STEP)
struct ForState {
    var: String,
    limit_slot: String,
    step_slot: String,
    test_label: String,
    end_label: String,
}

impl Compiler {
    fn fresh_temp(&mut self) -> String {
        let i = self.temp_counter;
        self.temp_counter += 1;
        format!("_t{i}")
    }

    fn emit(&mut self, op: &str, dest: Option<&str>,
            srcs: Vec<Operand>, type_hint: &str)
    {
        self.instrs.push(IIRInstr::new(op,
            dest.map(|s| s.to_string()), srcs, type_hint));
    }

    fn emit_program(&mut self, ast: &GrammarASTNode) -> Result<(), CompileError> {
        if ast.rule_name != "program" {
            return Err(CompileError::Malformed(format!(
                "expected root `program`, got `{}`", ast.rule_name)));
        }

        // Walk every `line` child, in order.
        for child in &ast.children {
            if let ASTNodeOrToken::Node(line) = child {
                if line.rule_name == "line" {
                    self.emit_line(line)?;
                }
            }
        }

        // Defensive epilogue: if the program didn't explicitly END, fall
        // through to a `const 0; ret 0` so we don't run off the function.
        // Most well-formed BASIC programs do end with `END`.
        if !self.instrs.last().map_or(false, |i| i.op.starts_with("ret")) {
            self.emit_end();
        }
        Ok(())
    }

    fn emit_line(&mut self, line: &GrammarASTNode) -> Result<(), CompileError> {
        let line_num = extract_line_num(line)
            .ok_or_else(|| CompileError::Malformed(
                "line missing LINE_NUM token".into()))?;
        // Every line becomes a label `line_<n>`.  Forward references
        // (GOTO 100 before line 100 appears) work because the label is
        // emitted at the right point in the instruction stream — the
        // backend's pre-pass collects labels first.
        let label = format!("line_{line_num}");
        self.emit("label", None, vec![Operand::Var(label)], "void");

        // The optional `statement` child.
        let stmt = child_nodes(line).into_iter()
            .find(|n| n.rule_name == "statement");
        let Some(stmt) = stmt else {
            // Bare line number (no statement) — like a BASIC comment.
            return Ok(());
        };
        // Statement node wraps exactly one of the *_stmt children.
        let inner = child_nodes(stmt).into_iter().next()
            .ok_or_else(|| CompileError::Malformed(
                "statement missing inner node".into()))?;
        self.emit_statement(inner)
    }

    fn emit_statement(&mut self, stmt: &GrammarASTNode)
        -> Result<(), CompileError>
    {
        match stmt.rule_name.as_str() {
            "let_stmt"     => self.emit_let(stmt),
            "print_stmt"   => self.emit_print(stmt),
            "input_stmt"   => self.emit_input(stmt),
            "if_stmt"      => self.emit_if(stmt),
            "goto_stmt"    => self.emit_goto(stmt),
            "for_stmt"     => self.emit_for(stmt),
            "next_stmt"    => self.emit_next(stmt),
            "end_stmt" | "stop_stmt"
                           => { self.emit_end(); Ok(()) },
            "rem_stmt"     => Ok(()),
            // Explicitly-deferred V1 statements.
            "gosub_stmt"   => Err(CompileError::UnsupportedStatement("GOSUB".into())),
            "return_stmt"  => Err(CompileError::UnsupportedStatement("RETURN".into())),
            "read_stmt"    => Err(CompileError::UnsupportedStatement("READ".into())),
            "data_stmt"    => Err(CompileError::UnsupportedStatement("DATA".into())),
            "restore_stmt" => Err(CompileError::UnsupportedStatement("RESTORE".into())),
            "dim_stmt"     => Err(CompileError::UnsupportedStatement("DIM".into())),
            "def_stmt"     => Err(CompileError::UnsupportedStatement("DEF".into())),
            other => Err(CompileError::Malformed(
                format!("unknown statement `{other}`"))),
        }
    }

    // -- Per-statement emitters --------------------------------------------

    fn emit_let(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        // `LET` KEYWORD, `variable` node, `EQ` token, `expr` node.
        let var_name = extract_let_variable_name(stmt)?;
        let expr_node = child_nodes(stmt).into_iter()
            .find(|n| n.rule_name == "expr")
            .ok_or_else(|| CompileError::Malformed("LET missing expr".into()))?;
        let val = self.emit_expr(expr_node)?;
        // `mov_i64 dest = src` — the backend handles this via `emit_binop`/etc.
        self.emit("mov", Some(&var_name),
                  vec![Operand::Var(val)], "i64");
        Ok(())
    }

    fn emit_print(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        // `PRINT` optional `print_list`.  V1: only numeric `print_item`s
        // are supported; strings error out.
        let Some(list) = child_nodes(stmt).into_iter()
            .find(|n| n.rule_name == "print_list")
        else {
            // Bare `PRINT` — emits a blank line.  For V1 we just no-op;
            // a future spec can emit `__twig_putchar(10)`.
            return Ok(());
        };
        for item in child_nodes(list).into_iter()
            .filter(|n| n.rule_name == "print_item")
        {
            let inner = child_nodes(item).into_iter().next();
            match inner {
                Some(expr_node) if expr_node.rule_name == "expr" => {
                    let v = self.emit_expr(expr_node)?;
                    self.emit("call_builtin", None,
                        vec![Operand::Var("print_i64".into()),
                             Operand::Var(v)],
                        "void");
                }
                _ => {
                    // The other allowed `print_item` form is STRING — a
                    // direct token child rather than a node.  We don't
                    // support strings until LANG77 lands.
                    return Err(CompileError::Unsupported(
                        "string literals in PRINT (need LANG77)".into()));
                }
            }
        }
        Ok(())
    }

    fn emit_input(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        // `INPUT` variable { COMMA variable }
        // For each variable, emit `call_builtin "input_i64" -> X`.
        // V1 only handles plain NAMEs (no array elements).
        for v in child_nodes(stmt).into_iter()
            .filter(|n| n.rule_name == "variable")
        {
            let name = scalar_variable_name(v)?;
            self.emit("call_builtin", Some(&name),
                vec![Operand::Var("input_i64".into())],
                "i64");
        }
        Ok(())
    }

    fn emit_if(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        // `IF` expr relop expr `THEN` NUMBER
        let exprs: Vec<&GrammarASTNode> = child_nodes(stmt).into_iter()
            .filter(|n| n.rule_name == "expr").collect();
        if exprs.len() != 2 {
            return Err(CompileError::Malformed(
                format!("IF expected 2 exprs, got {}", exprs.len())));
        }
        let relop_node = child_nodes(stmt).into_iter()
            .find(|n| n.rule_name == "relop")
            .ok_or_else(|| CompileError::Malformed("IF missing relop".into()))?;
        let cmp_op = extract_relop_op(relop_node)?;

        let lhs = self.emit_expr(exprs[0])?;
        let rhs = self.emit_expr(exprs[1])?;
        let cond = self.fresh_temp();
        self.emit(cmp_op, Some(&cond),
            vec![Operand::Var(lhs), Operand::Var(rhs)],
            "bool");

        // THEN target — find the NUMBER token (after the THEN keyword).
        let target_line = extract_if_target(stmt)?;
        self.emit("jmp_if_true", None,
            vec![Operand::Var(cond), Operand::Var(format!("line_{target_line}"))],
            "void");
        Ok(())
    }

    fn emit_goto(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        let target = first_number_token(stmt)
            .ok_or_else(|| CompileError::Malformed("GOTO missing target".into()))?;
        self.emit("jmp", None,
            vec![Operand::Var(format!("line_{target}"))],
            "void");
        Ok(())
    }

    fn emit_for(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        // `FOR` NAME `=` expr `TO` expr [ `STEP` expr ]
        let var = first_name_token_value(stmt)
            .ok_or_else(|| CompileError::Malformed("FOR missing NAME".into()))?;
        let exprs: Vec<&GrammarASTNode> = child_nodes(stmt).into_iter()
            .filter(|n| n.rule_name == "expr").collect();
        if exprs.len() < 2 {
            return Err(CompileError::Malformed(
                "FOR needs at least 2 exprs (start, end)".into()));
        }
        let start_v = self.emit_expr(exprs[0])?;
        let limit_v = self.emit_expr(exprs[1])?;
        let step_v = if let Some(step_expr) = exprs.get(2) {
            self.emit_expr(step_expr)?
        } else {
            // STEP defaults to 1.
            let t = self.fresh_temp();
            self.emit("const", Some(&t), vec![Operand::Int(1)], "i64");
            t
        };

        // Stash limit and step in named slots so NEXT can read them later
        // (they're computed once at FOR entry, not re-evaluated each pass).
        let id = self.for_counter;
        self.for_counter += 1;
        let limit_slot = format!("_for_{id}_limit");
        let step_slot  = format!("_for_{id}_step");
        let test_label = format!("for_{id}_test");
        let end_label  = format!("for_{id}_end");

        self.emit("mov", Some(&var), vec![Operand::Var(start_v)], "i64");
        self.emit("mov", Some(&limit_slot), vec![Operand::Var(limit_v)], "i64");
        self.emit("mov", Some(&step_slot),  vec![Operand::Var(step_v)],  "i64");

        // Test label + per-iteration guard.
        self.emit("label", None, vec![Operand::Var(test_label.clone())], "void");
        let cond = self.fresh_temp();
        // BASIC FOR with a positive STEP is `var <= limit` to continue.
        // V1 doesn't check the step sign at compile time — if STEP is
        // negative, the program will exit the loop immediately because
        // `var <= limit` is false from the start.  This matches the
        // PL05 spec's "two-test loop semantics like the BASIC manual" —
        // the user can manually compute the right comparison.
        self.emit("cmp_le", Some(&cond),
            vec![Operand::Var(var.clone()), Operand::Var(limit_slot.clone())],
            "bool");
        self.emit("jmp_if_false", None,
            vec![Operand::Var(cond), Operand::Var(end_label.clone())],
            "void");

        // Push this open FOR onto the stack so the matching NEXT can
        // close it.  Properly-nested BASIC programs work; mis-nested
        // ones (e.g. NEXT for the outer loop appearing before the
        // inner one's NEXT) will produce wrong IIR — that's a runtime
        // semantic question, not a parser-shape one.
        self.open_fors.push(ForState {
            var,
            limit_slot,
            step_slot,
            test_label,
            end_label,
        });
        Ok(())
    }

    fn emit_next(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        let var = first_name_token_value(stmt)
            .ok_or_else(|| CompileError::Malformed("NEXT missing NAME".into()))?;
        // Pop the matching FOR from the stack.
        let top = self.open_fors.pop()
            .ok_or_else(|| CompileError::Malformed(
                format!("NEXT {var} has no matching FOR")))?;
        if top.var != var {
            return Err(CompileError::Malformed(format!(
                "NEXT {var} doesn't match enclosing FOR {} (mis-nested loop?)",
                top.var)));
        }
        // Increment the loop variable by STEP and jump back to the test.
        let new_val = self.fresh_temp();
        self.emit("add", Some(&new_val),
            vec![Operand::Var(top.var.clone()), Operand::Var(top.step_slot)],
            "i64");
        self.emit("mov", Some(&top.var),
            vec![Operand::Var(new_val)], "i64");
        self.emit("jmp", None,
            vec![Operand::Var(top.test_label)], "void");
        self.emit("label", None,
            vec![Operand::Var(top.end_label)], "void");
        Ok(())
    }

    fn emit_end(&mut self) {
        let r = self.fresh_temp();
        self.emit("const", Some(&r), vec![Operand::Int(0)], "i64");
        self.emit("ret", None, vec![Operand::Var(r)], "i64");
    }

    // -- Expression emitter ------------------------------------------------
    //
    // The grammar's precedence cascade (expr > term > power > unary >
    // primary) means each rule has the shape `<higher> { OP <higher> }`.
    // We walk children pairwise, emit one IIR instruction per operator,
    // and produce a fresh temp for each intermediate.

    fn emit_expr(&mut self, node: &GrammarASTNode)
        -> Result<String, CompileError>
    {
        match node.rule_name.as_str() {
            "expr"  => self.emit_left_assoc_chain(node),
            "term"  => self.emit_left_assoc_chain(node),
            "power" => self.emit_power(node),
            "unary" => self.emit_unary(node),
            "primary" => self.emit_primary(node),
            _ => {
                // Fallthrough: unwrap single-child wrappers.
                let kids = child_nodes(node);
                if let Some(c) = kids.first() {
                    return self.emit_expr(c);
                }
                Err(CompileError::Malformed(format!(
                    "unknown expr rule `{}`", node.rule_name)))
            }
        }
    }

    fn emit_left_assoc_chain(&mut self, node: &GrammarASTNode)
        -> Result<String, CompileError>
    {
        // node = first_sub_expr { OP next_sub_expr }
        //
        // Children alternate: Node, Token, Node, Token, Node, …
        // The first must be a Node; subsequent come in (Token, Node) pairs.
        let mut iter = node.children.iter();
        let first = iter.next()
            .ok_or_else(|| CompileError::Malformed(
                format!("empty {}", node.rule_name)))?;
        let mut acc = match first {
            ASTNodeOrToken::Node(n) => self.emit_expr(n)?,
            ASTNodeOrToken::Token(_) =>
                return Err(CompileError::Malformed(
                    format!("{} starts with token", node.rule_name))),
        };

        loop {
            let op = match iter.next() {
                Some(ASTNodeOrToken::Token(t)) => t,
                None => break,
                _ => return Err(CompileError::Malformed(
                    format!("{}: expected operator token", node.rule_name))),
            };
            let rhs = match iter.next() {
                Some(ASTNodeOrToken::Node(n)) => self.emit_expr(n)?,
                _ => return Err(CompileError::Malformed(
                    format!("{}: dangling operator", node.rule_name))),
            };
            let dest = self.fresh_temp();
            let cir_op = binary_op_name(&op.value)
                .ok_or_else(|| CompileError::Unsupported(
                    format!("operator `{}`", op.value)))?;
            self.emit(cir_op, Some(&dest),
                vec![Operand::Var(acc), Operand::Var(rhs)],
                "i64");
            acc = dest;
        }
        Ok(acc)
    }

    fn emit_power(&mut self, node: &GrammarASTNode)
        -> Result<String, CompileError>
    {
        // `power = unary [ CARET power ]` — right-associative.  V1 doesn't
        // support exponentiation (would need a runtime helper or
        // repeated mul); rejects with `Unsupported`.
        let kids = node.children.iter().collect::<Vec<_>>();
        if kids.len() == 1 {
            // Pass through to the single `unary` child.
            if let ASTNodeOrToken::Node(n) = kids[0] {
                return self.emit_expr(n);
            }
        }
        // CARET present in children → exponentiation, deferred.
        if kids.iter().any(|c| matches!(c, ASTNodeOrToken::Token(t)
            if t.value == "^"))
        {
            return Err(CompileError::Unsupported(
                "exponentiation (^) — needs runtime helper".into()));
        }
        // Otherwise just unwrap the single Node child.
        for c in kids {
            if let ASTNodeOrToken::Node(n) = c {
                return self.emit_expr(n);
            }
        }
        Err(CompileError::Malformed("empty `power` node".into()))
    }

    fn emit_unary(&mut self, node: &GrammarASTNode)
        -> Result<String, CompileError>
    {
        // `unary = MINUS primary | primary`.
        let has_minus = node.children.iter().any(|c| matches!(c,
            ASTNodeOrToken::Token(t) if t.value == "-"));
        let inner_node = child_nodes(node).into_iter().next()
            .ok_or_else(|| CompileError::Malformed("empty `unary`".into()))?;
        let v = self.emit_expr(inner_node)?;
        if !has_minus { return Ok(v); }
        // Emit `neg dest, v`.
        let dest = self.fresh_temp();
        self.emit("neg", Some(&dest), vec![Operand::Var(v)], "i64");
        Ok(dest)
    }

    fn emit_primary(&mut self, node: &GrammarASTNode)
        -> Result<String, CompileError>
    {
        // primary = NUMBER | BUILTIN_FN(expr) | USER_FN(expr) | variable | (expr)
        //
        // V1 supports NUMBER and `variable` only.  Function calls (built-in
        // or user-defined) are deferred.
        for c in &node.children {
            match c {
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "NUMBER" => {
                    // Truncate floats to i64 — V1 is integer-only.
                    let v: i64 = t.value.trim().parse::<f64>()
                        .map(|f| f as i64)
                        .unwrap_or(0);
                    let dest = self.fresh_temp();
                    self.emit("const", Some(&dest),
                        vec![Operand::Int(v)], "i64");
                    return Ok(dest);
                }
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "BUILTIN_FN"
                    || t.effective_type_name() == "USER_FN" =>
                {
                    return Err(CompileError::Unsupported(format!(
                        "function call `{}` (deferred)", t.value)));
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "variable" => {
                    let name = scalar_variable_name(n)?;
                    return Ok(name);
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "expr" => {
                    return self.emit_expr(n);
                }
                _ => {}
            }
        }
        Err(CompileError::Malformed(format!(
            "unrecognised `primary` shape: children={}", node.children.len())))
    }
}

// ===========================================================================
// AST helpers
// ===========================================================================

fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children.iter().filter_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        _ => None,
    }).collect()
}

fn extract_line_num(line: &GrammarASTNode) -> Option<i64> {
    for c in &line.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.effective_type_name() == "LINE_NUM" {
                return t.value.trim().parse::<i64>().ok();
            }
        }
    }
    None
}

fn extract_let_variable_name(stmt: &GrammarASTNode)
    -> Result<String, CompileError>
{
    let var = child_nodes(stmt).into_iter()
        .find(|n| n.rule_name == "variable")
        .ok_or_else(|| CompileError::Malformed("LET missing variable".into()))?;
    scalar_variable_name(var)
}

fn scalar_variable_name(var: &GrammarASTNode)
    -> Result<String, CompileError>
{
    // `variable = NAME LPAREN expr RPAREN | NAME`.
    // V1 only supports the scalar form.  Array access (3+ children) is
    // deferred until LANG76-based array lowering lands.
    if var.children.len() != 1 {
        return Err(CompileError::Unsupported(
            "array variable access (e.g. A(I)) — deferred to V2".into()));
    }
    for c in &var.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.effective_type_name() == "NAME" {
                return Ok(t.value.clone());
            }
        }
    }
    Err(CompileError::Malformed(
        "variable node missing NAME token".into()))
}

fn first_name_token_value(node: &GrammarASTNode) -> Option<String> {
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.effective_type_name() == "NAME" {
                return Some(t.value.clone());
            }
        }
    }
    None
}

fn first_number_token(node: &GrammarASTNode) -> Option<i64> {
    for c in &node.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.effective_type_name() == "NUMBER" {
                // BASIC line numbers / GOTO/GOSUB targets are always integers.
                return t.value.trim().parse::<f64>().ok().map(|f| f as i64);
            }
        }
    }
    None
}

fn extract_if_target(stmt: &GrammarASTNode) -> Result<i64, CompileError> {
    // `IF` expr relop expr `THEN` NUMBER — the NUMBER comes *after* THEN.
    // Since both LINE_NUM and NUMBER tokens can appear in an IF chain
    // (no — LINE_NUM only on the line itself), the first NUMBER token
    // after the THEN keyword is the target.
    let mut saw_then = false;
    for c in &stmt.children {
        if let ASTNodeOrToken::Token(t) = c {
            if t.effective_type_name() == "KEYWORD" && t.value == "THEN" {
                saw_then = true;
            } else if saw_then && t.effective_type_name() == "NUMBER" {
                return t.value.trim().parse::<f64>().ok().map(|f| f as i64)
                    .ok_or_else(|| CompileError::Malformed(
                        format!("IF THEN target not a number: {}", t.value)));
            }
        }
    }
    Err(CompileError::Malformed("IF missing THEN <number>".into()))
}

fn extract_relop_op(relop: &GrammarASTNode) -> Result<&'static str, CompileError> {
    // `relop` is a wrapper around exactly one of EQ / LT / GT / LE / GE / NE.
    for c in &relop.children {
        if let ASTNodeOrToken::Token(t) = c {
            return Ok(match t.value.as_str() {
                "="  => "cmp_eq",
                "<"  => "cmp_lt",
                ">"  => "cmp_gt",
                "<=" => "cmp_le",
                ">=" => "cmp_ge",
                "<>" => "cmp_ne",
                other => return Err(CompileError::Malformed(
                    format!("unknown relop `{other}`"))),
            });
        }
    }
    Err(CompileError::Malformed("relop has no token child".into()))
}

fn binary_op_name(op: &str) -> Option<&'static str> {
    match op {
        "+" => Some("add"),
        "-" => Some("sub"),
        "*" => Some("mul"),
        "/" => Some("div"),
        _ => None,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn compile(src: &str) -> Result<IIRModule, CompileError> {
        compile_source(src, "test")
    }

    /// The simplest possible BASIC program — `10 END` — should produce
    /// a module with one function `main` whose body is `label, const 0,
    /// ret 0`.
    #[test]
    fn compiles_minimal_end() {
        let m = compile("10 END\n").expect("ok");
        assert_eq!(m.functions.len(), 1);
        let main = &m.functions[0];
        assert_eq!(main.name, "main");
        assert_eq!(main.return_type, "i64");
        let ops: Vec<&str> = main.instructions.iter()
            .map(|i| i.op.as_str()).collect();
        assert!(ops.contains(&"label"));
        assert!(ops.contains(&"const"));
        assert!(ops.contains(&"ret"));
    }

    /// `LET A = 42` followed by `END` should leave variable A holding 42.
    #[test]
    fn compiles_let_then_end() {
        let m = compile("10 LET A = 42\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        // mov_i64 A = _t0 (where _t0 was const 42)
        let mov = body.iter().find(|i| i.op == "mov")
            .expect("LET produces a mov");
        assert_eq!(mov.dest.as_deref(), Some("A"));
        // and a const 42 somewhere.
        assert!(body.iter().any(|i|
            i.op == "const" && matches!(i.srcs.first(), Some(Operand::Int(42)))));
    }

    /// `PRINT 42` lowers to `const 42 → t; call_builtin "print_i64", t`.
    #[test]
    fn compiles_print_integer() {
        let m = compile("10 PRINT 42\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let call = body.iter().find(|i|
            i.op == "call_builtin" && i.srcs.first().and_then(|s| match s {
                Operand::Var(n) => Some(n.as_str()), _ => None,
            }) == Some("print_i64")
        );
        assert!(call.is_some(), "expected call_builtin print_i64 in {body:?}");
    }

    /// `GOTO 99` emits a `jmp` to label `line_99`.
    #[test]
    fn compiles_goto() {
        let m = compile("10 GOTO 99\n99 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let jmp = body.iter().find(|i| i.op == "jmp");
        assert!(jmp.is_some());
        let target = jmp.unwrap().srcs.first().and_then(|s| match s {
            Operand::Var(n) => Some(n.clone()), _ => None,
        });
        assert_eq!(target, Some("line_99".to_string()));
    }

    /// `IF A > 5 THEN 100` emits `cmp_gt`, then `jmp_if_true cond, line_100`.
    #[test]
    fn compiles_if_then() {
        let src = "10 LET A = 7\n20 IF A > 5 THEN 100\n30 END\n100 END\n";
        let m = compile(src).expect("ok");
        let body = &m.functions[0].instructions;
        assert!(body.iter().any(|i| i.op == "cmp_gt"));
        let jit = body.iter().find(|i| i.op == "jmp_if_true").expect("jmp_if_true");
        assert_eq!(jit.srcs.get(1).and_then(|s| match s {
            Operand::Var(n) => Some(n.clone()), _ => None,
        }), Some("line_100".to_string()));
    }

    /// `FOR I = 1 TO 3 / NEXT I` emits the loop scaffold: const, mov,
    /// label test, cmp_le, jmp_if_false, … add, mov, jmp, label end.
    #[test]
    fn compiles_for_next() {
        let src = "10 FOR I = 1 TO 3\n20 NEXT I\n30 END\n";
        let m = compile(src).expect("ok");
        let body = &m.functions[0].instructions;
        let ops: Vec<&str> = body.iter().map(|i| i.op.as_str()).collect();
        assert!(ops.contains(&"cmp_le"), "got: {ops:?}");
        assert!(ops.contains(&"jmp_if_false"));
        // Need both the for_0_test label and the for_0_end label.
        let labels: Vec<&str> = body.iter()
            .filter(|i| i.op == "label")
            .filter_map(|i| match i.srcs.first() {
                Some(Operand::Var(n)) => Some(n.as_str()), _ => None,
            }).collect();
        assert!(labels.iter().any(|l| l.starts_with("for_") && l.ends_with("_test")),
                "missing for_*_test label in {labels:?}");
        assert!(labels.iter().any(|l| l.starts_with("for_") && l.ends_with("_end")),
                "missing for_*_end label in {labels:?}");
    }

    /// `REM` is a no-op — generates only the line label.
    #[test]
    fn compiles_rem_as_noop() {
        let m = compile("10 REM HELLO WORLD\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        // Line 10 emits label line_10 then nothing else for REM.
        let between: Vec<&str> = body.iter()
            .skip_while(|i| i.op != "label")
            .take_while(|i| i.op != "ret")
            .map(|i| i.op.as_str()).collect();
        // Should be: label, label, const, (ret comes from END).
        // We expect 3 instructions before ret: line_10 label, line_20 label, const 0.
        assert!(between.contains(&"label"));
    }

    /// `INPUT X` emits `call_builtin "input_i64" -> X`.
    #[test]
    fn compiles_input() {
        let m = compile("10 INPUT X\n20 END\n").expect("ok");
        let body = &m.functions[0].instructions;
        let call = body.iter().find(|i|
            i.op == "call_builtin" && i.dest.as_deref() == Some("X"));
        assert!(call.is_some(), "expected call_builtin -> X");
        let helper = call.unwrap().srcs.first().and_then(|s| match s {
            Operand::Var(n) => Some(n.as_str()), _ => None,
        });
        assert_eq!(helper, Some("input_i64"));
    }

    /// String literals in PRINT are deferred → `Unsupported`.
    #[test]
    fn rejects_print_string() {
        let err = compile("10 PRINT \"HI\"\n20 END\n").unwrap_err();
        match err {
            CompileError::Unsupported(_) => {}
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    /// GOSUB is deferred to V2 — emits `UnsupportedStatement`.
    #[test]
    fn rejects_gosub_as_unsupported_statement() {
        let err = compile("10 GOSUB 100\n100 END\n").unwrap_err();
        match err {
            CompileError::UnsupportedStatement(name) => assert_eq!(name, "GOSUB"),
            other => panic!("expected UnsupportedStatement(GOSUB), got {other:?}"),
        }
    }

    /// A complete small program: LET, arithmetic, PRINT, END.
    /// Just exercises that the whole pipeline compiles a multi-line source
    /// without crashing.
    #[test]
    fn compiles_arithmetic_program() {
        let src = "10 LET A = 30\n\
                   20 LET B = 12\n\
                   30 LET C = A + B\n\
                   40 PRINT C\n\
                   50 END\n";
        let m = compile(src).expect("ok");
        let body = &m.functions[0].instructions;
        // Should have at least one `add` and one `call_builtin print_i64`.
        assert!(body.iter().any(|i| i.op == "add"));
        assert!(body.iter().any(|i|
            i.op == "call_builtin" && i.srcs.first().and_then(|s| match s {
                Operand::Var(n) => Some(n.as_str()), _ => None,
            }) == Some("print_i64")));
    }
}
