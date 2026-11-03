//! # Warrant grammar artifacts (VER04 P006a)
//!
//! The canonical `.tokens` and `.grammar` files are compiled by the existing
//! grammar-tools emitter into ordinary Rust data. These constructors do no
//! file I/O and accept no untrusted source. Their byte-level regeneration and
//! syntax corpus are checked separately.
//!
//! A protected source lexer/parser, byte spans and quotas belong to P006b;
//! returning grammar data does not establish those guarantees or type/proof
//! checking. Consumers must not mistake these artifacts for that front end.

#[rustfmt::skip]
mod _parser;
#[rustfmt::skip]
mod _tokens;

pub use _parser::parser_grammar;
pub use _tokens::token_grammar;
