---
category: Testing & coverage
---

# A dry-run screen that reads each tranche alone cannot see clashes with tranches generated earlier in the same run

**What went wrong.** Five Punjabi A2 tranches were screened in one dry run.
Each builder pass compared a tranche's cues against the glosses in the
*worktree*, and in a dry run the worktree holds none of the run's own
tranches. Every cross-tranche screen passed. Real generation then writes
tranche a before building b, so the same check stopped tranche c's ਧੂੜ ("dust")
on tranche a's ਝਾੜਨਾ ("to dust"), with the leading "to" stripped. A
cross-tranche scan then found a true duplicate: tranche e's ਸੰਭਾਲਣਾ "to look
after" was a doublet of tranche c's ਸਾਂਭਣਾ. Each mismatch cost a full
generation and a reset.

**The fix.** `gen.sh` now has a screen-time "cue vs gloss across tranches"
check. It reads the dry-run registries (`paa2/built/<x>.json`) in claims order.
It refuses a later cue that equals an earlier tranche's gloss sense, after
stripping "to", "a" and "the", unless the word carries an `allow['cue']` with a
reason.

**Do differently.** Every check the builder runs against "what the track
already teaches" needs a twin that runs against the registries of the earlier
tranches in the same run. Otherwise a multi-tranche run validates each tranche
alone and fails only when the tranches meet.
