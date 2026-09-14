# A shard for a level-N document must contain exactly one level-N heading

`BACKLOG.md` is doc-sharded at heading level 2, so each `BACKLOG.d/*.md` holds one `##`
section. Editing an existing shard to add a second `##` sub-heading looks harmless and is
not: the document then has 131 sections while the shards define 130, because the second
heading rides inside the first shard instead of owning one.

**`check:doc-shards` does not catch this.** It rebuilds the monolith from the shards and
compares bytes, and that round trip is stable in the direction it tests — the extra
heading is emitted verbatim either way. What is unstable is the *other* direction: re-run
`--shard` and the file splits in two, with a new ordinal and a new digest.

`doc-shard.test.ts`'s "shard order on disk reproduces REAL section order" is the check that
notices, because it counts sections on both sides instead of comparing bytes.

**Use a lower heading level for structure inside a shard** (`###` under a `##` shard), and
if a finding genuinely deserves top billing, give it its own shard file with its own
ordinal.

**Generalisable check:** whenever a round-trip has two directions, a byte-comparison of one
direction is not evidence about the other. Assert on the structure both sides define, not
on the bytes one side produces.
