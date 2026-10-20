//! # Walking the `py-int-print` parse tree into IIR (LANG84 phase 7a)
//!
//! This is the hand-written half of the LANG84 pipeline. Everything before
//! it is generic and table-driven (`GrammarLexer` over
//! `python<release>.tokens`, `GrammarParser` over `python<release>.grammar`);
//! everything after it is a LANG83 backend. The input is the
//! `GrammarASTNode` the parser built, read in place: there is no owned Python
//! AST, no re-scanning of source text, and no second parser (LANG84 C1, C2).
//!
//! ## The tree, for `print(1 + 2 * 3)`
//!
//! The release grammars keep `python3.12.grammar`'s rule names (LANG84 C8):
//!
//! ```text
//! file
//! ├─ statement
//! │  └─ simple_stmt
//! │     ├─ small_stmt ─ expression ─ arith ─ term ─ factor ─ primary
//! │     │                                                   ├─ atom ─ NAME "print"
//! │     │                                                   └─ suffix
//! │     │                                                      ├─ LPAREN "("
//! │     │                                                      ├─ arguments ─ argument ─ expression
//! │     │                                                      │     └─ arith
//! │     │                                                      │        ├─ term ─ factor ─ primary ─ atom ─ INT "1"
//! │     │                                                      │        ├─ PLUS "+"
//! │     │                                                      │        └─ term
//! │     │                                                      │           ├─ factor … INT "2"
//! │     │                                                      │           ├─ STAR "*"
//! │     │                                                      │           └─ factor … INT "3"
//! │     │                                                      └─ RPAREN ")"
//! │     └─ NEWLINE
//! └─ (more statements, or NEWLINE trivia)
//! ```
//!
//! `arith` and `term` are **flat**: `1 - 2 - 3` is one `arith` with three
//! `term` children and two operator tokens, not a nested tree. Left
//! associativity is therefore a left fold over the children, done in a loop,
//! so a long operator chain costs no recursion at all. A chain of unary signs
//! (`- - - 1`) is also walked in a loop.
//!
//! ## What each construct lowers to
//!
//! | Python                | IIR (all `i64`)                                   |
//! |-----------------------|---------------------------------------------------|
//! | `42`, `1_000`, `00`   | `pyN = const 42`                                  |
//! | `a + b`, `a - b`      | `pyN = i64_add_ovf a, b` / `i64_sub_ovf`          |
//! | `a * b`               | `pyN = i64_mul_ovf a, b`                          |
//! | `-a`                  | `pyN = i64_neg_ovf a`                             |
//! | `+a`                  | nothing: `int.__pos__` is the identity            |
//! | `a // b`, `a % b`     | `i64_div_floor` / `i64_mod_floor`                 |
//! | `a // -1`, `a % -1`   | `i64_neg_ovf a`, `const 0` (divisor known `-1`)   |
//! | `print(a)`            | `call_builtin print_i64, a`                       |
//! | `print()`             | `call_builtin putchar, 10` (one newline)          |
//! | end of module         | `const 0`, `ret` (a script that ends exits 0)     |
//!
//! There is no Python-named builtin anywhere (LANG84 C2), so every backend
//! that lowers LANG83 runs these programs with no Python-specific code.
//!
//! ## Integers that leave `i64`
//!
//! Python's `int` never overflows; the IIR has `i64` only. Every `+ - *` and
//! unary `-` is therefore a LANG83 PR4 *checked* op: where CPython would
//! produce an `int` outside `[-2^63, 2^63 - 1]`, the program stops with
//! `Trap.Overflow` after the output so far. That is a stated divergence from
//! CPython (bignum promotion on that trap is later work); it is never a
//! wrapped, wrong number. A *literal* outside `i64` cannot even be written
//! into a `const`, so it is refused at compile time (`Unsupported`).
//!
//! A zero divisor stops the program with `Trap.DivZero` after the output so
//! far, as CPython's `ZeroDivisionError` does. The compiler does not refuse
//! it: the op is emitted and traps at run time.
//!
//! ## The `-1` divisor
//!
//! `a // -1 == -a` and `a % -1 == 0` for every integer `a`. When the divisor
//! is known to be `-1`, the compiler emits those forms instead of a divide.
//! That keeps `i64::MIN // -1` and `i64::MIN % -1`, the two operand pairs a
//! machine divide faults on (x86 `idiv` SIGFPE, LLVM `sdiv`/`srem` UB, a CLR
//! exception, a Rust panic), out of every backend: the first becomes
//! `i64_neg_ovf i64::MIN`, which traps `Trap.Overflow` (CPython's answer,
//! 2^63, is outside `i64`), and the second becomes `0`, CPython's answer.
//! LANG83 defines both edges and every backend guards them; this keeps them
//! from being exercised at all (Ruby 5a's security review made the same
//! call).
//!
//! ## Why values are tracked at compile time
//!
//! The subset has no variables, so the compiler knows every value while it
//! lowers. It keeps that value (exactly, in `i128`) only to recognise the
//! `-1` divisor; every operation is still emitted and checked at run time.
//! A value whose operation certainly traps is `None`: nothing after a certain
//! trap ever runs.

use interpreter_ir::{IIRInstr, Operand};
use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

use crate::frontend_error::FrontendError;
use crate::profiles::{ReleaseProfile, Slash};

/// LANG84 C5: most IIR instructions one module may hold.
pub const MAX_IIR_INSTRUCTIONS: usize = 1_000_000;

/// Every CST rule name the compiler dispatches on. The `cst_contract` test
/// (LANG84 C8) checks each one exists in every release that claims an IIR
/// subset, so a grammar rename cannot silently strand the compiler.
pub const DISPATCHED_RULES: &[&str] = &[
    "file",
    "statement",
    "simple_stmt",
    "small_stmt",
    "expression",
    "arith",
    "term",
    "factor",
    "primary",
    "suffix",
    "arguments",
    "argument",
    "atom",
    "paren_expr",
    "paren_body",
];

/// Every effective token type the compiler dispatches on (LANG84 C1.5: the
/// type, never the spelling alone; a `STRING` token spelling `print` is not a
/// callee).
pub const DISPATCHED_TOKENS: &[&str] = &[
    "NEWLINE",
    "SEMICOLON",
    "INT",
    "NAME",
    "LPAREN",
    "RPAREN",
    "COMMA",
    "PLUS",
    "MINUS",
    "STAR",
    "SLASH",
    "FLOOR_DIV",
    "PERCENT",
];

/// The newline `print()` writes, as the byte `putchar` takes.
const NEWLINE_BYTE: i64 = b'\n' as i64;

/// A binary operator of the subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinOp {
    Add,
    Sub,
    Mul,
    FloorDiv,
    Mod,
}

impl BinOp {
    /// The LANG83 op that computes it (overflow- and zero-checked).
    fn iir_op(self) -> &'static str {
        match self {
            BinOp::Add => "i64_add_ovf",
            BinOp::Sub => "i64_sub_ovf",
            BinOp::Mul => "i64_mul_ovf",
            BinOp::FloorDiv => "i64_div_floor",
            BinOp::Mod => "i64_mod_floor",
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

    /// Lower a whole `file` node and return `main`'s instructions.
    ///
    /// `file = { NEWLINE | statement }`: a NEWLINE child is a blank line the
    /// lexer reported (trivia); anything else must be a statement.
    pub(crate) fn lower_file(mut self, root: &GrammarASTNode) -> Result<Vec<IIRInstr>, FrontendError> {
        if root.rule_name != "file" {
            return Err(self.unsupported(root, "expected a Python file"));
        }
        for child in &root.children {
            match child {
                ASTNodeOrToken::Token(token) if is_newline(token) => {}
                ASTNodeOrToken::Node(statement) if statement.rule_name == "statement" => {
                    self.lower_statement(statement)?;
                }
                ASTNodeOrToken::Node(other) => {
                    return Err(self.unsupported(other, "expected a statement"));
                }
                ASTNodeOrToken::Token(token) => {
                    return Err(self.unsupported_token(root, token, "unexpected token between statements"));
                }
            }
        }
        // A module that runs off its end exits 0. Returning a real `i64`
        // (not `ret_void`) is what every AOT entry point expects: native and
        // LLVM turn it into the exit status, the CLR launcher and BEAM read
        // an integer back (LANG84 phase 5a/4a lesson).
        let exit = self.register();
        self.emit("const", Some(exit.clone()), vec![Operand::Int(0)], "i64")?;
        self.emit("ret", None, vec![Operand::Var(exit)], "i64")?;
        Ok(self.instructions)
    }

    /// `statement = simple_stmt` and
    /// `simple_stmt = small_stmt { SEMICOLON small_stmt } [ SEMICOLON ] NEWLINE`.
    fn lower_statement(&mut self, statement: &GrammarASTNode) -> Result<(), FrontendError> {
        let simple = self.only_child(statement, "simple_stmt")?;
        let mut ended = false;
        for child in &simple.children {
            if ended {
                return Err(self.unsupported(simple, "nothing may follow a statement's NEWLINE"));
            }
            match child {
                ASTNodeOrToken::Node(small) if small.rule_name == "small_stmt" => {
                    let expression = self.only_child(small, "expression")?;
                    self.lower_print_statement(small, expression)?;
                }
                ASTNodeOrToken::Token(token) if token.effective_type_name() == "SEMICOLON" => {}
                ASTNodeOrToken::Token(token) if is_newline(token) => ended = true,
                ASTNodeOrToken::Token(token) => {
                    return Err(self.unsupported_token(simple, token, "unexpected token in a statement"));
                }
                ASTNodeOrToken::Node(other) => {
                    return Err(self.unsupported(other, "expected a simple statement"));
                }
            }
        }
        Ok(())
    }

    /// The only statement in `py-int-print`: `print(<expr>)` or `print()`.
    fn lower_print_statement(
        &mut self,
        small: &GrammarASTNode,
        expression: &GrammarASTNode,
    ) -> Result<(), FrontendError> {
        let Some((callee, suffix)) = self.as_call(expression)? else {
            return Err(self.unsupported(
                small,
                "only `print(...)` statements are in py-int-print; a bare expression statement is not",
            ));
        };
        if callee.value != "print" {
            return Err(self.unsupported_token(
                small,
                callee,
                format!("only `print` is in py-int-print, not {}", shown(&callee.value)),
            ));
        }
        match self.call_arguments(suffix)?.as_slice() {
            [] => {
                // `print()` writes one newline and nothing else.
                let newline = self.register();
                self.emit("const", Some(newline.clone()), vec![Operand::Int(NEWLINE_BYTE)], "i64")?;
                self.emit(
                    "call_builtin",
                    None,
                    vec![Operand::Var("putchar".into()), Operand::Var(newline)],
                    "void",
                )
            }
            [argument] => {
                // The argument is evaluated completely before `print`
                // writes, so a trap inside it prints nothing for this call.
                let value = self.lower_expression(argument)?;
                // `print(n)` writes `str(n)` and `\n`: LANG83 `print_i64`.
                self.emit(
                    "call_builtin",
                    None,
                    vec![Operand::Var("print_i64".into()), value.operand],
                    "void",
                )
            }
            _ => Err(self.unsupported(
                suffix,
                "py-int-print's `print` takes at most one argument (more would need a separator)",
            )),
        }
    }

    /// If `expression` is exactly `NAME ( ... )`, its callee token and the
    /// call `suffix`; `None` for any other expression.
    ///
    /// `expression → arith → term → factor → primary`, each a single child,
    /// then `primary = atom suffix` with `atom = NAME`.
    fn as_call<'n>(
        &self,
        expression: &'n GrammarASTNode,
    ) -> Result<Option<(&'n Token, &'n GrammarASTNode)>, FrontendError> {
        let mut node = expression;
        for rule in ["arith", "term", "factor", "primary"] {
            match node.children.as_slice() {
                [ASTNodeOrToken::Node(child)] if child.rule_name == rule => node = child,
                _ => return Ok(None),
            }
        }
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(atom), ASTNodeOrToken::Node(suffix)]
                if atom.rule_name == "atom" && suffix.rule_name == "suffix" =>
            {
                match atom.children.as_slice() {
                    [ASTNodeOrToken::Token(name)] if name.effective_type_name() == "NAME" => Ok(Some((name, suffix))),
                    _ => Err(self.unsupported(atom, "the callee must be a name")),
                }
            }
            [ASTNodeOrToken::Node(atom), ASTNodeOrToken::Node(_), ..] if atom.rule_name == "atom" => {
                Err(self.unsupported(node, "a call of a call's result is not in py-int-print"))
            }
            _ => Ok(None),
        }
    }

    /// `suffix = LPAREN [ arguments ] RPAREN` and
    /// `arguments = argument { COMMA argument } [ COMMA ]`, as the list of
    /// argument expressions. A trailing comma is allowed, as in Python.
    fn call_arguments<'n>(&self, suffix: &'n GrammarASTNode) -> Result<Vec<&'n GrammarASTNode>, FrontendError> {
        let parts = suffix.children.as_slice();
        let (Some(open), Some(close)) = (parts.first(), parts.last()) else {
            return Err(self.unsupported(suffix, "expected `(` … `)`"));
        };
        if parts.len() < 2 || !is_token(open, "LPAREN") || !is_token(close, "RPAREN") {
            return Err(self.unsupported(suffix, "expected `(` … `)`"));
        }
        let arguments = match &parts[1..parts.len() - 1] {
            [] => return Ok(Vec::new()),
            [ASTNodeOrToken::Node(arguments)] if arguments.rule_name == "arguments" => arguments,
            _ => return Err(self.unsupported(suffix, "expected the call's arguments")),
        };
        let mut expressions = Vec::new();
        let mut expect_argument = true;
        for part in &arguments.children {
            match part {
                ASTNodeOrToken::Node(argument) if expect_argument && argument.rule_name == "argument" => {
                    // `argument = expression`: a keyword or starred argument
                    // is a different shape and is refused here.
                    expressions.push(self.only_child(argument, "expression")?);
                    expect_argument = false;
                }
                ASTNodeOrToken::Token(comma) if !expect_argument && comma.effective_type_name() == "COMMA" => {
                    expect_argument = true;
                }
                _ => return Err(self.unsupported(arguments, "arguments are expressions separated by `,`")),
            }
        }
        if expressions.is_empty() {
            return Err(self.unsupported(arguments, "expected an argument"));
        }
        Ok(expressions)
    }

    /// `expression = arith`, then the arithmetic.
    fn lower_expression(&mut self, expression: &GrammarASTNode) -> Result<Evaluated, FrontendError> {
        if expression.rule_name != "expression" {
            return Err(self.unsupported(expression, "expected an expression"));
        }
        let arith = self.only_child(expression, "arith")?;
        self.lower_chain(arith)
    }

    /// The flat, left-associative chains:
    ///
    /// ```text
    /// arith = term   { ( PLUS | MINUS ) term } ;
    /// term  = factor { ( STAR | SLASH | FLOOR_DIV | PERCENT ) factor } ;
    /// ```
    ///
    /// `a - b - c` folds as `(a - b) - c`: the accumulator starts at the
    /// first operand and each `(operator, operand)` pair is applied to it.
    fn lower_chain(&mut self, chain: &GrammarASTNode) -> Result<Evaluated, FrontendError> {
        let operand_rule = match chain.rule_name.as_str() {
            "arith" => "term",
            "term" => "factor",
            _ => return Err(self.unsupported(chain, "expected an arithmetic expression")),
        };
        let parts = chain.children.as_slice();
        if parts.len().is_multiple_of(2) {
            return Err(self.unsupported(chain, "an operator is missing an operand"));
        }
        let mut accumulator = self.lower_operand(chain, &parts[0], operand_rule)?;
        for pair in parts[1..].as_chunks::<2>().0 {
            let op = match &pair[0] {
                ASTNodeOrToken::Token(token) => self.binary_operator(chain, token)?,
                ASTNodeOrToken::Node(node) => return Err(self.unsupported(node, "expected an operator")),
            };
            if (operand_rule == "term") != matches!(op, BinOp::Add | BinOp::Sub) {
                return Err(self.unsupported(chain, "operator at the wrong precedence level"));
            }
            let right = self.lower_operand(chain, &pair[1], operand_rule)?;
            accumulator = self.binary(op, accumulator, right)?;
        }
        Ok(accumulator)
    }

    /// The `BinOp` of an operator token, by effective type.
    fn binary_operator(&self, chain: &GrammarASTNode, token: &Token) -> Result<BinOp, FrontendError> {
        Ok(match token.effective_type_name() {
            "PLUS" => BinOp::Add,
            "MINUS" => BinOp::Sub,
            "STAR" => BinOp::Mul,
            "FLOOR_DIV" => BinOp::FloorDiv,
            "PERCENT" => BinOp::Mod,
            "SLASH" => {
                return Err(match self.profile.slash {
                    Slash::True => self.unsupported_token(
                        chain,
                        token,
                        format!(
                            "`/` is true division in this release's profile ({}) and gives a \
                             float, which py-int-print does not have; use `//`",
                            self.profile.name
                        ),
                    ),
                    Slash::Classic => self.unsupported_token(
                        chain,
                        token,
                        "classic `/` division has no release pair yet (Python 2 is a later slice)",
                    ),
                })
            }
            _ => return Err(self.unsupported_token(chain, token, "not an operator of py-int-print")),
        })
    }

    /// One operand of a chain: a `term` of an `arith`, a `factor` of a `term`.
    fn lower_operand(&mut self, chain: &GrammarASTNode, part: &ASTNodeOrToken, rule: &str) -> Result<Evaluated, FrontendError> {
        match part {
            ASTNodeOrToken::Node(node) if node.rule_name == rule && rule == "term" => self.lower_chain(node),
            ASTNodeOrToken::Node(node) if node.rule_name == rule => self.lower_factor(node),
            _ => Err(self.unsupported(chain, format!("expected a {rule}"))),
        }
    }

    /// `factor = ( PLUS | MINUS ) factor | primary`.
    ///
    /// A run of signs is collected in a loop (no recursion per sign), the
    /// primary is lowered, and the signs are applied innermost first, as
    /// Python evaluates them.
    fn lower_factor(&mut self, factor: &GrammarASTNode) -> Result<Evaluated, FrontendError> {
        let mut signs: Vec<&GrammarASTNode> = Vec::new();
        let mut node = factor;
        let primary = loop {
            match node.children.as_slice() {
                [ASTNodeOrToken::Token(sign), ASTNodeOrToken::Node(inner)]
                    if inner.rule_name == "factor" && matches!(sign.effective_type_name(), "PLUS" | "MINUS") =>
                {
                    signs.push(node);
                    node = inner;
                }
                [ASTNodeOrToken::Node(primary)] if primary.rule_name == "primary" => break primary,
                _ => return Err(self.unsupported(node, "expected a signed operand or a primary")),
            }
        };
        let mut value = self.lower_primary(primary)?;
        for signed in signs.into_iter().rev() {
            let negate = matches!(
                signed.children.first(),
                Some(ASTNodeOrToken::Token(sign)) if sign.effective_type_name() == "MINUS"
            );
            if negate {
                value = self.negate(value)?;
            }
            // Unary `+` on an int is `int.__pos__`, the identity: no op.
        }
        Ok(value)
    }

    /// `primary = atom { suffix }`. A suffix here is a call used as a value.
    fn lower_primary(&mut self, primary: &GrammarASTNode) -> Result<Evaluated, FrontendError> {
        let atom = match primary.children.as_slice() {
            [ASTNodeOrToken::Node(atom)] if atom.rule_name == "atom" => atom,
            [ASTNodeOrToken::Node(atom), ..] if atom.rule_name == "atom" => {
                return Err(self.unsupported(
                    primary,
                    "a call's result used as a value is not in py-int-print (`print(...)` returns None)",
                ))
            }
            _ => return Err(self.unsupported(primary, "expected an atom")),
        };
        match atom.children.as_slice() {
            [ASTNodeOrToken::Token(number)] if number.effective_type_name() == "INT" => {
                let value = self.decode_integer(atom, number)?;
                let register = self.register();
                self.emit("const", Some(register.clone()), vec![Operand::Int(value)], "i64")?;
                Ok(Evaluated {
                    operand: Operand::Var(register),
                    known: Some(i128::from(value)),
                })
            }
            [ASTNodeOrToken::Token(name)] if name.effective_type_name() == "NAME" => Err(self.unsupported_token(
                atom,
                name,
                format!("names such as {} are not in py-int-print (no variables)", shown(&name.value)),
            )),
            [ASTNodeOrToken::Node(paren)] if paren.rule_name == "paren_expr" => match paren.children.as_slice() {
                [open, ASTNodeOrToken::Node(body), close]
                    if is_token(open, "LPAREN") && is_token(close, "RPAREN") && body.rule_name == "paren_body" =>
                {
                    let inner = self.only_child(body, "expression")?;
                    self.lower_expression(inner)
                }
                _ => Err(self.unsupported(paren, "expected `(` expression `)`")),
            },
            _ => Err(self.unsupported(atom, "expected an integer, a name or `(expr)`")),
        }
    }

    /// `-a` as LANG83 `i64_neg_ovf` (only `-i64::MIN` traps).
    fn negate(&mut self, value: Evaluated) -> Result<Evaluated, FrontendError> {
        let known = value.known.map(|v| -v).filter(|v| fits_i64(*v));
        let register = self.register();
        self.emit("i64_neg_ovf", Some(register.clone()), vec![value.operand], "i64")?;
        Ok(Evaluated {
            operand: Operand::Var(register),
            known,
        })
    }

    /// Emit one binary operator.
    fn binary(&mut self, op: BinOp, left: Evaluated, right: Evaluated) -> Result<Evaluated, FrontendError> {
        // A divisor known to be -1: `a // -1 == -a` and `a % -1 == 0` for
        // every integer. Emitting those keeps `i64::MIN // -1` and
        // `i64::MIN % -1` away from every backend's divide (module docs).
        // Both operands were already evaluated above, so nothing observable
        // is skipped.
        if right.known == Some(-1) {
            match op {
                BinOp::FloorDiv => return self.negate(left),
                BinOp::Mod => {
                    let register = self.register();
                    self.emit("const", Some(register.clone()), vec![Operand::Int(0)], "i64")?;
                    return Ok(Evaluated {
                        operand: Operand::Var(register),
                        known: Some(0),
                    });
                }
                _ => {}
            }
        }
        let known = match (left.known, right.known) {
            (Some(a), Some(b)) => fold(op, a, b).filter(|v| fits_i64(*v)),
            // An operand certainly traps first; this operator never runs.
            _ => None,
        };
        let register = self.register();
        self.emit(op.iir_op(), Some(register.clone()), vec![left.operand, right.operand], "i64")?;
        Ok(Evaluated {
            operand: Operand::Var(register),
            known,
        })
    }

    /// Decode one `INT` token's value (LANG84 C1.4 allows decoding a single
    /// token; it never splits one).
    ///
    /// The release tokens spell a decimal integer as a non-zero digit and
    /// more digits, or as zeros, with single `_` separators between digits
    /// (PEP 515, where the release lexes them). A caller-supplied tree is not
    /// bound by the lexer, so the spelling is checked again here:
    ///
    /// | token value   | value   | why                                       |
    /// |---------------|---------|-------------------------------------------|
    /// | `42`          | 42      |                                           |
    /// | `1_000`       | 1000    | `_` separates digits                      |
    /// | `00`, `0_0`   | 0       | Python allows any number of zeros         |
    /// | `012`         | refused | leading zeros are a SyntaxError in Python 3 |
    /// | `1__0`, `_1`, `1_` | refused | not a Python integer                |
    /// | `-1`, `+1`    | refused | a sign is a separate token in Python      |
    fn decode_integer(&self, atom: &GrammarASTNode, token: &Token) -> Result<i64, FrontendError> {
        let text = token.value.as_str();
        let well_formed = !text.is_empty()
            && text
                .split('_')
                .all(|group| !group.is_empty() && group.bytes().all(|b| b.is_ascii_digit()));
        if !well_formed {
            return Err(self.unsupported_token(atom, token, format!("{} is not a decimal integer", shown(text))));
        }
        let digits: String = text.chars().filter(|c| *c != '_').collect();
        if digits.len() > 1 && digits.starts_with('0') && digits.bytes().any(|b| b != b'0') {
            return Err(self.unsupported_token(
                atom,
                token,
                "leading zeros in a non-zero decimal integer are a SyntaxError in Python 3",
            ));
        }
        // Leading zeros are all zeros by now; strip them so the length test
        // below measures the significant digits. 19 digits always fit in
        // i128; more than 19 never fit in i64.
        let significant = digits.trim_start_matches('0');
        let value = if significant.is_empty() {
            Some(0)
        } else if significant.len() > 19 {
            None
        } else {
            significant.parse::<i128>().ok().and_then(|v| i64::try_from(v).ok())
        };
        value.ok_or_else(|| {
            self.unsupported_token(
                atom,
                token,
                format!(
                    "integer literal {} is outside i64; CPython would keep it as a bigger int, \
                     which py-int-print does not have yet",
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
        // `py` + digits: the same temporary namespace as LANG79's pilot.
        let name = format!("py{}", self.next_register);
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

fn fits_i64(value: i128) -> bool {
    i64::try_from(value).is_ok()
}

/// The exact value of `a op b`, or `None` when it traps on a zero divisor.
///
/// `i128` holds every sum, difference and product of two `i64`s exactly, so
/// nothing here can overflow; the caller decides whether the result fits.
/// Floor division rounds the truncated quotient down when the remainder is
/// non-zero and its sign differs from the divisor's: the same fix-up LANG83
/// specifies for `i64_div_floor`, and CPython's `l_divmod` makes.
fn fold(op: BinOp, a: i128, b: i128) -> Option<i128> {
    match op {
        BinOp::Add => Some(a + b),
        BinOp::Sub => Some(a - b),
        BinOp::Mul => Some(a * b),
        BinOp::FloorDiv | BinOp::Mod if b == 0 => None,
        BinOp::FloorDiv => {
            let (q, r) = (a / b, a % b);
            Some(if r != 0 && (r < 0) != (b < 0) { q - 1 } else { q })
        }
        BinOp::Mod => {
            let r = a % b;
            Some(if r != 0 && (r < 0) != (b < 0) { r + b } else { r })
        }
    }
}

fn is_token(child: &ASTNodeOrToken, type_name: &str) -> bool {
    matches!(child, ASTNodeOrToken::Token(token) if token.effective_type_name() == type_name)
}

fn is_newline(token: &Token) -> bool {
    token.effective_type_name() == "NEWLINE"
}

/// Caller text inside a diagnostic: at most 32 characters, control
/// characters escaped (a direct-CST caller can put anything in a token).
fn shown(text: &str) -> String {
    let mut chars = text.chars();
    let head: String = chars.by_ref().take(32).flat_map(char::escape_debug).collect();
    if chars.next().is_some() {
        format!("`{head}…`")
    } else {
        format!("`{head}`")
    }
}

/// A rule name inside a diagnostic, bounded and escaped the same way.
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
    fn fold_matches_pythons_floor_division_sign_table() {
        // (a, b, a // b, a % b), checked against host CPython 3.13.16.
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
            assert_eq!(fold(BinOp::FloorDiv, a, b), Some(q), "{a} // {b}");
            assert_eq!(fold(BinOp::Mod, a, b), Some(r), "{a} % {b}");
        }
        assert_eq!(fold(BinOp::FloorDiv, 1, 0), None);
        assert_eq!(fold(BinOp::Mod, 1, 0), None);
        let min = i128::from(i64::MIN);
        assert_eq!(fold(BinOp::FloorDiv, min, -1), Some(-min));
        assert!(!fits_i64(-min));
        assert_eq!(fold(BinOp::Mod, min, -1), Some(0));
        assert_eq!(fold(BinOp::Mul, min, min), Some(min * min));
    }

    #[test]
    fn diagnostics_bound_and_escape_caller_text() {
        assert_eq!(shown("print"), "`print`");
        assert_eq!(shown("a\nb"), "`a\\nb`");
        let long = shown(&"x".repeat(1000));
        assert!(long.len() < 40 && long.ends_with("…`"), "{long}");
        assert_eq!(shown_rule(&"r".repeat(1000)).chars().count(), 65);
    }

    #[test]
    fn op_names_are_the_lang83_registry_entries() {
        for op in [BinOp::Add, BinOp::Sub, BinOp::Mul, BinOp::FloorDiv, BinOp::Mod] {
            assert!(interpreter_ir::op_registry::lookup(op.iir_op()).is_some(), "{op:?}");
        }
        assert!(interpreter_ir::op_registry::lookup("i64_neg_ovf").is_some());
    }
}
