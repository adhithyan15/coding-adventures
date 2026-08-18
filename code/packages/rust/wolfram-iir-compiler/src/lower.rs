//! The lowering pass from `coding_adventures_wolfram_parser`'s generic
//! [`GrammarASTNode`] CST → [`IIRModule`], **v0.1.0**.
//!
//! # Retargeting `wolfram-runtime`/`wolfram-to-semantic-ir`, a third time
//!
//! `wolfram-runtime` already walks this exact CST and compiles it to
//! `symbolic_ir::IRNode`. `wolfram-to-semantic-ir` retargets the same
//! rule-name dispatch to build `semantic_ir::Expr` instead — covering
//! Wolfram's **full** grammar (pattern blanks, replacement rules,
//! `#`/pure functions, `/@`/`@@` sugar, and more), since SIR23's
//! "everything is data" design has no scope pressure forcing a narrower
//! cut there. This module is a **third retarget**, following every
//! sibling in this rollout's precedent directly — see
//! [`wolfram-iir-vm.md`](../../../specs/wolfram-iir-vm.md) and
//! [`macsyma-iir-vm.md`](../../../specs/macsyma-iir-vm.md) — but, unlike
//! `wolfram-to-semantic-ir`, this crate deliberately does NOT cover the
//! full grammar: v0's arithmetic/assignment/unevaluated-Apply scope is
//! identical to every other Wave 5 item, so pattern matching, rules,
//! replacement, pure functions, and `/@`/`@@` sugar are all rejected
//! outright here, even though `wolfram-to-semantic-ir` happily lowers
//! all of them. The richer grammar is real; the v0 scope cut is not
//! narrower because the grammar demands it (unlike Reduce's/Maple's own
//! genuinely-new constructs having no arithmetic fallback) — it is
//! narrower because this crate's OWN v0 scope, set once in
//! `macsyma-iir-vm.md` and held constant across all six Wave 5 items, is
//! narrower than what the grammar could support.
//!
//! # Scope (v0.1.0)
//!
//! Accepted: integer literals; `+ - * /` (binary chains) and **both**
//! unary `-`/`+` (Wolfram, unlike Derive/Reduce/Maple/Axiom, has a real
//! unary-plus no-op, matching Macsyma's own grammar); assignment (`x =
//! expr`, `SET` only, always a bare NAME target — `SETDELAYED` (`:=`,
//! function/pattern definition) is rejected outright, mirroring
//! `macsyma-iir-compiler`'s own `COLON`-vs-`COLONEQ` split exactly, since
//! Wolfram's grammar likewise has two distinct assignment tokens rather
//! than Derive's/Reduce's single overloaded one); free-symbol
//! references; any other head or symbolic operand, represented as an
//! *unevaluated* inert `cons`-chain.
//!
//! Rejected, with an explicit [`WolframIirError`]: `Float`/`String`
//! literals; pattern blanks (`_`/`_h`), rules (`->`/`:>`), replacement
//! (`/.`/`//.`), conditions (`/;`), alternatives (`|`), pattern tests
//! (`?`) — Wolfram's own genuinely new territory relative to every
//! sibling in this rollout, none of which has any arithmetic analogue;
//! comparisons; logic (`&&`/`||`/`!`); `^` (power); pure functions (`#`/
//! `#n`/`##`/`expr &`); `/@`/`@@` (map/apply sugar); `{...}` list
//! literals; any postfix suffix at all (`f[x]`, `x[[i]]` Part-indexing,
//! …) — Wolfram's `postfix` has several distinct suffix shapes, so v0
//! rejects the construct the moment ANY suffix is present, rather than
//! trying to distinguish "a call" from "a Part index" the way
//! `wolfram-to-semantic-ir` itself has to.
//!
//! # The `/` exactness rule
//!
//! Identical hazard and identical fix as every sibling frontend's `/`
//! handling: Wolfram's `/` on integers that don't divide evenly returns
//! an exact rational, which `dynval-runtime` cannot represent.
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

/// An error encountered during Wolfram → IIR lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WolframIirError {
    /// Human-readable explanation.
    pub message: String,
    /// 1-based source line.
    pub line: usize,
    /// 1-based source column.
    pub column: usize,
}

impl std::fmt::Display for WolframIirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "WolframIirError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for WolframIirError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed Wolfram CST (rooted at the `program` rule) into an
/// [`IIRModule`].
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<IIRModule, WolframIirError> {
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
    /// negate — i.e. it contains no free symbol anywhere.
    concrete: bool,
    /// `Some(v)` only when this value is a *direct* integer-literal token
    /// (or the statically-known result of combining such literals) —
    /// used exclusively by the `/` exactness check ([`Lowerer::combine`]).
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
    // `statement_line = statement (NEWLINE|SEMI) | statement | NEWLINE | SEMI ;`
    // -------------------------------------------------------------------

    fn lower_file(
        &mut self,
        program: &GrammarASTNode,
        module_name: &str,
    ) -> Result<IIRModule, WolframIirError> {
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

        let mut module = IIRModule::new(module_name, "wolfram");
        module.functions.push(main);
        module.entry_point = Some("main".to_string());

        let problems = module.validate();
        if !problems.is_empty() {
            return Err(WolframIirError {
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
    ) -> Result<Lowered, WolframIirError> {
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
                "replaceall" => Err(self.err_unsupported(node, "replacement (/. or //.)")),
                "rule" => Err(self.err_unsupported(node, "rules (-> or :>)")),
                "condition" => Err(self.err_unsupported(node, "pattern conditions (/;)")),
                "alternatives" => Err(self.err_unsupported(node, "pattern alternatives (|)")),
                "patterntest" => Err(self.err_unsupported(node, "pattern tests (?)")),
                "logical_or" | "logical_and" | "logical_not" => {
                    Err(self.err_unsupported(node, "'&&'/'||'/'!' logical expressions"))
                }
                "comparison" => {
                    Err(self.err_unsupported(node, "comparisons (==, !=, <, >, <=, >=)"))
                }
                "additive" | "multiplicative" => self.lower_binary_chain(node, depth),
                "unary" => self.lower_unary(node, depth),
                "power" => Err(self.err_unsupported(node, "exponentiation (^)")),
                "amp" => Err(self.err_unsupported(node, "pure functions (expr &)")),
                "mapapply" => Err(self.err_unsupported(node, "map/apply sugar (/@ or @@)")),
                "postfix" => self.lower_postfix(node, depth),
                "slot" => Err(self.err_unsupported(node, "slots (# / #n)")),
                "list" => Err(self.err_unsupported(node, "list literals ({...})")),
                "group" => self.lower_group(node, depth),
                "arglist" => Err(self.err_at(
                    node,
                    "an arglist cannot be lowered as a scalar expression".to_string(),
                )),
                other => Err(self.err_at(node, format!("no lowering for rule `{other}`"))),
            },
        }
    }

    /// Lower a raw token (a literal or a bare symbol). Pattern-related
    /// tokens (`BLANK`, `HASH`, `SLOTSEQ`) are rejected explicitly — v0
    /// has no pattern-matching or pure-function support at all.
    fn lower_token(&mut self, token: &Token) -> Result<Lowered, WolframIirError> {
        match token_type(token) {
            "NUMBER" => self.lower_number(&token.value, token),
            "NAME" => Ok(self.symbol_ref(&token.value)),
            "STRING" => Err(self.err_unsupported_tok(token, "string literals")),
            "BLANK" => Err(self.err_unsupported_tok(token, "pattern blanks (_)")),
            "HASH" => Err(self.err_unsupported_tok(token, "slots (#)")),
            "SLOTSEQ" => Err(self.err_unsupported_tok(token, "slot sequences (##)")),
            other => Err(WolframIirError {
                message: format!("unexpected token `{other}` = {:?}", token.value),
                line: token.line,
                column: token.column,
            }),
        }
    }

    /// Parse a `NUMBER` lexeme. Unlike `wolfram-to-semantic-ir`'s
    /// `number_literal_expr`, this does **not** fall back to a float for
    /// a too-large integer — v0 has no float representation at all.
    fn lower_number(&mut self, text: &str, token: &Token) -> Result<Lowered, WolframIirError> {
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
    /// stored value; anything else is a free symbol.
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

    /// `assignment = replaceall [ (SET|SETDELAYED) assignment ]` — see
    /// the module doc comment: `SET` (`=`) is plain assignment, always a
    /// bare-NAME target in v0; `SETDELAYED` (`:=`, function/pattern
    /// definition) is rejected outright, mirroring
    /// `macsyma-iir-compiler`'s own `COLON`-vs-`COLONEQ` split exactly.
    fn lower_assignment(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, WolframIirError> {
        let Some(op_index) = node.children.iter().position(|c| {
            as_token(c).is_some_and(|t| matches!(token_type(t), "SET" | "SETDELAYED"))
        }) else {
            return self.lower_first_node(node, depth);
        };
        if op_index == 0 || op_index + 1 >= node.children.len() {
            return Err(self.err_at(node, "malformed assignment node".to_string()));
        }
        let op = token_type(as_token(&node.children[op_index]).unwrap());
        if op == "SETDELAYED" {
            return Err(self.err_unsupported(node, "function/pattern definition (:=)"));
        }

        let name = match bare_name(&node.children[op_index - 1]) {
            Some(name) => name,
            None => {
                return Err(self.err_at(
                    node,
                    "assignment target must be a plain variable name (e.g. `x = 3`)".to_string(),
                ))
            }
        };
        let rhs = self.lower_child(&node.children[op_index + 1], depth + 1)?;
        self.env.insert(name, rhs.clone());
        Ok(rhs)
    }

    /// `additive`/`multiplicative` — a left-associative binary chain of
    /// `+`/`-`/`*`/`/`.
    fn lower_binary_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, WolframIirError> {
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
    ) -> Result<Lowered, WolframIirError> {
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
                     rational, which is not representable in v0 (see wolfram-iir-vm.md)"
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
                     in v0 (only literal / literal is supported); see wolfram-iir-vm.md"
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

    /// `unary = ( MINUS | PLUS ) unary | power ;` — Wolfram, unlike every
    /// other language in this rollout, has a real unary-plus (a no-op),
    /// matching Macsyma's own grammar shape exactly.
    fn lower_unary(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, WolframIirError> {
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

    /// `postfix = atom { suffix } ;` — a bare atom (no suffix at all)
    /// passes through; ANY suffix (a call `f[x]`, Part-indexing
    /// `x[[i]]`, or anything else `wolfram-to-semantic-ir`'s own richer
    /// `lower_postfix` handles) is rejected — v0 makes no attempt to
    /// distinguish suffix shapes, unlike that sibling crate.
    fn lower_postfix(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, WolframIirError> {
        if node.children.len() > 1 {
            return Err(self.err_unsupported(
                node,
                "postfix suffixes (function calls, Part-indexing, etc.)",
            ));
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
    ) -> Result<Lowered, WolframIirError> {
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
    fn check_chain_length(&self, node: &GrammarASTNode) -> Result<(), WolframIirError> {
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
    ) -> Result<Lowered, WolframIirError> {
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
    ) -> Result<Lowered, WolframIirError> {
        match child {
            ASTNodeOrToken::Node(node) => self.lower_node(node, depth),
            ASTNodeOrToken::Token(token) => self.lower_token(token),
        }
    }

    fn err_at(&self, node: &GrammarASTNode, message: String) -> WolframIirError {
        WolframIirError {
            message,
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        }
    }

    fn err_unsupported(&self, node: &GrammarASTNode, what: &str) -> WolframIirError {
        self.err_at(
            node,
            format!("{what} are not supported in this v0 slice — see wolfram-iir-vm.md"),
        )
    }

    fn err_unsupported_tok(&self, token: &Token, what: &str) -> WolframIirError {
        WolframIirError {
            message: format!("{what} are not supported in this v0 slice — see wolfram-iir-vm.md"),
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
