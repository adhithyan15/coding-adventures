# XLSW01 — Legacy `.xls` (BIFF8) writer

> A **writer** for the legacy **`.xls` / BIFF8** spreadsheet format ([MS-XLS]).
> You give it a simple spreadsheet model — sheets of string and number cells —
> and it produces the bytes of a real `.xls` file: a stream of **BIFF records**
> wrapped in an **OLE2 / Compound File** container. This is milestone **C4**
> (legacy write) of the OOXML/BIFF stack.
>
> Implemented by `code/packages/rust/xls-writer` (crate `xls-writer`).

This spec assumes you have read [CFBW01](CFBW01-cfb-writer.md) (the container
writer) and skimmed [CFB01](CFB01-compound-file.md) (the reader). This crate
sits **on top** of `cfb-writer`: it builds one `Workbook` byte-stream and hands
it to `cfb-writer` to be wrapped into the `.xls` file.

```
        deflate ── zip ── xml ── opc ── spreadsheetml ── xlsx-eval   (OOXML, .xlsx)

   cfb (reader) ◄──────── round-trip proof ────────► cfb-writer     (OLE2 container)
                                                          │
                                                     xls-writer      (this crate — BIFF8)
```

---

## 1. What a minimal `.xls` actually is

A `.xls` file is an **OLE2 Compound File** (the "FAT filesystem in a file" from
CFB01) that contains a single named stream called **`Workbook`**. That stream
holds a flat sequence of **BIFF records**. So writing a `.xls` is two layers:

1. **This crate:** turn the model into a `Vec<u8>` of BIFF records.
2. **`cfb-writer`:** wrap that `Vec<u8>` as the `Workbook` stream of a CFB file.

We are done the moment `cfb_writer::write_cfb(&[("Workbook", &biff)])` returns.

### 1.1 The BIFF record framing

Every BIFF record is a tiny TLV (type-length-value):

```text
   ┌────────────┬────────────┬───────────────────────────┐
   │ u16 type   │ u16 size   │  `size` bytes of body      │
   │ (LE)       │ (LE)       │                            │
   └────────────┴────────────┴───────────────────────────┘
     2 bytes      2 bytes       `size` bytes
```

To *walk* a BIFF stream you repeatedly read a 4-byte header, then skip `size`
body bytes — exactly what the round-trip test's in-test walker does. Because
`size` is a `u16`, **no single record body may exceed 65535 bytes**; larger
payloads (a big shared-string table) must be split across `CONTINUE` records.
We keep our one variable-length record (the SST) small enough to avoid that;
see §6.

The record types we emit:

| Name         | Type (hex) | Meaning                                        |
|--------------|-----------:|------------------------------------------------|
| `BOF`        |   `0x0809` | Beginning of a substream                        |
| `EOF`        |   `0x000A` | End of a substream                              |
| `BOUNDSHEET` |   `0x0085` | Declares one worksheet: name + byte offset      |
| `SST`        |   `0x00FC` | Shared string table (all distinct string values)|
| `LABELSST`   |   `0x00FD` | A string cell (references an SST index)         |
| `NUMBER`     |   `0x0203` | A numeric cell (an IEEE-754 `f64`)              |

### 1.2 Substreams: globals + one per worksheet

BIFF records are grouped into **substreams**, each bracketed by a `BOF` … `EOF`
pair. A workbook is:

```text
   ┌─────────────────────────── Workbook stream ───────────────────────────┐
   │  BOF(globals) BOUNDSHEET… SST EOF │ BOF(sheet) cells… EOF │ BOF … EOF   │
   │  └──────── globals substream ─────┘└─ worksheet 0 ───────┘└ worksheet 1┘│
   └───────────────────────────────────────────────────────────────────────┘
```

- The **globals substream** (`BOF` dt=`0x0005`) declares the sheets
  (`BOUNDSHEET`, one per sheet) and holds the shared-string table (`SST`).
- Each **worksheet substream** (`BOF` dt=`0x0010`) holds that sheet's cell
  records.

`BOF` bodies are 16 bytes: `u16 vers=0x0600` (BIFF8), `u16 dt` (substream type),
then `u16 rupBuild`, `u16 rupYear`, `u32 bfh`, `u32 sfo` — all four trailers
zeroed for deterministic output.

---

## 2. Cells: `LABELSST` and `NUMBER`

Every cell record starts with a 6-byte **cell head**: `u16 row`, `u16 col`,
`u16 ixfe` (the cell format / XF index — we always use `0`, the default XF).
All rows/cols are **0-based** in BIFF.

- **`NUMBER` (`0x0203`)** — cell head + an 8-byte little-endian IEEE-754 `f64`.
- **`LABELSST` (`0x00FD`)** — cell head + `u32 isst`, an index into the SST.

### 2.1 Why `NUMBER`, not `RK`?

BIFF also has an `RK` record that packs certain numbers (30-bit integers, or
`f64`s whose low 34 bits are zero) into 4 bytes. It is a *space optimization*
only. We deliberately choose `NUMBER` for **every** numeric cell:

- It is total: it represents **any** `f64` exactly, with no case analysis.
- It keeps the writer simple and the output easy to verify (the 8 bytes are just
  `f64::to_le_bytes`).

`RK` would save 4 bytes per qualifying cell but adds encoding branches and a
whole extra record type to test. For a from-scratch, correctness-first writer
that trade is not worth it. (A future optimization pass could add `RK`.)

---

## 3. Shared strings (the SST) and `LABELSST`

String cell *values* are not stored in the cell record. Instead every distinct
string lives once in the **SST** (shared string table) in the globals substream,
and each string cell is a `LABELSST` holding the **index** of its value in the
SST. This deduplicates repeated strings (think a column of `"Yes"`/`"No"`).

The SST body:

```text
   u32 cstTotal    ← total number of string-cell references (LABELSST count)
   u32 cstUnique   ← number of DISTINCT strings stored below
   then, cstUnique times, an XLUnicodeRichExtendedString:
       u16 cch      ← character count
       u8  grbit    ← flags; bit0 fHighByte (see §5)
       chars…       ← cch bytes (8-bit) or cch*2 bytes (16-bit UTF-16LE)
```

`cstTotal` counts *cells*; `cstUnique` counts *distinct strings*. Writing two
cells with the same string gives `cstTotal=2, cstUnique=1`, and both `LABELSST`
records carry the same `isst`.

We do **not** set the rich-text (`fRichSt`) or extended (`fExtSt`) bits, and we
emit no `rgRun`/`ExtRst` trailer — those bits being clear means the string is
exactly `cch` characters and nothing more.

---

## 4. The `BOUNDSHEET` `lbPlyPos` two-pass

This is the one genuinely fiddly part. Each `BOUNDSHEET` record body is:

```text
   u32 lbPlyPos    ← BYTE OFFSET, within the Workbook stream, of this sheet's
                     worksheet BOF record
   u8  hsState = 0 ← visible
   u8  dt      = 0 ← worksheet
   ShortXLUnicodeString name:
       u8 cch, u8 grbit, then the chars
```

`lbPlyPos` must point at the *start of that sheet's worksheet `BOF`*. But that
offset depends on the size of the globals substream — which **contains the
`BOUNDSHEET` records themselves**. Chicken and egg. We break it with a clean
two-pass:

```text
   ┌─────────────────────── Workbook stream ────────────────────────┐
   │  [ globals substream ]  [ worksheet 0 ]  [ worksheet 1 ] ...    │
   │  ^                      ^                ^                       │
   │  0                      G                G+W0                   │
   └────────────────────────────────────────────────────────────────┘

   lbPlyPos(sheet 0) = G            (= globals length)
   lbPlyPos(sheet 1) = G + W0
   lbPlyPos(sheet k) = G + Σ_{i<k} Wi
```

Algorithm:

1. **Build the globals buffer** with `BOUNDSHEET` records whose `lbPlyPos` field
   is a placeholder (`0`). While building, record the byte offset of each
   `BOUNDSHEET`'s `lbPlyPos` field inside the globals buffer.
2. **Build each worksheet buffer** independently; note each one's length `Wi`.
3. Now `G = globals.len()` is known. Compute each worksheet's absolute start
   offset `G + Σ_{i<k} Wi`.
4. **Backfill**: patch the 4 `lbPlyPos` bytes of each `BOUNDSHEET` in the
   globals buffer with its sheet's absolute offset.
5. **Concatenate** globals + all worksheets → the `Workbook` stream.

The round-trip test verifies this: it reads each `BOUNDSHEET.lbPlyPos`, seeks to
that offset in the stream, and asserts a `BOF` with dt=`0x0010` starts there.

---

## 5. String encoding: the `fHighByte` bit

BIFF8 strings are "compressed UTF-16": a flag bit (`grbit` bit0, `fHighByte`)
chooses the character width:

- `fHighByte = 0` → each character is **1 byte** (Latin-1 / the low byte of each
  UTF-16 code unit). Used only when *every* code unit is ≤ `0xFF`.
- `fHighByte = 1` → each character is **2 bytes**, little-endian **UTF-16LE**.

We pick per string: if all code units fit in a byte we use the compact 8-bit
form; otherwise the 16-bit form. Both the SST strings
(`XLUnicodeRichExtendedString`, `cch` is a `u16`) and the sheet names in
`BOUNDSHEET` (`ShortXLUnicodeString`, `cch` is a `u8`) use the same scheme.

The test's non-ASCII case (`"café"` → `é` is `U+00E9` ≤ `0xFF`, but a string
with, e.g., `U+2603 ☃` forces 16-bit) proves both paths decode correctly.

---

## 6. Limitations & robustness

- **No `CONTINUE` splitting.** A BIFF record body is capped at 65535 bytes. The
  SST is our only variable-length record; a workbook with enough distinct
  strings to overflow one SST record would, in a full implementation, spill into
  `CONTINUE` records. We do **not** implement that — we keep the SST small. If
  the encoded SST body would exceed the `u16` size field we clamp its declared
  size rather than emit a corrupt record; this is a documented limitation, not a
  silent-truncation bug. (Realistic small workbooks never approach the limit.)
- **`u16` field limits.** Rows and columns are `u16` in BIFF; a cell beyond
  `65535` in either axis cannot be represented and is **skipped** (documented)
  rather than wrapped into a wrong address. A string longer than `u16::MAX`
  characters likewise cannot fit its `cch` field; we clamp/skip rather than
  truncate into a wrong length.
- **Deterministic.** No timestamps or randomness anywhere; identical models
  produce identical bytes.
- **Totality.** `#![forbid(unsafe_code)]`; no `unwrap`/`expect`/`panic!` on the
  public path; `checked_*` arithmetic guards every size computation.

---

## 7. Public API

```rust
pub struct Workbook { /* sheets */ }
impl Workbook {
    pub fn new() -> Self;
    pub fn add_sheet(&mut self, name: &str) -> &mut Sheet;
}
pub struct Sheet { /* name + cells */ }
impl Sheet {
    pub fn set_string(&mut self, row: u32, col: u32, s: &str);
    pub fn set_number(&mut self, row: u32, col: u32, n: f64);
}
pub fn write_xls(wb: &Workbook) -> Vec<u8>;   // -> .xls bytes
```

---

## 8. The round-trip proof

The proof (a test) builds a workbook, calls `write_xls`, opens the bytes with
the `cfb` reader, extracts the `Workbook` stream, and walks its BIFF records to
assert: the `BOUNDSHEET` name and its `lbPlyPos` (pointing at a worksheet `BOF`),
the SST contents, and every cell's type/address/value. This closes the loop
`model → BIFF → CFB → cfb reader → BIFF → model` and is the single most
important test in the crate.
