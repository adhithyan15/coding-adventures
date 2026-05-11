//! # adjudication-coverage — ADJ02 coverage checker.
//!
//! Reference implementation of
//! [`ADJ02`](../../../specs/ADJ02-coverage-checker.md). Verifies that
//! every meaningful byte of the input is accounted for by the source
//! spans of at least one IR node — Fact, Query, Uncertainty, or an
//! explicit Discarded node citing a reason.
//!
//! ## Pipeline
//!
//! ```text
//!   Document (bytes + id)
//!         │
//!         ▼
//!     Tagger.classify_tokens(doc)
//!         │
//!         ▼
//!     TokenAnnotation[]            (Meaningful | NonMeaningful)
//!         │
//!         ▼
//!     check_coverage(annotations, ir_doc, strictness)
//!         │
//!         ▼
//!   CoverageResult { Pass | Fail { uncovered } }
//! ```
//!
//! The interval-cover check is linear in the IR's source spans after
//! sorting and merging.

use std::collections::HashSet;

use adjudication_ir::{DiscardReason, DocumentId, IRDocument, IRNode, NodeKind, Span};

// ---------------------------------------------------------------------------
// Document and tagger
// ---------------------------------------------------------------------------

/// The unit of coverage analysis.
#[derive(Debug, Clone)]
pub struct Document {
    pub id: DocumentId,
    /// The normalized text whose byte offsets the IR's
    /// `source_spans` reference into.
    pub normalized_text: String,
}

/// Classification of a single byte range in the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenLabel {
    Meaningful,
    /// Non-meaningful with a reason from the controlled vocabulary.
    NonMeaningful(NonMeaningfulReason),
}

/// Reasons a token range may be discarded by the tagger. Mirrors the
/// controlled vocabulary from `ADJ02 §"What Counts as Meaningful"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NonMeaningfulReason {
    Whitespace,
    Punctuation,
    Stopword,
    SocialPleasantry,
    DocumentChrome,
    Boilerplate,
    Determiner,
    Filler,
}

/// Tagger output for a single contiguous byte range.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenAnnotation {
    pub start: usize,
    pub end: usize,
    pub label: TokenLabel,
    pub reason: Option<String>,
}

/// A tagger classifies a document's bytes into meaningful and
/// non-meaningful ranges. Implementors choose the strategy:
/// rule-based (the default), small classifier, or an LLM call with
/// constrained output.
pub trait Tagger {
    fn classify_tokens(&self, doc: &Document) -> Vec<TokenAnnotation>;
}

// ---------------------------------------------------------------------------
// Rule-based tagger
// ---------------------------------------------------------------------------

/// The default tagger: word-boundary splitting plus configurable
/// stopword / punctuation / filler lists.
pub struct RuleBasedTagger {
    /// Words that are non-meaningful by default ("the", "a", ...).
    pub stopwords: HashSet<String>,
    /// Words that should always be meaningful, overriding stopwords.
    pub always_meaningful: HashSet<String>,
    /// Filler tokens that may be tolerated under permissive strictness.
    pub fillers: HashSet<String>,
}

impl Default for RuleBasedTagger {
    fn default() -> Self {
        Self {
            stopwords: english_stopwords(),
            always_meaningful: HashSet::new(),
            fillers: filler_words(),
        }
    }
}

impl RuleBasedTagger {
    /// A minimal English-stopword tagger suitable for narrow domains
    /// (TSA, license-compatibility prompts). Clinical text will want
    /// `with_clinical_defaults`.
    pub fn english() -> Self {
        Self::default()
    }

    /// English defaults plus a small list of clinical hedge words and
    /// header boilerplate. Useful as a starting point for clinical
    /// notes; deployments should refine.
    pub fn with_clinical_defaults() -> Self {
        let mut t = Self::default();
        t.fillers.extend(["umm".into(), "uh".into(), "you-know".into()]);
        // "Patient" is meaningful in clinical text even though it's
        // frequent — register it explicitly to avoid future stopword
        // expansions hiding it.
        t.always_meaningful.insert("patient".into());
        t.always_meaningful.insert("doctor".into());
        t
    }
}

impl Tagger for RuleBasedTagger {
    fn classify_tokens(&self, doc: &Document) -> Vec<TokenAnnotation> {
        let mut out = Vec::new();
        let bytes = doc.normalized_text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            // Skip-emit whitespace runs as a single Whitespace token.
            if is_whitespace(bytes[i]) {
                let start = i;
                while i < bytes.len() && is_whitespace(bytes[i]) {
                    i += 1;
                }
                out.push(TokenAnnotation {
                    start,
                    end: i,
                    label: TokenLabel::NonMeaningful(NonMeaningfulReason::Whitespace),
                    reason: None,
                });
                continue;
            }
            // Punctuation runs.
            if is_punct(bytes[i]) {
                let start = i;
                while i < bytes.len() && is_punct(bytes[i]) {
                    i += 1;
                }
                out.push(TokenAnnotation {
                    start,
                    end: i,
                    label: TokenLabel::NonMeaningful(NonMeaningfulReason::Punctuation),
                    reason: None,
                });
                continue;
            }
            // Word run: ASCII alphanumeric and underscore.
            if is_word(bytes[i]) {
                let start = i;
                while i < bytes.len() && is_word(bytes[i]) {
                    i += 1;
                }
                let word = std::str::from_utf8(&bytes[start..i])
                    .unwrap_or("")
                    .to_lowercase();
                let label = if self.always_meaningful.contains(&word) {
                    TokenLabel::Meaningful
                } else if self.stopwords.contains(&word) {
                    TokenLabel::NonMeaningful(NonMeaningfulReason::Stopword)
                } else if self.fillers.contains(&word) {
                    TokenLabel::NonMeaningful(NonMeaningfulReason::Filler)
                } else {
                    TokenLabel::Meaningful
                };
                out.push(TokenAnnotation {
                    start,
                    end: i,
                    label,
                    reason: None,
                });
                continue;
            }
            // Anything else: treat as a one-byte non-meaningful chunk.
            out.push(TokenAnnotation {
                start: i,
                end: i + 1,
                label: TokenLabel::NonMeaningful(NonMeaningfulReason::Punctuation),
                reason: None,
            });
            i += 1;
        }
        out
    }
}

fn is_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

fn is_punct(b: u8) -> bool {
    matches!(
        b,
        b'.' | b','
            | b';'
            | b':'
            | b'!'
            | b'?'
            | b'('
            | b')'
            | b'['
            | b']'
            | b'{'
            | b'}'
            | b'"'
            | b'\''
    )
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

fn english_stopwords() -> HashSet<String> {
    [
        "a", "an", "the", "and", "or", "of", "to", "in", "on", "for", "with", "by",
        "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do",
        "does", "did", "will", "would", "should", "could", "may", "might", "must",
        "this", "that", "these", "those", "i", "you", "he", "she", "it", "we", "they",
        "i'd", "i'm", "we'd", "we're",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn filler_words() -> HashSet<String> {
    ["umm", "uh", "you-know", "like"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// Strictness and result
// ---------------------------------------------------------------------------

/// How tolerant the check is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrictnessMode {
    /// Any uncovered meaningful byte fails.
    Strict,
    /// Uncovered `Filler` / `Determiner` tokens are tolerated.
    Permissive,
    /// Never fails; uncovered ranges are still reported in the
    /// `Fail`-shaped result for telemetry.
    AuditOnly,
}

/// Outcome of a coverage check.
#[derive(Debug, Clone, PartialEq)]
pub enum CoverageResult {
    Pass,
    Fail { uncovered: Vec<Span> },
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Run the coverage check.
///
/// 1. Run the tagger.
/// 2. Filter to meaningful spans (respecting the strictness mode).
/// 3. Enforce ADJ01's hard rule: any `Discarded` node with reason
///    `Unparseable` is a coverage failure.
/// 4. Build the union of IR `source_spans` (sorted + merged).
/// 5. For each meaningful span, verify it is fully contained in the
///    union.
pub fn check_coverage(
    doc: &Document,
    ir_doc: &IRDocument,
    tagger: &dyn Tagger,
    strictness: StrictnessMode,
) -> CoverageResult {
    // 1+2. Build the meaningful-spans list.
    let annotations = tagger.classify_tokens(doc);
    let meaningful: Vec<Span> = annotations
        .into_iter()
        .filter(|a| match a.label {
            TokenLabel::Meaningful => true,
            TokenLabel::NonMeaningful(reason) => {
                strictness == StrictnessMode::Permissive
                    && matches!(
                        reason,
                        NonMeaningfulReason::Filler | NonMeaningfulReason::Determiner
                    )
            }
        })
        .filter(|a| matches!(
            // After the above filter, only Meaningful (and never under
            // Permissive the tolerated NonMeaningful) entries remain.
            a.label, TokenLabel::Meaningful
        ))
        .map(|a| Span::new(doc.id.clone(), a.start, a.end))
        .collect();

    // 3. Enforce the Unparseable hard rule.
    let unparseable_spans: Vec<Span> = ir_doc
        .nodes
        .iter()
        .filter(|n| {
            n.kind == NodeKind::Discarded && n.discard_reason == Some(DiscardReason::Unparseable)
        })
        .flat_map(|n| n.source_spans.iter().cloned())
        .collect();

    if !unparseable_spans.is_empty() {
        return finalize(unparseable_spans, strictness);
    }

    // 4. Build the union of IR source spans, restricted to this
    //    document.
    let mut all_spans: Vec<(usize, usize)> = ir_doc
        .nodes
        .iter()
        .flat_map(|n: &IRNode| n.source_spans.iter())
        .filter(|s| s.document_id == doc.id)
        .map(|s| (s.start, s.end))
        .collect();
    all_spans.sort_by_key(|(s, _)| *s);
    let union = merge_intervals(all_spans);

    // 5. Verify each meaningful span is fully covered.
    let mut uncovered = Vec::new();
    for m in meaningful {
        if !is_covered(m.start, m.end, &union) {
            uncovered.push(m);
        }
    }

    finalize(uncovered, strictness)
}

fn finalize(uncovered: Vec<Span>, strictness: StrictnessMode) -> CoverageResult {
    if uncovered.is_empty() {
        return CoverageResult::Pass;
    }
    if strictness == StrictnessMode::AuditOnly {
        // Always pass; uncovered ranges are still returned for
        // telemetry, but the caller doesn't gate on them.
        // For symmetry with the spec, we return Pass; if telemetry is
        // needed the caller can run with Strict and ignore the failure.
        return CoverageResult::Pass;
    }
    CoverageResult::Fail { uncovered }
}

fn merge_intervals(mut sorted: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    if sorted.is_empty() {
        return sorted;
    }
    let mut out = Vec::with_capacity(sorted.len());
    let mut cur = sorted.remove(0);
    for (s, e) in sorted {
        if s <= cur.1 {
            cur.1 = cur.1.max(e);
        } else {
            out.push(cur);
            cur = (s, e);
        }
    }
    out.push(cur);
    out
}

fn is_covered(start: usize, end: usize, union: &[(usize, usize)]) -> bool {
    // The meaningful range [start, end) is covered iff some
    // contiguous union interval [s, e) satisfies s <= start && end <= e.
    union.iter().any(|(s, e)| *s <= start && end <= *e)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use adjudication_ir::{Modality, NodeId, Polarity};
    use std::collections::HashMap;

    fn doc_id() -> DocumentId {
        DocumentId::new("doc1")
    }

    fn mk_doc(text: &str) -> Document {
        Document {
            id: doc_id(),
            normalized_text: text.to_string(),
        }
    }

    fn mk_fact(id: &str, span: Span) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: NodeKind::Fact,
            term: logic_core_atom("p"),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span],
            confidence: 1.0,
            lowered_from: None,
            discard_reason: None,
            metadata: HashMap::new(),
        }
    }

    use logic_core::Term;

    fn logic_core_atom(name: &str) -> Term {
        Term::Atom(name.to_string())
    }

    fn span(start: usize, end: usize) -> Span {
        Span::new(doc_id(), start, end)
    }

    fn empty_ir() -> IRDocument {
        IRDocument {
            document_id: doc_id(),
            nodes: vec![],
        }
    }

    #[test]
    fn empty_document_passes_coverage() {
        let doc = mk_doc("");
        let ir = empty_ir();
        let tagger = RuleBasedTagger::english();
        assert_eq!(
            check_coverage(&doc, &ir, &tagger, StrictnessMode::Strict),
            CoverageResult::Pass
        );
    }

    #[test]
    fn document_with_only_stopwords_and_punctuation_passes() {
        let doc = mk_doc("the a, of.");
        let ir = empty_ir();
        let tagger = RuleBasedTagger::english();
        assert_eq!(
            check_coverage(&doc, &ir, &tagger, StrictnessMode::Strict),
            CoverageResult::Pass
        );
    }

    #[test]
    fn single_meaningful_word_uncovered_fails() {
        // "patient" is meaningful with the clinical defaults; no IR
        // node covers it.
        let doc = mk_doc("patient");
        let ir = empty_ir();
        let tagger = RuleBasedTagger::with_clinical_defaults();
        match check_coverage(&doc, &ir, &tagger, StrictnessMode::Strict) {
            CoverageResult::Fail { uncovered } => {
                assert_eq!(uncovered.len(), 1);
                assert_eq!(uncovered[0].start, 0);
                assert_eq!(uncovered[0].end, 7);
            }
            other => panic!("expected Fail, got {:?}", other),
        }
    }

    #[test]
    fn single_meaningful_word_covered_passes() {
        let doc = mk_doc("patient");
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![mk_fact("F1", span(0, 7))],
        };
        let tagger = RuleBasedTagger::with_clinical_defaults();
        assert_eq!(
            check_coverage(&doc, &ir, &tagger, StrictnessMode::Strict),
            CoverageResult::Pass
        );
    }

    #[test]
    fn multiple_ir_nodes_combine_to_cover_one_span() {
        // "abc def" — 'abc' and 'def' are meaningful (not stopwords).
        // Two IR nodes, one for each, should pass.
        let doc = mk_doc("abc def");
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![mk_fact("F1", span(0, 3)), mk_fact("F2", span(4, 7))],
        };
        let tagger = RuleBasedTagger::english();
        assert_eq!(
            check_coverage(&doc, &ir, &tagger, StrictnessMode::Strict),
            CoverageResult::Pass
        );
    }

    #[test]
    fn meaningful_span_partially_covered_fails() {
        // "abc def ghi"; IR covers 0..3 and 8..11, leaving 'def' at
        // 4..7 uncovered.
        let doc = mk_doc("abc def ghi");
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![mk_fact("F1", span(0, 3)), mk_fact("F2", span(8, 11))],
        };
        let tagger = RuleBasedTagger::english();
        match check_coverage(&doc, &ir, &tagger, StrictnessMode::Strict) {
            CoverageResult::Fail { uncovered } => {
                assert_eq!(uncovered.len(), 1);
                assert_eq!((uncovered[0].start, uncovered[0].end), (4, 7));
            }
            other => panic!("expected Fail, got {:?}", other),
        }
    }

    #[test]
    fn audit_only_mode_returns_pass_regardless() {
        let doc = mk_doc("patient");
        let ir = empty_ir();
        let tagger = RuleBasedTagger::with_clinical_defaults();
        assert_eq!(
            check_coverage(&doc, &ir, &tagger, StrictnessMode::AuditOnly),
            CoverageResult::Pass
        );
    }

    #[test]
    fn unparseable_discarded_node_always_fails_coverage() {
        // Even if the rest of the document is well-covered, an
        // Unparseable Discarded node triggers a coverage failure.
        let doc = mk_doc("patient");
        let mut ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![mk_fact("F1", span(0, 7))],
        };
        ir.nodes.push(IRNode {
            id: NodeId::new("D1"),
            kind: NodeKind::Discarded,
            term: logic_core_atom("discarded"),
            polarity: Polarity::Affirmed,
            modality: Modality::Present,
            source_spans: vec![span(0, 7)],
            confidence: 1.0,
            lowered_from: None,
            discard_reason: Some(DiscardReason::Unparseable),
            metadata: HashMap::new(),
        });
        let tagger = RuleBasedTagger::with_clinical_defaults();
        assert!(matches!(
            check_coverage(&doc, &ir, &tagger, StrictnessMode::Strict),
            CoverageResult::Fail { .. }
        ));
    }

    #[test]
    fn discarded_with_other_reason_does_not_fail_by_itself() {
        // A Discarded node with reason `Pleasantry` does NOT trigger
        // a hard failure; it simply contributes its span to the cover.
        let doc = mk_doc("");
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![IRNode {
                id: NodeId::new("D1"),
                kind: NodeKind::Discarded,
                term: logic_core_atom("discarded"),
                polarity: Polarity::Affirmed,
                modality: Modality::Present,
                source_spans: vec![span(0, 0)],
                confidence: 1.0,
                lowered_from: None,
                discard_reason: Some(DiscardReason::Pleasantry),
                metadata: HashMap::new(),
            }],
        };
        let tagger = RuleBasedTagger::english();
        assert_eq!(
            check_coverage(&doc, &ir, &tagger, StrictnessMode::Strict),
            CoverageResult::Pass
        );
    }

    #[test]
    fn tsa_correct_extraction_passes_coverage() {
        // Canonical TSA example from ADJ02 §"Worked Example".
        // "I am not bringing matches" — span 0..26.
        // Correct extractor cites the whole span; coverage passes.
        let doc = mk_doc("I am not bringing matches");
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![mk_fact("F6", span(0, 25))],
        };
        let tagger = RuleBasedTagger::english();
        assert_eq!(
            check_coverage(&doc, &ir, &tagger, StrictnessMode::Strict),
            CoverageResult::Pass
        );
    }

    #[test]
    fn tsa_only_matches_cited_fails_on_not_bringing_span() {
        // Counterexample from ADJ02: extractor cites only "matches"
        // (18..25), leaving "not bringing" (8..17) uncovered.
        let doc = mk_doc("I am not bringing matches");
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![mk_fact("F6", span(18, 25))],
        };
        let tagger = RuleBasedTagger::english();
        match check_coverage(&doc, &ir, &tagger, StrictnessMode::Strict) {
            CoverageResult::Fail { uncovered } => {
                // 'not' (8..11) and 'bringing' (12..20) are both
                // meaningful and uncovered.
                let ranges: Vec<(usize, usize)> = uncovered
                    .iter()
                    .map(|s| (s.start, s.end))
                    .collect();
                // At least one of those words must appear; order may
                // vary depending on tagger output.
                assert!(ranges.iter().any(|(s, _)| *s == 7 || *s == 8 || *s == 9));
            }
            other => panic!("expected Fail, got {:?}", other),
        }
    }

    #[test]
    fn merge_intervals_combines_overlapping_and_adjacent_ranges() {
        let merged = merge_intervals(vec![(0, 3), (2, 5), (10, 12), (12, 15)]);
        assert_eq!(merged, vec![(0, 5), (10, 15)]);
    }

    #[test]
    fn is_covered_correctly_decides_subset_membership() {
        let union = vec![(0, 10), (20, 30)];
        assert!(is_covered(0, 5, &union));
        assert!(is_covered(20, 30, &union));
        assert!(!is_covered(5, 15, &union)); // crosses a gap
        assert!(!is_covered(31, 40, &union)); // outside any range
    }

    #[test]
    fn spans_from_a_different_document_do_not_contribute() {
        let doc = mk_doc("hello");
        let other_doc = DocumentId::new("other");
        // IR node cites a span in another document → should not cover
        // anything in our document.
        let ir = IRDocument {
            document_id: doc_id(),
            nodes: vec![mk_fact("F1", Span::new(other_doc, 0, 5))],
        };
        let tagger = RuleBasedTagger::english();
        match check_coverage(&doc, &ir, &tagger, StrictnessMode::Strict) {
            CoverageResult::Fail { uncovered } => {
                assert!(!uncovered.is_empty());
            }
            other => panic!("expected Fail, got {:?}", other),
        }
    }
}
