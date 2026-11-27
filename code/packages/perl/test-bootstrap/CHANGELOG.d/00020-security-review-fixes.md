## [0.01] - Security review fixes

### Fixed

- A `Math::BigInt` NaN or infinity no longer hangs `Int::bit_length` (and through it
  Canon, `Values::show` and the corpus): Canon renders `NaN`, `inf`, `-inf`, `bit_length`
  refuses them, and every correction loop is bounded.
- Engine code no longer uses `->can` (a planted `Safe::can` could turn a failing
  property into a pass); every internal call is fully qualified. `Safe::call` restores
  `%Hooks::H` and `$Budget::CURRENT` after each user call. The seal comment no longer
  overstates.
- `Int::check` copies, normalizes, digit-caps and finite-checks a caller's big;
  `Int::cmp` is right for an unnormalized small big.
- A root budget node no longer keeps itself alive (about 650 bytes leaked per check).
- `redo` inside a property is an escape (the code runs once); documented that labelled
  `last`/`next` aimed at the caller's loop cannot be intercepted.
- Filter predicates run in scalar context through `Safe::call`.
- `holds` is strictly the integer 1 or the string "1" (by flags): `1 + 2**-52` and
  `1.0` falsify.
- `Canon::_decode` localizes `$@`.

### Documented

- `exit`, `exec`, `fork`, `kill` are not contained; `repro-fnv` is a non-secret
  fingerprint; the rule by which a Perl scalar reads as an int, bytes or a float.

### Tests

- `t/10-security.t` covers each item.

## Round 2

- `Int::cmp`/`eq`/`le` refuse NaN and infinity; `Values::same` never equates them.
- `holds` accepts an IV 1, an NV exactly 1.0 and the string "1" (also after float use or
  taint); dualvars with another number or string, `1 + 2**-52`, `"1.0"` and tied scalars
  falsify.
- `stash_name` no longer calls `->isa`.
- Documented that `Math::BigInt` global configuration must be at its defaults.
