---
category: Repo policy / workflow reminders
---

# A note can be precise about the grammar and wrong about the vocabulary, and the grammar half will convince you the whole note is true

`HI-A1-POST-10`'s note in the Hindi A1 exam inventory read:

> The proxy expresses purpose with *para* plus an infinitive; Hindi uses the
> oblique infinitive plus *ke liye*. **Neither the postposition nor the oblique
> infinitive is taught.**

The first sentence is a correct and non-obvious piece of contrastive grammar.
The second sentence is **half false** — `HI-C84-mere-liye` had taught **मेरे
लिए** nine chapters earlier.

## Why this one is harder to catch than the other two varieties

This campaign has now found three kinds of stale note, and they fail different
checks:

| variety | how it goes wrong | what catches it |
|---|---|---|
| false when written | the claim never held | grep the corpus for the headword (HL-C376) |
| stale by later work | true when written, a later chapter removed the blocker | grep the inventory for the point id you just closed |
| **half true** | **the analysis is right, one clause of the inventory is wrong** | **grep the headword anyway, even when the note reads like expertise** |

The first two are caught by a habit. The third defeats the habit, because the
note **sounds like it was written by someone who checked**. A note that names
the oblique infinitive, names the proxy's construction and contrasts the two
has already demonstrated real knowledge of the language — and that
demonstration is exactly what makes the reader skip the verification step on
the sentence that follows it.

## The rule

**Authority about the language is not evidence about the corpus.** They are
separate claims and they need separate checks. Grep every exponent a note
declares missing, however well-informed the surrounding prose is — especially
then, because a well-informed note is the one you will not think to check.

## What it cost, and what it bought

Nothing, and a chapter. Because half the note was already satisfied, the
remaining work was **one construction rather than two**: `ke liye` after a noun
(which is just the shape with a separate `के`, since a pronoun carries its own)
plus the oblique infinitive. The point turned out to be roughly half the size
it advertised.

That is the general shape of the win. Checking makes points **cheaper**, not
more expensive, and a point that looks too expensive to attempt is the one most
worth grepping before you skip it.
