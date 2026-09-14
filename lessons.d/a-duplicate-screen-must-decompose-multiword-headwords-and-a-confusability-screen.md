# A duplicate screen must decompose multiword headwords, and a confusability screen must not

Screening a candidate word against the corpus has two independent failure modes, and the
Spanish A1 exam tranches have now been bitten by both.

**Miss-mode five: a word owned only as a fragment of a multiword headword.** Four
already-owned traps were already on record, each invisible to a headword-only screen —
`llevar` inside `ES-C39-traer`, `andar` inside `ES-C36-caminar`, `dar` in the `grammar`
lesson `ES-C65-di`, `llover` in the `phrase` lesson `ES-C30-llueve`. The fifth is
different again: **`amigo` is owned by `ES-C09-falsos-amigos`**, whose headword is the
two-word term of art *falsos amigos*. No headword screen sees it, no atom-id screen sees
it, and the root ledger does not either. **Only a screen that splits multiword headwords
into their component words finds it.** Screen on articles, compounds, `+` patterns, U+2026
ellipsis, and morphology — and treat a fragment as ownership.

**But that same decomposition, applied to CONFUSABILITY, invents drops.** The two
questions need two indexes and it is tempting to build one:

- *Is this already taught?* — the **wide** index. Fragments count, because a word the
  corpus utters anywhere is a word the learner has met.
- *Will a learner conflate this with something we teach?* — the **narrow** index, whole
  displayed headword forms only. A fragment was never presented as a word, so it cannot be
  the thing the learner confuses it with.

Conflating them dropped `menor` against `mejor`, where `mejor` occurs only inside the idiom
*pasar a mejor vida* and is never taught as a word. One index gives a false duplicate or a
false drop depending on which way you lean it; build both.
