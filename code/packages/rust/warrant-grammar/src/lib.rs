//! # Warrant grammar artifacts (VER04 P006a)
//!
//! The canonical `.tokens` and `.grammar` files are compiled by the existing
//! grammar-tools emitter into ordinary Rust data. These constructors do no
//! file I/O and accept no untrusted source. Their byte-level regeneration and
//! syntax corpus are checked separately.
//!
//! The additive [`source`] module implements P006b1's bounded source admission
//! and CRLF/original-byte relation. A protected lexer/parser, significant-layout
//! origins and parser quotas remain P006b2/b3; admission does not establish
//! syntax/type/proof checking.

#[rustfmt::skip]
mod _parser;
#[rustfmt::skip]
mod _tokens;

pub mod layout;
pub mod parser;
pub mod source;

pub use _parser::parser_grammar;
pub use _tokens::token_grammar;
