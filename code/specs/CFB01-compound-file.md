# CFB01 — Compound File Binary Format (OLE2)

> A reader for the **Compound File Binary Format** ([MS-CFB]), also known as
> **OLE2 storage** or the **Structured Storage** format. This is the container
> that lives *inside* legacy Microsoft Office files: a `.xls`, `.doc`, or `.ppt`
> from before the Office 2007 XML era is, on disk, a CFB file. Inside it are
> named byte-streams (`Workbook`, `WordDocument`, `PowerPoint Document`, …) that
> the format-specific readers then parse.

This spec explains the format from first principles, the way you'd explain a
tiny filesystem to someone who has never seen one — because that is exactly
what CFB is: **a FAT filesystem crammed into a single file.**

---

## 1. The mental model: a filesystem in a file

If you have ever heard of the old FAT filesystem on a floppy disk, you already
understand 90% of CFB. A CFB file is organized exactly like a disk:

- The file is chopped into fixed-size **sectors** (usually 512 bytes each,
  like disk blocks).
- A **File Allocation Table (FAT)** is a big array of "next sector" pointers.
  To read a file that spans several sectors, you start at its first sector and
  follow the FAT like a linked list: `FAT[n]` tells you the sector that comes
  *after* sector `n`. A special marker (`ENDOFCHAIN`) means "this was the last
  sector."
- A **directory** lists the named objects (like files and folders). CFB calls
  a file a **stream** and a folder a **storage**.

So reading a stream by name is a two-step dance:

1. Look it up in the directory to find its *start sector* and *byte size*.
2. Follow the FAT chain from that start sector, gluing sectors together, then
   trim to the exact byte size.

There is one extra wrinkle — the **mini-stream** — which we cover in §6. It
exists purely to avoid wasting space on tiny streams. Hold that thought.

---

## 2. The header (first 512 bytes)

The very first 512 bytes are the header. It is *not* sector 0 — sectors are
numbered starting *after* the header. The fields we care about:

| Offset | Size | Meaning |
|-------:|-----:|---------|
| 0      | 8    | **Signature** — must be `D0 CF 11 E0 A1 B1 1A E1`. (A little joke: it looks like "DOCFILE" if you squint at the hex.) |
| 30     | 2    | **Sector shift** — `0x0009` → 512-byte sectors, `0x000C` → 4096-byte sectors. The sector size is `1 << shift`. |
| 32     | 2    | **Mini sector shift** — `0x0006` → 64-byte mini-sectors (`1 << 6`). |
| 44     | 4    | Number of FAT sectors. |
| 48     | 4    | First **directory** sector. |
| 56     | 4    | **Mini-stream cutoff** — streams *smaller* than this go in the mini-stream (usually 4096). |
| 60     | 4    | First **mini-FAT** sector. |
| 64     | 4    | Number of mini-FAT sectors. |
| 68     | 4    | First **DIFAT** sector (for files with > 109 FAT sectors). |
| 72     | 4    | Number of DIFAT sectors. |
| 76     | 436  | **DIFAT array** — the first 109 FAT-sector locations, as `u32` each. |

All multi-byte integers are **little-endian**.

### Sector addressing

Sector `N` begins at file offset:

```
offset(N) = 512 + N * sector_size
```

The `512` is the header. (For 4096-byte sectors, the header is still 512 bytes,
but padded out; sector 0 still starts at offset 512 in the layouts we support —
we compute `header_len.max(sector_size)` conceptually, but the fixture and all
common real files use 512-byte sectors where `offset(N) = 512 + N*512`.)

---

## 3. Special FAT / DIFAT values

Certain `u32` values are not real sector numbers but sentinels:

| Value        | Name        | Meaning |
|--------------|-------------|---------|
| `0xFFFFFFFF` | FREESECT    | Free / unused. |
| `0xFFFFFFFE` | ENDOFCHAIN  | End of a sector chain. |
| `0xFFFFFFFD` | FATSECT     | This sector holds part of the FAT itself. |
| `0xFFFFFFFC` | DIFSECT     | This sector holds part of the DIFAT. |

---

## 4. Assembling the FAT

The FAT is one giant array of `u32` next-pointers, but it is itself scattered
across the file in **FAT sectors**. Where are those FAT sectors? The **DIFAT**
(Double-Indirect FAT) tells you:

- The first **109** FAT-sector locations are inlined in the header (offset 76).
- If a file needs more than 109 FAT sectors, the header's *first DIFAT sector*
  (offset 68) starts a chain of extra DIFAT sectors, each holding more
  FAT-sector locations plus (in its last `u32`) a pointer to the next DIFAT
  sector.

To build the FAT: collect all FAT-sector numbers (header DIFAT + DIFAT chain),
then read each of those sectors as `sector_size/4` little-endian `u32` entries
and concatenate. Now `FAT[n]` is a plain array lookup.

---

## 5. The directory

The directory is *itself a stream* stored in the regular FAT, starting at the
header's "first directory sector" and chained via the FAT. It is a flat array of
**128-byte directory entries**:

| Offset | Size | Field |
|-------:|-----:|-------|
| 0      | 64   | Name, UTF-16LE, null-terminated. |
| 64     | 2    | Name byte-length (**including** the null terminator). |
| 66     | 1    | Object type: `0`=unused, `1`=storage, `2`=stream, `5`=root storage. |
| 67     | 1    | Node color (red/black — see below). |
| 68     | 4    | Left-sibling directory ID. |
| 72     | 4    | Right-sibling directory ID. |
| 76     | 4    | Child directory ID (for a storage). |
| 116    | 4    | Starting sector. |
| 120    | 8    | Stream size (`u64`). |

`0xFFFFFFFF` in a sibling/child field means "none".

### Entry 0 is special — the ROOT

The first directory entry is the **root storage**. Its two most important
fields are re-purposed:

- Its **starting sector** = the start of the **mini-stream** (§6).
- Its **size** = the total byte length of the mini-stream.

### Why a red-black tree?

Within one storage, the child entries are organized as a **red-black tree**
(balanced binary search tree) keyed by name, so a program can find a named
child in `O(log n)` without scanning. As a *reader* we don't need the balancing
— we just want every entry — so we walk it recursively: visit the child, then
recurse into its left and right subtrees. That enumerates every stream and
storage. (We still cycle-guard: a malicious file could point a sibling back at
an ancestor.)

---

## 6. The mini-stream and mini-FAT

Real Office files have *many* tiny streams (e.g. the summary-information blobs
are a few hundred bytes). Storing each in its own 512-byte sector would waste
enormous space. CFB's fix: a **mini-stream**.

- Streams whose size **≥ cutoff** (usually 4096) are stored the normal way —
  regular FAT, `sector_size` sectors.
- Streams whose size **< cutoff** are packed into the **mini-stream**: a single
  regular-FAT stream (pointed to by the root entry) subdivided into **64-byte
  mini-sectors**.
- A parallel **mini-FAT** (a regular-FAT stream starting at header offset 60)
  provides the "next mini-sector" links, exactly like the FAT does for real
  sectors.

So to read a small stream: follow its chain **in the mini-FAT**, and for each
mini-sector index `m`, slice 64 bytes out of the (already-assembled) mini-stream
at byte offset `m * 64`. Then trim to the stream's byte size.

A reader **must** implement both paths. The [MS-CFB] fixture in our tests uses
the regular-FAT path (a 4096-byte `Workbook` stream, exactly at the cutoff, so
it is *not* mini), and our unit tests separately exercise the mini path with a
hand-crafted case.

---

## 7. Reading untrusted bytes safely

CFB files come from the open internet (email attachments!). A hostile file can:

- **Lie about sizes** to make us allocate gigabytes (a directory entry claiming
  a 4 GB stream inside a 2 KB file). → We **bounds-check** every stream size
  against the file length and enforce a global output cap.
- **Loop the FAT** so a chain never reaches ENDOFCHAIN (`FAT[5] = 5`). → Every
  chain walk is **cycle-guarded** with a visited-set *and* a hard iteration cap
  equal to the total number of sectors. A cycle returns `Err`, never hangs.
- **Point sectors out of bounds.** → Every sector→offset computation is checked
  against the byte-slice length; out-of-range returns `Err`, never panics.
- **Be empty or truncated.** → Clean `Err(Truncated)`.

The reader is `#![forbid(unsafe_code)]` and contains **no** `unwrap`/`expect`/
`panic!` on input-derived data. Every fallible step returns a `CfbError`.

---

## 8. Public API

```rust
let cf = CompoundFile::open(bytes)?;          // parse header, FAT, directory
for e in cf.entries() {                        // list streams & storages
    println!("{} ({} bytes, {:?})", e.name, e.size, e.kind);
}
let data: Vec<u8> = cf.read_stream("Workbook") // decode a stream by name
    .ok_or("missing")?;
```

- `CompoundFile::open(&[u8]) -> Result<CompoundFile, CfbError>`
- `entries() -> &[Entry]` — name, size, kind (`Stream` / `Storage` / `RootStorage`).
- `stream_names() -> Vec<String>` — convenience: just the stream names.
- `read_stream(name) -> Option<Vec<u8>>` — case-insensitive top-level lookup.
- `read_stream_by_id(id) -> Result<Vec<u8>, CfbError>` — precise access.

### Error taxonomy (`CfbError`)

`BadSignature`, `Truncated`, `UnsupportedSectorSize`, `BadSectorChain`,
`CycleDetected`, `OutputTooLarge`, `BadDirectory`, `NotAStream`.

---

## 9. What this unblocks

CFB is the *bottom layer* of the legacy Office stack. Once we can pull the
`Workbook` / `WordDocument` / `PowerPoint Document` streams out by name, the
BIFF8 (.xls), Word binary (.doc), and PowerPoint binary (.ppt) parsers can be
built on top — each just consumes the raw stream bytes this crate hands them.

The end-to-end test proves the chain works: it opens a real minimal `.xls`,
finds the `Workbook` stream (4096 bytes), and verifies the first two bytes are
`0x09 0x08` — a **BIFF8 BOF** record (type `0x0809`), the signature of an
Excel workbook stream.
