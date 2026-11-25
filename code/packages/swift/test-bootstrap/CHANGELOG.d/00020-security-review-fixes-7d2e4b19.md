## 0.1.1

- Security review fixes: `PbtScope` / `Runner.currentScope()` and a `TaskLocal` bridge so
  spawned threads and tasks can stay inside the outer shrink budget; bounded `Canon`
  dictionaries and sets (shared output budget, sorted set elements, C1 / U+2028 / bidi escapes);
  clamped step budgets and `apiMaxBits` enforcement instead of traps; a 256-deep nesting cap;
  byte-exact JSON keys and corpus ids; JSON depth of exactly 64 containers.
- Tests now carry hard evaluation ceilings and timed waits so a regressed guarantee fails
  instead of hanging. Package platform floor raised to macOS 10.15 for `TaskLocal`.
