---
category: Repo policy / workflow reminders
---

# Use the exact rg match path in the follow-up reader

An `rg` search correctly printed the helper at
`tests/corpus/assert-language-corpus.ts`, but the chained `sed` command used the
assumed path `tests/assert-language-corpus.ts` and failed. This repeated the
same path-assumption class recorded moments earlier. Split discovery from
inspection when a path is not already verified, then copy the exact `rg`
result into the reader rather than reconstructing it from memory.
