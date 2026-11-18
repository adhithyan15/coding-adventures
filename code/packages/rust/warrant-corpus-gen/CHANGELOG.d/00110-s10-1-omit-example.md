## Unreleased - CV2-S10-1: `omit_example` profile lines and the `omitted_examples` key (generator half)

No version bump, and no emitted byte of an existing fixture changes: the committed ct-compare v2
fixture (`--check`), the oracle's `accept-blind-v2` and `synth-v2` fixtures and the fixed-forms
parity all pass unchanged (the key is written only when an example is omitted). The oracle half
merged first (#19603); this PR reproduces its `omit-example-v2` fixture byte for byte.

### Added

- **`omit_example <id>`** in `corpus.profile` ([D22], [D34], VER04r 8.4): `Profile::omit_examples`.
  Profile errors (`profile`): wrong token count or an empty token, a repeated id, an id that names no
  example (never echoed), and a line for an example that is emittable (a stale profile; the message
  names the id, never a value).
- **The computed reason**, `not-op-eq-pure` | `value-over-limit`, in `tfv::examples_omitting`: the
  shape first, then the normative five-step order of 8.4 (first error a size error, or a completed
  evaluation whose final value is over 4,096 bytes, also under `Ok`). A D18 wrap, a D19 out-of-admit
  call and any other error still refuse.
- **`omitted_examples`** conditional corpus key (`[{"id","reason"}]`, sorted by id bytes, never `[]`).
  Omitted examples are not evaluated for `cases.tfv`, not encoded and not admit-checked.
- `GenError::size` / `GenError::size_error`: a `W0701` raised at the two `Bytes` limit sites (a
  literal and a `concat` past `MAX_IR_VALUE`), the oracle's `SizeError`. The integer-window overflow
  is not a size error. A malformed literal is never a size error (the shape is checked first).
- `lower::lower_document_omitting`: lowering that defers the non-`op == pure` `W0701` of
  `lower_example` for an example named by an `omit_example` line (`LoweredExample::reference` is
  `None`); the impure-guard and v0 rule-3 refusals of such a body are deferred with it, as the
  oracle's lowering never applies them to an example. Without a line the refusal is unchanged.
- `tests/omit_example.rs` (32 tests): the oracle's `test_omit_example.py` cases, the byte parity with
  `python/warrant-oracle/tests/fixtures/omit-example-v2/`, and the binary's exit codes (2 refusal,
  1 drift).

### Changed

- Every example is evaluated in the order of 8.4 (arguments, pure side, then the D19 check, then the
  D18 check) in place of arguments, D19, pure side, D18; encoding against the declared types follows.
  Only the choice between two errors of an example that already refused can differ (messages); no
  emitted byte moves.
- `LoweredExample::reference` is an `Option<ExampleRef>` (`None` only for an omitted example whose
  shape is not `op == pure`).

### Not changed

- An `omit_example` id that names no example is reported at the example stage (as in the oracle), so
  a document that also has a non-omitted example of a refused shape reports that `W0701` first.
