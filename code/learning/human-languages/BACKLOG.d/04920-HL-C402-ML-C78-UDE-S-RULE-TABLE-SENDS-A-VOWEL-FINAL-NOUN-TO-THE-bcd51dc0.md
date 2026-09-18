## HL-C402 — ML-C78-ude's rule table sends a vowel-final noun to the wrong column, and പശു refutes it twice

Found in the re-review of Malayalam chapter 98. Chapter 98 narrowed eight
lessons that attributed a **യ** glide to vowel-hood rather than to a particular
vowel. `ML-C78-ude` carries the same defect, but it is **not a one-clause fix**
like the others, so it is filed rather than patched.

### The claim

`ML-C78-ude` (sequence 2700), lines 48-54:

> **A noun that ends in a vowel takes -യുടെ.** അമ്മ ends in a vowel, so that is
> the ending it gets. അധ്യാപകൻ ends in a consonant, so it took the other one.
>
> | noun ends in | ending |
> |---|---|
> | a consonant | **-ന്റെ** |
> | a vowel | **-യുടെ** |

### What breaks it

**പശു** — `ML-C60-cow`, sequence **1890**, so the learner has it 800 points
before this lesson. Its genitive is **പശുവിന്റെ**.

That refutes the table **twice over**:

1. The glide is **വ**, not **യ** — the same split chapter 98 teaches, where
   **ഇ** and **അ** take **യ** and a full **ഉ** takes **വ**.
2. **പശു ends in a vowel and lands in the `-ന്റെ` column**, which the table
   reserves for consonant-final nouns. So the row is wrong about the *ending*,
   not only about the glide.

A learner reading the table predicts ✗**പശുയുടെ**.

### Why this one is not a one-clause narrowing

The other eight sites state a glide rule, and naming the vowel fixes them.
This one states an **ending-selection** rule, and the real determinant is not
phonology alone: **-ഉടെ** and **-ന്റെ** divide partly on animacy and
rationality, which is a distinction `ML-C87-animacy` (3050) begins to draw
*after* this lesson. Patching the table with a third row would either state
that distinction 350 points before the book introduces it, or replace one
over-simplification with another.

The two candidate treatments both need thought this chapter did not have room
for:

- **Narrow the scope.** Say the table covers the nouns taught so far and name
  them, rather than stating it of any noun. Cheap, honest, and leaves the
  learner without a generative rule.
- **Move the split.** Teach `-ഉടെ` against `-ന്റെ` after `ML-C87-animacy`, so
  the real determinant is available. A structural change to the genitive
  chapter.

### The related sites, for context

Chapter 98 narrowed these; each now names the vowel rather than vowel-hood:

| lesson | seq | what it said |
|---|---|---|
| `ML-C70-um` | 2390 | *"a noun ending in a vowel → a **യ്** slides in"* |
| `ML-C70-um-more` | 2400 | the same, of the **-ഉം** ending |
| `ML-C70-o` | 2410 | *"both end in a vowel"* — and its **ചായ** ends in **അ**, which a first draft of the fix mislabelled **ആ** |
| `ML-C83-keralathil` | 2900 | *"**ഇന്ത്യ** ends in a vowel and takes a **-യ-**"* |
| `ML-C87-animacy` | 3050 | *"a **-യ-** joins, because the word ends in a vowel"* |
| `ML-C91-kada` | 3200 | *"joining with a **-യ-** because the word ends in a vowel"* |
| `ML-C97-bhangi` | 3450 | body **and** Wrap-up |
| `ML-R97-beauty-recall` | 3460 | its Grammar Lens |

**Nine sites in total, and `ML-C78-ude` is the ninth.** Every one was
*latent-false*: consistent with every token in the repository, because
**പശു** is the only **ഉ**-final noun taught and none of its oblique forms
(**പശുവിന്റെ**, **പശുവിൽ**, **പശുവിനെ**) appears anywhere in the corpus. No
gate can see any of them.

Same family as `HL-C400` and `HL-C401`: an early lesson stating a universal
that a later lesson breaks, with nothing automated able to notice.
