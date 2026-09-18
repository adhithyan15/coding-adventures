### Added — Malayalam chapter 100, what you wear, and a check demoted to a signal

- `ML-A1-LEX-29` closes. Malayalam A1 coverage 206/243 -> **207/243 (85%)**, 36
  points unmapped. The rounded percentage **stays** at 85 (85.19 rounds down) —
  recomputed, not assumed.

#### The note was accurate this time, and that was checked

The two points before this one — `LEX-37` and `LEX-47` — both had notes that
overstated their gaps, so this one was token-swept **before** anything was
written. **തുണി** really is the only textile word taught (`ML-C46-cloth`), and
**ഷർട്ട്**, **മുണ്ട്**, **സാരി**, **ചെരുപ്പ്**, **ഷൂസ്**, **തൊപ്പി**,
**കണ്ണട**, **വാച്ച്**, **ബാഗ്**, **ഉടുപ്പ്** and **പാവാട** all return **zero**.

The null result is recorded because two stale notes in a row made it worth
checking, not because it was in doubt.

#### No new grammar at all, deliberately

Chapter 99 spent itself on three *contrasting* joins. This one spends nothing:
only **two** endings appear across its four words, and the learner owns both.

| ends in | word | joined to **ആണ്** |
|---|---|---|
| half-u | **മുണ്ട്** · **ചെരുപ്പ്** | **മുണ്ടാണ്** · **ചെരുപ്പാണ്** |
| **ഇ** | **സാരി** · **തൊപ്പി** | **സാരിയാണ്** · **തൊപ്പിയാണ്** |

**Review caught the provenance wrong in the first draft**, which said the half-u
join *"arrived with the song word"* and the **യ** *"with the game and the beauty
words"*. It did not: **എന്താണ്** takes the half-u at sequence **130** and
**കുട്ടിയാണ്** takes the **യ** at **810**. **പാട്ട്**, **കളി** and **ഭംഗി** are
the most recent words to *use* those joins, not the words they began with — and
`ML-C98-sinima` and `ML-C99-paattu` both say so already. **This is the same
defect chapter 98 recorded fixing** — *"kali said 'two words now do this' when
kuttiyaaNu has done it since sequence 810"* — returned one chapter later in the
same shape. `ML-C100-mundu`'s narrow phrasing, *"the same join **പാട്ട്**
takes"*, was the correct model all along.

The frame is `ML-C42`'s attributive adjective — **പുതിയ** and **പഴയ** in front
and unchanged, which is that chapter's whole pattern applied to nouns it never
had.

**Two limbs are properly covered and the third is thin**, and review was right
to press on it. **മുണ്ട്**/**സാരി** are clothing and **ചെരുപ്പ്** is footwear,
but **തൊപ്പി** is *headwear* — which this chapter's own candidate list treats as
separate from the accessories **കണ്ണട**, **വാച്ച്**, **ബാഗ്**. So **തൊപ്പി** is
nearer a fourth clothing word than an accessory, and the accessory limb stays
thin. The point closes anyway, on the basis `LEX-37` closed with its free-time
limb open: the exponents that exist are real, and the gap is written down rather
than papered over. **ഷർട്ട്** was dropped because it adds no limb at all.

#### The glyph pre-check failed twice, in two different ways, and is now only a signal

- In chapter 99 its **control had rotted** — the remembered control no longer
  discriminated, so it had to be re-derived from the data.
- Here it produced **false negatives**: it refused **ഷർട്ട്** over the chillu
  **ർ** and *"to wear"* over **ധ**, both of which already sit in taught
  headwords — **ten** and **seven** of them respectively.

The check harvests glyphs from `ML-S*` *headwords*, which is **stricter than the
rule it stands in for**. That is not a gate. It is now used only to say what
deserves a second look, and the second look is a grep of the real headwords.

#### Every romanization derived letter by letter — and the counts offered as proof were wrong

**The first draft of this section gave counts instead of names** — *"sixteen
headwords"*, *"the lone `chaaya`"*, *"six against two"*. Review checked all
three and every one was wrong: **ച** → `c` on **30** headwords with **five**
`ch` sites, **ഴ** → `ḻ` on **12** against **six**. They were offered as
*evidence of* the letter-by-letter discipline, in the same commit whose own
changelog says **name lessons, never count them**. The rule existed, and the
sentence claiming rigour is the one that broke it. Names below, no numbers.

The correction chapter 99 earned by getting **നൃത്തം**'s vowel right and the
consonant beside it wrong. Word-by-word confidence is what produced `nṛtthaṁ`.

- **ച** → `c`, witnessed by `cintikkuka`, `cevi`, `ceṟiya`, `cuṇṭŭ`, `cūlŭ`,
  `cēcci`, `cērkkuka` and `ML-S111`'s own `ca`; the legacy `ch` spellings are
  `chaaya`, `nenchŭ`, `uchakaḻiññ`, `acchan` and `muttacchan`.
- **ഴ** → `ḻ`, witnessed by `paḻaya`, `paḻaṁ`, `vaḻi` and `tāḻe`, against
  `vyāzham`, `mazhakkālaṁ`, `ezhu` and `ML-S118`'s `llla`.
- **ണ്ട്** → `ṇṭŭ`, witnessed exactly by **ചുണ്ട്** → `cuṇṭŭ`.
- `sāri` is witnessed **inside** `saṁsārikkuka`'s own romanization.
- `putiya` and `paḻaya` were **read off `ML-C42`'s own fields**, not
  reconstructed — and the reconstruction happened to agree, which is not the
  same as knowing.

#### One claim narrowed before it shipped

`ML-C100-cheruppu`'s gloss said the book *"has had feet since the body chapter"*.
`ML-C47-leg` glosses **കാൽ** as *"leg, foot"*, so the claim was defensible — but
it was replaced with one that needs no defending. The cheaper sentence is the
better one when the flourish adds nothing.
