---
category: Repo policy / workflow reminders
---

# Appearing in a field is not being taught, and fixing which field you look at does not fix that

A metric credited a Devanagari glyph as *taught* whenever a script lesson's
**body** contained it. Letter lessons end with example words full of other
letters, so the body-based count understated real script debt roughly threefold.

The fix looked obvious: build the taught set from the lesson's **headword**
instead. That helper was load-bearing across four changes.

**It has the identical bug.** A script lesson's headword is not always a glyph —
sometimes it is a whole word. A lesson about the head-line has the headword
*śirorekhā*, and was silently teaching **श** and **ो**. A lesson about the full
stop has the headword *pūrṇ virām*, and was teaching **व** and **ू**. A
form-filling lesson whose headword contains a person's name was teaching **ण**.

Every "undrawn goes N → M" figure reported off that helper was too optimistic,
always in the same direction, and it nearly shipped a **false exam-point
closure** — a consonant series that looked complete with two letters left, when
five were missing and three of them are in everyday words.

**Both versions asked the same wrong question.** *Does the glyph appear in field
X?* The real question is *is the glyph what this lesson is about?* Moving from
one field to another kept the shape of the error intact, which is why the second
version felt like a fix and was not.

**The tell:** when you correct a measurement by pointing it at a different
field, ask what that field is allowed to contain. If it can contain something
larger than the thing you are counting, you have moved the bug, not fixed it.

The rule that finally worked names the *shape* rather than the field: a glyph is
taught when the headword is a **glyph inventory** — every token at most a base
plus one combining mark. That admits a mātrā or a row of dotted letters, and
rejects any actual word.
