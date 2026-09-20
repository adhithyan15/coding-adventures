## Chapter 86 — two kinds of with

`ML-A1-CASE-05` closes. Coverage **190/243 → 191/243 (79%)**.

The note named both halves: *"Neither -aal nor oppam is taught, so 'with a
friend' and 'by bus' are both out of reach although suhruthu is taught."* Both
close here, and the chapter's point is that **they are not the same
relationship.**

### English asks one question, Malayalam asks two

*I went **with** a friend.* *I cut it **with** a knife.* One English word, and
**two relationships that have nothing in common** — a companion and a tool.

| | | |
|---|---|---|
| **company** | **എന്റെ ഒപ്പം** / **എന്റെ കൂടെ** | after the owner-form, standing apart |
| **instrument** | **കത്തിയാൽ** | glued onto the noun |

So the useful question is never *how do I say with*. It is **which kind of
*with* this is.**

### The company words cost no new arrangement

**ഒപ്പം** and **കൂടെ** stand after the owner-form — exactly where **മുകളിൽ**
stood after *the chair's*:

| | |
|---|---|
| **കസേരയുടെ മുകളിൽ** | on top of the chair |
| **എന്റെ ഒപ്പം** | together with me |

**The slot does not care whether what follows it is a position or a company.**
That is the return the place-words chapter earned by spending its first lesson
on the pattern.

The pair splits by register the way the degree words did — **ഒപ്പം** to the
page, **കൂടെ** to the room.

### The instrument ending is one the learner already owns

**ആൽ** was taught in the conditional chapter, on a **verb**, meaning *if*. Put
it on a **noun** and it means *by means of*.

| what it lands on | what it does |
|---|---|
| a **verb** — *vannāl* | **if** he comes |
| a **noun** — *kattiyāl* | **by means of** a knife |

**The ending is identical and the meanings are unrelated.** So the lesson teaches
the habit rather than the form: **look at the host before you read the ending.**
One ending doing two unrelated jobs is ordinary here rather than exceptional.

### A glyph the book could not print

The full suite failed on `glyph-coverage`, not on anything a reader would call a
mistake: the romanization of **സുഹൃത്ത്** had been written with **r + U+0325**,
a combining ring below, which **the book font cannot render**.

`ML-C35-suhruthu` already romanizes it **suhṛttŭ**, with **U+1E5B**, a single
precomposed character. Three occurrences corrected to match.

Worth keeping: **the corpus already had the right answer**, and the failure was a
new file spelling a sound its own track had spelled correctly for fifty chapters.
Copy the romanization from the lesson that owns the word rather than typing it
again.

### Verified before writing

**ഒപ്പം** and **കൂടെ** each returned **zero** files. The instrument example uses
**കത്തി**, which is taught — **no vehicle word was invented for "by bus"**, even
though the point's own note mentions it.

### Metrics

| metric | before → after |
|---|---|
| exam-point coverage | 190/243 → **191/243 (79%)** |
| the case column | 5/7 → **6/7** |
| `forwardReferences` | unchanged |

