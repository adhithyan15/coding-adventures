# rans

**Range Asymmetric Numeral Systems (rANS)** entropy coder — zero dependencies.

rANS is the modern entropy engine used by JPEG XL, Zstandard (FSE mode), AV1, and
others. It achieves near-Shannon compression in O(1) time per symbol with extremely
simple encode and decode loops.

## Position in the stack

```
image-codec-jxl  ← consumes rans for Modular (lossless) coefficient coding
rans             ← this crate (no deps)
```

This crate implements the CMP11 specification (`code/specs/CMP11-rans.md`).

## Quick start

```rust
use rans::{AnsTable, RansEncoder, RansDecoder};

// 1) Define your symbol alphabet and frequencies.
//    Here: symbol 0 ("A") appears 3×, symbol 1 ("B") appears 1×.
let table = AnsTable::new(&[3, 1]).unwrap();

// 2) Encode a sequence.  Symbols must be pushed in *reverse* order.
let symbols = [0u8, 0, 1, 0]; // logical order: A A B A
let mut enc = RansEncoder::new(&table);
for &s in symbols.iter().rev() {
    enc.put(s);
}
let compressed = enc.finish();

// 3) Decode in forward order.
let mut dec = RansDecoder::new(&table, &compressed).unwrap();
for &expected in &symbols {
    assert_eq!(dec.get(), expected);
}
```

## API

| Type | Role |
|------|------|
| `AnsTable` | Precomputed frequency table for one alphabet |
| `RansEncoder` | Streaming encoder — push symbols in reverse, call `finish()` |
| `RansDecoder` | Streaming decoder — call `get()` for each symbol in forward order |

### `AnsTable::new(counts: &[u32]) -> Result<AnsTable, String>`

Accepts raw (unnormalized) symbol counts. The table is automatically scaled to
`M = 2^k` (the nearest power of two ≥ the number of symbols). Every symbol must
have a non-zero count; the maximum alphabet size is 256.

### `RansEncoder`

```rust
let mut enc = RansEncoder::new(&table);
enc.put(symbol);   // repeat for each symbol, in *reverse* logical order
let bytes = enc.finish();
```

### `RansDecoder`

```rust
let mut dec = RansDecoder::new(&table, &bytes)?;
let sym = dec.get();   // repeat for each symbol in forward order
```

## How it works

rANS represents the entire encoded message as a single large integer `x`.

**Encoding** symbol `s` with frequency `f` (out of `M` total):

1. Renormalize: emit low bytes until `x < f * 256`
2. Step: `x = (x / f) * M + cumfreq[s] + (x % f)`

**Decoding** from state `x`:

1. Lookup: `slot = x % M` → symbol, freq, cumfreq from precomputed table (O(1))
2. Step: `x = freq * (x / M) + (x % M) - cumfreq`
3. Renormalize: read bytes until `x ≥ M`

Because encoding emits bytes in reverse and the decoder reads them forward,
symbols must be pushed in reverse order when encoding.

## Wire format

The byte stream produced by `RansEncoder::finish()`:

```
[ initial_x: 4 bytes big-endian ][ renorm_bytes: variable ]
```

The decoder reads the 4-byte initial state, then consumes renorm bytes left-to-right
as it decodes each symbol.

## Testing

```
cargo test -p rans
```

25 unit tests + 4 doc-tests covering:
- Round-trips for 1, 2, 4, 256-symbol alphabets
- Skewed distributions, long sequences, single-symbol calls
- Table property invariants (M is power of two, frequencies sum to M, decode table coverage)
- Compression ratio sanity check (highly skewed → small output)
- Determinism regression test
- Error cases (empty alphabet, too-short data, etc.)
