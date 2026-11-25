## 0.2.1

- Security review round 3 (C#-specific HIGH plus smaller items).
- `Gen.ShrinkNested` is now internal (it joins the budget without being a safe user entry point) and
  charges every `falsifies` evaluation to the budget it joins, so leaf shrinkers that charge nothing
  are bounded even under recursion, with or without an ambient scope.
- The nested-check work estimate also counts `ICollection<T>`-only types (`HashSet<T>`, ...), lazy
  `IEnumerable`s (capped) and `ITuple`.
- Every `Filter` retry is charged inside a check: attempts in a nested check, work against the check
  root at top level, so stacked filters cannot multiply uncharged retries.
- Docs: the bound is per execution-context flow.
