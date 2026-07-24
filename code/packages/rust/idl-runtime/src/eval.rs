//! The tree-walking evaluator.
//!
//! [`Interpreter`] walks the [`GrammarASTNode`] CST from `idl-parser` and
//! computes values over [`IdlValue`] (`crate::value`). Per
//! `code/specs/MA12-idl-language.md` §3, IDL's *definition* side -- a named
//! callable with declared parameters, a multi-statement body, and a call
//! scope frame -- is "the same shape Q's `QFn::Lambda` already established"
//! (MA11 §2): this evaluator reuses that base mechanism (an environment
//! *stack* of variable-binding frames, a call pushes one and pops it via an
//! RAII guard on every exit path, a recursion-depth guard around every
//! recursive entry point) rather than re-deriving it. What is genuinely new
//! here, layered on top of that base (MA12 §3's own framing):
//!
//! 1. **`IdlCallable`: two separate namespaces.** Real IDL allows the same
//!    name to be both a `PRO` and a `FUNCTION` simultaneously, and
//!    `idl-parser`'s CST already structurally distinguishes the two call
//!    sites (`procedure_call_stmt` vs. an expression's `call_suffix`) -- so
//!    this evaluator keeps **two** dispatch tables, `procs`/`funcs`, and
//!    routes each call site to its own table. See
//!    [`Interpreter::eval_procedure_call`]/[`Interpreter::eval_function_call`].
//! 2. **Keyword-argument binding.** A call's mixed positional/keyword
//!    argument list is bound onto a callable's declared
//!    positional/keyword parameter list by *name* match (not position) for
//!    the keyword half -- see [`Interpreter::call_user_routine`].
//! 3. **No automatic outer/global visibility inside a call.** Unlike Q
//!    (whose lambda body falls back to the *global* frame for any
//!    non-parameter name, MA11 §4), MA12 §4 explicitly defers `COMMON`
//!    blocks and states plainly that "this cut's callables have their own
//!    local scope frame and read/write only their parameters, keywords, and
//!    locals" -- so, unlike `q-runtime::eval::Interpreter::lookup` (which
//!    searches every frame from innermost to the global frame), this
//!    evaluator's [`Interpreter::lookup`]/[`Interpreter::assign`] only ever
//!    touch the environment stack's **top** frame -- the global frame at
//!    the true top level, or the fresh, isolated frame a call just pushed.
//!    A routine calling another routine gets a *brand new* isolated frame,
//!    not one stacked with fallback onto the caller's own locals.
//!
//! Control flow (`IF`/`FOR`/`WHILE`/`REPEAT`/`BREAK`/`CONTINUE`/`RETURN`) has
//! no Q precedent at all (Q is expression-only) -- modeled here the
//! ordinary way a tree-walking imperative interpreter does, via a small
//! [`Flow`] signal threaded up through every nested statement execution.
//!
//! ## Case folding: **yes**, identifiers are folded to uppercase at bind/lookup time
//!
//! `idl-lexer`'s own README explicitly flagged this as an open MA-12d
//! decision: "whether `idl-runtime`'s symbol table should itself fold
//! identifier case at lookup time... is a runtime-layer decision for
//! MA-12d, not a lexer concern." This crate decides **yes**, and the
//! decision is *verified*, not guessed: real IDL is documented as
//! case-insensitive for its entire language surface, folding to uppercase
//! **internally** for both variable names and routine names -- e.g. NV5
//! Geospatial's own support article on resolving file-name case issues
//! states plainly that IDL "converts procedure names to uppercase
//! internally" (confirmed directly against NV5 Geospatial documentation
//! during this session, not assumed). Every identifier this evaluator binds
//! or looks up -- variable names, `PRO`/`FUNCTION` names, parameter/keyword
//! names -- is folded via [`fold_case`] (`str::to_uppercase`) at the point
//! it is first read off a `NAME` token, so `myVar`/`MYVAR`/`MyVar` are one
//! binding and `PLOT`/`plot`/`Plot` dispatch to the same routine. This
//! mirrors `idl-lexer`'s own narrower choice (only `keywords:`-block lookup
//! folds case, ordinary `NAME` tokens keep their exact source spelling) --
//! the *lexer* preserves spelling because folding it there would be a
//! stronger claim than IDL's own rule needs; the *symbol table* (this
//! crate) is exactly the layer real IDL's own case-folding actually lives
//! at, per the citation above.

use crate::builtins::{self};
use crate::value::IdlValue;
use array_runtime::{execute, ops, ops::BinOp, Array, Kernel};
use coding_adventures_idl_parser::try_parse_idl;
use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// Maximum recursion depth for this evaluator's own tree-walk *and* call
/// chain -- mirrors `q_runtime::eval::MAX_DEPTH`'s role exactly (a guard
/// around every recursive `eval_*`/`exec_*`/call entry point, an RAII
/// [`DepthGuard`] that decrements on every exit path including an early `?`
/// return).
///
/// **Disclosed simplification, not independently re-measured**: `q-runtime`
/// arrived at its own `MAX_DEPTH` (760) via a rigorous, documented
/// binary-search measurement of the real native-stack crash floor for its
/// own specific recursion shape (`eval.rs::MAX_DEPTH`'s own doc comment).
/// This crate does not repeat that full measurement -- IDL's evaluator has
/// a different recursion shape again (control-flow nesting *and* call
/// chains *and* the expression cascade all interleave), and reproducing
/// that methodology here was judged disproportionate to this cut's scope.
/// `500` is a reasonable, conservative, generously-large default consistent
/// with every sibling evaluator's own guard existing for the same reason
/// (a crafted, deeply-nested input must fail cleanly rather than
/// overflowing the native stack) -- flagged here as a judgment call, not
/// presented as an empirically-verified number the way `q-runtime`'s is.
const MAX_DEPTH: usize = 500;

/// Which of the two separate namespaces (MA12 §3) a callable lives in --
/// carried on [`IdlCallable`] itself purely for clearer error messages
/// ("PRO GREET" vs. "FUNCTION GREET"); the *actual* dispatch separation is
/// `Interpreter`'s two distinct `procs`/`funcs` tables, not this field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutineKind {
    Procedure,
    Function,
}

impl RoutineKind {
    fn label(self) -> &'static str {
        match self {
            RoutineKind::Procedure => "PRO",
            RoutineKind::Function => "FUNCTION",
        }
    }
}

/// One declared parameter in a `PRO`/`FUNCTION` header (MA12 §3/§4):
/// `param = NAME EQUALS NAME | NAME`.
///
/// - `NAME` alone -> a plain **positional** parameter: `keyword: None`,
///   `local` is that one name (both the call-site "slot" and the
///   body-local variable are the same spelling).
/// - `KEYWORD=local_var_name` -> a **keyword** parameter: `keyword` is the
///   call-site keyword name (`KEYWORD=value` / `/KEYWORD`), `local` is the
///   *body-local* variable name it binds to inside the routine -- which
///   MA12 §4 documents may be a **different spelling** from `KEYWORD`
///   itself (the header's own literal example: `KW=kw`).
#[derive(Debug, Clone)]
struct ParamSpec {
    keyword: Option<String>,
    local: String,
}

/// A user-defined `PRO`/`FUNCTION`: declared parameters plus a
/// multi-statement body, reusing Q's `QFn::Lambda` scope-frame precedent as
/// its base (MA12 §3) -- see this module's own top doc comment for exactly
/// what is reused vs. genuinely new here.
#[derive(Debug)]
pub struct IdlCallable {
    name: String,
    kind: RoutineKind,
    params: Vec<ParamSpec>,
    /// Owned clones of the body's `statement` nodes (mirrors
    /// `q_runtime::eval::Lambda::body`'s identical rationale: a
    /// [`Rc<IdlCallable>`] must outlive the single `feed()`/`run()` call
    /// that parsed it, since routines persist across a REPL session's many
    /// calls).
    body: Vec<GrammarASTNode>,
}

/// The result of running one statement (or a sequence of them): either
/// "keep going" (`Normal`), or one of the three control-flow signals a
/// nested statement can raise, threaded back up to whichever construct
/// knows how to handle it (a loop catches `Break`/`Continue`; only a call
/// boundary -- [`Interpreter::call_user_routine`] -- catches `Return`).
#[derive(Debug)]
enum Flow {
    Normal,
    Break,
    Continue,
    /// `RETURN` (procedure form, no value) or `RETURN, expr` (function
    /// form, `Some(value)`).
    Return(Option<IdlValue>),
}

/// RAII guard decrementing the depth counter on every exit path (including
/// an early `?` return) -- mirrors `q_runtime::eval::DepthGuard` exactly.
struct DepthGuard(Rc<Cell<usize>>);

impl Drop for DepthGuard {
    fn drop(&mut self) {
        self.0.set(self.0.get().saturating_sub(1));
    }
}

/// RAII guard popping the call-local frame [`Interpreter::call_user_routine`]
/// pushes, on every exit path -- mirrors `q_runtime::eval::FrameGuard`.
struct FrameGuard<'a> {
    env: &'a RefCell<Vec<HashMap<String, IdlValue>>>,
}

impl Drop for FrameGuard<'_> {
    fn drop(&mut self) {
        self.env.borrow_mut().pop();
    }
}

/// A persistent IDL session: a stack of variable-binding frames (index 0 is
/// the global frame; a call pushes exactly one fresh, *isolated* frame --
/// see this module's own top doc comment, point 3), two separate
/// `PRO`/`FUNCTION` dispatch tables (MA12 §3), an accumulated output buffer
/// (`PRINT`/Implied-Print both write here, so their output interleaves in
/// the correct order regardless of call depth), and the current evaluation
/// depth.
pub struct Interpreter {
    env: RefCell<Vec<HashMap<String, IdlValue>>>,
    procs: RefCell<HashMap<String, Rc<IdlCallable>>>,
    funcs: RefCell<HashMap<String, Rc<IdlCallable>>>,
    output: RefCell<String>,
    depth: Rc<Cell<usize>>,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            env: RefCell::new(vec![HashMap::new()]),
            procs: RefCell::new(HashMap::new()),
            funcs: RefCell::new(HashMap::new()),
            output: RefCell::new(String::new()),
            depth: Rc::new(Cell::new(0)),
        }
    }

    /// Parse and evaluate one chunk of IDL source, returning the
    /// accumulated `PRINT`/Implied-Print output. Variables and
    /// `PRO`/`FUNCTION` definitions persist across calls (a persistent
    /// session, exactly like `q_runtime::Interpreter::feed`).
    pub fn feed(&self, source: &str) -> Result<String, String> {
        let tree = try_parse_idl(source)?;
        self.run(&tree)
    }

    fn enter(&self) -> Result<DepthGuard, String> {
        self.depth.set(self.depth.get() + 1);
        let guard = DepthGuard(Rc::clone(&self.depth));
        if self.depth.get() > MAX_DEPTH {
            return Err("idl-runtime: expression/statement nesting too deep".to_string());
        }
        Ok(guard)
    }

    /// Append `line` plus a trailing newline to the accumulated output
    /// buffer -- the one place `PRINT` and top-level Implied Print both
    /// write through, so their output interleaves in the correct order no
    /// matter how deep inside a call chain a `PRINT` fires.
    fn emit(&self, line: &str) {
        let mut out = self.output.borrow_mut();
        out.push_str(line);
        out.push('\n');
    }

    /// Look up `name` (already uppercased by the caller) against the
    /// environment stack's **top** frame only -- see this module's own top
    /// doc comment, point 3, for why this does *not* fall back to the
    /// global frame the way `q_runtime::eval::Interpreter::lookup` does.
    fn lookup(&self, name: &str) -> Option<IdlValue> {
        let env = self.env.borrow();
        env.last().and_then(|frame| frame.get(name).cloned())
    }

    /// Bind `name` (already uppercased) in the top frame -- the global
    /// frame at the true top level, or the current call's own isolated
    /// frame.
    fn assign(&self, name: &str, value: IdlValue) {
        let mut env = self.env.borrow_mut();
        let top = env
            .last_mut()
            .expect("env always has at least the global frame");
        top.insert(name.to_string(), value);
    }

    // ── Top-level program / Implied Print ───────────────────────────────

    /// Evaluate a whole `program` node.
    ///
    /// Per NV5 Geospatial's own *Implied Print* reference (confirmed
    /// directly this session): a bare, non-assignment statement typed at
    /// the interactive top level auto-prints its value, an assignment
    /// produces no output, and Implied Print does **not** fire inside a
    /// routine body. This method is therefore the **one** place that
    /// decides "should this bare expression auto-print" -- every statement
    /// nested inside a control-flow block (even one typed directly at the
    /// top level) or inside a `PRO`/`FUNCTION` body goes through the
    /// ordinary, non-printing [`Interpreter::exec_statement`] instead (a
    /// deliberate, disclosed scope simplification: real IDL's Implied Print
    /// may also fire for a bare expression used as a *top-level* loop's own
    /// body; this cut only auto-prints the outermost statement sequence
    /// directly under `program`, not recursively inside nested blocks).
    pub fn run(&self, program: &GrammarASTNode) -> Result<String, String> {
        for top_item in node_children(program) {
            // `top_level_item = pro_def | func_def | statement_line` -- one
            // more unwrap than `program`'s own children, since every rule
            // (per `parser::grammar_parser`) gets its own wrapper node.
            let inner = only_node(top_item)?;
            match inner.rule_name.as_str() {
                "pro_def" => self.register_routine(inner, RoutineKind::Procedure)?,
                "func_def" => self.register_routine(inner, RoutineKind::Function)?,
                "statement_line" => {
                    for stmt in statement_line_statements(inner) {
                        self.exec_top_level_statement(stmt)?;
                    }
                }
                other => return Err(format!("idl-runtime: unexpected top-level node '{other}'")),
            }
        }
        Ok(std::mem::take(&mut *self.output.borrow_mut()))
    }

    /// Execute one top-level `statement` node, auto-printing a bare
    /// `expr_stmt`'s value (Implied Print) and rejecting a
    /// `BREAK`/`CONTINUE`/`RETURN` that escapes all the way to the top
    /// level (there is no enclosing loop or call there to catch it).
    fn exec_top_level_statement(&self, stmt: &GrammarASTNode) -> Result<(), String> {
        let inner = only_node(stmt)?;
        if inner.rule_name == "expr_stmt" {
            let value = self.eval_expr(only_node(inner)?)?;
            self.emit(&crate::value::display(&value));
            return Ok(());
        }
        match self.exec_statement(stmt)? {
            Flow::Normal => Ok(()),
            Flow::Break => Err("idl-runtime: BREAK used outside a loop".to_string()),
            Flow::Continue => Err("idl-runtime: CONTINUE used outside a loop".to_string()),
            Flow::Return(_) => Err("idl-runtime: RETURN used outside a routine".to_string()),
        }
    }

    // ── PRO/FUNCTION registration ────────────────────────────────────────

    fn register_routine(&self, node: &GrammarASTNode, kind: RoutineKind) -> Result<(), String> {
        // pro_def  = "PRO" NAME [ COMMA params ] block_body "END" ;
        // func_def = "FUNCTION" NAME [ COMMA params ] block_body "END" ;
        let name_tok = match node.children.get(1) {
            Some(ASTNodeOrToken::Token(t)) => t,
            _ => {
                return Err(format!(
                    "idl-runtime: malformed '{}' (missing name)",
                    node.rule_name
                ))
            }
        };
        let name = fold_case(&name_tok.value);
        let params = match find_child_opt(node, "params") {
            Some(p) => build_params(p)?,
            None => Vec::new(),
        };
        let block_body = find_child(node, "block_body")?;
        let body: Vec<GrammarASTNode> = block_body_statements(block_body)
            .into_iter()
            .cloned()
            .collect();
        let callable = Rc::new(IdlCallable {
            name: name.clone(),
            kind,
            params,
            body,
        });
        match kind {
            RoutineKind::Procedure => {
                self.procs.borrow_mut().insert(name, callable);
            }
            RoutineKind::Function => {
                self.funcs.borrow_mut().insert(name, callable);
            }
        }
        Ok(())
    }

    // ── Statement execution ──────────────────────────────────────────────

    fn exec_statement(&self, stmt: &GrammarASTNode) -> Result<Flow, String> {
        let _guard = self.enter()?;
        let inner = only_node(stmt)?;
        match inner.rule_name.as_str() {
            "if_stmt" => self.exec_if(inner),
            "for_stmt" => self.exec_for(inner),
            "while_stmt" => self.exec_while(inner),
            "repeat_stmt" => self.exec_repeat(inner),
            "begin_block" => self.exec_begin_block(inner),
            "break_stmt" => Ok(Flow::Break),
            "continue_stmt" => Ok(Flow::Continue),
            "return_stmt" => self.exec_return(inner),
            "procedure_call_stmt" => {
                self.exec_procedure_call(inner)?;
                Ok(Flow::Normal)
            }
            "assignment_stmt" => {
                self.exec_assignment(inner)?;
                Ok(Flow::Normal)
            }
            "expr_stmt" => {
                self.eval_expr(only_node(inner)?)?;
                Ok(Flow::Normal)
            }
            other => Err(format!("idl-runtime: unknown statement kind '{other}'")),
        }
    }

    /// Run a sequence of statements in order, stopping (and propagating)
    /// the first non-`Normal` [`Flow`].
    fn exec_block<'a, I: IntoIterator<Item = &'a GrammarASTNode>>(
        &self,
        stmts: I,
    ) -> Result<Flow, String> {
        for s in stmts {
            match self.exec_statement(s)? {
                Flow::Normal => continue,
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }

    fn exec_if(&self, node: &GrammarASTNode) -> Result<Flow, String> {
        // if_stmt = "IF" expr "THEN" then_branch [ "ELSE" else_branch ] ;
        let cond_node = find_child(node, "expr")?;
        let cond = self.eval_condition(cond_node)?;
        if cond {
            let then_branch = find_child(node, "then_branch")?;
            self.exec_block(body_statements(then_branch)?)
        } else if let Some(else_branch) = find_child_opt(node, "else_branch") {
            self.exec_block(body_statements(else_branch)?)
        } else {
            Ok(Flow::Normal)
        }
    }

    fn exec_for(&self, node: &GrammarASTNode) -> Result<Flow, String> {
        // for_stmt = "FOR" NAME EQUALS expr COMMA expr [ COMMA expr ] "DO" for_body ;
        let var_tok = match node.children.get(1) {
            Some(ASTNodeOrToken::Token(t)) => t,
            _ => return Err("idl-runtime: malformed for_stmt (missing loop variable)".to_string()),
        };
        let var = fold_case(&var_tok.value);
        let expr_nodes: Vec<&GrammarASTNode> = node_children(node)
            .into_iter()
            .filter(|n| n.rule_name == "expr")
            .collect();
        if expr_nodes.len() < 2 {
            return Err("idl-runtime: malformed for_stmt (missing init/limit)".to_string());
        }
        let init = self.eval_scalar(expr_nodes[0])?;
        let limit = self.eval_scalar(expr_nodes[1])?;
        let step = if expr_nodes.len() >= 3 {
            self.eval_scalar(expr_nodes[2])?
        } else {
            1.0
        };
        if step == 0.0 {
            return Err("idl-runtime: FOR step must not be zero".to_string());
        }
        let body = find_child(node, "for_body")?;
        let stmts = body_statements(body)?;

        self.assign(&var, IdlValue::num(init));
        let mut i = init;
        loop {
            let keep_going = if step > 0.0 { i <= limit } else { i >= limit };
            if !keep_going {
                break;
            }
            match self.exec_block(stmts.iter().copied())? {
                Flow::Break => break,
                Flow::Continue | Flow::Normal => {}
                Flow::Return(v) => return Ok(Flow::Return(v)),
            }
            i += step;
            self.assign(&var, IdlValue::num(i));
        }
        Ok(Flow::Normal)
    }

    fn exec_while(&self, node: &GrammarASTNode) -> Result<Flow, String> {
        // while_stmt = "WHILE" expr "DO" while_body ;
        let cond_node = find_child(node, "expr")?;
        let body = find_child(node, "while_body")?;
        let stmts = body_statements(body)?;
        while self.eval_condition(cond_node)? {
            match self.exec_block(stmts.iter().copied())? {
                Flow::Break => break,
                Flow::Continue | Flow::Normal => {}
                Flow::Return(v) => return Ok(Flow::Return(v)),
            }
        }
        Ok(Flow::Normal)
    }

    fn exec_repeat(&self, node: &GrammarASTNode) -> Result<Flow, String> {
        // repeat_stmt = "REPEAT" repeat_body "UNTIL" expr ;
        let body = find_child(node, "repeat_body")?;
        let stmts = body_statements(body)?;
        let cond_node = find_child(node, "expr")?;
        loop {
            match self.exec_block(stmts.iter().copied())? {
                Flow::Break => break,
                Flow::Continue | Flow::Normal => {}
                Flow::Return(v) => return Ok(Flow::Return(v)),
            }
            if self.eval_condition(cond_node)? {
                break;
            }
        }
        Ok(Flow::Normal)
    }

    fn exec_begin_block(&self, node: &GrammarASTNode) -> Result<Flow, String> {
        // begin_block = "BEGIN" block_body "END" ;
        let bb = find_child(node, "block_body")?;
        self.exec_block(block_body_statements(bb))
    }

    fn exec_return(&self, node: &GrammarASTNode) -> Result<Flow, String> {
        // return_stmt = "RETURN" [ COMMA expr ] ;
        match find_child_opt(node, "expr") {
            Some(e) => Ok(Flow::Return(Some(self.eval_expr(e)?))),
            None => Ok(Flow::Return(None)),
        }
    }

    fn exec_assignment(&self, node: &GrammarASTNode) -> Result<(), String> {
        // assignment_stmt = NAME [ index_suffix ] EQUALS expr ;
        let name_tok = match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) => t,
            _ => return Err("idl-runtime: malformed assignment_stmt".to_string()),
        };
        let name = fold_case(&name_tok.value);
        let index_suffix = find_child_opt(node, "index_suffix");
        let expr_node = find_child(node, "expr")?;
        let value = self.eval_expr(expr_node)?;
        match index_suffix {
            None => {
                self.assign(&name, value);
                Ok(())
            }
            Some(idx) => self.assign_indexed(&name, idx, value),
        }
    }

    fn assign_indexed(
        &self,
        name: &str,
        index_suffix: &GrammarASTNode,
        value: IdlValue,
    ) -> Result<(), String> {
        let current = self
            .lookup(name)
            .ok_or_else(|| format!("idl-runtime: undefined variable '{name}'"))?;
        let arr = match current {
            IdlValue::Num(a) => a,
            IdlValue::Str(_) => {
                return Err(format!(
                    "idl-runtime: cannot subscript-assign into string variable '{name}' (MA12 §2/§4: no string arrays this cut)"
                ))
            }
        };
        let value_arr = match value {
            IdlValue::Num(a) => a,
            IdlValue::Str(_) => {
                return Err(
                    "idl-runtime: cannot assign a string into a numeric subscript target"
                        .to_string(),
                )
            }
        };
        let subscript_list = find_child(index_suffix, "subscript_list")?;
        let positions = self.resolve_subscripts(subscript_list, &arr)?;
        let new_arr = write_positions(&arr, &positions, &value_arr)?;
        self.assign(name, IdlValue::Num(new_arr));
        Ok(())
    }

    fn exec_procedure_call(&self, node: &GrammarASTNode) -> Result<(), String> {
        // procedure_call_stmt = NAME COMMA arg_list ;
        let name_tok = match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) => t,
            _ => return Err("idl-runtime: malformed procedure_call_stmt".to_string()),
        };
        let name = fold_case(&name_tok.value);
        let arg_list = find_child_opt(node, "arg_list");
        self.eval_procedure_call(&name, arg_list)?;
        Ok(())
    }

    // ── Calls: two separate namespaces (MA12 §3) ─────────────────────────

    /// Dispatch a **procedure** call (statement position): built-in
    /// procedures (currently just `PRINT`) first, then the `procs` table.
    fn eval_procedure_call(
        &self,
        name: &str,
        arg_list: Option<&GrammarASTNode>,
    ) -> Result<(), String> {
        let call_args = self.eval_call_args(arg_list)?;
        if let Some(result) =
            builtins::call_procedure(name, &call_args.positional, &call_args.keywords)
        {
            let text = result?;
            self.emit(&text);
            return Ok(());
        }
        let callable = self.procs.borrow().get(name).cloned();
        if let Some(callable) = callable {
            self.call_user_routine(&callable, &call_args)?;
            return Ok(());
        }
        Err(format!("idl-runtime: undefined procedure '{name}'"))
    }

    /// Dispatch a **function** call (expression position): the
    /// `N_ELEMENTS`-on-a-possibly-unbound-name special case first (MA12
    /// §3), then built-in functions, then the `funcs` table.
    fn eval_function_call(
        &self,
        name: &str,
        arg_list: Option<&GrammarASTNode>,
    ) -> Result<IdlValue, String> {
        // MA12 §3: "an OMITTED keyword is left undefined... IDL's own
        // idiomatic N_ELEMENTS(kw) EQ 0 'was this keyword passed?' test
        // relies on omitted keywords being genuinely absent, not defaulted
        // to a sentinel." Evaluating `kw` the ORDINARY way would raise this
        // crate's own "undefined variable" error before N_ELEMENTS ever got
        // a chance to answer "zero" -- so a bare-NAME argument to
        // N_ELEMENTS is special-cased here, BEFORE ordinary argument
        // evaluation, to look the name up directly and answer 0 for an
        // absent binding instead of erroring.
        if name == "N_ELEMENTS" {
            if let Some(bare) = single_bare_name_arg(arg_list) {
                let folded = fold_case(&bare);
                let count = match self.lookup(&folded) {
                    Some(v) => builtins::element_count(&v),
                    None => 0,
                };
                return Ok(IdlValue::num(count as f64));
            }
        }
        let call_args = self.eval_call_args(arg_list)?;
        if let Some(result) =
            builtins::call_function(name, &call_args.positional, &call_args.keywords)
        {
            return result;
        }
        let callable = self.funcs.borrow().get(name).cloned();
        if let Some(callable) = callable {
            return self.call_user_routine(&callable, &call_args);
        }
        Err(format!("idl-runtime: undefined function '{name}'"))
    }

    /// Bind `args` onto `callable`'s declared parameters and run its body
    /// in a fresh, isolated call frame -- the keyword-aware binding step
    /// MA12 §3 layers on top of Q's base scope-frame mechanism.
    ///
    /// Binding rule (MA12 §3): positional call-site arguments bind, **by
    /// position**, to the callable's positional (non-keyword) parameters
    /// in header order; keyword call-site arguments (`KEYWORD=value` /
    /// `/KEYWORD`) bind **by name** against the callable's keyword
    /// parameters (`KEYWORD=local_var_name` headers). A parameter with no
    /// corresponding call-site argument is left **genuinely unbound** in
    /// the fresh frame (not defaulted to a sentinel) -- MA12 §3's own
    /// "omitted keyword left undefined" rule, which this evaluator applies
    /// uniformly to positional parameters too (both are real IDL behavior:
    /// a routine can be called with fewer positional arguments than it
    /// declares).
    fn call_user_routine(
        &self,
        callable: &Rc<IdlCallable>,
        args: &CallArgs,
    ) -> Result<IdlValue, String> {
        let _guard = self.enter()?;
        let positional_params: Vec<&ParamSpec> = callable
            .params
            .iter()
            .filter(|p| p.keyword.is_none())
            .collect();
        if args.positional.len() > positional_params.len() {
            return Err(format!(
                "idl-runtime: {} {} called with too many positional arguments ({} given, {} declared)",
                callable.kind.label(),
                callable.name,
                args.positional.len(),
                positional_params.len()
            ));
        }
        let mut frame = HashMap::new();
        for (param, value) in positional_params
            .iter()
            .zip(args.positional.iter().cloned())
        {
            frame.insert(param.local.clone(), value);
        }
        let mut remaining_keywords: Vec<&String> = args.keywords.keys().collect();
        for param in callable.params.iter().filter(|p| p.keyword.is_some()) {
            let kw = param.keyword.as_ref().expect("filtered to Some above");
            if let Some(value) = args.keywords.get(kw) {
                frame.insert(param.local.clone(), value.clone());
                remaining_keywords.retain(|k| *k != kw);
            }
        }
        if let Some(unknown) = remaining_keywords.into_iter().next() {
            return Err(format!(
                "idl-runtime: {} {} has no keyword parameter named '{unknown}'",
                callable.kind.label(),
                callable.name
            ));
        }

        self.env.borrow_mut().push(frame);
        let _frame_guard = FrameGuard { env: &self.env };
        let flow = self.exec_block(callable.body.iter())?;
        match flow {
            Flow::Break | Flow::Continue => Err(format!(
                "idl-runtime: BREAK/CONTINUE used outside a loop (inside {} {})",
                callable.kind.label(),
                callable.name
            )),
            Flow::Return(Some(value)) => match callable.kind {
                RoutineKind::Function => Ok(value),
                RoutineKind::Procedure => Err(format!(
                    "idl-runtime: PRO {} used RETURN with a value; procedures have no return value",
                    callable.name
                )),
            },
            Flow::Return(None) => match callable.kind {
                RoutineKind::Function => Err(format!(
                    "idl-runtime: FUNCTION {} used bare RETURN with no value",
                    callable.name
                )),
                RoutineKind::Procedure => Ok(IdlValue::num(0.0)),
            },
            Flow::Normal => match callable.kind {
                RoutineKind::Function => Err(format!(
                    "idl-runtime: FUNCTION {} completed without RETURN, value",
                    callable.name
                )),
                RoutineKind::Procedure => Ok(IdlValue::num(0.0)),
            },
        }
    }

    // ── Call-argument evaluation ──────────────────────────────────────────

    fn eval_call_args(&self, arg_list: Option<&GrammarASTNode>) -> Result<CallArgs, String> {
        let mut positional = Vec::new();
        let mut keywords = HashMap::new();
        if let Some(al) = arg_list {
            for arg in node_children(al) {
                match arg.children.first() {
                    Some(ASTNodeOrToken::Node(n)) if n.rule_name == "keyword_arg" => {
                        // keyword_arg = NAME EQUALS expr ;
                        let name_tok = match n.children.first() {
                            Some(ASTNodeOrToken::Token(t)) => t,
                            _ => return Err("idl-runtime: malformed keyword_arg".to_string()),
                        };
                        let expr_node = find_child(n, "expr")?;
                        let value = self.eval_expr(expr_node)?;
                        keywords.insert(fold_case(&name_tok.value), value);
                    }
                    Some(ASTNodeOrToken::Node(n)) if n.rule_name == "bool_keyword_arg" => {
                        // bool_keyword_arg = SLASH NAME ;
                        let name_tok = match n.children.get(1) {
                            Some(ASTNodeOrToken::Token(t)) => t,
                            _ => return Err("idl-runtime: malformed bool_keyword_arg".to_string()),
                        };
                        keywords.insert(fold_case(&name_tok.value), IdlValue::num(1.0));
                    }
                    Some(ASTNodeOrToken::Node(n)) if n.rule_name == "expr" => {
                        positional.push(self.eval_expr(n)?);
                    }
                    _ => return Err("idl-runtime: malformed arg node".to_string()),
                }
            }
        }
        Ok(CallArgs {
            positional,
            keywords,
        })
    }

    // ── Expression evaluation: the precedence cascade ────────────────────

    fn eval_expr(&self, node: &GrammarASTNode) -> Result<IdlValue, String> {
        let _guard = self.enter()?;
        match node.rule_name.as_str() {
            "expr" | "group" => self.eval_expr(only_node(node)?),
            "logical" => self.eval_logical(node),
            "comparison" => self.eval_comparison(node),
            "additive" => self.eval_additive(node),
            "unary" => self.eval_unary(node),
            "multiplicative" => self.eval_multiplicative(node),
            "power" => self.eval_power(node),
            "postfix" => self.eval_postfix(node),
            "primary" => self.eval_primary(node),
            other => Err(format!("idl-runtime: unexpected expression node '{other}'")),
        }
    }

    fn eval_scalar(&self, node: &GrammarASTNode) -> Result<f64, String> {
        require_scalar_num(&self.eval_expr(node)?)
    }

    fn eval_condition(&self, node: &GrammarASTNode) -> Result<bool, String> {
        Ok(self.eval_scalar(node)? != 0.0)
    }

    fn eval_logical(&self, node: &GrammarASTNode) -> Result<IdlValue, String> {
        // logical = comparison { ( "AND" | "OR" | "XOR" ) comparison } ;
        //
        // MA12 §4 spells these "logical/bitwise" deliberately: real IDL's
        // AND/OR/XOR/NOT are documented BITWISE operators over an integer
        // representation (confirmed directly against NV5 Geospatial's own
        // *Bitwise Operators* / *Logical vs. Bitwise Operators* pages this
        // session) -- NOT short-circuit boolean logic (that is `&&`/`||`,
        // explicitly out of scope, `idl.tokens`' own header note). For the
        // idiomatic case (both operands already 0/1 from a comparison), a
        // bitwise AND/OR/XOR of `{0,1}` values happens to coincide exactly
        // with logical AND/OR/XOR -- so ordinary `x GT 0 AND y LT 5`
        // conditions behave exactly as expected either way.
        let (first, rest) = chain_parts(node);
        let mut acc = self.eval_expr(first)?;
        for (op, operand) in rest {
            let rhs = self.eval_expr(operand)?;
            let a = into_num(acc)?;
            let b = into_num(rhs)?;
            let result = match op.value.as_str() {
                "AND" => builtins::bitwise_and(&a, &b)?,
                "OR" => builtins::bitwise_or(&a, &b)?,
                "XOR" => builtins::bitwise_xor(&a, &b)?,
                other => return Err(format!("idl-runtime: unknown logical operator '{other}'")),
            };
            acc = IdlValue::Num(result);
        }
        Ok(acc)
    }

    fn eval_comparison(&self, node: &GrammarASTNode) -> Result<IdlValue, String> {
        // comparison = additive { ( "EQ" | "NE" | "LE" | "LT" | "GE" | "GT" ) additive } ;
        let (first, rest) = chain_parts(node);
        let mut acc = self.eval_expr(first)?;
        for (op, operand) in rest {
            let rhs = self.eval_expr(operand)?;
            acc = IdlValue::Num(apply_comparison(op.value.as_str(), &acc, &rhs)?);
        }
        Ok(acc)
    }

    fn eval_additive(&self, node: &GrammarASTNode) -> Result<IdlValue, String> {
        // additive = unary { ( PLUS | MINUS ) unary } ;
        //
        // Every tier of this cascade sits on the path down to `primary`
        // even when there is no operator at this level at all (a bare
        // STRING primitive descends through `additive`/`multiplicative`/
        // `power` just to reach `postfix`/`primary`) -- so the no-operator
        // (`rest.is_empty()`) case must pass the value through UNCHANGED,
        // not force it through `into_num`, or a plain string literal like
        // `'hello'` would fail here with "expected a numeric value" before
        // ever reaching `PRINT`.
        let (first, rest) = chain_parts(node);
        if rest.is_empty() {
            return self.eval_expr(first);
        }
        let mut acc = into_num(self.eval_expr(first)?)?;
        for (op, operand) in rest {
            let rhs = into_num(self.eval_expr(operand)?)?;
            acc = match op.effective_type_name() {
                "PLUS" => ops::add(&acc, &rhs)?,
                "MINUS" => ops::sub(&acc, &rhs)?,
                other => return Err(format!("idl-runtime: unknown additive operator '{other}'")),
            };
        }
        Ok(IdlValue::Num(acc))
    }

    fn eval_unary(&self, node: &GrammarASTNode) -> Result<IdlValue, String> {
        // unary = ( PLUS | MINUS | "NOT" ) unary | multiplicative ;
        match node.children.as_slice() {
            [ASTNodeOrToken::Token(op), ASTNodeOrToken::Node(inner)] => {
                let v = into_num(self.eval_expr(inner)?)?;
                let op_name = if op.effective_type_name() == "KEYWORD" {
                    op.value.as_str()
                } else {
                    op.effective_type_name()
                };
                let result = match op_name {
                    "PLUS" => v,
                    "MINUS" => builtins::negate(&v),
                    // Real IDL's NOT is BITWISE negation, NOT logical
                    // negation (`~` is the deferred logical form) --
                    // confirmed against NV5 Geospatial's own docs this
                    // session. `NOT 0` is `-1` and `NOT 1` is `-2` --
                    // BOTH nonzero/truthy -- a well-known, documented IDL
                    // gotcha this evaluator faithfully reproduces rather
                    // than "fixing" into logical negation.
                    "NOT" => builtins::bitwise_not(&v)?,
                    other => return Err(format!("idl-runtime: unknown unary operator '{other}'")),
                };
                Ok(IdlValue::Num(result))
            }
            [ASTNodeOrToken::Node(inner)] => self.eval_expr(inner),
            _ => Err("idl-runtime: malformed unary node".to_string()),
        }
    }

    fn eval_multiplicative(&self, node: &GrammarASTNode) -> Result<IdlValue, String> {
        // multiplicative = power { ( STAR | SLASH | HASH_HASH | HASH ) power } ;
        // See `eval_additive`'s own comment: the no-operator case must pass
        // the value through unchanged (a bare string descends through
        // here too).
        let (first, rest) = chain_parts(node);
        if rest.is_empty() {
            return self.eval_expr(first);
        }
        let mut acc = into_num(self.eval_expr(first)?)?;
        for (op, operand) in rest {
            let rhs = into_num(self.eval_expr(operand)?)?;
            acc = match op.effective_type_name() {
                "STAR" => ops::mul(&acc, &rhs)?,
                "SLASH" => ops::div(&acc, &rhs)?,
                // `##` is IDL's ordinary/standard matrix product (rows of
                // the first array times columns of the second) --
                // confirmed against multiple secondary IDL references this
                // session.
                "HASH_HASH" => execute(Kernel::MatMul, &acc, &rhs)?,
                // `#` is IDL's OWN reversed/column-oriented product
                // (documented as "the opposite of normal matrix
                // multiplication," multiplying columns of the first by
                // rows of the second, with the result shape [nrows(B),
                // ncols(A)]) -- which matches `matmul(B, A)` exactly
                // (`matmul(X,Y)` requires `X.ncols()==Y.nrows()` and
                // produces shape `[X.nrows(), Y.ncols()]`; substituting
                // X=B, Y=A gives shape `[B.nrows(), A.ncols()]`, the
                // documented `#` shape). **Flagged as a moderate-confidence
                // judgment call**: this was derived from secondary
                // descriptions of `#`'s documented shape/compatibility
                // rule, not independently re-verified against a primary
                // IDL source's own worked numeric example in this session.
                "HASH" => execute(Kernel::MatMul, &rhs, &acc)?,
                other => {
                    return Err(format!(
                        "idl-runtime: unknown multiplicative operator '{other}'"
                    ))
                }
            };
        }
        Ok(IdlValue::Num(acc))
    }

    fn eval_power(&self, node: &GrammarASTNode) -> Result<IdlValue, String> {
        // power = postfix { CARET postfix } ; -- left-associative (idl-parser's
        // own header comment: 2^3^2 == (2^3)^2 == 64 in real IDL). See
        // `eval_additive`'s own comment: the no-operator case must pass
        // the value through unchanged.
        let (first, rest) = chain_parts(node);
        if rest.is_empty() {
            return self.eval_expr(first);
        }
        let mut acc = into_num(self.eval_expr(first)?)?;
        for (_caret, operand) in rest {
            let rhs = into_num(self.eval_expr(operand)?)?;
            acc = builtins::elementwise_pow(&acc, &rhs)?;
        }
        Ok(IdlValue::Num(acc))
    }

    fn eval_postfix(&self, node: &GrammarASTNode) -> Result<IdlValue, String> {
        // postfix = primary { index_suffix | call_suffix } ;
        let primary_node = match node.children.first() {
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "primary" => n,
            _ => return Err("idl-runtime: malformed postfix node".to_string()),
        };
        let suffixes: Vec<&GrammarASTNode> = node.children[1..]
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                ASTNodeOrToken::Token(_) => None,
            })
            .collect();

        // A bare NAME primary immediately followed by a call_suffix is a
        // FUNCTION CALL, never "look up a variable, then apply something
        // to it" -- IDL has no first-class function values (unlike Q),
        // so `NAME(...)` always means "invoke the routine named NAME."
        // This must be decided BEFORE evaluating `primary_node` the
        // ordinary way (which would try -- and fail -- to look NAME up as
        // a plain variable).
        let mut value: IdlValue;
        let mut start = 0;
        if let (Some(name), Some(first_suffix)) =
            (bare_name_primary(primary_node), suffixes.first())
        {
            if first_suffix.rule_name == "call_suffix" {
                let arg_list = find_child_opt(first_suffix, "arg_list");
                value = self.eval_function_call(&fold_case(name), arg_list)?;
                start = 1;
            } else {
                value = self.eval_primary(primary_node)?;
            }
        } else {
            value = self.eval_primary(primary_node)?;
        }

        for suffix in &suffixes[start..] {
            value = match suffix.rule_name.as_str() {
                "index_suffix" => self.read_index_suffix(suffix, &value)?,
                "call_suffix" => {
                    return Err(
                        "idl-runtime: cannot call a value that is not a bare routine name \
                         (IDL has no first-class function values in this cut)"
                            .to_string(),
                    )
                }
                other => return Err(format!("idl-runtime: unexpected postfix suffix '{other}'")),
            };
        }
        Ok(value)
    }

    fn eval_primary(&self, node: &GrammarASTNode) -> Result<IdlValue, String> {
        match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NUMBER" => {
                let n: f64 = t
                    .value
                    .parse()
                    .map_err(|_| format!("idl-runtime: invalid numeric literal '{}'", t.value))?;
                Ok(IdlValue::num(n))
            }
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "STRING" => {
                Ok(IdlValue::Str(t.value.clone()))
            }
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "array_literal" => {
                self.eval_array_literal(n)
            }
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "group" => self.eval_expr(n),
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NAME" => {
                let name = fold_case(&t.value);
                self.lookup(&name)
                    .ok_or_else(|| format!("idl-runtime: undefined variable '{name}'"))
            }
            _ => Err("idl-runtime: malformed primary node".to_string()),
        }
    }

    fn eval_array_literal(&self, node: &GrammarASTNode) -> Result<IdlValue, String> {
        // array_literal = LBRACKET [ array_elements ] RBRACKET ;
        // array_elements = expr { COMMA expr } ;
        //
        // Real IDL: even a one-element literal (`[5]`) is a genuine rank-1
        // array, never a scalar (MA12 §2) -- `Array::from_vec` always
        // produces shape `[n]`, matching that rule directly.
        let mut data = Vec::new();
        if let Some(elems) = find_child_opt(node, "array_elements") {
            for e in node_children(elems) {
                let v = into_num(self.eval_expr(e)?)?;
                if data.len().saturating_add(v.len()) > builtins::MAX_ARRAY_LENGTH {
                    return Err(format!(
                        "idl-runtime: array literal exceeds the {}-element cap",
                        builtins::MAX_ARRAY_LENGTH
                    ));
                }
                data.extend_from_slice(v.data());
            }
        }
        Ok(IdlValue::Num(Array::from_vec(data)))
    }

    // ── Subscripting: read and (for assignment) write ────────────────────

    fn read_index_suffix(
        &self,
        suffix: &GrammarASTNode,
        base: &IdlValue,
    ) -> Result<IdlValue, String> {
        let arr = match base {
            IdlValue::Num(a) => a,
            IdlValue::Str(_) => {
                return Err("idl-runtime: cannot subscript a string value in this cut".to_string())
            }
        };
        let subscript_list = find_child(suffix, "subscript_list")?;
        let positions = self.resolve_subscripts(subscript_list, arr)?;
        Ok(IdlValue::Num(read_positions(arr, &positions)))
    }

    /// Resolve a `subscript_list` node against `arr`'s shape.
    ///
    /// **A concrete, flagged judgment call (MA12 §2's own note 1)**: real
    /// IDL's 2-D subscript order is documented as `a[column, row]`
    /// (transposed from MATLAB's `[row, column]`), and MA12 §2 explicitly
    /// leaves "whether IDL's `a[i,j]` maps to `array-runtime`'s element
    /// `(i,j)` or `(j,i)`" as "a concrete lowering decision `idl-runtime`
    /// ... must make deliberately... confirmed empirically... before
    /// relying on it." This evaluator maps the FIRST subscript to
    /// `array-runtime`'s **column** axis and the SECOND to its **row**
    /// axis (`a[i, j]` reads `array-runtime`'s `get(row = j, col = i)`) --
    /// the direct, literal reading of "first subscript is the column,
    /// second is the row." This was **not** independently re-verified
    /// against a real IDL session's actual `PRINT` output of a constructed
    /// 2-D array in this session (no live IDL interpreter was available) --
    /// flagged here exactly as MA12 §2 anticipates, for empirical
    /// confirmation by a later item.
    fn resolve_subscripts(
        &self,
        subscript_list: &GrammarASTNode,
        arr: &Array,
    ) -> Result<SubscriptPositions, String> {
        let subs = node_children(subscript_list);
        match subs.len() {
            1 => {
                // A single subscript indexes the array's own FLAT,
                // column-major storage directly -- real IDL documents
                // exactly this "linear index into column-major storage"
                // rule, and `array-runtime` is column-major too (MA12 §2),
                // so no translation is needed for this arm at all.
                let axis_len = arr.len();
                Ok(SubscriptPositions::Linear(
                    self.subscript_positions(subs[0], axis_len)?,
                ))
            }
            2 => {
                let cols = self.subscript_positions(subs[0], arr.ncols())?;
                let rows = self.subscript_positions(subs[1], arr.nrows())?;
                Ok(SubscriptPositions::TwoD { rows, cols })
            }
            n => Err(format!(
                "idl-runtime: {n}-D subscripting is not supported in this cut (only 1-D and 2-D)"
            )),
        }
    }

    fn subscript_positions(
        &self,
        node: &GrammarASTNode,
        axis_len: usize,
    ) -> Result<Vec<usize>, String> {
        // subscript = STAR | range_subscript | expr ;
        match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "STAR" => {
                Ok((0..axis_len).collect())
            }
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "range_subscript" => {
                self.range_subscript_positions(n, axis_len)
            }
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "expr" => {
                let idx = resolve_index(self.eval_scalar(n)?, axis_len)?;
                Ok(vec![idx])
            }
            _ => Err("idl-runtime: malformed subscript".to_string()),
        }
    }

    /// `range_subscript = expr COLON range_end [ COLON expr ] ;` -- per NV5
    /// Geospatial's own *Array Subscript Ranges* reference (confirmed
    /// directly this session), IDL's `[s0:s1]` is **inclusive of both
    /// endpoints**.
    fn range_subscript_positions(
        &self,
        node: &GrammarASTNode,
        axis_len: usize,
    ) -> Result<Vec<usize>, String> {
        let exprs: Vec<&GrammarASTNode> = node_children(node)
            .into_iter()
            .filter(|n| n.rule_name == "expr")
            .collect();
        if exprs.is_empty() {
            return Err("idl-runtime: malformed range_subscript (missing start)".to_string());
        }
        let range_end = find_child(node, "range_end")?;
        let start = resolve_index(self.eval_scalar(exprs[0])?, axis_len)?;
        let end = match range_end.children.first() {
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "STAR" => {
                axis_len.saturating_sub(1)
            }
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "expr" => {
                resolve_index(self.eval_scalar(n)?, axis_len)?
            }
            _ => return Err("idl-runtime: malformed range_end".to_string()),
        };
        let stride_f = if exprs.len() >= 2 {
            self.eval_scalar(exprs[1])?
        } else {
            1.0
        };
        let stride = stride_f as i64;
        if stride == 0 {
            return Err("idl-runtime: subscript stride must not be zero".to_string());
        }
        let (start_i, end_i) = (start as i64, end as i64);
        let mut positions = Vec::new();
        let mut i = start_i;
        let ascending = stride > 0;
        loop {
            let done = if ascending { i > end_i } else { i < end_i };
            if done {
                break;
            }
            positions.push(i as usize);
            if positions.len() > builtins::MAX_ARRAY_LENGTH {
                return Err(format!(
                    "idl-runtime: subscript range exceeds the {}-element cap",
                    builtins::MAX_ARRAY_LENGTH
                ));
            }
            // `stride_f as i64` (above) saturates any sufficiently large
            // finite stride (e.g. `1e20`) to `i64::MAX`/`i64::MIN` -- the
            // `stride == 0` check above only rejects a NaN or literal-zero
            // stride, not a merely huge one, so `i += stride` here can
            // overflow `i64` on the very first iteration once `start_i` is
            // nonzero (a debug-build panic, or silent release-build wrap
            // into a garbage index passed to `arr.data()[i]`/`arr.get(...)`
            // downstream -- either way an unauthenticated, two-line-of-
            // input crash, the same severity as the sibling NaN-index bug
            // this same review round fixed). `checked_add` closes this the
            // same way `arr_zeros`'s `checked_mul` already guards its own
            // overflow-prone multiplication elsewhere in this file.
            i = i.checked_add(stride).ok_or_else(|| {
                "idl-runtime: subscript stride overflow (start + stride is out of range)"
                    .to_string()
            })?;
        }
        Ok(positions)
    }
}

/// One evaluated call's argument list, split the way MA12 §3 requires:
/// positional args (bound by position) and keyword args (bound by name,
/// `KEYWORD=value` and `/KEYWORD` both landing here as an ordinary
/// `IdlValue`, the latter defaulted to `1`, per MA12 §3 item 3).
struct CallArgs {
    positional: Vec<IdlValue>,
    keywords: HashMap<String, IdlValue>,
}

/// Where a resolved subscript reads/writes: either a flat set of positions
/// into the array's own linear (column-major) storage (the 1-subscript
/// form), or a separate row/column position list (the 2-subscript form).
enum SubscriptPositions {
    Linear(Vec<usize>),
    TwoD { rows: Vec<usize>, cols: Vec<usize> },
}

fn read_positions(arr: &Array, positions: &SubscriptPositions) -> Array {
    match positions {
        SubscriptPositions::Linear(idxs) => {
            let data: Vec<f64> = idxs.iter().map(|&i| arr.data()[i]).collect();
            // A single plain index reads back a genuine SCALAR (rank 0),
            // matching real IDL's own "a(i) is a scalar" rule -- a
            // disclosed simplification: this cut does not separately track
            // "was this one position a bare index, or a range that just
            // happened to resolve to one element" (`a[2]` and `a[2:2]`
            // both collapse to a scalar here, though real IDL keeps a
            // length-1 RANGE result as a genuine rank-1 array).
            if data.len() == 1 {
                Array::scalar(data[0])
            } else {
                Array::from_vec(data)
            }
        }
        SubscriptPositions::TwoD { rows, cols } => {
            if rows.len() == 1 && cols.len() == 1 {
                Array::scalar(
                    arr.get(rows[0], cols[0])
                        .expect("resolved subscript is in bounds"),
                )
            } else if rows.len() == 1 {
                Array::from_vec(
                    cols.iter()
                        .map(|&c| arr.get(rows[0], c).expect("in bounds"))
                        .collect(),
                )
            } else if cols.len() == 1 {
                Array::from_vec(
                    rows.iter()
                        .map(|&r| arr.get(r, cols[0]).expect("in bounds"))
                        .collect(),
                )
            } else {
                let mut data = vec![0.0; rows.len() * cols.len()];
                for (ci, &c) in cols.iter().enumerate() {
                    for (ri, &r) in rows.iter().enumerate() {
                        data[ci * rows.len() + ri] = arr.get(r, c).expect("in bounds");
                    }
                }
                Array::from_shape(data, vec![rows.len(), cols.len()])
                    .expect("shape matches gathered data")
            }
        }
    }
}

fn write_positions(
    arr: &Array,
    positions: &SubscriptPositions,
    value: &Array,
) -> Result<Array, String> {
    let mut data = arr.data().to_vec();
    match positions {
        SubscriptPositions::Linear(idxs) => write_flat(&mut data, idxs, value)?,
        SubscriptPositions::TwoD { rows, cols } => {
            // Same column-major (columns outer, rows inner) iteration order
            // `read_positions` uses, so a read-then-write round-trips.
            let nrows = arr.nrows();
            let flat_positions: Vec<usize> = cols
                .iter()
                .flat_map(|&c| rows.iter().map(move |&r| c * nrows + r))
                .collect();
            write_flat(&mut data, &flat_positions, value)?;
        }
    }
    Array::from_shape(data, arr.shape().to_vec())
}

fn write_flat(data: &mut [f64], positions: &[usize], value: &Array) -> Result<(), String> {
    if value.is_scalar() {
        let v = value.data()[0];
        for &p in positions {
            data[p] = v;
        }
    } else if value.len() == positions.len() {
        for (&p, &v) in positions.iter().zip(value.data()) {
            data[p] = v;
        }
    } else {
        return Err(format!(
            "idl-runtime: subscripted assignment count mismatch: {} target position(s), {} value(s)",
            positions.len(),
            value.len()
        ));
    }
    Ok(())
}

/// Resolve one raw (possibly negative-from-end) subscript value to a
/// bounds-checked 0-based index. Negative-from-end (`a[-1]` is the last
/// element) falls out of `unary`'s own `MINUS` handling upstream -- by the
/// time a subscript value reaches here it is an ordinary (possibly
/// negative) `f64`, resolved the same way MATLAB's `end`-relative indexing
/// is (SIR22 precedent, cited directly by MA12 §2).
fn resolve_index(raw: f64, axis_len: usize) -> Result<usize, String> {
    let idx_f = if raw < 0.0 {
        axis_len as f64 + raw
    } else {
        raw
    };
    // Written as the IN-RANGE condition, negated, rather than an "out of
    // range" disjunction of `<`/`>=` comparisons: IEEE-754 comparisons
    // against NaN are always `false` (`NaN < 0.0` and `NaN >= len` both
    // evaluate `false`), so the disjunction form let a NaN subscript
    // (`a[SQRT(-1)]`, `a[0.0/0.0]`) skip this check entirely and fall
    // through to `NaN as usize` (Rust's saturating float-to-int cast,
    // which returns 0), reported as a validated in-bounds index even
    // against a zero-length axis -- an unauthenticated, two-line-of-input
    // panic (`arr.data()[0]` / `arr.get(...).expect(...)` downstream) that
    // is not caught anywhere between here and the REPL's process boundary.
    // `!(idx_f >= 0.0 && idx_f < axis_len as f64)` is `true` for NaN
    // (since both `&&` operands are `false`), so this form rejects it.
    if !(idx_f >= 0.0 && idx_f < axis_len as f64) {
        return Err(format!(
            "idl-runtime: subscript {raw} is out of range for a dimension of size {axis_len}"
        ));
    }
    Ok(idx_f as usize)
}

/// `EQ`/`NE`/`LT`/`LE`/`GT`/`GE` -- numeric comparisons map directly onto
/// `array_runtime::ops::elementwise` + `BinOp`; string comparisons are
/// restricted to `EQ`/`NE` (MA12 §2: `Str` has "no operator overloading...
/// yet" beyond assignment/display/`PRINT`/equality/keyword-value).
fn apply_comparison(op: &str, lhs: &IdlValue, rhs: &IdlValue) -> Result<Array, String> {
    match (lhs, rhs) {
        (IdlValue::Num(a), IdlValue::Num(b)) => {
            let binop = match op {
                "EQ" => BinOp::Eq,
                "NE" => BinOp::Ne,
                "LT" => BinOp::Lt,
                "LE" => BinOp::Le,
                "GT" => BinOp::Gt,
                "GE" => BinOp::Ge,
                other => return Err(format!("idl-runtime: unknown comparison operator '{other}'")),
            };
            ops::elementwise(binop, a, b)
        }
        (IdlValue::Str(a), IdlValue::Str(b)) => match op {
            "EQ" => Ok(Array::scalar(if a == b { 1.0 } else { 0.0 })),
            "NE" => Ok(Array::scalar(if a != b { 1.0 } else { 0.0 })),
            other => Err(format!(
                "idl-runtime: comparison '{other}' is not supported between strings in this cut (only EQ/NE, MA12 §2)"
            )),
        },
        _ => Err("idl-runtime: cannot compare a string and a numeric value".to_string()),
    }
}

fn require_scalar_num(v: &IdlValue) -> Result<f64, String> {
    match v {
        IdlValue::Num(a) if a.len() == 1 => Ok(a.data()[0]),
        IdlValue::Num(a) => Err(format!(
            "idl-runtime: expected a scalar numeric value, got a {}-element array",
            a.len()
        )),
        IdlValue::Str(_) => Err("idl-runtime: expected a numeric value, got a string".to_string()),
    }
}

fn into_num(v: IdlValue) -> Result<Array, String> {
    match v {
        IdlValue::Num(a) => Ok(a),
        IdlValue::Str(_) => Err("idl-runtime: expected a numeric value, got a string".to_string()),
    }
}

/// Uppercase-fold one identifier -- the one place this crate's own
/// case-folding decision (see this module's own top doc comment) is
/// actually applied. Every `NAME` token this evaluator ever binds or looks
/// up (variables, routine names, parameter/keyword names) goes through
/// this function exactly once, at the point it is first read off the CST.
fn fold_case(s: &str) -> String {
    s.to_uppercase()
}

/// Split a left-associative chain rule's children (`operand { OP operand
/// }` -- `logical`/`comparison`/`additive`/`multiplicative`/`power`) into
/// the first operand and a list of (operator token, next operand) pairs.
fn chain_parts(node: &GrammarASTNode) -> (&GrammarASTNode, Vec<(&Token, &GrammarASTNode)>) {
    let mut first: Option<&GrammarASTNode> = None;
    let mut rest = Vec::new();
    let mut pending_op: Option<&Token> = None;
    for c in &node.children {
        match c {
            ASTNodeOrToken::Node(n) => {
                if first.is_none() {
                    first = Some(n);
                } else {
                    let op = pending_op
                        .take()
                        .expect("an operator token precedes every operand after the first");
                    rest.push((op, n));
                }
            }
            ASTNodeOrToken::Token(t) => pending_op = Some(t),
        }
    }
    (
        first.expect("a chain rule always has at least one operand"),
        rest,
    )
}

/// If `node` (a `primary`) is a bare `NAME` token with no suffixes of its
/// own, return its raw (un-folded) source spelling.
fn bare_name_primary(node: &GrammarASTNode) -> Option<&str> {
    match node.children.first() {
        Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NAME" => {
            Some(t.value.as_str())
        }
        _ => None,
    }
}

/// If `arg_list` has exactly one argument, and that argument is a bare
/// variable reference with no suffixes at all (no arithmetic, no
/// indexing, no nested call), return its raw name -- used only by
/// `N_ELEMENTS`'s special "was this ever bound" check (MA12 §3).
fn single_bare_name_arg(arg_list: Option<&GrammarASTNode>) -> Option<String> {
    let al = arg_list?;
    let args = node_children(al);
    if args.len() != 1 {
        return None;
    }
    match args[0].children.first() {
        Some(ASTNodeOrToken::Node(n)) if n.rule_name == "expr" => trivial_name_descent(n),
        _ => None,
    }
}

fn trivial_name_descent(node: &GrammarASTNode) -> Option<String> {
    match node.rule_name.as_str() {
        "expr" | "logical" | "comparison" | "additive" | "unary" | "multiplicative" | "power" => {
            let nodes = node_children(node);
            if nodes.len() == 1 {
                trivial_name_descent(nodes[0])
            } else {
                None
            }
        }
        "postfix" => {
            if node.children.len() == 1 {
                match node.children.first() {
                    Some(ASTNodeOrToken::Node(n)) if n.rule_name == "primary" => {
                        trivial_name_descent(n)
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
        "primary" => bare_name_primary(node).map(|s| s.to_string()),
        _ => None,
    }
}

fn build_params(node: &GrammarASTNode) -> Result<Vec<ParamSpec>, String> {
    // params = param { COMMA param } ;
    // param  = NAME EQUALS NAME | NAME ;
    let mut out = Vec::new();
    for p in node_children(node) {
        let names: Vec<&Token> = p
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => Some(t),
                _ => None,
            })
            .collect();
        match names.len() {
            2 => out.push(ParamSpec {
                keyword: Some(fold_case(&names[0].value)),
                local: fold_case(&names[1].value),
            }),
            1 => out.push(ParamSpec {
                keyword: None,
                local: fold_case(&names[0].value),
            }),
            _ => return Err("idl-runtime: malformed parameter declaration".to_string()),
        }
    }
    Ok(out)
}

// ── AST helpers ───────────────────────────────────────────────────────────

fn node_children(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

fn first_node(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        ASTNodeOrToken::Token(_) => None,
    })
}

fn only_node(node: &GrammarASTNode) -> Result<&GrammarASTNode, String> {
    first_node(node).ok_or_else(|| format!("idl-runtime: malformed '{}' node", node.rule_name))
}

fn find_child<'a>(node: &'a GrammarASTNode, rule_name: &str) -> Result<&'a GrammarASTNode, String> {
    find_child_opt(node, rule_name).ok_or_else(|| {
        format!(
            "idl-runtime: malformed '{}' node (missing '{rule_name}')",
            node.rule_name
        )
    })
}

fn find_child_opt<'a>(node: &'a GrammarASTNode, rule_name: &str) -> Option<&'a GrammarASTNode> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == rule_name => Some(n),
        _ => None,
    })
}

/// `statement_line = statement { STMT_SEP statement } [ NEWLINE ] | NEWLINE ;`
/// -- returns every `statement` child (a blank line yields none).
fn statement_line_statements(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node_children(node)
        .into_iter()
        .filter(|n| n.rule_name == "statement")
        .collect()
}

/// `block_body = { statement_line } ;` -- flattens every contained
/// `statement_line` into its own `statement` children, in order.
fn block_body_statements(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node_children(node)
        .into_iter()
        .flat_map(statement_line_statements)
        .collect()
}

/// A conditional/loop body has two forms (MA12 §3): a single `statement`
/// with no closer, or a `BEGIN block_body ENDxxx|END` block. Used uniformly
/// for `then_branch`/`else_branch`/`for_body`/`while_body`/`repeat_body`,
/// which all share this identical two-shape structure.
fn body_statements(node: &GrammarASTNode) -> Result<Vec<&GrammarASTNode>, String> {
    let nodes = node_children(node);
    if nodes.len() == 1 && nodes[0].rule_name == "statement" {
        Ok(vec![nodes[0]])
    } else if let Some(bb) = nodes.iter().find(|n| n.rule_name == "block_body") {
        Ok(block_body_statements(bb))
    } else {
        Err(format!("idl-runtime: malformed '{}' node", node.rule_name))
    }
}
