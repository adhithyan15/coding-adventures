//! Tests for `ppt-writer`.
//!
//! The centrepiece is the **round-trip proof** (`round_trip_two_slides`): we
//! write a real `.ppt`, reopen it with the `cfb` reader, extract the "PowerPoint
//! Document" stream, and walk its record tree, asserting the decoded slide text.
//! The remaining tests cover the atom-choice logic, header bit-packing, the
//! empty cases, and the padding-stop rule.

use super::*;
use cfb::CompoundFile;

// ---------------------------------------------------------------------------
// A small, self-contained record walker used by the tests. It mirrors §5.1 and
// §8 of the spec: read an 8-byte header; if recVer == 0xF recurse into the body
// (a container); else if it is a text atom, decode it. Stop on the padding
// sentinel (recType == 0 && recLen == 0) or when < 8 bytes remain.
// ---------------------------------------------------------------------------

/// One decoded record the walker cares about.
#[derive(Debug, PartialEq, Eq)]
enum Node {
    /// A Slide container and its decoded child text atoms.
    Slide(Vec<String>),
}

/// Read a little-endian u16 at `off`, or `None` if out of range.
fn rd_u16(buf: &[u8], off: usize) -> Option<u16> {
    let b = buf.get(off..off + 2)?;
    Some(u16::from_le_bytes([b[0], b[1]]))
}

/// Read a little-endian u32 at `off`, or `None` if out of range.
fn rd_u32(buf: &[u8], off: usize) -> Option<u32> {
    let b = buf.get(off..off + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Decode a text atom body by its recType into a `String`.
fn decode_text_atom(rec_type: u16, body: &[u8]) -> String {
    if rec_type == REC_TYPE_TEXT_BYTES {
        // One byte per char (Latin-1): each byte is a code point ≤ 0xFF.
        body.iter().map(|&b| b as char).collect()
    } else {
        // TextChars: UTF-16LE.
        let units: Vec<u16> = body
            .as_chunks::<2>().0.iter()
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    }
}

/// Walk the record stream and return the top-level nodes we recognise.
///
/// Returns `Err(&str)` if the bytes are malformed in a way that would run us off
/// the end — the walker is defensive so a test failure is a clean assertion, not
/// a panic.
fn walk(stream: &[u8]) -> Result<Vec<Node>, &'static str> {
    let mut nodes = Vec::new();
    let mut pos = 0usize;

    while pos + 8 <= stream.len() {
        let ver_and_instance = rd_u16(stream, pos).ok_or("short header")?;
        let rec_type = rd_u16(stream, pos + 2).ok_or("short header")?;
        let rec_len = rd_u32(stream, pos + 4).ok_or("short header")? as usize;
        let rec_ver = ver_and_instance & 0x000F;

        // Padding sentinel: a zeroed region past the logical records.
        if rec_type == 0 && rec_len == 0 {
            break;
        }

        let body_start = pos + 8;
        let body_end = body_start.checked_add(rec_len).ok_or("length overflow")?;
        if body_end > stream.len() {
            return Err("record body runs past end of stream");
        }
        let body = &stream[body_start..body_end];

        if rec_ver == REC_VER_CONTAINER && rec_type == REC_TYPE_SLIDE {
            // Recurse into the container body, collecting its text atoms.
            let texts = walk_slide_atoms(body)?;
            nodes.push(Node::Slide(texts));
        }
        // (Other record types are ignored at the top level in this profile.)

        pos = body_end;
    }
    Ok(nodes)
}

/// Walk a Slide container's body, decoding each text atom to a `String`.
fn walk_slide_atoms(body: &[u8]) -> Result<Vec<String>, &'static str> {
    let mut texts = Vec::new();
    let mut pos = 0usize;
    while pos + 8 <= body.len() {
        let rec_type = rd_u16(body, pos + 2).ok_or("short child header")?;
        let rec_len = rd_u32(body, pos + 4).ok_or("short child header")? as usize;
        if rec_type == 0 && rec_len == 0 {
            break;
        }
        let start = pos + 8;
        let end = start.checked_add(rec_len).ok_or("child length overflow")?;
        if end > body.len() {
            return Err("child body runs past end of container");
        }
        let child = &body[start..end];
        if rec_type == REC_TYPE_TEXT_BYTES || rec_type == REC_TYPE_TEXT_CHARS {
            texts.push(decode_text_atom(rec_type, child));
        }
        pos = end;
    }
    Ok(texts)
}

// ---------------------------------------------------------------------------
// The round-trip proof.
// ---------------------------------------------------------------------------

#[test]
fn round_trip_two_slides() {
    // Build the exact deck the milestone specifies.
    let mut deck = Presentation::new();
    let s1 = deck.add_slide();
    s1.add_text("Slide One Title");
    s1.add_text("First slide body");
    let s2 = deck.add_slide();
    s2.add_text("Slide Two Title");
    s2.add_text("Second slide body");

    // Write the .ppt and reopen it with the real CFB reader.
    let bytes = write_ppt(&deck);
    let cf = CompoundFile::open(&bytes).expect(".ppt should be a valid Compound File");

    // Extract the payload stream by name and walk its record tree.
    let stream = cf
        .read_stream(STREAM_POWERPOINT_DOCUMENT)
        .expect("PowerPoint Document stream present");
    let nodes = walk(&stream).expect("record tree should walk cleanly");

    // Exactly two Slide containers, with the expected paragraph text.
    assert_eq!(nodes.len(), 2, "expected two Slide containers");
    assert_eq!(
        nodes[0],
        Node::Slide(vec![
            "Slide One Title".to_string(),
            "First slide body".to_string(),
        ])
    );
    assert_eq!(
        nodes[1],
        Node::Slide(vec![
            "Slide Two Title".to_string(),
            "Second slide body".to_string(),
        ])
    );
}

#[test]
fn current_user_stream_is_present() {
    // The stub "Current User" stream should be wrapped in too.
    let deck = Presentation::new();
    let bytes = write_ppt(&deck);
    let cf = CompoundFile::open(&bytes).unwrap();
    let cu = cf
        .read_stream(STREAM_CURRENT_USER)
        .expect("Current User stub present");
    assert_eq!(cu, CURRENT_USER_STUB.to_vec());
}

// ---------------------------------------------------------------------------
// Atom-choice and encoding.
// ---------------------------------------------------------------------------

#[test]
fn latin1_text_uses_text_bytes_atom() {
    // Pure ASCII → TextBytes (one byte per char).
    let (rec_type, body) = encode_text_body("Hi!");
    assert_eq!(rec_type, REC_TYPE_TEXT_BYTES);
    assert_eq!(body, b"Hi!".to_vec());

    // A char at the top of Latin-1 (é = U+00E9) is still TextBytes.
    let (rec_type, body) = encode_text_body("caf\u{00E9}");
    assert_eq!(rec_type, REC_TYPE_TEXT_BYTES);
    assert_eq!(body, vec![b'c', b'a', b'f', 0xE9]);
}

#[test]
fn non_latin1_text_uses_text_chars_atom_and_round_trips() {
    // "你好" forces the TextChars (UTF-16LE) path.
    let text = "你好";
    let (rec_type, body) = encode_text_body(text);
    assert_eq!(rec_type, REC_TYPE_TEXT_CHARS);
    // UTF-16LE of "你好": U+4F60, U+597D → 60 4F 7D 59.
    assert_eq!(body, vec![0x60, 0x4F, 0x7D, 0x59]);

    // And it decodes back through a full write/walk.
    let mut deck = Presentation::new();
    deck.add_slide().add_text(text);
    let bytes = write_ppt(&deck);
    let cf = CompoundFile::open(&bytes).unwrap();
    let stream = cf.read_stream(STREAM_POWERPOINT_DOCUMENT).unwrap();
    let nodes = walk(&stream).unwrap();
    assert_eq!(nodes, vec![Node::Slide(vec!["你好".to_string()])]);
}

#[test]
fn is_all_latin1_boundary() {
    assert!(is_all_latin1("plain ascii"));
    assert!(is_all_latin1("\u{00FF}")); // exactly 0xFF is still Latin-1
    assert!(!is_all_latin1("\u{0100}")); // one past the boundary
    assert!(!is_all_latin1("emoji \u{1F600}")); // astral plane
    assert!(is_all_latin1("")); // empty is trivially Latin-1
}

// ---------------------------------------------------------------------------
// Header bit-packing round-trip.
// ---------------------------------------------------------------------------

#[test]
fn record_header_bit_packing_round_trips() {
    // Emit a container header with a known body length and decode every field.
    let mut buf = Vec::new();
    assert!(push_record_header(
        &mut buf,
        REC_VER_CONTAINER,
        REC_TYPE_SLIDE,
        0x0001_2345
    ));
    assert_eq!(buf.len(), 8);

    let ver_and_instance = rd_u16(&buf, 0).unwrap();
    let rec_type = rd_u16(&buf, 2).unwrap();
    let rec_len = rd_u32(&buf, 4).unwrap();
    assert_eq!(ver_and_instance & 0x000F, REC_VER_CONTAINER); // recVer
    assert_eq!((ver_and_instance >> 4) & 0x0FFF, 0); // recInstance
    assert_eq!(rec_type, REC_TYPE_SLIDE);
    assert_eq!(rec_len, 0x0001_2345);

    // An atom header packs recVer 0.
    let mut buf2 = Vec::new();
    assert!(push_record_header(
        &mut buf2,
        REC_VER_ATOM,
        REC_TYPE_TEXT_BYTES,
        4
    ));
    let vai = rd_u16(&buf2, 0).unwrap();
    assert_eq!(vai & 0x000F, REC_VER_ATOM);
    assert_eq!(rd_u16(&buf2, 2).unwrap(), REC_TYPE_TEXT_BYTES);
    assert_eq!(rd_u32(&buf2, 4).unwrap(), 4);
}

#[test]
fn oversize_body_is_skipped_not_wrapped() {
    // A body length beyond u32::MAX cannot be described; the header write must
    // refuse (return false) and append nothing, rather than wrap the length.
    let mut buf = Vec::new();
    let too_big = (u32::MAX as usize) + 1;
    // Only meaningful on 64-bit targets where such a usize exists.
    if too_big > u32::MAX as usize {
        assert!(!push_record_header(&mut buf, REC_VER_ATOM, REC_TYPE_TEXT_BYTES, too_big));
        assert!(buf.is_empty(), "nothing should be appended on overflow");
    }
}

// ---------------------------------------------------------------------------
// Structural: container count and the empty cases.
// ---------------------------------------------------------------------------

#[test]
fn multiple_slides_produce_multiple_containers() {
    let mut deck = Presentation::new();
    for i in 0..5 {
        deck.add_slide().add_text(&format!("slide {i}"));
    }
    let stream = build_powerpoint_document(&deck);
    let nodes = walk(&stream).unwrap();
    assert_eq!(nodes.len(), 5);
    for (i, node) in nodes.iter().enumerate() {
        assert_eq!(node, &Node::Slide(vec![format!("slide {i}")]));
    }
}

#[test]
fn empty_presentation_produces_empty_stream() {
    let deck = Presentation::new();
    let stream = build_powerpoint_document(&deck);
    assert!(stream.is_empty(), "no slides → no records");
    // It still wraps into a valid Compound File.
    let bytes = write_ppt(&deck);
    let cf = CompoundFile::open(&bytes).expect("empty deck still valid .ppt");
    // The stream reads back as empty (or all-padding), and the walker yields
    // zero nodes.
    let s = cf.read_stream(STREAM_POWERPOINT_DOCUMENT).unwrap_or_default();
    assert!(walk(&s).unwrap().is_empty());
}

#[test]
fn empty_slide_is_a_container_with_no_atoms() {
    let mut deck = Presentation::new();
    deck.add_slide(); // no add_text → zero paragraphs
    let stream = build_powerpoint_document(&deck);

    // One Slide container with an empty body: header only, recLen == 0.
    assert_eq!(stream.len(), 8, "just the 8-byte container header");
    let rec_ver = rd_u16(&stream, 0).unwrap() & 0x000F;
    let rec_type = rd_u16(&stream, 2).unwrap();
    let rec_len = rd_u32(&stream, 4).unwrap();
    assert_eq!(rec_ver, REC_VER_CONTAINER);
    assert_eq!(rec_type, REC_TYPE_SLIDE);
    assert_eq!(rec_len, 0);

    // The walker sees one Slide with no texts.
    let nodes = walk(&stream).unwrap();
    assert_eq!(nodes, vec![Node::Slide(Vec::new())]);
}

#[test]
fn walker_stops_on_zero_padding() {
    // Append trailing zero padding to a valid one-slide stream (simulating the
    // CFB sector padding) and confirm the walker stops cleanly.
    let mut deck = Presentation::new();
    deck.add_slide().add_text("only");
    let mut stream = build_powerpoint_document(&deck);
    stream.extend(std::iter::repeat_n(0u8, 100)); // pad like a sector would

    let nodes = walk(&stream).unwrap();
    assert_eq!(nodes, vec![Node::Slide(vec!["only".to_string()])]);
}

#[test]
fn output_is_deterministic() {
    let mut a = Presentation::new();
    a.add_slide().add_text("determinism");
    let mut b = Presentation::new();
    b.add_slide().add_text("determinism");
    assert_eq!(write_ppt(&a), write_ppt(&b));
}

#[test]
fn model_accessors_reflect_input() {
    let mut deck = Presentation::new();
    let s = deck.add_slide();
    s.add_text("a");
    s.add_text("b");
    assert_eq!(deck.slides().len(), 1);
    assert_eq!(deck.slides()[0].paragraphs(), &["a".to_string(), "b".to_string()]);
}
