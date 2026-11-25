## 0.2.0

- Security review round 3: the join/isolate classification of the shrink budget is replaced by the
  classification-free hierarchical budget tree designed for the Java engine (VER05e 4.1).
- `ShrinkBudget` is a tree of scopes with one lock at the outermost root; `Take` walks node to root,
  checks room everywhere (`cost > max - used`, overflow-safe) and increments all or nothing; the root
  records refusals; owning depth is capped at 64; closed scopes resolve to the nearest open ancestor
  and a fully closed chain gets a fresh root. `AsyncLocal` carries the ambient scope into threads and
  tasks.
- Public entries (`Runner.Check`, `Gen.Shrink`, `ShrinkInt`, `ShrinkList`, `ShrinkBytes`,
  `SimulateReplay`) open a child scope with a fresh local cap; combinators join (`Gen.ShrinkNested`).
- `Runner.Check` runs its whole body in one scope. A nested check charges entry, cases, sampled size
  (counted one level deeper, capped, hostile-collection safe) and filter sampler retries; a starved
  nested check is `property-exhausted`; a top-level check never passes when something in its tree was
  starved and never overrides a falsified verdict; re-verification is charged to the check scope.
- `Filter` charges before its predicate; `maxSteps == 0` returns the input uncopied.
- Version 0.2.0.
