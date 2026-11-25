## 0.1.1

- Security review round 1 (2 MEDIUM, 5 LOW).
- Shrinking now charges one shared budget per falsified case (`ShrinkBudget`): attempts
  (`16 * maxSteps + 64`) and elements copied (`Limits.MaxShrinkWork`, 64M). It is shared by every
  nested shrinker (lists of lists, tuples, custom shrinkers), is charged BEFORE a candidate is
  built, and a spent budget ends every loop while keeping the best minimal found. Accepted moves
  are applied in place instead of copying a second time.
- `Canon`: stack guard before calling a user `ToString()`, a 1 MiB total output cap, and escapes
  for every non-printing character (C0, C1, U+2028, U+2029); the same escaping in TFV diagnostics.
- `Gens.Filter` retries are capped at `Limits.MaxRetries` (10,000).
- Documented: nested generators are length-bounded per level, not in total.
