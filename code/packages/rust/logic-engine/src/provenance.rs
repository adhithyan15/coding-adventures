//! # Provenance — first-class citations for every clause.
//!
//! Addresses ADJ46 awkwardness item **A2**: "Provenance is not a
//! clause field." Each prior, contribution, and joint contribution
//! now carries a [`Provenance`] value that names the authority the
//! clause is grounded in. The proof DAG threads this through every
//! step's [`crate::DerivationOrigin`] so an audit-trail consumer can
//! print, for every contribution that fired, the exact citation it
//! came from — without consulting a side-table.
//!
//! The shape is intentionally small. ADJ44's recursive-rulebook
//! derivation produces a richer object; this is the minimum the
//! engine needs to be able to *cite* a clause back to its origin.
//! Richer provenance (recursion depth, derivation tree, content-match
//! verification status) lives at the rulebook-acquisition layer and
//! is referenced from here by `source` + `locator`.

/// Citation + trust signal for a single clause.
///
/// Designed so the common case is a one-liner — `Provenance::cited("AHA 2021
/// chest-pain guideline §3.2")` — while still carrying enough
/// structure that an audit reader can sort, filter, and aggregate
/// across clauses by trust tier.
///
/// `Provenance::unattributed()` is the explicit sentinel for "this
/// clause has no citation." It is preserved as a value rather than
/// `None` so the audit trail can distinguish "no one bothered to
/// attribute" from "the modeller explicitly committed to a fact
/// without external grounding" at proof-display time.
#[derive(Debug, Clone, PartialEq)]
pub struct Provenance {
    /// The **verbatim span** this clause rests on — the bytes an auditor
    /// re-finds at the locator (`ADJ-REASON-MATH.md` §E.3).
    ///
    /// # Why this is separate from `source`
    ///
    /// Until now the quoted span was *stuffed into* `source` by convention.
    /// That conflates two different things: the **quotation** (bytes that must
    /// appear at the locator, unchanged) and the **citation label** (how a
    /// human names the document). A verifier needs the first; a reader needs
    /// the second. One string cannot be checked as both.
    pub quote: Quote,
    /// A content hash of the source document as captured **at ingest time**.
    ///
    /// Verification runs against this snapshot, not against the live web, and
    /// that is not a performance choice — it is what makes the check mean
    /// anything. A verbatim check against a live URL is decided by whoever
    /// controls that URL *at verification time*, so anyone able to publish
    /// there (a compromised source, a DNS hijack, or just an ordinary content
    /// edit) could make a fabricated quote verify. Pinning inverts that: the
    /// snapshot is fixed when the fact enters, and later divergence becomes
    /// **evidence of drift** rather than a passing grade.
    ///
    /// `None` means no snapshot was captured, which is a reason to report the
    /// step `Unverified` — never `Verified`.
    pub snapshot: Option<ContentHash>,
    /// Human-readable citation. Typically a journal reference, a
    /// guideline name + year, a statute, a clinical trial id, etc.
    /// Empty string only when `trust_tier == TrustTier::Unattributed`.
    pub source: String,
    /// Optional locator within the source — page, section, paragraph,
    /// figure number, court-opinion page-line.
    pub locator: Option<String>,
    /// How much weight a reviewer should place on this clause.
    pub trust_tier: TrustTier,
    /// **Corroborating** citations (ADJ-A9): additional independent sources
    /// that support the *same* clause/LR. Distinct from the engine's
    /// `source_disagreements` machinery, which compares *different* clauses
    /// whose LRs **disagree**; these are co-equal citations for the same
    /// fact, recorded so the audit trail can list every span a reader can
    /// re-fetch. They are **documentary only** — they carry no extra
    /// evidential weight and never enter the LR arithmetic (double-counting
    /// the same fact would inflate posteriors). Empty for the common
    /// single-citation case; defaults to empty everywhere.
    pub corroborations: Vec<Citation>,
}

/// The verbatim span a clause rests on, or an explicit admission that it has
/// not been recorded yet.
///
/// # Why this is an enum and not a `String`
///
/// `ADJ-REASON-MATH.md` §E.3 writes the field as `quote: String` with an
/// `Unmigrated` state alongside. A plain `String` cannot express that state
/// safely — the sentinel would be indistinguishable from a library that
/// genuinely quoted the word "Unmigrated", and, worse, the obvious migration
/// (default `quote` to the `source` label) **fails open**.
///
/// That failure mode is the reason this type exists. `source` labels are short
/// — "NIST", "AQI basics" — and would trivially appear *somewhere* on the cited
/// page. The strongest check in the system would therefore pass while verifying
/// nothing, and report the step as verified. That manufactures confidence,
/// which is the precise failure the whole audit-trail effort exists to prevent,
/// and it would have been the default state of the entire stdlib on day one.
///
/// Making it a closed sum moves "never fail open" from a convention someone
/// must remember into a fact the compiler enforces: you cannot read a quote
/// without deciding what to do about `Unmigrated`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Quote {
    /// The recorded span, and where it sits in the pinned snapshot.
    ///
    /// The payload's fields are **private**, so the only way to obtain one is
    /// [`VerbatimSpan::new`], which enforces the invariant. See that type for
    /// why the check cannot live in a builder.
    Verbatim(VerbatimSpan),
    /// This clause predates the `quote`/`source` split. **Never verifiable.**
    Unmigrated,
}

impl Quote {
    /// The recorded span, if there is one.
    pub fn text(&self) -> Option<&str> {
        match self {
            Quote::Verbatim(v) => Some(v.text()),
            Quote::Unmigrated => None,
        }
    }

    /// Where the span sits in the snapshot, if both are recorded.
    pub fn byte_offset(&self) -> Option<usize> {
        match self {
            Quote::Verbatim(v) => v.byte_offset(),
            Quote::Unmigrated => None,
        }
    }

    /// `true` when this clause carries no checkable span. A verifier that sees
    /// this **must** report `Unverified`; it must not report `Verified`, and it
    /// must not silently skip the step.
    pub fn is_unmigrated(&self) -> bool {
        matches!(self, Quote::Unmigrated)
    }
}

/// A span that is guaranteed, by construction, to be capable of supporting a
/// claim.
///
/// # Why the fields are private
///
/// The first version of this enforced "a span must not be blank" inside the
/// `with_quote` builder. A security review disproved that with a downstream
/// probe crate: `Quote::Verbatim` was a public struct-variant, so a consumer
/// could write `Quote::Verbatim { text: String::new(), .. }` directly and
/// bypass the builder entirely — and an empty span satisfies the verifier's
/// `doc[at..at + text.len()] == text` at **every offset in every document**.
///
/// The builder was never going to be the chokepoint, and the reason matters:
/// **deserialization builds the enum directly**, so a trail read back from disk
/// would reconstruct exactly the value the builder refused to make. That is
/// PR-D2's whole job, which means the hole would have reopened precisely where
/// it does the most damage. Private fields plus one fallible constructor make
/// the invariant hold on every path, including ones not written yet.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VerbatimSpan {
    text: String,
    byte_offset: Option<usize>,
}

impl VerbatimSpan {
    /// Build a span, or `None` if it could not support a claim.
    ///
    /// Rejects text with no **visible** content. `str::trim` alone is not
    /// enough: it follows the Unicode `White_Space` property, which covers
    /// U+00A0 and U+3000 but *not* the zero-width family (U+200B–U+200D,
    /// U+2060, U+FEFF). A zero-width span is invisible in every rendering of
    /// the trail, so a human auditor would see what looks like a blank quote
    /// while the verifier reported a real one — the same manufactured
    /// confidence, in miniature.
    pub fn new(text: impl Into<String>, byte_offset: Option<usize>) -> Option<Self> {
        let text = text.into();
        if !has_visible_content(&text) {
            return None;
        }
        Some(Self { text, byte_offset })
    }

    /// The exact bytes, as captured.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Byte offset of the span within the snapshot document.
    ///
    /// Verification is **anchored** to this offset rather than searching the
    /// document for the text. An unanchored substring search would accept a
    /// quote that appears anywhere — a footnote, a navigation menu, a passage
    /// saying the opposite — so it would confirm the words exist somewhere, not
    /// that they support this clause. `None` means no offset was recorded,
    /// which downgrades the step to `Unverified` rather than falling back to
    /// searching.
    ///
    /// Note what this does **not** promise: a very short span (one or two
    /// characters) is anchored but weak. Anchoring means it must really occur
    /// at that offset, so it is not the universal match a blank span would be —
    /// but it is thin evidence. No arbitrary minimum length is imposed here,
    /// because any threshold would be false precision; instead `adj-verify`
    /// (PR-D2) should surface span length so a reader can judge for themselves.
    pub fn byte_offset(&self) -> Option<usize> {
        self.byte_offset
    }
}

/// `true` if `s` contains at least one character a human would actually see.
fn has_visible_content(s: &str) -> bool {
    s.chars().any(|c| !is_invisible(c))
}

/// Whitespace *plus* the zero-width characters `str::trim` does not treat as
/// whitespace but which render as nothing.
fn is_invisible(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{200B}'..='\u{200D}' | '\u{2060}' | '\u{FEFF}')
}

/// A content-addressed hash of a source document, captured at ingest.
///
/// # Why SHA-256 and not the repo's `hash-functions` crate
///
/// This hash is **tamper-evidence**, not a hash-table index. The threat is an
/// adversary who wants a forged snapshot to verify, so the property required is
/// collision resistance. FNV, DJB2, murmur and SipHash — everything in
/// `hash-functions` — are fast non-cryptographic hashes with no such guarantee;
/// using one here would look like a security control while providing none.
/// `coding_adventures_sha256` is the repo's own zero-dependency implementation,
/// so this stays within the no-third-party rule.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentHash {
    /// Lowercase hex of the SHA-256 digest.
    hex: String,
}

impl ContentHash {
    /// Hash a source document's bytes.
    pub fn of(bytes: &[u8]) -> Self {
        Self {
            hex: coding_adventures_sha256::sha256_hex(bytes),
        }
    }

    /// Reconstruct from a stored hex digest (e.g. read back from a trail).
    ///
    /// Returns `None` unless the input really is a 64-character SHA-256 digest.
    /// Validating here is what makes the type mean something: a `ContentHash`
    /// value is a well-formed digest **by construction**, so the weaker
    /// hash-to-hash comparison below cannot be satisfied by two copies of the
    /// same garbage string read out of the same untrusted trail.
    ///
    /// Case and surrounding whitespace are normalized rather than rejected —
    /// a digest that survived a round trip through a system that upcased it
    /// should read as the same digest, not as permanent, silent drift.
    pub fn from_hex(hex: impl AsRef<str>) -> Option<Self> {
        let h = hex.as_ref().trim().to_ascii_lowercase();
        if h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()) {
            Some(Self { hex: h })
        } else {
            None
        }
    }

    /// The hex digest.
    pub fn as_hex(&self) -> &str {
        &self.hex
    }

    /// `true` iff `bytes` hash to this digest — i.e. the document on hand is
    /// byte-for-byte the one that was captured at ingest.
    ///
    /// **This, not `==`, is verification.** Comparing two `ContentHash` values
    /// compares two hex strings, which says only that a trail agrees with
    /// itself; it never touches the document. A verifier must re-hash the bytes
    /// it actually has, which is what this does.
    pub fn matches(&self, bytes: &[u8]) -> bool {
        Self::of(bytes).hex == self.hex
    }
}

/// One corroborating citation (ADJ-A9). Both fields are required: a
/// corroboration with no locator is not re-checkable, and the whole point is
/// that an auditor can re-fetch the span. Co-equal with the clause's primary
/// `source`/`locator`; inherits the clause's [`TrustTier`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Citation {
    /// Human-readable citation span — the verbatim quote or reference.
    pub source: String,
    /// Where the span can be re-fetched — URL, page, section.
    pub locator: String,
}

impl Citation {
    /// Construct a corroborating citation from a source span + locator.
    pub fn new(source: impl Into<String>, locator: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            locator: locator.into(),
        }
    }
}

/// A coarse trust-tier rank. The variants are ordered from highest
/// (consensus across multiple authoritative sources) to lowest
/// (modeller intuition with no external grounding).
///
/// `Eq + Ord` is derived deliberately: tools that aggregate proofs
/// can sort by trust tier without re-implementing the ordering, and
/// the order in the source determines the order — `Consensus <
/// Authoritative < Empirical < Inferred < Unattributed` by `PartialOrd`,
/// which corresponds to "more trustworthy is smaller" so a min-tier
/// reduction picks the strongest provenance in a proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TrustTier {
    /// Multi-society / cross-authority consensus. E.g. AHA + ESC
    /// + ACC agreeing on a treatment recommendation.
    Consensus,
    /// Single authoritative source — a guideline, a statute, a
    /// peer-reviewed meta-analysis. The "default reasonable" tier
    /// for most clauses.
    Authoritative,
    /// Cohort study, case series, retrospective analysis. Lower
    /// power than `Authoritative` but still empirically grounded.
    Empirical,
    /// Derived by an LLM or by a knowledge engineer from training
    /// data, not from an external citation. ADJ44's recursive
    /// rulebook derivation marks rules at this tier when no
    /// external authority has been resolved.
    Inferred,
    /// No citation. The clause is asserted by the modeller without
    /// external grounding. Distinct from "citation not yet
    /// recorded" — the modeller explicitly committed to no source.
    Unattributed,
}

impl Provenance {
    /// Construct a `Provenance` with the classic three fields explicit.
    ///
    /// The quote is [`Quote::Unmigrated`] and the snapshot is `None`: every
    /// existing caller predates the §E.3 split, and the honest record of that
    /// is "no checkable span was captured", not a guess. A verifier reports
    /// these `Unverified` — never `Verified`. Use [`with_quote`](Self::with_quote)
    /// to record a real span.
    pub fn new(source: impl Into<String>, locator: Option<String>, trust_tier: TrustTier) -> Self {
        Self {
            quote: Quote::Unmigrated,
            snapshot: None,
            source: source.into(),
            locator,
            trust_tier,
            corroborations: Vec::new(),
        }
    }

    /// Record the verbatim span this clause rests on, anchored at `byte_offset`
    /// within the document whose content hashes to `snapshot`.
    ///
    /// Both anchors are optional at the type level because real libraries are
    /// migrated incrementally — but a verifier treats a missing offset or a
    /// missing snapshot as `Unverified`, so partial migration never reads as
    /// success.
    pub fn with_quote(
        mut self,
        text: impl Into<String>,
        byte_offset: Option<usize>,
        snapshot: Option<ContentHash>,
    ) -> Self {
        // A BLANK SPAN IS NOT A WEAKER QUOTE — IT IS A UNIVERSAL ONE, matching
        // at every offset in every document. `VerbatimSpan::new` is the single
        // place that decides; a span it refuses is recorded as the absence it
        // is, rather than as a check that would always pass.
        self.quote = match VerbatimSpan::new(text, byte_offset) {
            Some(v) => Quote::Verbatim(v),
            None => Quote::Unmigrated,
        };
        self.snapshot = snapshot;
        self
    }

    /// Record a span **against the document it came from** — the safer path,
    /// and the one new grounded libraries should use.
    ///
    /// [`with_quote`](Self::with_quote) takes the text, the offset and the
    /// snapshot as three independent values, so nothing stops them describing
    /// three different documents: an offset that does not point at the text, or
    /// a hash of something else entirely. This constructor takes the document
    /// itself and *derives* the other two, which makes that disagreement
    /// unrepresentable.
    ///
    /// Returns `None` — rather than panicking — when the range does not name a
    /// real span: past the end, not on a UTF-8 character boundary, arithmetic
    /// overflow, or blank text. A caller that cannot record a span must find
    /// that out, not discover it later as a slicing panic inside a verifier.
    pub fn with_quote_in(mut self, doc: &str, byte_offset: usize, len: usize) -> Option<Self> {
        let end = byte_offset.checked_add(len)?;
        let text = doc.get(byte_offset..end)?;
        self.quote = Quote::Verbatim(VerbatimSpan::new(text, Some(byte_offset))?);
        self.snapshot = Some(ContentHash::of(doc.as_bytes()));
        Some(self)
    }

    /// The common case: a single-line citation at
    /// [`TrustTier::Authoritative`]. Used for the bulk of
    /// guideline-derived clauses.
    pub fn cited(source: impl Into<String>) -> Self {
        Self::new(source, None, TrustTier::Authoritative)
    }

    /// A citation at consensus tier.
    pub fn consensus(source: impl Into<String>) -> Self {
        Self::new(source, None, TrustTier::Consensus)
    }

    /// Empirical observation; the bulk of LR magnitudes in the
    /// HEART-score / Panju 1998 literature land here.
    pub fn empirical(source: impl Into<String>) -> Self {
        Self::new(source, None, TrustTier::Empirical)
    }

    /// No external grounding. Use this when adapting a rulebook
    /// from a non-citing source or when the modeller is explicitly
    /// committing to a fact on their own authority.
    pub fn unattributed() -> Self {
        Self::new(String::new(), None, TrustTier::Unattributed)
    }

    /// Convenience: attach a locator to an existing provenance.
    /// Returns a new value, leaves the receiver unchanged.
    pub fn with_locator(mut self, locator: impl Into<String>) -> Self {
        self.locator = Some(locator.into());
        self
    }

    /// ADJ-A9: append a corroborating citation (a co-equal source for the
    /// *same* fact). Documentary only — does not affect the LR arithmetic.
    /// Returns a new value, leaves the receiver unchanged.
    pub fn with_corroboration(
        mut self,
        source: impl Into<String>,
        locator: impl Into<String>,
    ) -> Self {
        self.corroborations.push(Citation::new(source, locator));
        self
    }
}

impl Default for Provenance {
    /// The default is [`Provenance::unattributed`], not a panic.
    /// Defaulting to "no citation" is the right behaviour for tests
    /// and small examples where citing every clause would be noise;
    /// real rulebooks should set this explicitly via the constructor.
    fn default() -> Self {
        Self::unattributed()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cited_lands_at_authoritative_tier() {
        let p = Provenance::cited("AHA 2021 chest-pain guideline");
        assert_eq!(p.trust_tier, TrustTier::Authoritative);
        assert!(p.locator.is_none());
        assert_eq!(p.source, "AHA 2021 chest-pain guideline");
    }

    #[test]
    fn unattributed_has_empty_source() {
        let p = Provenance::unattributed();
        assert_eq!(p.trust_tier, TrustTier::Unattributed);
        assert!(p.source.is_empty());
    }

    #[test]
    fn trust_tier_ord_puts_consensus_lowest_value() {
        // "Lower variant => stronger trust" is the convention so
        // that min(...) over a proof's contributions picks the
        // strongest provenance in evidence.
        assert!(TrustTier::Consensus < TrustTier::Authoritative);
        assert!(TrustTier::Authoritative < TrustTier::Empirical);
        assert!(TrustTier::Empirical < TrustTier::Inferred);
        assert!(TrustTier::Inferred < TrustTier::Unattributed);
    }

    #[test]
    fn with_locator_threads_immutably() {
        let p = Provenance::cited("ACOG bulletin").with_locator("§4.2");
        assert_eq!(p.locator.as_deref(), Some("§4.2"));
        // Original `cited` still has no locator if reused:
        let q = Provenance::cited("ACOG bulletin");
        assert!(q.locator.is_none());
    }

    #[test]
    fn default_is_unattributed() {
        assert_eq!(Provenance::default(), Provenance::unattributed());
    }

    #[test]
    fn corroborations_default_empty_and_append_in_order() {
        // ADJ-A9: the common case carries no corroborations.
        let p = Provenance::cited("Tunkel 2004");
        assert!(p.corroborations.is_empty());

        // Two corroborating citations accumulate in call order, each carrying
        // a required locator, and the primary citation is untouched.
        let q = Provenance::cited("Tunkel 2004")
            .with_locator("§3.2")
            .with_corroboration("van de Beek 2006", "https://nejm.org/a")
            .with_corroboration("Brouwer 2010", "https://asm.org/b");
        assert_eq!(q.source, "Tunkel 2004");
        assert_eq!(q.locator.as_deref(), Some("§3.2"));
        assert_eq!(q.corroborations.len(), 2);
        assert_eq!(
            q.corroborations[0],
            Citation::new("van de Beek 2006", "https://nejm.org/a")
        );
        assert_eq!(q.corroborations[1].source, "Brouwer 2010");
        assert_eq!(q.corroborations[1].locator, "https://asm.org/b");
        // Corroborations are documentary: trust tier is unchanged.
        assert_eq!(q.trust_tier, TrustTier::Authoritative);
    }
}
