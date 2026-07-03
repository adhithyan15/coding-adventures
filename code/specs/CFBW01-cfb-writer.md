# CFBW01 — Compound File Binary Format writer (OLE2)

> A **writer** for the **Compound File Binary Format** ([MS-CFB]) — the exact
> inverse of the [CFB01](CFB01-compound-file.md) reader. You give it a set of
> named byte-streams; it produces a valid CFB file (version 3, 512-byte
> sectors). This is the container foundation for **writing** legacy `.xls`,
> `.doc`, and `.ppt` files (milestone **C4**): a `.xls` writer builds a
> `Workbook` stream of BIFF records and hands it here to be wrapped.

This spec assumes you have read [CFB01](CFB01-compound-file.md), which explains
the format from first principles ("a FAT filesystem crammed into a single
file"). Here we focus on the *encoding* problem: given streams, how do we lay
out sectors, build FAT chains, and emit a header the reader will accept?

Implemented by `code/packages/rust/cfb-writer` (crate `cfb-writer`).

---

## 1. The mental model, from the writer's side

Reading a CFB file is *following* linked lists. Writing one is *constructing*
them. The whole job is bookkeeping:

1. Decide where every byte lives (which sector, or which mini-sector).
2. Write the "next" pointers (the FAT and mini-FAT) that stitch those sectors
   into per-stream chains.
3. Describe the named objects in a directory.
4. Emit a header that points at the first directory sector, the FAT, the
   mini-FAT, and records the mini-stream cutoff.

The one genuinely circular part — the FAT must describe *its own* sectors — is
resolved with a tiny fixed-point loop (§6).

---

## 2. The output we commit to

We always emit **version 3**:

| Field                | Value    | Meaning                                  |
| -------------------- | -------- | ---------------------------------------- |
| signature            | `D0 CF 11 E0 A1 B1 1A E1` | the CFB magic            |
| major version        | `0x0003` | version 3                                |
| sector shift         | `0x0009` | 512-byte sectors (`1 << 9`)              |
| mini sector shift    | `0x0006` | 64-byte mini-sectors (`1 << 6`)          |
| mini-stream cutoff   | `0x1000` | 4096 bytes                               |
| byte order           | `0xFFFE` | little-endian                            |

All CLSID and timestamp fields are **zeroed**, making output deterministic
(identical input → identical bytes), which keeps tests byte-stable.

---

## 3. The sector layout we choose

CFB does not mandate an order for the FAT, directory, mini-FAT, mini-stream, and
data sectors — any order the pointers describe is legal. We pick a **fixed,
simple order** so the writer is easy to reason about and the reader trivially
walks it. Sector 0 is the first sector *after* the 512-byte header.

```text
  offset 0        HEADER (512 bytes)
  ├── sector 0..a   directory stream      (128-byte entries; Root Entry first)
  ├── sector a..b   mini-FAT stream        (only if any small streams exist)
  ├── sector b..c   mini-stream            (packed 64-byte mini-sectors)
  ├── sector c..d   large stream 0 data    (>= cutoff), back to back
  │                 large stream 1 data ...
  └── sector d..e   FAT sectors            (marked FATSECT; listed in the DIFAT)
```

Everything except the FAT sectors is a **data sector**; the FAT sectors are
appended last because their count depends on the total (§6).

---

## 4. The small-vs-large decision

Each stream is routed by size, matching the reader's `read_stream_by_id`:

- **size == 0** → *empty*: no sectors at all; the directory entry's start sector
  is `ENDOFCHAIN` and its size is 0.
- **0 < size < 4096** → *small*: packed into the **mini-stream**; the directory
  entry's start sector is a **mini-sector index**, and its chain lives in the
  **mini-FAT**.
- **size >= 4096** → *large*: gets its own **regular 512-byte sectors**; the
  directory entry's start sector is a regular sector index, and its chain lives
  in the **FAT**.

The boundary is strict: exactly 4096 bytes is **large** (the cutoff is `<`).

### The mini-stream and mini-FAT

The mini-stream is the concatenation of every small stream, each padded up to a
whole number of 64-byte mini-sectors. It is itself an ordinary FAT stream
**owned by the Root Entry** (the root's start-sector + size fields point at it).
The mini-FAT is a parallel `u32` chain indexing 64-byte mini-sectors: for a
small stream occupying mini-sectors *k..k+m*, `mini_fat[k+i] = k+i+1` for the
first *m-1*, and `mini_fat[k+m-1] = ENDOFCHAIN`.

```text
   small streams:  [ "hi" (2B) ][ 100×0x01 (100B) ]
   mini-stream:    | ms0 (64B) | ms1 (64B) | ms2 (64B) |   (padded)
                     "hi"+pad     100 bytes spill ------>
   mini-FAT:       ms0 -> EOC ;  ms1 -> ms2 ;  ms2 -> EOC
```

---

## 5. The directory

The directory is a stream of 128-byte entries. Entry 0 is the **Root Entry**
(object type `0x05`); entries 1.. are the streams (object type `0x02`) in
insertion order.

We build the simplest valid tree the reader accepts: the root's **child**
points at the first stream (id 1), and each stream's **right-sibling** points at
the next (id 1 → 2 → 3 → … → NOSTREAM). Every node is coloured **black**; left
and child pointers on stream nodes are `NOSTREAM`. An "all black" tree with no
left children is a degenerate-but-valid red-black tree — the reader only needs
it *walkable*, not balanced, so this is sufficient and simple.

Directory entry fields we write:

| Offset | Size | Field                                                    |
| ------ | ---- | -------------------------------------------------------- |
| 0      | 64   | name, UTF-16LE, NUL-terminated                           |
| 64     | 2    | name byte-length *including* the NUL                     |
| 66     | 1    | object type (`0x05` root, `0x02` stream)                 |
| 67     | 1    | colour (`0x01` = black)                                  |
| 68     | 4    | left sibling id (always `NOSTREAM` for us)               |
| 72     | 4    | right sibling id                                         |
| 76     | 4    | child id (root only)                                     |
| 80     | 16   | CLSID (zero)                                             |
| 96     | 4    | state flags (zero)                                       |
| 100    | 16   | created/modified timestamps (zero)                       |
| 116    | 4    | starting sector (regular sector, or mini-sector index)   |
| 120    | 8    | stream size (u64); for the root, the mini-stream length  |

Trailing space in the last directory sector is filled with **unused** entries
(object type 0, left/right/child = `NOSTREAM`), which the reader skips.

### Names

The name field is 64 bytes = 32 UTF-16 units, one reserved for the NUL, so a
name may be at most **31 UTF-16 code units**. Longer names are **truncated** to
31 units (the API stays infallible). We store the exact **logical** byte length
in the size field — the reader slices to that — so padding a sector's tail with
zeros is harmless.

---

## 6. The fixed-point FAT-sector count

Here is the only circular dependency. The FAT needs one `u32` slot per sector —
**including the FAT's own sectors**. A 512-byte FAT sector holds 128 slots.
Suppose we have *D* data sectors. Then:

```text
  num_fat_sectors = ceil( (D + num_fat_sectors) / 128 )
```

`num_fat_sectors` appears on both sides. We solve it by iteration from 0:

```text
  n = 0
  loop:
     needed = ceil((D + n) / 128)
     if needed == n: done
     n = needed
```

This converges in at most two rounds for any realistic file: adding a FAT sector
only pushes you over a 128-sector boundary occasionally. Once *n* is known, the
FAT sectors occupy sector indices `D .. D+n`, we mark each `FATSECT` in the FAT,
and we list them in the DIFAT.

### The DIFAT

The DIFAT is the "where are the FAT sectors" index. Its first 109 entries are
**inlined in the header** (offset 76). One 512-byte FAT sector maps 128 sectors
= 64 KiB of file; 109 of them map hundreds of MiB — beyond any realistic legacy
Office file and beyond the reader's 256 MiB safety cap — so we never need a
DIFAT-sector *chain*. Unused DIFAT slots are `FREESECT`. First-DIFAT-sector is
`ENDOFCHAIN`; num-DIFAT-sectors is 0.

---

## 7. Serialisation, end to end

```text
  Layout::build(streams):
    1. partition streams: empty / small / large
    2. build mini-stream + mini-FAT (pad mini-stream to whole sectors)
    3. build directory entries (root + one per stream)
    4. assign regular sectors: directory, mini-FAT, mini-stream, large data
    5. write FAT chains for every data sector
    6. patch large-stream + root start sectors into the directory; re-encode
    7. fixed-point num_fat_sectors; append + mark FAT sectors FATSECT

  Layout::serialise():
    header (512B) + DIFAT
    ++ directory ++ mini-FAT ++ mini-stream ++ large data ++ FAT sectors
```

---

## 8. Robustness

- `#![forbid(unsafe_code)]`, pure `std`.
- No `unwrap`/`expect`/`panic!` on the public path. The output buffer is always
  pre-sized to a computed length, so every write is in bounds by construction.
- Overlong names are truncated (documented), not rejected.
- Empty stream and empty stream set both yield valid files.
- Sector-count arithmetic uses checked/`u64` math where sizes could be large, so
  a huge stream cannot silently wrap the sector count.
- Deterministic: no timestamps, no randomness.

---

## 9. Round-trip verification (the proof)

The contract is: **anything we write, the [CFB01](CFB01-compound-file.md) reader
reads back byte-for-byte.** The centrepiece test writes a mix of a large stream
(≥ 4096 → regular FAT) and small streams (< 4096 → mini-FAT), reopens the bytes
with `cfb::CompoundFile::open`, and asserts each `read_stream` equals the
original. Additional tests cover: a single stream, an empty stream, the exact
4096-byte boundary, many small streams overflowing one mini-sector, a stream
spanning multiple FAT sectors, control-prefixed/Unicode names, name truncation,
and determinism.

[MS-CFB]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-cfb/
