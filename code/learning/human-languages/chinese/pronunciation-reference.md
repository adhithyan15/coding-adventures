# Mandarin Pronunciation Reference

Reference material, never a gate. Every lesson teaches the sounds its own word
needs, inline, and links here for anyone who wants the fuller picture. Nobody has
to read this page before Lesson 1, and most readers never will.

The ids in each heading are the values a lesson's `sounds:` frontmatter list
points at.

## Why this page is different from every other track's

In an alphabetic or abugida track, "the sounds you'll need" is a list of
*segments*: this letter is silent, that vowel is long, this consonant is
retroflex. Each fact is attached to a letter, and the letter is on the page.

Mandarin adds a second, orthogonally different kind of fact. **Tone** rides on a
whole syllable, is not written in the characters at all, and is *phonemic* — it
distinguishes words the way a consonant does. A segmental note cannot express it.
So this page has two halves: tones first, because they are the part an English
speaker has no instinct for, then the segments.

The machine-readable form of the first half lives in the
[Chinese script reference](../data/scripts/chinese.json) under `tones` and
`toneSandhi`.

## Tones

Mandarin has four tones plus a neutral one. Pinyin writes the tone as a mark over
the vowel; the characters do not record it anywhere.

The classic demonstration uses one syllable, *ma*:

- `tone-1` — first, high level (contour 55). Held high and flat, one steady
  note. *mā* — mother.
- `tone-2` — second, rising (35). Climbs from mid to high, like the pitch on
  a surprised English "huh?". *má* — hemp.
- `tone-3` — third, dipping (214). Sinks low and creaky; said alone it rises
  again a little at the end, but in running speech it usually just stays low.
  *mǎ* — horse.
- `tone-4` — fourth, falling (51). Drops sharply from high to low, like a
  clipped "No!". *mà* — to scold.
- `tone-neutral` — unstressed. Short and light, with no mark written at all,
  pitched by whatever came before it. *ma* — a question particle.

### tone-lexical — tone is part of the word

The single fact that matters most, taught in the first lesson: changing the pitch
of a Mandarin syllable changes **which word it is**, not how you feel about it.
*mā*, *má*, *mǎ* and *mà* are four separate words. English uses pitch for mood
and emphasis only, so this is a genuinely new job for an English speaker's voice.

### tone-sandhi-third — two third tones in a row

**When a third tone is followed by another third tone, the first is spoken as a
rising second tone.**

Written **nǐ hǎo**, said **ní hǎo**. Neither the pinyin nor the characters record
the change; dictionaries print the citation tone each word carries alone. This is
why a reader who trusts only the page mispronounces the commonest greeting in the
language.

### tone-sandhi-bu — 不 before a fourth tone

不 *bù* is said *bú* when the next syllable has a fourth tone: *bù shì* → *bú
shì*. Taught in full on 不是, the first collocation in the track that forces it.

## Segments

Pinyin is a romanization, not English spelling. A few letters do work you would
not guess.

- `pinyin-n` — as in English *no*.
- `pinyin-i` — after most consonants, the *ee* of "see". (After *z-, c-, s-,
  zh-, ch-, sh-, r-* it is a different, buzzier vowel; no Chapter 1 word uses
  those.)
- `pinyin-h` — rougher than English *h*, scraped at the back of the mouth,
  closer to the *ch* of Scottish *loch* softened.
- `pinyin-ao` — one gliding vowel, the *ow* of "how". Not two syllables.

## The characters themselves

Stroke shapes, components, and stroke order for every character this track has
introduced live in
[Chinese script reference](../data/scripts/chinese.json). Unlike the Indic
scripts in this curriculum, where stroke order is conventional, Chinese stroke
order is a **taught, standardised system** — top before bottom, left before
right, horizontal before vertical, outside before inside, and close a box last.
That file marks its stroke orders `authoritative` for exactly that reason.
