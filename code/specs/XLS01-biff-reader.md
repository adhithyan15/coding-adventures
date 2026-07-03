# XLS01 — Legacy `.xls` (BIFF8) reader on top of CFB

> Milestone **B1** of the OOXML effort. A zero-third-party-dependency Rust crate
> (`xls`) that reads a legacy `.xls` workbook into a typed
> `Workbook → Sheet → Cell` model, layered on the already-built `cfb` crate.

## 1. Where this sits in the stack

The OOXML effort builds a bottom-up stack of pure-`std` Rust crates:

```
    zip → deflate → xml → opc → spreadsheetml → xlsx-eval   (the MODERN .xlsx path)
    cfb → xls                                                (the LEGACY .xls path)  ← THIS SPEC
```

A modern `.xlsx` is a ZIP of XML parts. A legacy `.xls` is something else
entirely: an **OLE2 Compound File** (a tiny FAT filesystem crammed into one
file) whose `Workbook` stream is a flat sequence of **BIFF records**. The `cfb`
crate (spec `CFB01`) already turns the outer container into named byte streams.
This crate parses the `Workbook` byte stream into cells.

```
   .xls bytes
      │
      ▼
   cfb::CompoundFile::open(bytes)          ← the OLE2 container (CFB01)
      │  .read_stream("Workbook")  (or "Book" for very old files)
      ▼
   Vec<u8>  = the BIFF record stream       ← THIS CRATE parses this
      │
      ▼
   Workbook { sheets: [ Sheet { name, cells: [ Cell { row, col, value } ] } ] }
```

## 2. The one-paragraph mental model of BIFF

A BIFF stream is **back-to-back records**. Every record is:

```
   ┌────────────┬────────────┬───────────────────────────┐
   │ u16 type   │ u16 size   │  `size` bytes of body      │
   │  (LE)      │  (LE)      │                            │
   └────────────┴────────────┴───────────────────────────┘
```

You walk the stream reading a 4-byte header then skipping `size` body bytes,
stopping when fewer than 4 bytes remain. Records are grouped into **substreams**
bracketed by a **BOF** (`0x0809`) record and an **EOF** (`0x000A`) record. The
BOF body's bytes `[2..4]` say what kind of substream follows:

| BOF substream type | meaning                                  |
| ------------------ | ---------------------------------------- |
| `0x0005`           | **workbook globals** (SST + sheet list)  |
| `0x0010`           | **worksheet** (one sheet's cells)        |
| `0x0006`           | chart / `0x0020` macro / `0x0040` etc.   |

The **globals** substream holds:
- the **shared string table** (`SST`) — a deduplicated pool of every string used
  by every sheet, referenced by index; and
- the **sheet directory** (`BOUNDSHEET` records) — one per sheet, giving the
  sheet's name, visibility, and the **byte offset of that sheet's BOF** within
  the stream (`lbPlyPos`).

Each **worksheet** substream then holds that sheet's cell records. We match a
worksheet substream to its name by comparing the byte offset where its BOF
starts against the `lbPlyPos` values collected from the globals BOUNDSHEETs.

## 3. Record catalogue (exact byte layouts, all little-endian)

Every *cell* record starts with a **cell head**: `u16 row, u16 col, u16 xf`
(the `xf` is a style/format index we retain but do not interpret). Rows and
columns are **0-based** exactly as BIFF stores them.

### Globals records

**BOUNDSHEET** `0x0085` — one per sheet, in sheet order.
```
   u32 lbPlyPos   byte offset of this sheet's BOF within the Workbook stream
   u8  hsState    visibility: 0 visible, 1 hidden, 2 very hidden
   u8  dt         sheet type: 0 worksheet, 1 macro, 2 chart, 6 VB module
   ShortXLUnicodeString name:
      u8  cch      character count
      u8  grbit    bit0 fHighByte: 0 → 8-bit latin1 chars, 1 → 16-bit UTF-16LE
      (chars)      cch × (2 if fHighByte else 1) bytes
```

**SST** `0x00FC` — the shared string table.
```
   u32 cstTotal    total # of string references across the workbook
   u32 cstUnique   # of unique strings that follow
   cstUnique × XLUnicodeRichExtendedString
```
Each **XLUnicodeRichExtendedString** is read **in this exact field order**:
```
   u16 cch         character count
   u8  grbit       bit0 fHighByte (16-bit chars) | bit2 fExtSt (phonetic)
                                                 | bit3 fRichSt (rich runs)
   if fRichSt:  u16 cRun       (# formatting runs)
   if fExtSt:   u32 cbExtRst   (phonetic-data byte count)
   (char DATA)  cch × (2 if fHighByte else 1) bytes
   if fRichSt:  skip cRun × 4 bytes   (the FormatRun array)
   if fExtSt:   skip cbExtRst bytes   (the ExtRst phonetic block)
```

### The CONTINUE gotcha (the crux of BIFF8 SST parsing)

No BIFF record body may exceed **8224** bytes. A big SST therefore **spills
across following `CONTINUE` (`0x003C`) records**, and — the nasty part — a single
string's character data can be **split across a record boundary**. When a string
is continued, the **first byte of the CONTINUE body is a *fresh* grbit byte**
whose `fHighByte` flag applies to the *remainder* of that string. So the second
half of a string may switch between 8-bit and 16-bit encoding relative to its
first half.

We handle this with a **record-spanning byte reader** that presents the SST +
its trailing CONTINUE bodies as one logical byte source. When it hits the end of
the current record mid-string, it pulls the next CONTINUE, reads one flag byte,
and resumes decoding the rest of the string under that new flag. (Runs and
phonetic ext blocks are likewise skipped across the boundary.)

```
   record A body: [ ...string S first 5 of 8 chars (16-bit) ...| end of A ]
   record B (CONTINUE): [ grbit' | ...string S last 3 chars under grbit' ... ]
                          ^^^^^^^ NEW fHighByte flag for the remainder of S
```

### Worksheet cell records

| Type       | Code     | Body after cell head                                       |
| ---------- | -------- | ---------------------------------------------------------- |
| `LABELSST` | `0x00FD` | `u32 isst` → text is `SST[isst]`                           |
| `RK`       | `0x027E` | `u32 rk` → encoded number (see below)                     |
| `MULRK`    | `0x00BD` | `u16 row, u16 colFirst`, N×(`u16 xf, u32 rk`), `u16 colLast` |
| `NUMBER`   | `0x0203` | `f64` (8 bytes)                                            |
| `LABEL`    | `0x0204` | XLUnicodeString (u16 cch, u8 grbit, chars) — inline text  |
| `BOOLERR`  | `0x0205` | `u8 value, u8 fError` (0 → bool, 1 → error code)          |
| `BLANK`    | `0x0201` | (nothing — empty cell)                                     |
| `FORMULA`  | `0x0006` | 8-byte cached result, `u16 grbit`, `u32 chn`, rgce bytes  |

Note `MULRK` does **not** use the standard cell head (it has its own
`row, colFirst … colLast` framing) and emits **one numeric cell per column** in
`[colFirst, colLast]`.

### RK number decoding (the packed-number trick)

An `RK` value packs a number into 32 bits using the two low bits as flags:

```
   bit0  fx100  : if set, the decoded number is divided by 100
   bit1  fInt   : if set, value is a 30-bit signed integer; else a truncated f64
   bits 2..31   : the payload (30 bits)
```

Decoding:
- **`fInt` set** → take `rk >> 2` as a 30-bit value and **sign-extend bit 29**
  into a signed `i32`. That integer is the number.
- **`fInt` clear** → the payload is the **top 30 bits of an IEEE-754 `f64`**;
  the low 34 bits were zero. Reconstruct:
  `f64::from_bits( ((rk & 0xFFFF_FFFC) as u64) << 32 )`.
- Finally, if **`fx100`** is set, divide the result by 100.0.

Four combinations, all tested: int/no-div, int/÷100, float/no-div, float/÷100.

### FORMULA cached result decoding

A `FORMULA` record carries the **cached** result of the formula in its first 8
bytes so a reader need not evaluate anything. The encoding:
- If `result[6] == 0xFF && result[7] == 0xFF`, it is a **special** value keyed by
  `result[0]`:
  - `0` → **string**: the actual text is in a **following `STRING` (`0x0207`)**
    record (`u16 cch, u8 grbit, chars`).
  - `1` → **boolean**, value in `result[2]`.
  - `2` → **error**, code in `result[2]`.
  - `3` → **empty string**.
- Otherwise the 8 bytes are an IEEE-754 `f64` numeric result.

We expose a `Formula { cached }` cell carrying whatever cached value we decoded;
we do **not** decode the formula expression (the `rgce` token stream).

**Unknown record types** are skipped by their `size` (forward-compatibility).

## 4. Public API

```rust
pub fn open_xls(bytes: &[u8]) -> Result<Workbook, XlsError>;

pub struct Workbook { /* sheets in order */ }
impl Workbook {
    pub fn sheets(&self) -> &[Sheet];
    pub fn sheet(&self, name: &str) -> Option<&Sheet>;
}

pub struct Sheet { pub name: String, /* cells */ }
impl Sheet {
    pub fn cells(&self) -> &[Cell];
    pub fn cell(&self, row: u32, col: u32) -> Option<&Cell>;
}

pub struct Cell { pub row: u32, pub col: u32, pub value: CellValue }

pub enum CellValue {
    Number(f64),
    Text(String),
    Bool(bool),
    Error(u8),
    Formula { cached: Box<CellValue> },
    Blank,
}

pub enum XlsError {
    Cfb(cfb::CfbError),   // From<cfb::CfbError>
    NoWorkbookStream,     // container has neither "Workbook" nor "Book"
    Truncated,            // a record ran past the end of the stream
    TooLarge,             // a declared count/size exceeds the stream — refuse
    BadString,            // a string's bytes were not valid UTF-16
}
// XlsError: Display + std::error::Error
```

## 5. Reading untrusted bytes — the security model

A `.xls` arrives as an email attachment, so we assume the bytes are hostile:

- `#![forbid(unsafe_code)]`; **no** `unwrap`/`expect`/`panic!`, no panicking
  index/slice, no unchecked arithmetic on the parse path.
- Every record `size`, every `cch`, every `cstUnique`, every `MULRK` column span
  is **bounds-checked against the bytes actually remaining** before we read or
  allocate. A count that would exceed the stream is a clean `TooLarge` /
  `Truncated` error, never an allocation or a hang.
- A `BOUNDSHEET.lbPlyPos` pointing outside the stream simply fails to match any
  worksheet BOF — the sheet is emitted empty rather than panicking.
- A `MULRK` with `colLast < colFirst` yields **zero** cells (checked
  subtraction), never an underflow.
- The **CONTINUE chain** is walked with a hard cap and a running byte budget, so
  a crafted SST cannot make us loop forever or allocate unbounded memory.

## 6. Ground-truth fixture

`src/fixture.rs` (test-only) carries a real **5632-byte** `.xls` produced by
Python `xlwt`. Its `Workbook` stream decodes to exactly:

| Cell        | Record     | Value                                       |
| ----------- | ---------- | ------------------------------------------- |
| (0,0) "A1"  | `LABELSST` | Text `"Q1"`                                 |
| (0,1) "B1"  | `RK`       | Number `1000.0`                             |
| (1,0) "A2"  | `LABELSST` | Text `"Total"`                              |
| (1,1) "B2"  | `FORMULA`  | `SUM(B1:B1)`, cached as the special string  |

The single sheet is named **"Revenue"**. The end-to-end test opens this fixture
and asserts each cell. The fixture's SST is tiny and does **not** trigger
CONTINUE; a separate unit test constructs a **synthetic** SST whose string is
split across a CONTINUE record (with the encoding flipping 16-bit → 8-bit at the
boundary) to prove the record-spanning reader is correct.
