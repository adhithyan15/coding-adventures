//! The PDF file-structure writer: header, body, cross-reference table, trailer.
//!
//! ## The shape of a PDF file
//!
//! ```text
//!   %PDF-1.7                     <- header
//!   %<binary comment>            <- marks the file as binary, not text
//!
//!   1 0 obj                      <- body: numbered indirect objects
//!   << /Type /Catalog … >>
//!   endobj
//!   …
//!
//!   xref                         <- cross-reference table
//!   0 4
//!   0000000000 65535 f␍␊         <- 20 bytes per entry, exactly
//!   0000000015 00000 n␍␊
//!   …
//!
//!   trailer
//!   << /Size 4 /Root 1 0 R >>
//!   startxref
//!   408                          <- BYTE OFFSET of the `xref` keyword
//!   %%EOF
//! ```
//!
//! ## Why this is mostly an exercise in counting bytes
//!
//! Two things in that layout are byte offsets into the file itself: every xref
//! entry, and `startxref`. A reader opens a PDF by seeking to the end, reading
//! `startxref`, jumping to the xref table, and then jumping directly to each
//! object's recorded offset. **Nothing is scanned for.** So an offset that is
//! wrong by one byte does not degrade — the reader lands mid-token and the file
//! is broken.
//!
//! That is why this writer tracks position as it appends rather than computing
//! offsets afterwards from a finished buffer: the moment an offset is derived
//! from a second traversal, it can disagree with the first.
//!
//! The xref entry format is also **fixed-width and load-bearing**: exactly
//! twenty bytes, `nnnnnnnnnn ggggg n eol`, where the two-character `eol` makes
//! the arithmetic work. Readers index into the table by multiplying, so a
//! shorter line silently shifts every subsequent entry.
//!
//! ## The free-object chain
//!
//! Object 0 is always present, always free, and always has generation 65535. It
//! is the head of the linked list of free entries. In a freshly written file
//! nothing else is free, so it points at itself (offset 0).

use crate::object::{Dict, ObjId, Object};

/// PDF's binary marker comment.
///
/// The second line of a PDF should contain bytes above 127 so that tools which
/// sniff text-versus-binary do not treat the file as text and "helpfully"
/// translate line endings — which would corrupt every stream and every offset.
const BINARY_MARKER: [u8; 6] = [b'%', 0xE2, 0xE3, 0xCF, 0xD3, b'\n'];

/// Builds a PDF file.
///
/// Objects are added and given identities up front, so they can reference each
/// other before being written — a page needs its parent's id, and the parent
/// needs the page's, so one of them must be able to name an object that does
/// not exist yet.
#[derive(Debug, Default)]
pub struct PdfWriter {
    objects: Vec<Option<Object>>,
}

impl PdfWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reserve an id without supplying the object yet.
    ///
    /// This is what makes cyclic references expressible. `finish` refuses to
    /// write a file with an id still unfilled, rather than emitting `null` and
    /// producing a structurally valid file that means something different from
    /// what the caller intended.
    pub fn reserve(&mut self) -> ObjId {
        self.objects.push(None);
        ObjId::new(self.objects.len() as u32)
    }

    /// Fill a reserved id.
    ///
    /// # Panics
    /// If `id` was not produced by this writer.
    pub fn fill(&mut self, id: ObjId, object: Object) {
        let index = (id.number as usize)
            .checked_sub(1)
            .filter(|i| *i < self.objects.len())
            .expect("object id was not reserved by this writer");
        self.objects[index] = Some(object);
    }

    /// Add an object and return its new id.
    pub fn add(&mut self, object: Object) -> ObjId {
        let id = self.reserve();
        self.fill(id, object);
        id
    }

    /// Number of objects reserved so far.
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Serialise the file, with `root` as the document catalog.
    ///
    /// Returns an error rather than writing a broken file if any reserved id
    /// was never filled.
    pub fn finish(&self, root: ObjId) -> Result<Vec<u8>, PdfError> {
        if let Some(index) = self.objects.iter().position(Option::is_none) {
            return Err(PdfError::UnfilledObject(index as u32 + 1));
        }
        if (root.number as usize) > self.objects.len() || root.number == 0 {
            return Err(PdfError::UnknownRoot(root.number));
        }

        let mut out = Vec::new();
        out.extend_from_slice(b"%PDF-1.7\n");
        out.extend_from_slice(&BINARY_MARKER);

        // Offsets are recorded as the body is appended. `offsets[i]` is where
        // object i+1 begins.
        let mut offsets = Vec::with_capacity(self.objects.len());
        for (index, object) in self.objects.iter().enumerate() {
            let object = object.as_ref().expect("checked above");
            offsets.push(out.len());
            let number = index + 1;
            out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
            match object {
                Object::Stream { dict, data } => write_stream(dict, data, &mut out),
                other => {
                    other.write(&mut out);
                    out.push(b'\n');
                }
            }
            out.extend_from_slice(b"endobj\n");
        }

        let xref_offset = out.len();
        out.extend_from_slice(b"xref\n");
        out.extend_from_slice(format!("0 {}\n", self.objects.len() + 1).as_bytes());
        // Object 0: head of the free list, pointing at itself.
        out.extend_from_slice(b"0000000000 65535 f \n");
        for offset in &offsets {
            // Exactly 20 bytes: 10 digits, space, 5 digits, space, type, 2 eol.
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }

        let mut trailer = Dict::new();
        trailer.set("Size", Object::Int(self.objects.len() as i64 + 1));
        trailer.set("Root", Object::Ref(root));
        out.extend_from_slice(b"trailer\n");
        Object::Dict(trailer).write(&mut out);
        out.extend_from_slice(b"\nstartxref\n");
        out.extend_from_slice(format!("{xref_offset}\n").as_bytes());
        out.extend_from_slice(b"%%EOF\n");

        Ok(out)
    }
}

/// Write a stream object, deriving `/Length` from the data.
///
/// `/Length` is filled in here rather than taken from the caller's dictionary
/// because a `/Length` that disagrees with the actual bytes produces a file
/// some readers accept and others reject — the worst failure mode available,
/// since it looks fine until it reaches a different reader.
fn write_stream(dict: &Dict, data: &[u8], out: &mut Vec<u8>) {
    let mut dict = dict.clone();
    dict.set("Length", Object::Int(data.len() as i64));
    Object::Dict(dict).write(out);
    out.extend_from_slice(b"\nstream\n");
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream\n");
}

/// Compress bytes with PDF's `FlateDecode` filter.
///
/// ## `FlateDecode` is zlib, not raw deflate
///
/// This is the trap, and it is worth stating plainly because the two are
/// trivially confusable and the failure is silent on our side. PDF's
/// `FlateDecode` is **RFC 1950 zlib**: a two-byte header, the RFC 1951 deflate
/// payload, and a four-byte Adler-32 checksum. ZIP method 8, by contrast, is
/// the bare deflate stream with no wrapper at all — which is what
/// `zip::raw_deflate` produces, correctly, for its own format.
///
/// Handing raw deflate to a PDF reader yields `unknown compression method`,
/// because the first byte of a deflate block is read as the zlib CMF. Our own
/// reader would have inflated it back happily; `qpdf` did not, which is the
/// entire reason the oracle is a hard gate.
pub fn flate_encode(data: &[u8]) -> (Vec<u8>, Object) {
    let mut out = Vec::new();
    // CMF: compression method 8 (deflate) with a 32 KiB window.
    // FLG: chosen so that (CMF << 8 | FLG) is a multiple of 31, which is the
    // header's own check constraint. 0x78 0x9C is the conventional pair.
    out.push(0x78);
    out.push(0x9C);
    out.extend_from_slice(&zip::raw_deflate(data));
    out.extend_from_slice(&adler32(data).to_be_bytes());
    (out, Object::name("FlateDecode"))
}

/// Adler-32 over `data`, as RFC 1950 specifies for the zlib trailer.
///
/// Two running sums modulo 65521 — the largest prime below 2^16. `s1` is the
/// sum of the bytes; `s2` is the sum of the successive `s1` values, which is
/// what makes it sensitive to ordering rather than only to content.
fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let mut s1: u32 = 1;
    let mut s2: u32 = 0;
    for &byte in data {
        s1 = (s1 + byte as u32) % MOD;
        s2 = (s2 + s1) % MOD;
    }
    (s2 << 16) | s1
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdfError {
    /// An id was reserved but never filled. Writing it as `null` would produce
    /// a structurally valid file that silently means something else.
    UnfilledObject(u32),
    UnknownRoot(u32),
    /// The document as described cannot be written -- a page tree with no
    /// pages, or a content stream mirrored about the wrong page height. These
    /// are caught here because the resulting file would be *accepted* by a
    /// reader and simply be wrong.
    Invalid(String),
}

impl std::fmt::Display for PdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdfError::Invalid(message) => write!(f, "{message}"),
            PdfError::UnfilledObject(number) => write!(
                f,
                "object {number} was reserved but never filled; writing it \
                 would produce a file that parses but means something else"
            ),
            PdfError::UnknownRoot(number) => {
                write!(f, "root object {number} was never reserved by this writer")
            }
        }
    }
}

impl std::error::Error for PdfError {}
