---
category: CI & GitHub Actions
---

# A retry budget cannot rescue a mirror that is slow rather than stalled

**What went wrong.** After the TeX install step's per-attempt cap was cut to
270 s so that three attempts fit its 20-minute budget, the books job on #17191
still failed in that step. This time the mirror was not stalled: it served the
180 MB download at a steady ~150 KB/s. Every attempt was making progress
(165 MB left after the first, 137 MB after the second) when its cap fired.
At that rate the download alone takes about twenty minutes, so no split of the
step's budget into attempts could have passed.

**Fix.** The TeX archives are now cached between runs with actions/cache, and
the apt wrapper seeds apt from that cache (only with hash-verified files), so
a warm run downloads only what changed.

**Next time.** Before tuning a timeout, read the log for which failure it is.
Timeouts and retries cure a connection that has gone silent; they make a slow
but healthy transfer worse, because each retry throws away progress. If the
transfer itself does not fit the budget, remove the transfer.
