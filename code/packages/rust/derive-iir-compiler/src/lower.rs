//! The lowering pass from `coding_adventures_derive_parser`'s generic
//! [`GrammarASTNode`] CST → [`IIRModule`], **v0.1.0**.
//!
//! # Retargeting `derive-runtime`/`derive-to-semantic-ir`, a third time
//!
//! `derive-runtime` already walks this exact CST and compiles it to
//! `symbolic_ir::IRNode`. `derive-to-semantic-ir` retargets the same
//! rule-name dispatch to build `semantic_ir::Expr` instead. This module is
//! a **third retarget**, following `macsyma-iir-compiler`'s precedent
//! (the first language in this rollout) directly — see
//! [`derive-iir-vm.md`](../../../specs/derive-iir-vm.md) and
//! [`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md) for the shared
//! design. It emits `interpreter_ir::IIRInstr`s directly and shares no
//! code with either sibling beyond the parser.
//!
//! # Scope (v0.1.0)
//!
//! Accepted: integer literals; `+ - * /` (binary chains and unary `-` —
//! Derive's grammar, unlike Macsyma's, has **no unary-plus alternative at
//! all**: `unary = MINUS unary | power`, so there is no "unary plus is a
//! no-op" branch to port); assignment (`x := expr`, plain-name target
//! only); free-symbol references; any other head or symbolic operand,
//! represented as an *unevaluated* inert `cons`-chain (mirrors
//! `macsyma-iir-compiler::inert_apply`, itself mirroring
//! `mccarthy-lisp-iir-compiler::lower_quote`'s `QUOTE` materialisation).
//!
//! Rejected, with an explicit [`DeriveIirError`] rather than a silent
//! mis-lowering: `Float` literals (Derive has no `STRING` token or
//! boolean literal keywords at all, unlike Macsyma); `F(x) := body`
//! (function definition — see below); comparisons; `AND`/`OR`/`NOT`;
//! `^` (power); `[...]`/`[...;...]` (vector/matrix literals); any postfix
//! function call `F(x)`.
//!
//! Comparisons and `AND`/`OR`/`NOT` are rejected **outright** — unlike
//! `+`/`-`/`*`/`/`, they are *not* given an inert-data fallback for
//! symbolic operands, for the identical reason `macsyma-iir-compiler`'s
//! module doc gives: `symbolic-vm`'s handler table (the real evaluator
//! `derive-runtime` runs) numerically evaluates a concrete pair of
//! operands for these heads, so building inert data for a *concrete*
//! instance would disagree with `derive-runtime`'s own ground truth.
//! `^`/`POWER` gets the identical treatment for the same reason.
//!
//! # The `/` exactness rule
//!
//! Identical hazard and identical fix as `macsyma-iir-compiler`'s own
//! `/` handling (see that crate's module doc comment for the full
//! argument): Derive's `/` on integers that don't divide evenly returns
//! an exact rational, which `dynval-runtime` cannot represent, and
//! `dynval-runtime`'s own `/` builtin is C-style truncating division. So
//! [`Lowerer::combine`] only takes the evaluated `call_builtin "/"` path
//! when both direct operands are literal integer tokens at this exact
//! node and their quotient is exact; a symbolic operand falls back to
//! inert data; anything else is rejected.
//!
//! # `:=` disambiguation has no operator to branch on
//!
//! Like `derive-to-semantic-ir::lower::lower_assignment`'s own note:
//! Derive's grammar has exactly ONE assignment token, `ASSIGN` (`:=`) —
//! `x := 5` and `F(x) := x^2 + 1` are syntactically identical until this
//! lowering step. Since v0 has no user-defined-function support at all
//! (any call, LHS or not, is already rejected by [`Lowerer::
//! lower_postfix`]), [`Lowerer::lower_assignment`] disambiguates purely
//! by checking whether the LHS is a bare `NAME` token *before* attempting
//! to lower it as an expression: a bare name is plain assignment; anything
//! else (most concretely a call-shaped `F(x)`) is rejected with a
//! `"function definition"`-specific message, not lowered and then
//! rejected one level down inside `lower_postfix` with a more generic
//! "function calls" message — a small but deliberate precision choice,
//! matching `macsyma-iir-compiler::lower_assign`'s identical two-branch
//! shape even though Derive's single `ASSIGN` token has no
//! `COLON`/`COLONEQ` distinction to dispatch on directly.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use symbolic_ir::{ADD, DIV, MUL, NEG, SUB};

/// Maximum expression-nesting depth for *this crate's own* lowering
/// recursion — mirrors `macsyma-iir-compiler::lower::MAX_EXPR_DEPTH` and
/// `derive-to-semantic-ir::lower::MAX_EXPR_DEPTH` (same CST, same
/// generic `GrammarParser` dispatch engine).
const MAX_EXPR_DEPTH: usize = 256;

/// IIR type-hint string for the nil / cons reference type, matching
/// `macsyma-iir-compiler`'s `REF_PAIR` convention.
const REF_PAIR: &str = "ref<LispyPair>";

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during Derive → IIR lowering.
///
/// Mirrors `MacsymaIirError`/`DeriveLowerError`'s shape exactly (`message`
/// + 1-based `line`/`column`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeriveIirError {
    /// Human-readable explanation.
    pub message: String,
    /// 1-based source line.
    pub line: usize,
    /// 1-based source column.
    pub column: usize,
}

impl std::fmt::Display for DeriveIirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeriveIirError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for DeriveIirError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed Derive CST (rooted at the `program` rule) into an
/// [`IIRModule`].
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<IIRModule, DeriveIirError> {
    Lowerer::new().lower_file(tree, module_name)
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

/// The lowered value of one CST node: the IIR register holding it, plus
/// two facts used only to decide how a *later* arithmetic step may
/// combine it — never observed by the emitted IIR itself. Identical
/// shape to `macsyma-iir-compiler::lower::Lowered`.
#[derive(Debug, Clone)]
struct Lowered {
    /// The register this value lives in.
    reg: String,
    /// `true` if this value was built purely from integer literals and
    /// bound (concrete) variables combined via `+`/`-`/`*`/`/`/unary
    /// negate — i.e. it contains no free symbol anywhere.
    concrete: bool,
    /// `Some(v)` only when this value is a *direct* integer-literal token
    /// (or the statically-known result of combining such literals via
    /// `+`/`-`/`*`/unary negate) — used exclusively by the `/` exactness
    /// check ([`Lowerer::combine`]). Never propagated through a variable
    /// reference.
    literal: Option<i64>,
}

struct Lowerer {
    instrs: Vec<IIRInstr>,
    tmp: usize,
    /// Assigned variable name → its most recently assigned value.
    env: std::collections::HashMap<String, Lowered>,
}

impl Lowerer {
    fn new() -> Self {
        Lowerer {
            instrs: Vec::new(),
            tmp: 0,
            env: std::collections::HashMap::new(),
        }
    }

    // -------------------------------------------------------------------
    // top level: `program = { statement_line } ;`
    // `statement_line = statement NEWLINE | statement | NEWLINE ;`
    // -------------------------------------------------------------------

    fn lower_file(
        &mut self,
        program: &GrammarASTNode,
        module_name: &str,
    ) -> Result<IIRModule, DeriveIirError> {
        if program.rule_name != "program" {
            return Err(self.err_at(
                program,
                format!("expected `program` root, got `{}`", program.rule_name),
            ));
        }

        let mut last: Option<Lowered> = None;
        for line in child_nodes(program) {
            if line.rule_name != "statement_line" {
                continue;
            }
            // A blank/terminator-only line has no `statement` child at
            // all — skip it, mirroring `derive-to-semantic-ir::lower::
            // lower_file`'s identical filter.
            let Some(statement_node) = child_nodes(line).find(|n| n.rule_name == "statement")
            else {
                continue;
            };
            last = Some(self.lower_node(statement_node, 0)?);
        }
        let final_val = match last {
            Some(l) => l,
            None => self.emit_nil(),
        };
        self.emit(IIRInstr::new(
            "ret",
            None,
            vec![Operand::Var(final_val.reg)],
            "any",
        ));

        let mut main =
            IIRFunction::new("main", Vec::new(), "any", std::mem::take(&mut self.instrs));
        main.register_count = self.tmp;

        let mut module = IIRModule::new(module_name, "derive");
        module.functions.push(main);
        module.entry_point = Some("main".to_string());

        let problems = module.validate();
        if !problems.is_empty() {
            return Err(DeriveIirError {
                message: format!(
                    "internal: emitted IIR failed validation: {}",
                    problems.join("; ")
                ),
                line: 1,
                column: 1,
            });
        }
        Ok(module)
    }

    // -------------------------------------------------------------------
    // Dispatch
    // -------------------------------------------------------------------

    fn lower_node(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, DeriveIirError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        match unwrap_single(node) {
            Unwrapped::Token(token) => self.lower_token(token),
            Unwrapped::Node(node) => match node.rule_name.as_str() {
                "program" => {
                    Err(self.err_at(node, "nested program node is not an expression".to_string()))
                }
                "statement_line" | "statement" | "expr" | "atom" => {
                    self.lower_first_node(node, depth)
                }
                "assignment" => self.lower_assignment(node, depth),
                "logical_or" | "logical_and" | "logical_not" => {
                    Err(self.err_unsupported(node, "'AND'/'OR'/'NOT' logical expressions"))
                }
                "comparison" => Err(self.err_unsupported(node, "comparisons (=, <=, <, >, >=)")),
                "additive" | "multiplicative" => self.lower_binary_chain(node, depth),
                "unary" => self.lower_unary(node, depth),
                "power" => Err(self.err_unsupported(node, "exponentiation (^)")),
                "postfix" => self.lower_postfix(node, depth),
                "vector" => Err(self.err_unsupported(node, "vector/matrix literals ([...])")),
                "row" => Err(self.err_at(
                    node,
                    "a `row` node must be lowered via vector-lowering logic, not `lower_node` \
                     directly (and vectors are unsupported in v0 regardless)"
                        .to_string(),
                )),
                "group" => self.lower_group(node, depth),
                "arglist" => Err(self.err_at(
                    node,
                    "an arglist cannot be lowered as a scalar expression".to_string(),
                )),
                other => Err(self.err_at(node, format!("no lowering for rule `{other}`"))),
            },
        }
    }

    /// Lower a raw token (a literal or a bare symbol). Derive has no
    /// `STRING` token and no boolean literal keywords in this grammar, so
    /// unlike `macsyma-iir-compiler::lower_token`, this only ever needs
    /// `NUMBER`/`NAME` arms.
    fn lower_token(&mut self, token: &Token) -> Result<Lowered, DeriveIirError> {
        match token_type(token) {
            "NUMBER" => self.lower_number(&token.value, token),
            "NAME" => Ok(self.symbol_ref(&token.value)),
            other => Err(DeriveIirError {
                message: format!("unexpected token `{other}` = {:?}", token.value),
                line: token.line,
                column: token.column,
            }),
        }
    }

    /// Parse a `NUMBER` lexeme. Unlike `derive-to-semantic-ir`'s
    /// `number_literal_expr`, this does **not** fall back to a float for
    /// a too-large integer — v0 has no float representation at all, so
    /// that shape is an explicit error, never a silent reinterpretation.
    fn lower_number(&mut self, text: &str, token: &Token) -> Result<Lowered, DeriveIirError> {
        if text.contains('.') || text.contains('e') || text.contains('E') {
            return Err(self
                .err_unsupported_tok(token, "floating-point literals (not representable in v0)"));
        }
        match text.parse::<i64>() {
            Ok(v) => Ok(self.emit_int(v)),
            Err(_) => Err(self.err_unsupported_tok(
                token,
                "an integer literal too large for i64 (bignum is not representable in v0)",
            )),
        }
    }

    /// A bare `NAME`: a bound (assigned earlier) variable resolves to its
    /// stored value; anything else is a free symbol. `literal` is always
    /// cleared here — see `macsyma-iir-compiler::symbol_ref`'s identical
    /// note.
    fn symbol_ref(&mut self, name: &str) -> Lowered {
        if let Some(bound) = self.env.get(name) {
            Lowered {
                reg: bound.reg.clone(),
                concrete: bound.concrete,
                literal: None,
            }
        } else {
            self.emit_symbol(name)
        }
    }

    /// `assignment = logical_or [ ASSIGN assignment ] ;`
    ///
    /// See the module doc comment's "`:=` disambiguation" section: v0
    /// rejects a call-shaped LHS as unsupported function definition
    /// *before* attempting to lower it as an expression, rather than
    /// letting it fall through to `lower_postfix`'s generic call
    /// rejection.
    fn lower_assignment(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, DeriveIirError> {
        let Some(op_index) = node
            .children
            .iter()
            .position(|c| as_token(c).is_some_and(|t| token_type(t) == "ASSIGN"))
        else {
            return self.lower_first_node(node, depth);
        };
        if op_index == 0 || op_index + 1 >= node.children.len() {
            return Err(self.err_at(node, "malformed assignment node".to_string()));
        }

        let name = match bare_name(&node.children[op_index - 1]) {
            Some(name) => name,
            None => {
                return Err(self.err_unsupported(
                    node,
                    "function definition (F(x) := body) -- v0 has no user-defined-function \
                     support",
                ))
            }
        };
        let rhs = self.lower_child(&node.children[op_index + 1], depth + 1)?;
        self.env.insert(name, rhs.clone());
        Ok(rhs)
    }

    /// `additive`/`multiplicative` — a left-associative binary chain of
    /// `+`/`-`/`*`/`/`. Same flat-CST-node shape as `macsyma-iir-compiler`
    /// (see [`Self::check_chain_length`]).
    fn lower_binary_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, DeriveIirError> {
        self.check_chain_length(node)?;
        let mut children = node.children.iter();
        let first = children
            .next()
            .ok_or_else(|| self.err_at(node, "empty binary chain".to_string()))?;
        let mut result = self.lower_child(first, depth + 1)?;
        while let Some(op_child) = children.next() {
            let head = as_token(op_child)
                .and_then(|t| binary_head(token_type(t)))
                .ok_or_else(|| self.err_at(node, "expected a binary operator".to_string()))?;
            let rhs_child = children.next().ok_or_else(|| {
                self.err_at(node, "binary operator with no right operand".to_string())
            })?;
            let rhs = self.lower_child(rhs_child, depth + 1)?;
            result = self.combine(head, result, rhs, node)?;
        }
        Ok(result)
    }

    /// Combine `lhs op rhs` for `op` one of `Add`/`Sub`/`Mul`/`Div`. See
    /// the module doc comment's "The `/` exactness rule" section —
    /// identical logic to `macsyma-iir-compiler::Lowerer::combine`.
    fn combine(
        &mut self,
        head: &str,
        lhs: Lowered,
        rhs: Lowered,
        node: &GrammarASTNode,
    ) -> Result<Lowered, DeriveIirError> {
        if head == DIV {
            if !lhs.concrete || !rhs.concrete {
                return Ok(self.inert_apply(DIV, vec![lhs, rhs]));
            }
            return match (lhs.literal, rhs.literal) {
                (Some(_), Some(0)) => Err(self.err_at(node, "division by zero".to_string())),
                // See `macsyma-iir-compiler::Lowerer::combine`'s
                // identical comment: `i64::MIN / -1` panics on plain
                // `i64` division/remainder in every build profile, and
                // is reachable here without ever tripping an overflow
                // error via the checked_sub path below.
                (Some(a), Some(b)) if a.checked_rem(b) == Some(0) => {
                    let reg = self.emit_builtin("/", &[lhs.reg.as_str(), rhs.reg.as_str()], "i64");
                    Ok(Lowered {
                        reg,
                        concrete: true,
                        literal: a.checked_div(b),
                    })
                }
                (Some(a), Some(b)) if a.checked_rem(b).is_some() => Err(self.err_at(
                    node,
                    "this division does not divide evenly; the exact result would be a \
                     rational, which is not representable in v0 (see derive-iir-vm.md)"
                        .to_string(),
                )),
                (Some(_), Some(_)) => Err(self.err_at(
                    node,
                    "this division cannot be evaluated in v0 (i64::MIN / -1 is not representable)"
                        .to_string(),
                )),
                _ => Err(self.err_at(
                    node,
                    "division of a non-literal value cannot be verified exact at compile time \
                     in v0 (only literal / literal is supported); see derive-iir-vm.md"
                        .to_string(),
                )),
            };
        }

        if lhs.concrete && rhs.concrete {
            let op_name = op_symbol_for(head);
            let reg = self.emit_builtin(op_name, &[lhs.reg.as_str(), rhs.reg.as_str()], "i64");
            let literal = match (head, lhs.literal, rhs.literal) {
                (h, Some(a), Some(b)) if h == ADD => a.checked_add(b),
                (h, Some(a), Some(b)) if h == SUB => a.checked_sub(b),
                (h, Some(a), Some(b)) if h == MUL => a.checked_mul(b),
                _ => None,
            };
            Ok(Lowered {
                reg,
                concrete: true,
                literal,
            })
        } else {
            Ok(self.inert_apply(head, vec![lhs, rhs]))
        }
    }

    /// `unary = MINUS unary | power ;` — Derive's grammar has NO
    /// unary-plus alternative at all (unlike Macsyma's), so this only
    /// ever has one or two children, never a `PLUS`-vs-`MINUS` branch.
    fn lower_unary(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, DeriveIirError> {
        match node.children.len() {
            1 => self.lower_child(&node.children[0], depth + 1),
            2 => {
                let operand = self.lower_child(&node.children[1], depth + 1)?;
                if operand.concrete {
                    let reg = self.emit_builtin("-", &[operand.reg.as_str()], "i64");
                    let literal = operand.literal.and_then(i64::checked_neg);
                    Ok(Lowered {
                        reg,
                        concrete: true,
                        literal,
                    })
                } else {
                    Ok(self.inert_apply(NEG, vec![operand]))
                }
            }
            _ => Err(self.err_at(node, "malformed unary node".to_string())),
        }
    }

    /// `postfix = atom { LPAREN [ arglist ] RPAREN } ;` — a bare atom (no
    /// call suffix) passes through; any `(...)` call suffix is rejected
    /// (v0 has no user-defined functions to call, and no way to look one
    /// up if it did).
    fn lower_postfix(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, DeriveIirError> {
        let has_call = node
            .children
            .iter()
            .any(|c| as_token(c).is_some_and(|t| token_type(t) == "LPAREN"));
        if has_call {
            return Err(self.err_unsupported(node, "function calls (e.g. F(x))"));
        }
        let base = node
            .children
            .first()
            .ok_or_else(|| self.err_at(node, "postfix has no base".to_string()))?;
        self.lower_child(base, depth + 1)
    }

    /// `group = LPAREN expr RPAREN ;` — grouping only.
    fn lower_group(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, DeriveIirError> {
        let inner = child_nodes(node)
            .next()
            .ok_or_else(|| self.err_at(node, "empty group `( )`".to_string()))?;
        self.lower_node(inner, depth + 1)
    }

    // -------------------------------------------------------------------
    // Emission helpers
    // -------------------------------------------------------------------

    fn fresh(&mut self) -> String {
        let name = format!("v{}", self.tmp);
        self.tmp += 1;
        name
    }

    fn emit(&mut self, instr: IIRInstr) {
        self.instrs.push(instr);
    }

    fn emit_int(&mut self, n: i64) -> Lowered {
        let reg = self.fresh();
        self.emit(IIRInstr::new(
            "const",
            Some(reg.clone()),
            vec![Operand::Int(n)],
            "i64",
        ));
        Lowered {
            reg,
            concrete: true,
            literal: Some(n),
        }
    }

    fn emit_symbol(&mut self, name: &str) -> Lowered {
        let reg = self.fresh();
        self.emit(IIRInstr::new(
            "const",
            Some(reg.clone()),
            vec![Operand::Var(name.to_string())],
            "symbol",
        ));
        Lowered {
            reg,
            concrete: false,
            literal: None,
        }
    }

    fn emit_nil(&mut self) -> Lowered {
        let reg = self.fresh();
        self.emit(IIRInstr::new(
            "const",
            Some(reg.clone()),
            vec![Operand::Int(0)],
            REF_PAIR,
        ));
        Lowered {
            reg,
            concrete: false,
            literal: None,
        }
    }

    fn emit_builtin(&mut self, name: &str, args: &[&str], type_hint: &str) -> String {
        let reg = self.fresh();
        let mut srcs = vec![Operand::Var(name.to_string())];
        srcs.extend(args.iter().map(|a| Operand::Var((*a).to_string())));
        self.emit(IIRInstr::new(
            "call_builtin",
            Some(reg.clone()),
            srcs,
            type_hint,
        ));
        reg
    }

    /// Materialise an unevaluated `Apply(head, args)` as an inert
    /// `cons`-chain `(head arg0 arg1 …)`, mirroring
    /// `macsyma-iir-compiler::inert_apply` exactly.
    fn inert_apply(&mut self, head: &str, args: Vec<Lowered>) -> Lowered {
        let head_reg = self.emit_symbol(head).reg;
        let mut acc = self.emit_nil().reg;
        for arg in args.into_iter().rev() {
            acc = self.emit_builtin("cons", &[arg.reg.as_str(), acc.as_str()], REF_PAIR);
        }
        let reg = self.emit_builtin("cons", &[head_reg.as_str(), acc.as_str()], REF_PAIR);
        Lowered {
            reg,
            concrete: false,
            literal: None,
        }
    }

    // -------------------------------------------------------------------
    // Guards
    // -------------------------------------------------------------------

    /// Reject a same-precedence operator chain (`additive`/
    /// `multiplicative`) with more than `MAX_EXPR_DEPTH` operands — see
    /// `macsyma-iir-compiler::lower::Lowerer::check_chain_length`'s doc
    /// comment for the full DoS rationale, which applies unchanged here.
    fn check_chain_length(&self, node: &GrammarASTNode) -> Result<(), DeriveIirError> {
        let operand_count = node
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(_)))
            .count();
        if operand_count > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression chain too long ({operand_count} operands, exceeds {MAX_EXPR_DEPTH})"),
            ));
        }
        Ok(())
    }

    // -------------------------------------------------------------------
    // Small helpers
    // -------------------------------------------------------------------

    fn lower_first_node(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, DeriveIirError> {
        let child = child_nodes(node).next().ok_or_else(|| {
            self.err_at(
                node,
                format!("`{}` has no expression child", node.rule_name),
            )
        })?;
        self.lower_node(child, depth + 1)
    }

    fn lower_child(
        &mut self,
        child: &ASTNodeOrToken,
        depth: usize,
    ) -> Result<Lowered, DeriveIirError> {
        match child {
            ASTNodeOrToken::Node(node) => self.lower_node(node, depth),
            ASTNodeOrToken::Token(token) => self.lower_token(token),
        }
    }

    fn err_at(&self, node: &GrammarASTNode, message: String) -> DeriveIirError {
        DeriveIirError {
            message,
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        }
    }

    fn err_unsupported(&self, node: &GrammarASTNode, what: &str) -> DeriveIirError {
        self.err_at(
            node,
            format!("{what} are not supported in this v0 slice — see derive-iir-vm.md"),
        )
    }

    fn err_unsupported_tok(&self, token: &Token, what: &str) -> DeriveIirError {
        DeriveIirError {
            message: format!("{what} are not supported in this v0 slice — see derive-iir-vm.md"),
            line: token.line,
            column: token.column,
        }
    }
}

// ---------------------------------------------------------------------------
// Free helpers (no `&mut self` needed)
// ---------------------------------------------------------------------------

fn child_nodes(node: &GrammarASTNode) -> impl Iterator<Item = &GrammarASTNode> {
    node.children.iter().filter_map(as_node)
}

fn as_node(child: &ASTNodeOrToken) -> Option<&GrammarASTNode> {
    match child {
        ASTNodeOrToken::Node(node) => Some(node),
        ASTNodeOrToken::Token(_) => None,
    }
}

fn as_token(child: &ASTNodeOrToken) -> Option<&Token> {
    match child {
        ASTNodeOrToken::Token(token) => Some(token),
        ASTNodeOrToken::Node(_) => None,
    }
}

fn token_type(token: &Token) -> &str {
    token.effective_type_name()
}

/// Map an arithmetic token type to its canonical IR head. Note `TIMES`,
/// not Macsyma's `STAR` — `derive.tokens` spells the multiplication token
/// `TIMES`.
fn binary_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "PLUS" => Some(ADD),
        "MINUS" => Some(SUB),
        "TIMES" => Some(MUL),
        "SLASH" => Some(DIV),
        _ => None,
    }
}

/// Map a canonical arithmetic head to the `call_builtin` name
/// `dynval-runtime` resolves it to. Only ever called for `ADD`/`SUB`/`MUL`
/// (see [`Lowerer::combine`] — `DIV` is handled separately).
fn op_symbol_for(head: &str) -> &'static str {
    match head {
        h if h == ADD => "+",
        h if h == SUB => "-",
        h if h == MUL => "*",
        _ => unreachable!("op_symbol_for called with a non-arithmetic head: {head}"),
    }
}

/// If `child` (peeling away any single-child wrapper nodes, the same way
/// [`unwrap_single`] does for lowering) bottoms out at a bare `NAME`
/// token, return its text. Used by [`Lowerer::lower_assignment`] to check
/// an assignment target *without* lowering it as an expression.
fn bare_name(child: &ASTNodeOrToken) -> Option<String> {
    let token = match child {
        ASTNodeOrToken::Token(t) => t,
        ASTNodeOrToken::Node(n) => match unwrap_single(n) {
            Unwrapped::Token(t) => t,
            Unwrapped::Node(_) => return None,
        },
    };
    (token_type(token) == "NAME").then(|| token.value.clone())
}

enum Unwrapped<'a> {
    Node(&'a GrammarASTNode),
    Token(&'a Token),
}

/// Peel away single-child wrapper nodes until we reach a node with
/// structure (or a leaf token). Mirrors
/// `macsyma-iir-compiler::lower::unwrap_single`.
fn unwrap_single(mut node: &GrammarASTNode) -> Unwrapped<'_> {
    loop {
        if node.children.len() != 1 {
            return Unwrapped::Node(node);
        }
        match &node.children[0] {
            ASTNodeOrToken::Node(child) => node = child,
            ASTNodeOrToken::Token(token) => return Unwrapped::Token(token),
        }
    }
}
