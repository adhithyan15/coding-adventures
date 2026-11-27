## Unreleased - CV2 S10e review 2: the `match` budget must not be tighter than what a generator emits

The kit run is unchanged (1,452 run, 0 skipped, 0 failed). No dependency added.

- The usefulness check no longer branches on every declared case of a column. It splits a `Result` column into the
  constructors its patterns name and **one rest branch** for the others (Maranget's default matrix; `ok` nobody names and
  an `err` case nobody names are separate rests, since `err_any` matches only the second). The work no longer depends on
  how many cases an error type declares: a `match` over `Result<Bool,E>` columns with a 21-case `E` was refused on budget
  and is now accepted.
- The budget is 200,000 per `match`, each step costing `1 + rows` (it was 20,000 steps). A guard-column family of 8 and 10
  data columns is accepted; a clearly exponential shape is still refused in well under a second.
- A variable bound to a value of an undetermined type (`ok(bind x)` on a bare `err`) may not be used in an arm body
  (`ill-typed`); an unused one is still accepted.
- Tests in `tests/test_v2_tuple.py`.
