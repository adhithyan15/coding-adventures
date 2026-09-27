## HL-C414-b95d5c14 — remaining Indian corpus regression ownership is sharded

**Status: CLOSED (2026-09-20). Tracks #15741.** Bengali, Gujarati, Kannada,
Marathi, Marwadi, Punjabi, Sanskrit, Tamil, Telugu, and Urdu still kept unrelated
track, writing, exam, and chapter regressions in one language-wide test file.
Those aggregates forced independent curriculum work through a shared edit
surface even after the lesson and changelog ownership boundaries were split.

Each remaining Indian track now has stable concern or chapter modules under its
own corpus directory. A tiny explicit entrypoint loads every sorted owner,
rejects duplicate or unsafe names, and prevents the retired aggregate from
returning. The original 82 assertions and one Vitest worker entry point per
language are preserved, so parallel ownership does not weaken the regression
gate or trade edit contention for runner contention.
