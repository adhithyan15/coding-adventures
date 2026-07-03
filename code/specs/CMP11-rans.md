# CMP11 — Asymmetric Numeral Systems (ANS / rANS)

> **Series position**: CMP00 LZ77 → CMP01 LZ78 → CMP02 LZSS → CMP03 LZW →
> CMP04 Huffman → CMP05 DEFLATE → CMP06 Brotli → CMP07 ZStd → CMP08 LZMA →
> CMP09 ZIP → CMP10 Range Coder → **CMP11 rANS (this spec)**

---

## 1. Overview

Asymmetric Numeral Systems (ANS) is a family of entropy coding algorithms
invented by Jarek Duda in 2009.  It achieves the same near-optimal compression
as arithmetic coding — every symbol costs exactly `log₂(M / freq)` bits, where
`M` is the total symbol count and `freq` is the frequency of that symbol — but
uses integer arithmetic instead of interval subdivision, making it 3–10× faster
in practice.

ANS has become the dominant entropy coder in modern compression formats:

| Format | ANS variant |
|--------|------------|
| JPEG XL | rANS (12-bit precision, M = 4096) |
| Zstandard (ZStd) | tANS (finite-state automaton table) |
| AV1 / AVIF | rANS (15-bit precision, M = 32768) |
| Apple LZFSE | tANS |
| LZHAM | rANS |

This spec covers **rANS** (range-ANS), the streaming variant.  The table-based
variant (tANS) is a space/time tradeoff on top of the same mathematical core;
once rANS is understood, tANS follows as a precomputed lookup optimisation.

### Why ANS over Huffman?

Huffman codes round every symbol's cost to an integer number of bits.  The
symbol `A` with true cost 0.4 bits gets a 1-bit code — a 150% overhead.
Arithmetic coding and ANS charge fractional bits, approaching Shannon entropy
exactly.  ANS does this without the division-heavy normalisation of an
arithmetic coder: the inner loop is just integer multiply, add, and compare.

### Why ANS over the VP8 range coder (CMP10)?

The VP8 range coder (CMP10) is a *binary* arithmetic coder: it codes one
bit at a time with a per-bit probability.  ANS codes an *alphabet* of symbols
(up to 256 or more) in one operation.  This makes ANS a better fit when you
already have a modelled probability distribution over symbols — as JPEG XL
does for its residuals, palette entries, and transform coefficients.

---

## 2. Conceptual Background

### 2.1 The state number

An ANS coder maintains a single non-negative integer `x` called the **state**.
The state encodes the entire history of symbols seen so far, compressed together
into one number.

Think of it like reading and writing a number in a strange base.  Each symbol
we push onto the coder "digits" the state by appending information about that
symbol.  Each symbol we pop "undigits" it, recovering the original state.

Because the coder processes symbols in one direction but the decoder must reverse
this, ANS has an inherent asymmetry:

```
Encoder:  A B C D ... → push symbols LIFO → bitstream
Decoder:  reads bitstream → pop symbols LIFO → D C B A ...
```

**The decoder reads the bitstream backwards relative to the encoder.**  In
practice the encoder buffers symbols and flushes them in reverse, or the
application reverses the output — either way the decoder reads from the end.

### 2.2 The frequency table

The probability model is a table of **frequencies** (integer counts) over an
alphabet of size `N`.  All frequencies must be positive and sum to exactly `M`,
a power of two chosen at design time (e.g. M = 4096 for 12-bit precision).

```
Symbol  freq[s]  cum[s]
  A       2048     0
  B       1024  2048
  C        512  3072
  D        256  3584
  E        256  3840
        ------
Total M = 4096
```

`cum[s]` is the cumulative frequency: `cum[s] = sum of freq[t] for t < s`.
The pair `(cum[s], freq[s])` defines the "slot range" `[cum[s], cum[s]+freq[s])`
that identifies symbol `s` within `[0, M)`.

### 2.3 State range (normalisation invariant)

The state `x` is kept in the range `[L, b·L)` where:

- `L` is the **lower bound** — a constant chosen so that `L` is a multiple of
  `M` and `L ≥ M` (e.g. `L = M = 4096` for JPEG XL's per-symbol precision, or
  `L = 1 << 23` for byte-streaming variants).
- `b` is the **radix** — the number of values that fit in one output unit.  For
  byte-streaming ANS, `b = 256` (one byte output per normalisation step).

The invariant `x ∈ [L, b·L)` means the state always fits in a bounded window.
When encoding pushes `x` out of range, bytes are flushed to restore it.  When
decoding pops a symbol and `x` falls below `L`, bytes are read to restore it.

---

## 3. rANS Encode

### 3.1 Encoding one symbol

Given symbol `s` with frequency `freq[s]` and cumulative `cum[s]`:

```
encode_symbol(x, s):
    // Step 1: Normalise — shrink x into the sub-range for symbol s
    x_max = ((L / M) * b) * freq[s]   // = (L * b / M) * freq[s]
    while x >= x_max:
        output_byte(x & 0xFF)          // flush low byte
        x >>= 8                        // shift right by b bits

    // Step 2: Push symbol — mix symbol identity into state
    x = (x / freq[s]) * M + cum[s] + (x % freq[s])
    return x
```

**Why this works (intuition):**  Dividing `x` by `freq[s]` removes `log₂(M/freq[s])`
bits of information (the symbol cost).  Multiplying by `M` and adding `cum[s]`
re-encodes those bits as symbol identity.  The remainder `x % freq[s]` carries
over the sub-symbol precision.

### 3.2 Worked encoding example

Alphabet from §2.2 (M=4096, L=4096, b=256).  Encode sequence `[A, B, C]`
(in reverse — encoder pushes last symbol first for correct decode order):

```
Initial state: x = L = 4096

Encode C (freq=512, cum=3072):
  x_max = (4096/4096 * 256) * 512 = 256 * 512 = 131072
  x=4096 < 131072 → no bytes flushed
  x = (4096/512)*4096 + 3072 + (4096%512)
     = 8*4096 + 3072 + 0
     = 32768 + 3072 = 35840

Encode B (freq=1024, cum=2048):
  x_max = (1 * 256) * 1024 = 262144
  x=35840 < 262144 → no bytes flushed
  x = (35840/1024)*4096 + 2048 + (35840%1024)
     = 35 * 4096 + 2048 + 0
     = 143360 + 2048 = 145408

Encode A (freq=2048, cum=0):
  x_max = (1 * 256) * 2048 = 524288
  x=145408 < 524288 → no bytes flushed
  x = (145408/2048)*4096 + 0 + (145408%2048)
     = 71 * 4096 + 0 + 0
     = 290816

Final state: x = 290816   (must be written to bitstream for decoder seed)
```

### 3.3 Flushing the final state

After all symbols are encoded, the final state `x` must be written to the
bitstream so the decoder can reconstruct it.  Conventionally this is stored as a
fixed-width integer (e.g. 32 bits) at the end of the compressed block, or the
minimum number of bytes needed to represent `x` (preceded by a length byte).

---

## 4. rANS Decode

### 4.1 Decoding one symbol

```
decode_symbol(x):
    // Step 1: Identify symbol from slot
    slot = x % M
    s = symbol_table[slot]               // lookup cum/freq from slot
    freq_s = freq[s]
    cum_s  = cum[s]

    // Step 2: Pop symbol — recover pre-encode state
    x = freq_s * (x / M) + slot - cum_s

    // Step 3: Renormalise — restore x to [L, b·L)
    while x < L:
        x = (x << 8) | read_byte()      // shift left, fill with next byte

    return (s, x)
```

**Why this is the inverse of encode:**  The encode step computed
`x' = (x/freq)*M + cum + (x%freq)`.  So `x'%M = cum + (x%freq)` is the slot.
`x = freq*(x'/M) + slot - cum = freq*(x'/M) + (x%freq)` recovers `x`.

### 4.2 Fast symbol lookup with a decode table

The naive `symbol_table` requires a linear scan over all cumulative frequencies.
In practice a precomputed lookup table of length `M` maps each slot to `(s, freq_s, cum_s)`:

```
// Precompute once per probability table:
decode_table: array[M] of (symbol: u8, freq: u32, cum: u32)

for s in 0..N:
    for slot in cum[s]..cum[s]+freq[s]:
        decode_table[slot] = (s, freq[s], cum[s])
```

This turns the decode loop into a single array lookup — O(1) per symbol.

### 4.3 Worked decoding example

Continuing from §3.2. Decoder reads state `x = 290816` from the bitstream.

```
Decode 1:
  slot = 290816 % 4096 = 0
  symbol_table[0] = A (cum=0, freq=2048)
  x = 2048 * (290816/4096) + 0 - 0 = 2048*71 = 145408
  x=145408 ≥ 4096 → no renorm needed
  → emits A

Decode 2:
  slot = 145408 % 4096 = 0
  symbol_table[0] = A? No — slot=0 → A (cum=0, freq=2048)
  Wait: 145408 % 4096 = 0 → A again?

  Actually: 145408 % 4096:
    145408 / 4096 = 35.5 → 35 * 4096 = 143360 → 145408 - 143360 = 2048
  slot = 2048 → B (cum=2048, freq=1024)
  x = 1024 * (145408/4096) + 2048 - 2048 = 1024*35 + 0 = 35840
  → emits B

Decode 3:
  slot = 35840 % 4096 = 35840 - 8*4096 = 35840 - 32768 = 3072
  symbol_table[3072] = C (cum=3072, freq=512)
  x = 512 * (35840/4096) + 3072 - 3072 = 512*8 = 4096
  x=4096 = L → no renorm needed
  → emits C

Decoded sequence: A B C ✓
```

---

## 5. Probability Tables and Normalisation

### 5.1 Building a frequency table from counts

Raw symbol counts from the data stream must be normalised to sum to exactly `M`:

```
normalise_frequencies(raw_counts, M):
    total = sum(raw_counts)
    freqs = [max(1, round(c * M / total)) for c in raw_counts]
    // Adjust for rounding errors so sum == M exactly:
    excess = sum(freqs) - M
    // Distribute excess by incrementing/decrementing the most frequent symbols
    ... (greedy adjustment, e.g. heapq)
    return freqs
```

The constraint `freq[s] ≥ 1` prevents symbols from being "lost" (a symbol with
freq=0 would be undecodable).  If the alphabet has symbols with zero count, they
should be removed from the coding alphabet and their absence signalled separately.

### 5.2 Transmitting the frequency table

The decoder needs the same frequency table as the encoder.  JPEG XL uses a
custom prefix-coded representation.  A simpler approach:

1. **Fixed precision**: store each `freq[s]` as a `log₂(M)`-bit integer.  For
   M=4096 this is 12 bits per symbol × 256 symbols = 384 bytes, acceptable for
   a block header.
2. **Delta coding**: store differences between successive cumulative counts.
3. **Run-length coding**: many symbols often have zero frequency; store the
   non-zero prefix only.

For JPEG XL's hybrid-ANS, the frequency table is itself coded with a recursive
Huffman prefix code — but a flat fixed-width encoding is correct and simpler for
a first implementation.

### 5.3 Choosing M (precision)

| M | log₂(M) | Typical use |
|---|---------|-------------|
| 256 | 8 | Small alphabets, very fast table |
| 4096 | 12 | JPEG XL, good balance |
| 32768 | 15 | AV1, higher precision |
| 1048576 | 20 | LZHAM, maximum precision |

Higher `M` allows closer approximation to true entropy but requires a larger
precomputed decode table (M entries) and a larger frequency table header.

---

## 6. Wire Format

rANS produces a **byte stream** read in **reverse** by the decoder.  The encoder
outputs bytes from the MSB end of the state and appends them to a buffer; the
decoder initialises from the buffer end and reads backwards.

```
┌───────────────────────────────────────────────────────┐
│ Block header:                                         │
│   num_symbols   (u32 LE) — number of decoded symbols  │
│   final_state   (u32 LE) — initial decoder state      │
│   freq_table    (N × log₂M bits) — symbol frequencies │
├───────────────────────────────────────────────────────┤
│ Compressed body (read backwards by decoder):          │
│   byte₀ byte₁ byte₂ … byteₖ                          │
│   ↑ decoder reads byteₖ first                         │
└───────────────────────────────────────────────────────┘
```

The decoder:
1. Reads `final_state` from the header and sets `x = final_state`.
2. Sets a read cursor to `byteₖ` (last byte of the body).
3. Calls `decode_symbol()` `num_symbols` times, reading bytes from `byteₖ`
   downward during renormalisation.

The encoder:
1. Encodes symbols in *reverse* order (last symbol first).
2. Each normalisation step appends a byte to an output buffer.
3. After all symbols, writes the final state and reverses the output buffer
   (or equivalently, reads the reversed buffer as the bitstream body).

---

## 7. API

```rust
/// Symbol probability model: frequencies summing to M (a power of 2).
///
/// `frequencies[s]` is the integer frequency of symbol `s`; all values
/// must be ≥ 1 and sum to exactly `M`.
pub struct AnsTable {
    pub frequencies: Vec<u32>,    // freq[s] for each symbol
    pub cumulatives: Vec<u32>,    // cum[s] = sum of freq[t] for t < s
    pub m: u32,                   // total = sum of all frequencies (power of 2)
    decode_table: Vec<DecodeEntry>, // length M — precomputed slot→symbol map
}

struct DecodeEntry {
    symbol: u16,
    freq:   u32,
    cum:    u32,
}

impl AnsTable {
    /// Build from a slice of positive frequencies that sum to M (a power of 2).
    pub fn new(frequencies: &[u32]) -> Result<Self, String>;
    /// Build from raw counts, normalising to precision `m_log2` bits.
    pub fn from_counts(counts: &[u32], m_log2: u8) -> Result<Self, String>;
}

/// Streaming rANS encoder.  Call `encode()` for each symbol in reverse order,
/// then `finish()` to obtain the compressed block.
pub struct RansEncoder {
    state: u64,
    output: Vec<u8>,   // bytes flushed during normalisation
    l: u64,            // lower bound = table.m (or larger)
}

impl RansEncoder {
    pub fn new(table: &AnsTable) -> Self;
    /// Encode one symbol (call in reverse symbol order).
    pub fn encode(&mut self, symbol: usize, table: &AnsTable);
    /// Flush final state and return the compressed block header + body.
    pub fn finish(self) -> Vec<u8>;
}

/// Streaming rANS decoder.  Initialise from a compressed block, then call
/// `decode()` for each symbol in forward order.
pub struct RansDecoder<'a> {
    state: u64,
    data:  &'a [u8],   // compressed body
    pos:   usize,      // read cursor (reads backwards from end)
    l: u64,
}

impl<'a> RansDecoder<'a> {
    /// Parse block header and initialise decoder state.
    pub fn new(compressed: &'a [u8], table: &AnsTable) -> Result<Self, String>;
    /// Decode one symbol, advancing the cursor.
    pub fn decode(&mut self, table: &AnsTable) -> Result<u16, String>;
    /// True when all symbols have been decoded.
    pub fn is_done(&self) -> bool;
}

/// Normalise raw counts to a valid AnsTable with M = 2^m_log2.
pub fn normalise_frequencies(counts: &[u32], m_log2: u8) -> Vec<u32>;
```

---

## 8. Round-Trip Property

For any sequence of symbols drawn from the alphabet and any valid `AnsTable`,
encoding then decoding must reproduce the original sequence exactly:

```rust
let symbols: Vec<usize> = vec![/* any valid symbol indices */];
let table = AnsTable::from_counts(&counts, 12).unwrap();

// Encode (symbols reversed)
let mut enc = RansEncoder::new(&table);
for &s in symbols.iter().rev() { enc.encode(s, &table); }
let compressed = enc.finish();

// Decode (forward)
let mut dec = RansDecoder::new(&compressed, &table).unwrap();
let mut decoded = Vec::new();
while !dec.is_done() { decoded.push(dec.decode(&table).unwrap() as usize); }

assert_eq!(symbols, decoded);
```

---

## 9. Error Cases

| Condition | Behaviour |
|-----------|-----------|
| `freq[s] == 0` for any symbol | `AnsTable::new` returns `Err` |
| Frequencies do not sum to M | `AnsTable::new` returns `Err` |
| M is not a power of two | `AnsTable::new` returns `Err` |
| Compressed block too short | `RansDecoder::new` returns `Err` |
| `decode()` called after `is_done()` | Returns `Err("exhausted")` |
| State underflows during decode | Returns `Err("corrupt")` |
| Symbol index ≥ alphabet size in `encode` | Panic in debug, UB in release (caller must validate) |

---

## 10. Test Vectors

### 10.1 Uniform distribution, 4 symbols

```
Alphabet: {A, B, C, D}
Counts:   [1, 1, 1, 1]   (M=4, log₂M=2)
Frequencies after normalise: [1, 1, 1, 1]
Cumulatives: [0, 1, 2, 3]

Input:  A B C D A
Encode reversed: A D C B A

L = 4, b = 256
x = 4

encode A (freq=1, cum=0): x_max=1*256*1=256; no flush; x=(4/1)*4+0+0=16
encode D (freq=1, cum=3): x_max=256; no flush; x=(16/1)*4+3+0=67
encode C (freq=1, cum=2): x_max=256; no flush; x=(67/1)*4+2+0=270
encode B (freq=1, cum=1):
  x_max=256; x=270≥256 → flush byte 270%256=14; x=270/256=1
  x=(1/1)*4+1+0=5
encode A (freq=1, cum=0): x_max=256; x=5<256; x=(5/1)*4+0=20

Final state: x=20; 1 byte flushed: [14]
Compressed block: [final_state=20 (4B LE)] [body=14]

Decode from x=20, body=[14] (read backward from end):
  decode: slot=20%4=0→A; x=1*(20/4)+0-0=5; renorm: x<4? no (x=5); → A
  decode: slot=5%4=1→B;  x=1*(5/4)+1-1=1; renorm: x<4→ x=1*256|14=270; → B
  decode: slot=270%4=2→C; x=1*(270/4)+2-2=67; x≥4→ok; → C
  decode: slot=67%4=3→D; x=1*(67/4)+3-3=16; → D
  decode: slot=16%4=0→A; x=1*(16/4)+0=4; → A

Decoded: A B C D A ✓
```

### 10.2 Skewed distribution

```
Alphabet: {0, 1}, M=4
Frequencies: freq[0]=3, freq[1]=1
Cumulatives: [0, 3]

Input: 0 0 0 0 1
Encode reversed: 1 0 0 0 0
...
(exact byte values are implementation-specific based on L choice;
 round-trip correctness is the invariant)
```

### 10.3 Single-symbol degenerate case

```
Alphabet: {X}, M=1, freq[0]=1
Any input sequence encodes to 0 bytes (state = L constant throughout).
Decoder reconstructs via the known length stored separately.
```

---

## 11. Teaching Notes

### 11.1 ANS vs arithmetic coding

Both achieve Shannon entropy.  The key difference is direction:

- **Arithmetic coder**: processes symbols in *forward* order; encoder and decoder
  run in sync.  State is a rational number (interval).  Normalisation requires
  integer division — slower on modern CPUs.

- **rANS**: processes symbols in *reverse* for encoding; decoder reads forward.
  State is a single integer.  Normalisation uses shift + bitwise ops — faster.

The "asymmetry" in ANS refers to this direction asymmetry: the system is not
symmetric with respect to time ordering.

### 11.2 tANS — the table-based speedup

rANS recomputes `(x/freq)*M + cum + (x%freq)` per symbol.  tANS precomputes a
finite-state automaton with `L*b/M` states and one next-state table per symbol,
turning the encode/decode into a single table lookup.  ZStd and AV1 use tANS
for their entropy stage.  The state space grows with M, so tANS is memory-bound
at high precision; rANS is arithmetic-bound.  For JPEG XL's use case (M=4096,
up to 256 symbols) tANS tables would be 1 MB — too large — so rANS is used.

### 11.3 Connection to Huffman (CMP04)

Huffman codes are a special case of ANS where M is a power of two and every
symbol frequency is also a power of two.  Under those constraints the ANS
encode reduces to the same code-word table a Huffman coder would emit.
The cost is rounded to the nearest bit; ANS's fractional-bit precision vanishes.

### 11.4 Connection to the VP8 range coder (CMP10)

The VP8 boolean range coder (CMP10) is a *binary* arithmetic coder with
a fixed probability representation.  It processes one bit at a time with
a per-bit probability `p ∈ [0, 255]`.  rANS codes entire symbols (up to 256)
in one pass and uses a precomputed frequency table.  rANS is faster for
well-modelled alphabets; the VP8 range coder is simpler and better for
skewed binary streams with adaptive probabilities.

---

## 12. Crate Layout

```
rans/
  Cargo.toml       (no external deps — zero dependency crate)
  BUILD
  README.md
  CHANGELOG.md
  src/
    lib.rs         (pub use, module re-exports, VERSION, crate doc)
    table.rs       (AnsTable, DecodeEntry, normalise_frequencies)
    encoder.rs     (RansEncoder)
    decoder.rs     (RansDecoder)
```

**Cargo.toml skeleton:**
```toml
[package]
name        = "rans"
version     = "0.1.0"
edition     = "2021"
description = "rANS (range Asymmetric Numeral Systems) entropy coder"
license     = "MIT"

[dependencies]  # none
```

---

## 13. Relationship to IC06 (JPEG XL)

JPEG XL uses rANS as its primary entropy coder for:

- **Modular mode** (lossless): symbol values from the MA (meta-adaptive) tree
  residuals are entropy coded with rANS, precision M = 4096.
- **VarDCT mode** (lossy): DCT coefficient residuals and quantisation maps
  are entropy coded with rANS.

The JPEG XL spec calls its entropy coder "ANS" and uses a hybrid: if a
distribution is highly peaked (one symbol dominates), it falls back to a simple
prefix code (Huffman); otherwise rANS is used.  The choice is signalled per
block in the bitstream.

The `rans` crate (this spec) is a direct dependency of the `image-codec-jxl`
crate (IC06).
