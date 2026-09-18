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

### RESOLVED

**Closed on the HL-C400 branch.** The fix is the one this shard proposed, and it
held: all four sites now say **a case ending** where they said *anything*.

| lesson | now reads |
|---|---|
| `ML-C83-keralathil` | *"the moment **a case ending** is added"*, plus one added sentence: *"Other kinds of ending do other things to it, and you will meet them in their own chapters."* Its recall asks *"when a case ending is added"*. |
| `ML-R83-from-recall` | *"dropped the moment **a case ending** is added"* |
| `ML-C91-vila-panam` | body and recall both say *"when **a case ending** attaches"* |
| `ML-C92-bhakshanam` | it **demonstrated nothing** — it only asserted *"you already know how it will behave when anything attaches"*. It now names what it knows: *"you already know what a **case ending** will do to it — the **ം** gives way to **-ത്ത-**, as it did on **കേരളം** and **പുസ്തകം**."* |

**"Case ending" is not new metalanguage**: `ML-C06-dative-ikku` uses it at
sequence **320**, long before any of these sites, and `ML-C96-eluppam` (3410)
already models the exact phrasing — *"A case ending takes the **ം** away and
puts **ത്ത** in its place."*

**The sweep returned exactly these four canonical lessons and nine lines**, and
the recall partners `ML-R91-shopping-recall` and `ML-R92-eating-out-recall` were
checked and are clean — **null result stated**.

**But the first version of this section claimed "no fifth site" after a sweep
"script and romanization", and that was false.** Review found the identical
sentence still asserted in romanization in `core/exam-inventory-malayalam-a1.json`
(twice) and a now-stale *"still tell the learner"* record in
`malayalam/chapters.d/0096.json`. All three are corrected in the same commit.
The claim now says what it can support: four canonical lessons.

**The four-way picture is deliberately not taught in chapter 83** — but not for
the reason first given. The first version said *"the learner has met one of the
four at that point"*, and that is wrong: they have met **two**. `ML-C70-um`
(2390) teaches the **-ം → -വും** rule outright, 510 points earlier. So
`ML-C83` now **names** that one — *"You have met one already: the and ending
puts **വും** there instead"* — rather than promising it as something still to
come. The other two (**ങ്ങൾ** at `ML-C93-angal`, **ആണ്**/**ഇല്ല** at
`ML-C89`/`ML-C96`) genuinely do come later.

**And the narrowing introduced a false claim of its own**, caught in review:
`ML-C91-vila-panam` said the **ം** gives way *"exactly as it did on the state
name and on the tens"*. Loose but harmless while the sentence said *"when
something attaches"*; once narrowed to *"a case ending"* it asserted that a case
ending removed a **ം** from the tens — which have **no ം** (they end in **ത്**)
and take **no case ending** (their trigger is a following digit). Now *"on the
state name and on the book"*. Three merged lessons that generalise stem
alternation more broadly are filed separately as `HL-C404`, with a dissent.

### Shape of a fix (as originally proposed)

Narrow the four recall answers to what each lesson actually demonstrates — *"an
ending that marks a case"* rather than *"an ending"*, *"when a case ending
attaches"* rather than *"when anything attaches"*. That is a four-line change and
costs nothing pedagogically, because each lesson only ever shows the case
behaviour.

Do **not** try to teach the full four-way picture in chapter 83; the learner has
met one of the four at that point. The honest fix is to stop the early lessons
claiming more than they show.
