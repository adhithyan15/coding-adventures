---
category: Testing & coverage
---

# A wall-clock bound in a debug-build test cannot separate linear from quadratic cost on a slow CI runner; compare the code with itself at two input sizes (or count work)

`code/packages/rust/lzw/tests/encoder_cost.rs` asserted that `compress` of 8 MiB of zeros
finishes in under 10 s in a debug build, to catch the old encoder that hashed the whole
prefix (about n^1.5). On the Windows CI runner the correct, linear encoder took 12.3 s and
the test went red. The bound cannot be both loose enough for the slowest runner and tight
enough to fail the quadratic encoder (old: ~15 s for 4 MiB on a fast Linux box).

Fix: time the same function at 256 KiB and 4 MiB (16x), take the best of several runs, and
bound the ratio at 32. Linear measures 16, the old encoder 54-56, and runner speed cancels.
Checked by copying the pre-fix `src/lib.rs` (`783810d4e89`) into a scratch crate and watching
the test fail. When a test guards an algorithmic property, compare the code with itself at
two sizes (or count operations), and prove the test fails against the bad implementation.
