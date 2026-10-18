## Chapter 2 is generated, and a paradigm the reader was never shown is not printed

Chapter 2 — *Introducing Yourself* — was hand-written LaTeX over nine schema-v1
lessons. It is now nine schema-v2 lessons and generated from them. **French has
eight hand-written chapters left, all in the 9–16 range.**

### Four ways of sizing it

| instrument | answer |
| --- | --- |
| `handwritten_parity.py` | **0** — the chapter does not appear in the report |
| `grep -l '^chapter: 2$' lessons/*.md` | **9** lessons |
| the `.tex` | **9** sections |
| taught-word census | **0** French words owned by no lesson |

Same shape as chapter 1 and for the same reason: chapters 1 and 2 are the two
whose lessons were authored before the LaTeX. The census residue is 99 distinct
emphasised tokens, 55 matching no headword, and all of them are Latin etymons
(*appellāre*, *cantāre*, *mē*, *tē*, *sē*, *tū*, *vōs*, *nōmen*, *quo modo*),
Spanish comparanda (*cómo*, *usted*), pronunciation respellings (*zhuh*, *muh*,
*ahn-shahn-TAY*), or elided and inflected forms of words the chapter owns
(*m'appelle*, *j'ai*, *vous appelez-vous*, *tu t'appelles*).

Two words are *named and deliberately not taught* — *nom* (in **mon nom est**,
offered as the stiff alternative you should not reach for) and *vouvoyer* (the
verb for addressing someone as *vous*). Both were named-not-taught in the `.tex`
and are named-not-taught in the lessons, so nothing moved.

### The fourth instrument found the real defect, and it was in a table

`FR-C02-me` carried the reflexive set as a **four-column table** — French /
means / root / English cousin — with **me**, **te** and **se** down the first
column. The hand-written chapter had never printed it: its `grammarlens` wrote
the same three pronouns as one sentence of prose. So generating the chapter
would have put a paradigm on the reader's page that the reader had never been
shown, at the second lesson of the second chapter.

Three things were wrong with it at once, and only one of them is a gate:

- HL10 forbids printing a paradigm table until **every cell in it has been
  taught individually**. *te* is used once in this chapter and *se* not at all.
- Four columns is over `maxLinearisableTableColumns`, so the narrator **refused**
  the table and spoke an apology instead. The corpus-wide refusal count falls
  **51 → 50**.
- `info-dump.ts` would not have caught it. Its French person labels are *je, tu,
  il, elle, on, nous, vous, ils, elles* — subject pronouns. `me / te / se` are
  reflexive pronouns and match none of them, so `personRowCount` returns 0 and a
  genuine three-person paradigm scores clean. **The gate not flagging a table is
  not evidence the table is fine.**

The lesson now carries the `.tex`'s prose, and gains a strand the table had no
column for: *sē* is the "self" root behind *separate* and, at a distance,
**su**icide (*suī*, "of oneself"). It says outright why the set is named and not
gridded — *"you meet each one where it earns its place."*

### Twelve atoms, two of them minted beside an id a later chapter owns

`FR-LEX-COMMENT-HOW-10` sits **beside** chapter 32's `FR-LEX-COMMENT-05`, and
`FR-GRAM-INVERSION-NAME-11` beside its `FR-GRAMMAR-INVERSION-08`. Re-pointing the
generated chapter 32 would have been the tidier-looking move and a far larger
blast radius: `FR-LEX-COMMENT-05` is one of the five atoms in the A1-INT-04 probe
and is named in chapter 32's payoff. The two readings are genuinely different —
here *comment* is **"how"**, the manner word the name-question runs on; there it
is one of the five question words in the interrogative system.

### It closes an exam point that the corpus had always taught

French A1 coverage **30/74 → 31/74**: **A1-V-12**, reflexive verbs with *se* in
the present. Chapter 27 has conjugated *s'asseoir* and *se lever* through the
whole present — *il se lève*, *ils se lèvent* — since it was written, and this
chapter has always taught *je m'appelle*. But the **construction** was owned by
nobody: chapter 27's atoms type the two verbs and their stems, and chapter 2 was
hand-written. A corpus that fully teaches the point had no atom that named it.
The probe lists the rule and the two conjugated verbs, because the rule alone is
not the present tense and the verbs alone were typed as lexis.

**A1-P-04** was already covered and its probe is corrected in the same pass. It
read `FR-GRAMMAR-PLEASE-REGISTER-04` — chapter 19's *s'il vous plaît*, which
*demonstrates* the tu/vous register without naming it. Chapter 2 owns the point
directly and both its atoms are added.

### One repair the ordering gate found

`FR-PATH-007` listed *je, appeler, me, …*, so *appeler* preceded the *me* its own
grammar lens builds on. Invisible while the lessons were schema-v1; a hard
prerequisite-order failure the moment they were typed. The segment is now in
teaching order.

### Measured, against the merged tree

- Hand-written French chapters: **9 -> 8**. Every French chapter that remains is
  in the 9–16 range.
- Corpus hand-written chapters: **French 8, German 9, nothing else** — stated as
  a census rather than a total, and deliberately not differenced. The total was
  20 when this branch was cut, 19 after this chapter, 18 after German chapter 5
  landed from a sibling, and 17 after German chapter 15 did, none of which this
  branch touched. A corpus counter that two other branches are moving is not
  this change's to compose; the half that IS this change's to claim is French's
  9 → 8, and the census survives the next merge in a way a total does not.
- `handwritten_parity.py french`: **45 -> 45** blocks at risk — chapter 2
  contributed none of them.
- Schema-v2 French lessons: **119 -> 128**.
- Atoms taught: **201 -> 213**.
- Atom-measurement-blind lessons: **28 -> 19**.
- Chapters over the 12-atom budget: **0 -> 0**.
- Culture claims: **20 -> 21** — French *prefers* the verb-based *je m'appelle*
  to the literal *mon nom est*, which exists, is understood, and sounds like a
  passport desk. It was the chapter's only `culture` block and no lesson owned it.
- Forward references: **48 -> 48**.
- Atoms never revisited: **37 -> 37**.
- Reinforcement window misses: **443 -> 474**. Twelve new atoms exist to be
  missed.
- Narrator table refusals, corpus-wide: **51 -> 50**.
- Cross-chapter prose references: **0 -> 0**.
- Completion-plan uncovered points: **785 -> 784**, re-measured by running the CLI.
- The book compiles under XeLaTeX with `missing_character = 0`, every warning
  class at its baseline and no undefined reference surviving the final pass, and
  the rendered pages were read.
