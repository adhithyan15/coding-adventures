## HL-C402 — the genitive's rule table sends a vowel-final noun to the wrong column, and its recall drills the wrong answer

Found in the re-review of Malayalam chapter 98, which narrowed eight lessons
that attributed a **യ** glide to vowel-hood rather than to a particular vowel.
Three lessons carry the same defect **in the genitive**, where it is not a glide
claim but an **ending-selection** claim — so they are filed together rather than
patched with the one-clause narrowing the other eight got.

### The three sites

| lesson | seq | wording |
|---|---|---|
| `ML-C78-ude` | **2700** | *"**A noun that ends in a vowel takes -യുടെ.**"* plus a two-row table: `a consonant → -ന്റെ` / `a vowel → -യുടെ` |
| `ML-R78-genitive-recall` | **2710** | the same table (`\| **അമ്മയുടെ** \| -യുടെ \| a vowel \|`) **and a Wrap-up drill**: *"Which follows a vowel? (**-യുടെ**.)"* |
| `ML-C84-mukalil` | **2920** | Warm-up: *"put the owner-ending on it — the one **a word ending in a vowel takes**"* |

**`ML-R78` is the one to fix first.** It is not a stray clause: it repeats the
table *and* supplies the false answer as a drill. Whoever works this item must
fix the recall in the same pass — fixing `ML-C78` alone leaves the learner
rehearsing the claim, which is exactly the outcome `HL-C401` exists to record,
and exactly what happened to `ML-C97-bhangi`'s Wrap-up in chapter 98's own
round two.

### What breaks all three

**പശു** — `ML-C60-cow`, sequence **1890**, so the learner has it 800 points
before `ML-C78-ude`. Its genitive is **പശുവിന്റെ**.

That refutes the table **twice over**:

1. The glide is **വ**, not **യ** — the same split chapter 98 teaches, where
   **ഇ** and **അ** take **യ** and a full **ഉ** takes **വ**.
2. **പശു ends in a vowel and lands in the `-ന്റെ` column**, which the table
   reserves for consonant-final nouns. So the row is wrong about the *ending*,
   not only about the glide.

A learner reading the table predicts ✗**പശുയുടെ**.

### RESOLVED — and the first attempt at resolving it was wrong in the same way the shard was

**Closed on the chapter-101 branch, at the second attempt.** Both attempts are
recorded, because the first one failed in a way worth keeping.

#### What the shard originally proposed, and why it sat open

A third row — **പശു** → **പശുവിന്റെ** — flagged as *the one claim resting on
morphology rather than on a grep*. That flag kept the item open across chapters
99, 100 and 101.

#### The first attempt: an enumeration keyed on one spelling

I enumerated every genitive in the corpus and concluded that **at sequence 2700
the two-row table was accurate for everything the learner had seen**, with the
falsification arriving later at 3000 and 3210. On that basis the fix was to say
the table is not exhaustive and promise a third landing later.

**That enumeration was keyed on Malayalam script, and the corpus does not keep
all its genitives in script.** Three forms were missed:

| form | stem | stem ends in | ending | first at |
|---|---|---|---|---|
| **എന്റെ** | ഞാൻ (suppletive) | — | **-ന്റെ** | **80** |
| **നിന്റെ** | **നീ** | **a vowel** | **-ന്റെ** | **130** |
| *niṅṅaḷuṭe* | **നിങ്ങൾ** | **chillu — a consonant** | **-ുടെ**, no glide | **130** |

**`niṅṅaḷuṭe` appears in the corpus only in romanization**, so a script-keyed
grep returns zero for it. This repeats
`lessons.d/a-census-keyed-on-one-of-a-construct-s-two-spellings-undercounts.md`
exactly: *"a census keyed on one of a construct's two spellings undercounts
silently, and the number gets published."*

**So the table was never accurate.** `നിന്റെ` breaks the vowel row and
*niṅṅaḷuṭe* breaks the consonant row, both at **sequence 130** — 2,570 points
before `ML-C78-ude`, in a lesson `ML-R78` itself points the learner back to.

#### The second attempt: stop claiming the ending is predictable

Since the two chapter-2 pronouns break both rows, the honest teaching is not
*"here are two landings, more later"* — it is that **which ending a noun takes
comes with the noun**:

- `ML-C78-ude` — the table rows now carry no "because", and the text says
  outright: *"Which of the two a noun takes is learned with the noun, not worked
  out from its last sound — and you have had the proof since your first chapter.
  **നിന്റെ** is the owner-form of **നീ**. **നീ** ends in a vowel, exactly as
  **അമ്മ** does, and it does not take **-യുടെ** at all."* What is kept is the
  part that is reliable: the shape of the phrase.
- `ML-R78-genitive-recall` — its Grammar Lens header **"because it ends in"**
  is gone, and its Wrap-up now asks *"Can you tell which one a new noun will
  take by listening to its end? (**No** — **നീ** ends like **അമ്മ** and takes
  **നിന്റെ**.)"* The first attempt had fixed this file's drill and **left its
  table**, which is the `ML-C97-bhangi` failure mode inside the very lesson this
  shard named as the one to fix first.
- `ML-C84-mukalil` — Warm-up now says *"the one **അമ്മ** took."*

**പശുവിന്റെ is asserted nowhere**, and no unattested form is used. The
counterexamples are **നീ**/**നിന്റെ**, taught in chapter 2 and owned by
`ML-C02-nii-ningal` (110) and `ML-C02-ninre-peru-entaanu` (130).

#### The forward promise was dropped, not kept

The first attempt added *"this book will show you another before long."* No
lesson in the corpus ever presents a third genitive ending **as** a third
ending: `ML-C86-oppam` (3000) folds **-ിന്റെ** into *"the ending consonants
take"*, and `ML-C91-vila-panam` (3210) explains only the **ം → ത്ത** oblique.
A promise the corpus does not honour is worse than no promise.

### The original candidate treatment, kept for the record — and the claim in it that needed checking first

**A third row appears to be enough**, and it needs no forward reference and no
new vocabulary beyond the **full vowel** term chapter 98 established:

| noun ends in | genitive | witness |
|---|---|---|
| **അ**, **ഇ** | **-യുടെ** | അമ്മ → അമ്മയുടെ, കുട്ടി → കുട്ടിയുടെ |
| a full **ഉ** | **-ന്റെ**, on the oblique stem | പശു → **പശുവിന്റെ** |
| a consonant or chillu | **-ന്റെ** | അധ്യാപകൻ → അധ്യാപകന്റെ |

**Do not apply this without verifying the morphology.** It is the one part of
this shard that rests on a claim about Malayalam rather than on a grep of the
corpus, and this item was filed in the first place because an earlier draft of
it got exactly that wrong — see below.

**An earlier draft of this shard justified filing rather than patching by
saying the split turns on animacy and rationality, which `ML-C87-animacy` only
draws 350 points later. That was a category error and is recorded here so it is
not repeated.** `ML-C87-animacy` is the **accusative** lesson — headword
`പുസ്തകം വായിക്കുന്നു`, atom `ML-GRAMMAR-C87-ANIMACY-01`, teaching that a
living thing being acted on takes **-എ** and a lifeless one does not. It has
nothing to do with the genitive. And animacy does not separate the genitive rows
anyway: **അധ്യാപകൻ** is rational and takes **-ന്റെ**, **കുട്ടി** is rational and
takes **-ഉടെ**, **കട** is inanimate and takes **-ഉടെ**. The real conditioner is
the stem's final segment and its oblique form.

### Why these three and not the other eight

Chapter 98 narrowed eight lessons by naming the vowel; each stated a **glide**
rule, and naming the vowel is the whole fix. These three state which **ending**
the noun selects, so naming the vowel is not sufficient on its own — the `ഉ` row
changes column as well as glide.

### All of them are latent-false

**പശു** is the only **ഉ**-final noun the book teaches, and none of its oblique
forms — **പശുവിന്റെ**, **പശുവിൽ**, **പശുവിനെ** — appears anywhere in the
corpus. `grep -l 'പശു' *.md` returns `ML-C60-cow.md` and the chapter-98 files
and nothing else. So no token contradicts any of these lessons and no gate can
see them.

### The sweep rule this item produced

**Whenever `ML-Cnn-x` is narrowed, `ML-Rnn-*` is a site until checked.** Chapter
98 established this on `ML-C97-bhangi` — whose body was narrowed while its
Wrap-up still carried the claim — and then failed to generalise it, so the
recall partners of three more narrowed lessons were missed for another round.
Applying it mechanically would have found `ML-R70-cherkkuka`,
`ML-R78-genitive-recall` and `ML-R87-object-recall` with no new grep at all.

Same family as `HL-C400` and `HL-C401`: an early lesson stating a universal
that a later lesson breaks, with nothing automated able to notice.
