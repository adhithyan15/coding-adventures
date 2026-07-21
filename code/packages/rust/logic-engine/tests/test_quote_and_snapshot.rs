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
    // CHECKED slicing, deliberately — this is the pattern the verifier copies.
    // `&doc[at..at + text.len()]` panics three ways on stale or hostile input:
    // an offset past the end, an offset off a UTF-8 character boundary, and
    // `at + len` overflowing. A verifier must treat all three as Unverified,
    // never as a crash.
    let found = doc.get(at..at.checked_add(text.len()).expect("no overflow"));
    assert_eq!(
        found,
        Some(text.as_str()),
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
    let restored = ContentHash::from_hex(snap.as_hex()).expect("a real digest");
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

// ---------------------------------------------------------------------------
// (5) The blank-span door: an empty quote would verify against EVERYTHING.
//
//     Found by this PR's security review. The `Unmigrated` variant closes
//     "the label silently became the quote"; this closes the other way in.
//     A verifier checks `doc[at..at + text.len()] == text`, which is trivially
//     true for an empty span at every offset in every document — so an empty
//     quote reports Verified while resting on nothing, and does it through the
//     intended builder where nothing looks wrong.
// ---------------------------------------------------------------------------

#[test]
fn a_blank_span_is_recorded_as_absent_rather_than_as_a_universal_match() {
    let snap = ContentHash::of(b"any document at all");
    for blank in ["", " ", "\t\n  "] {
        let p = Provenance::cited("EPA AirNow").with_quote(blank, Some(0), Some(snap.clone()));
        assert!(
            p.quote.is_unmigrated(),
            "a blank span ({blank:?}) must degrade to Unmigrated — it would \
             otherwise match every document at every offset"
        );
        assert_eq!(p.quote.text(), None);
    }
}

// ---------------------------------------------------------------------------
// (6) A digest is a digest by CONSTRUCTION, so `==` cannot be satisfied by
//     two copies of the same garbage out of the same untrusted trail.
// ---------------------------------------------------------------------------

#[test]
fn from_hex_rejects_anything_that_is_not_a_real_digest() {
    for junk in [
        "not-a-hash",
        "",
        "deadbeef",                                                         // too short
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b8",   // 62 chars
        "g3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", // non-hex
    ] {
        assert!(
            ContentHash::from_hex(junk).is_none(),
            "{junk:?} is not a SHA-256 digest and must not become a ContentHash"
        );
    }
}

#[test]
fn a_digest_survives_case_and_whitespace_without_becoming_silent_drift() {
    // A trail round-tripped through something that upcases hex, or leaves a
    // stray newline, must still read as the SAME digest. Otherwise it would
    // never match, and that permanent failure is indistinguishable at the call
    // site from the genuine document drift the snapshot exists to detect.
    let snap = ContentHash::of(b"Yellow   Moderate   51 to 100");
    let upper = ContentHash::from_hex(snap.as_hex().to_uppercase()).expect("still a digest");
    let padded = ContentHash::from_hex(format!("  {}\n", snap.as_hex())).expect("still a digest");
    assert_eq!(snap, upper);
    assert_eq!(snap, padded);
    assert!(upper.matches(b"Yellow   Moderate   51 to 100"));
}

// ---------------------------------------------------------------------------
// (7) `with_quote_in` makes the three anchors agree by construction.
// ---------------------------------------------------------------------------

#[test]
fn recording_a_span_against_its_document_derives_a_consistent_snapshot() {
    let doc = "Green   Good   0 to 50   Air quality is satisfactory.";
    let at = doc.find("Good   0 to 50").expect("fixture");
    let p = Provenance::cited("EPA AirNow")
        .with_quote_in(doc, at, "Good   0 to 50".len())
        .expect("a real span");

    assert_eq!(p.quote.text(), Some("Good   0 to 50"));
    // The snapshot was DERIVED from the same document, so it cannot disagree
    // with the offset — which is the whole point of this constructor.
    assert!(p.snapshot.expect("derived").matches(doc.as_bytes()));
}

#[test]
fn recording_a_span_refuses_ranges_that_would_panic_a_verifier() {
    let doc = "Purple   Very Unhealthy   201 to 300";

    // Past the end.
    assert!(Provenance::cited("x").with_quote_in(doc, 30, 999).is_none());
    // Arithmetic overflow rather than a wrapped, in-range-looking end.
    assert!(Provenance::cited("x")
        .with_quote_in(doc, usize::MAX, 2)
        .is_none());
    // Blank.
    assert!(Provenance::cited("x").with_quote_in(doc, 6, 3).is_none());

    // A multi-byte document, sliced off a character boundary.
    // 'R' is byte 0; 'é' occupies bytes 1..3, so byte 2 is INSIDE it — the
    // offset that would panic a naive `&doc[2..4]`.
    let utf8 = "Résumé — 51 to 100";
    assert!(
        !utf8.is_char_boundary(2),
        "fixture: byte 2 is mid-character"
    );
    assert!(
        Provenance::cited("x").with_quote_in(utf8, 2, 2).is_none(),
        "an offset off a UTF-8 boundary must be refused, not panicked on"
    );
}
