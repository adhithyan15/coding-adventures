//! # Source map — where a token *really* came from.
//!
//! Once a token can arrive from an included file or a macro body, its
//! `line:column` stops being enough to identify it. Two files both have a
//! line 12.
//!
//! The obvious fix is to widen the token. We do not do that, for a concrete
//! reason: `lexer::token::Token` is shared by **136 crates** in this
//! repository. Adding a `file` field to it would be a repo-wide change to
//! serve a feature two frontends need. So the mapping lives beside the token
//! stream instead:
//!
//! ```text
//!   tokens[i]        keeps a PRESUMED line/column — the position in whatever
//!                    file's text it came from.  Consumers that do not care
//!                    about inclusion (every existing parser) see sensible
//!                    numbers and need no changes at all.
//!
//!   map.locus(i)     resolves to (FileId, line, column, expansion chain) for
//!                    consumers that do care -- diagnostics that want to say
//!                    "in expansion of FOO, from bar.h:12, included from
//!                    main.c:3".
//! ```
//!
//! This is the design GCC's line maps and LLVM's `SourceManager` both use, and
//! for the same reason.
//!
//! ## Why the expansion chain is interned
//!
//! A `Locus` that *owned* its expansion chain would make the map
//! `O(tokens × expansion_depth)`. With a macro-depth bound of 200, a token
//! budget of N would admit 200 N chain entries — so an operator who sets the
//! token cap believing it bounds memory would under-count by up to 200×.
//!
//! Instead each expansion is interned once and names its parent:
//!
//! ```text
//!   ExpansionId(3) ─parent─▶ ExpansionId(1) ─parent─▶ (none)
//!         ▲                        ▲
//!    many tokens              many tokens
//! ```
//!
//! which makes the map `O(tokens + expansions)`. This is load-bearing for the
//! memory bounds in [`crate::bounds`], not an optimisation.

use std::num::NonZeroU32;

/// An opaque handle to a source file.
///
/// Deliberately opaque and constructible only inside this crate: a `FileId` is
/// handed to dialects (see [`crate::dialect::Dialect::lex`]), and a
/// transparent newtype over an integer would let a dialect mint one naming a
/// *different* file, silently misattributing provenance across the whole map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(NonZeroU32);

impl FileId {
    /// Mint a `FileId`. Crate-internal on purpose — see the type docs.
    pub(crate) fn new(raw: u32) -> FileId {
        FileId(NonZeroU32::new(raw.saturating_add(1)).expect("raw+1 is nonzero"))
    }

    /// Index form, for a `SourceFs` that stores files in a `Vec`.
    #[must_use]
    pub fn index(self) -> usize {
        (self.0.get() - 1) as usize
    }
}

/// An interned macro expansion: which macro, expanded where, nested in what.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpansionId(u32);

#[derive(Debug, Clone)]
struct Expansion {
    /// The macro's name, for "in expansion of FOO".
    name: String,
    /// Where the invocation appeared.
    at: Position,
    /// The expansion this one happened inside, if any.
    parent: Option<ExpansionId>,
}

/// A concrete position in one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub file: FileId,
    /// 1-based.
    pub line: u32,
    /// 1-based.
    pub column: u32,
}

/// The full provenance of one emitted token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Locus {
    /// Where the token's text physically sits.
    pub position: Position,
    /// The innermost expansion it came out of, if any. Walk
    /// [`SourceMap::expansion_parent`] to recover the whole chain.
    pub expansion: Option<ExpansionId>,
}

/// Per-token provenance for one preprocessing run, plus the interned
/// expansion arena.
///
/// The `Locus` vector is **positional**: `locus(i)` describes `tokens[i]`. That
/// is only valid for a consumer that does not reorder or synthesise tokens
/// between the engine and the parser, which is why the engine must be the last
/// `post_tokenize` hook. [`SourceMap::check_len`] turns a violation into a
/// hard error instead of silent misattribution.
#[derive(Debug, Default, Clone)]
pub struct SourceMap {
    loci: Vec<Locus>,
    expansions: Vec<Expansion>,
}

impl SourceMap {
    #[must_use]
    pub fn new() -> SourceMap {
        SourceMap::default()
    }

    /// Record provenance for the next token.
    pub fn push(&mut self, locus: Locus) {
        self.loci.push(locus);
    }

    /// Provenance of the `i`th token, if recorded.
    #[must_use]
    pub fn locus(&self, i: usize) -> Option<Locus> {
        self.loci.get(i).copied()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.loci.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.loci.is_empty()
    }

    /// Intern one expansion, returning its id.
    pub fn intern_expansion(
        &mut self,
        name: impl Into<String>,
        at: Position,
        parent: Option<ExpansionId>,
    ) -> ExpansionId {
        let id = ExpansionId(self.expansions.len() as u32);
        self.expansions.push(Expansion { name: name.into(), at, parent });
        id
    }

    /// The expansion this one was nested inside, if any. Walk this to build
    /// "in expansion of A, in expansion of B, …".
    #[must_use]
    pub fn expansion_parent(&self, id: ExpansionId) -> Option<ExpansionId> {
        self.expansions.get(id.0 as usize).and_then(|e| e.parent)
    }

    /// The macro name and invocation site for an expansion.
    #[must_use]
    pub fn expansion_site(&self, id: ExpansionId) -> Option<(&str, Position)> {
        self.expansions.get(id.0 as usize).map(|e| (e.name.as_str(), e.at))
    }

    /// Number of interned expansions. With [`SourceMap::len`] this is the whole
    /// memory story: the map is `O(tokens + expansions)`, never
    /// `O(tokens × depth)`.
    #[must_use]
    pub fn expansion_count(&self) -> usize {
        self.expansions.len()
    }

    /// Verify the map still lines up with the token stream it describes.
    ///
    /// Called where the map is consumed. A mismatch means something reordered
    /// or synthesised tokens after the engine ran, which would make every
    /// diagnostic point at the wrong place — so it is an error, not a warning.
    pub fn check_len(&self, tokens: usize) -> Result<(), LenMismatch> {
        if self.loci.len() == tokens {
            Ok(())
        } else {
            Err(LenMismatch { map: self.loci.len(), tokens })
        }
    }
}

/// The source map no longer describes the token stream beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LenMismatch {
    pub map: usize,
    pub tokens: usize,
}

impl std::fmt::Display for LenMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "source map describes {} tokens but the stream has {} — something \
             reordered or synthesised tokens after preprocessing, so every \
             location would be misattributed",
            self.map, self.tokens
        )
    }
}

impl std::error::Error for LenMismatch {}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(file: FileId, line: u32) -> Position {
        Position { file, line, column: 1 }
    }

    #[test]
    fn file_ids_are_distinct_and_round_trip_their_index() {
        let a = FileId::new(0);
        let b = FileId::new(1);
        assert_ne!(a, b);
        assert_eq!(a.index(), 0);
        assert_eq!(b.index(), 1);
    }

    #[test]
    fn expansion_chain_is_walkable_to_the_root() {
        let mut map = SourceMap::new();
        let f = FileId::new(0);
        let outer = map.intern_expansion("OUTER", pos(f, 1), None);
        let inner = map.intern_expansion("INNER", pos(f, 2), Some(outer));

        assert_eq!(map.expansion_parent(inner), Some(outer));
        assert_eq!(map.expansion_parent(outer), None);
        assert_eq!(map.expansion_site(inner).unwrap().0, "INNER");
        assert_eq!(map.expansion_site(outer).unwrap().0, "OUTER");
    }

    #[test]
    fn many_tokens_share_one_interned_expansion() {
        // This is the memory property the bounds depend on: 1000 tokens from
        // one expansion must cost ONE expansion entry, not 1000 chains.
        let mut map = SourceMap::new();
        let f = FileId::new(0);
        let e = map.intern_expansion("BIG", pos(f, 1), None);
        for line in 0..1000 {
            map.push(Locus { position: pos(f, line), expansion: Some(e) });
        }
        assert_eq!(map.len(), 1000);
        assert_eq!(map.expansion_count(), 1, "chain must be interned, not owned per token");
    }

    #[test]
    fn check_len_rejects_a_stream_that_no_longer_matches() {
        let mut map = SourceMap::new();
        let f = FileId::new(0);
        map.push(Locus { position: pos(f, 1), expansion: None });

        assert!(map.check_len(1).is_ok());
        // A later hook inserted a token: every locus after it now describes
        // the wrong token, so this must fail loudly.
        let err = map.check_len(2).unwrap_err();
        assert_eq!(err.map, 1);
        assert_eq!(err.tokens, 2);
    }
}
