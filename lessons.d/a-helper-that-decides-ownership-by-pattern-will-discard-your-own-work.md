# A helper that decides ownership by pattern will discard your own work when you rename it

Re-sharding a merged document renames shards it did not author (previous entry), so I wrote
a helper to restore the foreign ones: it matched shard filenames against a regex of *my*
slugs, kept those, and reverted the rest.

Then I rewrote my three entries' headings — which is what the whole edit was for — and ran
it. The slugs no longer matched, the helper classified my own new shards as foreign,
deleted all three, and the subsequent `--unshard` rebuilt the document from what was left.
The monolith lost three sections and `--check` passed, because the shards and the monolith
agreed perfectly about the reduced content.

The section count is what caught it: 132 where 135 was expected. The byte-level round trip
cannot see this, and neither can a grep for the titles — the titles were the thing that had
changed.

**Rules:**

1. **Ownership is a fact you know, not a pattern you infer.** Pass the file list explicitly.
   A helper that guesses which changes are yours will guess wrong exactly when you have
   changed them.
2. **Count before and after, and make the expected count a number you wrote down first.**
   "135 sections, 135 shards" is a check; "shards and monolith agree" is not, because a
   tool that deletes from both keeps them agreeing.
