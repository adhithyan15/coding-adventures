# Specification: Ruby Lexer Token Flag Bit Allocation (#18798 Item 17)

## Problem Statement

The `lexer` base crate defines four metadata flag bits on `Token.flags: Option<u32>` (`code/packages/rust/lexer/src/token.rs`):
```rust
pub const TOKEN_PRECEDED_BY_NEWLINE: u32 = 1; // 1 << 0
pub const TOKEN_CONTEXT_KEYWORD: u32 = 2;     // 1 << 1
pub const TOKEN_PRECEDED_BY_SPACE: u32 = 4;   // 1 << 2
pub const TOKEN_SYNTHETIC: u32 = 8;           // 1 << 3
```

In `code/packages/rust/ruby-lexer/src/lib.rs`, Ruby-specific token flags were allocated using the same bit positions:
```rust
pub const NUMBERED_BLOCK_PARAM_FLAG: u32 = 1 << 0;
pub const ENDLESS_RANGE_FLAG: u32 = 1 << 1;
```

This caused direct bit collisions:
1. `NUMBERED_BLOCK_PARAM_FLAG` collided with `TOKEN_PRECEDED_BY_NEWLINE` (`1 << 0`). Any token preceded by a newline (including non-block parameter identifiers and keywords) had bit 0 set in its flags, falsely satisfying `(t.flags.unwrap_or(0) & NUMBERED_BLOCK_PARAM_FLAG) != 0`. Conversely, setting `NUMBERED_BLOCK_PARAM_FLAG` on an inline parameter identifier marked it as preceded by a newline.
2. `ENDLESS_RANGE_FLAG` collided with `TOKEN_CONTEXT_KEYWORD` (`1 << 1`). Tagging range tokens with `ENDLESS_RANGE_FLAG` marked them as context keywords, and any token flagged as a context keyword appeared to have `ENDLESS_RANGE_FLAG`.

## Design & Bit Allocation

The base `lexer` crate reserves bits 0..7 for general token metadata:
- Bit 0 (`1 << 0`): `TOKEN_PRECEDED_BY_NEWLINE`
- Bit 1 (`1 << 1`): `TOKEN_CONTEXT_KEYWORD`
- Bit 2 (`1 << 2`): `TOKEN_PRECEDED_BY_SPACE`
- Bit 3 (`1 << 3`): `TOKEN_SYNTHETIC`
- Bits 4..7: Reserved for common lexer flags.

Language-specific lexers allocate flags in bits 8 and above (`1 << 8` through `1 << 31`):
- Bit 8 (`1 << 8`): `NUMBERED_BLOCK_PARAM_FLAG` in `ruby-lexer` (Ruby 2.7+ `_1`..`_9` candidate block parameter).
- Bit 9 (`1 << 9`): `ENDLESS_RANGE_FLAG` in `ruby-lexer` (Ruby 2.6+ `..` / `...` followed by closer).

## Invariants & Verification

1. **Disjointness**: `(NUMBERED_BLOCK_PARAM_FLAG & 0xFF) == 0` and `(ENDLESS_RANGE_FLAG & 0xFF) == 0`.
2. **Newline independence**:
   - In Ruby 2.6, `\n_1\n_2` has `TOKEN_PRECEDED_BY_NEWLINE` set on `_1` and `_2`, but `NUMBERED_BLOCK_PARAM_FLAG` remains unset.
   - In Ruby 2.7, `\n_1\n_2` has BOTH `TOKEN_PRECEDED_BY_NEWLINE` and `NUMBERED_BLOCK_PARAM_FLAG` set.
   - In Ruby 2.7, inline `_1 + _2` has `NUMBERED_BLOCK_PARAM_FLAG` set on `_1` without `TOKEN_PRECEDED_BY_NEWLINE`.
3. **Context keyword independence**:
   - A range token tagged with `ENDLESS_RANGE_FLAG` does not have `TOKEN_CONTEXT_KEYWORD` set.
   - A token with `TOKEN_CONTEXT_KEYWORD` does not have `ENDLESS_RANGE_FLAG` set.
