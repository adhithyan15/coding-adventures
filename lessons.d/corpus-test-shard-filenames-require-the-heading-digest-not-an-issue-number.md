---
category: Repo policy / workflow reminders
---

# Corpus test shard filenames require the heading digest, not an issue number

Two Sanskrit corpus cases were named with the tracking issue number before the
focused suite ran. Corpus discovery rejected both: a case filename must end in
the first eight hexadecimal digits of SHA-256 over its exact `it(...)` title,
not an issue or backlog id. The files were renamed from `-17619.case.ts` to the
digests calculated from their titles, after which discovery could import them.

When adding a corpus case, choose the final test title first, calculate
`printf '%s' '<exact title>' | shasum -a 256`, and use the first eight digits in
the filename. Run the language's corpus suite immediately so a discovery error
is caught before broader validation.
