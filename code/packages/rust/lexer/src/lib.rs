//! # Lexer — turning source code into tokens.
//!
//! A **lexer** (also called a tokenizer or scanner) is the first stage of
//! every compiler and interpreter. Its job is simple but essential: take a
//! stream of raw characters and break it into meaningful units called
//! **tokens**.
//!
//! Think of it like reading English. When you see the text `2 + 3`, you
//! don't process it one character at a time — you immediately recognize
//! three units: the number `2`, the operator `+`, and the number `3`.
//! A lexer does the same thing for programming languages.
//!
//! # Why do we need a lexer?
//!
//! Parsers (the next stage) work with tokens, not raw characters. Without
//! a lexer, the parser would have to handle low-level details like:
//!
//! - Skipping whitespace and comments
//! - Recognizing that `42` is one number, not two separate digits
//! - Distinguishing `=` (assignment) from `==` (comparison)
//! - Processing escape sequences in strings (`\n` -> newline)
//!
//! By separating lexing from parsing, each stage has a clear, focused job.
//! The lexer handles characters; the parser handles structure.
//!
//! # Two approaches to lexing
//!
//! This crate provides two lexer implementations:
//!
//! ## 1. Hand-written lexer ([`tokenizer`])
//!
//! A character-by-character lexer with rules baked into Rust code. It reads
//! one character at a time and uses `if`/`match` logic to decide what token
//! to produce. This is the approach used by most production compilers (GCC,
//! Clang, V8, CPython) because it gives maximum control and performance.
//!
//! ## 2. Grammar-driven lexer ([`grammar_lexer`])
//!
//! A lexer that reads its rules from a [`TokenGrammar`](grammar_tools::token_grammar::TokenGrammar)
//! — a structured description of tokens parsed from a `.tokens` file by the
//! [`grammar_tools`] crate. Instead of hard-coding rules, it compiles
//! grammar patterns into regexes and tries them in priority order.
//!
//! The grammar-driven approach is more flexible (change the grammar, change
//! the language) but slightly slower due to regex overhead. It is ideal for
//! prototyping new languages, building syntax highlighters, and educational
//! tools.
//!
//! # Crate structure
//!
//! - **[`token`]** — the `Token` struct, `TokenType` enum, and `LexerError` type.
//! - **[`tokenizer`]** — the hand-written Python lexer.
//! - **[`grammar_lexer`]** — the grammar-driven universal lexer.

pub mod token;
pub mod tokenizer;
pub mod tokenizer_dfa;
pub mod grammar_lexer;
