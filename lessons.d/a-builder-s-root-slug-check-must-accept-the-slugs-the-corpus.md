---
category: Repo policy / workflow reminders
---

# A builder's root-slug check must accept the slugs the corpus already uses, or new words split old roots

The Arabic B1 builder accepted a root slug only if it was a triliteral
(`k-t-b`) or a tagged ASCII slug (`greek-kalamos`). Older Arabic lessons
also name some roots with descriptive slugs: `ithnayn` for th-n-y,
`fadl` for f-ḍ-l. When a reviewer tried to move في هذه الأثناء onto
`ithnayn`, the slug أثناء already uses in AR-C203, the builder refused it.
The only way through was a second spelling of the root, which would have
split the root ledger without `check:root-slug-splits` noticing, since it
reads only declared tags.

The fix: the builder also accepts any slug already present in the corpus
root index (`arb1-data/roots.tsv`, built from every lesson's `roots:`).
Reviewers then moved the B1 words onto the track's existing slugs
(jamaa-gather, zahara-appear, ithnayn, ayn-f-w, sawad-black,
khariif-autumn, h-w-l).

What to do differently: a slug-shape check in an authoring tool should
treat "already used by the corpus" as valid. Ask reviewers to grep each
new letter slug against the `roots:` lines of existing lessons, because
the split test cannot see a descriptive slug and a letter slug for the
same root.
