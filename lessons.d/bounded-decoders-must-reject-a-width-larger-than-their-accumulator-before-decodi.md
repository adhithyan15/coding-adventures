---
category: Rust
---

# Bounded decoders must reject a width larger than their accumulator before decoding

**What went wrong.** `wasm-leb128`'s `decode_unsigned_bounded` / `decode_signed_bounded`
took a `max_bits: u32` but accumulated into a `u64`. Nothing rejected `max_bits > 64`.
Widths 66..=70 kept a ten-byte budget and silently dropped every data bit above bit 63
(`FF x9 7F` decoded to `u64::MAX`); widths >= 71 allowed an eleven-byte input and shifted
a `u64` by 70 (debug panic, wrong value in release). It survived because every caller
passed a constant <= 64 and the first Warrant document *refined the parameter away*
(`max_bits <= 64u32`) instead of specifying the rejection, which hid the gap from the
generated tests.

**Fix.** Reject the width at the top of the shared core, before the bounds check and any
shift, with an error naming `max_bits`; specify it as a `rejects` row and add claims over
the excluded widths.

**Do differently.** When a width/size/count parameter is wider than the accumulator that
serves it, that is an input contract: reject it. A refinement on a parameter in a
specification means "undefined outside here" and is a finding to fix, not a way to make
the claims pass. Write the failing test for the widest value (`u32::MAX`) as well as the
first one past the limit.
