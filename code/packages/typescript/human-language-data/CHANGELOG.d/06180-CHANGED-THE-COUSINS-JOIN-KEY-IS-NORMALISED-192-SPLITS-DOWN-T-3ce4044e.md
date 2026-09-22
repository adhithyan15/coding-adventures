### Changed — the cousins join key is normalised, 192 splits down to 82 (HL-C419)

The guard landed one release ago and recorded the debt; this is the
normalisation it was built to make provable. **All 110 `shape` entries are
gone** — 184 `roots:` uses rewritten across **169 lessons in 18 tracks**.

| | before | after |
|---|---:|---:|
| baseline entries | 192 | **82** |
| shape entries | 110 | **0** |
| bare-vs-tagged entries | 82 | 82 |

A shape split is two slugs differing only in arrangement — same lemma, same
declared tag — which `findRootSlugSplits` calls "defects with no argument
available". Merging one asserts nothing the guard had not already established.

**The 82 bare-vs-tagged were left alone on purpose.** `bonus` against
`bonus-latin` is the opposite case: a bare slug could be a different word that
happens to share a spelling, so merging asserts an etymology rather than
revealing one — the failure `cousins.ts` exists to prevent. Each needs its two
lessons read, which is a separate pass.

#### The canonical shape is per tag, because there is no corpus-wide majority

Measured by lesson-slug incidence the global split is nearly even — prefix 1671
against suffix 1507 — and that near-tie is two opposite conventions summed:

| tag | `tag-lemma` | `lemma-tag` |
|---|---:|---:|
| latin | 257 | **1121** |
| sanskrit | **405** | 54 |
| pie | **289** | 5 |

Each entry was therefore resolved to the shape its own tag already prefers,
moving 184 uses rather than the ~1100 or ~800 a single global shape would have
cost. `pie` was kept over its `proto-indo-european` alias (289 uses against 39)
because the join is exact string equality even where the guard sees one tag.

#### What the joiner gained, and what still renders it

| | before | after |
|---|---:|---:|
| lessons with a cousin panel | 365 | **492** |
| cousin pairs joined | 575 | **857** |

The three panels HL-C419 named as silently empty now join. `ES-C280-queso`
gains `portuguese:o queijo`; `ES-C36-dormir`, `ES-C327-dormido`,
`ES-C455-dormitorio` and `LA-C40-dormio` each gain `french:dormir`; and every
`scribere` lesson but one gains at least one cousin it did not have.

Two of them still read `(none)` in the reverse direction — `PT-C24-queijo` and
`FR-C26-dormir` — and that is **not** this fix falling short. `cousinsFor`
defaults to `ROMANCE_COUSINS` (french, italian, portuguese) and excludes the
lesson's own language, so a Portuguese lesson whose only cousin is Spanish has
nowhere to show it. That is a separate limit, in the default language list.

**Nothing renders any of this yet:** `cousinsFor` is exported and tested but has
no production consumer, since `book.ts` builds its `cousinweb` from an authored
`etymology` block. No book text changed here. The gain is in the join layer,
waiting on a surface to read it.
