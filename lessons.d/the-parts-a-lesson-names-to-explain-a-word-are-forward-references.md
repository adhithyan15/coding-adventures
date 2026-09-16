---
category: Repo policy / workflow reminders
---

# The parts a lesson names to explain a word are forward references too, not only its headword

Hindi chapter 92 taught the possessive **hamaaraa**, "our", and explained it the
way this corpus explains everything -- by naming its parts: *built on haM, "we"*.
To show that, the lesson printed **haM**.

**haM** was not taught anywhere. Chapter 94, thirteen lessons later, introduced
it. `forward-language` went 11 to 12, and the entry points at chapter 92 using a
word chapter 94 teaches.

This is a THIRD distinct way to create a forward reference, and the first two
were both about the headword:

- HL-C378: an English word in prose that is spelled like a target-language
  headword (`actor`). A false positive.
- HL-C379: two target-language words spelled alike (`coma` the comma, `coma`
  the imperative). Also a false positive.
- This one: the lesson's own **etymological gloss** naming an untaught component.
  **Real**, and caused by the lesson that names it.

The check added after the 143-entry incident was "count the standalone
occurrences of your HEADWORD in earlier lessons." That check would not have
caught this, because the problem is not in the headword at all -- it is in the
body, in exactly the sentence that makes the lesson good teaching.

The honest verdict is an ordering mistake: the pronoun should have been taught
before the possessive built on it. By the time it was noticed, chapter 92 was
merged, so the cost was paid rather than avoided, and the later lesson turned it
into a callback instead.

What to do differently. When a lesson explains a word by taking it apart, treat
every named component as something that must already be taught -- check the
PARTS, not only the headword. And when a chapter's subject is derived from a
more basic word (a possessive from a pronoun, an agent noun from a verb), teach
the basic one first; the derivation then reads as a payoff rather than as a
promise.
