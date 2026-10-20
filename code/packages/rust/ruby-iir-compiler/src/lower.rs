//! # Walking the rb-int-puts parse tree into IIR
//!
//! This is the hand-written half of the LANG84 pipeline. Everything before it
//! is generic and table-driven (`GrammarLexer` over `ruby<release>.tokens`,
//! `GrammarParser` over `ruby<release>.grammar`); everything after it is a
//! LANG83 backend. The input is the `GrammarASTNode` the parser built, read
//! in place: there is no owned Ruby AST, no re-scanning of source text, and
//! no second parser (LANG84 C1, C2).
//!
//! ## The tree, for `puts(1 + 2 * 3)`
//!
//! The 3.3 grammar keeps `ruby.grammar`'s rule names (LANG84 C8). Several of
//! them (`ternary` … `shift`) are single-child links inside this subset —
//! kept so later subsets can grow into them without renaming anything:
//!
//! ```text
//! program
//! └─ statement
//!    └─ method_call
//!       ├─ NAME "puts"
//!       ├─ LPAREN "("
//!       ├─ call_arg
//!       │  └─ expression → ternary → range → logical_or → logical_and
//!       │       → logical_not → comparison → shift → sum
//!       │                                             ├─ term ─ factor ─ NUMBER "1"
//!       │                                             ├─ PLUS "+"
//!       │                                             └─ term
//!       │                                                ├─ factor ─ NUMBER "2"
//!       │                                                ├─ STAR "*"
//!       │                                                └─ factor ─ NUMBER "3"
//!       └─ RPAREN ")"
//! ```
//!
//! `sum` and `term` are **flat**: `1 - 2 - 3` is one `sum` with three `term`
//! children and two operator tokens, not a nested tree. Left associativity
//! is therefore a left fold over the children, done in a loop, so a long
//! operator chain costs no recursion at all.
//!
//! ## What each construct lowers to
//!
//! | Ruby                 | IIR (all `i64`)                               |
//! |----------------------|-----------------------------------------------|
//! | `42`, `1_000`, `+5`  | `rbN = const 42`                              |
//! | `-x`                 | `rbN = neg x`                                 |
//! | `a + b` / `a - b`    | `rbN = add a, b` / `sub`                      |
//! | `a * b`              | `rbN = mul a, b`                              |
//! | `a / b`              | `rbN = i64_div_floor a, b`  (LANG83)          |
//! | `a % b`              | `rbN = i64_mod_floor a, b`  (LANG83)          |
//! | `puts(a, b, …)`      | evaluate every argument, then `call_builtin print_i64` per value |
//! | `puts()`             | `call_builtin putchar 10`  (one newline)      |
//! | end of program       | `ret 0` (a script that falls off its end exits 0) |
//!
//! There is no Ruby-named builtin anywhere (LANG84 C2: `rb_int_div` *is*
//! `i64_div_floor`, `rb_puts_int` *is* `print_i64`), so every backend that
//! lowers LANG83 runs these programs with no Ruby-specific code.
//!
//! ## Why every value is tracked at compile time
//!
//! Ruby integers never overflow: past the machine word they become Bignums.
//! The IIR has `i64` only. The subset has no variables, so every value is
//! known while the compiler lowers it, and the compiler checks each literal
//! and each intermediate result against the `i64` range, in exact `i128`
//! arithmetic, before emitting anything. A program whose arithmetic would
//! leave `i64` is refused (`Unsupported`); one that stays inside computes the
//! same answer on every backend as it does in Ruby.
//!
//! A zero divisor is different: Ruby raises `ZeroDivisionError` *at run
//! time*, after the earlier statements have printed. So the compiler does
//! not refuse it. It emits the division anyway and LANG83's `i64_div_floor`
//! traps with `Trap.DivZero`, after flushing the output so far. From that
//! point the program's values are unknown (`known: None`): nothing after a
//! certain trap ever runs, so nothing after it needs a range check — but the
//! statements are still lowered and range-checked when their values are
//! known, which can only refuse more, never accept a wrong answer.

use interpreter_ir::{IIRInstr, Operand};
use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

use crate::error::FrontendError;
use crate::profiles::{IntDiv, ReleaseProfile};

/// LANG84 C5: most IIR instructions one module may hold.
pub const MAX_IIR_INSTRUCTIONS: usize = 1_000_000;

/// Every CST rule name the compiler dispatches on. The `cst_contract` test
/// (LANG84 C8) checks each one exists in every release that claims an IIR
/// subset, so a grammar rename cannot silently strand the compiler.
pub const DISPATCHED_RULES: &[&str] = &[
    "program",
    "terms",
    "statement",
    "method_call",
    "method_call_no_paren",
    "call_arg",
    "expression",
    "ternary",
    "range",
    "logical_or",
    "logical_and",
    "logical_not",
    "comparison",
    "shift",
    "sum",
    "term",
    "factor",
    "unary_minus",
];

/// Every effective token type the compiler dispatches on (LANG84 C1.5: the
/// type, never the spelling alone — a `STRING` token spelling `puts` is not a
/// callee).
pub const DISPATCHED_TOKENS: &[&str] = &[
    "NAME",
    "LPAREN",
    "RPAREN",
    "COMMA",
    "NEWLINE",
    "SEMICOLON",
    "NUMBER",
    "PLUS",
    "MINUS",
    "STAR",
    "SLASH",
    "PERCENT",
];

/// The single-child links between `expression` and `sum`, in order. Inside
/// rb-int-puts each of these rules has exactly one child, the next rule.
const LINKS: &[(&str, &str)] = &[
    ("expression", "ternary"),
    ("ternary", "range"),
    ("range", "logical_or"),
    ("logical_or", "logical_and"),
    ("logical_and", "logical_not"),
    ("logical_not", "comparison"),
    ("comparison", "shift"),
    ("shift", "sum"),
];

/// The newline `puts()` prints, as the byte `putchar` writes.
const NEWLINE_BYTE: i64 = b'\n' as i64;

/// A binary operator of the subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl BinOp {
    fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "%",
        }
    }
}

/// One evaluated expression: where its value lives, and the value itself
/// unless evaluation certainly traps first.
struct Evaluated {
    operand: Operand,
    known: Option<i128>,
}

/// The state of one compile.
pub(crate) struct Lowering<'a> {
    release: &'a str,
    profile: &'a ReleaseProfile,
    instructions: Vec<IIRInstr>,
    next_register: usize,
}

impl<'a> Lowering<'a> {
    pub(crate) fn new(release: &'a str, profile: &'a ReleaseProfile) -> Self {
        Self {
            release,
            profile,
            instructions: Vec::new(),
            next_register: 0,
        }
    }

    /// Lower a whole `program` node and return `main`'s instructions.
    pub(crate) fn lower_program(mut self, root: &GrammarASTNode) -> Result<Vec<IIRInstr>, FrontendError> {
        if root.rule_name != "program" {
            return Err(self.unsupported(root, "expected a Ruby program"));
        }
        for child in &root.children {
            match child {
                // Blank lines and stray `;` between statements are trivia.
                ASTNodeOrToken::Token(token) if is_terminator(token) => {}
                ASTNodeOrToken::Node(terms) if terms.rule_name == "terms" => {
                    if !terms
                        .children
                        .iter()
                        .all(|c| matches!(c, ASTNodeOrToken::Token(t) if is_terminator(t)))
                    {
                        return Err(self.unsupported(terms, "statement separators must be newlines or `;`"));
                    }
                }
                ASTNodeOrToken::Node(statement) if statement.rule_name == "statement" => {
                    self.lower_statement(statement)?;
                }
                ASTNodeOrToken::Node(other) => {
                    return Err(self.unsupported(other, "only `puts` statements are in rb-int-puts"));
                }
                ASTNodeOrToken::Token(token) => {
                    return Err(self.unsupported_token(root, token, "unexpected token between statements"));
                }
            }
        }
        // A script that falls off its end exits 0. Returning a real `i64`
        // (rather than `ret_void`) is what every AOT entry point expects:
        // native and LLVM turn it into the exit status, the CLR launcher and
        // BEAM read an integer back.
        let exit = self.register();
        self.emit("const", Some(exit.clone()), vec![Operand::Int(0)], "i64")?;
        self.emit("ret", None, vec![Operand::Var(exit)], "i64")?;
        Ok(self.instructions)
    }

    /// `statement = method_call | method_call_no_paren`.
    fn lower_statement(&mut self, statement: &GrammarASTNode) -> Result<(), FrontendError> {
        let [ASTNodeOrToken::Node(call)] = statement.children.as_slice() else {
            return Err(self.unsupported(statement, "a statement is exactly one call"));
        };
        let arguments = match call.rule_name.as_str() {
            "method_call" => self.parenthesized_arguments(call)?,
            "method_call_no_paren" => self.bare_arguments(call)?,
            _ => return Err(self.unsupported(call, "only `puts` statements are in rb-int-puts")),
        };
        // Ruby evaluates every argument before `puts` writes anything, so a
        // trap in the second argument prints nothing for this call.
        let mut values = Vec::with_capacity(arguments.len());
        for expression in arguments {
            values.push(self.lower_expression(expression)?);
        }
        if values.is_empty() {
            // `puts()` writes one newline and nothing else.
            let newline = self.register();
            self.emit("const", Some(newline.clone()), vec![Operand::Int(NEWLINE_BYTE)], "i64")?;
            self.emit(
                "call_builtin",
                None,
                vec![Operand::Var("putchar".into()), Operand::Var(newline)],
                "void",
            )?;
        }
        for value in values {
            // `puts n` writes `n.to_s` and a newline: LANG83 `print_i64`.
            self.emit(
                "call_builtin",
                None,
                vec![Operand::Var("print_i64".into()), value.operand],
                "void",
            )?;
        }
        Ok(())
    }

    /// `method_call = NAME LPAREN {NEWLINE} [call_arg {COMMA {NEWLINE} call_arg} {NEWLINE}] RPAREN`.
    ///
    /// Newlines are trivia here; what is left must be exactly
    /// `puts ( [arg {, arg}] )`.
    fn parenthesized_arguments<'n>(&self, call: &'n GrammarASTNode) -> Result<Vec<&'n GrammarASTNode>, FrontendError> {
        let parts = significant(call);
        let (Some(first), Some(last)) = (parts.first(), parts.last()) else {
            return Err(self.unsupported(call, "empty call"));
        };
        self.expect_puts(call, first)?;
        if parts.len() < 3 || !is_token(parts[1], "LPAREN") || !is_token(last, "RPAREN") {
            return Err(self.unsupported(call, "expected `puts(` … `)`"));
        }
        let mut arguments = Vec::new();
        for (index, part) in parts[2..parts.len() - 1].iter().enumerate() {
            if index % 2 == 1 {
                if !is_token(part, "COMMA") {
                    return Err(self.unsupported(call, "arguments are separated by `,`"));
                }
                continue;
            }
            let ASTNodeOrToken::Node(argument) = part else {
                return Err(self.unsupported(call, "expected an argument"));
            };
            if argument.rule_name != "call_arg" {
                return Err(self.unsupported(argument, "expected a positional argument"));
            }
            // `call_arg = expression`: anything else (a splat, a keyword, a
            // block pass) is a different shape and is refused here.
            arguments.push(self.only_child(argument, "expression")?);
        }
        if parts.len() > 3 && parts.len() % 2 == 1 {
            // An even number of middle items ends in a comma: `puts(1,)`.
            return Err(self.unsupported(call, "trailing `,` in a call"));
        }
        Ok(arguments)
    }

    /// `method_call_no_paren = NAME !MINUS expression {COMMA {NEWLINE} expression}`.
    fn bare_arguments<'n>(&self, call: &'n GrammarASTNode) -> Result<Vec<&'n GrammarASTNode>, FrontendError> {
        let parts = significant(call);
        let Some(first) = parts.first() else {
            return Err(self.unsupported(call, "empty call"));
        };
        self.expect_puts(call, first)?;
        if parts.len() < 2 || parts.len() % 2 == 1 {
            return Err(self.unsupported(call, "expected `puts expr {, expr}`"));
        }
        let mut arguments = Vec::new();
        for (index, part) in parts[1..].iter().enumerate() {
            if index % 2 == 1 {
                if !is_token(part, "COMMA") {
                    return Err(self.unsupported(call, "arguments are separated by `,`"));
                }
                continue;
            }
            match part {
                ASTNodeOrToken::Node(expression) if expression.rule_name == "expression" => {
                    arguments.push(expression);
                }
                _ => return Err(self.unsupported(call, "expected an argument expression")),
            }
        }
        Ok(arguments)
    }

    /// The callee must be the name `puts` — by token type, then by spelling.
    fn expect_puts(&self, call: &GrammarASTNode, callee: &ASTNodeOrToken) -> Result<(), FrontendError> {
        match callee {
            ASTNodeOrToken::Token(token) if token.effective_type_name() == "NAME" && token.value == "puts" => Ok(()),
            ASTNodeOrToken::Token(token) if token.effective_type_name() == "NAME" => Err(self.unsupported_token(
                call,
                token,
                format!("only `puts` is in rb-int-puts, not {}", shown(&token.value)),
            )),
            _ => Err(self.unsupported(call, "the callee must be a name")),
        }
    }

    /// `expression → ternary → … → shift → sum`, then the arithmetic.
    fn lower_expression(&mut self, expression: &GrammarASTNode) -> Result<Evaluated, FrontendError> {
        if expression.rule_name != "expression" {
            return Err(self.unsupported(expression, "expected an expression"));
        }
        let mut node = expression;
        for (from, to) in LINKS {
            debug_assert_eq!(node.rule_name, *from);
            node = self.only_child(node, to)?;
        }
        self.lower_chain(node)
    }

    /// The flat, left-associative chains:
    ///
    /// ```text
    /// sum  = term   { ( PLUS | MINUS )          {NEWLINE} term   } ;
    /// term = factor { ( STAR | SLASH | PERCENT ) {NEWLINE} factor } ;
    /// ```
    ///
    /// `a - b - c` folds as `(a - b) - c`: the accumulator starts at the
    /// first operand and each `(operator, operand)` pair is applied to it.
    fn lower_chain(&mut self, chain: &GrammarASTNode) -> Result<Evaluated, FrontendError> {
        let (operand_rule, operators): (&str, &[(&str, BinOp)]) = match chain.rule_name.as_str() {
            "sum" => ("term", &[("PLUS", BinOp::Add), ("MINUS", BinOp::Sub)]),
            "term" => (
                "factor",
                &[("STAR", BinOp::Mul), ("SLASH", BinOp::Div), ("PERCENT", BinOp::Mod)],
            ),
            _ => return Err(self.unsupported(chain, "expected an arithmetic expression")),
        };
        let parts = significant(chain);
        if parts.len().is_multiple_of(2) {
            return Err(self.unsupported(chain, "an operator is missing its right operand"));
        }
        let mut accumulator = self.lower_operand(chain, parts[0], operand_rule)?;
        for pair in parts[1..].as_chunks::<2>().0 {
            let op = match pair[0] {
                ASTNodeOrToken::Token(token) => operators
                    .iter()
                    .find(|(name, _)| token.effective_type_name() == *name)
                    .map(|(_, op)| *op)
                    .ok_or_else(|| self.unsupported_token(chain, token, "not an operator of rb-int-puts"))?,
                ASTNodeOrToken::Node(node) => return Err(self.unsupported(node, "expected an operator")),
            };
            let right = self.lower_operand(chain, pair[1], operand_rule)?;
            accumulator = self.binary(chain, op, accumulator, right)?;
        }
        Ok(accumulator)
    }

    /// One operand of a chain: a `term` of a `sum`, a `factor` of a `term`.
    fn lower_operand(
        &mut self,
        chain: &GrammarASTNode,
        part: &ASTNodeOrToken,
        rule: &str,
    ) -> Result<Evaluated, FrontendError> {
        match part {
            ASTNodeOrToken::Node(node) if node.rule_name == rule && rule == "term" => self.lower_chain(node),
            ASTNodeOrToken::Node(node) if node.rule_name == rule => self.lower_factor(node),
            _ => Err(self.unsupported(chain, format!("expected a {rule}"))),
        }
    }

    /// `factor = NUMBER | LPAREN {NEWLINE} expression {NEWLINE} RPAREN | unary_minus`.
    fn lower_factor(&mut self, factor: &GrammarASTNode) -> Result<Evaluated, FrontendError> {
        match significant(factor).as_slice() {
            [ASTNodeOrToken::Token(number)] if number.effective_type_name() == "NUMBER" => {
                let value = self.decode_integer(factor, number)?;
                let register = self.register();
                self.emit("const", Some(register.clone()), vec![Operand::Int(value)], "i64")?;
                Ok(Evaluated {
                    operand: Operand::Var(register),
                    known: Some(i128::from(value)),
                })
            }
            [ASTNodeOrToken::Node(unary)] if unary.rule_name == "unary_minus" => self.lower_unary_minus(unary),
            [open, ASTNodeOrToken::Node(inner), close] if is_token(open, "LPAREN") && is_token(close, "RPAREN") => {
                self.lower_expression(inner)
            }
            _ => Err(self.unsupported(factor, "expected an integer, `(expr)` or `-factor`")),
        }
    }

    /// `unary_minus = MINUS {NEWLINE} factor`.
    fn lower_unary_minus(&mut self, unary: &GrammarASTNode) -> Result<Evaluated, FrontendError> {
        let parts = significant(unary);
        let [minus, ASTNodeOrToken::Node(factor)] = parts.as_slice() else {
            return Err(self.unsupported(unary, "expected `-` and one operand"));
        };
        if !is_token(minus, "MINUS") || factor.rule_name != "factor" {
            return Err(self.unsupported(unary, "expected `-` and one operand"));
        }
        let inner = self.lower_factor(factor)?;
        let known = inner.known.map(|value| -value);
        self.check_range(unary, known, || "negation".into())?;
        let register = self.register();
        self.emit("neg", Some(register.clone()), vec![inner.operand], "i64")?;
        Ok(Evaluated {
            operand: Operand::Var(register),
            known,
        })
    }

    /// Emit one binary operator, after checking its compile-time value.
    fn binary(
        &mut self,
        chain: &GrammarASTNode,
        op: BinOp,
        left: Evaluated,
        right: Evaluated,
    ) -> Result<Evaluated, FrontendError> {
        let iir_op = match op {
            BinOp::Add => "add",
            BinOp::Sub => "sub",
            BinOp::Mul => "mul",
            BinOp::Div | BinOp::Mod => match self.profile.int_div {
                IntDiv::Floor if op == BinOp::Div => "i64_div_floor",
                IntDiv::Floor => "i64_mod_floor",
                IntDiv::Truncate => {
                    return Err(self.unsupported(
                        chain,
                        format!(
                            "integer `{}` truncates in this release's profile ({}); LANG83 \
                             `i64_div_trunc` is not lowered by any backend yet",
                            op.symbol(),
                            self.profile.name
                        ),
                    ))
                }
            },
        };
        let known = match (left.known, right.known) {
            (Some(a), Some(b)) => fold(op, a, b),
            // An operand certainly traps first; this operator never runs.
            _ => None,
        };
        self.check_range(chain, known, || {
            format!(
                "`{} {} {}`",
                show_known(left.known),
                op.symbol(),
                show_known(right.known)
            )
        })?;
        let register = self.register();
        // `i64::MIN % -1` is the one modulus a machine `rem` cannot compute:
        // x86 `idiv` raises SIGFPE, LLVM `srem` is undefined, the CLR throws,
        // and Rust's `%` panics. LANG83 defines `i64_mod_floor` there as 0 and
        // every PR1b backend guards it, but the compiler already knows the
        // answer (Ruby's, too: 0), so it emits that constant and no backend
        // ever sees the edge (security review, LANG84 5a). The operands were
        // evaluated above, so nothing observable is skipped.
        if op == BinOp::Mod && (left.known, right.known) == (Some(i128::from(i64::MIN)), Some(-1)) {
            self.emit("const", Some(register.clone()), vec![Operand::Int(0)], "i64")?;
        } else {
            self.emit(iir_op, Some(register.clone()), vec![left.operand, right.operand], "i64")?;
        }
        Ok(Evaluated {
            operand: Operand::Var(register),
            known,
        })
    }

    /// Refuse a compile-time value that leaves `i64` (it would be a Bignum).
    fn check_range(
        &self,
        node: &GrammarASTNode,
        known: Option<i128>,
        what: impl FnOnce() -> String,
    ) -> Result<(), FrontendError> {
        match known {
            Some(value) if i64::try_from(value).is_err() => Err(self.unsupported(
                node,
                format!(
                    "{} is {value}, outside i64; Ruby would make it a Bignum, which \
                     rb-int-puts does not have",
                    what()
                ),
            )),
            _ => Ok(()),
        }
    }

    /// Decode one `NUMBER` token's value (LANG84 C1.4 allows decoding a
    /// single token; it never splits one).
    ///
    /// The R0 tokens spell a decimal integer as digits with single `_`
    /// separators, optionally preceded by the sign the lexer folds into the
    /// literal (`+5` after an operator in 3.3; `-7` in the 1994 lexers):
    ///
    /// | token value   | value   | why                                   |
    /// |---------------|---------|---------------------------------------|
    /// | `42`          | 42      |                                       |
    /// | `1_000`       | 1000    | `_` separates digit groups            |
    /// | `+5`          | 5       | 3.3 folds a unary `+` into the number |
    /// | `010`         | refused | Ruby reads a leading `0` as octal     |
    /// | `1__0`, `_1`  | refused | not a Ruby integer                    |
    fn decode_integer(&self, factor: &GrammarASTNode, token: &Token) -> Result<i64, FrontendError> {
        let text = token.value.as_str();
        let (negative, digits) = match text.as_bytes().first() {
            Some(b'+') => (false, &text[1..]),
            Some(b'-') => (true, &text[1..]),
            _ => (false, text),
        };
        let well_formed = !digits.is_empty()
            && digits.split('_').all(|group| !group.is_empty() && group.bytes().all(|b| b.is_ascii_digit()));
        if !well_formed {
            return Err(self.unsupported_token(factor, token, format!("{} is not a decimal integer", shown(text))));
        }
        let plain: String = digits.chars().filter(|c| *c != '_').collect();
        if plain.len() > 1 && plain.starts_with('0') {
            return Err(self.unsupported_token(factor, token, "a leading `0` makes an octal literal, outside rb-int-puts"));
        }
        // 19 digits always fit in i128; more than 19 never fit in i64.
        let magnitude = if plain.len() > 19 { None } else { plain.parse::<i128>().ok() };
        let value = magnitude.map(|m| if negative { -m } else { m }).and_then(|v| i64::try_from(v).ok());
        value.ok_or_else(|| {
            self.unsupported_token(
                factor,
                token,
                format!(
                    "integer literal {} is outside i64; Ruby would make it a Bignum, which \
                     rb-int-puts does not have",
                    shown(text)
                ),
            )
        })
    }

    /// The single child of `parent`, which must be a node named `rule`.
    fn only_child<'n>(&self, parent: &'n GrammarASTNode, rule: &str) -> Result<&'n GrammarASTNode, FrontendError> {
        match parent.children.as_slice() {
            [ASTNodeOrToken::Node(child)] if child.rule_name == rule => Ok(child),
            _ => Err(self.unsupported(parent, format!("expected exactly one {rule}"))),
        }
    }

    fn register(&mut self) -> String {
        let name = format!("rb{}", self.next_register);
        self.next_register += 1;
        name
    }

    fn emit(&mut self, op: &str, dest: Option<String>, srcs: Vec<Operand>, type_hint: &str) -> Result<(), FrontendError> {
        if self.instructions.len() >= MAX_IIR_INSTRUCTIONS {
            return Err(FrontendError::Budget {
                release: self.release.to_string(),
                which: "IIR instructions",
                limit: MAX_IIR_INSTRUCTIONS,
            });
        }
        self.instructions.push(IIRInstr::new(op, dest, srcs, type_hint));
        Ok(())
    }

    fn unsupported(&self, node: &GrammarASTNode, message: impl Into<String>) -> FrontendError {
        FrontendError::Unsupported {
            release: self.release.to_string(),
            rule: shown_rule(&node.rule_name),
            start: node.start_line.zip(node.start_column),
            end: node.end_line.zip(node.end_column),
            message: message.into(),
        }
    }

    fn unsupported_token(&self, node: &GrammarASTNode, token: &Token, message: impl Into<String>) -> FrontendError {
        FrontendError::Unsupported {
            release: self.release.to_string(),
            rule: shown_rule(&node.rule_name),
            start: Some((token.line, token.column)),
            end: None,
            message: message.into(),
        }
    }
}

/// The compile-time value of `a op b`, or `None` when it traps.
///
/// `i128` holds every product of two `i64`s exactly, so nothing here can
/// overflow; the caller range-checks the result. Floor division rounds the
/// truncated quotient down when the remainder is non-zero and its sign
/// differs from the divisor's — the same fix-up LANG83 specifies for
/// `i64_div_floor`.
fn fold(op: BinOp, a: i128, b: i128) -> Option<i128> {
    match op {
        BinOp::Add => Some(a + b),
        BinOp::Sub => Some(a - b),
        BinOp::Mul => Some(a * b),
        BinOp::Div | BinOp::Mod if b == 0 => None,
        BinOp::Div => {
            let (q, r) = (a / b, a % b);
            Some(if r != 0 && (r < 0) != (b < 0) { q - 1 } else { q })
        }
        BinOp::Mod => {
            let r = a % b;
            Some(if r != 0 && (r < 0) != (b < 0) { r + b } else { r })
        }
    }
}

fn show_known(value: Option<i128>) -> String {
    value.map_or_else(|| "…".into(), |v| v.to_string())
}

/// The children of `node` minus `NEWLINE` tokens. Inside the subset a
/// newline is only ever trivia (the grammar admits it exactly where Ruby's
/// lexer would be in EXPR_BEG and ignore it), so the shape checks run on
/// what is left.
fn significant(node: &GrammarASTNode) -> Vec<&ASTNodeOrToken> {
    node.children
        .iter()
        .filter(|child| !matches!(child, ASTNodeOrToken::Token(t) if t.effective_type_name() == "NEWLINE"))
        .collect()
}

fn is_token(child: &ASTNodeOrToken, type_name: &str) -> bool {
    matches!(child, ASTNodeOrToken::Token(token) if token.effective_type_name() == type_name)
}

fn is_terminator(token: &Token) -> bool {
    matches!(token.effective_type_name(), "NEWLINE" | "SEMICOLON")
}

/// Caller text inside a diagnostic: at most 32 characters, control
/// characters escaped (a direct-AST caller can put anything in a token).
fn shown(text: &str) -> String {
    let mut chars = text.chars();
    let head: String = chars.by_ref().take(32).flat_map(char::escape_debug).collect();
    if chars.next().is_some() {
        format!("`{head}…`")
    } else {
        format!("`{head}`")
    }
}

/// A rule name inside a diagnostic, bounded the same way.
pub(crate) fn shown_rule(rule: &str) -> String {
    let mut chars = rule.chars();
    let head: String = chars.by_ref().take(64).flat_map(char::escape_debug).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_matches_rubys_floor_division_sign_table() {
        // (a, b, a / b, a % b) from host ruby 3.3.6.
        let table = [
            (7, 2, 3, 1),
            (-7, 2, -4, 1),
            (7, -2, -4, -1),
            (-7, -2, 3, -1),
            (6, 3, 2, 0),
            (-6, 3, -2, 0),
            (0, -5, 0, 0),
        ];
        for (a, b, q, r) in table {
            assert_eq!(fold(BinOp::Div, a, b), Some(q), "{a} / {b}");
            assert_eq!(fold(BinOp::Mod, a, b), Some(r), "{a} % {b}");
        }
        assert_eq!(fold(BinOp::Div, 1, 0), None);
        assert_eq!(fold(BinOp::Mod, 1, 0), None);
        let min = i128::from(i64::MIN);
        assert_eq!(fold(BinOp::Div, min, -1), Some(-min));
        assert_eq!(fold(BinOp::Mod, min, -1), Some(0));
        assert_eq!(fold(BinOp::Mul, min, min), Some(min * min));
    }

    #[test]
    fn diagnostics_bound_and_escape_caller_text() {
        assert_eq!(shown("puts"), "`puts`");
        assert_eq!(shown("a\nb"), "`a\\nb`");
        let long = shown(&"x".repeat(1000));
        assert!(long.len() < 40 && long.ends_with("…`"), "{long}");
        assert!(shown_rule(&"r".repeat(1000)).chars().count() == 65);
        assert_eq!(show_known(None), "…");
    }
}
