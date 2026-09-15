---
category: Native extensions & FFI
---

# OTP 25+ BEAM file format (`ir-to-beam` / `encode_beam`)

: The `ir-to-beam` encoder currently produces **pre-OTP-25 format** files that OTP 28 rejects with `"This BEAM file was compiled for an old version of the runtime system"`. Three things are required for OTP 25+ compatibility: (1) an `Attr` chunk (ETF-encoded `[]`), (2) a `CInf` chunk (ETF-encoded `[]`), (3) a `Meta` chunk (ETF-encoded `[{enabled_features, []}]` = bytes `<<131,108,0,0,0,1,104,2,119,16,...,106,106>>`), AND (4) `AtU8` chunk with a **negative count** as the first 4-byte field (e.g. count = -N → `0xFFFF_FFFB` for N=5) followed by compact-term-encoded atom lengths. `beam_lib.erl` distinguishes old format (positive count) from new long-atom format (negative count, `signed-integer`). Until `encode_beam` is fixed, any test that writes `.beam` and calls `erl` must be marked `#[ignore]`.
