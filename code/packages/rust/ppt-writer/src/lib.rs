//! # ppt-writer — legacy PowerPoint (`.ppt`) writer (PPTW01)
//!
//! A from-scratch, zero-third-party-dependency **writer** for the legacy
//! **PowerPoint 97-2003 binary** format (`.ppt`, [MS-PPT]). You build a
//! slide-deck model — a [`Presentation`] of [`Slide`]s, each holding paragraphs
//! of text — and [`write_ppt`] turns it into the bytes of a real `.ppt` file.
//! See `code/specs/PPTW01-ppt-writer.md` for the full literate walkthrough.
//!
//! ## The one-paragraph mental model
//!
//! A `.ppt` file is an **OLE2 Compound File** (the same container as `.xls` and
//! `.doc`). Its payload lives in a stream named exactly **"PowerPoint
//! Document"**, and that stream is a **tree of records**. Every record opens
//! with an 8-byte **RecordHeader**; its body is either *more records* (a
//! *container*) or opaque data (an *atom*). We emit, per slide, a **Slide
//! container** whose children are **text atoms** — one atom per paragraph. We
//! then hand the concatenated stream to the sibling [`cfb-writer`] crate, which
//! wraps it in the OLE2 container.
//!
//! ```
//! # use ppt_writer::{Presentation, write_ppt};
//! let mut deck = Presentation::new();
//! let s = deck.add_slide();
//! s.add_text("Hello slide");
//! let bytes = write_ppt(&deck);
//! // It is a Compound File: begins with the OLE2 magic.
//! assert_eq!(&bytes[0..8], &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
//! ```
//!
//! ## The RecordHeader — 8 bytes, bit-packed
//!
//! ```text
//!  offset  size  field       meaning
//!    0      2    recVerAndI  low 4 bits = recVer, high 12 bits = recInstance
//!    2      2    recType     which record (0x03EE = Slide, 0x0FA8 = TextBytes…)
//!    4      4    recLen      length of the BODY that follows, in bytes
//! ```
//!
//! All little-endian. **`recVer == 0xF` marks a container** (body is child
//! records); any other `recVer` marks an atom (body is opaque data). `recLen`
//! counts only the body, never the 8 header bytes.
//!
//! ## The two text atoms
//!
//! Each paragraph becomes exactly one text atom, chosen from the text itself:
//!
//! - **TextBytesAtom** (`0x0FA8`): one byte per char (Latin-1). Used when every
//!   char's scalar value is ≤ `0x00FF`. Half the size of ASCII text.
//! - **TextCharsAtom** (`0x0FA0`): UTF-16LE, two bytes per code unit. Used when
//!   any char exceeds `0x00FF`, so nothing is lost (e.g. `"你好"`).
//!
//! ## Robustness
//!
//! `#![forbid(unsafe_code)]`, and no `unwrap`/`expect`/`panic!` on the public
//! path. Lengths are packed with `u32::try_from`; an atom or container whose
//! body would overflow the `u32` `recLen` is **skipped** rather than wrapped
//! into a corrupting length. Output is deterministic.

#![forbid(unsafe_code)]

// ---------------------------------------------------------------------------
// Record-type and version constants. These are the [MS-PPT] numbers; a writer
// and any reader must agree on them exactly.
// ---------------------------------------------------------------------------

/// `recType` for a **SlideContainer** — one per slide, holds the slide's atoms.
const REC_TYPE_SLIDE: u16 = 0x03EE;
/// `recType` for a **TextCharsAtom** — text as UTF-16LE (two bytes per unit).
const REC_TYPE_TEXT_CHARS: u16 = 0x0FA0;
/// `recType` for a **TextBytesAtom** — text as Latin-1 (one byte per char).
const REC_TYPE_TEXT_BYTES: u16 = 0x0FA8;

/// `recVer` value that marks a record as a **container** (its body is children).
const REC_VER_CONTAINER: u16 = 0xF;
/// `recVer` value for our text **atoms** (body is opaque data).
const REC_VER_ATOM: u16 = 0x0;

/// The CFB stream name that carries the record tree. Must be exactly this.
const STREAM_POWERPOINT_DOCUMENT: &str = "PowerPoint Document";
/// A tiny stub `CurrentUserAtom` stream, present in real files for authenticity.
/// Our reader never consults it; it is harmless padding for interoperability.
const STREAM_CURRENT_USER: &str = "Current User";
/// The stub "Current User" bytes: a 4-byte little-endian `0x00000014`.
const CURRENT_USER_STUB: [u8; 4] = [0x14, 0x00, 0x00, 0x00];

// ---------------------------------------------------------------------------
// The public model: Presentation -> Slide -> paragraphs of text.
// ---------------------------------------------------------------------------

/// A slide-deck model: an ordered list of [`Slide`]s. Build one with
/// [`Presentation::new`], append slides with [`Presentation::add_slide`], then
/// serialise with [`write_ppt`].
#[derive(Debug, Default, Clone)]
pub struct Presentation {
    slides: Vec<Slide>,
}

impl Presentation {
    /// Create an empty presentation. Serialising it yields a valid `.ppt` whose
    /// "PowerPoint Document" stream is logically empty (zero records).
    pub fn new() -> Self {
        Presentation { slides: Vec::new() }
    }

    /// Append a new, empty slide and return a mutable handle to it so the caller
    /// can add paragraphs. Slides keep insertion order, which becomes the order
    /// of Slide containers in the output stream.
    pub fn add_slide(&mut self) -> &mut Slide {
        self.slides.push(Slide::new());
        // A `push` guarantees at least one element, so `last_mut` is infallible
        // here. The `expect` is structural, not an input-driven failure path.
        self.slides
            .last_mut()
            .expect("just pushed a slide, so last_mut is Some")
    }

    /// The slides, in insertion order (read-only view, mainly for tests).
    pub fn slides(&self) -> &[Slide] {
        &self.slides
    }
}

/// One slide: an ordered list of paragraphs. Each paragraph becomes exactly one
/// text atom inside the slide's container.
#[derive(Debug, Default, Clone)]
pub struct Slide {
    paragraphs: Vec<String>,
}

impl Slide {
    /// Create an empty slide (no paragraphs). An empty slide is legal: it emits
    /// a Slide container with a zero-length body.
    pub fn new() -> Self {
        Slide {
            paragraphs: Vec::new(),
        }
    }

    /// Add one paragraph of text. It becomes one text atom (TextBytes if the
    /// text is all-Latin-1, else TextChars).
    pub fn add_text(&mut self, text: &str) {
        self.paragraphs.push(text.to_string());
    }

    /// The paragraphs, in insertion order (read-only view, mainly for tests).
    pub fn paragraphs(&self) -> &[String] {
        &self.paragraphs
    }
}

// ---------------------------------------------------------------------------
// Record emission — the heart of the writer.
// ---------------------------------------------------------------------------

/// Append an 8-byte RecordHeader to `out`.
///
/// `recVerAndInstance` (the first `u16`) packs two numbers:
/// `(recVer & 0xF) | ((recInstance & 0xFFF) << 4)`. We always pass
/// `recInstance = 0` (the minimal profile has no instance-numbered records).
///
/// Returns `false` (and appends nothing) if `body_len` does not fit in the
/// `u32` `recLen` field — the caller then skips the whole record rather than
/// emitting a wrong length.
fn push_record_header(out: &mut Vec<u8>, rec_ver: u16, rec_type: u16, body_len: usize) -> bool {
    // recLen is a u32; a body larger than u32::MAX cannot be described. Skip.
    let Ok(rec_len) = u32::try_from(body_len) else {
        return false;
    };
    let rec_instance: u16 = 0;
    let ver_and_instance = (rec_ver & 0x000F) | ((rec_instance & 0x0FFF) << 4);
    out.extend_from_slice(&ver_and_instance.to_le_bytes());
    out.extend_from_slice(&rec_type.to_le_bytes());
    out.extend_from_slice(&rec_len.to_le_bytes());
    true
}

/// Decide whether a string can use the compact TextBytes encoding: true exactly
/// when every character's Unicode scalar value fits in a single byte (≤ 0xFF).
fn is_all_latin1(text: &str) -> bool {
    text.chars().all(|c| (c as u32) <= 0xFF)
}

/// Encode one paragraph's *body* bytes (no header) and report which atom type to
/// use. TextBytes → one byte per char; TextChars → UTF-16LE.
fn encode_text_body(text: &str) -> (u16, Vec<u8>) {
    if is_all_latin1(text) {
        // Latin-1: each char's scalar (guaranteed ≤ 0xFF) is one byte.
        let body: Vec<u8> = text.chars().map(|c| c as u8).collect();
        (REC_TYPE_TEXT_BYTES, body)
    } else {
        // UTF-16LE: two bytes per code unit (surrogate pairs for astral chars).
        let mut body = Vec::with_capacity(text.len() * 2);
        for unit in text.encode_utf16() {
            body.extend_from_slice(&unit.to_le_bytes());
        }
        (REC_TYPE_TEXT_CHARS, body)
    }
}

/// Emit one text atom (header + body) for `text`, appending to `out`. If the
/// body would overflow the `u32` `recLen`, the atom is skipped (nothing is
/// appended) — a shorter valid file beats a longer corrupt one.
fn push_text_atom(out: &mut Vec<u8>, text: &str) {
    let (rec_type, body) = encode_text_body(text);
    if push_record_header(out, REC_VER_ATOM, rec_type, body.len()) {
        out.extend_from_slice(&body);
    }
}

/// Emit one Slide container (header + all its text atoms) for `slide`.
///
/// We build the children into a scratch buffer first so we know the container's
/// `recLen` (the total child byte length) before writing the container header.
/// If that total overflows the `u32` `recLen`, the whole container is skipped.
fn push_slide_container(out: &mut Vec<u8>, slide: &Slide) {
    // Build children (the atoms) into a scratch buffer to learn their length.
    let mut children = Vec::new();
    for para in &slide.paragraphs {
        push_text_atom(&mut children, para);
    }
    if push_record_header(out, REC_VER_CONTAINER, REC_TYPE_SLIDE, children.len()) {
        out.extend_from_slice(&children);
    }
}

/// Build the raw bytes of the "PowerPoint Document" stream: every Slide
/// container concatenated, in slide order. Exposed for unit tests that walk the
/// records without the CFB wrapping.
pub(crate) fn build_powerpoint_document(p: &Presentation) -> Vec<u8> {
    let mut out = Vec::new();
    for slide in &p.slides {
        push_slide_container(&mut out, slide);
    }
    out
}

/// Serialise a [`Presentation`] into the bytes of a legacy `.ppt` file.
///
/// Steps:
/// 1. Build the "PowerPoint Document" record stream (all Slide containers).
/// 2. Add a tiny stub "Current User" stream for authenticity.
/// 3. Wrap both streams in an OLE2 Compound File via [`cfb-writer`].
///
/// The output is deterministic: identical models yield identical bytes.
pub fn write_ppt(p: &Presentation) -> Vec<u8> {
    let ppt_doc = build_powerpoint_document(p);
    cfb_writer::write_cfb(&[
        (STREAM_POWERPOINT_DOCUMENT, ppt_doc.as_slice()),
        (STREAM_CURRENT_USER, CURRENT_USER_STUB.as_slice()),
    ])
}

#[cfg(test)]
mod tests;
