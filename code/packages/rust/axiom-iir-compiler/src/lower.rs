//! The lowering pass from `coding_adventures_axiom_parser`'s generic
//! [`GrammarASTNode`] CST → [`IIRModule`], **v0.1.0**.
//!
//! # Retargeting `axiom-runtime`/`axiom-to-semantic-ir`, a third time
//!
//! `axiom-runtime` already walks this exact CST and compiles it to
//! `symbolic_ir::IRNode`. `axiom-to-semantic-ir` retargets the same
//! rule-name dispatch to build `semantic_ir::Expr` instead. This module is
//! a **third retarget**, following the precedent every other language in
//! this rollout has established — see
//! [`axiom-iir-vm.md`](../../../specs/axiom-iir-vm.md) and
//! [`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md).
//!
//! # `program` is a SINGLE expression — unlike every sibling in this rollout
//!
//! `axiom.grammar`'s own `program = expr` parses exactly ONE expression per
//! call (Axiom is modeled as a numbered, per-line interactive session, not a
//! batch worksheet — see `axiom-to-semantic-ir/src/lower.rs`'s identical
//! note). So, unlike `macsyma-iir-compiler`/`derive-iir-compiler`/
//! `reduce-iir-compiler`/`maple-iir-compiler`'s own multi-statement
//! `lower_file` loops, [`Lowerer::lower_file`] here lowers exactly one
//! top-level expression into `main`'s body.
//!
//! **A genuine consequence, not carried over from any sibling:** since a
//! compiled module can never contain more than one statement, a bound
//! variable can never be *referenced* after its own `x := e` statement
//! within the same compilation — there is no second statement to read it
//! back in. So this crate's `symbol_ref` needs no `env: HashMap` lookup at
//! all (every sibling crate has one, since their multi-statement programs
//! genuinely thread bindings across statements): a bare `NAME` is *always*
//! a free symbol here. `x := e` still lowers and evaluates `e` and returns
//! its value (matching `axiom-runtime::eval_assignment`'s own "assignment's
//! value is the RHS's value" convention, and every sibling crate's
//! identical treatment) — there is simply no binding to persist, since
//! nothing in the same compiled module could ever read it back.
//!
//! # Scope (v0.1.0)
//!
//! Accepted: integer literals; `+ - * /` (binary chains and unary `-`
//! only); assignment (`x := expr`, always a bare NAME target —
//! `axiom.grammar`'s own `assignment = NAME ASSIGN expr` guarantees this
//! structurally, so unlike Derive's/Reduce's own bare-name-vs-call check,
//! there is nothing to disambiguate); free-symbol references; any other
//! head or symbolic operand, represented as an *unevaluated* inert
//! `cons`-chain.
//!
//! Rejected, with an explicit [`AxiomIirError`]: `Float`/`String` literals
//! (Axiom's grammar has real `STRING` tokens, unlike Derive/Reduce/Maple);
//! `declared_define`/`undeclared_define` (function definitions); `a : T`
//! (declaration), `e :: T` (coercion), `D has C` (category-membership
//! query) — Axiom's own genuinely new territory relative to every sibling
//! in this rollout, with no arithmetic analogue to fall back to; `if`/
//! `then`/`else`; comparisons; `^`/`**` (power); `[...]` list literals;
//! any postfix function call `f(x)`.
//!
//! # The `/` exactness rule
//!
//! Identical hazard and identical fix as every sibling frontend's `/`
//! handling: Axiom's `/` on integers that don't divide evenly returns an
//! exact rational, which `dynval-runtime` cannot represent.
//! [`Lowerer::combine`] only takes the evaluated `call_builtin "/"` path
//! when both direct operands are literal integer tokens at this exact
//! node and their quotient is exact; a symbolic operand falls back to
//! inert data; anything else is rejected.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use symbolic_ir::{ADD, DIV, MUL, NEG, SUB};

/// Maximum expression-nesting depth for *this crate's own* lowering
/// recursion — mirrors every sibling frontend's identically-named guard.
const MAX_EXPR_DEPTH: usize = 256;

/// IIR type-hint string for the nil / cons reference type.
const REF_PAIR: &str = "ref<LispyPair>";

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during Axiom → IIR lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxiomIirError {
    /// Human-readable explanation.
    pub message: String,
    /// 1-based source line.
    pub line: usize,
    /// 1-based source column.
    pub column: usize,
}

impl std::fmt::Display for AxiomIirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AxiomIirError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for AxiomIirError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed Axiom CST (rooted at the `program` rule, a SINGLE
/// expression — see the module doc comment) into an [`IIRModule`].
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<IIRModule, AxiomIirError> {
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
    /// `true` if this value was built purely from integer literals
    /// combined via `+`/`-`/`*`/`/`/unary negate — i.e. it contains no
    /// free symbol anywhere.
    concrete: bool,
    /// `Some(v)` only when this value is a *direct* integer-literal token
    /// (or the statically-known result of combining such literals) —
    /// used exclusively by the `/` exactness check ([`Lowerer::combine`]).
    literal: Option<i64>,
}

/// Unlike every sibling crate's `Lowerer`, this one has **no** mutable
/// environment at all — see the module doc comment's "no `env`" section:
/// a single-expression program can never reference a binding from an
/// earlier statement, because there is no earlier statement.
struct Lowerer {
    instrs: Vec<IIRInstr>,
    tmp: usize,
}

impl Lowerer {
    fn new() -> Self {
        Lowerer {
            instrs: Vec::new(),
            tmp: 0,
        }
    }

    // -------------------------------------------------------------------
    // top level: `program = expr ;` -- a SINGLE expression.
    // -------------------------------------------------------------------

    fn lower_file(
        &mut self,
        program: &GrammarASTNode,
        module_name: &str,
    ) -> Result<IIRModule, AxiomIirError> {
        if program.rule_name != "program" {
            return Err(self.err_at(
                program,
                format!("expected `program` root, got `{}`", program.rule_name),
            ));
        }

        // `lower_node` peels `program`'s own single child away via
        // `unwrap_single` before dispatching -- safe to call directly on
        // the root, mirroring `axiom-to-semantic-ir::Lowerer::
        // lower_program`'s identical direct call.
        let final_val = self.lower_node(program, 0)?;

        self.emit(IIRInstr::new(
            "ret",
            None,
            vec![Operand::Var(final_val.reg)],
            "any",
        ));

        let mut main =
            IIRFunction::new("main", Vec::new(), "any", std::mem::take(&mut self.instrs));
        main.register_count = self.tmp;

        let mut module = IIRModule::new(module_name, "axiom");
        module.functions.push(main);
        module.entry_point = Some("main".to_string());

        let problems = module.validate();
        if !problems.is_empty() {
            return Err(AxiomIirError {
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
    ) -> Result<Lowered, AxiomIirError> {
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
                "if_expr" => Err(self.err_unsupported(node, "if/then/else")),
                "declared_define" | "undeclared_define" => {
                    Err(self.err_unsupported(node, "function definitions"))
                }
                "assignment" => self.lower_assignment(node, depth),
                "declaration" => Err(self.err_unsupported(node, "declaration (a : T)")),
                "has_query" => {
                    Err(self.err_unsupported(node, "category-membership query (D has C)"))
                }
                "comparison" => {
                    Err(self.err_unsupported(node, "comparisons (=, ~=, <, >, <=, >=)"))
                }
                "coercion" => Err(self.err_unsupported(node, "coercion (e :: T)")),
                "additive" | "multiplicative" => self.lower_binary_chain(node, depth),
                "unary" => self.lower_unary(node, depth),
                "power" => Err(self.err_unsupported(node, "exponentiation (^)")),
                "postfix" => self.lower_postfix(node, depth),
                "atom" => self.lower_first_node(node, depth),
                "list_literal" => Err(self.err_unsupported(node, "list literals ([...])")),
                "group" => self.lower_group(node, depth),
                "call_args" => Err(self.err_at(
                    node,
                    "`call_args` cannot be lowered as a standalone expression".to_string(),
                )),
                "arglist" => Err(self.err_at(
                    node,
                    "an arglist cannot be lowered as a scalar expression".to_string(),
                )),
                "elem_list" => Err(self.err_at(
                    node,
                    "an elem_list cannot be lowered as a scalar expression".to_string(),
                )),
                "type_expr" => Err(self.err_at(
                    node,
                    "a bare type expression is not a value-producing expression".to_string(),
                )),
                other => Err(self.err_at(node, format!("no lowering for rule `{other}`"))),
            },
        }
    }

    /// Lower a raw token (a literal or a bare symbol). Axiom has real
    /// `STRING` tokens (unlike Derive/Reduce/Maple) — v0 rejects them,
    /// matching its treatment of every other out-of-scope literal kind.
    fn lower_token(&mut self, token: &Token) -> Result<Lowered, AxiomIirError> {
        match token_type(token) {
            "NUMBER" => self.lower_number(&token.value, token),
            "NAME" => Ok(self.emit_symbol(&token.value)),
            "STRING" => Err(self.err_unsupported_tok(token, "string literals")),
            other => Err(AxiomIirError {
                message: format!("unexpected token `{other}` = {:?}", token.value),
                line: token.line,
                column: token.column,
            }),
        }
    }

    /// Parse a `NUMBER` lexeme. Unlike `axiom-to-semantic-ir`'s
    /// `number_literal_expr`, this does **not** fall back to a float for
    /// a too-large integer — v0 has no float representation at all.
    fn lower_number(&mut self, text: &str, token: &Token) -> Result<Lowered, AxiomIirError> {
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

    /// `assignment = NAME ASSIGN expr ;` — the grammar guarantees a bare
    /// NAME target, so there is nothing to disambiguate. See the module
    /// doc comment's "no `env`" section: the assigned value is lowered
    /// and returned, but never bound anywhere — there is no later
    /// statement in the same compiled module that could ever read it
    /// back.
    fn lower_assignment(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, AxiomIirError> {
        let has_name = node
            .children
            .iter()
            .filter_map(as_token)
            .any(|t| token_type(t) == "NAME");
        if !has_name {
            return Err(self.err_at(node, "malformed assignment: missing name".to_string()));
        }
        let rhs_node = child_nodes(node).next().ok_or_else(|| {
            self.err_at(
                node,
                "malformed assignment: missing right-hand side".to_string(),
            )
        })?;
        self.lower_node(rhs_node, depth + 1)
    }

    /// `additive`/`multiplicative` — a left-associative binary chain of
    /// `+`/`-`/`*`/`/`.
    fn lower_binary_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, AxiomIirError> {
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
    ) -> Result<Lowered, AxiomIirError> {
        if head == DIV {
            if !lhs.concrete || !rhs.concrete {
                return Ok(self.inert_apply(DIV, vec![lhs, rhs]));
            }
            return match (lhs.literal, rhs.literal) {
                (Some(_), Some(0)) => Err(self.err_at(node, "division by zero".to_string())),
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
                     rational, which is not representable in v0 (see axiom-iir-vm.md)"
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
                     in v0 (only literal / literal is supported); see axiom-iir-vm.md"
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

    /// `unary = MINUS unary | power ;` — Axiom's grammar has no
    /// unary-plus alternative.
    fn lower_unary(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, AxiomIirError> {
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

    /// `postfix = atom [ call_args ] ;` — a bare atom (no call suffix)
    /// passes through; any call suffix is rejected (v0 has no
    /// user-defined functions to call).
    fn lower_postfix(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, AxiomIirError> {
        let has_call = child_nodes(node).any(|n| n.rule_name == "call_args");
        if has_call {
            return Err(self.err_unsupported(node, "function calls (e.g. f(x))"));
        }
        let base = child_nodes(node)
            .next()
            .ok_or_else(|| self.err_at(node, "postfix has no base".to_string()))?;
        self.lower_node(base, depth + 1)
    }

    /// `group = LPAREN expr RPAREN ;` — grouping only.
    fn lower_group(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, AxiomIirError> {
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
    /// `cons`-chain `(head arg0 arg1 …)`.
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
    /// `multiplicative`) with more than `MAX_EXPR_DEPTH` operands.
    fn check_chain_length(&self, node: &GrammarASTNode) -> Result<(), AxiomIirError> {
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
    ) -> Result<Lowered, AxiomIirError> {
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
    ) -> Result<Lowered, AxiomIirError> {
        match child {
            ASTNodeOrToken::Node(node) => self.lower_node(node, depth),
            ASTNodeOrToken::Token(token) => self.lower_token(token),
        }
    }

    fn err_at(&self, node: &GrammarASTNode, message: String) -> AxiomIirError {
        AxiomIirError {
            message,
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        }
    }

    fn err_unsupported(&self, node: &GrammarASTNode, what: &str) -> AxiomIirError {
        self.err_at(
            node,
            format!("{what} are not supported in this v0 slice — see axiom-iir-vm.md"),
        )
    }

    fn err_unsupported_tok(&self, token: &Token, what: &str) -> AxiomIirError {
        AxiomIirError {
            message: format!("{what} are not supported in this v0 slice — see axiom-iir-vm.md"),
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

/// Map an arithmetic token type to its canonical IR head.
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

enum Unwrapped<'a> {
    Node(&'a GrammarASTNode),
    Token(&'a Token),
}

/// Peel away single-child wrapper nodes until we reach a node with
/// structure (or a leaf token).
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
