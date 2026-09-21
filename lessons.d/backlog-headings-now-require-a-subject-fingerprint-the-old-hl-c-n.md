---
category: Repo policy / workflow reminders
---

# Backlog headings now require a subject fingerprint; the old '## HL-C<n> — subject' form is rejected above rank 5000

Writing a `BACKLOG.d/` shard for chapter 112, I copied the heading style from
`HL-C408`, an existing shard I had written myself, and `check:doc-shards`
rejected it:

```
new backlog headings must use '## HL-C<number>-<subject fingerprint> — <subject>'
```

**Copying an existing shard is not evidence of the current format.** The check
only applies above `BACKLOG_FINGERPRINT_CUTOVER_RANK = 5000`, so every shard
below that rank — including the ones most likely to be reached for as examples —
is grandfathered and still shows the old style. Read a shard whose numeric
prefix is above 5000, or read the rule.

The fingerprint is derived, not chosen:

```
sha256(subject.normalize("NFC").trim()).hex.slice(0, 8)
```

where `subject` is everything after the em-dash separator ` — `, backticks and
punctuation included. `backlogIdForSubject()` in
`code/packages/typescript/human-language-data/src/doc-shard-cli.ts` is the
definition; the check recomputes it and compares, so a hand-invented suffix
fails with `backlog id 'X' must be 'Y' for subject '...'`.

**Why this exists is worth knowing, because it fixes a real failure I hit.**
Two branches may legitimately pick the same next number; the fingerprint makes
different subjects unable to silently claim the same id. That is the concrete
remedy for the HL-C410/HL-C411 collision, where another session had already
reserved my chosen number in an unmerged PR and the numbers alone could not tell
us apart.

Practically: allocate the number by scanning `grep -ho "HL-C[0-9]\+"` for the
max, then compute the fingerprint from the exact subject string rather than
typing one, and run `npm run check:doc-shards` before committing.
