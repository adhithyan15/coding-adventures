# A whole-registry data hash can preserve data and preserve the merge conflict

The Script Ductus owner split pinned one SHA-256 over the complete serialized registry. That was
excellent migration evidence: it caught any missing, reordered, or changed record. But once Tamil
records moved again from one script file to one file per glyph, keeping that expected hash in a
shared test meant every legitimate glyph correction still had to edit the same line. The data was
sharded while its proof remained a monolith.

**Rule:** distinguish a one-time migration comparison from steady-state ownership. Keep shared
gates for stable structure — discovered identities, order, counts, duplicate rejection — and put
mutable exact-data hashes beside the smallest owner they prove. During the migration, independently
compare the assembled whole object before and after; after it lands, do not require unrelated
owners to coordinate on one expected digest.
