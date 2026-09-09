# MA04 — QR Code Decoder: `ModuleGrid` → Data

## Status

Normative spec for a new `qr-decoder` crate, closing the "decode a QR
code's *data*" half of the deferral `VLT-PM49-cli-external-import.md`
§9 originally recorded for `vault-pm import otpauth-qr FILE`. Named
`MA04` per `MA02-reed-solomon.md`'s own roadmap table, which already
reserved this slot: "MA04 | qr-decoder | Inverse: recognise and decode
a QR matrix back to a string."

**Scope split, decided explicitly by the user rather than defaulted.**
"Recognise" in that roadmap line covers two genuinely different
problems: (1) given a QR code's module grid — its black/white bit
pattern, already located and sampled — decode it back into the
original bytes; (2) given an arbitrary raster image that merely
*contains* a QR code somewhere in it, find it (binarize, scan for
finder patterns, cluster candidates, determine module geometry,
sample), producing exactly the `ModuleGrid` (1) consumes. This spec is
**only (1)**. Locating a QR code inside a larger image — pixel
scanning, finder-pattern run-length detection, and eventually (per the
user's stated intent) full computer-vision robustness for rotated,
skewed, or photographed codes — is real, separable follow-up work,
tracked in its own issue rather than folded into this one. Concretely,
this means: at the end of this spec, `vault-pm import otpauth-qr FILE`
**still does not work end to end** on a real image file — what ships
here is the decode engine the image-locating work will eventually feed
into, verified by round-tripping against this workspace's own QR
*encoder* (`qr-code`) rather than against a real photograph or
screenshot.

No external (crates.io) dependency is introduced anywhere in this
spec — every algorithmic piece is either reused from an existing
workspace crate or added to one, matching this repo's stated
zero-external-dependency policy for this class of package (§9's own
`rqrr`/`png` discussion is moot under this scope split, since neither
image decoding nor image analysis is part of this slice).

## 1. Reuse precedent: what already exists, verified from source

Before any new code, the existing `qr-code` (encoder) and
`reed-solomon` crates were read in full to establish exactly what a
decoder can reuse unchanged, what needs extending, and what has no
prior art:

- **`gf256`** — GF(256) field arithmetic (`0x11D` primitive
  polynomial). Used identically by encode and decode. No changes.
- **`reed-solomon`** — already implements a complete RS decoder
  (syndromes → Berlekamp-Massey → Chien search → Forney correction),
  and its `error_locator` function's own doc comment says it is
  "exposed... for higher-level tools (e.g. QR decoders)" — this crate
  was written anticipating this exact consumer. But `build_generator`
  and `syndromes` hardcode the **b=1** root convention (`for i in
  1..=n_check`, roots α¹…α^n_check) documented as this package's own
  convention in `MA02-reed-solomon.md`. QR's actual RS codes use the
  **b=0** convention (`qr-code`'s own `build_generator`: `for i in
  0..n`, roots α⁰…α^(n-1)) — a real, load-bearing mismatch, not a
  cosmetic one: calling `reed_solomon::decode` directly on real QR
  codewords would silently compute wrong syndromes. §2 below extends
  `reed-solomon` with a base-parameterized variant rather than
  reimplementing Berlekamp-Massey/Chien/Forney a second time — those
  three functions operate only on already-computed syndromes and the
  error-locator polynomial (never on a raw root index), so they are
  b-agnostic and need no changes; only `build_generator`/`syndromes`
  need a `b` parameter threaded through.
- **`qr-code`** — every ISO 18004 structural table this decoder needs
  already exists and is correct: `ECC_CODEWORDS_PER_BLOCK`,
  `NUM_BLOCKS` (indexed `[ecc_idx][version]`), `ALIGNMENT_POSITIONS`
  (indexed `version - 1`), plus the geometry formulas
  `symbol_size`/`num_raw_data_modules`/`num_data_codewords`/
  `num_remainder_bits`, the format-info/version-info BCH generator
  constants (`0x537`, `0x1F25`, mask `0x5412`), the 8 `mask_condition`
  formulas, and — critically — `compute_blocks`'s block-size
  arithmetic (`short_len = total_data / total_blocks`, `num_long =
  total_data % total_blocks`, `g1_count = total_blocks - num_long`) is
  **derivable purely from `(version, ecc)`**, not from the message
  content, so a decoder can reconstruct identical block boundaries
  without any new table. All of this exists today as **private**
  functions/statics inside `qr-code` — copy-safe as constants, but the
  *traversal logic* (`place_bits`'s zigzag order, `place_finder`/
  `place_alignment`/`place_timing`/`reserve_format_info`/
  `reserve_version_info`'s reserved-module computation, `apply_mask`)
  is write-shaped and needs either duplicating or exposing. §3 chooses
  exposing: `qr-code` gains new `pub` functions, used by both its own
  existing `encode()` (refactored to call them, eliminating the
  now-doubled logic) and the new `qr-decoder` crate. `qr-code` has
  **zero existing dependents anywhere in the Rust workspace**
  (confirmed by grep), so this is a safe, purely-additive extension of
  an otherwise-unconsumed crate's public surface.
- **No prior art for image-level QR location** anywhere in this repo,
  in any language port, confirmed by exhaustive grep (finder-pattern
  detection, run-length scanning, binarization beyond a fixed
  threshold) — consistent with this spec's explicit scope cut in
  favor of a later slice.

## 2. `reed-solomon`: base-parameterized decode

Add, without removing or changing any existing public function
(backward compatible, though nothing in the workspace depends on this
crate today so compatibility is a courtesy, not a hard constraint):

```rust
pub fn build_generator_with_base(n_check: usize, b: u32) -> Result<Vec<u8>, RSError>;
pub fn syndromes_with_base(received: &[u8], n_check: usize, b: u32) -> Vec<u8>;
pub fn decode_with_base(received: &[u8], n_check: usize, b: u32) -> Result<Vec<u8>, RSError>;
```

Each existing `build_generator`/`syndromes`/`decode` becomes a thin
wrapper calling the `_with_base` variant with `b = 1`, preserving this
package's documented b=1 convention as the default. `berlekamp_massey`,
`chien_search`, and `forney` (already private, already b-agnostic —
confirmed by reading every `power(2, …)` call site in the crate) are
called unchanged from `decode_with_base`; only the two root-generating
loops (`for i in 1..=n_check` in `build_generator`, `(1..=n_check)` in
`syndromes`) change to `for i in b..(b + n_check as u32)` (or the
equivalent range starting at `b`) in the `_with_base` versions.

`MA02-reed-solomon.md` gets a short addendum documenting the `b`
parameter and cross-referencing this spec, rather than restating the
whole decode pipeline — the algorithm is unchanged, only the root
offset is now a parameter instead of a hardcoded 1.

## 3. `qr-code`: new `pub` surface (purely additive)

All new, no existing signature changes:

```rust
// Geometry — promote existing private fns to pub, unchanged bodies.
pub fn symbol_size(version: usize) -> usize;
pub fn num_raw_data_modules(version: usize) -> usize;
pub fn num_data_codewords(version: usize, ecc: EccLevel) -> usize;
pub fn num_remainder_bits(version: usize) -> usize;
pub fn ecc_codewords_per_block(ecc: EccLevel, version: usize) -> usize;
pub fn num_blocks(ecc: EccLevel, version: usize) -> usize;

// Reserved-module computation, extracted from encode()'s grid-building
// so encode and decode share one implementation.
pub fn reserved_modules(version: usize) -> Vec<Vec<bool>>;

// Zigzag data/ECC module traversal order, extracted from `place_bits`
// so encode (writing) and decode (reading) walk the identical sequence
// by construction, not by two independently-written loops that must
// happen to agree.
pub fn data_module_order(version: usize) -> Vec<(usize, usize)>;

// Masking — promote existing private fn to pub, unchanged body.
// Self-inverse (XOR), so decode calls this the same way encode does.
pub fn mask_condition(mask: u32, r: usize, c: usize) -> bool;

// Format/version info, decode direction — new functions mirroring the
// existing (soon-renamed-from-test-only) `format_info_valid` helper
// and a new version-info equivalent, both promoted/added as real pub
// API rather than staying test-only.
pub fn read_format_info(bits15: u32) -> Option<(EccLevel, u32)>;   // (ecc, mask), BCH-validated
pub fn read_version_info(bits18: u32) -> Option<usize>;            // version, BCH-validated

// Inverse of the existing private `ecc_indicator`.
pub fn ecc_from_indicator(bits: u16) -> Option<EccLevel>;
```

`encode()` is refactored to call `reserved_modules`/`data_module_order`
instead of its own inline reservation/traversal logic — this is a
behavior-preserving refactor (the bodies move, they do not change),
verified by requiring `qr-code`'s full existing test suite to pass
unchanged (§8, gate 1). `format_info_valid` (currently a private
`#[cfg(test)]` helper) is replaced by the new public
`read_format_info`, and the crate's own tests are updated to call the
public version instead of duplicating it.

## 4. New crate: `qr-decoder`

`code/packages/rust/qr-decoder`, depending on `qr-code` (tables/
geometry/traversal), `reed-solomon` (base-parameterized decode),
`barcode-2d` (the `ModuleGrid` type, already re-exported by `qr-code`
but depended on directly since it's the origin), `gf256` (transitively
sufficient via `reed-solomon`; not depended on directly unless the
implementation finds a direct need).

### 4.1 Public API

```rust
pub fn decode(grid: &ModuleGrid) -> Result<Vec<u8>, QrDecodeError>;
```

One function. Input is a `ModuleGrid` exactly as `qr-code::encode`
produces it — `rows == cols == symbol_size(version)` for some version
1–40, no quiet zone, mask already applied, format/version info already
written (i.e. what a real scanner would see, module for module — this
crate does not accept or produce anything with quiet-zone padding
baked in; that is the image-locating layer's job in the deferred
follow-up work). `QrDecodeError` variants cover every failure mode
named in §4.2–§4.5 below, each distinct enough to unit-test
individually (not one catch-all "decode failed").

### 4.2 Pipeline

```text
ModuleGrid
    │
    ▼
[1] Validate size: rows == cols == 4V + 17 for some V in 1..=40
    │
    ▼
[2] reserved = qr_code::reserved_modules(V)
    │
    ▼
[3] Read format info: try Copy 1 positions first, BCH-validate via
    qr_code::read_format_info; on failure, try Copy 2. Both fail → Err.
    → (ecc_level, mask)
    │
    ▼
[4] If V >= 7: read version info (Copy 1, then Copy 2 on failure) via
    qr_code::read_version_info; must equal the size-derived V from
    step 1, or Err (a real, if redundant, consistency check ISO
    provides for exactly this purpose).
    │
    ▼
[5] Unmask: for every non-reserved (r,c),
    bit = grid.modules[r][c] != qr_code::mask_condition(mask, r, c)
    (the same XOR `apply_mask` used to write it — self-inverse).
    │
    ▼
[6] Walk qr_code::data_module_order(V) in order, reading one unmasked
    bit per position, for exactly num_raw_data_modules(V) positions.
    Split into 8-bit codewords (MSB-first, matching how `place_bits`
    packed them) — the first `num_raw_data_modules(V) / 8` bytes are
    the interleaved codeword stream; the trailing num_remainder_bits(V)
    bits are padding, discarded.
    │
    ▼
[7] De-interleave: reconstruct per-block (data_len, ecc_len) sizes
    identically to qr-code's own `compute_blocks` arithmetic
    (short_len = total_data/total_blocks, num_long = total_data %
    total_blocks, g1_count = total_blocks - num_long — a pure function
    of (V, ecc_level), needing no new table), then invert
    `interleave_blocks`'s round-robin write order to recover each
    block's own (data ++ ecc) codeword sequence.
    │
    ▼
[8] Per block: reed_solomon::decode_with_base(block, ecc_len, b=0).
    Any block's TooManyErrors -> Err (identifies the block index, not
    partial decoded content, in the error).
    │
    ▼
[9] Concatenate corrected data codewords across blocks, in block
    order (not interleaved) -> the original data-codeword bit stream.
    │
    ▼
[10] Parse ONE data segment (this crate's encoder never emits more
     than one -- see §4.4): 4-bit mode indicator, mode-and-version-
     dependent character-count width, mode-specific payload decode
     (numeric / alphanumeric / byte -- §4.4), optional terminator.
    │
    ▼
Vec<u8>
```

### 4.3 Format/version info: Copy-1-then-Copy-2, not full error correction

Both format and version info are stored as two independent BCH-coded
copies specifically so a decoder can fall back from one to the other.
This spec implements exactly that fallback — recompute-and-compare
BCH validation on each copy in turn — and explicitly does **not**
implement full format-info error correction (nearest-valid-codeword
search by Hamming distance among the 32 valid post-mask 15-bit words,
which can correct up to 3 bit errors even when *both* copies are
independently damaged). That is real, separable robustness work
appropriate to the same later slice that adds image-level tolerance to
noise/damage — this spec's own "someone already handed you a clean
`ModuleGrid`" framing (§ Status) makes the simpler two-copy fallback
sufficient for what this slice actually needs to prove: that the
inversion of every ISO 18004 write-direction algorithm in `qr-code` is
correct.

### 4.4 Data segment decode: exactly the modes `qr-code` can produce

`qr-code`'s own encoder (per `qr-code.md`) implements only numeric,
alphanumeric, and byte mode as data segments, and never emits more
than one segment (no mixed-mode packing) or an ECI header. A decoder
only needs to invert what the paired encoder can produce, so:

- **Mode indicator** `0001`/`0010`/`0100` → numeric/alphanumeric/byte,
  read and dispatched. `1000` (Kanji) or `0111` (ECI) →
  `QrDecodeError::UnsupportedMode` — named and distinct from a
  structural decode failure, not silently misread. `0000` alone (no
  data) is a valid empty-message terminator.
- **Character count indicator** width from the existing table
  (mode × version tier: 1–9 / 10–26 / 27–40), read immediately after
  the mode indicator.
- **Numeric mode**: inverse of the encoder's grouping — read 10-bit
  groups for each full triple of digits, a final 7-bit group for a
  remaining pair, or a final 4-bit group for a single remaining digit,
  driven by the character count (not by scanning for a terminator
  mid-stream).
- **Alphanumeric mode**: inverse of `first_idx * 45 + second_idx`
  11-bit pairs, with a final 6-bit single-character group when the
  count is odd, mapped back through the same 45-character table
  `qr-code.md` §"Alphanumeric mode" already documents.
- **Byte mode**: read `count` literal 8-bit bytes. This is the mode
  the actual downstream use case exercises — an `otpauth://totp/...`
  URI contains lowercase letters and `?`/`=`/`&`, none of which are in
  the 45-character alphanumeric set, so `qr-code`'s own mode-selection
  heuristic always encodes it in byte mode.
- After the one segment: an optional 4-bit (or shorter, if at exact
  capacity) `0000` terminator, then 0xEC/0x11 alternating padding to
  the codeword boundary — read the terminator if present but do not
  attempt to parse anything after it as a second segment (single-
  segment scope cut, matching the encoder's own single-segment
  behavior, stated explicitly rather than silently assumed).

### 4.5 Errors

```rust
pub enum QrDecodeError {
    InvalidSize,              // rows != cols, or not 4V+17 for any V in 1..=40
    FormatInfoUnreadable,      // both format-info copies fail BCH validation
    VersionInfoUnreadable,     // V >= 7 and both version-info copies fail BCH,
                               // or the recovered version disagrees with size
    UnrecoverableBlock(usize), // RS decode exhausted for block index N
    UnsupportedMode(u8),       // Kanji or ECI mode indicator encountered
    Truncated,                 // character count claims more data than remains
}
```

No variant carries decoded content — every failure is structural,
matching this crate's general-purpose-library posture (it has no
knowledge that a future caller might feed it secret-shaped data such
as an otpauth TOTP seed; keeping errors content-free is good practice
regardless, and costs nothing here).

## 5. Language scope: Rust only

`qr-code` and `reed-solomon` both exist in most (though, per an
audit during this spec's own research, not literally all — the
existing `qr-code.md`'s "9-language" package matrix has already
drifted from the real tree) sibling-language directories in this
workspace. This decoder is **Rust-only**, matching the precedent
already set by the entire `vault-pm-*` family (14+ crates, zero
sibling-language directories anywhere) — the only real consumer driving
this work, `vault-pm-cli`, is Rust, and no tooling in this repo
enforces or checks cross-language package parity. A multi-language
port is legitimate future work if ever wanted, tracked separately, not
assumed here.

## 6. Explicitly deferred (separate issue, not this spec)

- **Locating a QR code within an arbitrary raster image** — pixel
  binarization beyond a fixed threshold, finder-pattern run-length
  scanning (the 1:1:3:1:1 ratio), candidate clustering, module-grid
  sampling from recovered finder centers, and (per the user's stated
  eventual goal) full perspective/rotation/lighting robustness for
  photographed rather than screenshotted codes. This is what turns
  `qr_decoder::decode(&ModuleGrid)` into an actual `import otpauth-qr
  FILE` CLI ceremony — genuinely separate, larger, and open-ended work,
  tracked in its own issue (#14456) and reframed as a general
  computer-vision investment (not a QR-specific build) in
  `VIS00-vision-roadmap.md`.
- **Full format-info/version-info error correction** (nearest-valid-
  codeword search) — §4.3.
- **Kanji mode, ECI, structured append, mixed-mode segments** — none
  of these exist in the paired encoder either; adding decode support
  for them without encoder support would be asymmetric scope with no
  test-fixture story (this crate's whole test strategy is round-
  tripping against `qr-code::encode`, §7).
- **Micro QR, rMQR** — `qr-code.md` already lists these as encoder
  future work; the decoder tracks whatever the encoder eventually
  supports, not ahead of it.

## 7. Test strategy

Because `qr-code::encode` already exists and is known-correct (its own
test suite passes today), this crate's primary verification is
round-tripping against it — no external QR-generation tool or
image-fixture pipeline is needed, unlike the KDBX4 decoder (VLT-PM49
Amendment 2), which had no in-workspace encoder to round-trip against
for its container format:

1. **Exhaustive round-trip**: for a representative input at every
   version 1–40 × every `EccLevel` (L/M/Q/H), `qr_code::encode(input,
   ecc)` → `qr_decoder::decode(&grid)` → assert equals the original
   input bytes. Inputs chosen to exercise all three modes: a numeric
   string, an alphanumeric string, and a byte-mode string (including
   one shaped like a real `otpauth://totp/...` URI, the actual driving
   use case).
2. **Corruption-and-recovery**: encode a message, flip up to `t`
   codeword bytes within a single RS block (t = ecc_len/2, per
   MA02's own error-correction-capacity math), decode, assert the
   original message is still recovered — proving `decode_with_base`'s
   b=0 wiring is actually correct, not merely "compiles and round-
   trips on undamaged input."
3. **Beyond-capacity corruption**: corrupt more than `t` bytes in one
   block, assert `QrDecodeError::UnrecoverableBlock` — not a silently
   wrong result.
4. **Format-info Copy-2 fallback**: corrupt Copy 1's format-info
   modules directly in a `ModuleGrid` (bypassing the encoder), assert
   decode still succeeds via Copy 2.
5. **Version-info mismatch**: for a version ≥ 7 grid, corrupt both
   version-info copies (or write a value disagreeing with the grid
   size), assert `QrDecodeError::VersionInfoUnreadable`.
6. **`InvalidSize`**: non-square grid, and a square grid whose size is
   not `4V+17` for any integer `V` in 1..=40.
7. **`qr-code`'s own existing test suite must pass unchanged** after
   the §3 refactor — this is the regression gate proving
   `reserved_modules`/`data_module_order` extraction didn't change
   `encode`'s behavior.
8. **`reed-solomon`'s existing test suite must also pass unchanged**
   after §2's addition — proving the new `_with_base` functions didn't
   disturb the existing b=1 API.
9. **`reed_solomon::decode_with_base` cross-checked directly against
   `qr-code`'s own `build_generator`/`rs_encode`**: encode a block with
   `qr_code`'s b=0 RS encoder, corrupt it, decode with
   `reed_solomon::decode_with_base(..., b=0)`, assert recovery — this
   is the one place two independently-written pieces of code (an
   encoder in one crate, a decoder in another) must agree on wire
   format, so it gets its own direct test rather than only being
   exercised indirectly through the full `qr-decoder` pipeline.

## 8. Acceptance gates

1. `qr-code`'s full existing test suite passes unchanged after the §3
   refactor (no behavior change, only extraction).
2. `reed-solomon`'s full existing test suite passes unchanged after
   the §2 addition.
3. §7's exhaustive round-trip (all 40 versions × 4 ECC levels × 3
   modes) passes.
4. §7's corruption-and-recovery and beyond-capacity tests both pass,
   proving `decode_with_base`'s b=0 correctness is verified by
   induced-error recovery, not merely by successful undamaged
   round-trips (which could pass even with a subtly wrong RS
   convention if errors are never actually exercised).
5. Every `QrDecodeError` variant has at least one dedicated test
   proving it is returned for its specific failure shape, not
   conflated with `InvalidSize`/generic failure.
6. `#![forbid(unsafe_code)]` on `qr-decoder`; no `.unwrap()`/`.expect()`
   reachable on a malformed-but-correctly-sized `ModuleGrid` (a
   `ModuleGrid` is a public, hand-constructible type — a caller could
   build one with garbage content, not only via `qr_code::encode` — so
   every read path (module indexing, bit-stream parsing, character
   count vs. remaining-data-length) must be bounds-checked, matching
   the "untrusted input" discipline every other decoder in this repo's
   VLT-PM49 tier already follows, even though this crate's declared
   input contract assumes a well-formed-shaped grid).
