## HL-C401 — two lessons say a chillu unfurls when anything follows it, and the corpus keeps eight words where it does not

Found in the fourth review round on Malayalam chapter 97, which had inherited the
same wording. Chapter 97 is fixed, and **`ML-C87-accusative` was fixed with it** because chapter
97 cites it by name. **`ML-C70-um-more` was not**, and was fixed on the chapter-98 branch — that
chapter narrowed the same lesson's *ya*-glide over-generalisation, so the chillu
one was closed in the same place rather than left to rot. **All three sites are
now fixed and this item is closed.**

### The claim

`ML-C70-um-more` line 68 and its Wrap-up:

> **ൽ** is the compressed, end-of-word form of **ല**; **putting anything after it
> gives the full letter back**, because it is no longer at the end.
>
> What happens to **ൽ** when **something follows it**? (*It unfurls back to* **ല**.)

### What breaks it

A chillu reverts **only before a vowel**, because a chillu cannot carry a vowel
sign. Before a consonant it stays. Eight words already in the learner's
vocabulary keep theirs:

### The sites

| lesson | wording | status |
|---|---|---|
| `ML-C97-alla-sundaram` | inherited it | **fixed** |
| `ML-C87-accusative` | *"the chillu is the shape a consonant takes at the end of a word, and here it is no longer at the end"* — predicts ✗*നിങ്ങളിക്ക് | **fixed**, because chapter 97 points the learner at it by name |
| `ML-C70-um-more` | *"putting anything after it gives the full letter back"*, and its Wrap-up drills it | **fixed** on the chapter-98 branch: body, Guided Practice and Wrap-up all now turn on the vowel, with നിങ്ങൾക്ക് (owned since `ML-C19-vayassu`, well before this lesson) as the counter-example |

### The words that refute it

| word | where |
|---|---|
| **നിങ്ങൾക്ക്** | `ML-C19-vayassu` headword — a case ending landing straight on the chillu |
| **കേൾക്കൂ** | `ML-C44-listen` |
| **വേനൽക്കാലം** | `ML-C14-kaalangal`, which decomposes it as വേനൽ + കാലം |
| **നാൽപത്** | `ML-C88-tens` |
| **ഭർത്താവ്**, **ശർക്കര**, **കർഷകൻ**, **വിദ്യാർത്ഥി** | C45, C61, C48, C48 |

**നിങ്ങൾക്ക് is the decisive one**: it is the same operation as പാൽ + ഉം →
പാലും — a suffix landing on a chillu-final word — and the chillu survives,
because **ക്ക്** begins with a consonant.

### Why it matters

The corpus is *consistent in wording* and consistently wrong here. A learner who
takes `ML-C70-um-more` at its word will expect ✗*നിങ്ങളിക്ക് or ✗*കേളിക്കൂ, and
the words that refute it are ones they already say.

### Fix

Narrow both sentences to the vowel condition, and give the reason, which is the
part that makes it memorable: **a chillu has no way to carry a vowel, so a vowel
forces the full letter back; a consonant does not.** Chapter 97 and ML-C87 now word it that way; chapter 97 cites നിങ്ങൾക്ക് as the
counter-example.

Same family as `HL-C400`: an early lesson stating a universal that later lessons
break, with no gate able to see it.
