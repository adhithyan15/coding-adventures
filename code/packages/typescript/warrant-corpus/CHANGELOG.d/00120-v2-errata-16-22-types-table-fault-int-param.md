## Unreleased - v2 runner: the `types` table read whole, `v2-fault` `ill-typed`, `Int` model parameters (VER04r 16.21 R108, 16.22)

- **R108, R117, R118.** `readTypeTable` (`src/v2/types.ts`) now reads every
  field and constructor type of the table, whole, before any value, descriptor
  or expression is read, and refuses with the caller's code (`type`,
  `ill-typed`, `descriptor`, `corpus`): a record or variant that contains
  itself directly or through `Option`, `Result` payloads, tuples or `List`
  (`List<Self>` is recursion, R118), a repeated field or constructor name, a
  member naming a type the table does not declare, a member naming an error
  type outside `Result` (R117), a member of type `Int`, and a malformed or
  over-deep member type. The cycle test is Kahn's algorithm, not a recursion,
  and is tested on a chain of 200,000 records.
- **[D74], R110.** A `v2-fault` case's bad `types` table is `ill-typed`, not
  `corpus` (`runFault` builds its `Program` with `structuralCode:
  "ill-typed"`). The order is the table, `level-missing`, `subject-forbidden`,
  then the typing of the row; tests pin each step.
- **[D75], R111, R112.** A model parameter may be `Int`, and `Int` under
  `Option`, `Result`, a tuple or `List`. Op rows, binders, fields, `env`
  entries and `v2-value` types still refuse it. Model results are unchanged.
  `tests/v2/ir.test.ts` pinned the old refusal and now pins the acceptance.
- **R115 (b).** The order inside `v2-ir` already was the bound on
  `max_ir_cost`, then the table, then everything else; now tested.
- No committed kit case changes outcome: 782 run, 176 skipped, 0 failed, as
  before. Runner source changed, so `runner.src_sha256` moves (see
  `ct-compare`).
- **R117 (b), (c) in `v2-fault`.** `runFault` name-checks the op row's
  parameter and result types against the table (the check `v2-ir` makes), so
  an error type or an undeclared name there is `ill-typed`, not
  `level-missing`. It runs after the table and before `level-missing`.
