# lzw — LZW Compression (CMP03)

LZW (Lempel-Ziv-Welch, 1984) refines LZ78 by pre-seeding the dictionary with all 256
single-byte sequences. Because every byte already has a code (0–255), the encoder emits
only dictionary codes — no raw literals. This enables variable-width bit-packing (9–16
bits per code), the scheme used in GIF, TIFF, and Unix `compress`.

## Where it fits

```
CMP00 LZ77 → CMP01 LZ78 → CMP02 LZSS → CMP03 LZW (this crate) → CMP04 Huffman → CMP05 DEFLATE
```

A leaf crate with no dependencies. Its contract, the findings made while converting
it to Warrant-derived evidence, and what is and is not verified are in
[`code/specs/LZ01-lzw-contract.md`](../../../specs/LZ01-lzw-contract.md); the executable
specification is [`code/specs/lzw.warrant`](../../../specs/lzw.warrant).

## Usage

```rust
use lzw::{compress, decompress};

let data = b"hello hello hello";
let compressed = compress(data);
assert_eq!(decompress(&compressed).unwrap(), data);
```

`decompress` takes untrusted input and returns `Err(String)` for anything that is not a
stream — a missing header, a missing CLEAR code at the start, an invalid code — and it
never panics.

```rust
use lzw::decompress;

assert!(decompress(b"").is_err());
assert!(decompress(&[0, 0, 0, 1, 0xFF, 0, 0]).is_err()); // first code is not CLEAR
```

## Hostile input: the 16 MiB output cap

LZW has no upper bound on how much a short stream can expand to: the code for "the
previous string plus its own first byte" makes dictionary entries grow by one byte per
code, so a 15 KB stream is 40 MB and a 128 KB one two gigabytes. The header's
`original_length` is attacker-controlled and cannot be trusted to stop that (backlog
SEC-002, CWE-409).

`decompress` therefore refuses to deliver more than [`MAX_OUTPUT_LEN`] = 16 MiB and
returns `Err` instead, before it writes the entry that would pass the limit:

```rust
use lzw::{compress, decompress, MAX_OUTPUT_LEN};

assert_eq!(MAX_OUTPUT_LEN, 16 * 1024 * 1024);
// Anything that round-trips below the cap is unaffected, however redundant:
let zeros = vec![0u8; 1 << 20];
assert_eq!(decompress(&compress(&zeros)).unwrap(), zeros);
```

* Only **delivered** bytes count: a header that declares fewer bytes than the stream
  could produce stops the decoder early and succeeds.
* The header is an **upper bound**, never an allocation size: nothing is sized from it, and
  a stream that ends earlier (or without a STOP code) is not an error. That leniency is a
  recorded limitation (LZ01), not a feature to rely on.
* `compress` has no error channel, so an input longer than the cap produces a stream that
  `decompress` rejects (fail closed). Its header saturates at `u32::MAX` rather than
  wrapping, so a 4 GiB + 10 byte input cannot masquerade as a ten-byte one.
* The cap is a constant. A `max_output` parameter is the better API; it is a breaking change
  across every port (backlog LP-003) and is left to the owner.

Both directions are linear: the dictionary of `decompress` is at most 65,536 eight-byte
links (512 KiB) however the stream is built, and `compress` does one dictionary lookup per
input byte.

## Wire Format (CMP03)

```
Bytes 0–3:  original_length (big-endian u32)
Bytes 4+:   LSB-first bit-packed variable-width codes (9–16 bits each)
            CLEAR(256) ... data codes ... STOP(257)
```

## Verification

Every claim in `lzw.warrant` is compiled into `tests/generated/warrant_evidence.rs` and run
against the real `compress` and `decompress` (VER04h): round trip, every input of length
≤ 3 for `decompress`, hostile streams, the output and heap bounds, a real decompression
bomb built from a 279-byte unit, and sabotage controls. `evidence/` holds the manifests
those runs wrote, `effect_certificate.json` the capability certificate and
`assurance_report.json` the report derived from them (`ca-assurance-report --check`
re-derives and cross-checks it, including CWE-409). `tests/runtime_capability.rs` runs both
functions, with hostile input, inside a seccomp/Landlock jail derived from
`required_capabilities.json` (no capabilities), watched by an independent `strace`
(VER08b).

## Development

```bash
cargo test -p lzw --lib --doc                      # unit and doc tests
cargo test -p lzw --test decompression_cap --test decoder_memory --test encoder_cost \
    --test decoder_equivalence --test hostile_headers
RCG_STRICT=1 cargo test -p lzw --test runtime_capability     # the jail (Linux)
cargo test -p lzw --test warrant_evidence          # the generated evidence (minutes)
```

The `BUILD_linux` file runs all of the above plus the sabotage lines.
