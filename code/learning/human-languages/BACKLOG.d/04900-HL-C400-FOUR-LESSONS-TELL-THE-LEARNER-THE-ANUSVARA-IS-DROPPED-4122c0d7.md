## HL-C400 — four lessons tell the learner the anusvara is dropped whenever anything attaches, and three other lessons show otherwise

Found while writing Malayalam chapter 96, in the fourth round of review on a
single paragraph. **Not introduced by that chapter** — chapter 96 states the
facts correctly. The fault is that four earlier lessons state a universal the
corpus itself breaks, and nothing flags it.

### The unqualified claims

| lesson | wording |
|---|---|
| `ML-C83-keralathil` | *"the moment anything is added, the word changes into its working shape first"*; recall: *"What happens to **-ം** when an ending is added? (**It is replaced by -ത്ത-**.)"* |
| `ML-R83-from-recall` | *"**The -ം is a citation dress**: what the word wears standing alone, **dropped the moment anything is added**."* |
| `ML-C91-vila-panam` | recall: *"what does the **ം** warn you about? (**It will give way when something attaches**.)"* |
| `ML-C92-bhakshanam` | *"It ends in **ം**, so you already know how it will behave when **anything attaches** to it."* |

### What breaks them

**Four different things happen to a word-final ം, depending on what attaches:**

| attaching thing | outcome | taught at |
|---|---|---|
| a case ending | **ത്ത** — കേരളത്തിൽ, പുസ്തകത്തിന്റെ | `ML-C83`, `ML-C91` |
| the *and* ending | **വും** — വെള്ളവും | `ML-C70-um` |
| the plural | **ങ്ങൾ** — പുസ്തകങ്ങൾ | `ML-C93-angal` |
| **ആണ്**, **ഇല്ല** | **nothing is taken away** — എളുപ്പമാണ്, സാരമില്ല | `ML-C89`, `ML-C96`, `ML-C03` |

The last row falsifies *"dropped the moment anything is added"* outright, and
**സാരമില്ല has been in the book since chapter 3** — so the universal was already
false when it was written. The plural row falsifies *"replaced by -ത്ത-"*, and
that was true from chapter 93, three chapters before this was noticed.

### Why it matters and why nothing caught it

A learner rehearsing `ML-R83` or `ML-C91` from cold is drilled on a universal
that chapters 3, 89, 93 and 96 all break. Chapter 96's framing is **additive**
(*"Here is another thing it says"*) rather than corrective, so it states the true
picture without ever signalling that the earlier phrasing was shorthand.

No gate reads any of this. `validate` checks atom availability, the twelve gates
check structure and wiring, the suite checks pins, glyphs and banned words. **A
recall answer that is a false universal about the language is invisible to all of
them** — the same gap as `HL-C399`, and the same gap that let three successive
false accounts of this very join pass every check.

### Shape of a fix

Narrow the four recall answers to what each lesson actually demonstrates — *"an
ending that marks a case"* rather than *"an ending"*, *"when a case ending
attaches"* rather than *"when anything attaches"*. That is a four-line change and
costs nothing pedagogically, because each lesson only ever shows the case
behaviour.

Do **not** try to teach the full four-way picture in chapter 83; the learner has
met one of the four at that point. The honest fix is to stop the early lessons
claiming more than they show.
