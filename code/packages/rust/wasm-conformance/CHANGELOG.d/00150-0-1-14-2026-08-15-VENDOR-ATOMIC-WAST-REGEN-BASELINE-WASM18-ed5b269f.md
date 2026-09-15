## 0.1.14 — 2026-08-15 — vendor atomic.wast + regen baseline (WASM18)

Vendors `proposals/threads/atomic.wast` from the same pinned commit as
the rest of the corpus (fetch script updated to handle the one file
living at an upstream subdirectory path different from its local flat
filename), and regenerates the baseline against `wasm-execution` 0.6.9 /
`wasm-validator` 0.2.3 / `wasm-wast-parser` 0.1.9's new atomics support.

Verified via a full structured diff of every already-parsing file's
tally against the pre-WASM18 baseline: every non-`atomic.wast` entry is
byte-for-byte UNCHANGED. The entire aggregate delta (module +3, action
+59, `assert_invalid` +48, `assert_return` +142, `assert_trap` +45) is
exactly `atomic.wast`'s own per-file contribution -- zero regressions
elsewhere.

`atomic.wast` itself reached 100% on every directive kind except
`assert_trap`, which was 0/45 on the first regen -- the real corpus's 45
`assert_trap ... "unaligned atomic"` cases test a RUNTIME alignment
check `wasm-execution` didn't have yet (only the declared `align=`
immediate was validated statically; the effective runtime address
wasn't checked at all). `wasm-execution` 0.6.9 adds that check; a second
regen brought `assert_trap` to 45/45 and this is the baseline committed
here.

