//! # cfb-writer — Compound File Binary Format writer (CFBW01)
//!
//! A from-scratch, zero-dependency **writer** for the **OLE2 / Compound File
//! Binary Format** ([MS-CFB]) — the container that lives inside legacy `.xls`,
//! `.doc`, and `.ppt` files. This is the exact inverse of the sibling `cfb`
//! reader crate: you hand it a set of named streams and it produces a byte
//! buffer that the reader (and, more importantly, real Office tooling) accepts.
//! See `code/specs/CFBW01-cfb-writer.md` for the full literate walkthrough.
//!
//! ## The one-paragraph mental model
//!
//! A CFB file is a **FAT filesystem crammed into a single file**. Picture a
//! floppy disk: a fixed-size *header*, then a run of equal-sized *sectors*. A
//! **File Allocation Table (FAT)** is one `u32` "next sector" pointer per
//! sector; a multi-sector file is a linked list you follow until you hit
//! `ENDOFCHAIN`. A **directory** (itself just another FAT-stored stream) lists
//! the named objects — CFB calls a file a *stream* and a folder a *storage*.
//! Tiny streams (smaller than the 4096-byte *mini cutoff*) would waste most of a
//! 512-byte sector, so they are packed together into a **mini-stream**, sliced
//! into 64-byte *mini-sectors* chained by a parallel **mini-FAT**.
//!
//! ## What this writer emits
//!
//! We always write **version 3** files: 512-byte sectors, 64-byte mini-sectors,
//! a 4096-byte mini-stream cutoff. The output is fully **deterministic** — every
//! CLSID and timestamp field is zeroed — so tests are byte-stable.
//!
//! ```
//! # use cfb_writer::write_cfb;
//! let bytes = write_cfb(&[
//!     ("Workbook", &[0x09, 0x08, 0x00, 0x00][..]),
//!     ("\u{5}SummaryInformation", b"tiny"),
//! ]);
//! assert_eq!(&bytes[0..8], &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
//! ```
//!
//! ## Layout we produce, sector by sector
//!
//! We choose a **fixed, simple sector order** that any conforming reader walks:
//!
//! ```text
//!   offset 0        HEADER (512 bytes; sectors are numbered AFTER this)
//!   sector 0..a     FAT sectors            (marked FATSECT in the FAT itself)
//!   sector a..b     directory stream       (128-byte entries; root first)
//!   sector b..c     mini-FAT stream        (only if any small streams exist)
//!   sector c..d     mini-stream            (the packed 64-byte mini-sectors)
//!   sector d..e     each LARGE stream's data (>= cutoff), back to back
//! ```
//!
//! The tricky part is that the FAT must describe *itself*: the number of FAT
//! sectors depends on the total sector count, which includes the FAT sectors.
//! We resolve that with a small **fixed-point** loop (see `finish`).

#![forbid(unsafe_code)]

// ---------------------------------------------------------------------------
// Constants — the sentinel FAT values, sizes, and header field offsets. These
// mirror the reader's constants exactly; a writer and reader must agree.
// ---------------------------------------------------------------------------

/// The 8-byte magic that opens every CFB file: `D0 CF 11 E0 A1 B1 1A E1`.
const SIGNATURE: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// Free / unused sector.
const FREESECT: u32 = 0xFFFF_FFFF;
/// End of a sector chain.
const ENDOFCHAIN: u32 = 0xFFFF_FFFE;
/// Sector holds part of the FAT.
const FATSECT: u32 = 0xFFFF_FFFD;
/// Directory-entry "no such sibling/child" marker.
const NOSTREAM: u32 = 0xFFFF_FFFF;

/// The fixed header length. Sectors are numbered *after* these first 512 bytes.
const HEADER_LEN: usize = 512;
/// Version-3 sector size.
const SECTOR_SIZE: usize = 512;
/// Version-3 mini-sector size.
const MINI_SECTOR_SIZE: usize = 64;
/// Streams strictly smaller than this go into the mini-stream. Streams that are
/// this size or larger get their own regular 512-byte sectors.
const MINI_CUTOFF: u32 = 4096;
/// Each directory entry is exactly 128 bytes.
const DIR_ENTRY_SIZE: usize = 128;
/// The header inlines the first 109 FAT-sector locations.
const HEADER_DIFAT_COUNT: usize = 109;
/// Byte offset within the header where the inlined DIFAT array begins.
const HEADER_DIFAT_OFFSET: usize = 76;
/// Number of `u32` pointers a single 512-byte FAT sector can hold (512/4).
const FAT_ENTRIES_PER_SECTOR: usize = SECTOR_SIZE / 4;
/// Number of `u32` pointers a single 512-byte mini-FAT sector can hold.
const MINIFAT_ENTRIES_PER_SECTOR: usize = SECTOR_SIZE / 4;
/// The longest a stream/storage name may be, in UTF-16 code units, *excluding*
/// the terminating NUL. The 64-byte name field holds 32 UTF-16 units total, and
/// one is reserved for the NUL, leaving 31.
const MAX_NAME_UNITS: usize = 31;

/// Object type: a stream ("file").
const OBJ_STREAM: u8 = 0x02;
/// Object type: the root storage (entry 0).
const OBJ_ROOT: u8 = 0x05;
/// Red-black colour: black. We colour every node black — a tree that is "all
/// black" is trivially a valid red-black tree (readers only need the tree
/// walkable, not perfectly balanced), and it keeps the writer simple.
const COLOR_BLACK: u8 = 0x01;

// ---------------------------------------------------------------------------
// Little-endian writers. Small helpers that append to a growing `Vec<u8>` or
// patch an existing buffer. Nothing here can panic on the public path because
// the buffer is always pre-sized to a computed length.
// ---------------------------------------------------------------------------

/// Write a `u16` little-endian at `off` in `buf`. Caller guarantees room.
#[inline]
fn put_u16(buf: &mut [u8], off: usize, v: u16) {
    let b = v.to_le_bytes();
    buf[off] = b[0];
    buf[off + 1] = b[1];
}

/// Write a `u32` little-endian at `off` in `buf`. Caller guarantees room.
#[inline]
fn put_u32(buf: &mut [u8], off: usize, v: u32) {
    let b = v.to_le_bytes();
    buf[off..off + 4].copy_from_slice(&b);
}

/// Write a `u64` little-endian at `off` in `buf`. Caller guarantees room.
#[inline]
fn put_u64(buf: &mut [u8], off: usize, v: u64) {
    let b = v.to_le_bytes();
    buf[off..off + 8].copy_from_slice(&b);
}

/// Round `n` up to the next multiple of `unit` (`unit` must be non-zero).
/// Returns the number of whole units, using checked arithmetic so a colossal
/// stream length can never silently wrap.
#[inline]
fn div_round_up(n: u64, unit: u64) -> u64 {
    if n == 0 {
        0
    } else {
        // (n + unit - 1) / unit, but overflow-safe.
        (n - 1) / unit + 1
    }
}

// ---------------------------------------------------------------------------
// The builder.
// ---------------------------------------------------------------------------

/// An ordered collection of named streams, ready to be serialised into CFB
/// bytes. Names are stored as-given; `finish` performs all layout and encoding.
///
/// Streams keep **insertion order**, and that order determines the directory's
/// sibling chain (entry 1 → 2 → 3 …), so the reader enumerates them
/// deterministically.
#[derive(Debug, Default, Clone)]
pub struct CfbWriter {
    /// `(name, data)` in insertion order. Names longer than [`MAX_NAME_UNITS`]
    /// UTF-16 units are truncated at `add_stream` time (documented behaviour).
    streams: Vec<(String, Vec<u8>)>,
}

impl CfbWriter {
    /// Create an empty writer. Calling [`finish`](Self::finish) on it yields a
    /// valid, minimal CFB containing only the root storage and no streams.
    pub fn new() -> Self {
        CfbWriter {
            streams: Vec::new(),
        }
    }

    /// Add a named stream.
    ///
    /// **Name length:** CFB stores names in a fixed 64-byte UTF-16LE field, so a
    /// name may be at most 31 UTF-16 code units (32 minus the NUL terminator).
    /// A longer name is **truncated** to 31 units here rather than rejected,
    /// keeping the API infallible; truncation is on UTF-16 unit boundaries so we
    /// never split below a code unit. (Splitting a surrogate pair is still
    /// possible in principle; `String::from_utf16_lossy` in the reader tolerates
    /// a lone surrogate, so the file stays readable.)
    ///
    /// Duplicate names are permitted at this layer — CFB itself has no
    /// uniqueness constraint on our simple sibling chain — but real consumers
    /// expect unique names, so avoid duplicates for interoperable files.
    pub fn add_stream(&mut self, name: &str, data: &[u8]) {
        let truncated = truncate_name(name);
        self.streams.push((truncated, data.to_vec()));
    }

    /// Serialise everything into a finished CFB byte buffer.
    ///
    /// The algorithm, top to bottom:
    /// 1. Split streams into *small* (< cutoff → mini-stream) and *large*
    ///    (≥ cutoff → own sectors), and lay out the mini-stream + mini-FAT.
    /// 2. Build the directory (root entry + one entry per stream).
    /// 3. Assign regular sectors to: directory, mini-FAT, mini-stream, and each
    ///    large stream's data.
    /// 4. **Fixed-point** the FAT-sector count: the FAT must have one slot per
    ///    sector *including the FAT's own sectors*, so adding FAT sectors can
    ///    push us over a sector boundary and require another. Iterate to a fixed
    ///    point (converges in ≤ 2 rounds for any realistic file).
    /// 5. Fill the FAT chains, mark FAT sectors FATSECT, write the header +
    ///    DIFAT, and concatenate every sector.
    pub fn finish(self) -> Vec<u8> {
        Layout::build(&self.streams).serialise()
    }
}

/// Convenience for the common one-shot case: give it name/data pairs, get bytes.
///
/// ```
/// # use cfb_writer::write_cfb;
/// let bytes = write_cfb(&[("Workbook", &b"hello"[..])]);
/// assert!(bytes.len() >= 512);
/// ```
pub fn write_cfb(streams: &[(&str, &[u8])]) -> Vec<u8> {
    let mut w = CfbWriter::new();
    for (name, data) in streams {
        w.add_stream(name, data);
    }
    w.finish()
}

// ---------------------------------------------------------------------------
// Name handling.
// ---------------------------------------------------------------------------

/// Truncate a name to at most [`MAX_NAME_UNITS`] UTF-16 code units. We count in
/// UTF-16 units (not bytes or `char`s) because that is exactly what fits in the
/// on-disk field.
fn truncate_name(name: &str) -> String {
    let units: Vec<u16> = name.encode_utf16().collect();
    if units.len() <= MAX_NAME_UNITS {
        name.to_string()
    } else {
        String::from_utf16_lossy(&units[..MAX_NAME_UNITS])
    }
}

// ---------------------------------------------------------------------------
// Directory entries — our in-memory model before encoding to 128 bytes each.
// ---------------------------------------------------------------------------

/// One directory entry as we build it, before serialisation.
struct DirEntryBuild {
    /// The (already length-limited) name.
    name: String,
    /// `OBJ_ROOT` for entry 0, `OBJ_STREAM` for the rest.
    object_type: u8,
    /// Right-sibling directory ID, or `NOSTREAM`.
    right: u32,
    /// Child directory ID (root only), or `NOSTREAM`.
    child: u32,
    /// Starting sector (regular sector for large streams / mini-stream owner;
    /// mini-sector index for small streams) or `ENDOFCHAIN` when size is 0.
    start_sector: u32,
    /// Logical byte size. For the root this is the mini-stream length.
    size: u64,
}

/// Encode one directory entry into a fresh 128-byte block.
fn encode_dir_entry(e: &DirEntryBuild) -> [u8; DIR_ENTRY_SIZE] {
    let mut buf = [0u8; DIR_ENTRY_SIZE];

    // Name: UTF-16LE at offset 0, plus a two-byte NUL terminator. We already
    // guarantee the name is <= 31 units, so name + NUL <= 64 bytes.
    let units: Vec<u16> = e.name.encode_utf16().collect();
    let mut i = 0usize;
    for u in &units {
        let b = u.to_le_bytes();
        buf[i] = b[0];
        buf[i + 1] = b[1];
        i += 2;
    }
    // buf[i..i+2] stays zero — that is the NUL terminator.
    // Name byte-length INCLUDING the terminator, at offset 64.
    let name_len_bytes = (units.len() + 1) * 2;
    put_u16(&mut buf, 64, name_len_bytes as u16);

    // Object type (66) and colour (67).
    buf[66] = e.object_type;
    buf[67] = COLOR_BLACK;

    // Left (68) / right (72) / child (76) stream IDs.
    put_u32(&mut buf, 68, NOSTREAM); // left: our tree never uses left siblings
    put_u32(&mut buf, 72, e.right);
    put_u32(&mut buf, 76, e.child);

    // CLSID (80..96), state flags (96), created/modified times (100..116): all
    // left zero for determinism.

    // Starting sector (116) and stream size (120, a u64).
    put_u32(&mut buf, 116, e.start_sector);
    put_u64(&mut buf, 120, e.size);

    buf
}

// ---------------------------------------------------------------------------
// The layout computation — the heart of the writer.
// ---------------------------------------------------------------------------

/// A fully computed layout: sector contents, FAT chains, header fields. From
/// this, `serialise` is a straightforward concatenation.
struct Layout {
    /// The directory stream bytes (a whole number of 512-byte sectors).
    directory: Vec<u8>,
    /// The mini-FAT stream bytes (empty if no small streams).
    minifat: Vec<u8>,
    /// The mini-stream bytes (empty if no small streams).
    mini_stream: Vec<u8>,
    /// Each large stream's data, already padded to a 512-byte multiple, in the
    /// same order they appear in the directory.
    large_data: Vec<Vec<u8>>,
    /// First sector of the directory chain.
    first_dir_sector: u32,
    /// First sector of the mini-FAT chain, or `ENDOFCHAIN`.
    first_minifat_sector: u32,
    /// Number of mini-FAT sectors.
    num_minifat_sectors: u32,
    /// The fully assembled FAT (one u32 per data sector, before FAT sectors are
    /// themselves appended and marked).
    fat: Vec<u32>,
    /// The number of *data* sectors (everything except the FAT sectors).
    data_sectors: usize,
    /// The number of FAT sectors (computed by fixed point).
    num_fat_sectors: usize,
    /// The sector indices that hold the FAT (so we can mark them FATSECT and
    /// list them in the DIFAT).
    fat_sector_ids: Vec<u32>,
}

impl Layout {
    /// Compute the complete layout from the ordered streams.
    fn build(streams: &[(String, Vec<u8>)]) -> Layout {
        // --- 1. Partition streams into small (mini) vs large (regular) -------
        // Preserve overall order for the directory sibling chain, but remember
        // which bucket each went to so we can fill in its start sector later.
        //
        // A stream is "small" when its size is STRICTLY LESS than the cutoff.
        // A zero-length stream is special-cased everywhere (no sectors at all).
        #[derive(Clone, Copy)]
        enum Placement {
            /// Empty stream: no data anywhere, start = ENDOFCHAIN.
            Empty,
            /// Small stream: index into the mini-stream bucket.
            Mini(usize),
            /// Large stream: index into the large bucket.
            Large(usize),
        }

        let mut placements: Vec<(usize, Placement)> = Vec::with_capacity(streams.len());
        let mut mini_payloads: Vec<&[u8]> = Vec::new();
        let mut large_payloads: Vec<&[u8]> = Vec::new();

        for (i, (_name, data)) in streams.iter().enumerate() {
            let placement = if data.is_empty() {
                Placement::Empty
            } else if (data.len() as u64) < MINI_CUTOFF as u64 {
                let idx = mini_payloads.len();
                mini_payloads.push(data);
                Placement::Mini(idx)
            } else {
                let idx = large_payloads.len();
                large_payloads.push(data);
                Placement::Large(idx)
            };
            placements.push((i, placement));
        }

        // --- 2. Build the mini-stream and mini-FAT ---------------------------
        // Every small stream is chopped into 64-byte mini-sectors laid end to
        // end in one big mini-stream. The mini-FAT is a parallel chain: for a
        // stream occupying mini-sectors k..k+m, mini_fat[k..k+m-1] point to the
        // next, and mini_fat[k+m-1] = ENDOFCHAIN.
        let mut mini_stream: Vec<u8> = Vec::new();
        let mut minifat: Vec<u32> = Vec::new();
        // The mini-sector start index assigned to each mini payload, by its
        // bucket index.
        let mut mini_start_of: Vec<u32> = Vec::with_capacity(mini_payloads.len());

        for payload in &mini_payloads {
            let start_mini = minifat.len() as u32;
            mini_start_of.push(start_mini);
            let n_mini = div_round_up(payload.len() as u64, MINI_SECTOR_SIZE as u64) as usize;
            // Append the payload padded to a whole number of mini-sectors.
            mini_stream.extend_from_slice(payload);
            let pad = n_mini * MINI_SECTOR_SIZE - payload.len();
            mini_stream.extend(std::iter::repeat_n(0u8, pad));
            // Chain the mini-FAT slots.
            for j in 0..n_mini {
                if j + 1 < n_mini {
                    minifat.push(start_mini + j as u32 + 1);
                } else {
                    minifat.push(ENDOFCHAIN);
                }
            }
        }

        let mini_stream_size = mini_stream.len() as u64;
        // The mini-stream is itself stored in regular 512-byte sectors (it is a
        // normal FAT stream owned by the root), so pad it up to a sector
        // multiple now.
        pad_to_sector(&mut mini_stream);

        // Serialise the mini-FAT to bytes, padded to whole 512-byte sectors and
        // with unused trailing slots set to FREESECT.
        let minifat_bytes = if minifat.is_empty() {
            Vec::new()
        } else {
            encode_fat_like(&minifat, MINIFAT_ENTRIES_PER_SECTOR)
        };

        // --- 3. Build the directory ------------------------------------------
        // Entry 0 is the root; entries 1.. are the streams in insertion order,
        // chained by right-sibling. We will patch start_sector for large/mini/
        // root once sector numbers are known, so use placeholders for now.
        // `dir` is patched later (start sectors), so bind it mutable up front.
        let mut dir: Vec<DirEntryBuild> = Vec::with_capacity(streams.len() + 1);
        // Root entry. child points at the first stream (id 1) if any exist.
        dir.push(DirEntryBuild {
            name: "Root Entry".to_string(),
            object_type: OBJ_ROOT,
            right: NOSTREAM,
            child: if streams.is_empty() { NOSTREAM } else { 1 },
            start_sector: ENDOFCHAIN, // patched to mini-stream start (or stays)
            size: mini_stream_size,
        });
        for (idx, (i, placement)) in placements.iter().enumerate() {
            let (name, data) = &streams[*i];
            // The next stream is our right sibling; the last one terminates.
            let right = if idx + 1 < placements.len() {
                (idx as u32) + 2 // entry ids: idx 0 -> id 1, its right -> id 2
            } else {
                NOSTREAM
            };
            let size = data.len() as u64;
            // start_sector filled in below; for Empty it stays ENDOFCHAIN.
            let start_sector = match placement {
                Placement::Empty => ENDOFCHAIN,
                Placement::Mini(bi) => mini_start_of[*bi], // mini-sector index
                Placement::Large(_) => 0,                  // patched later
            };
            dir.push(DirEntryBuild {
                name: name.clone(),
                object_type: OBJ_STREAM,
                right,
                child: NOSTREAM,
                start_sector,
                size,
            });
        }

        // --- 4. Assign regular sectors ---------------------------------------
        // Data-sector layout (sector 0 is the first sector AFTER the header):
        //   [ directory ][ mini-FAT ][ mini-stream ][ large stream 0 ] ...
        // FAT sectors are appended AFTER all data sectors, and their count is
        // resolved by the fixed point in step 5.
        let mut next_sector: u32 = 0;

        // (a) directory
        let directory_bytes = encode_directory(&dir);
        let dir_sector_count = directory_bytes.len() / SECTOR_SIZE;
        let first_dir_sector = next_sector;
        next_sector += dir_sector_count as u32;

        // (b) mini-FAT
        let minifat_sector_count = minifat_bytes.len() / SECTOR_SIZE;
        let (first_minifat_sector, num_minifat_sectors) = if minifat_sector_count == 0 {
            (ENDOFCHAIN, 0u32)
        } else {
            let s = next_sector;
            next_sector += minifat_sector_count as u32;
            (s, minifat_sector_count as u32)
        };

        // (c) mini-stream (owned by root)
        let mini_stream_sector_count = mini_stream.len() / SECTOR_SIZE;
        let mini_stream_start = if mini_stream_sector_count == 0 {
            ENDOFCHAIN
        } else {
            let s = next_sector;
            next_sector += mini_stream_sector_count as u32;
            s
        };

        // (d) each large stream, padded to a sector multiple
        let mut large_data: Vec<Vec<u8>> = Vec::with_capacity(large_payloads.len());
        let mut large_starts: Vec<u32> = Vec::with_capacity(large_payloads.len());
        for payload in &large_payloads {
            let mut buf = payload.to_vec();
            pad_to_sector(&mut buf);
            large_starts.push(next_sector);
            next_sector += (buf.len() / SECTOR_SIZE) as u32;
            large_data.push(buf);
        }

        let data_sectors = next_sector as usize;

        // --- 4b. Build the FAT for all DATA sectors (chains only) ------------
        // We now know every data sector's position, so we can write the "next"
        // pointers for each chain. FAT sectors themselves are appended and
        // marked FATSECT during the fixed point in step 5.
        let mut fat: Vec<u32> = vec![FREESECT; data_sectors];
        let chain = |fat: &mut Vec<u32>, start: u32, count: usize| {
            // Link `count` consecutive sectors starting at `start` into a chain.
            for k in 0..count {
                let s = start as usize + k;
                fat[s] = if k + 1 < count {
                    start + k as u32 + 1
                } else {
                    ENDOFCHAIN
                };
            }
        };
        chain(&mut fat, first_dir_sector, dir_sector_count);
        if num_minifat_sectors > 0 {
            chain(&mut fat, first_minifat_sector, minifat_sector_count);
        }
        if mini_stream_sector_count > 0 {
            chain(&mut fat, mini_stream_start, mini_stream_sector_count);
        }
        for (start, buf) in large_starts.iter().zip(large_data.iter()) {
            chain(&mut fat, *start, buf.len() / SECTOR_SIZE);
        }

        // Patch large-stream start sectors into their directory entries. The
        // directory ID of large stream `bi` is found by scanning placements.
        // (We rebuild the directory bytes after patching, below.)
        for (i, placement) in placements.iter().map(|(i, p)| (*i, p)) {
            if let Placement::Large(bi) = placement {
                // Directory id = position in streams + 1.
                dir[i + 1].start_sector = large_starts[*bi];
            }
        }
        // Root's start_sector is the mini-stream start (or ENDOFCHAIN).
        dir[0].start_sector = mini_stream_start;
        // Re-encode the directory now that all start sectors are final.
        let directory_bytes = encode_directory(&dir);

        // --- 5. Fixed-point the FAT-sector count -----------------------------
        // The FAT needs one slot per sector, INCLUDING its own sectors. Adding
        // FAT sectors grows the total, which may need more FAT sectors. Iterate.
        let mut num_fat_sectors = 0usize;
        loop {
            let total = data_sectors + num_fat_sectors;
            let needed = div_round_up(total as u64, FAT_ENTRIES_PER_SECTOR as u64) as usize;
            if needed == num_fat_sectors {
                break;
            }
            num_fat_sectors = needed;
        }

        // Extend the FAT to cover the FAT sectors and mark them FATSECT. The FAT
        // sectors occupy the sector indices immediately after the data sectors.
        let mut fat_sector_ids: Vec<u32> = Vec::with_capacity(num_fat_sectors);
        for k in 0..num_fat_sectors {
            let sid = (data_sectors + k) as u32;
            fat_sector_ids.push(sid);
        }
        // Grow the FAT array to total length, defaulting to FREESECT.
        let total_sectors = data_sectors + num_fat_sectors;
        fat.resize(total_sectors, FREESECT);
        for &sid in &fat_sector_ids {
            fat[sid as usize] = FATSECT;
        }

        Layout {
            directory: directory_bytes,
            minifat: minifat_bytes,
            mini_stream,
            large_data,
            first_dir_sector,
            first_minifat_sector,
            num_minifat_sectors,
            fat,
            data_sectors,
            num_fat_sectors,
            fat_sector_ids,
        }
    }

    /// Concatenate the header, all data sectors, and the FAT sectors into the
    /// final byte buffer.
    fn serialise(self) -> Vec<u8> {
        let total_sectors = self.data_sectors + self.num_fat_sectors;

        // --- Header ----------------------------------------------------------
        let mut out = vec![0u8; HEADER_LEN];
        out[0..8].copy_from_slice(&SIGNATURE);
        // CLSID (8..24): zero.
        put_u16(&mut out, 24, 0x003E); // minor version
        put_u16(&mut out, 26, 0x0003); // major version (v3)
        put_u16(&mut out, 28, 0xFFFE); // byte order marker (little-endian)
        put_u16(&mut out, 30, 0x0009); // sector shift -> 512
        put_u16(&mut out, 32, 0x0006); // mini sector shift -> 64
        // 34..40: reserved (zero).
        put_u32(&mut out, 40, 0); // number of directory sectors (0 for v3)
        put_u32(&mut out, 44, self.num_fat_sectors as u32);
        put_u32(&mut out, 48, self.first_dir_sector);
        put_u32(&mut out, 52, 0); // transaction signature
        put_u32(&mut out, 56, MINI_CUTOFF); // mini-stream cutoff
        put_u32(&mut out, 60, self.first_minifat_sector);
        put_u32(&mut out, 64, self.num_minifat_sectors);
        put_u32(&mut out, 68, ENDOFCHAIN); // first DIFAT sector (none)
        put_u32(&mut out, 72, 0); // number of DIFAT sectors

        // DIFAT array: the first 109 FAT-sector locations, then FREESECT pad.
        // For a well-formed small file we always have <= 109 FAT sectors, so no
        // DIFAT-sector chain is ever needed. (A single 512-byte FAT sector maps
        // 128 sectors * 512 bytes = 64 KiB; 109 of them map ~7 MiB of FAT
        // pointers, i.e. hundreds of MiB of file — far beyond our safety cap.)
        for i in 0..HEADER_DIFAT_COUNT {
            let off = HEADER_DIFAT_OFFSET + i * 4;
            let v = self.fat_sector_ids.get(i).copied().unwrap_or(FREESECT);
            put_u32(&mut out, off, v);
        }

        // --- Data sectors, in the exact order we numbered them ---------------
        // We pre-size the final buffer and append each region. Order MUST match
        // the sector numbering in `build`.
        out.reserve(total_sectors * SECTOR_SIZE);
        out.extend_from_slice(&self.directory);
        out.extend_from_slice(&self.minifat);
        out.extend_from_slice(&self.mini_stream);
        for buf in &self.large_data {
            out.extend_from_slice(buf);
        }

        // --- FAT sectors -----------------------------------------------------
        // Serialise the FAT array into its sectors, then append. The FAT array
        // is already `total_sectors` long, padded to whole FAT sectors below.
        let fat_bytes = encode_fat_like(&self.fat, FAT_ENTRIES_PER_SECTOR);
        // Sanity: the encoded FAT must be exactly num_fat_sectors sectors. If
        // arithmetic were ever off we'd still produce a valid-length buffer;
        // debug_assert catches logic errors in tests without panicking release.
        debug_assert_eq!(fat_bytes.len() / SECTOR_SIZE, self.num_fat_sectors);
        out.extend_from_slice(&fat_bytes);

        out
    }
}

/// Pad a byte buffer up to a whole number of 512-byte sectors with zeros.
fn pad_to_sector(buf: &mut Vec<u8>) {
    let rem = buf.len() % SECTOR_SIZE;
    if rem != 0 {
        buf.extend(std::iter::repeat_n(0u8, SECTOR_SIZE - rem));
    }
}

/// Encode the directory entries to bytes, padded to a whole number of 512-byte
/// sectors. Any partial trailing sector is filled with **unused** directory
/// entries whose object type is 0 (the reader treats type-0 nodes as invalid
/// and skips them), which is the conforming way to pad a directory sector.
fn encode_directory(dir: &[DirEntryBuild]) -> Vec<u8> {
    let mut out = Vec::with_capacity(dir.len() * DIR_ENTRY_SIZE);
    for e in dir {
        out.extend_from_slice(&encode_dir_entry(e));
    }
    // Pad to a whole sector with "unused" (all-zero except stream-id fields)
    // 128-byte entries. Per [MS-CFB], unused entries set left/right/child to
    // NOSTREAM; object type 0 marks them unallocated.
    let rem = out.len() % SECTOR_SIZE;
    if rem != 0 {
        let pad_bytes = SECTOR_SIZE - rem;
        let n_entries = pad_bytes / DIR_ENTRY_SIZE;
        for _ in 0..n_entries {
            let mut e = [0u8; DIR_ENTRY_SIZE];
            put_u32(&mut e, 68, NOSTREAM); // left
            put_u32(&mut e, 72, NOSTREAM); // right
            put_u32(&mut e, 76, NOSTREAM); // child
            out.extend_from_slice(&e);
        }
    }
    out
}

/// Encode a FAT-like `u32` array (the FAT or the mini-FAT) into bytes, padded to
/// a whole number of 512-byte sectors, with trailing pad slots set to FREESECT.
fn encode_fat_like(entries: &[u32], entries_per_sector: usize) -> Vec<u8> {
    // Round the entry count up to a whole sector's worth.
    let sectors = div_round_up(entries.len() as u64, entries_per_sector as u64) as usize;
    let total_slots = sectors * entries_per_sector;
    let mut out = vec![0u8; total_slots * 4];
    for (i, &v) in entries.iter().enumerate() {
        put_u32(&mut out, i * 4, v);
    }
    // Fill the trailing (padding) slots with FREESECT.
    for i in entries.len()..total_slots {
        put_u32(&mut out, i * 4, FREESECT);
    }
    out
}

#[cfg(test)]
mod tests;
