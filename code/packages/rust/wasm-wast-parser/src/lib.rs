//! # wasm-wast-parser
//!
//! Parses the WebAssembly **text** format — both plain `.wat` modules and
//! the official spec testsuite's `.wast` script dialect (`assert_return`,
//! `assert_trap`, `invoke`, and friends) — into [`wasm_types::WasmModule`]
//! and a sequence of [`script::Directive`]s.
//!
//! ## Why this crate exists
//!
//! Every other WASM crate in this repo (`wasm-module-parser`,
//! `wasm-module-encoder`) speaks the **binary** format only. The official
//! WebAssembly spec testsuite ships almost entirely as `.wast` text files —
//! this repo cannot run it without a text-format front end, and none
//! existed. See `code/specs/W05-wasm-conformance-harness.md` for the full
//! design.
//!
//! ## Pipeline
//!
//! ```text
//! source text
//!   │  tokenizer::tokenize
//!   ▼
//! flat token stream
//!   │  module::parse_module        (a single `(module ...)` form)
//!   │  script::parse_script        (a whole .wast file: modules + directives)
//!   ▼
//! wasm_types::WasmModule  +  Vec<script::Directive>
//! ```
//!
//! `module::parse_module` does the real work: flattening folded
//! instructions, resolving symbolic identifiers (`$name`) to numeric
//! indices per their own scope, deduplicating implicit function types, and
//! **encoding** each function body straight to the same raw WASM bytecode
//! `wasm-module-parser` would have produced from a binary file — this
//! crate produces exactly the `WasmModule` shape every other WASM crate
//! already consumes, not a parallel text-specific AST.

pub mod module;
pub mod numeric;
pub mod script;
pub mod sexpr;
pub mod tokenizer;

pub use module::parse_module;
pub use script::{parse_script, Directive};

/// Every error this crate can produce, carrying a byte offset into the
/// source for error messages. Distinct from
/// [`wasm_module_parser::WasmParseError`] on purpose — a harness grading
/// `assert_malformed` needs to tell "our text parser rejected this" apart
/// from "our binary parser rejected this."
#[derive(Debug, Clone, PartialEq)]
pub enum WastParseError {
    UnterminatedBlockComment { pos: usize },
    UnterminatedString { pos: usize },
    InvalidEscape { pos: usize },
    InvalidUtf8 { pos: usize },
    UnexpectedByte { pos: usize, byte: u8 },
    UnexpectedEof,
    UnexpectedToken { pos: usize, found: String, expected: &'static str },
    UnknownInstruction { pos: usize, name: String },
    UnknownIdentifier { pos: usize, name: String, space: &'static str },
    DuplicateIdentifier { pos: usize, name: String, space: &'static str },
    InvalidNumericLiteral { pos: usize, text: String },
    InvalidNumericLiteralForType { pos: usize, text: String, ty: &'static str },
    /// Nested `(...)` forms deeper than [`sexpr::MAX_NESTING_DEPTH`] — a
    /// clean, catchable error instead of a hard stack overflow (an abort,
    /// not a `panic!`, and NOT catchable by any Rust code) on adversarially
    /// deep input like `((((((...))))))`.
    TooDeeplyNested { pos: usize },
    /// A `func` gives BOTH an explicit `(type $sig)` reference AND its own
    /// literal `(param ...)` forms, and the two disagree in arity. A real
    /// `.wat` file never does this (when both are given, the inline forms
    /// are purely for naming and must already match `$sig` exactly), and
    /// downstream code (`build_func`'s local-index computation) trusts
    /// that invariant to keep every local index within the function's
    /// real local array — a security review found that trusting a
    /// mismatched arity here reaches a raw, unchecked `Vec` index in
    /// `wasm-execution`'s `local.get`/`local.set`/`local.tee` handlers, a
    /// real crash (not memory-unsafe, but a real DoS) once the module is
    /// actually run. Rejecting the mismatch here, at parse time, is both
    /// the spec-correct behavior (the official text-format grammar
    /// requires the two to agree) and the fix that keeps every later
    /// local-index computation sound by construction, rather than
    /// patching each place that could otherwise be fooled by it.
    TypeUseParamCountMismatch { pos: usize, declared: usize, referenced: usize },
    /// A `(module binary "...")` **directive** (not `assert_malformed`'s
    /// deliberately-broken bytes, which stay a raw
    /// [`script::ModuleSource::Binary`] and are graded, never routed
    /// through this error) failed to decode as a real `.wasm` binary via
    /// `wasm-module-parser`.
    EmbeddedBinaryModuleError { pos: usize, message: String },
    /// A `(@...)` **annotation** form (WAT's custom out-of-band tooling
    /// metadata syntax — source maps, etc.) whose id is empty: bare `(@)`,
    /// `(@ )`, `(@ x)` (whitespace between `@` and what should be the id
    /// text), or `(@"")` (an adjacent but empty quoted id). Per the real
    /// spec grammar, the id must immediately follow `@` with no
    /// intervening whitespace, and must be non-empty — see
    /// `sexpr::strip_annotations`'s own doc comment for the full
    /// annotation-skipping design this error is one small part of.
    EmptyAnnotationId { pos: usize },
    /// A raw (unescaped) control character or DEL (byte `< 0x20` or
    /// `0x7F`) appeared literally inside a `"..."` string literal. WAT's
    /// own `stringchar` grammar requires such bytes to be written as an
    /// escape (`\t`, `\n`, or the raw-byte `\XX` hex form) — a literal
    /// unescaped newline, tab, or NUL inside a string is malformed. Only
    /// bytes the tokenizer sees OUTSIDE of any `\`-escape trigger this;
    /// `"\00"`/`"\0a"` (the hex-byte escape) are unaffected, since that's
    /// exactly the mechanism `assert_malformed` fixtures use to embed
    /// otherwise-illegal bytes on purpose.
    IllegalStringCharacter { pos: usize, byte: u8 },
    /// A memory instruction's `align=N` immediate whose `N` is not a power
    /// of two (including `N = 0`) -- e.g. `i32.load align=7`. The real
    /// spec text-format grammar (`interpreter/text/parser.mly`'s
    /// `ALIGN_EQ_NAT` rule in the upstream reference implementation)
    /// requires `align=N` to name a power of two, since it's stored in the
    /// binary encoding as `log2(N)`, not `N` itself -- there is no bit
    /// pattern for "align to 7 bytes." This is a TEXT-FORMAT syntax rule
    /// (caught here, at parse time), distinct from `align.wast`'s sibling
    /// `assert_invalid` cases, which use a syntactically valid power-of-two
    /// `align=N` that's simply larger than the instruction's natural
    /// access width (a semantic rule `wasm-validator` checks instead, well
    /// after this parser has already produced `log2(N)`). Before this
    /// variant existed, `parse_memarg` computed `align_log2` via
    /// `n.trailing_zeros()` unconditionally: `align=0` produced
    /// `trailing_zeros() == 32` (silently corrupting the align byte with a
    /// nonsense log2), and `align=7` produced `trailing_zeros() == 0`
    /// (silently treated as `align=1`, which is always <= any natural
    /// alignment, so the module parsed and validated cleanly instead of
    /// being rejected) -- both hid a real malformed-input case as a
    /// falsely-valid module.
    InvalidAlignment { pos: usize, value: u32 },
}

impl std::fmt::Display for WastParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WastParseError::UnterminatedBlockComment { pos } => {
                write!(f, "unterminated block comment starting at byte {pos}")
            }
            WastParseError::UnterminatedString { pos } => {
                write!(f, "unterminated string literal starting at byte {pos}")
            }
            WastParseError::InvalidEscape { pos } => write!(f, "invalid string escape at byte {pos}"),
            WastParseError::InvalidUtf8 { pos } => write!(f, "invalid UTF-8 at byte {pos}"),
            WastParseError::UnexpectedByte { pos, byte } => {
                write!(f, "unexpected byte {byte:#04x} at offset {pos}")
            }
            WastParseError::UnexpectedEof => write!(f, "unexpected end of input"),
            WastParseError::UnexpectedToken { pos, found, expected } => {
                write!(f, "at byte {pos}: expected {expected}, found {found:?}")
            }
            WastParseError::UnknownInstruction { pos, name } => {
                write!(f, "at byte {pos}: unknown instruction {name:?}")
            }
            WastParseError::UnknownIdentifier { pos, name, space } => {
                write!(f, "at byte {pos}: unknown {space} identifier {name:?}")
            }
            WastParseError::DuplicateIdentifier { pos, name, space } => {
                write!(f, "at byte {pos}: duplicate {space} identifier {name:?}")
            }
            WastParseError::InvalidNumericLiteral { pos, text } => {
                write!(f, "at byte {pos}: invalid numeric literal {text:?}")
            }
            WastParseError::InvalidNumericLiteralForType { pos, text, ty } => {
                write!(f, "at byte {pos}: {text:?} is not a valid {ty} literal")
            }
            WastParseError::TooDeeplyNested { pos } => {
                write!(f, "at byte {pos}: nesting depth exceeds the parser's limit")
            }
            WastParseError::TypeUseParamCountMismatch { pos, declared, referenced } => {
                write!(
                    f,
                    "at byte {pos}: func declares {declared} param(s) inline but its (type ...) reference has {referenced}"
                )
            }
            WastParseError::EmbeddedBinaryModuleError { pos, message } => {
                write!(f, "at byte {pos}: embedded (module binary ...) failed to decode: {message}")
            }
            WastParseError::EmptyAnnotationId { pos } => {
                write!(f, "at byte {pos}: empty annotation id")
            }
            WastParseError::IllegalStringCharacter { pos, byte } => {
                write!(f, "at byte {pos}: illegal unescaped character {byte:#04x} inside a string literal")
            }
            WastParseError::InvalidAlignment { pos, value } => {
                write!(f, "at byte {pos}: alignment {value} must be a power of two")
            }
        }
    }
}

impl std::error::Error for WastParseError {}
