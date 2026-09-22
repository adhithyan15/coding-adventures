### Added — check:root-slug-splits, a guard for the cousins join key (HL-C419)

`roots:` is the cousins join key and `cousins.ts` joins on **exact string
equality**. The corpus spells one etymon three ways — bare `stare`,
`stare-latin`, `latin-stare` — so three lessons sharing a Latin verb produce
three panels of one item each instead of one panel of three.

**The loss is entirely of omission**, which is why it survived: a missing
cousin panel looks exactly like a word that has no cousins yet.

```
core/root-tags.json                  99 declared tags, 1 alias
core/root-slug-split-baseline.json   192 entries = 175 etymon groups, 372 slugs
check:root-slug-splits               fails on a new split, and on a stale baseline
```

**This fixes nothing.** The ~700-lesson normalisation still wants its own PR,
because a wrong merge *asserts a shared etymology that is not there* — the one
failure mode `cousins.ts` exists to prevent. Without a guard the corpus drifts
back the first time somebody types a slug from memory, which is not
hypothetical: writing the Spanish A2 chapters produced three invented slugs in
four tranches.

#### The vocabulary is declared, not inferred

A split cannot be detected without knowing which hyphen-separated token is a
language tag, and the corpus has both kinds: `-latin` (651 slugs) and
`-dravidian` (63) are tags, while `-see`, `-heart`, `-speak` and bare single
letters are glosses. Nothing in a slug distinguishes them.

The earlier census of this defect was wrong three times running and every one
was the same mistake — a category list living in the counting code rather than
in the data. `core/root-tags.json` is that list, written down and read from the
module. A slug whose tag is **not** declared is opaque: an undeclared tag costs
coverage, never a false positive.

#### The unit is an entry, not an etymon

192 baseline entries: 110 *shape* plus 82 *bare-vs-tagged*. Sixteen
bare-vs-tagged entries strictly contain a shape entry for the same lemma, so
the file covers **175 distinct etymon groups over 372 slugs**. Saying "192
splits" would double-count those sixteen — the mistake HL-C419's own method
note records.

#### What it does not catch

- **One lemma under two declared tags** — `bursa-greek` against `bursa-latin`.
  Folding these would *assert* a shared etymology. Thirty-eight lemmas carry
  two or more tags and most are legitimate ancestor/descendant pairs, so
  `bursa` is a real defect this guard will not report.
- **Prefix families** — `permittere-latin` against `mittere-latin`. Different
  lemmas, so no normalisation of shape can see the relationship. Not
  hypothetical: a lesson claimed *admitir* was the third corpus word off
  *mittere* when it is the fourth, and a census keyed on the slug cannot see
  why.

175 is a **floor**.

#### Fixed in review

- **Case-insensitive matching.** The corpus has exactly one case-only
  duplicate — `SANSKRIT-PA-DRINK` in two Marwadi lessons against
  `sanskrit-pa-drink` in four Gujarati, Punjabi and Marathi ones. Six lessons,
  four tracks, one etymon; the case-sensitive draft left it invisible.
- **One notion of a slug.** `liveRootSlugs` now calls `cousins.ts`'s exported
  `rootSlugs()`. The first draft read the frontmatter separately via
  `arrayify`, which wraps an unbracketed `roots: a, b` as one string where the
  joiner splits on the comma — a guard reading different bytes from the thing
  it guards.
- **The write side guards itself.** `--write` refuses a symlinked or sharded
  target, mirroring `script-owner-evidence.ts`; `readLedgerFile` already
  refused them on read, so the two halves disagreed about one path.
- **Corpus bytes are stripped before printing.** A slug reaches a CI log
  through the thrown message; `\u001b[2K` erases the rendered line and a bare
  `\r` can forge a log line. Now passed through `stripControlCharacters`.
- **`--write` refuses to grow the baseline** without `--allow-new`, since it is
  the command contributors are told to run.
- Prototype-free alias table, blank tags rejected, element-wise slug
  comparison, code-unit ordering, and a shape assertion on the baseline.
