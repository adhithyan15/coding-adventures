//! The media budget bounds **peak** memory, not just retained memory (#13672).
//!
//! `read_media_files` has refused over-budget packages for a while; the unit
//! test `media_expanding_past_the_budget_is_refused` proves the refusal
//! happens. What it cannot prove is *when*. The budget used to be checked
//! after each entry was decoded in full, so the entry that crossed the line had
//! already been held in memory -- up to each decoder's own 256 MiB ceiling --
//! by the time it was refused. A refusal that arrives after the allocation it
//! exists to prevent is a refusal in name only, and on wasm (32 MiB budget,
//! `panic = "abort"`) the allocation is the failure.
//!
//! So this test measures. A counting global allocator records the peak number
//! of live heap bytes while `read_media_files` runs, and each package below
//! carries a payload far past its budget. If any decoder expands the payload
//! before the budget is consulted, the peak shows it.
//!
//! ## Why this is its own test binary, with one test function
//!
//! A `#[global_allocator]` replaces the allocator for the *whole* binary, and
//! the harness runs tests in parallel threads. Any other test running
//! alongside would add its allocations to the count. One file, one `#[test]`,
//! and the cases run in sequence.
//!
//! ## How the threshold was chosen
//!
//! ```text
//!   path                    what a correct reader may hold at peak
//!   ---------------------   -----------------------------------------------
//!   DEFLATE (legacy)        nothing: the declared entry size is refused
//!                           before inflation starts
//!   zstd, size declared     nothing: the frame header is refused before
//!                           any block is decoded
//!   zstd, size undeclared   the output Vec up to the budget, and while it
//!                           grows, the old buffer beside the new one --
//!                           under 2x the budget
//! ```
//!
//! The undeclared zstd case is asserted under 2x the budget, the other two
//! under 1x (each plus 1 MiB of slack), and every payload is at least twice
//! its case's bound, so a reader that decodes first and checks after cannot
//! pass.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use engram_anki_package::read_media_files;
use zip::ZipWriter;

// ─── Counting allocator ──────────────────────────────────────────────────────

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn record_alloc(size: usize) {
    let live = LIVE.fetch_add(size, Ordering::SeqCst) + size;
    PEAK.fetch_max(live, Ordering::SeqCst);
}

// SAFETY: every method forwards to `System` with the caller's own arguments;
// the counters are bookkeeping only and never affect what is returned.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            record_alloc(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
    }

    /// Counted as "new block allocated, then old block freed", which is the
    /// worst case a moving `realloc` reaches: both buffers live at once. An
    /// in-place grow is over-counted, which only makes the bound stricter.
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            record_alloc(new_size);
            LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Peak live heap growth, in bytes, while `f` runs.
fn peak_during<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let base = LIVE.load(Ordering::SeqCst);
    PEAK.store(base, Ordering::SeqCst);
    let out = f();
    (out, PEAK.load(Ordering::SeqCst).saturating_sub(base))
}

// ─── Fixtures ────────────────────────────────────────────────────────────────

const MIB: usize = 1024 * 1024;

/// The crate's media budget for an archive of `archive_len` bytes, restated
/// from `media_budget` (private) for a native build: 50x the archive, clamped
/// to [16 MiB, 256 MiB]. Restating it is safe in one direction only, which is
/// the direction that matters: if the crate's real budget were larger than
/// this, a decode-first reader could hide under it -- but the payloads below
/// are at least twice the *bound* this implies, and the crate's budget can
/// only reach them by rising past the whole payload.
fn native_budget(archive_len: usize) -> usize {
    (archive_len.saturating_mul(50)).clamp(16 * MIB, 256 * MIB)
}

/// Protobuf for Anki's `MediaEntries { repeated MediaEntry entries = 1; }`
/// with one entry `{ name = 1, size = 2 }` -- hand-encoded, since the crate's
/// own message types are private.
fn media_entries_pb(name: &str, size: u32) -> Vec<u8> {
    fn varint(mut value: u64, out: &mut Vec<u8>) {
        while value >= 0x80 {
            out.push((value as u8) | 0x80);
            value >>= 7;
        }
        out.push(value as u8);
    }
    let mut entry = vec![0x0A];
    varint(name.len() as u64, &mut entry);
    entry.extend_from_slice(name.as_bytes());
    entry.push(0x10);
    varint(u64::from(size), &mut entry);

    let mut entries = vec![0x0A];
    varint(entry.len() as u64, &mut entries);
    entries.extend_from_slice(&entry);
    entries
}

/// A modern (`collection.anki21b`) package whose single media file is
/// `payload_frame`, a zstd frame.
fn modern_package(payload_frame: &[u8], declared: usize) -> Vec<u8> {
    let mut writer = ZipWriter::new();
    let collection = zstd_crate::stream::encode_all(&b"not read by this test"[..], 0).unwrap();
    writer.add_file("collection.anki21b", &collection, false);
    let map = zstd_crate::stream::encode_all(&media_entries_pb("huge.wav", declared as u32)[..], 0)
        .unwrap();
    writer.add_file("media", &map, false);
    writer.add_file("0", payload_frame, false);
    writer.finish()
}

/// A raw DEFLATE stream that inflates to `1 + 258 * matches` zero bytes.
///
/// Hand-built rather than produced by `zip::raw_deflate`, which takes minutes
/// on tens of megabytes in a debug build. One fixed-Huffman block (RFC 1951
/// §3.2.6): a literal 0, then `matches` copies of "length 258, distance 1",
/// then end-of-block. Huffman codes are written most-significant bit first
/// into the least-significant-bit-first stream, as the format requires.
fn zero_bomb_deflate(matches: usize) -> Vec<u8> {
    struct Bits {
        out: Vec<u8>,
        acc: u32,
        n: u32,
    }
    impl Bits {
        fn raw(&mut self, value: u32, width: u32) {
            for i in 0..width {
                self.bit((value >> i) & 1);
            }
        }
        fn code(&mut self, value: u32, width: u32) {
            for i in (0..width).rev() {
                self.bit((value >> i) & 1);
            }
        }
        fn bit(&mut self, bit: u32) {
            self.acc |= bit << self.n;
            self.n += 1;
            if self.n == 8 {
                self.out.push(self.acc as u8);
                self.acc = 0;
                self.n = 0;
            }
        }
    }
    let mut bits = Bits {
        out: Vec::new(),
        acc: 0,
        n: 0,
    };
    bits.raw(1, 1); // BFINAL
    bits.raw(1, 2); // BTYPE = 01, fixed Huffman
    bits.code(0x30, 8); // literal 0: codes 0..=143 are 0x30 + literal, 8 bits
    for _ in 0..matches {
        bits.code(0b1100_0101, 8); // length code 285 = 258, no extra bits
        bits.code(0, 5); // distance code 0 = 1, no extra bits
    }
    bits.code(0, 7); // end of block: code 256, 7 bits of zero
    if bits.n > 0 {
        bits.out.push(bits.acc as u8);
    }
    bits.out
}

/// A minimal ZIP archive, written by hand so one member can carry a
/// precomputed DEFLATE stream. Each member is `(name, method, stored bytes,
/// uncompressed size, crc32)`.
fn zip_by_hand(members: &[(&str, u16, Vec<u8>, u32, u32)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, method, data, size, crc) in members {
        let offset = out.len() as u32;
        let header = |sig: u32, central: bool| {
            let mut h = Vec::new();
            h.extend_from_slice(&sig.to_le_bytes());
            if central {
                h.extend_from_slice(&20u16.to_le_bytes()); // version made by
            }
            h.extend_from_slice(&20u16.to_le_bytes()); // version needed
            h.extend_from_slice(&0u16.to_le_bytes()); // flags
            h.extend_from_slice(&method.to_le_bytes());
            h.extend_from_slice(&[0; 4]); // time, date
            h.extend_from_slice(&crc.to_le_bytes());
            h.extend_from_slice(&(data.len() as u32).to_le_bytes());
            h.extend_from_slice(&size.to_le_bytes());
            h.extend_from_slice(&(name.len() as u16).to_le_bytes());
            h.extend_from_slice(&0u16.to_le_bytes()); // extra length
            if central {
                h.extend_from_slice(&[0; 6]); // comment len, disk start, internal attrs
                h.extend_from_slice(&[0; 4]); // external attrs
                h.extend_from_slice(&offset.to_le_bytes());
            }
            h.extend_from_slice(name.as_bytes());
            h
        };
        out.extend_from_slice(&header(0x0403_4B50, false));
        out.extend_from_slice(data);
        central.extend_from_slice(&header(0x0201_4B50, true));
    }
    let cd_offset = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4B50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]); // disk numbers
    out.extend_from_slice(&(members.len() as u16).to_le_bytes());
    out.extend_from_slice(&(members.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment length
    out
}

/// A legacy (`collection.anki2`) package whose single media file is
/// `1 + 258 * matches` zeros, DEFLATE-compressed inside the zip.
fn legacy_package(matches: usize) -> (Vec<u8>, usize) {
    let payload = 1 + 258 * matches;
    let stored = |name: &'static str, data: &[u8]| {
        (
            name,
            0u16,
            data.to_vec(),
            data.len() as u32,
            zip::crc32(data, 0),
        )
    };
    let zeros = vec![0u8; payload];
    let bomb = (
        "0",
        8u16,
        zero_bomb_deflate(matches),
        payload as u32,
        zip::crc32(&zeros, 0),
    );
    drop(zeros);
    let apkg = zip_by_hand(&[
        stored("collection.anki2", b"not read by this test"),
        stored("media", br#"{"0": "huge.wav"}"#),
        bomb,
    ]);
    (apkg, payload)
}

/// Run one case: assert the premise, measure the read, assert the refusal and
/// the peak.
fn assert_refused_within_budget(case: &str, apkg: &[u8], payload: usize, slack_budgets: usize) {
    let budget = native_budget(apkg.len());
    // The archive is already in memory before the call; everything the reader
    // allocates on top of that is what this bounds. 1 MiB of slack covers
    // the zip index, the manifest, and the decoders' fixed tables.
    let bound = slack_budgets * budget + MIB;
    assert!(
        payload >= 2 * bound,
        "{case}: the payload ({payload}) must dwarf the bound ({bound}), or a          decode-first reader could pass"
    );

    let (result, peak) = peak_during(|| read_media_files(apkg));
    // Only the size is reported on success: formatting the Ok value would
    // render the payload a byte at a time.
    let error = match result {
        Err(error) => error,
        Ok(files) => panic!("{case}: must be refused, got {} file(s)", files.len()),
    };
    assert!(
        error.message.contains("expands to more than"),
        "{case}: the refusal must be the media budget's, got: {}",
        error.message
    );

    eprintln!("{case}: peak heap growth {peak} bytes, bound {bound}");
    assert!(
        peak <= bound,
        "{case}: peak heap growth was {peak} bytes; a reader that honours the \
         {budget}-byte budget stays under {bound}"
    );
}

#[test]
fn over_budget_media_is_refused_before_it_is_decoded() {
    // DEFLATE: the zip entry declares 64 MiB. Refused from the declaration,
    // so the reader allocates essentially nothing for it -- well under one
    // budget, where a decode-first reader would reach 64 MiB.
    let (apkg, payload) = legacy_package(64 * MIB / 258);
    // Positive control: the hand-built member really is a valid entry that
    // inflates to the payload, so a refusal below is the budget's and not a
    // corrupt-archive error. (Allocated before the measurement starts.)
    let inflated = zip::ZipReader::new(&apkg)
        .unwrap()
        .read_by_name("0")
        .unwrap();
    assert_eq!(inflated.len(), payload);
    assert!(inflated.iter().all(|&b| b == 0));
    drop(inflated);
    assert_refused_within_budget("legacy DEFLATE", &apkg, payload, 1);

    // zstd with Frame_Content_Size (libzstd's one-shot API writes it): the
    // header is refused before any block is decoded.
    let payload = 128 * MIB;
    let frame = zstd_crate::bulk::compress(&vec![0u8; payload], 3).unwrap();
    let apkg = modern_package(&frame, payload);
    assert_refused_within_budget("zstd, size declared", &apkg, payload, 1);

    // zstd without Frame_Content_Size (the streaming API omits it): only the
    // incremental check can stop this one, at the block that crosses the
    // budget, with the output buffer's growth as the only overhead.
    let frame = zstd_crate::stream::encode_all(std::io::repeat(0).take_bytes(payload), 3).unwrap();
    let apkg = modern_package(&frame, payload);
    assert_refused_within_budget("zstd, size undeclared", &apkg, payload, 2);
}

/// `Read::take` with a `usize` count, so the call above reads as prose.
trait TakeBytes: std::io::Read + Sized {
    fn take_bytes(self, n: usize) -> std::io::Take<Self> {
        self.take(n as u64)
    }
}
impl<R: std::io::Read> TakeBytes for R {}
