### Added — Malayalam chapter 98, games and films, and the vowel that chooses the glide

- `ML-A1-LEX-37` closes. Malayalam A1 coverage 204/243 -> **205/243 (84%)**, 38
  points unmapped.
- **The exam point's own note was overstated, and the correction is on the
  record rather than quietly dropped.** It said *"not one leisure word is taught
  in 292 lessons."* **വായിക്കുക** (ch33), **കാണുക** (ch32) and **പുസ്തകം**
  (ch46) are all owned headwords, and the lesson count had rotted by well over a
  hundred files. The real gap was **sport, games and shows**, and that is what
  the chapter closes.
- **Free time is deliberately left open.** **ഒഴിവുസമയം** would be a compound of
  an unowned part — **സമയം** is not taught — and building it would smuggle in a
  headword nothing owns. The limb stays open with the reason written down.

#### The chapter

| lesson | what it does |
|---|---|
| `ML-C98-kali` | **കളി**, a game. Ends in **-ഇ** exactly as **ഭംഗി** does, so the **ആണ്** weld is one the learner already built. |
| `ML-C98-kalikkuka` | **കളിക്കുക**, to play — the noun visible inside the verb, on an ending seven owned verbs already carry. |
| `ML-C98-sinima` | **സിനിമ**, a film, and the first vowel-final word in the book to weld to **ആണ്** without ending in **-ഇ**. |
| `ML-R98-leisure-recall` | cold retrieval of the three, and of what actually decides the glide. |

#### The generalising sentence was again where the error lived

- Three chapters running, the defect has been in the one sentence that
  summarises. Here the draft said, of **സിനിമയാണ്**, that the vowel at the end
  is not the same in all three words *"and the **യ** comes anyway."*
- Read as a rule that is **a vowel brings യ**, and it is **false of a word the
  learner has had since chapter 60**: **പശു** gives **പശുവാണ്**, with **വ**.
- **Nothing in the corpus could have caught it.** No **വ** glide *after a vowel*
  is attested anywhere — the corpus's vowel-final welds are **ഭംഗിയാണ്**,
  **കുട്ടിയാണ്**, **കളിയാണ്**, all after **-ഇ**. The claim was *latent-false*:
  consistent with every line in the repository and wrong about the language. It
  surfaced only by asking what a **taught word** that the chapter did not
  mention would do.

#### And the replacement was latent-false in its turn

- *"The last vowel chooses, and **ഉ** takes **വ**"* is refuted by **എന്ത്** +
  **ആണ്** = **എന്താണ്**, taught at sequence **130** — and by **ഇത്** standing
  un-glided inside the sinima lesson's **own example sentence**,
  **ഇത് നല്ല സിനിമയാണ്**.
- The reason it counts: `ML-C69-words` (sequence 2360) teaches word-final
  chandrakkala as **a faint half-u**, *"breathed rather than said"*, and the
  track romanizes accordingly — `itŭ`, `entŭ`. So by the book's own account
  those words **are** vowel-final, they end in a *u*, and they take neither
  **യ** nor **വ**: the half-u simply steps out of the way.
- A learner holding the round-one fix would predict ✗**എന്തുവാണ്** and
  ✗**ഇതുവാണ്**. The rule is now stated of **full** vowels, and the half-u gets
  its own paragraph and its own recall question — which turns the chapter's
  worst claim into one of its better rows.
- **Same species as the error the chapter was written to fix, one layer down.**
  Narrowing a false generalisation produced a second one that was, again,
  consistent with every token in the repository.

#### Five merged lessons narrowed with it — not the three first counted

Stating the narrow rule in chapter 98 would have contradicted lessons already in
the book, which is the stale-cross-reference failure chapter 97 hit in its fifth
review round. Each was narrowed in the same commit, to name the environment
rather than the resemblance:

- `ML-C97-bhangi` — *"It ends in a vowel ... so a **യ** slides in"* → it ends in
  **ഇ**, a letter slides in, and **after ഇ that letter is യ**. Its **Wrap-up**
  still credited the **യ** to vowel-hood after the body had been narrowed, and
  the drill is what a learner actually rehearses, so it was fixed too.
- `ML-R97-beauty-recall` — the same attribution in its Grammar Lens.
- `ML-C70-um-more` — *"a noun ending in a **vowel** → a **യ്** slides in"*, said
  of the **-ഉം** ending, which splits identically.
- `ML-C70-um` (2390) and `ML-C70-o` (2410) — **missed on the first pass**, and
  they sit on either side of the lesson that was fixed. `ML-C70-o`'s **ചായ** is
  an **ആ**-final word taking **യ**, so it was live evidence for the new narrow
  rule while being written up as evidence for the broad one.

#### Caught before review, by counting instead of trusting

- `ML-C98-kalikkuka` claimed *"eight verbs in this book already end that way."* A
  token sweep of every headword found **seven** **-ിക്കുക** verbs, and the count
  would rot on the next verb regardless — so the number was replaced by four
  named ones: **സംസാരിക്കുക**, **വായിക്കുക**, **ചോദിക്കുക**, **കുടിക്കുക**.
- `ML-C98-kali` claimed **കളി** was *"the first word in the book for something
  done for its own sake"* — a claim no reading of the corpus could settle — and
  that *"the language had no way to name one"*, which confuses the book with
  Malayalam. Both narrowed to **the book's first word for play**.
- `ML-C98-kali` also said *"**two words** now do this"* of the **യ** weld.
  **കുട്ടിയാണ്** has done it since sequence **810**, and chapter 97 cites it
  twice by name — so the count was three, and it was a rotting count of exactly
  the kind removed from `kalikkuka` two bullets up. It now names **കുട്ടി**
  instead of counting.
- `ML-C98-kali`'s Warm-up asked the learner to say *"this is beauty"* — the
  exact gloss `ML-C97-bhangi` tells them is **wrong** twenty sequence-points
  earlier (**ഇത് ഭംഗിയാണ്** is *this is beautiful*). Now *"this is beautiful"*.
- Both new Wrap-ups asked the **unqualified** question — *"what slides in before
  **ആണ്**?"*, answer *"**യ**"* — which is the broad rule the chapter exists to
  undo, drilled by the chapter itself. Each now names the vowel in the question.
- **പശുവാണ്** was never romanized in learner-facing text, so the audio learner
  met the chapter's one counter-example as bare script. Every joined form in
  both tables now carries its romanization, `paśuvāṇŭ` included.
