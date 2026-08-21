//! # cfb — Compound File Binary Format reader (CFB01)
//!
//! A from-scratch, zero-dependency reader for the **OLE2 / Compound File Binary
//! Format** ([MS-CFB]) — the container that lives inside legacy `.xls`, `.doc`,
//! and `.ppt` files. See `code/specs/CFB01-compound-file.md` for the full
//! literate walkthrough of the format.
//!
//! ## The one-paragraph mental model
//!
//! A CFB file is a **FAT filesystem crammed into a single file**. The file is
//! chopped into fixed-size **sectors** (usually 512 bytes). A **File Allocation
//! Table (FAT)** is an array of "next sector" pointers: to read a multi-sector
//! stream you follow `FAT[n]` like a linked list until you hit `ENDOFCHAIN`. A
//! **directory** (itself a FAT-stored stream) lists the named objects — CFB
//! calls a file a *stream* and a folder a *storage*. Tiny streams (< the
//! mini-stream cutoff, usually 4096 bytes) live packed inside a **mini-stream**
//! chained by a parallel **mini-FAT**, to avoid wasting a whole sector each.
//!
//! ## Reading untrusted bytes
//!
//! CFB files arrive as email attachments, so this parser assumes hostility:
//! every sector-chain walk is cycle-guarded, every sector offset is
//! bounds-checked, and total assembled output is capped. It is
//! `#![forbid(unsafe_code)]` and never `unwrap`/`panic!`s on input.
//!
//! ```
//! # use cfb::{CompoundFile, EntryKind};
//! # fn demo(bytes: &[u8]) -> Result<(), cfb::CfbError> {
//! let cf = CompoundFile::open(bytes)?;
//! for e in cf.entries() {
//!     println!("{} — {} bytes ({:?})", e.name, e.size, e.kind);
//! }
//! if let Some(data) = cf.read_stream("Workbook") {
//!     assert_eq!(&data[0..2], &[0x09, 0x08]); // BIFF8 BOF
//! }
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![deny(warnings)]

use std::collections::HashSet;
use std::fmt;

// ---------------------------------------------------------------------------
// Constants — the sentinel FAT values and header field offsets.
// ---------------------------------------------------------------------------

/// The 8-byte magic that opens every CFB file. Looks like "DOCFILE" if you
/// squint at the hex: `D0 CF 11 E0 A1 B1 1A E1`.
const SIGNATURE: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// Free / unused sector.
const FREESECT: u32 = 0xFFFF_FFFF;
/// End of a sector chain.
const ENDOFCHAIN: u32 = 0xFFFF_FFFE;
/// Sector holds part of the FAT.
const FATSECT: u32 = 0xFFFF_FFFD;
/// Sector holds part of the DIFAT.
const DIFSECT: u32 = 0xFFFF_FFFC;
/// Directory-entry "no such sibling/child" marker.
const NOSTREAM: u32 = 0xFFFF_FFFF;

/// The fixed header length. Sectors are numbered *after* these first 512 bytes.
const HEADER_LEN: usize = 512;
/// Each directory entry is exactly 128 bytes.
const DIR_ENTRY_SIZE: usize = 128;
/// The header inlines the first 109 FAT-sector locations.
const HEADER_DIFAT_COUNT: usize = 109;
/// Byte offset within the header where the inlined DIFAT array begins.
const HEADER_DIFAT_OFFSET: usize = 76;

/// Hard ceiling on total assembled output, so a lying directory size cannot
/// make us allocate unbounded memory. 256 MiB comfortably exceeds any real
/// legacy Office document while stopping abuse.
const MAX_OUTPUT: u64 = 256 * 1024 * 1024;

// ---------------------------------------------------------------------------
// Errors.
// ---------------------------------------------------------------------------

/// Everything that can go wrong reading a (possibly hostile) CFB file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CfbError {
    /// The first 8 bytes were not the CFB signature.
    BadSignature,
    /// The input ended before a structure we needed to read.
    Truncated,
    /// The sector shift was not one we support (only 512 and 4096 byte sectors).
    UnsupportedSectorSize(u16),
    /// A sector chain referenced a sector number past the end of the file.
    BadSectorChain,
    /// A sector chain looped instead of terminating — refused to hang.
    CycleDetected,
    /// A structure's declared size would exceed the safety output cap.
    OutputTooLarge,
    /// The directory was malformed (bad entry count, bad root, etc.).
    BadDirectory,
    /// Asked to read a directory entry that is not a stream.
    NotAStream,
}

impl fmt::Display for CfbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CfbError::BadSignature => write!(f, "not a Compound File (bad signature)"),
            CfbError::Truncated => write!(f, "input truncated"),
            CfbError::UnsupportedSectorSize(s) => {
                write!(f, "unsupported sector shift 0x{s:04x}")
            }
            CfbError::BadSectorChain => write!(f, "sector chain out of bounds"),
            CfbError::CycleDetected => write!(f, "cycle detected in sector chain"),
            CfbError::OutputTooLarge => write!(f, "assembled output exceeds safety cap"),
            CfbError::BadDirectory => write!(f, "malformed directory"),
            CfbError::NotAStream => write!(f, "directory entry is not a stream"),
        }
    }
}

impl std::error::Error for CfbError {}

// ---------------------------------------------------------------------------
// Little-endian readers. These never panic: they return `None` on truncation,
// which callers turn into `CfbError::Truncated`.
// ---------------------------------------------------------------------------

#[inline]
fn read_u16(buf: &[u8], off: usize) -> Option<u16> {
    let b = buf.get(off..off + 2)?;
    Some(u16::from_le_bytes([b[0], b[1]]))
}

#[inline]
fn read_u32(buf: &[u8], off: usize) -> Option<u32> {
    let b = buf.get(off..off + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

#[inline]
fn read_u64(buf: &[u8], off: usize) -> Option<u64> {
    let b = buf.get(off..off + 8)?;
    let mut a = [0u8; 8];
    a.copy_from_slice(b);
    Some(u64::from_le_bytes(a))
}

// ---------------------------------------------------------------------------
// Public entry description.
// ---------------------------------------------------------------------------

/// What kind of object a directory entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// A named byte-stream (a "file").
    Stream,
    /// A storage (a "folder") that can contain other entries.
    Storage,
    /// The single root storage (entry 0). Its stream fields describe the
    /// mini-stream, not user data.
    RootStorage,
}

/// One enumerated object from the directory: its name, size, kind, and the raw
/// directory ID (index) so callers can read it precisely.
#[derive(Debug, Clone)]
pub struct Entry {
    /// The decoded UTF-16LE name (raw — a leading control char like `\u{5}` is
    /// preserved, per the format).
    pub name: String,
    /// Byte size of the stream (0 for storages).
    pub size: u64,
    /// Stream vs storage vs root.
    pub kind: EntryKind,
    /// The directory index of this entry, for [`CompoundFile::read_stream_by_id`].
    pub id: u32,
}

// ---------------------------------------------------------------------------
// A raw 128-byte directory entry, parsed but not yet interpreted.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct DirEntry {
    name: String,
    object_type: u8,
    left: u32,
    right: u32,
    child: u32,
    start_sector: u32,
    size: u64,
}

// ---------------------------------------------------------------------------
// The parsed compound file.
// ---------------------------------------------------------------------------

/// A parsed Compound File. Holds an owned copy of the input plus the assembled
/// FAT, mini-FAT, directory, and mini-stream, ready for by-name stream reads.
#[derive(Debug)]
pub struct CompoundFile {
    data: Vec<u8>,
    sector_size: usize,
    mini_sector_size: usize,
    mini_cutoff: u32,
    /// The fully assembled File Allocation Table: `fat[n]` = sector after `n`.
    fat: Vec<u32>,
    /// The fully assembled mini-FAT: `mini_fat[m]` = mini-sector after `m`.
    mini_fat: Vec<u32>,
    /// All directory entries, indexed by directory ID.
    dir: Vec<DirEntry>,
    /// The assembled mini-stream bytes (all tiny streams packed together).
    mini_stream: Vec<u8>,
    /// Flattened, user-facing entry list (streams + storages, excluding root).
    entries: Vec<Entry>,
}

impl CompoundFile {
    /// Parse a CFB file from raw bytes. Performs all structural validation up
    /// front (header, FAT, directory, mini-stream) so later reads are cheap and
    /// infallible-by-name.
    pub fn open(bytes: &[u8]) -> Result<CompoundFile, CfbError> {
        // --- Header ------------------------------------------------------
        if bytes.len() < HEADER_LEN {
            return Err(CfbError::Truncated);
        }
        if bytes[0..8] != SIGNATURE {
            return Err(CfbError::BadSignature);
        }

        let sector_shift = read_u16(bytes, 30).ok_or(CfbError::Truncated)?;
        let sector_size = match sector_shift {
            0x0009 => 512usize,
            0x000C => 4096usize,
            other => return Err(CfbError::UnsupportedSectorSize(other)),
        };
        let mini_sector_shift = read_u16(bytes, 32).ok_or(CfbError::Truncated)?;
        // Only 64-byte mini-sectors (shift 6) are defined by the spec.
        if mini_sector_shift != 0x0006 {
            return Err(CfbError::UnsupportedSectorSize(mini_sector_shift));
        }
        let mini_sector_size = 1usize << mini_sector_shift;

        let num_fat_sectors = read_u32(bytes, 44).ok_or(CfbError::Truncated)?;
        let first_dir_sector = read_u32(bytes, 48).ok_or(CfbError::Truncated)?;
        let mini_cutoff = read_u32(bytes, 56).ok_or(CfbError::Truncated)?;
        let first_minifat_sector = read_u32(bytes, 60).ok_or(CfbError::Truncated)?;
        let num_minifat_sectors = read_u32(bytes, 64).ok_or(CfbError::Truncated)?;
        let first_difat_sector = read_u32(bytes, 68).ok_or(CfbError::Truncated)?;
        let num_difat_sectors = read_u32(bytes, 72).ok_or(CfbError::Truncated)?;

        // How many sectors does the file actually contain? Used as the hard
        // upper bound for every chain walk (cycle guard) and bounds check.
        let total_sectors = bytes.len().saturating_sub(HEADER_LEN) / sector_size;

        let mut cf = CompoundFile {
            data: bytes.to_vec(),
            sector_size,
            mini_sector_size,
            mini_cutoff,
            fat: Vec::new(),
            mini_fat: Vec::new(),
            dir: Vec::new(),
            mini_stream: Vec::new(),
            entries: Vec::new(),
        };

        // --- Collect the DIFAT: the list of FAT-sector locations ---------
        let fat_sector_ids = cf.collect_difat(
            first_difat_sector,
            num_difat_sectors,
            num_fat_sectors,
            total_sectors,
        )?;

        // --- Assemble the FAT itself -------------------------------------
        cf.fat = cf.assemble_fat(&fat_sector_ids, total_sectors)?;

        // --- Assemble the mini-FAT (a regular-FAT stream) ----------------
        cf.mini_fat = if first_minifat_sector == ENDOFCHAIN || num_minifat_sectors == 0 {
            Vec::new()
        } else {
            let raw = cf.read_fat_chain(first_minifat_sector, None)?;
            let mut mf = Vec::with_capacity(raw.len() / 4);
            for chunk in raw.as_chunks::<4>().0 {
                mf.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
            }
            mf
        };

        // --- Read the directory (a regular-FAT stream) -------------------
        cf.dir = cf.read_directory(first_dir_sector)?;
        if cf.dir.is_empty() {
            return Err(CfbError::BadDirectory);
        }

        // --- The root entry (index 0) describes the mini-stream ----------
        let root = &cf.dir[0];
        if root.object_type != 5 {
            return Err(CfbError::BadDirectory);
        }
        let mini_stream_start = root.start_sector;
        let mini_stream_size = root.size;
        cf.mini_stream = if mini_stream_size == 0 {
            Vec::new()
        } else {
            let mut ms = cf.read_fat_chain(mini_stream_start, None)?;
            // Trim to declared size (already bounds-checked against file len).
            let want = mini_stream_size as usize;
            if want > ms.len() {
                return Err(CfbError::BadDirectory);
            }
            ms.truncate(want);
            ms
        };

        // --- Flatten the red-black directory tree into a linear list -----
        cf.entries = cf.enumerate_entries()?;

        Ok(cf)
    }

    /// The enumerated non-root objects (streams and storages) with name, size,
    /// and kind.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Convenience: just the names of the enumerated stream entries.
    pub fn stream_names(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.kind == EntryKind::Stream)
            .map(|e| e.name.clone())
            .collect()
    }

    /// Read a top-level stream's bytes by name (case-insensitive, per spec).
    /// Returns `None` if no such stream exists.
    pub fn read_stream(&self, name: &str) -> Option<Vec<u8>> {
        let target = name.to_lowercase();
        let id = self
            .entries
            .iter()
            .find(|e| e.kind == EntryKind::Stream && e.name.to_lowercase() == target)
            .map(|e| e.id)?;
        self.read_stream_by_id(id).ok()
    }

    /// Read a stream precisely by its directory ID.
    pub fn read_stream_by_id(&self, id: u32) -> Result<Vec<u8>, CfbError> {
        let entry = self
            .dir
            .get(id as usize)
            .ok_or(CfbError::BadDirectory)?
            .clone();
        if entry.object_type != 2 {
            return Err(CfbError::NotAStream);
        }
        let size = entry.size;
        if size > MAX_OUTPUT {
            return Err(CfbError::OutputTooLarge);
        }
        if size == 0 {
            return Ok(Vec::new());
        }

        // The cutoff decides which store holds this stream.
        let mut bytes = if size < self.mini_cutoff as u64 {
            self.read_mini_chain(entry.start_sector, size)?
        } else {
            self.read_fat_chain(entry.start_sector, Some(size))?
        };
        let want = size as usize;
        if want > bytes.len() {
            return Err(CfbError::BadSectorChain);
        }
        bytes.truncate(want);
        Ok(bytes)
    }

    // -------------------------------------------------------------------
    // Internal helpers.
    // -------------------------------------------------------------------

    /// Absolute byte range of sector `n`, bounds-checked against the file.
    fn sector_range(&self, n: u32) -> Result<(usize, usize), CfbError> {
        // Reject sentinels and impossibly large indices before arithmetic.
        if n >= FREESECT - 4 {
            return Err(CfbError::BadSectorChain);
        }
        let start = HEADER_LEN
            .checked_add((n as usize).checked_mul(self.sector_size).ok_or(CfbError::BadSectorChain)?)
            .ok_or(CfbError::BadSectorChain)?;
        let end = start.checked_add(self.sector_size).ok_or(CfbError::BadSectorChain)?;
        if end > self.data.len() {
            return Err(CfbError::BadSectorChain);
        }
        Ok((start, end))
    }

    /// Gather every FAT-sector location: the 109 inlined in the header, then any
    /// in the DIFAT-sector chain (for files with more than 109 FAT sectors).
    fn collect_difat(
        &self,
        first_difat_sector: u32,
        num_difat_sectors: u32,
        num_fat_sectors: u32,
        total_sectors: usize,
    ) -> Result<Vec<u32>, CfbError> {
        let mut fat_sectors: Vec<u32> = Vec::new();

        // (1) The 109 header-inlined entries.
        for i in 0..HEADER_DIFAT_COUNT {
            let off = HEADER_DIFAT_OFFSET + i * 4;
            let v = read_u32(&self.data, off).ok_or(CfbError::Truncated)?;
            if v == FREESECT {
                continue;
            }
            fat_sectors.push(v);
        }

        // (2) Walk the DIFAT-sector chain, if any. Each DIFAT sector holds
        //     (sector_size/4 - 1) FAT-sector pointers plus a "next DIFAT
        //     sector" pointer in its final u32.
        if first_difat_sector != ENDOFCHAIN && num_difat_sectors > 0 {
            let per_sector = self.sector_size / 4;
            let mut current = first_difat_sector;
            let mut visited: HashSet<u32> = HashSet::new();
            // Cap iterations at the declared count AND total sectors.
            let cap = (num_difat_sectors as usize).min(total_sectors.max(1)) + 1;
            let mut steps = 0usize;
            while current != ENDOFCHAIN && current != FREESECT {
                if !visited.insert(current) {
                    return Err(CfbError::CycleDetected);
                }
                if steps >= cap || steps > total_sectors {
                    return Err(CfbError::CycleDetected);
                }
                steps += 1;
                let (s, e) = self.sector_range(current)?;
                let sec = &self.data[s..e];
                for i in 0..(per_sector - 1) {
                    let v = read_u32(sec, i * 4).ok_or(CfbError::Truncated)?;
                    if v != FREESECT {
                        fat_sectors.push(v);
                    }
                }
                current = read_u32(sec, (per_sector - 1) * 4).ok_or(CfbError::Truncated)?;
            }
        }

        // Trim to the declared FAT-sector count if the header padded with extras.
        if (num_fat_sectors as usize) < fat_sectors.len() {
            fat_sectors.truncate(num_fat_sectors as usize);
        }
        Ok(fat_sectors)
    }

    /// Read each FAT sector and concatenate into the flat FAT array.
    fn assemble_fat(
        &self,
        fat_sector_ids: &[u32],
        total_sectors: usize,
    ) -> Result<Vec<u32>, CfbError> {
        let per_sector = self.sector_size / 4;
        // Guard against a bogus DIFAT claiming more FAT sectors than the file
        // could ever hold.
        if fat_sector_ids.len() > total_sectors + 1 {
            return Err(CfbError::BadSectorChain);
        }
        let mut fat = Vec::with_capacity(fat_sector_ids.len() * per_sector);
        for &sid in fat_sector_ids {
            let (s, e) = self.sector_range(sid)?;
            let sec = &self.data[s..e];
            for i in 0..per_sector {
                fat.push(read_u32(sec, i * 4).ok_or(CfbError::Truncated)?);
            }
        }
        Ok(fat)
    }

    /// Walk a regular-FAT sector chain from `start`, concatenating sector bytes.
    /// If `size_hint` is given we stop once we have enough bytes (an
    /// optimization *and* an extra safety bound). Cycle- and bounds-guarded.
    fn read_fat_chain(&self, start: u32, size_hint: Option<u64>) -> Result<Vec<u8>, CfbError> {
        let mut out: Vec<u8> = Vec::new();
        let mut current = start;
        let mut visited: HashSet<u32> = HashSet::new();
        // Hard iteration cap = total FAT slots. A valid chain can never revisit
        // a sector, so it can never exceed this.
        let cap = self.fat.len().max(1) + 1;
        let mut steps = 0usize;

        while current != ENDOFCHAIN {
            if current == FREESECT
                || current == FATSECT
                || current == DIFSECT
            {
                return Err(CfbError::BadSectorChain);
            }
            if steps >= cap {
                return Err(CfbError::CycleDetected);
            }
            if !visited.insert(current) {
                return Err(CfbError::CycleDetected);
            }
            steps += 1;

            let (s, e) = self.sector_range(current)?;
            out.extend_from_slice(&self.data[s..e]);
            if out.len() as u64 > MAX_OUTPUT {
                return Err(CfbError::OutputTooLarge);
            }
            if let Some(want) = size_hint {
                if out.len() as u64 >= want {
                    break;
                }
            }

            // Follow the chain. `FAT[current]` must exist.
            current = *self
                .fat
                .get(current as usize)
                .ok_or(CfbError::BadSectorChain)?;
        }
        Ok(out)
    }

    /// Walk a mini-FAT chain from mini-sector `start`, slicing 64-byte pieces
    /// out of the already-assembled mini-stream. Cycle- and bounds-guarded.
    fn read_mini_chain(&self, start: u32, size: u64) -> Result<Vec<u8>, CfbError> {
        if size > MAX_OUTPUT {
            return Err(CfbError::OutputTooLarge);
        }
        let mut out: Vec<u8> = Vec::new();
        let mut current = start;
        let mut visited: HashSet<u32> = HashSet::new();
        let cap = self.mini_fat.len().max(1) + 1;
        let mut steps = 0usize;

        while current != ENDOFCHAIN {
            if current == FREESECT || current == FATSECT || current == DIFSECT {
                return Err(CfbError::BadSectorChain);
            }
            if steps >= cap {
                return Err(CfbError::CycleDetected);
            }
            if !visited.insert(current) {
                return Err(CfbError::CycleDetected);
            }
            steps += 1;

            let off = (current as usize)
                .checked_mul(self.mini_sector_size)
                .ok_or(CfbError::BadSectorChain)?;
            let end = off
                .checked_add(self.mini_sector_size)
                .ok_or(CfbError::BadSectorChain)?;
            if end > self.mini_stream.len() {
                return Err(CfbError::BadSectorChain);
            }
            out.extend_from_slice(&self.mini_stream[off..end]);
            if out.len() as u64 > MAX_OUTPUT {
                return Err(CfbError::OutputTooLarge);
            }
            if out.len() as u64 >= size {
                break;
            }

            current = *self
                .mini_fat
                .get(current as usize)
                .ok_or(CfbError::BadSectorChain)?;
        }
        Ok(out)
    }

    /// Read the directory stream and parse it into 128-byte entries.
    fn read_directory(&self, first_dir_sector: u32) -> Result<Vec<DirEntry>, CfbError> {
        let raw = self.read_fat_chain(first_dir_sector, None)?;
        if raw.is_empty() || raw.len() % DIR_ENTRY_SIZE != 0 {
            // A directory must be a whole number of entries.
            if raw.is_empty() {
                return Err(CfbError::BadDirectory);
            }
        }
        let count = raw.len() / DIR_ENTRY_SIZE;
        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let base = i * DIR_ENTRY_SIZE;
            let e = &raw[base..base + DIR_ENTRY_SIZE];
            entries.push(parse_dir_entry(e)?);
        }
        Ok(entries)
    }

    /// Recursively walk the red-black directory tree (root's child, then left
    /// and right subtrees) to flatten every stream and storage into a list.
    /// Cycle-guarded via a visited-set on directory IDs.
    fn enumerate_entries(&self) -> Result<Vec<Entry>, CfbError> {
        let mut out = Vec::new();
        let mut visited: HashSet<u32> = HashSet::new();

        // The root (entry 0) is reported separately; its *child* is the entry
        // into the top-level tree.
        let root = &self.dir[0];
        out.push(Entry {
            name: root.name.clone(),
            size: root.size,
            kind: EntryKind::RootStorage,
            id: 0,
        });
        if root.child != NOSTREAM {
            self.walk_tree(root.child, &mut visited, &mut out)?;
        }
        Ok(out)
    }

    /// Flatten one storage's child tree into `out`.
    ///
    /// This uses an **explicit work-list** rather than native recursion. A
    /// hostile file could craft a degenerate directory tree (e.g. every node's
    /// left-sibling points to the next, forming a chain as deep as the number
    /// of entries — tens of thousands for a large file). Native recursion on
    /// such a chain would overflow the stack. An explicit `Vec` work-list moves
    /// that growth to the heap, which the OS bounds far more generously.
    ///
    /// Termination is doubly guaranteed: the `visited` set means each directory
    /// ID is pushed at most once (a cycle → `CycleDetected`), so the loop runs
    /// at most `dir.len()` times.
    fn walk_tree(
        &self,
        start: u32,
        visited: &mut HashSet<u32>,
        out: &mut Vec<Entry>,
    ) -> Result<(), CfbError> {
        let mut stack: Vec<u32> = Vec::new();
        stack.push(start);

        while let Some(id) = stack.pop() {
            if id == NOSTREAM {
                continue;
            }
            if !visited.insert(id) {
                // A sibling/child pointer looped back — hostile file, stop safely.
                return Err(CfbError::CycleDetected);
            }
            let entry = self.dir.get(id as usize).ok_or(CfbError::BadDirectory)?;
            let (left, right, child) = (entry.left, entry.right, entry.child);
            let kind = match entry.object_type {
                1 => Some(EntryKind::Storage),
                2 => Some(EntryKind::Stream),
                5 => Some(EntryKind::RootStorage),
                // Unused/invalid node: don't emit it, but still follow siblings.
                _ => None,
            };
            if let Some(kind) = kind {
                out.push(Entry {
                    name: entry.name.clone(),
                    size: entry.size,
                    kind,
                    id,
                });
                // A storage owns a child sub-tree.
                if kind == EntryKind::Storage {
                    stack.push(child);
                }
            }
            // Always follow left/right siblings.
            stack.push(left);
            stack.push(right);
        }
        Ok(())
    }
}

/// Parse a single 128-byte directory entry.
fn parse_dir_entry(e: &[u8]) -> Result<DirEntry, CfbError> {
    if e.len() < DIR_ENTRY_SIZE {
        return Err(CfbError::Truncated);
    }
    let name_len = read_u16(e, 64).ok_or(CfbError::Truncated)? as usize;
    let object_type = e[66];
    let left = read_u32(e, 68).ok_or(CfbError::Truncated)?;
    let right = read_u32(e, 72).ok_or(CfbError::Truncated)?;
    let child = read_u32(e, 76).ok_or(CfbError::Truncated)?;
    let start_sector = read_u32(e, 116).ok_or(CfbError::Truncated)?;
    let size = read_u64(e, 120).ok_or(CfbError::Truncated)?;

    // Decode the UTF-16LE name. `name_len` counts bytes *including* the null
    // terminator; clamp to the 64-byte field and drop the trailing NUL.
    let name = decode_utf16_name(&e[0..64], name_len);

    Ok(DirEntry {
        name,
        object_type,
        left,
        right,
        child,
        start_sector,
        size,
    })
}

/// Decode a UTF-16LE directory name. `name_len` is the byte length including the
/// null terminator; a value of 0 (or an out-of-range one) yields an empty name.
/// A leading control char (e.g. `\u{5}` on `\u{5}DocumentSummaryInformation`)
/// is preserved raw, as the format intends.
fn decode_utf16_name(field: &[u8], name_len: usize) -> String {
    // Clamp to the physical field, and strip the 2-byte NUL terminator.
    let usable = name_len.min(field.len());
    let chars = usable.saturating_sub(2); // bytes of actual name (excl. NUL)
    let mut units = Vec::with_capacity(chars / 2);
    let mut i = 0;
    while i + 1 < field.len() && i < chars {
        units.push(u16::from_le_bytes([field[i], field[i + 1]]));
        i += 2;
    }
    // Lossy decode so a corrupt name never fails the whole parse.
    String::from_utf16_lossy(&units)
}

#[cfg(test)]
mod fixture;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::MINIMAL_XLS;

    // ---- End-to-end: the real .xls fixture --------------------------------

    #[test]
    fn opens_real_xls_fixture() {
        let cf = CompoundFile::open(MINIMAL_XLS).expect("fixture should parse");
        // 512-byte sectors per the header.
        assert_eq!(cf.sector_size, 512);
    }

    #[test]
    fn fixture_lists_workbook_stream() {
        let cf = CompoundFile::open(MINIMAL_XLS).unwrap();
        let names = cf.stream_names();
        assert!(
            names.iter().any(|n| n == "Workbook"),
            "expected a Workbook stream, got {names:?}"
        );
    }

    #[test]
    fn fixture_workbook_is_4096_bytes_and_biff8_bof() {
        let cf = CompoundFile::open(MINIMAL_XLS).unwrap();
        let data = cf.read_stream("Workbook").expect("Workbook stream present");
        assert_eq!(data.len(), 4096, "Workbook should be exactly 4096 bytes");
        // BIFF8 BOF record: first LE u16 == record type 0x0809.
        assert_eq!(data[0], 0x09);
        assert_eq!(data[1], 0x08);
        assert_eq!(u16::from_le_bytes([data[0], data[1]]), 0x0809);
    }

    #[test]
    fn read_stream_is_case_insensitive() {
        let cf = CompoundFile::open(MINIMAL_XLS).unwrap();
        assert!(cf.read_stream("WORKBOOK").is_some());
        assert!(cf.read_stream("workbook").is_some());
        assert!(cf.read_stream("does-not-exist").is_none());
    }

    #[test]
    fn entries_include_root_storage() {
        let cf = CompoundFile::open(MINIMAL_XLS).unwrap();
        assert!(cf.entries().iter().any(|e| e.kind == EntryKind::RootStorage));
    }

    // ---- Error paths ------------------------------------------------------

    #[test]
    fn bad_signature_is_rejected() {
        let mut bytes = MINIMAL_XLS.to_vec();
        bytes[0] = 0x00; // corrupt the magic
        assert_eq!(CompoundFile::open(&bytes).err(), Some(CfbError::BadSignature));
        // A completely unrelated blob:
        assert_eq!(
            CompoundFile::open(&[0u8; 600]).err(),
            Some(CfbError::BadSignature)
        );
    }

    #[test]
    fn empty_input_is_truncated_not_panic() {
        assert_eq!(CompoundFile::open(&[]).err(), Some(CfbError::Truncated));
    }

    #[test]
    fn short_input_is_truncated() {
        // Valid signature but far too short for a full header.
        let mut bytes = SIGNATURE.to_vec();
        bytes.extend_from_slice(&[0u8; 10]);
        assert_eq!(CompoundFile::open(&bytes).err(), Some(CfbError::Truncated));
    }

    #[test]
    fn unsupported_sector_shift_is_rejected() {
        let mut bytes = MINIMAL_XLS.to_vec();
        // Set sector shift (offset 30) to an invalid value 0x000A.
        bytes[30] = 0x0A;
        bytes[31] = 0x00;
        assert_eq!(
            CompoundFile::open(&bytes).err(),
            Some(CfbError::UnsupportedSectorSize(0x000A))
        );
    }

    #[test]
    fn truncated_after_header_gives_error_not_panic() {
        // Keep the whole header (so signature + sizes parse) but drop the body,
        // so the FAT/directory sectors point past the end of the file.
        let bytes = &MINIMAL_XLS[..HEADER_LEN];
        let res = CompoundFile::open(bytes);
        assert!(res.is_err(), "expected clean error, got {res:?}");
    }

    // ---- Cycle guards (the security-critical bit) -------------------------

    /// Build a minimal-but-valid CFB in memory so we can then poison its FAT
    /// with a self-loop and prove the reader terminates with an error rather
    /// than hanging. Uses 512-byte sectors.
    fn craft_cfb_with_fat_cycle() -> Vec<u8> {
        // Layout: header (512) + sector0 (FAT) + sector1 (directory).
        let sector = 512usize;
        let mut buf = vec![0u8; HEADER_LEN + 2 * sector];
        // Header
        buf[0..8].copy_from_slice(&SIGNATURE);
        buf[30] = 0x09; // sector shift -> 512
        buf[32] = 0x06; // mini sector shift -> 64
        // num FAT sectors = 1
        buf[44..48].copy_from_slice(&1u32.to_le_bytes());
        // first dir sector = 1
        buf[48..52].copy_from_slice(&1u32.to_le_bytes());
        // mini cutoff 4096
        buf[56..60].copy_from_slice(&4096u32.to_le_bytes());
        // first mini-fat = ENDOFCHAIN, count 0
        buf[60..64].copy_from_slice(&ENDOFCHAIN.to_le_bytes());
        buf[64..68].copy_from_slice(&0u32.to_le_bytes());
        // first DIFAT = ENDOFCHAIN, count 0
        buf[68..72].copy_from_slice(&ENDOFCHAIN.to_le_bytes());
        buf[72..76].copy_from_slice(&0u32.to_le_bytes());
        // DIFAT[0] = 0 (FAT lives in sector 0)
        buf[76..80].copy_from_slice(&0u32.to_le_bytes());
        for i in 1..HEADER_DIFAT_COUNT {
            let off = HEADER_DIFAT_OFFSET + i * 4;
            buf[off..off + 4].copy_from_slice(&FREESECT.to_le_bytes());
        }
        // FAT sector (sector 0 -> file offset 512): fill with FREESECT.
        let fat_off = HEADER_LEN;
        for i in 0..(sector / 4) {
            let o = fat_off + i * 4;
            buf[o..o + 4].copy_from_slice(&FREESECT.to_le_bytes());
        }
        // FAT[0] = FATSECT (sector 0 is the FAT itself)
        buf[fat_off..fat_off + 4].copy_from_slice(&FATSECT.to_le_bytes());
        // FAT[1] = 1  <-- the POISON: directory sector 1 points to itself.
        buf[fat_off + 4..fat_off + 8].copy_from_slice(&1u32.to_le_bytes());
        buf
    }

    #[test]
    fn fat_cycle_does_not_hang() {
        let bytes = craft_cfb_with_fat_cycle();
        // The directory chain (sector 1 -> 1 -> 1 ...) loops. Must be an Err,
        // and — crucially — must return promptly instead of hanging.
        let res = CompoundFile::open(&bytes);
        assert!(
            matches!(res, Err(CfbError::CycleDetected) | Err(CfbError::BadSectorChain)),
            "expected a cycle/chain error, got {res:?}"
        );
    }

    #[test]
    fn read_fat_chain_detects_direct_self_loop() {
        // Unit-test the chain walker in isolation with a hand-built FAT.
        let cf = CompoundFile {
            data: vec![0u8; HEADER_LEN + 512 * 4],
            sector_size: 512,
            mini_sector_size: 64,
            mini_cutoff: 4096,
            fat: vec![1, 1, 2, 3], // sector 0 -> 1 -> 1 -> ... loop
            mini_fat: vec![],
            dir: vec![],
            mini_stream: vec![],
            entries: vec![],
        };
        let res = cf.read_fat_chain(0, None);
        assert_eq!(res, Err(CfbError::CycleDetected));
    }

    #[test]
    fn read_mini_chain_detects_cycle() {
        let cf = CompoundFile {
            data: vec![0u8; HEADER_LEN],
            sector_size: 512,
            mini_sector_size: 64,
            mini_cutoff: 4096,
            fat: vec![],
            mini_fat: vec![0, 0], // mini-sector 0 -> 0 loop
            dir: vec![],
            mini_stream: vec![0u8; 128],
            entries: vec![],
        };
        let res = cf.read_mini_chain(0, 128);
        assert_eq!(res, Err(CfbError::CycleDetected));
    }

    // ---- Bounds guards ----------------------------------------------------

    #[test]
    fn sector_range_rejects_out_of_bounds() {
        let cf = CompoundFile {
            data: vec![0u8; HEADER_LEN + 512], // only sector 0 exists
            sector_size: 512,
            mini_sector_size: 64,
            mini_cutoff: 4096,
            fat: vec![],
            mini_fat: vec![],
            dir: vec![],
            mini_stream: vec![],
            entries: vec![],
        };
        assert!(cf.sector_range(0).is_ok());
        assert_eq!(cf.sector_range(1), Err(CfbError::BadSectorChain));
        // Sentinel-ish huge index must be rejected, not overflow.
        assert_eq!(cf.sector_range(0xFFFF_FFF0), Err(CfbError::BadSectorChain));
    }

    #[test]
    fn read_fat_chain_rejects_special_sector_in_chain() {
        let cf = CompoundFile {
            data: vec![0u8; HEADER_LEN + 512 * 2],
            sector_size: 512,
            mini_sector_size: 64,
            mini_cutoff: 4096,
            fat: vec![FATSECT], // chain begins on a FAT sector -> invalid
            mini_fat: vec![],
            dir: vec![],
            mini_stream: vec![],
            entries: vec![],
        };
        assert_eq!(cf.read_fat_chain(0, None), Err(CfbError::BadSectorChain));
    }

    // ---- Round-trip / mini-stream (crafted) -------------------------------

    /// Build a tiny valid CFB whose single stream is small enough to live in the
    /// mini-stream, exercising the mini-FAT read path end-to-end.
    fn craft_cfb_with_mini_stream() -> Vec<u8> {
        let sector = 512usize;
        // Sectors: 0=FAT, 1=directory, 2=mini-FAT, 3=mini-stream.
        let mut buf = vec![0u8; HEADER_LEN + 4 * sector];
        buf[0..8].copy_from_slice(&SIGNATURE);
        buf[30] = 0x09;
        buf[32] = 0x06;
        buf[44..48].copy_from_slice(&1u32.to_le_bytes()); // 1 FAT sector
        buf[48..52].copy_from_slice(&1u32.to_le_bytes()); // dir @ sector 1
        buf[56..60].copy_from_slice(&4096u32.to_le_bytes()); // cutoff
        buf[60..64].copy_from_slice(&2u32.to_le_bytes()); // mini-FAT @ sector 2
        buf[64..68].copy_from_slice(&1u32.to_le_bytes()); // 1 mini-FAT sector
        buf[68..72].copy_from_slice(&ENDOFCHAIN.to_le_bytes());
        buf[72..76].copy_from_slice(&0u32.to_le_bytes());
        // DIFAT[0] = 0
        buf[76..80].copy_from_slice(&0u32.to_le_bytes());
        for i in 1..HEADER_DIFAT_COUNT {
            let off = HEADER_DIFAT_OFFSET + i * 4;
            buf[off..off + 4].copy_from_slice(&FREESECT.to_le_bytes());
        }
        // ---- FAT (sector 0) ----
        let fat_off = HEADER_LEN;
        let set_fat = |buf: &mut [u8], idx: usize, val: u32| {
            let o = fat_off + idx * 4;
            buf[o..o + 4].copy_from_slice(&val.to_le_bytes());
        };
        for i in 0..(sector / 4) {
            set_fat(&mut buf, i, FREESECT);
        }
        set_fat(&mut buf, 0, FATSECT); // sector 0 is the FAT
        set_fat(&mut buf, 1, ENDOFCHAIN); // directory: one sector
        set_fat(&mut buf, 2, ENDOFCHAIN); // mini-FAT: one sector
        set_fat(&mut buf, 3, ENDOFCHAIN); // mini-stream: one sector
        // ---- Directory (sector 1) : root + one stream ----
        let dir_off = HEADER_LEN + sector;
        // Root entry (index 0): type 5, child -> entry 1, mini-stream @ sector 3.
        let root = dir_off;
        write_name(&mut buf, root, "Root Entry");
        buf[root + 66] = 5; // root storage
        buf[root + 68..root + 72].copy_from_slice(&NOSTREAM.to_le_bytes()); // left
        buf[root + 72..root + 76].copy_from_slice(&NOSTREAM.to_le_bytes()); // right
        buf[root + 76..root + 80].copy_from_slice(&1u32.to_le_bytes()); // child=1
        buf[root + 116..root + 120].copy_from_slice(&3u32.to_le_bytes()); // mini-stream start sector
        buf[root + 120..root + 128].copy_from_slice(&64u64.to_le_bytes()); // mini-stream size
        // Stream entry (index 1): type 2, "Tiny", 8 bytes, mini-sector 0.
        let st = dir_off + DIR_ENTRY_SIZE;
        write_name(&mut buf, st, "Tiny");
        buf[st + 66] = 2; // stream
        buf[st + 68..st + 72].copy_from_slice(&NOSTREAM.to_le_bytes());
        buf[st + 72..st + 76].copy_from_slice(&NOSTREAM.to_le_bytes());
        buf[st + 76..st + 80].copy_from_slice(&NOSTREAM.to_le_bytes());
        buf[st + 116..st + 120].copy_from_slice(&0u32.to_le_bytes()); // mini-sector 0
        buf[st + 120..st + 128].copy_from_slice(&8u64.to_le_bytes()); // 8 bytes
        // ---- mini-FAT (sector 2) ----
        let mf_off = HEADER_LEN + 2 * sector;
        for i in 0..(sector / 4) {
            let o = mf_off + i * 4;
            buf[o..o + 4].copy_from_slice(&FREESECT.to_le_bytes());
        }
        // mini-sector 0 is the whole (single-mini-sector) stream: ENDOFCHAIN.
        buf[mf_off..mf_off + 4].copy_from_slice(&ENDOFCHAIN.to_le_bytes());
        // ---- mini-stream (sector 3) : put recognizable bytes at mini-sector 0 ----
        let ms_off = HEADER_LEN + 3 * sector;
        let payload = [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04];
        buf[ms_off..ms_off + 8].copy_from_slice(&payload);
        buf
    }

    fn write_name(buf: &mut [u8], entry_off: usize, name: &str) {
        let utf16: Vec<u16> = name.encode_utf16().collect();
        let mut i = 0;
        for u in &utf16 {
            let b = u.to_le_bytes();
            buf[entry_off + i] = b[0];
            buf[entry_off + i + 1] = b[1];
            i += 2;
        }
        // NUL terminator already zero. name byte-len incl. terminator:
        let nlen = (utf16.len() + 1) * 2;
        buf[entry_off + 64..entry_off + 66].copy_from_slice(&(nlen as u16).to_le_bytes());
    }

    #[test]
    fn mini_stream_round_trip() {
        let bytes = craft_cfb_with_mini_stream();
        let cf = CompoundFile::open(&bytes).expect("crafted mini CFB should parse");
        let names = cf.stream_names();
        assert!(names.iter().any(|n| n == "Tiny"), "got {names:?}");
        let data = cf.read_stream("Tiny").expect("Tiny stream");
        assert_eq!(data, vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn read_stream_by_id_on_storage_is_not_a_stream() {
        let cf = CompoundFile::open(MINIMAL_XLS).unwrap();
        // Entry 0 is the root storage, never a stream.
        assert_eq!(cf.read_stream_by_id(0), Err(CfbError::NotAStream));
    }

    #[test]
    fn error_display_is_human_readable() {
        assert!(format!("{}", CfbError::BadSignature).contains("signature"));
        assert!(format!("{}", CfbError::CycleDetected).contains("cycle"));
        assert!(!format!("{}", CfbError::Truncated).is_empty());
    }

    #[test]
    fn utf16_name_decodes_and_preserves_control_prefix() {
        // Build a 64-byte field for "\u{5}Info".
        let mut field = [0u8; 64];
        let name = "\u{5}Info";
        let utf16: Vec<u16> = name.encode_utf16().collect();
        let mut i = 0;
        for u in &utf16 {
            let b = u.to_le_bytes();
            field[i] = b[0];
            field[i + 1] = b[1];
            i += 2;
        }
        let nlen = (utf16.len() + 1) * 2;
        let decoded = decode_utf16_name(&field, nlen);
        assert_eq!(decoded, "\u{5}Info");
    }

    #[test]
    fn utf16_name_zero_length_is_empty() {
        let field = [0u8; 64];
        assert_eq!(decode_utf16_name(&field, 0), "");
    }

    /// Build a synthetic `CompoundFile` whose directory is a very deep degenerate
    /// left-sibling chain (entry i's left = i+1). Native recursion would blow the
    /// stack; the iterative work-list must flatten it without panicking.
    fn cf_with_deep_dir_chain(n: u32, make_cycle: bool) -> CompoundFile {
        let mut dir = Vec::with_capacity(n as usize + 1);
        // Root (index 0), child -> entry 1.
        dir.push(DirEntry {
            name: "Root Entry".into(),
            object_type: 5,
            left: NOSTREAM,
            right: NOSTREAM,
            child: if n > 0 { 1 } else { NOSTREAM },
            start_sector: ENDOFCHAIN,
            size: 0,
        });
        for i in 1..=n {
            // Each entry chains to the next via left; last one terminates
            // (or, if make_cycle, points back to entry 1 to force a cycle).
            let left = if i < n {
                i + 1
            } else if make_cycle {
                1
            } else {
                NOSTREAM
            };
            dir.push(DirEntry {
                name: format!("s{i}"),
                object_type: 2, // stream
                left,
                right: NOSTREAM,
                child: NOSTREAM,
                start_sector: ENDOFCHAIN,
                size: 0,
            });
        }
        CompoundFile {
            data: vec![0u8; HEADER_LEN],
            sector_size: 512,
            mini_sector_size: 64,
            mini_cutoff: 4096,
            fat: vec![],
            mini_fat: vec![],
            dir,
            mini_stream: vec![],
            entries: vec![],
        }
    }

    #[test]
    fn deep_directory_chain_does_not_overflow_stack() {
        // 200k-deep chain: native recursion would overflow; iterative must not.
        let cf = cf_with_deep_dir_chain(200_000, false);
        let entries = cf.enumerate_entries().expect("deep chain should flatten");
        // Root + all n stream entries.
        assert_eq!(entries.len(), 200_001);
    }

    #[test]
    fn deep_directory_cycle_is_detected_not_hung() {
        let cf = cf_with_deep_dir_chain(50_000, true);
        assert_eq!(cf.enumerate_entries().err(), Some(CfbError::CycleDetected));
    }
}
