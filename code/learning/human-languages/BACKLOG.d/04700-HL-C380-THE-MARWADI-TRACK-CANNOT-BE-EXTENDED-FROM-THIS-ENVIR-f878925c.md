## HL-C380 — the Marwadi track cannot be extended from this environment, because every source it cites is unreachable

Marwadi is the weakest Indian track in the corpus: **79/197 (40%)**, 118 points
uncovered. Its gaps are unusually cheap on paper, because five of them share a
single insight — the **-ओ / -ई** alternation running through adjectives,
possessives and demonstratives alike:

| point | what is missing |
|---|---|
| `MW-A1-ADJ-02` | every adjective in the corpus stands in a predicate; none is attributive |
| `MW-A1-ADJ-03` | *sasto* against *sasti* — the agreement is never shown on an adjective |
| `MW-A1-POS-03` | *mhaaro* is drilled for twenty chapters and *mhaari* is unsayable |
| `MW-A1-DEM-02` | the far demonstrative *vo* returns ZERO occurrences; the deixis is half a system |
| `MW-A1-DEM-03` | *ye* is only ever attributive, never standing alone, never in another form |

**The blocker is sourcing, not authoring.** 212 of the track's 346 lessons carry
a `Source:` line with a citable URL — Marwari Pathshala, Omniglot, the SIL
*Marwari–English Dictionary*, the Rajasthan Sahitya Akademi's *Jāgtī Jot*. That
convention is the track's whole claim to reliability, because Marwari is a
low-resource language where a plausible-sounding paradigm is easy to invent and
hard to check.

From the current execution environment **every one of those domains is refused
by the network egress proxy**, including `unicode.org` and `omniglot.com`, which
the track already cites. Web *search* returns snippets; web *fetch* returns
`EGRESS_BLOCKED` for all of them. A tranche written here would have to either
carry no `Source:` line, breaking the track's convention, or cite a page the
author could not read.

**So the tranche was not written.** Authoring Marwari morphology from model
recall, in the one track that cites a source on every lesson, is the precise
failure this backlog exists to prevent — and it would be least detectable
exactly where it is most dangerous.

**What unblocks it**, in order of preference:

1. Run the tranche from an environment whose egress allows the cited domains.
2. Vendor the needed pages into the repo, the way `_fonts/` vendors script
   fonts, so the claims are checkable at review time by someone without network
   access.
3. Failing both, prefer tracks whose next tranche needs no external source.
   Spanish orthography was taken instead for exactly this reason: the marks of
   Spanish punctuation are demonstrated on every page of the corpus already.

Until one of those holds, **Marwadi's 40% is a reporting fact rather than a work
queue**, and picking it up because it is the lowest number will produce
unsourced content.
