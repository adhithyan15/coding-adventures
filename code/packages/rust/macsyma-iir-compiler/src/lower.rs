//! The lowering pass from `coding_adventures_macsyma_parser`'s generic
//! [`GrammarASTNode`] CST → [`IIRModule`], **v0.1.0**.
//!
//! # Retargeting `macsyma-compiler`, a third time
//!
//! `macsyma-compiler` already walks this exact CST and compiles it to
//! `symbolic_ir::IRNode`. `macsyma-to-semantic-ir` retargets the same
//! rule-name dispatch to build `semantic_ir::Expr` instead. This module is
//! a **third retarget**, emitting `interpreter_ir::IIRInstr`s directly —
//! see [`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md) §1. It
//! shares no code with either sibling beyond the parser; all three are
//! independent consumers of one CST.
//!
//! # Scope (v0.1.0)
//!
//! Accepted: integer literals; `+ - * /` (binary chains and unary
//! `-`/`+`); assignment (`x: expr`, plain-name target only); free-symbol
//! references; any other head or symbolic operand, represented as an
//! *unevaluated* inert `cons`-chain (mirrors
//! `mccarthy-lisp-iir-compiler::lower_quote`'s `QUOTE` materialisation).
//!
//! Rejected, with an explicit [`MacsymaIirError`] rather than a silent
//! mis-lowering: `Rational`/`Float`/`Str` literals; `:=` (function
//! definition); `if`/`while`/`for`/`block`/`return`; `[...]` (list
//! literals); comparisons; `and`/`or`/`not`; `^`/`**` (power); any postfix
//! function call `f(x)`.
//!
//! Comparisons, `and`/`or`/`not`, and `^` are rejected **outright** —
//! unlike `+`/`-`/`*`/`/`, they are *not* given an inert-data fallback for
//! symbolic operands. `symbolic-vm`'s handler table (the real evaluator
//! `macsyma-runtime` runs) has genuine handlers for `Pow`/comparison/
//! logical heads that evaluate a concrete pair of operands (`2^3` → `8`,
//! `3<5` → `true`) rather than leaving them symbolic — so building inert
//! data for a *concrete* instance of one of these would disagree with
//! `macsyma-runtime`'s own ground truth. `+`/`-`/`*` never have this
//! hazard (integer arithmetic is always exact); `/` has a narrower version
//! of the same hazard, handled explicitly below rather than by blanket
//! rejection.
//!
//! # The `/` exactness rule
//!
//! Macsyma's `/` on integers that don't divide evenly returns an exact
//! `Rational` (`7/2` stays `7/2`), which `dynval-runtime` cannot represent
//! (§3 of the spec). `dynval-runtime`'s own `/` builtin is C-style
//! truncating division, so emitting it unconditionally would silently
//! produce a wrong answer whenever a division isn't exact — a real
//! correctness bug, not a disclosed gap. [`Lowerer::combine`] resolves
//! this without constant-folding the surrounding arithmetic (which would
//! defeat the point of targeting the VM — see the spec's "genuinely
//! executed... not frontend-folded" goal): a `/` node only takes the
//! evaluated `call_builtin "/"` path when **both** its direct operands are
//! literal integer tokens *at this exact node* and their quotient is
//! exact; a symbolic operand falls back to inert data (matching
//! `macsyma-runtime` leaving `x/y` unevaluated); anything else — a
//! concrete-but-non-literal operand (e.g. the result of a prior
//! computation, or a variable), or a non-exact literal division — is
//! rejected, since neither the evaluated nor the inert-data
//! representation is verifiably correct for it in v0.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use symbolic_ir::{ADD, DIV, MUL, NEG, SUB};

/// Maximum expression-nesting depth for this crate's own lowering
/// recursion — mirrors `macsyma-to-semantic-ir::lower::MAX_EXPR_DEPTH`
/// (same CST, same generic `GrammarParser` dispatch engine, same measured
/// native-stack crash floor).
const MAX_EXPR_DEPTH: usize = 256;

/// IIR type-hint string for the nil / cons reference type, matching
/// `mccarthy-lisp-iir-compiler`'s `REF_PAIR` convention (the VM and every
/// IIR backend special-case `const 0 : ref<LispyPair>` into the nil
/// sentinel and `call_builtin "cons"` into a fresh pair).
const REF_PAIR: &str = "ref<LispyPair>";

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during Macsyma → IIR lowering.
///
/// Mirrors `MacsymaLowerError`/`WolframLowerError`/etc.'s shape exactly
/// (`message` + 1-based `line`/`column`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacsymaIirError {
    /// Human-readable explanation.
    pub message: String,
    /// 1-based source line.
    pub line: usize,
    /// 1-based source column.
    pub column: usize,
}

impl std::fmt::Display for MacsymaIirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MacsymaIirError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for MacsymaIirError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed Macsyma CST (rooted at the `program` rule) into an
/// [`IIRModule`].
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<IIRModule, MacsymaIirError> {
    Lowerer::new().lower_file(tree, module_name)
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

/// The lowered value of one CST node: the IIR register holding it, plus
/// two facts used only to decide how a *later* arithmetic step may
/// combine it — never observed by the emitted IIR itself.
#[derive(Debug, Clone)]
struct Lowered {
    /// The register this value lives in.
    reg: String,
    /// `true` if this value was built purely from integer literals and
    /// bound (concrete) variables combined via `+`/`-`/`*`/`/`/unary
    /// negate — i.e. it contains no free symbol anywhere. Concrete values
    /// are evaluated via a real `call_builtin`; anything else is
    /// represented as inert symbolic data.
    concrete: bool,
    /// `Some(v)` only when this value is a *direct* integer-literal token
    /// (or the statically-known result of combining such literals via
    /// `+`/`-`/`*`/unary negate) — used exclusively by the `/` exactness
    /// check ([`Lowerer::combine`]). Never propagated through a variable
    /// reference, matching the "direct literal token" restriction that
    /// check relies on.
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
    // top level: `program = { statement }`
    // -------------------------------------------------------------------

    fn lower_file(
        &mut self,
        program: &GrammarASTNode,
        module_name: &str,
    ) -> Result<IIRModule, MacsymaIirError> {
        if program.rule_name != "program" {
            return Err(self.err_at(
                program,
                format!("expected `program` root, got `{}`", program.rule_name),
            ));
        }

        let mut last: Option<Lowered> = None;
        for stmt_node in child_nodes(program) {
            if stmt_node.rule_name != "statement" {
                continue;
            }
            last = Some(self.lower_node(stmt_node, 0)?);
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

        let mut module = IIRModule::new(module_name, "macsyma");
        module.functions.push(main);
        module.entry_point = Some("main".to_string());

        let problems = module.validate();
        if !problems.is_empty() {
            return Err(MacsymaIirError {
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
    ) -> Result<Lowered, MacsymaIirError> {
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
                "statement" | "expression" | "atom" => self.lower_first_node(node, depth),
                "assign" => self.lower_assign(node, depth),
                "logical_or" | "logical_and" | "logical_not" => {
                    Err(self.err_unsupported(node, "'and'/'or'/'not' logical expressions"))
                }
                "comparison" => Err(self.err_unsupported(node, "comparisons (=, #, <, >, <=, >=)")),
                "additive" | "multiplicative" => self.lower_binary_chain(node, depth),
                "unary" => self.lower_unary(node, depth),
                "power" => Err(self.err_unsupported(node, "exponentiation (^ / **)")),
                "postfix" => self.lower_postfix(node, depth),
                "group" => self.lower_group(node, depth),
                "list" => Err(self.err_unsupported(node, "list literals ([...])")),
                "if_expr" => Err(self.err_unsupported(node, "if/elseif/else")),
                "while_expr" => Err(self.err_unsupported(node, "while")),
                "for_expr" | "for_each_expr" | "for_range_expr" => {
                    Err(self.err_unsupported(node, "for loops"))
                }
                "block_expr" => Err(self.err_unsupported(node, "block(...)")),
                "return_expr" => Err(self.err_unsupported(node, "return(...)")),
                "arglist" => Err(self.err_at(
                    node,
                    "an arglist cannot be lowered as a scalar expression".to_string(),
                )),
                other => Err(self.err_at(node, format!("no lowering for rule `{other}`"))),
            },
        }
    }

    fn lower_token(&mut self, token: &Token) -> Result<Lowered, MacsymaIirError> {
        match token_type(token) {
            "NUMBER" => self.lower_number(&token.value, token),
            "NAME" => Ok(self.symbol_ref(&token.value)),
            "STRING" => Err(self.err_unsupported_tok(token, "string literals")),
            "KEYWORD" if token.value == "true" || token.value == "false" => {
                Err(self.err_unsupported_tok(token, "boolean literals"))
            }
            other => Err(MacsymaIirError {
                message: format!("unexpected token `{other}` = {:?}", token.value),
                line: token.line,
                column: token.column,
            }),
        }
    }

    /// Parse a `NUMBER` lexeme. Unlike `macsyma-to-semantic-ir`'s
    /// `number_literal_expr`, this does **not** fall back to a float for a
    /// too-large integer — v0 has no float representation at all (§3), so
    /// that shape is an explicit error, never a silent reinterpretation.
    fn lower_number(&mut self, text: &str, token: &Token) -> Result<Lowered, MacsymaIirError> {
        if text.contains('.') || text.contains('e') || text.contains('E') {
            return Err(self.err_unsupported_tok(
                token,
                "floating-point literals (Rational/Float are not representable in v0)",
            ));
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
    /// stored value; anything else is a free symbol.
    ///
    /// `literal` is always cleared here, even if the bound value happens
    /// to trace back to a literal integer (`x: 6$ x`) — a variable
    /// reference is never a *direct* literal token, and [`Lowerer::combine`]'s
    /// `/` exactness check relies on that distinction (see the module doc
    /// comment).
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

    /// `assign = logical_or [ ( COLON | COLONEQ ) assign ] ;`
    ///
    /// Only `x: e` (`COLON`) is in scope. `f(x) := body` / `name := body`
    /// (`COLONEQ`, function definition) is rejected — v0 has no way to
    /// call a user-defined function (`postfix` with an `LPAREN` suffix is
    /// itself rejected), so a definition it could never invoke would be
    /// dead weight, not a useful accepted construct.
    fn lower_assign(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, MacsymaIirError> {
        let Some(op_index) = node.children.iter().position(|c| {
            as_token(c).is_some_and(|t| matches!(token_type(t), "COLON" | "COLONEQ"))
        }) else {
            return self.lower_first_node(node, depth);
        };
        if op_index == 0 || op_index + 1 >= node.children.len() {
            return Err(self.err_at(node, "malformed assign node".to_string()));
        }
        let op = token_type(as_token(&node.children[op_index]).unwrap());
        if op == "COLONEQ" {
            return Err(self.err_unsupported(node, "function definition (:=)"));
        }

        let name = match bare_name(&node.children[op_index - 1]) {
            Some(name) => name,
            None => {
                return Err(self.err_at(
                    node,
                    "assignment target must be a plain variable name (e.g. `x: 3`)".to_string(),
                ))
            }
        };
        let rhs = self.lower_child(&node.children[op_index + 1], depth + 1)?;
        self.env.insert(name, rhs.clone());
        Ok(rhs)
    }

    /// `additive`/`multiplicative` — a left-associative binary chain of
    /// `+`/`-`/`*`/`/`. Same flat-CST-node shape as `macsyma-to-semantic-ir`
    /// (see [`Self::check_chain_length`]).
    fn lower_binary_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, MacsymaIirError> {
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
    /// the module doc comment's "The `/` exactness rule" section.
    fn combine(
        &mut self,
        head: &str,
        lhs: Lowered,
        rhs: Lowered,
        node: &GrammarASTNode,
    ) -> Result<Lowered, MacsymaIirError> {
        if head == DIV {
            if !lhs.concrete || !rhs.concrete {
                return Ok(self.inert_apply(DIV, vec![lhs, rhs]));
            }
            return match (lhs.literal, rhs.literal) {
                (Some(_), Some(0)) => Err(self.err_at(node, "division by zero".to_string())),
                (Some(a), Some(b)) if a % b == 0 => {
                    let reg = self.emit_builtin("/", &[lhs.reg.as_str(), rhs.reg.as_str()], "i64");
                    Ok(Lowered { reg, concrete: true, literal: Some(a / b) })
                }
                (Some(_), Some(_)) => Err(self.err_at(
                    node,
                    "this division does not divide evenly; the exact result would be a Rational, \
                     which is not representable in v0 (see macsyma-iir-vm.md \u{a7}3/\u{a7}6)"
                        .to_string(),
                )),
                _ => Err(self.err_at(
                    node,
                    "division of a non-literal value cannot be verified exact at compile time in v0 \
                     (only literal / literal is supported); see macsyma-iir-vm.md \u{a7}3"
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

    /// `unary = ( MINUS | PLUS ) unary | power ;`
    fn lower_unary(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, MacsymaIirError> {
        match node.children.len() {
            1 => self.lower_child(&node.children[0], depth + 1),
            2 => {
                let op =
                    token_type(as_token(&node.children[0]).ok_or_else(|| {
                        self.err_at(node, "unary op must be a token".to_string())
                    })?);
                let operand = self.lower_child(&node.children[1], depth + 1)?;
                if op == "MINUS" {
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
                } else {
                    Ok(operand) // unary plus is a no-op
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
    ) -> Result<Lowered, MacsymaIirError> {
        let has_call = node
            .children
            .iter()
            .any(|c| as_token(c).is_some_and(|t| token_type(t) == "LPAREN"));
        if has_call {
            return Err(self.err_unsupported(node, "function calls (e.g. f(x))"));
        }
        let base = node
            .children
            .first()
            .ok_or_else(|| self.err_at(node, "postfix has no base".to_string()))?;
        self.lower_child(base, depth + 1)
    }

    /// `group = LPAREN expression RPAREN ;` — grouping only.
    fn lower_group(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, MacsymaIirError> {
        let inner = child_nodes(node)
            .into_iter()
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
    /// `mccarthy-lisp-iir-compiler::lower_quote`'s `QUOTE` shape.
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
    /// `macsyma-to-semantic-ir::lower::Lowerer::check_chain_length`'s doc
    /// comment for the full DoS rationale, which applies unchanged here.
    fn check_chain_length(&self, node: &GrammarASTNode) -> Result<(), MacsymaIirError> {
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
    ) -> Result<Lowered, MacsymaIirError> {
        let child = child_nodes(node).into_iter().next().ok_or_else(|| {
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
    ) -> Result<Lowered, MacsymaIirError> {
        match child {
            ASTNodeOrToken::Node(node) => self.lower_node(node, depth),
            ASTNodeOrToken::Token(token) => self.lower_token(token),
        }
    }

    fn err_at(&self, node: &GrammarASTNode, message: String) -> MacsymaIirError {
        MacsymaIirError {
            message,
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        }
    }

    fn err_unsupported(&self, node: &GrammarASTNode, what: &str) -> MacsymaIirError {
        self.err_at(
            node,
            format!("{what} are not supported in this v0 slice — see macsyma-iir-vm.md \u{a7}4"),
        )
    }

    fn err_unsupported_tok(&self, token: &Token, what: &str) -> MacsymaIirError {
        MacsymaIirError {
            message: format!(
                "{what} are not supported in this v0 slice — see macsyma-iir-vm.md \u{a7}4"
            ),
            line: token.line,
            column: token.column,
        }
    }
}

// ---------------------------------------------------------------------------
// Free helpers (no `&mut self` needed)
// ---------------------------------------------------------------------------

fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
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

/// Map an arithmetic token type to its canonical IR head.
fn binary_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "PLUS" => Some(ADD),
        "MINUS" => Some(SUB),
        "STAR" => Some(MUL),
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
/// token, return its text. Used by [`Lowerer::lower_assign`] to check an
/// assignment target *without* lowering it as an expression (a bare NAME
/// on the left of `:` is a binding target, not a symbol read).
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
/// `macsyma-to-semantic-ir::lower::unwrap_single`.
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
