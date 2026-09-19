---
category: Testing & coverage
---

# "Every glyph is already taught" is a claim about codepoints and says nothing about clusters

Choosing Malayalam chapter 107's three words, I checked each character in
**മരുന്ന്**, **ചികിത്സ** and **വൃത്തി** against the script lessons and the
headword census, and wrote in the chapter note that *"every glyph in all three is
already script-taught and already in use in headwords."*

That sentence is true and it is also useless, because the unit it measures is not
the unit the learner reads. **ത** (U+0D24) and **സ** (U+0D38) are both long
taught and both common. The conjunct **ത്സ** — `ത` + U+0D4D + `സ` — occurs in no
headword anywhere in the corpus before this chapter. A per-codepoint sweep
reports "all familiar" for a shape the learner has never seen, and the shape is
what appears on the page.

The same trap sits behind every alphabetic script with conjuncts, ligatures or
contextual forms: Devanagari **क्ष**, Perso-Arabic initial/medial forms, Tamil
grantha clusters. In each the rendered unit is a *sequence*, and a codepoint
census cannot see sequences at all.

**Do this instead.** When a new headword is proposed, census the **sequences**,
not only the characters:

```sh
# every ta-virama-sa in any headword, across the whole track
grep -h '^headword:' code/learning/human-languages/<track>/lessons/*.md \
  | grep 'ത്സ'
```

A zero result is not a reason to refuse the word — it is a reason to teach the
join. Chapter 107 kept **ചികിത്സ** and added a *The word taken apart* block
against the chandrakkala mechanism already taught in chapter 1. What is not
acceptable is letting a codepoint sweep report the cluster as familiar.
