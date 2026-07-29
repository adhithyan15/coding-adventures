# Tamil Roadmap

Same shape as the other tracks: deep one-word lessons in themed chapters.
Slug-identified; order lives in the book and
[`session-map.md`](./session-map.md). See
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md).

Grounding: English + the **Dravidian family** (Kannada, Telugu, Malayalam) +
Sanskrit/Hindi, with Tamil as the native-Dravidian root the sisters trace back
to. Tamil script is taught **inline**, inside the word lessons, never as a
gated reading course. Grammar is introduced piece by piece, on the first word
that needs it.

## Authored

- **Ch. 1 — Greetings**: vaṇakkam → naṉṟi → ām → illai → sari → practice.
  Tamil script introduced through the words (inherent *a*, puḷḷi, vowel signs,
  independent vowels, the three-way *n*/*l*/*r* distinction), and the native
  vs. Sanskrit split shown through the greetings themselves.
- **Ch. 2 — Introducing Yourself**: peyar → eṉ → **eṉ peyar** ("my name is,"
  zero copula) → nī/nīṅgaḷ → eṉṉa → **uṅgaḷ peyar eṉṉa?** → magiḻcci → practice.
  Every atom native Dravidian and traced (*peyar* ≠ *name*); the **zero copula**
  (no word for "is"); respect-by-plural like French *vous*.

- **Ch. 3 — How Are You**: eppaḍi (how; the native *e-* questions) → nīṅgaḷ
  eppaḍi irukkiṟīrgaḷ? (the verb *iru* "to be" — the copula returns for states) →
  nāṉ (I ← Proto-Dravidian, unrelated to *me*) → nalam (well ← *nal-* "good," the
  root of *naṉṟi*) → paravāyillai (you're welcome; the *iru*/*illai* pair) →
  practice.
- **Ch. 4 — Farewells**: pō/vā (go/come) → pōy varugiṟēṉ ("I'll go and come
  back" — the Dravidian promise-of-return goodbye) → nāḷai pārkkalām (see you
  tomorrow) → mīṇḍum sandippōm (we'll meet again; native *mīṇḍum* + Sanskrit
  *sandi*) → practice.
- **Ch. 5 — First Verbs**: pēsu (to speak; stem + tense + person) → nāṉ tamiḻ
  pēsugiṟēṉ (I speak Tamil; the retroflex *ḻ*; no gender in the 1st person) →
  vāḻ (to live/flourish) → vēlai sey (to work; noun + *sey*, the twin of Hindi's
  *karnā*) → practice.

- **Ch. 6 — Case endings, and the sentence with no subject**: **-உக்கு** (*-ukku*,
  "to/for") — the first case ending, taught as the entry point to
  **agglutination**: Tamil **adds** a suffix that carries **one** meaning, keeps
  its shape and leaves the **seam visible** (*peyar* + *ukku*), where a Latin
  ending like *-īs* **fuses** case *and* number *and* declension into one
  indivisible lump. Includes the irregular *nāṉ* → **எனக்கு** *enakku* ("to me") →
  **எனக்குத் தமிழ் தெரியும்** (*enakku tamiḻ teriyum*, "I know Tamil") — literally
  "**to-me Tamil is-known**," with **no nominative "I"** — the person sits in the
  dative instead (a **dative subject**), while the theme *tamiḻ* stays unmarked. The
  **dative-subject** rule: knowing, liking, wanting and being cold *happen to*
  you rather than being done *by* you, so the experiencer takes the dative
  (English keeps one fossil of it in "**methinks**"). Closes on the four-sister
  table — *-ukku / -ku / -ge / -ikku* are visibly the **same suffix**, the
  Dravidian family showing its bones the way *blanc/bianco/branco* did for
  Romance. **Authored.**

### Writing the letters *(authored)* — the "break it apart and write it" strand

The **first handwriting track for any Dravidian language**. Until now
`data/scripts/` held Arabic, Chinese, Cyrillic, Devanagari, Gujarati and Hebrew —
nothing for Tamil, Telugu, Kannada or Malayalam — so four tracks had vocabulary
through Chapter 6 and no way to learn to read it. `tamil.json` is new here.

Lessons follow **book order**, taking the letters the Chapter 1 words actually
need, and rise one piece at a time:

- **`TA-W01`** — **வ, க**. Opens on the question the whole script answers: *why
  is Tamil round?* The usual account is **palm leaves**: incised with a stylus,
  where a straight stroke along the grain can **split the leaf**, so strokes bend
  into curves. The lesson gives that as the standard explanation *rather than a
  settled fact* — earliest Tamil-Brahmi is angular, the rounding arrived later
  via Vaṭṭeḻuttu, and Devanagari used the same leaves without going round. The
  durable point is that **the tool leaves fingerprints on the letters**. Then the
  **abugida** principle (க is *ka*, not *k*), and the fact that **one letter க
  spells k, g and h**, decided by position — which is why Tamil needs 18
  consonant letters where Devanagari needs 33.
- **`TA-W02`** — **ம, ண**, and the **retroflex**: ண is said with the tongue
  curled back, a sound English lacks and cannot hear at first. Introduces the
  three-n table (**ந** dental · **ன** alveolar · **ண** retroflex) without yet
  drawing the other two.
- **`TA-W03`** — the **puḷḷi** ் ("the dot"), which removes the inherent vowel —
  and the sharp divergence from Devanagari: Tamil does **not fuse** the bare
  consonant into a conjunct. Both letters keep their shape and the dot stays
  visible, which is why Tamil's whole character set is ~247 where Devanagari has
  hundreds of ligatures. **Assembles வணக்கம்** — the first word of the course.
- **`TA-W04`** — **ந, ன, ற** and the first vowel sign **ி**, completing the
  three n's. **Assembles நன்றி**, and shows why the three n's earn their keep:
  **ன் + ற** is said together as *ndr* — one instance of the general rule that a
  **nasal voices the stop after it** (ந்த *nd* · ண்ட *ṇḍ* · ன்ற *ndr*), so each
  n produces its own cluster and the spelling tells you which. It is also why
  *naṉṟi* is so often written *nandri* in English.

Next in this strand: the remaining vowel signs (**ை** is written *before* the
consonant and pronounced *after*, the same trap as Devanagari's ि), then the
letters the lessons already quote but do not yet teach — **ப, ள, ு** from
*puḷḷi*'s own name, and ங, ட, த, ர among others. W03 carries a **standing
read-now-draw-later note** for the whole track rather than an enumerated list,
since the list grows with every example.

## Planned

| Chapter | Theme |
|---|---|
| 7 | The rest of the case suffixes — accusative *-ai*, locative *-il* — now that Ch. 6 has established how stacking works |
| 8+ | Tense (past/future), numbers, family, food — always with the Dravidian-cognate thread |

Note: Tamil marks "you" by **register** (*nī* familiar / *nīṅgaḷ* respectful,
also plural) — like the Romance/Germanic tracks, and worth teaching beside
them.
