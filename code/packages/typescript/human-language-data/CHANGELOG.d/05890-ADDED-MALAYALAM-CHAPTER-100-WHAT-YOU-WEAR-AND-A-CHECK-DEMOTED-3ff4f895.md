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

| ends in | word | joined to **ആണ്** | where the join came from |
|---|---|---|---|
| half-u | **മുണ്ട്** · **ചെരുപ്പ്** | **മുണ്ടാണ്** · **ചെരുപ്പാണ്** | **പാട്ട്**, ch99 |
| **ഇ** | **സാരി** · **തൊപ്പി** | **സാരിയാണ്** · **തൊപ്പിയാണ്** | **കളി**, **ഭംഗി** |

The frame is `ML-C42`'s attributive adjective — **പുതിയ** and **പഴയ** in front
and unchanged, which is that chapter's whole pattern applied to nouns it never
had. All three limbs of the label get an exponent: **മുണ്ട്**/**സാരി**
clothing, **ചെരുപ്പ്** footwear, **തൊപ്പി** accessories. **ഷർട്ട്** was dropped
on **scope** — a fifth clothing word adds no limb.

#### The glyph pre-check failed twice, in two different ways, and is now only a signal

- In chapter 99 its **control had rotted** — the remembered control no longer
  discriminated, so it had to be re-derived from the data.
- Here it produced **false negatives**: it refused **ഷർട്ട്** over the chillu
  **ർ** and *"to wear"* over **ധ**, both of which already sit in taught
  headwords — **ten** and **seven** of them respectively.

The check harvests glyphs from `ML-S*` *headwords*, which is **stricter than the
rule it stands in for**. That is not a gate. It is now used only to say what
deserves a second look, and the second look is a grep of the real headwords.

#### Every romanization derived letter by letter

The correction chapter 99 earned by getting **നൃത്തം**'s vowel right and the
consonant beside it wrong. Word-by-word confidence is what produced `nṛtthaṁ`.

- **ച** → `c`, on sixteen headwords (`cintikkuka`, `cevi`, `ceṟiya`, `cuṇṭŭ`,
  `cūlŭ`, `cēcci`, `cērkkuka`…) plus `ML-S111`'s own `ca`. The lone `chaaya`
  also lacks its macrons — unmigrated drift, not a convention.
- **ഴ** → `ḻ` on six (`paḻaya`, `paḻaṁ`, `vaḻi`, `tāḻe`, `puḻa`, `ēḻāṁ`) against
  two legacy spellings.
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
