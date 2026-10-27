//! # Walking the `py-int-print` / `py-float-print` / `py2-int-print` parse tree into IIR (LANG84 phases 7a, 7c)
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
//! ## The Python 2 tree, for `print 1 + 2` (phase 7c slice 1)
//!
//! A `py2` pair adds one rule, `print_stmt`, because in Python 2 `print` is a
//! keyword that starts a statement, not a function:
//!
//! ```text
//! simple_stmt
//! ├─ small_stmt ─ print_stmt
//! │               ├─ KEYWORD "print"
//! │               └─ expression ─ arith ─ … (the same arithmetic as above)
//! └─ NEWLINE
//! ```
//!
//! Which `print` a statement is comes from this **shape** (a `print_stmt`
//! node or a call expression), never from the release ID (LANG84 C2). A
//! Python 3 grammar has no `print_stmt` rule, and in a Python 2 grammar
//! `print` is a keyword, so it can never be a call's callee.
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
//! | `print a` (Python 2)  | `call_builtin print_i64, a`                       |
//! | `print` (Python 2)    | `call_builtin putchar, 10`                        |
//! | `a / b` (`py2` only)  | as `a // b`: classic division floors on ints      |
//! | end of module         | `const 0`, `ret` (a script that ends exits 0)     |
//!
//! The float rows (`py-float-print`, phase 7c slice 3, `py3` profile only)
//! are in "Floats" below.
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
//!
//! ## Floats (`py-float-print`, phase 7c slice 3)
//!
//! Under a profile with [`Floats::Repr`] (`py3`: 3.5.10 and later) every
//! value has a static type, `int` or `float`, known while lowering. An
//! `int` lives in an `i64` register exactly as above; a `float` in an `f64`
//! register, and all its arithmetic is the LANG83 PR6c IEEE family, which
//! never traps (CPython's `float_add`/`float_sub`/`float_mul` never raise
//! either: an overflow is `inf`, `inf - inf` is `nan`).
//!
//! | Python                          | IIR                                              |
//! |---------------------------------|--------------------------------------------------|
//! | `1.5`, `.5`, `1.`, `1e16`       | `pyN = const 1.5 : f64` (correctly rounded)      |
//! | `int` operand of a float op     | `pyN = int_to_real i : f64`                      |
//! | `a + b`, `a - b`, `a * b`       | `f64_add` / `f64_sub` / `f64_mul` (either is a float) |
//! | `a / b` (any types)             | both converted, then `f64_div`                   |
//! | `a / b`, `b` known `±0.0`       | `trap "Trap.DivZero"`, then the (dead) `f64_div` |
//! | `-a`, `+a` (float)              | `f64_neg` (`-0.0` stays `-0.0`); nothing         |
//! | `print(a)` (float)              | `call __py_write_float, a`; `putchar 10`         |
//! | `a // b`, `a % b`, one a float  | refused (`Unsupported`): no LANG83 op yet        |
//!
//! Three refusals keep the IIR exactly CPython's arithmetic:
//!
//! - **A zero divisor** is a compile-time fact here (every value is known),
//!   so the `ZeroDivisionError` that `float_div` and `long_true_divide`
//!   raise becomes LANG83's `trap`, after the program's earlier output.
//! - **`int / int` with an operand above 2^53** is refused: CPython divides
//!   the exact integers and rounds once, but converting first rounds twice.
//!   At or below 2^53 both convert exactly and `f64_div` rounds once, which
//!   is `long_true_divide`'s own fast path.
//! - **A literal that overflows to `inf`** (`1e400`) is refused: a
//!   non-finite `const` has no LANG83 contract (BEAM's literal table cannot
//!   hold one). `1e308 * 10` still reaches `inf`, through `f64_mul`.
//!
//! Known float values are folded with Rust's `f64`, the same IEEE binary64
//! arithmetic the PR6c ops specify; they decide only the zero-divisor test.
//!
//! ## Printing a float (LANG83 PR7c-py)
//!
//! No backend knows Python's float layout. `repr(x)` is a function this
//! compiler writes into the module, `__py_write_float`, built from the
//! neutral `f64_class`, `f64_signbit`, `f64_digits` and `f64_exp10` plus
//! integer code and `putchar` (see the `float_repr` module). Every float
//! `print` is one `call` of it. Python prints the sign of a zero itself
//! (`-0.0`), so unlike JavaScript's `console.log` the call site needs no sign
//! test. Only a profile with [`Floats::Repr`] ever reaches a float print:
//! under `py30` and `py2` a `FLOAT` token and a true `/` are refused, so
//! their modules never carry the helpers.

use interpreter_ir::{IIRInstr, Operand};
use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

use crate::float_repr;
use crate::frontend_error::FrontendError;
use crate::profiles::{Floats, IntLiterals, ReleaseProfile, Slash};

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

/// The register every `call __py_write_float` writes its unused `0` into.
/// A temporary is `py` followed only by digits, so this cannot collide.
const REPR_RESULT: &str = "py_repr";

/// The newline `print()` writes, as the byte `putchar` takes.
const NEWLINE_BYTE: i64 = b'\n' as i64;

/// The largest magnitude an `int` may have in a true division: 2^53, the
/// last power of two below which every integer is an exact `f64`.
const EXACT_F64_INT: i128 = 1 << 53;

/// A binary operator of the subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinOp {
    Add,
    Sub,
    Mul,
    FloorDiv,
    Mod,
    /// `/` under [`Slash::True`] with floats: always a float result.
    TrueDiv,
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
            // Never reached: true division is always lowered as a float.
            BinOp::TrueDiv => "f64_div",
        }
    }

    /// The LANG83 PR6c op that computes it on two `f64`s, if there is one.
    fn f64_op(self) -> Option<&'static str> {
        match self {
            BinOp::Add => Some("f64_add"),
            BinOp::Sub => Some("f64_sub"),
            BinOp::Mul => Some("f64_mul"),
            BinOp::TrueDiv => Some("f64_div"),
            BinOp::FloorDiv | BinOp::Mod => None,
        }
    }
}

/// A value's static type. Python's `int` and `float` are the only two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ty {
    Int,
    Float,
}

/// A value known while lowering: exact for an `int`, IEEE for a `float`.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Known {
    Int(i128),
    Float(f64),
}

/// One evaluated expression: where its value lives, its type, and the
/// value itself unless evaluation certainly traps first.
struct Evaluated {
    operand: Operand,
    ty: Ty,
    known: Option<Known>,
}

impl Evaluated {
    /// An `int` result.
    fn int(register: String, known: Option<i128>) -> Self {
        Evaluated { operand: Operand::Var(register), ty: Ty::Int, known: known.map(Known::Int) }
    }

    /// A `float` result.
    fn float(register: String, known: Option<f64>) -> Self {
        Evaluated { operand: Operand::Var(register), ty: Ty::Float, known: known.map(Known::Float) }
    }

    /// The exact value of a known `int`.
    fn known_int(&self) -> Option<i128> {
        match self.known {
            Some(Known::Int(value)) => Some(value),
            _ => None,
        }
    }

    /// The value of a known `float`.
    fn known_float(&self) -> Option<f64> {
        match self.known {
            Some(Known::Float(value)) => Some(value),
            _ => None,
        }
    }
}

/// The state of one compile.
pub(crate) struct Lowering<'a> {
    release: &'a str,
    profile: &'a ReleaseProfile,
    instructions: Vec<IIRInstr>,
    next_register: usize,
}

impl<'a> Lowering<'a> {
    /// Whether this release's profile adds `rule` to its grammars
    /// ([`crate::profiles::PROFILE_RULES`]). The lowering uses it to refuse a
    /// tree shaped by another profile's grammar.
    fn profile_owns_rule(&self, rule: &str) -> bool {
        crate::profiles::PROFILE_RULES
            .iter()
            .any(|(profile, rules)| *profile == self.profile.name && rules.contains(&rule))
    }

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
                ASTNodeOrToken::Node(small) if small.rule_name == "small_stmt" => match small.children.as_slice() {
                    // Python 2: `small_stmt = print_stmt | expression`.
                    [ASTNodeOrToken::Node(print)] if print.rule_name == "print_stmt" => {
                        // A caller-supplied tree (`compile_cst`) may come from
                        // another release's grammar. Only a profile whose
                        // grammars define `print_stmt` may lower one: under
                        // `py3`, `print x` is a SyntaxError, not output.
                        if !self.profile_owns_rule("print_stmt") {
                            return Err(self.unsupported(
                                print,
                                format!("a `print` statement is not in {}; this tree is not from this release's grammar", self.profile.subset()),
                            ));
                        }
                        self.lower_print_stmt(print)?;
                    }
                    _ => {
                        let expression = self.only_child(small, "expression")?;
                        self.lower_print_statement(small, expression)?;
                    }
                },
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

    /// Python 2's `print` statement (`py2-int-print`):
    ///
    /// ```text
    /// print_stmt = "print" [ expression { COMMA expression } [ COMMA ] ] ;
    /// ```
    ///
    /// | source      | CPython 2 bytecode               | IIR                    |
    /// |-------------|----------------------------------|------------------------|
    /// | `print`     | PRINT_NEWLINE                    | `putchar 10`           |
    /// | `print e`   | e; PRINT_ITEM; PRINT_NEWLINE     | e; `print_i64`         |
    /// | `print a, b`| refused (softspace, see below)   |                        |
    /// | `print a,`  | refused (softspace, see below)   |                        |
    ///
    /// `print a, b` and `print a,` leave the stream's softspace flag set
    /// between items, and CPython 2 writes a pending softspace as `\n`
    /// before an uncaught exception's traceback (`Py_FlushLine`). Without a
    /// trap handler the compiler cannot reproduce that newline, so both forms
    /// are `Unsupported` rather than subtly wrong.
    fn lower_print_stmt(&mut self, print: &GrammarASTNode) -> Result<(), FrontendError> {
        let mut parts = print.children.iter();
        match parts.next() {
            Some(ASTNodeOrToken::Token(keyword))
                if keyword.effective_type_name() == "KEYWORD" && keyword.value == "print" => {}
            _ => return Err(self.unsupported(print, "a print statement starts with the keyword `print`")),
        }
        let rest: Vec<&ASTNodeOrToken> = parts.collect();
        match rest.as_slice() {
            [] => self.emit_newline(),
            [ASTNodeOrToken::Node(expression)] if expression.rule_name == "expression" => {
                // The item is evaluated before PRINT_ITEM writes anything, so
                // a trap inside it writes nothing for this statement.
                let value = self.lower_expression(expression)?;
                self.emit_print(value)
            }
            [ASTNodeOrToken::Node(expression), ASTNodeOrToken::Token(comma)]
                if expression.rule_name == "expression" && comma.effective_type_name() == "COMMA" =>
            {
                Err(self.unsupported_token(
                    print,
                    comma,
                    format!(
                        "`print x,` (no newline) is not in {}: it leaves softspace set, and CPython 2 \
                         flushes softspace as a newline before an uncaught exception",
                        self.profile.subset()
                    ),
                ))
            }
            [ASTNodeOrToken::Node(expression), ASTNodeOrToken::Token(comma), ..]
                if expression.rule_name == "expression" && comma.effective_type_name() == "COMMA" =>
            {
                Err(self.unsupported_token(
                    print,
                    comma,
                    format!(
                        "`print a, b` is not in {}: items are separated by softspace, which CPython 2 \
                         flushes as a newline before an uncaught exception; print one value per statement",
                        self.profile.subset()
                    ),
                ))
            }
            _ => Err(self.unsupported(print, "expected `print` or `print <expression>`")),
        }
    }

    /// `print(x)` writes `str(x)` and `\n`. For an `int` that is LANG83
    /// `print_i64`, newline included; for a `float` it is `repr(x)` (what
    /// `str` is since 3.2, and the only floats a profile admits), which the
    /// module's own `__py_write_float` writes with no newline, then
    /// `putchar 10`:
    ///
    /// ```text
    ///   call          py_repr <- __py_write_float, x   (result unused, 0)
    ///   const         pyM <- 10
    ///   call_builtin  putchar, pyM
    /// ```
    ///
    /// One `call` per print site, never the layout inlined (LANG83 PR7c-py):
    /// [`crate::release_api`] adds the helper and the three it calls to the
    /// module when `main` holds such a call. Up to 0.3.0 this was the
    /// `write_f64_py` builtin, which every backend had to implement.
    fn emit_print(&mut self, value: Evaluated) -> Result<(), FrontendError> {
        match value.ty {
            Ty::Int => self.emit(
                "call_builtin",
                None,
                vec![Operand::Var("print_i64".into()), value.operand],
                "void",
            ),
            Ty::Float => {
                // Every print writes its (unused) `0` into the same register,
                // so a float print costs `main` no register of its own: a
                // VM frame holds at most 65,536 (`VMFrame::MAX_FRAME_REGISTERS`).
                self.emit(
                    "call",
                    Some(REPR_RESULT.into()),
                    vec![Operand::Var(float_repr::WRITE_FLOAT.into()), value.operand],
                    "i64",
                )?;
                self.emit_newline()
            }
        }
    }

    /// One `\n` through LANG83 `putchar` (`print()` and a bare `print`).
    fn emit_newline(&mut self) -> Result<(), FrontendError> {
        let newline = self.register();
        self.emit("const", Some(newline.clone()), vec![Operand::Int(NEWLINE_BYTE)], "i64")?;
        self.emit(
            "call_builtin",
            None,
            vec![Operand::Var("putchar".into()), Operand::Var(newline)],
            "void",
        )
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
                format!(
                    "only print statements are in {}; a bare expression statement is not",
                    self.profile.subset()
                ),
            ));
        };
        if callee.value != "print" {
            return Err(self.unsupported_token(
                small,
                callee,
                format!("only `print` is in {}, not {}", self.profile.subset(), shown(&callee.value)),
            ));
        }
        // In a `py2` grammar `print` is a keyword, so a call-shaped `print(…)`
        // can only reach here in a tree from a Python 3 grammar. Lowering it
        // under `py2` would be wrong, not just lenient: Python 2 prints
        // `print ()` as `()` and `print (1, 2)` as the tuple `(1, 2)`.
        if self.profile_owns_rule("print_stmt") {
            return Err(self.unsupported_token(
                small,
                callee,
                format!(
                    "`print` is a statement in {}; a call-form `print(…)` tree is not from this release's grammar",
                    self.profile.subset()
                ),
            ));
        }
        match self.call_arguments(suffix)?.as_slice() {
            // `print()` writes one newline and nothing else.
            [] => self.emit_newline(),
            [argument] => {
                // The argument is evaluated completely before `print`
                // writes, so a trap inside it prints nothing for this call.
                let value = self.lower_expression(argument)?;
                self.emit_print(value)
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
                Err(self.unsupported(node, format!("a call of a call's result is not in {}", self.profile.subset())))
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
            let (op, token) = match &pair[0] {
                ASTNodeOrToken::Token(token) => (self.binary_operator(chain, token)?, token),
                ASTNodeOrToken::Node(node) => return Err(self.unsupported(node, "expected an operator")),
            };
            if (operand_rule == "term") != matches!(op, BinOp::Add | BinOp::Sub) {
                return Err(self.unsupported(chain, "operator at the wrong precedence level"));
            }
            let right = self.lower_operand(chain, &pair[1], operand_rule)?;
            accumulator = self.binary(chain, token, op, accumulator, right)?;
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
                return Err(match (self.profile.slash, self.profile.floats) {
                    // True division gives a float (PEP 238).
                    (Slash::True, Floats::Repr) => return Ok(BinOp::TrueDiv),
                    (Slash::True, Floats::None) => self.unsupported_token(
                        chain,
                        token,
                        format!(
                            "`/` is true division in this release's profile ({}) and gives a \
                             float, which {} does not have; use `//`",
                            self.profile.name,
                            self.profile.subset()
                        ),
                    ),
                    // Classic division on two ints is floor division
                    // (`i_divmod` in Python 2's `Objects/intobject.c`).
                    (Slash::Classic, _) => return Ok(BinOp::FloorDiv),
                })
            }
            _ => {
                return Err(self.unsupported_token(
                    chain,
                    token,
                    format!("not an operator of {}", self.profile.subset()),
                ))
            }
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
            // Unary `+` is `int.__pos__` / `float.__pos__`, the identity: no
            // op (so `+-0.0` is still `-0.0`).
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
                    format!(
                        "a call's result used as a value is not in {} (there are no functions to call)",
                        self.profile.subset()
                    ),
                ))
            }
            _ => return Err(self.unsupported(primary, "expected an atom")),
        };
        match atom.children.as_slice() {
            [ASTNodeOrToken::Token(number)] if number.effective_type_name() == "INT" => {
                let value = self.decode_integer(atom, number)?;
                let register = self.register();
                self.emit("const", Some(register.clone()), vec![Operand::Int(value)], "i64")?;
                Ok(Evaluated::int(register, Some(i128::from(value))))
            }
            [ASTNodeOrToken::Token(number)] if number.effective_type_name() == "FLOAT" => {
                let value = self.decode_float(atom, number)?;
                let register = self.register();
                self.emit("const", Some(register.clone()), vec![Operand::Float(value)], "f64")?;
                Ok(Evaluated::float(register, Some(value)))
            }
            [ASTNodeOrToken::Token(name)] if name.effective_type_name() == "NAME" => Err(self.unsupported_token(
                atom,
                name,
                format!(
                    "names such as {} are not in {} (no variables)",
                    shown(&name.value),
                    self.profile.subset()
                ),
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
            _ => Err(self.unsupported(atom, "expected a number, a name or `(expr)`")),
        }
    }

    /// `-a`: LANG83 `i64_neg_ovf` on an `int` (only `-i64::MIN` traps), PR6c
    /// `f64_neg` on a `float` (a sign flip, so `-0.0` is `-0.0`; `0 - x`
    /// would give `0.0`).
    fn negate(&mut self, value: Evaluated) -> Result<Evaluated, FrontendError> {
        let register = self.register();
        match value.ty {
            Ty::Int => {
                let known = value.known_int().map(|v| -v).filter(|v| fits_i64(*v));
                self.emit("i64_neg_ovf", Some(register.clone()), vec![value.operand], "i64")?;
                Ok(Evaluated::int(register, known))
            }
            Ty::Float => {
                let known = value.known_float().map(|v| -v);
                self.emit("f64_neg", Some(register.clone()), vec![value.operand], "f64")?;
                Ok(Evaluated::float(register, known))
            }
        }
    }

    /// An `int` operand of a float operation, widened with LANG83
    /// `int_to_real` (round to nearest even, as `PyLong_AsDouble` does); a
    /// `float` is returned as it is.
    fn widen_to_float(&mut self, value: Evaluated) -> Result<Evaluated, FrontendError> {
        match value.ty {
            Ty::Float => Ok(value),
            Ty::Int => {
                // In range: a known int is an `i64` (larger ones trapped).
                let known = value.known_int().and_then(|v| i64::try_from(v).ok()).map(|v| v as f64);
                let register = self.register();
                self.emit("int_to_real", Some(register.clone()), vec![value.operand], "f64")?;
                Ok(Evaluated::float(register, known))
            }
        }
    }

    /// Emit one binary operator: on two `int`s as checked `i64` arithmetic,
    /// otherwise (or for a true `/`) as IEEE `f64` arithmetic.
    fn binary(
        &mut self,
        chain: &GrammarASTNode,
        token: &Token,
        op: BinOp,
        left: Evaluated,
        right: Evaluated,
    ) -> Result<Evaluated, FrontendError> {
        if op == BinOp::TrueDiv || left.ty == Ty::Float || right.ty == Ty::Float {
            return self.float_binary(chain, token, op, left, right);
        }
        self.int_binary(op, left, right)
    }

    /// Two `int`s: the checked `i64` ops of `py-int-print`.
    fn int_binary(&mut self, op: BinOp, left: Evaluated, right: Evaluated) -> Result<Evaluated, FrontendError> {
        // A divisor known to be -1: `a // -1 == -a` and `a % -1 == 0` for
        // every integer. Emitting those keeps `i64::MIN // -1` and
        // `i64::MIN % -1` away from every backend's divide (module docs).
        // Both operands were already evaluated above, so nothing observable
        // is skipped.
        if right.known_int() == Some(-1) {
            match op {
                BinOp::FloorDiv => return self.negate(left),
                BinOp::Mod => {
                    let register = self.register();
                    self.emit("const", Some(register.clone()), vec![Operand::Int(0)], "i64")?;
                    return Ok(Evaluated::int(register, Some(0)));
                }
                _ => {}
            }
        }
        let known = match (left.known_int(), right.known_int()) {
            (Some(a), Some(b)) => fold(op, a, b).filter(|v| fits_i64(*v)),
            // An operand certainly traps first; this operator never runs.
            _ => None,
        };
        let register = self.register();
        self.emit(op.iir_op(), Some(register.clone()), vec![left.operand, right.operand], "i64")?;
        Ok(Evaluated::int(register, known))
    }

    /// A float operation (`py-float-print`): either operand is a `float`,
    /// or the operator is a true `/`. The module docs' "Floats" table.
    fn float_binary(
        &mut self,
        chain: &GrammarASTNode,
        token: &Token,
        op: BinOp,
        left: Evaluated,
        right: Evaluated,
    ) -> Result<Evaluated, FrontendError> {
        let Some(iir_op) = op.f64_op() else {
            // `float_floor_div` and `float_rem` take `fmod` and adjust its
            // sign; LANG83 has no op for that yet.
            return Err(self.unsupported_token(
                chain,
                token,
                format!("`//` and `%` on a float are not in {} yet", self.profile.subset()),
            ));
        };
        if op == BinOp::TrueDiv && left.ty == Ty::Int && right.ty == Ty::Int {
            // `long_true_divide` rounds the exact quotient once; converting
            // first is the same only while both operands are exact `f64`s.
            for operand in [&left, &right] {
                if operand.known_int().is_some_and(|v| v.abs() > EXACT_F64_INT) {
                    return Err(self.unsupported_token(
                        chain,
                        token,
                        format!(
                            "`/` on an int above 2**53 in magnitude is not in {}: CPython rounds the \
                             exact quotient once, and converting the int first would round twice",
                            self.profile.subset()
                        ),
                    ));
                }
            }
        }
        let left = self.widen_to_float(left)?;
        let right = self.widen_to_float(right)?;
        let (a, b) = (left.known_float(), right.known_float());
        if op == BinOp::TrueDiv && b == Some(0.0) {
            // `±0.0` divisor (`0.0 == -0.0`): CPython raises
            // ZeroDivisionError before dividing. Every value is known, so
            // the trap is emitted here, after the output so far; the
            // division below is never reached.
            self.emit("trap", None, vec![Operand::Str("Trap.DivZero".into())], "void")?;
        }
        let known = match (a, b) {
            (Some(_), Some(b)) if op == BinOp::TrueDiv && b == 0.0 => None,
            (Some(a), Some(b)) => Some(match op {
                BinOp::Add => a + b,
                BinOp::Sub => a - b,
                BinOp::Mul => a * b,
                _ => a / b,
            }),
            // An operand certainly trapped first.
            _ => None,
        };
        let register = self.register();
        self.emit(iir_op, Some(register.clone()), vec![left.operand, right.operand], "f64")?;
        Ok(Evaluated::float(register, known))
    }

    /// Decode one `INT` token's value (LANG84 C1.4 allows decoding a single
    /// token; it never splits one). The profile's [`IntLiterals`] says which
    /// spellings exist; a caller-supplied tree is not bound by the lexer, so
    /// the spelling is checked again here against the *profile*, and a value
    /// above `i64::MAX` is refused (CPython would keep it as a bigger int or a
    /// long, which no integer subset has yet).
    ///
    /// The check is per profile, not per release: lowering never branches on
    /// the release ID (LANG84 C2). Spellings that only some releases of a
    /// profile lex (PEP 3127's `0o`/`0b` from 2.6, PEP 515's `_` from 3.6)
    /// are refused by that release's own lexer; a forged `compile_cst` tree
    /// claiming 2.2 with `0o17` decodes to 15, the value every release that
    /// can spell it gives. No input a release's lexer can produce decodes to
    /// a wrong value.
    fn decode_integer(&self, atom: &GrammarASTNode, token: &Token) -> Result<i64, FrontendError> {
        let text = token.value.as_str();
        let decoded = match self.profile.int_literals {
            IntLiterals::Decimal => decode_decimal(text),
            IntLiterals::Python2 => decode_python2(text),
        };
        match decoded {
            Ok(Some(value)) => Ok(value),
            Ok(None) => Err(self.unsupported_token(
                atom,
                token,
                format!(
                    "integer literal {} is outside i64; CPython would keep it as a bigger int, \
                     which {} does not have yet",
                    shown(text),
                    self.profile.subset()
                ),
            )),
            Err(why) => Err(self.unsupported_token(atom, token, why)),
        }
    }

    /// Decode one `FLOAT` token (`py-float-print`). Only a profile with
    /// floats has them; a caller-supplied tree is re-checked against the
    /// Python 3 float grammar ([`decode_float_text`]) per profile, as
    /// integers are (C2). The profile allows PEP 515's `_` in a float, so a
    /// tree forged through `compile_cst` that claims 3.5.10 and carries
    /// `1_0.5` decodes to 10.5. Only the 3.5.10 lexer refuses that spelling:
    /// lowering never branches on the release, and no source text 3.5.10 can
    /// lex decodes to a wrong value. A literal that overflows to `inf` is
    /// refused (module docs, "Floats").
    fn decode_float(&self, atom: &GrammarASTNode, token: &Token) -> Result<f64, FrontendError> {
        if self.profile.floats == Floats::None {
            return Err(self.unsupported_token(
                atom,
                token,
                format!(
                    "floats are not in {}: this release's `str(float)` is not `repr`, and LANG83 \
                     has no writer for its layout",
                    self.profile.subset()
                ),
            ));
        }
        let text = token.value.as_str();
        match decode_float_text(text) {
            Ok(value) if value.is_finite() => Ok(value),
            Ok(_) => Err(self.unsupported_token(
                atom,
                token,
                format!(
                    "float literal {} overflows to inf; a non-finite constant is not in {}",
                    shown(text),
                    self.profile.subset()
                ),
            )),
            Err(why) => Err(self.unsupported_token(atom, token, why)),
        }
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

/// Why a literal's spelling is refused (`Err`), or its value: `Some` when
/// it fits `i64`, `None` when it does not.
type Decoded = Result<Option<i64>, String>;

/// `py3` literals ([`IntLiterals::Decimal`]): a non-zero digit and more
/// digits, or zeros, with single `_` separators between digits (PEP 515,
/// where the release lexes them).
///
/// | token value        | value   | why                                       |
/// |--------------------|---------|-------------------------------------------|
/// | `42`               | 42      |                                           |
/// | `1_000`            | 1000    | `_` separates digits                      |
/// | `00`, `0_0`        | 0       | Python 3 allows any number of zeros       |
/// | `012`              | refused | a SyntaxError in every Python 3           |
/// | `1__0`, `_1`, `1_` | refused | not a Python integer                      |
/// | `0x1F`, `-1`, `+1` | refused | not in the subset; a sign is its own token |
fn decode_decimal(text: &str) -> Decoded {
    let well_formed = !text.is_empty()
        && text
            .split('_')
            .all(|group| !group.is_empty() && group.bytes().all(|b| b.is_ascii_digit()));
    if !well_formed {
        return Err(format!("{} is not a decimal integer", shown(text)));
    }
    let digits: String = text.chars().filter(|c| *c != '_').collect();
    if digits.len() > 1 && digits.starts_with('0') && digits.bytes().any(|b| b != b'0') {
        return Err("leading zeros in a non-zero integer are a SyntaxError in Python 3 \
                    (Python 2 read them as octal)"
            .to_string());
    }
    Ok(digits_value(&digits, 10))
}

/// `py2` literals ([`IntLiterals::Python2`]), as `tok_get` lexes them and
/// `parsenumber` reads them (see the `py2` profile in `profiles.rs`):
///
/// | token value          | value   | why                                    |
/// |----------------------|---------|----------------------------------------|
/// | `42`, `42L`, `42l`   | 42      | `L` makes a long; `str()` drops it      |
/// | `0`, `00`, `0L`      | 0       | a lone `0` and `00` are octal zero      |
/// | `010`                | 8       | a leading `0` means octal              |
/// | `0x1F`, `0X1fL`      | 31      | hex                                    |
/// | `0o17`, `0O17`       | 15      | octal, PEP 3127 (2.6 and later lex it) |
/// | `0b101`, `0B101L`    | 5       | binary, PEP 3127 (2.6 and later)       |
/// | `08`, `0x`, `0b2`    | refused | not a Python 2 integer                 |
/// | `1_000`, `1LL`       | refused | no `_` in Python 2; one suffix         |
///
/// The prefix and suffix letters are ASCII, so slicing after them is safe.
fn decode_python2(text: &str) -> Decoded {
    let body = text.strip_suffix(['l', 'L']).unwrap_or(text);
    let refused = || Err(format!("{} is not a Python 2 integer literal", shown(text)));
    let (digits, radix) = match body.as_bytes() {
        [b'0', b'x' | b'X', ..] => (&body[2..], 16),
        [b'0', b'o' | b'O', ..] => (&body[2..], 8),
        [b'0', b'b' | b'B', ..] => (&body[2..], 2),
        [b'0', ..] => (body, 8),
        [b'1'..=b'9', ..] => (body, 10),
        _ => return refused(),
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return refused();
    }
    Ok(digits_value(digits, radix))
}

/// The value of a run of digits already checked to be in `radix`: `Some`
/// when it fits `i64`, `None` when it does not. Checked accumulation keeps
/// any number of leading zeros harmless.
fn digits_value(digits: &str, radix: u32) -> Option<i64> {
    digits.chars().try_fold(0_i64, |value, c| {
        let digit = i64::from(c.to_digit(radix)?);
        value.checked_mul(i64::from(radix))?.checked_add(digit)
    })
}

/// A Python 3 float literal's value, correctly rounded (as CPython's
/// `PyOS_string_to_double` is), or why the text is not one:
///
/// | token value                  | value     | why                                  |
/// |------------------------------|-----------|--------------------------------------|
/// | `1.5`, `1.`, `.5`, `00.5`    | 1.5 … 0.5 | a float may start with zeros         |
/// | `1e3`, `1.5E-3`, `1.e5`      | 1000.0 …  | an exponent needs a digit            |
/// | `1_000.5`, `1e1_0`           | 1000.5 …  | single `_` between digits (PEP 515)  |
/// | `1e400`                      | `inf`     | the caller refuses it                |
/// | `1`, `1e`, `1_.5`, `inf`, `.`| refused   | not a Python float literal           |
///
/// The lexer already enforced this shape (and, per release, whether `_`
/// exists); a caller-supplied tree is not bound by it, so it is checked
/// again, and only digits, `.`, `e`/`E` and an exponent sign can reach
/// Rust's parser (never `inf` or `nan`).
fn decode_float_text(text: &str) -> Result<f64, String> {
    let refused = || Err(format!("{} is not a Python float literal", shown(text)));
    // A digit part: digits with single `_` separators between digits.
    let digit_part = |part: &str| {
        !part.is_empty() && part.split('_').all(|group| !group.is_empty() && group.bytes().all(|b| b.is_ascii_digit()))
    };
    let (mantissa, exponent) = match text.find(['e', 'E']) {
        Some(at) => (&text[..at], Some(&text[at + 1..])),
        None => (text, None),
    };
    let mantissa_ok = match mantissa.split_once('.') {
        Some((whole, fraction)) => {
            (whole.is_empty() || digit_part(whole))
                && (fraction.is_empty() || digit_part(fraction))
                && !(whole.is_empty() && fraction.is_empty())
        }
        // No `.`: digits, and then an exponent is required.
        None => digit_part(mantissa) && exponent.is_some(),
    };
    let exponent_ok = exponent.is_none_or(|e| digit_part(e.strip_prefix(['+', '-']).unwrap_or(e)));
    if !mantissa_ok || !exponent_ok {
        return refused();
    }
    // A token's text, decoded once (C1.4); never a release ID (C2).
    let digits: String = text.chars().filter(|c| *c != '_').collect();
    let value: Result<f64, _> = digits.parse();
    value.or_else(|_| refused())
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
        // Never an integer result: `binary` sends `/` to `float_binary`.
        BinOp::TrueDiv => None,
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
