## 0.1.2

- Canon sorts set/dictionary entries by UTF-8 bytes (NFC and NFD stay distinct and stable).
- Step counts returned by nested user shrinkers are clamped to [0, remaining] in sequence, `list`
  and tuple shrinking, so `Int.max` or negative values cannot trap.
- Nesting cap lowered from 256 to 64 so recursion fits a 512 KiB secondary-thread stack.
- `lastWork`, `lastAttempts` and `defaultMaxWork` are guarded by a lock.
