## HL-C413-2a2560b4 — ten remaining Indian changelogs still serialize independent chapter work

**Status: CLOSED (2026-09-20). Tracks #15740.** The post-membership contention
audit found ten Indian-track changelog aggregates still edited by otherwise
independent chapter work. Across the latest 500 commits, Marathi led with 39
unique touches; Tamil and Telugu had 38 each, and even the smallest current
owner was a 29,457-byte shared file.

Bengali, Gujarati, Kannada, Marathi, Marwadi, Punjabi, Sanskrit, Tamil, Telugu,
and Urdu now use the same strict `CHANGELOG.d/` ownership boundary proven by
Hindi and Malayalam. Their 294 historical sections moved byte-for-byte into
stable ranked fragments. The aggregate files are absent and ignored local
render targets; the CI deletion and resurrection gates cover every new plan.

Pinned pre-migration byte counts, section counts, and SHA-256 hashes prove that
all ten rendered histories are exact. Ordinary chapter work now adds one
collision-resistant fragment instead of rewriting a track-wide document.
