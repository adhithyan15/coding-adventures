//! # adjudication-polarity-modality — ADJ03 checker.
//!
//! Reference implementation of
//! [`ADJ03`](../../../specs/ADJ03-polarity-modality-checker.md).
//! Catches the canonical "denies chest pain" → `symptom(chest_pain)`
//! failure class with a NegEx/ConText-style scope detector.
//!
//! The check is **per-node**: it inspects each node's `source_spans`
//! for trigger phrases (negation, hedging, temporality, family
//! history, rule-out), computes each trigger's scope, and verifies
//! that the node's `polarity` and `modality` are consistent with the
//! triggers whose scope covers the node's content. Cross-node scope
//! analysis is `ADJ03a`, a planned follow-up.
//!
//! Pure (no LLM at check time); embarrassingly parallel across nodes.

use adjudication_ir::{IRDocument, IRNode, Modality, NodeId, NodeKind, Polarity};

// ---------------------------------------------------------------------------
// Trigger taxonomy
// ---------------------------------------------------------------------------

/// What kind of cue a trigger represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerClass {
    Negation,
    Hedge,
    TemporalPast,
    TemporalPresent,
    TemporalFuture,
    Hypothetical,
    FamilyHistory,
    RuleOut,
    Subject,
}

/// Which side of the trigger phrase its scope applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerDirection {
    /// Scope extends from after the trigger toward the right end of
    /// the text (e.g. "denies chest pain" — "denies" scopes forward).
    Forward,
    /// Scope extends backwards from before the trigger (e.g.
    /// "chest pain ruled out" — "ruled out" scopes back).
    Backward,
    /// The trigger affects content on either side.
    Bidirectional,
}

/// How far a trigger's scope extends.
#[derive(Debug, Clone, PartialEq)]
pub enum ScopeRule {
    /// Until the end of the current sentence (next `.`, `?`, `!` or
    /// end of source span text).
    UntilSentenceEnd,
    /// Until any of the listed delimiter characters is reached.
    UntilPunctuation(Vec<char>),
    /// Until any of the listed termination keywords is encountered
    /// (e.g., "but", "however", "except").
    UntilTermination(Vec<String>),
    /// A fixed token count from the trigger.
    UntilTokenCount(usize),
}

/// One entry in the trigger taxonomy.
#[derive(Debug, Clone, PartialEq)]
pub struct Trigger {
    pub class: TriggerClass,
    /// The lexical form. Multi-word phrases supported. Matched
    /// case-insensitively as a whole word/phrase (word boundaries on
    /// both sides).
    pub surface: String,
    pub direction: TriggerDirection,
    pub scope: ScopeRule,
}

/// Versioned, configurable trigger taxonomy. Mirror the Python
/// taxonomy's discipline of recording the version in audit-trail
/// metadata.
#[derive(Debug, Clone, Default)]
pub struct TriggerTaxonomy {
    pub triggers: Vec<Trigger>,
    pub version: String,
}

impl TriggerTaxonomy {
    pub fn new(version: &str) -> Self {
        Self {
            triggers: Vec::new(),
            version: version.to_string(),
        }
    }

    pub fn add(&mut self, t: Trigger) -> &mut Self {
        self.triggers.push(t);
        self
    }

    /// A reasonable English clinical taxonomy. NegEx + ConText
    /// inspired. Covers the high-impact cases for the worked
    /// examples; deployments are expected to extend.
    pub fn clinical_default() -> Self {
        let mut t = TriggerTaxonomy::new("clinical-en-v1");

        // Negation triggers — forward scope to sentence end, with
        // "but"/"however"/"except" terminating the scope.
        let neg_terms = vec![
            "but".to_string(),
            "however".to_string(),
            "except".to_string(),
            "although".to_string(),
        ];
        for s in &["denies", "no", "without", "not", "negative for", "denied"] {
            t.add(Trigger {
                class: TriggerClass::Negation,
                surface: (*s).to_string(),
                direction: TriggerDirection::Forward,
                scope: ScopeRule::UntilTermination(neg_terms.clone()),
            });
        }

        // Hedges — forward scope until end of sentence.
        for s in &["possibly", "questionable", "may have", "suggestive of", "consistent with", "concerning for"] {
            t.add(Trigger {
                class: TriggerClass::Hedge,
                surface: (*s).to_string(),
                direction: TriggerDirection::Forward,
                scope: ScopeRule::UntilSentenceEnd,
            });
        }

        // Temporality (past).
        for s in &["history of", "previously", "prior", "in 2019", "in 2020", "in 2021", "in 2022", "in 2023", "in 2024", "in 2025"] {
            t.add(Trigger {
                class: TriggerClass::TemporalPast,
                surface: (*s).to_string(),
                direction: TriggerDirection::Forward,
                scope: ScopeRule::UntilSentenceEnd,
            });
        }

        // Temporality (present).
        for s in &["currently", "today", "on admission"] {
            t.add(Trigger {
                class: TriggerClass::TemporalPresent,
                surface: (*s).to_string(),
                direction: TriggerDirection::Forward,
                scope: ScopeRule::UntilSentenceEnd,
            });
        }

        // Hypothetical.
        for s in &["if", "when", "in case of"] {
            t.add(Trigger {
                class: TriggerClass::Hypothetical,
                surface: (*s).to_string(),
                direction: TriggerDirection::Forward,
                scope: ScopeRule::UntilSentenceEnd,
            });
        }

        // Family history.
        for s in &["father", "mother", "brother", "sister", "family history of"] {
            t.add(Trigger {
                class: TriggerClass::FamilyHistory,
                surface: (*s).to_string(),
                direction: TriggerDirection::Forward,
                scope: ScopeRule::UntilSentenceEnd,
            });
        }

        // RuleOut — backwards over the diagnosis token, forward over
        // the test used to rule it out.
        for s in &["ruled out by", "ruled out", "excluded by", "negative on"] {
            t.add(Trigger {
                class: TriggerClass::RuleOut,
                surface: (*s).to_string(),
                direction: TriggerDirection::Bidirectional,
                scope: ScopeRule::UntilSentenceEnd,
            });
        }

        t
    }
}

// ---------------------------------------------------------------------------
// Document type (intentionally minimal; matches adjudication-coverage's
// shape so callers can pass the same document into both checkers).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Document {
    pub id: adjudication_ir::DocumentId,
    pub normalized_text: String,
}

// ---------------------------------------------------------------------------
// Violation type
// ---------------------------------------------------------------------------

/// What kind of field a trigger constrains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequiredField {
    Polarity(Polarity),
    Modality(Modality),
}

/// One per-node violation.
#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    pub node_id: NodeId,
    pub trigger_class: TriggerClass,
    pub trigger_surface: String,
    pub required: RequiredField,
    /// The actual value the IR carries.
    pub actual: RequiredField,
    /// Human-readable summary of the trigger phrase's location and
    /// scope, suitable for surfacing via ADJ06.
    pub suggestion: String,
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Run the polarity / modality check across every node in `ir_doc`.
/// Returns the list of violations (empty list = pass).
pub fn check_polarity_modality(
    doc: &Document,
    ir_doc: &IRDocument,
    taxonomy: &TriggerTaxonomy,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    for node in &ir_doc.nodes {
        // Only nodes that take a polarity/modality value participate.
        if !matches!(
            node.kind,
            NodeKind::Fact | NodeKind::Query | NodeKind::Uncertainty | NodeKind::Rule
        ) {
            continue;
        }
        // Stitch the span texts together so a single trigger search
        // sees the whole span.
        let span_text = collect_node_text(doc, node);
        if span_text.is_empty() {
            continue;
        }
        let mut local: Vec<Violation> = scan_node(node, &span_text, taxonomy);
        if !local.is_empty() {
            violations.append(&mut local);
        }
    }
    violations
}

fn collect_node_text(doc: &Document, node: &IRNode) -> String {
    let mut s = String::new();
    for span in &node.source_spans {
        if span.document_id != doc.id {
            continue;
        }
        if let Some(slice) = doc.normalized_text.get(span.start..span.end) {
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(slice);
        }
    }
    s
}

/// Run every trigger against the node's span text. Per ADJ03, returns
/// **at most one** Violation per node — the first trigger whose scope
/// covers the node's term content and whose required field disagrees
/// with the actual.
fn scan_node(node: &IRNode, span_text: &str, taxonomy: &TriggerTaxonomy) -> Vec<Violation> {
    let lower = span_text.to_lowercase();
    // Match longest surface first so e.g. "ruled out by" wins over
    // "ruled out" when both could match.
    let mut indexed: Vec<&Trigger> = taxonomy.triggers.iter().collect();
    indexed.sort_by_key(|t| std::cmp::Reverse(t.surface.len()));

    for trigger in indexed {
        let Some(pos) = find_phrase(&lower, &trigger.surface.to_lowercase()) else {
            continue;
        };
        let scoped_range = scoped_range(&lower, pos, &trigger.surface, &trigger.direction, &trigger.scope);
        let in_scope_text = &lower[scoped_range.0..scoped_range.1];
        if !term_content_in_text(node, in_scope_text) {
            continue;
        }
        if let Some(v) = check_violation(node, trigger) {
            return vec![v];
        }
    }
    Vec::new()
}

/// Find a whitespace-delimited phrase in `text`. Returns the start
/// byte index, or `None` if not found.
fn find_phrase(text: &str, phrase: &str) -> Option<usize> {
    let mut start = 0;
    while let Some(idx) = text[start..].find(phrase) {
        let abs = start + idx;
        let before_ok = abs == 0
            || text.as_bytes()[abs - 1].is_ascii_whitespace()
            || is_punct_byte(text.as_bytes()[abs - 1]);
        let end = abs + phrase.len();
        let after_ok = end == text.len()
            || text.as_bytes()[end].is_ascii_whitespace()
            || is_punct_byte(text.as_bytes()[end]);
        if before_ok && after_ok {
            return Some(abs);
        }
        start = abs + phrase.len();
    }
    None
}

fn is_punct_byte(b: u8) -> bool {
    matches!(
        b,
        b'.' | b',' | b';' | b':' | b'!' | b'?' | b'(' | b')' | b'[' | b']' | b'{' | b'}'
    )
}

/// Compute the scoped byte range that a trigger covers in `text`.
fn scoped_range(
    text: &str,
    trigger_pos: usize,
    surface: &str,
    direction: &TriggerDirection,
    scope: &ScopeRule,
) -> (usize, usize) {
    let after_trigger = trigger_pos + surface.len();
    match (direction, scope) {
        (TriggerDirection::Forward, ScopeRule::UntilSentenceEnd) => {
            (after_trigger, find_sentence_end(text, after_trigger))
        }
        (TriggerDirection::Forward, ScopeRule::UntilPunctuation(chars)) => {
            (after_trigger, find_first_char(text, after_trigger, chars))
        }
        (TriggerDirection::Forward, ScopeRule::UntilTermination(words)) => {
            (after_trigger, find_termination(text, after_trigger, words))
        }
        (TriggerDirection::Forward, ScopeRule::UntilTokenCount(n)) => {
            (after_trigger, find_after_tokens(text, after_trigger, *n))
        }
        (TriggerDirection::Backward, ScopeRule::UntilSentenceEnd) => {
            (find_sentence_start(text, trigger_pos), trigger_pos)
        }
        (TriggerDirection::Backward, _) => (find_sentence_start(text, trigger_pos), trigger_pos),
        (TriggerDirection::Bidirectional, _) => {
            (
                find_sentence_start(text, trigger_pos),
                find_sentence_end(text, after_trigger),
            )
        }
    }
}

fn find_sentence_end(text: &str, from: usize) -> usize {
    let bytes = text.as_bytes();
    for (i, b) in bytes.iter().enumerate().skip(from) {
        if matches!(*b, b'.' | b'?' | b'!') {
            return i;
        }
    }
    text.len()
}

fn find_sentence_start(text: &str, from: usize) -> usize {
    let bytes = text.as_bytes();
    for i in (0..from).rev() {
        if matches!(bytes[i], b'.' | b'?' | b'!') {
            return i + 1;
        }
    }
    0
}

fn find_first_char(text: &str, from: usize, chars: &[char]) -> usize {
    for (i, ch) in text.char_indices().skip_while(|(idx, _)| *idx < from) {
        if chars.contains(&ch) {
            return i;
        }
    }
    text.len()
}

fn find_termination(text: &str, from: usize, words: &[String]) -> usize {
    // Linear scan for any whole-word terminator.
    for word in words {
        if let Some(idx) = text[from..].find(word.as_str()) {
            let abs = from + idx;
            let before_ok = abs == 0
                || text.as_bytes()[abs - 1].is_ascii_whitespace();
            let end = abs + word.len();
            let after_ok = end == text.len()
                || text.as_bytes()[end].is_ascii_whitespace()
                || is_punct_byte(text.as_bytes()[end]);
            if before_ok && after_ok {
                return abs;
            }
        }
    }
    find_sentence_end(text, from)
}

fn find_after_tokens(text: &str, from: usize, n: usize) -> usize {
    let mut count = 0;
    let mut in_token = false;
    for (i, b) in text.as_bytes().iter().enumerate().skip(from) {
        if b.is_ascii_whitespace() || is_punct_byte(*b) {
            if in_token {
                count += 1;
                if count >= n {
                    return i;
                }
            }
            in_token = false;
        } else {
            in_token = true;
        }
    }
    text.len()
}

/// `true` iff any of the node's term's content words (functor name
/// or atom arguments) appears inside `text`.
///
/// Multi-word content words packed into a single atom with
/// underscores (e.g. `chest_pain`) match if **all** of their pieces
/// appear in `text`. This handles the common extraction convention
/// of joining multi-word phrases with underscores without producing
/// false positives when only one piece appears (e.g. a `back_pain`
/// term should not match text that says "chest pain").
fn term_content_in_text(node: &IRNode, text: &str) -> bool {
    for word in extract_content_words(&node.term) {
        let lower = word.to_lowercase();
        // Single-piece content word: direct phrase match.
        if !lower.contains('_') {
            if find_phrase(text, &lower).is_some() {
                return true;
            }
            continue;
        }
        // Multi-piece content word: every piece (length >= 3, to
        // avoid noise like "of") must appear in `text` as a phrase.
        let pieces: Vec<&str> = lower
            .split('_')
            .filter(|p| p.len() >= 3)
            .collect();
        if !pieces.is_empty() && pieces.iter().all(|p| find_phrase(text, p).is_some()) {
            return true;
        }
        // Also accept the literal joined form (some texts do use
        // underscores in identifiers).
        if find_phrase(text, &lower).is_some() {
            return true;
        }
    }
    false
}

fn extract_content_words(term: &logic_core::Term) -> Vec<String> {
    use logic_core::Term;
    let mut out = Vec::new();
    match term {
        Term::Atom(name) => out.push(name.clone()),
        Term::Compound { functor, args } => {
            out.push(functor.clone());
            for a in args {
                out.extend(extract_content_words(a));
            }
        }
        Term::Str(s) => out.push(s.clone()),
        _ => {}
    }
    out
}

/// Required (polarity, modality) for a given trigger class.
fn required_for(class: TriggerClass) -> Option<RequiredField> {
    match class {
        TriggerClass::Negation => Some(RequiredField::Polarity(Polarity::Denied)),
        TriggerClass::Hedge => Some(RequiredField::Polarity(Polarity::Uncertain)),
        TriggerClass::TemporalPast => Some(RequiredField::Modality(Modality::Past)),
        TriggerClass::TemporalPresent => Some(RequiredField::Modality(Modality::Present)),
        TriggerClass::TemporalFuture => Some(RequiredField::Modality(Modality::Future)),
        TriggerClass::Hypothetical => Some(RequiredField::Modality(Modality::Hypothetical)),
        TriggerClass::FamilyHistory => Some(RequiredField::Modality(Modality::FamilyHistory)),
        TriggerClass::RuleOut => Some(RequiredField::Modality(Modality::RuledOut)),
        // Subject triggers are bookkeeping; no required value.
        TriggerClass::Subject => None,
    }
}

fn actual_for(class: TriggerClass, node: &IRNode) -> Option<RequiredField> {
    match class {
        TriggerClass::Negation | TriggerClass::Hedge => Some(RequiredField::Polarity(node.polarity)),
        TriggerClass::TemporalPast
        | TriggerClass::TemporalPresent
        | TriggerClass::TemporalFuture
        | TriggerClass::Hypothetical
        | TriggerClass::FamilyHistory
        | TriggerClass::RuleOut => Some(RequiredField::Modality(node.modality)),
        TriggerClass::Subject => None,
    }
}

fn check_violation(node: &IRNode, trigger: &Trigger) -> Option<Violation> {
    let required = required_for(trigger.class)?;
    let actual = actual_for(trigger.class, node)?;
    if required != actual {
        Some(Violation {
            node_id: node.id.clone(),
            trigger_class: trigger.class,
            trigger_surface: trigger.surface.clone(),
            required,
            actual,
            suggestion: format!(
                "The cue '{}' suggests {:?}; the IR is {:?}.",
                trigger.surface, required, actual
            ),
        })
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use adjudication_ir::{DocumentId, Span};
    use logic_core::{atom, compound};
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

    fn mk_node(
        id: &str,
        term: logic_core::Term,
        polarity: Polarity,
        modality: Modality,
        start: usize,
        end: usize,
    ) -> IRNode {
        IRNode {
            id: NodeId::new(id),
            kind: NodeKind::Fact,
            term,
            polarity,
            modality,
            source_spans: vec![Span::new(doc_id(), start, end)],
            confidence: 1.0,
            lowered_from: None,
            discard_reason: None,
            metadata: HashMap::new(),
        }
    }

    fn ir_with(nodes: Vec<IRNode>) -> IRDocument {
        IRDocument {
            document_id: doc_id(),
            nodes,
        }
    }

    #[test]
    fn denies_with_affirmed_polarity_is_violation() {
        let doc = mk_doc("Patient denies chest pain.");
        let n = mk_node(
            "F1",
            compound("chest_pain", vec![atom("patient")]),
            Polarity::Affirmed,
            Modality::Present,
            0,
            26,
        );
        let ir = ir_with(vec![n]);
        let vs = check_polarity_modality(&doc, &ir, &TriggerTaxonomy::clinical_default());
        assert_eq!(vs.len(), 1);
        assert_eq!(vs[0].trigger_class, TriggerClass::Negation);
        assert_eq!(
            vs[0].required,
            RequiredField::Polarity(Polarity::Denied)
        );
    }

    #[test]
    fn denies_with_denied_polarity_passes() {
        let doc = mk_doc("Patient denies chest pain.");
        let n = mk_node(
            "F1",
            compound("chest_pain", vec![atom("patient")]),
            Polarity::Denied,
            Modality::Present,
            0,
            26,
        );
        let ir = ir_with(vec![n]);
        let vs = check_polarity_modality(&doc, &ir, &TriggerTaxonomy::clinical_default());
        assert!(vs.is_empty(), "expected no violations, got {:?}", vs);
    }

    #[test]
    fn father_with_past_modality_flags_family_history() {
        let doc = mk_doc("Father had MI at 50.");
        let n = mk_node(
            "F1",
            atom("mi"),
            Polarity::Affirmed,
            Modality::Past,
            0,
            20,
        );
        let ir = ir_with(vec![n]);
        let vs = check_polarity_modality(&doc, &ir, &TriggerTaxonomy::clinical_default());
        assert_eq!(vs.len(), 1);
        assert_eq!(vs[0].trigger_class, TriggerClass::FamilyHistory);
        assert_eq!(
            vs[0].required,
            RequiredField::Modality(Modality::FamilyHistory)
        );
    }

    #[test]
    fn ruled_out_with_ruled_out_modality_and_affirmed_polarity_passes() {
        // The spec is emphatic: RuledOut is modality only; polarity
        // stays Affirmed. This is the test that catches a naive
        // extractor flipping polarity instead of setting modality.
        let doc = mk_doc("PE ruled out by CT angio.");
        let n = mk_node(
            "F1",
            atom("pe"),
            Polarity::Affirmed,
            Modality::RuledOut,
            0,
            25,
        );
        let ir = ir_with(vec![n]);
        let vs = check_polarity_modality(&doc, &ir, &TriggerTaxonomy::clinical_default());
        assert!(vs.is_empty(), "expected no violations, got {:?}", vs);
    }

    #[test]
    fn ruled_out_with_denied_polarity_does_not_satisfy_the_modality_check() {
        // The RuleOut trigger requires modality RuledOut. A node with
        // polarity Denied and modality Present should still flag the
        // modality mismatch (Denied does not satisfy RuledOut).
        let doc = mk_doc("PE ruled out by CT angio.");
        let n = mk_node(
            "F1",
            atom("pe"),
            Polarity::Denied,
            Modality::Present,
            0,
            25,
        );
        let ir = ir_with(vec![n]);
        let vs = check_polarity_modality(&doc, &ir, &TriggerTaxonomy::clinical_default());
        assert_eq!(vs.len(), 1);
        assert_eq!(vs[0].trigger_class, TriggerClass::RuleOut);
        assert_eq!(
            vs[0].required,
            RequiredField::Modality(Modality::RuledOut)
        );
    }

    #[test]
    fn possibly_flags_uncertainty_when_polarity_is_affirmed() {
        let doc = mk_doc("Possibly pneumonia.");
        let n = mk_node(
            "F1",
            atom("pneumonia"),
            Polarity::Affirmed,
            Modality::Present,
            0,
            19,
        );
        let ir = ir_with(vec![n]);
        let vs = check_polarity_modality(&doc, &ir, &TriggerTaxonomy::clinical_default());
        assert_eq!(vs.len(), 1);
        assert_eq!(vs[0].trigger_class, TriggerClass::Hedge);
        assert_eq!(
            vs[0].required,
            RequiredField::Polarity(Polarity::Uncertain)
        );
    }

    #[test]
    fn history_of_flags_past_modality_when_modality_is_present() {
        let doc = mk_doc("History of asthma.");
        let n = mk_node(
            "F1",
            atom("asthma"),
            Polarity::Affirmed,
            Modality::Present,
            0,
            18,
        );
        let ir = ir_with(vec![n]);
        let vs = check_polarity_modality(&doc, &ir, &TriggerTaxonomy::clinical_default());
        assert_eq!(vs.len(), 1);
        assert_eq!(vs[0].trigger_class, TriggerClass::TemporalPast);
        assert_eq!(
            vs[0].required,
            RequiredField::Modality(Modality::Past)
        );
    }

    #[test]
    fn termination_keyword_limits_negation_scope() {
        // "Denies chest pain but admits back pain." — "but"
        // terminates the negation scope before "back". A
        // back_pain(patient) Affirmed node should NOT be flagged.
        let doc = mk_doc("Denies chest pain but admits back pain.");
        let n = mk_node(
            "F1",
            atom("back_pain"),
            Polarity::Affirmed,
            Modality::Present,
            0,
            39,
        );
        let ir = ir_with(vec![n]);
        let vs = check_polarity_modality(&doc, &ir, &TriggerTaxonomy::clinical_default());
        // Note: "back_pain" as an atom doesn't appear literally in the
        // text (the text says "back pain"). The check looks for
        // content words from the term; the functor "back_pain" won't
        // match because the text has the space. This documents the
        // current limit: word-boundary matching does not split
        // underscores. Result: no violation regardless, because the
        // content word is not found in scope.
        // We assert no violation; if a future tokenizer-aware check
        // changes this, the assertion is the trigger to update.
        assert!(vs.is_empty(), "expected no violations, got {:?}", vs);
    }

    #[test]
    fn unrelated_trigger_in_span_does_not_flag_if_term_not_in_scope() {
        // Span includes "denies" but the term content word is outside
        // the negation scope.
        let doc = mk_doc("Denies chest pain. Admits fever.");
        let n = mk_node(
            "F1",
            atom("fever"),
            Polarity::Affirmed,
            Modality::Present,
            0,
            32,
        );
        let ir = ir_with(vec![n]);
        let vs = check_polarity_modality(&doc, &ir, &TriggerTaxonomy::clinical_default());
        // "fever" is after the sentence-ending "." that terminates
        // the negation scope, so no violation should fire.
        assert!(
            vs.iter().all(|v| v.trigger_class != TriggerClass::Negation),
            "negation should not fire on fever; got {:?}",
            vs
        );
    }

    #[test]
    fn no_triggers_in_span_pass_silently() {
        let doc = mk_doc("Patient reports clear breath sounds.");
        let n = mk_node(
            "F1",
            atom("clear_breath_sounds"),
            Polarity::Affirmed,
            Modality::Present,
            0,
            36,
        );
        let ir = ir_with(vec![n]);
        let vs = check_polarity_modality(&doc, &ir, &TriggerTaxonomy::clinical_default());
        assert!(vs.is_empty());
    }

    #[test]
    fn previously_flags_past_modality() {
        let doc = mk_doc("Patient previously had asthma.");
        let n = mk_node(
            "F1",
            atom("asthma"),
            Polarity::Affirmed,
            Modality::Present,
            0,
            30,
        );
        let ir = ir_with(vec![n]);
        let vs = check_polarity_modality(&doc, &ir, &TriggerTaxonomy::clinical_default());
        assert!(vs
            .iter()
            .any(|v| v.trigger_class == TriggerClass::TemporalPast));
    }

    #[test]
    fn no_token_flags_negation() {
        let doc = mk_doc("No fever today.");
        let n = mk_node(
            "F1",
            atom("fever"),
            Polarity::Affirmed,
            Modality::Present,
            0,
            15,
        );
        let ir = ir_with(vec![n]);
        let vs = check_polarity_modality(&doc, &ir, &TriggerTaxonomy::clinical_default());
        assert!(vs
            .iter()
            .any(|v| v.trigger_class == TriggerClass::Negation));
    }

    #[test]
    fn nodes_without_polarity_modality_value_are_skipped() {
        // Discarded nodes don't participate in the check.
        let doc = mk_doc("Denies chest pain.");
        let n = IRNode {
            id: NodeId::new("D1"),
            kind: NodeKind::Discarded,
            term: atom("chest_pain"),
            polarity: Polarity::Affirmed, // would otherwise be flagged
            modality: Modality::Present,
            source_spans: vec![Span::new(doc_id(), 0, 18)],
            confidence: 1.0,
            lowered_from: None,
            discard_reason: Some(adjudication_ir::DiscardReason::Pleasantry),
            metadata: HashMap::new(),
        };
        let ir = ir_with(vec![n]);
        let vs = check_polarity_modality(&doc, &ir, &TriggerTaxonomy::clinical_default());
        assert!(vs.is_empty(), "Discarded nodes should be skipped");
    }

    #[test]
    fn empty_taxonomy_never_flags() {
        let doc = mk_doc("Denies chest pain.");
        let n = mk_node(
            "F1",
            atom("chest_pain"),
            Polarity::Affirmed,
            Modality::Present,
            0,
            18,
        );
        let ir = ir_with(vec![n]);
        let empty = TriggerTaxonomy::new("empty");
        assert!(check_polarity_modality(&doc, &ir, &empty).is_empty());
    }
}
