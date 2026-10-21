//! Direct Perl grammar-tree to InterpreterIR lowering (LANG81, LANG84).
//!
//! The production path is the repository's own grammar-driven lexer and
//! parser, then this crate, then any LANG83 backend:
//!
//! ```text
//!   source ──► resolve_release ──► GrammarLexer(perl<release>.tokens)
//!          ──► GrammarParser(perl<release>.grammar) ──► GrammarASTNode
//!          ──► compile_ast_at ──► IIRModule (LANG83 ops only)
//! ```
//!
//! Semantic IR and a host Perl interpreter are not stages in that path. A
//! host `perl` appears only in tests, as an oracle that says what the right
//! output is.
//!
//! ## Which grammar runs
//!
//! Exactly one row of `code/grammars/perl/releases.csv` (LANG84 C4). A caller
//! names it (`Some("5.004_72")`) or names none and gets [`DEFAULT_RELEASE`],
//! the `5.38.0` pair grounded in the `v5.38.0` tag. The produced module
//! records the ID in `IIRModule::release`, every error names it, and nothing
//! ever falls back to a neighbouring release or to LANG81's unversioned
//! `perl.tokens`/`perl.grammar`, which are no longer on this path.
//!
//! ## The accepted subset
//!
//! One or more `print(<expr>);` statements, where an expression is decimal
//! integer literals, parentheses, unary `-`, and binary `+`, `-`, `*` with
//! Perl's precedence and left associativity. Every literal and every
//! intermediate result must provably fit in a signed 32-bit integer; the
//! compiler tracks each value while it lowers, so an out-of-range program is
//! refused before any IIR exists, rather than letting a backend wrap it.
//!
//! ## What `print` lowers to
//!
//! Perl's `print` writes its argument's decimal text and **no** newline, so
//! `print(-4); print(2);` prints `-42`. That is exactly the LANG83 standard
//! builtin `write_i64`, which every backend lowers natively — `printf("%lld")`
//! on native, `System.out.print(J)V` on the JVM, `Console.Write(int64)` on the
//! CLR, `env.__write_i64` on WASM, and so on. The frontend therefore emits
//!
//! ```text
//!   pl0 = const 1        : i64
//!   pl1 = const 2        : i64
//!   pl2 = add pl0, pl1   : i64
//!         call_builtin write_i64, pl2 : void
//! ```
//!
//! and no language-named helper. (Before LANG84 phase 4a this was a
//! Perl-only `pl_print_int` builtin, which only an embedder that registered
//! it could run, so no lang-aot backend could.)

use coding_adventures_perl_lexer::release_token_grammar;
use coding_adventures_perl_parser::{release_parser_grammar, MAX_RULE_DEPTH};
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use lexer::grammar_lexer::GrammarLexer;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode, GrammarParser};
use std::fmt;
use std::sync::{Arc, Mutex};
use vm_core::{errors::VMError, value::Value, VMCore};

mod release;
pub use release::{resolve_release, Release, DEFAULT_RELEASE, DEFAULT_RELEASE_ID};

/// The CST contract (LANG84 C8): every name this compiler dispatches on.
///
/// The compiler walks the parse tree a release's `.grammar` produces, so it
/// silently depends on that grammar's rule names, on one token type, and on
/// the spellings of the punctuation it checks. If a release pair renamed
/// `term` to `product`, every program would be refused for that release —
/// a breaking change to the compiler, not a grammar detail. Naming them here,
/// and using these constants (not string literals) in the compiler below,
/// lets the `cst_contract` test check every checked release against exactly
/// what the compiler reads:
///
/// | Kind | Names | Checked against |
/// |---|---|---|
/// | rule | [`CST_RULES`] | the release's `ParserGrammar` rules |
/// | token type | [`CST_TOKEN_TYPES`] | the release's `TokenGrammar` definitions |
/// | spelling | [`CST_TOKEN_SPELLINGS`] | lexing each spelling alone yields one token with that value |
pub mod cst {
    /// The whole program: one or more print statements.
    pub const PROGRAM: &str = "program";
    /// `print ( expr ) ;`
    pub const PRINT_STATEMENT: &str = "print_statement";
    /// An additive chain: `term { (+|-) term }`.
    pub const EXPR: &str = "expr";
    /// A multiplicative chain: `unary { * unary }`.
    pub const TERM: &str = "term";
    /// `- unary` or an atom.
    pub const UNARY: &str = "unary";
    /// An integer literal or a parenthesized expression.
    pub const ATOM: &str = "atom";
    /// The integer-literal token type (`type_name`).
    pub const INT: &str = "INT";
}

/// Every CST rule name the compiler dispatches on (LANG84 C8).
pub const CST_RULES: &[&str] = &[
    cst::PROGRAM,
    cst::PRINT_STATEMENT,
    cst::EXPR,
    cst::TERM,
    cst::UNARY,
    cst::ATOM,
];

/// Every token type the compiler dispatches on (LANG84 C8).
pub const CST_TOKEN_TYPES: &[&str] = &[cst::INT];

/// Every token spelling the compiler compares a token's value against.
pub const CST_TOKEN_SPELLINGS: &[&str] = &["print", "(", ")", ";", "+", "-", "*"];

const MAX_AST_ITEMS: usize = 16_384;
const MAX_AST_DEPTH: usize = 64;
const MAX_AST_TEXT_BYTES: usize = 1024 * 1024;
const MAX_AST_FIELD_BYTES: usize = 64 * 1024;
const MAX_OUTPUT_BYTES: usize = 1_000_000;

/// The LANG83 standard builtin Perl `print` lowers to: decimal, no newline.
pub const PRINT_BUILTIN: &str = "write_i64";

/// Limits on untrusted source (LANG84 C5). Source programs are untrusted;
/// these bound the work any one of them can cause. Trusted callers may raise
/// them; [`FrontendBudgets::default`] is what [`compile_source`] uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontendBudgets {
    /// Source bytes accepted before lexing begins.
    pub source_bytes: usize,
    /// `GrammarParser` rule-depth cap. The parser's own default is
    /// `usize::MAX`, so this is always set explicitly.
    pub parser_depth: usize,
    /// IIR instructions one module may contain.
    pub iir_instructions: usize,
}

impl Default for FrontendBudgets {
    fn default() -> Self {
        Self {
            source_bytes: coding_adventures_perl_lexer::MAX_SOURCE_BYTES,
            parser_depth: MAX_RULE_DEPTH,
            iir_instructions: 1_000_000,
        }
    }
}

/// Every way a Perl compile can fail (LANG84 C6). Each variant that follows
/// release resolution names the release it ran against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontendError {
    /// The requested ID is not a row of `releases.csv`.
    UnknownRelease {
        /// The request, truncated for display.
        release: String,
        /// Up to five inventory IDs sharing the longest prefix with it.
        nearest: Vec<&'static str>,
    },
    /// The release is in the inventory but has no usable pair yet.
    ReleaseNotAvailable {
        /// The requested inventory ID.
        release: String,
        /// Why: `"pending"` for a row with no grammar.
        stage: &'static str,
    },
    /// A [`FrontendBudgets`] limit was exceeded.
    Budget {
        /// The release the compile ran against.
        release: String,
        /// Which budget.
        which: &'static str,
        /// Its limit.
        limit: usize,
    },
    /// The release's token grammar matched no token at this position.
    Lex {
        /// The release the compile ran against.
        release: String,
        /// 1-based line.
        line: usize,
        /// 1-based column.
        column: usize,
        /// The lexer's message.
        message: String,
    },
    /// The release's parser grammar rejected the token sequence.
    Parse {
        /// The release the compile ran against.
        release: String,
        /// 1-based line of the furthest failure.
        line: usize,
        /// 1-based column of the furthest failure.
        column: usize,
        /// The parser's message, including the expected terminals.
        message: String,
    },
    /// The tree parsed but is outside the subset this compiler lowers, or a
    /// value leaves the signed 32-bit range.
    Unsupported {
        /// The release the compile ran against.
        release: String,
        /// The CST rule the compiler was lowering, when it had one.
        rule: Option<String>,
        /// The node's 1-based start `(line, column)`, when the CST records it.
        start: Option<(usize, usize)>,
        /// The node's 1-based end `(line, column)`, when the CST records it.
        end: Option<(usize, usize)>,
        /// What was refused.
        message: String,
    },
}

impl fmt::Display for FrontendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrontendError::UnknownRelease { release, nearest } => {
                write!(f, "unknown Perl release {release:?}")?;
                if !nearest.is_empty() {
                    write!(f, "; nearest inventory IDs: {}", nearest.join(", "))?;
                }
                Ok(())
            }
            FrontendError::ReleaseNotAvailable { release, stage } => {
                write!(
                    f,
                    "Perl release {release} has no grammar pair yet ({stage})"
                )
            }
            FrontendError::Budget {
                release,
                which,
                limit,
            } => write!(f, "{release}: {which} budget of {limit} exceeded"),
            FrontendError::Lex {
                release,
                line,
                column,
                message,
            } => write!(
                f,
                "{release}: Perl tokenization failed at {line}:{column}: {message}"
            ),
            FrontendError::Parse {
                release,
                line,
                column,
                message,
            } => write!(
                f,
                "{release}: Perl parse failed at {line}:{column}: {message}"
            ),
            FrontendError::Unsupported {
                release,
                rule,
                start,
                end,
                message,
            } => {
                write!(f, "{release}: {message}")?;
                if let Some(rule) = rule {
                    write!(f, " (rule {rule}")?;
                    if let Some((line, column)) = start {
                        write!(f, " at {line}:{column}")?;
                    }
                    if let Some((line, column)) = end {
                        write!(f, "-{line}:{column}")?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for FrontendError {}

/// Parse and compile Perl source against exactly one release (LANG84).
///
/// `release` is `None` for [`DEFAULT_RELEASE`], or an exact inventory ID such
/// as `Some("5.004_72")`. A pending or unknown ID is an error; nothing falls
/// back to another grammar.
pub fn compile_source_at(
    source: &str,
    release: Option<&str>,
    module_name: &str,
    budgets: &FrontendBudgets,
) -> Result<IIRModule, FrontendError> {
    let release = resolve_release(release)?;
    let label = release.to_string();
    if source.len() > budgets.source_bytes {
        return Err(FrontendError::Budget {
            release: label,
            which: "source bytes",
            limit: budgets.source_bytes,
        });
    }
    // Resolution guarantees both tables exist for an inventory release: the
    // lexer and parser crates are generated from the same CSV rows, and their
    // own tests pin that. There is no other kind of release (LANG84 C4.5):
    // the unversioned LANG81 pair is unreachable from here.
    let (Some(token_grammar), Some(parser_grammar)) = (
        release_token_grammar(release.id()),
        release_parser_grammar(release.id()),
    ) else {
        return Err(FrontendError::ReleaseNotAvailable {
            release: release.id().to_string(),
            stage: "no compiled table",
        });
    };
    let tokens = GrammarLexer::new(source, &token_grammar)
        .tokenize()
        .map_err(|error| FrontendError::Lex {
            release: label.clone(),
            line: error.line,
            column: error.column,
            message: error.message,
        })?;
    let tree = GrammarParser::new(tokens, parser_grammar)
        .with_max_depth(budgets.parser_depth)
        .parse()
        .map_err(|error| FrontendError::Parse {
            release: label.clone(),
            line: error.token.line,
            column: error.token.column,
            message: error.message,
        })?;
    compile_ast_at(&tree, release, module_name, budgets)
}

/// Parse and compile Perl source with [`DEFAULT_RELEASE`] and default
/// budgets. Kept for callers that predate release selection; it is
/// `compile_source_at(source, None, module_name, &FrontendBudgets::default())`.
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, String> {
    compile_source_at(source, None, module_name, &FrontendBudgets::default())
        .map_err(|error| error.to_string())
}

/// Compile a Perl parser tree supplied directly by a caller, as
/// [`DEFAULT_RELEASE`] with default budgets.
pub fn compile_ast(tree: &GrammarASTNode, module_name: &str) -> Result<IIRModule, String> {
    compile_ast_at(
        tree,
        DEFAULT_RELEASE,
        module_name,
        &FrontendBudgets::default(),
    )
    .map_err(|error| error.to_string())
}

/// Compile a Perl parser tree for a resolved release.
///
/// The tree may come from a caller rather than from this crate's parser, so
/// it is bounded (items, depth, text) before anything recursive touches it.
pub fn compile_ast_at(
    tree: &GrammarASTNode,
    release: Release,
    module_name: &str,
    budgets: &FrontendBudgets,
) -> Result<IIRModule, FrontendError> {
    let label = release.to_string();
    let unsupported = |error: Refusal| FrontendError::Unsupported {
        release: label.clone(),
        rule: error.rule,
        start: error.start,
        end: error.end,
        message: error.message,
    };
    if module_name.len() > MAX_AST_FIELD_BYTES {
        return Err(unsupported(Refusal::bare(
            "Perl IIR module name exceeds the native pilot limit",
        )));
    }
    check_ast_budget(tree).map_err(|message| unsupported(Refusal::bare(message)))?;
    if tree.rule_name != cst::PROGRAM || tree.children.is_empty() {
        return Err(unsupported(Refusal::at(
            tree,
            "expected a Perl print program",
        )));
    }
    let mut compiler = Compiler::default();
    for child in &tree.children {
        let ASTNodeOrToken::Node(statement) = child else {
            return Err(unsupported(Refusal::at(
                tree,
                "unsupported Perl program item",
            )));
        };
        compiler.compile_print(statement).map_err(unsupported)?;
        if compiler.instructions.len() > budgets.iir_instructions {
            return Err(FrontendError::Budget {
                release: label,
                which: "IIR instructions",
                limit: budgets.iir_instructions,
            });
        }
    }
    // `main` returns the exit status 0 rather than `ret_void`. Every backend
    // reads the entry function's result as the process status: CoreCLR
    // rejects an `int32 MccarthyEntry()` that returns nothing, and on BEAM a
    // void `return` hands back whatever x0 last held (the atom `ok` from the
    // final print), which the launcher cannot read as an integer. Returning a
    // real `i64` 0 gives every column the same, well-defined status.
    let exit = compiler.register();
    compiler.emit("const", Some(exit.clone()), vec![Operand::Int(0)], "i64");
    compiler.emit("ret", None, vec![Operand::Var(exit)], "i64");
    let mut module = IIRModule::new(module_name, "perl");
    module.entry_point = Some("main".into());
    // LANG84 C4.1: the module records the one release that produced it.
    module.release = Some(release.id().to_string());
    module.add_or_replace(IIRFunction::new(
        "main",
        vec![],
        "i64",
        compiler.instructions,
    ));
    let errors = module.validate();
    if !errors.is_empty() {
        return Err(unsupported(Refusal::bare(format!(
            "invalid Perl IIR: {}",
            errors.join("; ")
        ))));
    }
    Ok(module)
}

/// Run Perl source on the native Rust VM and capture its `print` output.
pub fn run_source(source: &str) -> Result<String, String> {
    run_source_at(source, None)
}

/// [`run_source`] against an explicit release.
pub fn run_source_at(source: &str, release: Option<&str>) -> Result<String, String> {
    let mut module = compile_source_at(source, release, "perl-script", &FrontendBudgets::default())
        .map_err(|error| error.to_string())?;
    let output = Arc::new(Mutex::new(String::new()));
    let captured = Arc::clone(&output);
    let mut vm = VMCore::new();
    vm.max_instructions = Some(100_000);
    // vm-core's default `write_i64` writes to the process stdout. Replace it
    // with a capturing, bounded one so the caller receives the text instead.
    vm.builtins_mut().register(PRINT_BUILTIN, move |args| {
        let [Value::Int(number)] = args else {
            return Err(VMError::Custom("write_i64 expects one integer".into()));
        };
        let rendered = number.to_string();
        let mut sink = captured
            .lock()
            .map_err(|_| VMError::Custom("Perl output lock poisoned".into()))?;
        if sink
            .len()
            .checked_add(rendered.len())
            .is_none_or(|length| length > MAX_OUTPUT_BYTES)
        {
            return Err(VMError::Custom("Perl output limit exceeded".into()));
        }
        sink.push_str(&rendered);
        Ok(Value::Null)
    });
    vm.execute(&mut module, "main", &[])
        .map_err(|error| error.to_string())?;
    output
        .lock()
        .map_err(|_| "Perl output lock poisoned".to_string())
        .map(|sink| sink.clone())
}

fn check_ast_budget(root: &GrammarASTNode) -> Result<(), String> {
    let mut pending = vec![(root, 1_usize)];
    let mut seen = 1_usize;
    let mut text_bytes = 0_usize;
    while let Some((node, depth)) = pending.pop() {
        if depth > MAX_AST_DEPTH {
            return Err("Perl AST exceeds the native pilot depth limit".into());
        }
        count_ast_text(&node.rule_name, &mut text_bytes)?;
        for child in &node.children {
            if seen == MAX_AST_ITEMS {
                return Err("Perl AST exceeds the native pilot item limit".into());
            }
            seen += 1;
            match child {
                ASTNodeOrToken::Node(inner) => {
                    if depth == MAX_AST_DEPTH {
                        return Err("Perl AST exceeds the native pilot depth limit".into());
                    }
                    pending.push((inner, depth + 1));
                }
                ASTNodeOrToken::Token(token) => {
                    count_ast_text(&token.value, &mut text_bytes)?;
                    if let Some(type_name) = &token.type_name {
                        count_ast_text(type_name, &mut text_bytes)?;
                    }
                    if let Some(cv) = &token.cv {
                        count_ast_text(cv, &mut text_bytes)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn count_ast_text(field: &str, total: &mut usize) -> Result<(), String> {
    if field.len() > MAX_AST_FIELD_BYTES
        || total
            .checked_add(field.len())
            .is_none_or(|next| next > MAX_AST_TEXT_BYTES)
    {
        return Err("Perl AST exceeds the native pilot text limit".into());
    }
    *total += field.len();
    Ok(())
}

fn token_value(child: &ASTNodeOrToken) -> Option<&str> {
    match child {
        ASTNodeOrToken::Token(token) => Some(&token.value),
        ASTNodeOrToken::Node(_) => None,
    }
}

/// A refusal before it is tagged with the release: the message plus, when
/// the compiler was looking at a node, that node's rule and span.
struct Refusal {
    rule: Option<String>,
    start: Option<(usize, usize)>,
    end: Option<(usize, usize)>,
    message: String,
}

impl Refusal {
    fn bare(message: impl Into<String>) -> Self {
        Self {
            rule: None,
            start: None,
            end: None,
            message: message.into(),
        }
    }

    fn at(node: &GrammarASTNode, message: impl Into<String>) -> Self {
        Self {
            rule: Some(node.rule_name.clone()),
            start: node.start_line.zip(node.start_column),
            end: node.end_line.zip(node.end_column),
            message: message.into(),
        }
    }
}

fn checked_i32(node: &GrammarASTNode, value: i64) -> Result<i32, Refusal> {
    i32::try_from(value).map_err(|_| Refusal::at(node, "Perl integer exceeds native pilot range"))
}

struct Compiled {
    operand: Operand,
    value: i32,
}

#[derive(Default)]
struct Compiler {
    instructions: Vec<IIRInstr>,
    next_register: usize,
}

impl Compiler {
    fn emit(&mut self, op: &str, dest: Option<String>, srcs: Vec<Operand>, type_hint: &str) {
        self.instructions
            .push(IIRInstr::new(op, dest, srcs, type_hint));
    }

    fn register(&mut self) -> String {
        let name = format!("pl{}", self.next_register);
        self.next_register += 1;
        name
    }

    fn compile_print(&mut self, node: &GrammarASTNode) -> Result<(), Refusal> {
        let [name, open, ASTNodeOrToken::Node(expr), close, semicolon] = node.children.as_slice()
        else {
            return Err(Refusal::at(node, "unsupported Perl print statement"));
        };
        if node.rule_name != cst::PRINT_STATEMENT
            || token_value(name) != Some("print")
            || token_value(open) != Some("(")
            || expr.rule_name != cst::EXPR
            || token_value(close) != Some(")")
            || token_value(semicolon) != Some(";")
        {
            return Err(Refusal::at(
                node,
                "native Perl pilot requires print(one expression);",
            ));
        }
        let value = self.compile_expression(expr)?;
        self.emit(
            "call_builtin",
            None,
            vec![Operand::Var(PRINT_BUILTIN.into()), value.operand],
            "void",
        );
        Ok(())
    }

    fn compile_expression(&mut self, node: &GrammarASTNode) -> Result<Compiled, Refusal> {
        match node.rule_name.as_str() {
            cst::EXPR | cst::TERM => self.compile_chain(node),
            cst::UNARY => match node.children.as_slice() {
                [ASTNodeOrToken::Node(atom)] if atom.rule_name == cst::ATOM => {
                    self.compile_expression(atom)
                }
                [sign, ASTNodeOrToken::Node(inner)]
                    if token_value(sign) == Some("-") && inner.rule_name == cst::UNARY =>
                {
                    let value = self.compile_expression(inner)?;
                    let checked = checked_i32(node, -i64::from(value.value))?;
                    let dest = self.register();
                    self.emit("neg", Some(dest.clone()), vec![value.operand], "i64");
                    Ok(Compiled {
                        operand: Operand::Var(dest),
                        value: checked,
                    })
                }
                _ => Err(Refusal::at(node, "unsupported Perl unary expression")),
            },
            cst::ATOM => match node.children.as_slice() {
                [ASTNodeOrToken::Token(token)] if token.type_name.as_deref() == Some(cst::INT) => {
                    if token.value.len() > 1 && token.value.starts_with('0') {
                        return Err(Refusal::at(
                            node,
                            "Perl legacy octal literal is outside the decimal pilot",
                        ));
                    }
                    let parsed = token.value.parse::<i64>().map_err(|_| {
                        Refusal::at(node, "Perl integer exceeds native pilot range")
                    })?;
                    let value = checked_i32(node, parsed)?;
                    let dest = self.register();
                    self.emit(
                        "const",
                        Some(dest.clone()),
                        vec![Operand::Int(i64::from(value))],
                        "i64",
                    );
                    Ok(Compiled {
                        operand: Operand::Var(dest),
                        value,
                    })
                }
                [open, ASTNodeOrToken::Node(expr), close]
                    if token_value(open) == Some("(")
                        && expr.rule_name == cst::EXPR
                        && token_value(close) == Some(")") =>
                {
                    self.compile_expression(expr)
                }
                _ => Err(Refusal::at(node, "unsupported Perl atom")),
            },
            _ => Err(Refusal::at(
                node,
                format!("unsupported Perl {} expression", node.rule_name),
            )),
        }
    }

    fn compile_chain(&mut self, node: &GrammarASTNode) -> Result<Compiled, Refusal> {
        let child_rule = if node.rule_name == cst::EXPR {
            cst::TERM
        } else {
            cst::UNARY
        };
        let Some(ASTNodeOrToken::Node(first)) = node.children.first() else {
            return Err(Refusal::at(node, "empty Perl arithmetic expression"));
        };
        if first.rule_name != child_rule {
            return Err(Refusal::at(node, "unsupported Perl arithmetic chain"));
        }
        let mut acc = self.compile_expression(first)?;
        let (pairs, remainder) = node.children[1..].as_chunks::<2>();
        for pair in pairs {
            let [operator, ASTNodeOrToken::Node(right)] = pair else {
                return Err(Refusal::at(node, "unsupported Perl arithmetic chain"));
            };
            if right.rule_name != child_rule {
                return Err(Refusal::at(node, "unsupported Perl arithmetic chain"));
            }
            let rhs = self.compile_expression(right)?;
            let (opcode, value) = match (node.rule_name.as_str(), token_value(operator)) {
                (cst::EXPR, Some("+")) => ("add", i64::from(acc.value) + i64::from(rhs.value)),
                (cst::EXPR, Some("-")) => ("sub", i64::from(acc.value) - i64::from(rhs.value)),
                (cst::TERM, Some("*")) => ("mul", i64::from(acc.value) * i64::from(rhs.value)),
                _ => return Err(Refusal::at(node, "unsupported Perl arithmetic operator")),
            };
            let value = checked_i32(node, value)?;
            let dest = self.register();
            self.emit(
                opcode,
                Some(dest.clone()),
                vec![acc.operand, rhs.operand],
                "i64",
            );
            acc = Compiled {
                operand: Operand::Var(dest),
                value,
            };
        }
        if !remainder.is_empty() {
            return Err(Refusal::at(node, "incomplete Perl arithmetic expression"));
        }
        Ok(acc)
    }
}

#[cfg(test)]
mod tests;
