## HL-C406 — ML-C10-azhcha prints a Malayalam word in Tamil letters, and glosses *week* as *day*

**Status: OPEN, filed rather than patched.** Found while writing chapter 102,
which uses **തിങ്കളാഴ്ച** from this lesson's table.

**The sentence**, at `ML-C10-azhcha.md:61`:

> Where Tamil says **கிழமை** (*kizhamai*) for "day," Malayalam says **ஆழ்ச**
> (*āzhcha*) — a different word for the same idea …

**Two separate defects in one clause.**

**One: the Malayalam word is spelled in Tamil script.** **ஆழ்ச** is
U+0B86 U+0BB4 U+0BCD U+0B9A — Tamil letters, in a sentence whose whole point is
that this is the word *Malayalam* uses. The Malayalam spelling is **ആഴ്ച**. The
lesson's own `etymology_hook` gives it correctly in romanization (*āzhcha*), so
the error is confined to this one script rendering.

**Two: the gloss.** **ആഴ്ച** means **week**. Tamil **கிழமை** means *day of the
week*. Calling them "a different word for the same idea" where the idea has been
named as "day" is loose in a lesson that is otherwise careful about exactly this
kind of cousin-language comparison.

**Why this is filed and not fixed here.** The obvious repair — write **ആഴ്ച** in
Malayalam letters — introduces a Malayalam token into chapter 10 that **no
lesson owns**: **ആഴ്ച** is a headword nowhere in the corpus and appears only in
this lesson's romanized etymology hook. Under this repo's own rules that creates
script debt at sequence 380, which is not a thing to do as a side effect of a
chapter about telling the time. The alternatives are to drop the script spelling
and leave the romanization (symmetrical with nothing else in the sentence, but
free of debt), or to give **ആഴ്ച** an owner — and that is a decision about
chapter 10, made by somebody looking at chapter 10.

**What closing this needs.** Decide the ownership question first, then fix the
script and the gloss together. Check the same sentence's claim that the two
words "aren't confirmed cognates" survives whatever source is used; it is
already hedged, and the hedge looks right.

**Related.** `HL-C405` records that chapter 10 also uses the corpus's older
**zh** and **th** romanizations. Chapter 102 deliberately copies them for
**തിങ്കളാഴ്ച** so a learner meets one spelling twice. Whoever takes either of
these should read the other first.
