//! Tests for the `ppt` reader.
//!
//! These exercise (a) the required end-to-end path against the real CFB-wrapped
//! fixture, and (b) the record walker in isolation via `parse_document_stream`
//! fed synthetic byte streams. Synthetic streams are built with the tiny
//! `record` helper below so the tests read like the format itself.

use super::*;
use crate::fixture;

// ---------------------------------------------------------------------------
// Synthetic-record builders — mirror the RecordHeader layout exactly.
// ---------------------------------------------------------------------------

/// Build one record: 8-byte header (recVerInst, recType, recLen LE) + body.
/// `container` sets recVer to 0xF (else 0). `instance` fills the high 12 bits.
fn record(container: bool, instance: u16, rec_type: u16, body: &[u8]) -> Vec<u8> {
    let ver: u16 = if container { 0x0F } else { 0x00 };
    let ver_inst: u16 = (instance << 4) | ver;
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(&ver_inst.to_le_bytes());
    out.extend_from_slice(&rec_type.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    out
}

/// A TextBytesAtom (`0x0FA8`) body: one byte per char, NUL-terminated like
/// PowerPoint writes it.
fn text_bytes_atom(s: &str) -> Vec<u8> {
    let mut body: Vec<u8> = s.bytes().collect();
    body.push(0);
    record(false, 0, REC_TEXT_BYTES, &body)
}

/// A TextCharsAtom (`0x0FA0`) body: UTF-16LE, NUL-terminated.
fn text_chars_atom(s: &str) -> Vec<u8> {
    let mut body = Vec::new();
    for u in s.encode_utf16() {
        body.extend_from_slice(&u.to_le_bytes());
    }
    body.extend_from_slice(&0u16.to_le_bytes());
    record(false, 0, REC_TEXT_CHARS, &body)
}

/// A Slide container (`0x03EE`) wrapping the given child bytes.
fn slide_container(children: &[u8]) -> Vec<u8> {
    record(true, 0, REC_SLIDE, children)
}

// ---------------------------------------------------------------------------
// 1. REQUIRED end-to-end test against the real fixture.
// ---------------------------------------------------------------------------

#[test]
fn end_to_end_fixture() {
    let p = open_ppt(fixture::MINIMAL_PPT).expect("open ppt");
    assert_eq!(p.slide_count(), 2);

    let s0 = p.slides()[0].text();
    assert!(s0.contains("Slide One Title") && s0.contains("First slide body"));

    let s1 = p.slides()[1].text();
    assert!(s1.contains("Slide Two Title") && s1.contains("Second slide body"));

    // Order guard: slide-2 text must not appear on slide 1.
    assert!(!s0.contains("Second slide body"));
}

#[test]
fn fixture_runs_are_separate_in_order() {
    let p = open_ppt(fixture::MINIMAL_PPT).expect("open ppt");
    let s0 = &p.slides()[0];
    assert_eq!(s0.text_runs(), &["Slide One Title", "First slide body"]);
    let s1 = &p.slides()[1];
    assert_eq!(s1.text_runs(), &["Slide Two Title", "Second slide body"]);
}

// ---------------------------------------------------------------------------
// 2. Atom decoding in isolation.
// ---------------------------------------------------------------------------

#[test]
fn text_bytes_latin1_decoding() {
    // Latin-1 high byte 0xE9 = 'é' (U+00E9). Build a body directly to control
    // the raw bytes, plus a trailing NUL that must be stripped.
    let body = [b'c', b'a', b'f', 0xE9, 0x00];
    let out = decode_text_bytes(&body);
    assert_eq!(out, "café");
}

#[test]
fn text_bytes_no_trailing_nul() {
    let out = decode_text_bytes(b"hi");
    assert_eq!(out, "hi");
}

#[test]
fn text_chars_utf16_decoding() {
    // "Ω!" as UTF-16LE: U+03A9, U+0021, then a NUL terminator.
    let mut body = Vec::new();
    for u in "Ω!".encode_utf16() {
        body.extend_from_slice(&u.to_le_bytes());
    }
    body.extend_from_slice(&0u16.to_le_bytes());
    let out = decode_text_chars(&body);
    assert_eq!(out, "Ω!");
}

#[test]
fn text_chars_odd_trailing_byte_ignored() {
    // Two full UTF-16 units ("Hi") plus one stray odd byte → the stray is
    // dropped, no panic.
    let mut body = Vec::new();
    for u in "Hi".encode_utf16() {
        body.extend_from_slice(&u.to_le_bytes());
    }
    body.push(0x42); // lone trailing byte
    assert_eq!(decode_text_chars(&body), "Hi");
}

#[test]
fn text_chars_unpaired_surrogate_becomes_replacement() {
    // A lone high surrogate 0xD800 is invalid UTF-16; must become U+FFFD, not
    // panic.
    let body = [0x00, 0xD8]; // 0xD800 LE
    let out = decode_text_chars(&body);
    assert_eq!(out, "\u{FFFD}");
}

// ---------------------------------------------------------------------------
// 3. Synthetic record streams via parse_document_stream.
// ---------------------------------------------------------------------------

#[test]
fn two_slides_from_synthetic_stream() {
    let mut s1 = Vec::new();
    s1.extend(text_bytes_atom("Title A"));
    s1.extend(text_bytes_atom("Body A"));

    let mut s2 = Vec::new();
    s2.extend(text_chars_atom("Title B"));
    s2.extend(text_bytes_atom("Body B"));

    let mut stream = Vec::new();
    stream.extend(slide_container(&s1));
    stream.extend(slide_container(&s2));

    let p = parse_document_stream(&stream).unwrap();
    assert_eq!(p.slide_count(), 2);
    assert_eq!(p.slides()[0].text(), "Title A\nBody A");
    assert_eq!(p.slides()[1].text(), "Title B\nBody B");
}

#[test]
fn container_recursion_text_nested_in_document_and_slide() {
    // Document(0x03E8) → Slide(0x03EE) → arbitrary wrapper container → text.
    // Verifies we recurse through the Document wrapper AND through an unknown
    // intermediate container to reach the text atom, attaching it to the slide.
    let wrapper = record(true, 0, 0x0FF0 /* some drawing container */, &text_bytes_atom("Deep text"));
    let slide = slide_container(&wrapper);
    let document = record(true, 0, REC_DOCUMENT, &slide);

    let p = parse_document_stream(&document).unwrap();
    assert_eq!(p.slide_count(), 1);
    assert_eq!(p.slides()[0].text(), "Deep text");
}

#[test]
fn text_outside_any_slide_is_ignored() {
    // A bare text atom at the top level (not inside a Slide) yields no slides
    // and no text.
    let stream = text_bytes_atom("orphan");
    let p = parse_document_stream(&stream).unwrap();
    assert_eq!(p.slide_count(), 0);
}

// ---------------------------------------------------------------------------
// 4. Robustness: padding, truncation, deep nesting.
// ---------------------------------------------------------------------------

#[test]
fn trailing_zero_padding_stops_walk() {
    // A real slide followed by a block of zeros (CFB sector padding). The walker
    // must read the slide and then stop cleanly at the zeros — no panic, no hang.
    let mut stream = slide_container(&text_bytes_atom("Only slide"));
    stream.extend(std::iter::repeat_n(0u8, 512)); // padding

    let p = parse_document_stream(&stream).unwrap();
    assert_eq!(p.slide_count(), 1);
    assert_eq!(p.slides()[0].text(), "Only slide");
}

#[test]
fn empty_stream_yields_no_slides() {
    let p = parse_document_stream(&[]).unwrap();
    assert_eq!(p.slide_count(), 0);
}

#[test]
fn fewer_than_header_bytes_stops_cleanly() {
    // Only 3 bytes — not even a full 8-byte header. Must not panic.
    let p = parse_document_stream(&[0x0F, 0x00, 0xEE]).unwrap();
    assert_eq!(p.slide_count(), 0);
}

#[test]
fn reclen_past_buffer_stops_cleanly() {
    // A header claiming a 1000-byte body but with almost no body present. Must
    // stop cleanly rather than slice out of bounds.
    let mut stream = Vec::new();
    // Slide container header claiming recLen = 1000, but no body follows.
    stream.extend_from_slice(&0x000Fu16.to_le_bytes()); // container
    stream.extend_from_slice(&REC_SLIDE.to_le_bytes());
    stream.extend_from_slice(&1000u32.to_le_bytes()); // lies about length
    stream.extend_from_slice(&[0u8; 4]); // only 4 body bytes present

    let p = parse_document_stream(&stream).unwrap();
    // The over-long slide is rejected (body doesn't fit) → no slides.
    assert_eq!(p.slide_count(), 0);
}

#[test]
fn deep_nesting_does_not_overflow_stack() {
    // Build MANY nested containers (far beyond MAX_DEPTH). The walker must stop
    // at the depth cap without overflowing the native stack. We nest a text
    // atom at the very bottom, inside a slide near the top, and assert we do NOT
    // crash (and that beyond the cap the text is unreachable).
    const NEST: usize = 500; // >> MAX_DEPTH (64)
    let mut inner = text_bytes_atom("bottom");
    for _ in 0..NEST {
        inner = record(true, 0, 0x0FF0, &inner);
    }
    let slide = slide_container(&inner);

    // Must not panic/overflow.
    let p = parse_document_stream(&slide).unwrap();
    // One slide is created (the outer Slide container), but the text is deeper
    // than MAX_DEPTH so it is never reached.
    assert_eq!(p.slide_count(), 1);
    assert_eq!(p.slides()[0].text(), "");
}

#[test]
fn zero_length_reclen_container_advances() {
    // A non-padding record with recLen 0 (a Slide container with an empty body)
    // must still advance the cursor and be counted, and a following record must
    // be read too.
    let mut stream = Vec::new();
    stream.extend(slide_container(&[])); // empty slide
    stream.extend(slide_container(&text_bytes_atom("second")));

    let p = parse_document_stream(&stream).unwrap();
    assert_eq!(p.slide_count(), 2);
    assert_eq!(p.slides()[0].text(), "");
    assert_eq!(p.slides()[1].text(), "second");
}

// ---------------------------------------------------------------------------
// 5. Error paths on the CFB boundary.
// ---------------------------------------------------------------------------

#[test]
fn non_cfb_bytes_error_is_cfb() {
    let err = open_ppt(b"this is not a compound file at all").unwrap_err();
    match err {
        PptError::Cfb(_) => {}
        other => panic!("expected Cfb error, got {other:?}"),
    }
    // Display + Error wiring.
    assert!(!format!("{err}").is_empty());
    assert!(std::error::Error::source(&err).is_some());
}

#[test]
fn cfb_without_document_stream_is_no_document_stream() {
    // A genuinely valid CFB whose only stream is "Workbook" (a real .xls), NOT
    // "PowerPoint Document". `open_ppt` must open the container successfully but
    // then fail with `NoDocumentStream` because the PowerPoint record stream is
    // absent. This exercises the real `read_stream(...).ok_or(...)` branch.
    let err = open_ppt(fixture::VALID_CFB_NO_PPT_STREAM).unwrap_err();
    match err {
        PptError::NoDocumentStream => {}
        other => panic!("expected NoDocumentStream, got {other:?}"),
    }
    assert!(format!("{err}").contains("PowerPoint Document"));
    assert!(std::error::Error::source(&err).is_none());
}

#[test]
fn error_display_and_from_conversions() {
    // From<CfbError> path: force a CfbError by opening bad bytes, then convert.
    let cfb_err = cfb::CompoundFile::open(b"nope").unwrap_err();
    let ppt_err: PptError = cfb_err.into();
    assert!(matches!(ppt_err, PptError::Cfb(_)));

    let trunc = PptError::Truncated;
    assert!(format!("{trunc}").contains("truncated") || format!("{trunc}").contains("malformed"));
}
