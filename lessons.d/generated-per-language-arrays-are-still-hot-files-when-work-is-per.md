# Generated per-language arrays are still hot files when work is per lesson

The modality manifest had already escaped a corpus-wide aggregate, but every
lesson in one language still lived in the same generated `<language>.json` array.
That was too coarse for the way the curriculum grows: two agents changing two
Spanish lessons both regenerated the same 619 KB file even though their semantic
owners did not overlap.

The stable ownership boundary is the lesson id:
`core/lesson-modality/<language>.d/<lesson-id>.json`, with only low-churn format and
policy fields in `_meta.json`. The complete public manifest can still be folded in
memory. Source hashes, chapter prefixes, track summaries, and corpus totals are
projections, not reasons to preserve a shared tracked file.

Generated data also needs stronger completeness evidence than the directory being
generated. Here the registered languages determine the exact directory set, parsed
lesson sources determine every expected lesson owner, and narration hashes provide a
second independent lesson-id ledger. All three sets must agree before owner bytes are
accepted. The generator builds the replacement under a private staging root, verifies
canonical bytes and exact identities there, publishes it only when complete, and
deletes the old aggregates last.

**Rule:** match generated ownership to the smallest unit parallel work changes, derive
rollups in memory, and close deletion against independent source ledgers. Do not keep a
compatibility aggregate or fallback reader: either one silently restores the conflict.
