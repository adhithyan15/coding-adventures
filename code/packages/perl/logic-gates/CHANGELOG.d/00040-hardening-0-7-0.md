## [0.7.0] - LGE-PL-HARDEN

### Security

- State is validated and SNAPSHOTTED before any computation: `DFlipFlop`,
  `Register`, `ShiftRegister` and `Counter` accept only unblessed, untied
  hashes/arrays with exactly the expected keys (`exists`, never autovivifying),
  copy the values, validate every bit, then compute from the copy. Non-hash
  elements and undef elements now die with the package Error (previously plain
  strings, and undef elements were autovivified into `{}` in the caller's array).
- No caller code runs during validation: tied arrays/hashes are rejected before
  length or elements are read; references and overloaded objects are never
  stringified or numified.
- Outputs are canonical fresh 0/1 integers; `ShiftRegister` `serial_out` no
  longer returns the caller's own scalar (a `dualvar(5,"1")` used to come back
  numerically 5). Nothing writes to `@_`.
- Caps: `ANDn`/`ORn`/`XORn` inputs and `Register`/`ShiftRegister` lengths are
  limited to `MAX_WIDTH` = 65536, checked before any work.
### Breaking

- `Error->new` always blesses into `CodingAdventures::LogicGates::Error`; the
  invocant is ignored. It also rejects reference/object field values and bad
  codes with a fixed-text package Error; `is_invalid_bit` never dies on forged
  or tied objects.
- `Counter` width is capped at 64 (was 65536); `new_counter_state(65)` dies.
- Tied scalars (arguments, array elements, hash values) are rejected with
  `INVALID_SHAPE` before being read; each state value is read once.
- `SRLatch`, `DLatch`, `DFlipFlop`, `Register`, `ShiftRegister` and `Counter`
  die with the new code `INVALID_CONTEXT` when called in scalar context (they
  return lists; scalar context silently yielded only the last element). List and
  void context are unchanged.
- `Register([1],1,[undef])` now dies (`INVALID_SHAPE`) instead of defaulting.
- Flip-flop state hashes must have exactly the four keys; Counter state exactly
  `bits` and `width`. Counter validates `bits` even on reset.
- Messages for state-shape errors gained a parenthetical (`plain`, `untied`).

### Added

- `INVALID_CONTEXT` error code; `required_capabilities.json` (no capabilities);
  `t/03-hardening.t`.
