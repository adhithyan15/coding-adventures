//! # Input budgets every Twig consumer inherits (#18474 security review)
//!
//! The IR compiler's budgets (`twig-ir-compiler/src/budgets.rs`) bound what
//! a program can make the *compiler* do.  A security review found an
//! attack that reaches the compiler and the type checker before any of
//! them runs: names.
//!
//! A record declaration makes the toolchain *invent* names.  The compiler
//! and the type checker both register, for `(record Name (f : T) …)`,
//!
//! ```text
//!   name?            the predicate
//!   name-f           one accessor per field
//! ```
//!
//! and a union does the same per variant.  Each accessor name repeats the
//! record's name, so the bytes invented are
//!
//! ```text
//!   (|name| + 1)  +  Σ over fields (|name| + 1 + |field|)
//! ```
//!
//! — the record's name *times* its field count.  With no limit on a name's
//! length, a 512 KB record name and 58,000 three-letter fields fit in about
//! 1 MiB of source and asked for about 30 GB of accessor names, allocated
//! in the compiler's pre-pass and in the type checker's `register_record`
//! before any instruction budget could count a thing.
//!
//! Two budgets close that, here in the parser, so that every consumer —
//! the compiler, the type checker, `twig-module-driver`, the LSP crates —
//! is covered by the one place they all go through:
//!
//! | Budget | Limit | Enforced | Why this number |
//! |--------|-------|----------|-----------------|
//! | [`MAX_NAME_BYTES`] | 256 | every `NAME` token, in [`crate::parse_to_ast`] before the grammar parser runs | the longest name in the repo's Twig (`cst-parser.tw`) is 33 bytes; 256 matches common identifier caps (C's 63 significant characters, many assemblers' 255) with room to spare |
//! | [`MAX_GENERATED_NAME_BYTES`] | 1 MiB | the formula above, summed over the program, in [`crate::extract_program`] and by each consumer that invents the names, *before* it formats one | no more name bytes than the source budget allows of source; the repo's most record-heavy file (`ast.tw`) generates 467 bytes |
//! | [`MAX_TOKENS`] | 2²⁰ | the lexer's output, in [`crate::parse_to_ast`] before the grammar parser runs | the same number as the IR compiler's CST-items budget: every token is a CST item, so a longer token stream could only ever be refused later |
//!
//! [`MAX_TOKENS`] is also what bounds the parser's peak memory for source
//! that skipped the compiler's 1 MiB source budget (an LSP buffer, a direct
//! `parse_to_ast` caller): the grammar parser's tree is a bounded number of
//! nodes per token, so capping tokens before parsing caps the tree being
//! built, without changing the shared `GrammarParser`.
//!
//! With names capped at 256 bytes the formula is at most 513 bytes per
//! field, so even without [`MAX_GENERATED_NAME_BYTES`] a 1 MiB program
//! could not ask for more than about 30 MB.  The generated-name budget
//! makes the bound 1 MiB and makes it independent of the name cap.

use lexer::token::Token;

use crate::ast_nodes::{Form, Program};
use crate::TwigParseError;

/// Bytes in one `NAME` token (an identifier, a symbol, a record, field or
/// variant name).
pub const MAX_NAME_BYTES: usize = 256;

/// Bytes of names the toolchain invents from record and union declarations
/// (predicates and accessors), summed over a program.
pub const MAX_GENERATED_NAME_BYTES: usize = 1 << 20;

/// Tokens the lexer may hand the grammar parser.
pub const MAX_TOKENS: usize = 1 << 20;

fn breach(which: &str, limit: usize, line: usize, column: usize) -> TwigParseError {
    TwigParseError {
        message: format!("{which} exceeds the limit of {limit}"),
        line,
        column,
    }
}

/// Refuse a token stream longer than [`MAX_TOKENS`], or a `NAME` token
/// longer than [`MAX_NAME_BYTES`].  One pass, before the grammar parser.
pub fn check_tokens(tokens: &[Token]) -> Result<(), TwigParseError> {
    if tokens.len() > MAX_TOKENS {
        let t = &tokens[MAX_TOKENS];
        return Err(breach("tokens", MAX_TOKENS, t.line, t.column));
    }
    for t in tokens {
        check_name_token(t)?;
    }
    Ok(())
}

/// Refuse `t` if it is a `NAME` longer than [`MAX_NAME_BYTES`].  Public
/// so a consumer of a hand-built tree (the IR compiler's tree walk) applies
/// the same rule to tokens that never came through the lexer.
pub fn check_name_token(t: &Token) -> Result<(), TwigParseError> {
    if t.value.len() > MAX_NAME_BYTES && t.effective_type_name() == "NAME" {
        return Err(breach("name bytes", MAX_NAME_BYTES, t.line, t.column));
    }
    Ok(())
}

/// The bytes of names a record (or one union variant) called `name` with
/// fields of the given lengths makes the toolchain invent: the predicate
/// `name?` and one accessor `name-field` per field.  Saturating, so a
/// hostile count cannot wrap.
///
/// ```
/// use twig_parser::generated_name_bytes;
/// // (record Span (start : int) (end : int)):  "span?" + "span-start" + "span-end"
/// assert_eq!(generated_name_bytes(4, [5, 3]), 5 + 10 + 8);
/// ```
pub fn generated_name_bytes(name_len: usize, field_lens: impl IntoIterator<Item = usize>) -> usize {
    let per_name = name_len.saturating_add(1);
    field_lens
        .into_iter()
        .fold(per_name, |total, field| total.saturating_add(per_name).saturating_add(field))
}

/// A running total against [`MAX_GENERATED_NAME_BYTES`]: each consumer
/// that invents the names keeps one and calls [`Self::add`] *before*
/// formatting a declaration's names.
#[derive(Debug, Default)]
pub struct GeneratedNameBudget {
    bytes: usize,
}

impl GeneratedNameBudget {
    /// Charge one declaration's [`generated_name_bytes`]; refuse it, at
    /// `(line, column)`, if the program total goes over the budget.
    pub fn add(&mut self, bytes: usize, line: usize, column: usize) -> Result<(), TwigParseError> {
        self.bytes = self.bytes.saturating_add(bytes);
        if self.bytes > MAX_GENERATED_NAME_BYTES {
            return Err(breach("generated name bytes", MAX_GENERATED_NAME_BYTES, line, column));
        }
        Ok(())
    }
}

/// Check a whole program's records and unions against
/// [`MAX_GENERATED_NAME_BYTES`], without formatting a name.  Run by
/// [`crate::extract_program`], and by the type checker for a `Program` it
/// did not extract itself.
pub fn check_generated_names(program: &Program) -> Result<(), TwigParseError> {
    let mut budget = GeneratedNameBudget::default();
    for form in &program.forms {
        match form {
            Form::RecordDef(r) => {
                let bytes = generated_name_bytes(r.name.len(), r.fields.iter().map(|f| f.name.len()));
                budget.add(bytes, r.line, r.column)?;
            }
            Form::UnionDef(u) => {
                for v in &u.variants {
                    let bytes =
                        generated_name_bytes(v.name.len(), v.fields.iter().map(|f| f.name.len()));
                    budget.add(bytes, u.line, u.column)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
