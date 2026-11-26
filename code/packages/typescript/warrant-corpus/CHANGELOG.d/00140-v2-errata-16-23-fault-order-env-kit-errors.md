## Unreleased - v2 runner: VER04r 16.23 fault-row order, `env` kit errors, bare `model_call` (CV2-16-23)

- **R119 (c), `pre` rewrites.** `compileFault` compiled the rewrites one at a
  time, so a typing defect or a `subject_op` in an earlier rewrite hid
  `unknown-tag`, `depth` or `model-order` in a later one. Each step now runs over
  every expression of the row before the next starts: `level-missing` of the tags
  the expressions reach (an unknown tag has no level and is skipped; `list` is
  `ir.list`), then the three codes, then `subject-forbidden`, then the typing. New
  `Program.checkReach` and the `fault-reach` role (a fault expression read with
  `subject_op` allowed) carry the first two steps.
- **R119 (a), bare `["model_call"]`.** In a fault case (no models) a `model_call`
  with no or a non-text name is `model-order`, like any other `model_call`
  there; it was `ill-typed`.
- **R120 (b), `env` in `v2-ir`.** An entry with no `value`, a missing or
  non-text `type`, an entry that is not a `[name, object]` pair, or a value that
  does not decode as its type is a kit error (a harness error, counted as a
  failed case, never an outcome). It returned `value` or threw a raw
  `TypeError`. A type text the reader refuses stays `ill-typed` (a vector).
- **R126.** A malformed fault row (extra or missing key, bad `kind`, undefined
  level in `requires`, malformed `rewrites`) is a kit error in `runCase`/`runKit`
  (a plain `Error`; R126 (b) pins no code). The row's shape, `rewrites`
  included, is read whole before any level. `compileFault` itself still raises
  `corpus`, which the corpus reader needs (S6b, S7b).
- No committed kit case changes outcome: 782 run, 176 skipped, 0 failed.
  Runner source changed, so `ct-compare`'s manifest was regenerated.
