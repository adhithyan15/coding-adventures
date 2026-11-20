# Unreleased: `saturate-add`, `saturate-sub`, `shift-mask` and the scalar `missing-error` (S11f-2)

- `faults.rs`: the registry gains `saturate-add`, `saturate-sub` and `shift-mask` (17 names), in the exact
  JSON of VER04r 8.2 [D51] and byte for byte what the oracle's S11f-1 emits.
  - `saturate-add` / `saturate-sub` (kind `replace`): the first two parameters share one fixed-width type `T`, the
    result is `T` or `Result<T,E>` (then `["ok", V]`); `V` is `unwrap_or(checked_add|checked_sub(A,B), bound)`,
    the bound `max_value(T)` / `min_value(T)` for an unsigned `T` and `if B < 0 then .. else ..` for a signed one.
    Template level `ir.fixed`.
  - `shift-mask` (kind `pre`): the last parameter `p` becomes `unwrap_or(rem(p, w), 0)` (`rem_trunc` for a signed
    one), literals in `p`'s type, `w` the FIRST parameter's width. Template level `ir.fixed`.
  - `missing-error` is generalised ([D50], R31): a `Bool`, `Nat` or fixed-width payload is the first parameter of
    exactly that type, else the type's zero literal; a `Bytes` payload with no `Bytes` parameter is now the zero
    `Bytes` literal (it was refused). Every existing form is byte-identical.
- Refusals: a non-`Result` result is `missing-error needs a Result result` (was `needs a Result<Bytes, E> or
  Result<tuple, E> result and a Bytes parameter`, which no longer describes the rule), a payload with no template
  (`Option`, a variant) is `has no template for this payload type`, and a named-width `Bytes<N>` payload with no
  `Bytes` parameter is `a named width has no zero literal`. Error text only: the code stays `fault`.
- `tests/scalar_faults_parity.rs`: byte parity with the oracle's `scalar-faults-v1` dumps (41 fault rows and
  controls under the closed and the open gate), the 53 refused signatures (schema `warrant-faults-refusals-v2`,
  a `fault` per row), and unit tests of the exact JSON of each new form.
- `tests/faults.rs`: `KNOWN_FAULTS.len()` is 17, and the `missing-error` refusal on a `Bool`-result op now
  matches `needs a Result result`.
- No version bump: no committed fixture names the new faults and the `missing-error` forms that existed are
  unchanged, so no emitted byte of an existing fixture moves.
