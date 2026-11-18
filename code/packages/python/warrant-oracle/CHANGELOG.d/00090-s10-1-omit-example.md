## Unreleased - S10-1: `omit_example` profile lines and the `omitted_examples` key (oracle half)

No version bump, and no emitted byte of an existing fixture changes: `synth-v2`, `accept-blind-v2`,
the ct-compare v2 fixture and `fixed-forms-v1` verify unchanged (the key is written only when an
example is omitted). The generator follows in its own PR (VER04r section 13, S10-1).

### Added

- **`omit_example <id>`** in `corpus.profile` ([D22], [D34], VER04r 8.4). Profile errors (`corpus`): wrong
  token count or an empty token, a repeated id, an id that names no example, and a line for an example
  that is emittable (a stale profile; the message names the id, never a value).
- **The computed reason**, from the closed set `not-op-eq-pure` | `value-over-limit`: the shape is
  tested first; `value-over-limit` follows the normative five-step order of 8.4 (`corpus_v2.omit_reason`).
  A D18 wrap, a D19 out-of-admit call or any other error still refuses generation.
- **`omitted_examples`** conditional corpus key (`[{"id", "reason"}]`, sorted by id, written only when
  non-empty). Omitted examples are not evaluated for `cases.tfv`, not encoded and not admit-checked.
- **`verify-v2`** recomputes the key and rejects `[]`, unsorted or duplicate rows, extra row keys and
  unknown reasons with a message naming the rule; it prints `N omitted examples` when `N > 0`.
- `errors.SizeError` (a subclass of `OracleError`, code `ir-overflow` unchanged) for a `Bytes` past
  `MAX_IR_VALUE`, to tell it from the exact-integer window overflow.
- `tests/test_omit_example.py` and the agreement fixture `tests/fixtures/omit-example-v2/`
  (`make_fixture.py` writes and `--check`s it; `BUILD` and `BUILD_windows` run it and `verify-v2`).

### Changed

- `_examples` now evaluates an example in the order of 8.4 (arguments, pure side, then the D19 and D18
  checks); an example that fails two of these reports the one that comes first in that order (messages
  only; no emitted byte).

### Review fixes (Fable, CV2-S10-1)

- The `not-op-eq-pure` shape test now matches the generator's `lower_example`: a subject call inside
  an argument (`echo(echo(x)) == y`) is `not-op-eq-pure` (it was refused with `ir: a subject call is
  forbidden here`). Two subject calls, a subject on both sides, a non-equality body and a
  non-top subject call are pinned by tests.
- VER04r 8.4 and the section 13 lzw row carry the four rulings (shape, first error, levels,
  evaluation order) and the lzw acceptance conditions as author refinements.
