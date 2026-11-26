## Unreleased - three spec gaps found by the three-way differential (R58, R114, section 11 `types` shape)

The kit is unchanged: 782 cases run, 0 failed, the mutants pass, and no committed kit case changes outcome. The
`ct-compare` v1 manifest is regenerated, because the runner digest moves with the source. No dependency added.

- **R58 reserved built-ins.** `wtypes.check_types_table` refused `Nat`, `I64` and the other type-grammar names but
  accepted a row or constructor named `Some`, `None`, `Ok`, `Err`, `len`, `at`, `concat`, `fixed` or `div`. A record,
  variant or error row name and a constructor name now may not be any of the checker's `V0_BUILTINS` or a fixed-width
  type name (`RESERVED_DECLARED_NAMES`), with the code of the context. Field names stay free. The lane reads no document
  version, so the v1-only functions (`rem`, `take`, ...) are not applied; the v1 types were already refused.
- **`types: null` in `v2-ir`.** `Program` turned an explicit `None` into `{}`; only an absent table means `{}` now, and
  a present non-object (`null`, `[]`, text, a number) is `ill-typed`.
- **`types` in `v2-domain` (R114).** A present `types` is read whole as `descriptor` before any descriptor node, so
  `null` and `[]` (falsy, so skipped before) are refused; an absent one still means `{}`.
- `v2-value` (`type`), `v2-fault` (`ill-typed`) and `v2-chunk` (`corpus`) already refused a non-object table; tests now
  pin all four contexts for `null`, `[]`, text, a number and `false`, and every reserved name in every context.
- Tests: `tests/test_v2_lane_checks.py`.
