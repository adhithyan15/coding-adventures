---
category: Repo policy / workflow reminders
---

# A duration helper that filters by lesson id reports nothing when given no ids; run the validator instead

**What went wrong.** A scratch helper printed the duration estimate for each
lesson id it was given. It was run with no ids after every content change, and
its empty output was read as "no lesson over budget". Every one of those checks
was a no-op. The Gujarati A1 tranche added one warm-up retrieval to
`GU-C08-lakhvun`, which pushed the lesson from 297s to 320s against the 300s
budget. Only `tests/cli.test.ts`'s `runValidate` caught it, as
`schema-v2-duration-budget`. Moving the line to `GU-C08-vanchvun` failed the
same way. It finally fit in `GU-C07-jaanvun`, which went from 253s to 280s.

**Why it matters.** A warm-up retrieval line costs roughly 25-45 estimated
seconds. Chapter-7 and chapter-8 verb lessons already sit at 260-300s, so
retrievals aimed at that part of a track can break the budget without anyone
noticing.

**Do differently.**
- After any lesson edit, run the validator (`node dist/cli.js` in
  `human-language-data`) and read its `ERROR` lines. Empty output from a
  filtering helper proves nothing.
- Before placing a retrieval, read the host lesson's `estimateLessonDuration`
  and choose one with at least 45s of headroom.
