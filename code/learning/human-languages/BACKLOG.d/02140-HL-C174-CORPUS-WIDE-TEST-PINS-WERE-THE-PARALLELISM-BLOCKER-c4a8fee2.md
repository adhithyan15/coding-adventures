## HL-C174 — Corpus-wide test pins were the parallelism blocker

Measuring before fanning out changed the plan. Per-language sharding is already
thorough — `curriculum.d`, `chapters.d`, `gentle-ramp-snapshots/<lang>.d/`,
`sound-tags.d/<lang>.json`, `lesson-modality/<lang>.d/`, the artifact ceilings,
and the per-language corpus tests all give each track its own owners. Across the
last 40 human-language commits, every hotspot but one is per-language.

The exception was **`tests/grouped-shards.test.ts`, touched by 22 of those 40**.
It pinned corpus-wide totals: the reconstructed ledger's byte length and
SHA-256, plus `1_118` generated, `69` handwritten, `1_187` combined, `23`
languages. Adding one chapter in any language rewrites five of those lines, so
parallel chapter branches conflict by construction — and the second to merge
lands a stale digest that breaks main. That, not the content model, is why this
work has been running one chapter at a time.

Those expectations are now derived (see the package changelog), but the first
attempt got the source wrong and a security review caught it: it compared the
ledger to `core/generated-book-hashes/`, which `book-cli` GENERATES by iterating
that same ledger. That asserts `f(X) == X` and is weaker than the literal it
replaced. The rule this produced is worth keeping: **a derived expectation is
only a check if it comes from a tree that is not built from the thing under
test.** Here `<track>/chapters.d/` qualifies and is now compared as a set.

A second rule came out of the same reviews, in the opposite direction: **check
which pins were actually volatile before removing them.** The handwritten count
`69` looked like part of the write-lock and was not — across 300 commits it
moved only when sharding was introduced, never with chapter work — and it is the
only gate that can see a chapter flipped from `handwritten` into `targets`, the
flip `handwritten_parity.py` measures at 88 deleted prose blocks across the six
Indic tracks. Deriving it from the authored `.tex` fails too, because the
`% GENERATED FILE.` stamp is a function of `targets`, so regenerating destroys
the witness. It stays pinned. De-pin what moves; keep what only moves when a
person means it.

The remaining known pin of the same shape is
`script-ductus/tests/stroke-ownership.test.ts`, which freezes `keys: 349`, three
SHA-256 digests, and a per-script `counts` map. The per-script entries are
naturally partitioned, but `keys`, `keyHash`, and `nonTamilDataHash` are global,
so glyph work in two scripts still collides. Give it the same treatment before
fanning script work out.

One contention point is real rather than incidental: **Devanagari is shared by
hindi, marathi, sanskrit, and marwadi**, so those four cannot independently
extend the Devanagari ductus. Sequence them, or shard the ductus by script with
a declared owner per track.
