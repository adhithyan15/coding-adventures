## 2026-10-19 (A number field's placeholder is omitted by decision)

- `HostNumberInput` lowers no `placeholder`, literal or bound, and that is now
  a recorded decision (UI29 §2, EM-14) rather than an unexplained gap: the
  field's text is always the bound number's `toString()`, so it is never empty
  and a placeholder has no state in which to show. A test pins the omission
  together with the `toString()` it rests on, and the emitter says to lower
  the placeholder if the field ever gains an empty state.
