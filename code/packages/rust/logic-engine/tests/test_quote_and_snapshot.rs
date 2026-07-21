//! RS-4 PR-D1 — the verbatim `quote` and its pinned `snapshot`
//! (`ADJ-REASON-MATH.md` §E.3).
//!
//! These two fields are what turn the audit trail from something ADJ *reports*
//! into something a third party can *check*. The tests below are mostly about
//! one property, because it is the one that can silently destroy the value of
//! everything else:
//!
//! **A clause with no recorded span must never look verified.**
//!
//! The tempting migration — default `quote` to the existing `source` label —
//! fails open. `source` labels are short ("NIST", "AQI basics") and would
//! trivially appear *somewhere* on the cited page, so the strongest check in
//! the system would pass while checking nothing, and would say so in a field
//! named `verified`. That is worse than having no verifier: it manufactures
//! confidence at scale, and it would have been the default state of the entire
//! shipped stdlib on day one.

use logic_engine::{ContentHash, Provenance, Quote};

// ---------------------------------------------------------------------------
// (1) The fail-open trap: an un-migrated clause is explicitly unverifiable.
// ---------------------------------------------------------------------------

#[test]
fn a_clause_built_the_old_way_is_unmigrated_not_silently_quoted() {
    // Every pre-existing call site goes through `new`/`cited`/`empirical`.
    let p = Provenance::cited("Pope 1995");

    assert!(
        p.quote.is_unmigrated(),
        "an un-split clause must SAY it has no checkable span"
    );
    assert_eq!(
        p.quote.text(),
        None,
        "and must not hand a verifier the citation label as if it were a quote"
    );
    // The label is still there for humans — the split loses nothing.
    assert_eq!(p.source, "Pope 1995");
}

#[test]
fn the_citation_label_is_never_reused_as_the_quote() {
    // The specific fail-open shape, stated as an executable claim: a short
    // label like "NIST" would appear on almost any NIST page, so if it were
    // ever promoted to `quote` the verifier would pass without checking.
    for label in ["NIST", "AQI basics", "WHO", "Pope 1995"] {
        let p = Provenance::cited(label);
        assert!(
            p.quote.text() != Some(label),
            "the label {label:?} must never become the quote — it would verify \
             trivially and prove nothing"
        );
    }
}

// ---------------------------------------------------------------------------
// (2) A migrated clause carries the span, its anchor, and its snapshot.
// ---------------------------------------------------------------------------

#[test]
fn a_migrated_clause_carries_an_anchored_span_and_a_pinned_snapshot() {
    let doc = b"Green   Good   0 to 50   Air quality is satisfactory.";
    let snap = ContentHash::of(doc);
    let p =
        Provenance::cited("EPA AirNow").with_quote("Good   0 to 50", Some(8), Some(snap.clone()));

    assert_eq!(p.quote.text(), Some("Good   0 to 50"));
    assert!(!p.quote.is_unmigrated());
    match &p.quote {
        Quote::Verbatim { byte_offset, .. } => assert_eq!(*byte_offset, Some(8)),
        Quote::Unmigrated => panic!("expected a verbatim quote"),
    }
    assert_eq!(p.snapshot.as_ref(), Some(&snap));
}

#[test]
fn the_anchor_actually_points_at_the_quote_in_the_document() {
    // The anchor is the whole reason verification is not a substring search:
    // it says WHERE the span sits, so a verifier confirms the clause rests on
    // this passage rather than on the words appearing somewhere in a footnote.
    let doc = "Green   Good   0 to 50   Air quality is satisfactory.";
    let quote = "Good   0 to 50";
    let offset = doc.find(quote).expect("fixture");
    let p = Provenance::cited("EPA AirNow").with_quote(
        quote,
        Some(offset),
        Some(ContentHash::of(doc.as_bytes())),
    );

    let Quote::Verbatim { text, byte_offset } = &p.quote else {
        panic!("expected a verbatim quote");
    };
    let at = byte_offset.expect("anchored");
    assert_eq!(
        &doc[at..at + text.len()],
        text,
        "the recorded offset must land exactly on the recorded span"
    );
}

// ---------------------------------------------------------------------------
// (3) The snapshot is tamper-evident: any edit breaks the match.
// ---------------------------------------------------------------------------

#[test]
fn a_snapshot_detects_that_the_document_changed_under_the_quote() {
    let original = b"Maroon   Hazardous   301 and higher";
    let snap = ContentHash::of(original);
    assert!(
        snap.matches(original),
        "the captured document still matches"
    );

    // One character edited — the sort of quiet revision that would otherwise
    // let a stale citation keep passing.
    let edited = b"Maroon   Hazardous   300 and higher";
    assert!(
        !snap.matches(edited),
        "an edited document must NOT match the pinned snapshot"
    );
}

#[test]
fn a_snapshot_round_trips_through_its_hex_form() {
    // A trail is stored and re-read; the hash has to survive that trip, or the
    // pin is only good within one process.
    let snap = ContentHash::of(b"The AQI includes six color-coded categories.");
    let restored = ContentHash::from_hex(snap.as_hex());
    assert_eq!(snap, restored);
    assert!(restored.matches(b"The AQI includes six color-coded categories."));
    assert_eq!(snap.as_hex().len(), 64, "SHA-256 hex is 64 chars");
}

#[test]
fn distinct_documents_get_distinct_hashes() {
    let a = ContentHash::of(b"Yellow   Moderate   51 to 100");
    let b = ContentHash::of(b"Orange   Unhealthy for Sensitive Groups   101 to 150");
    assert_ne!(a, b);
}

// ---------------------------------------------------------------------------
// (4) Partial migration never reads as success.
// ---------------------------------------------------------------------------

#[test]
fn a_quote_without_an_anchor_or_snapshot_is_still_incomplete() {
    // Real libraries migrate incrementally, so the type permits a span with no
    // offset and no snapshot. What must NOT happen is that state reading as
    // fully checkable — the verifier (PR-D2) treats either gap as Unverified.
    let p = Provenance::cited("EPA AirNow").with_quote("Good   0 to 50", None, None);

    assert_eq!(p.quote.text(), Some("Good   0 to 50"));
    assert!(
        p.snapshot.is_none(),
        "no snapshot was captured, and the record says so"
    );
    match &p.quote {
        Quote::Verbatim { byte_offset, .. } => assert_eq!(
            *byte_offset, None,
            "no anchor was captured, and the record says so rather than \
             implying the span can be located"
        ),
        Quote::Unmigrated => panic!("expected a verbatim quote"),
    }
}
