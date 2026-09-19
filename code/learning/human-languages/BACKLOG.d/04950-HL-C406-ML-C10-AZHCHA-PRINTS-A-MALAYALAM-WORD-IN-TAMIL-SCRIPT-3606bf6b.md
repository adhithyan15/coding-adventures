## HL-C406 — ML-C10-azhcha prints a Malayalam word in Tamil letters, and glosses *week* as *day*

**Status: CLOSED (2026-09-19).** Found while writing chapter 102, which uses
**തിങ്കളാഴ്ച** from this lesson's table.

The lesson now prints Malayalam **ആഴ്ച** in Malayalam script and contrasts the
meanings precisely: Tamil **கிழமை** is "day of the week," while Malayalam
**ആഴ്ച** is "week." The existing non-cognate hedge remains. No standalone
headword was added: this lesson uses **ആഴ്ച** to explain how its seven owned
weekday forms are built, while chapter 102 continues to teach **തിങ്കളാഴ്ച** as
a use assembled from those forms rather than pretending the corpus has gained a
separate word owner.

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

**Why this is filed and not fixed here — and the first version of this
paragraph was wrong.** It argued that writing **ആഴ്ച** in Malayalam letters would
introduce a token into chapter 10 that no lesson owns, because the word
"appears only in this lesson's romanized etymology hook". **That is false, and
this lesson's own frontmatter refutes it.** `ML-C10-azhcha.md:15` already carries
**ആഴ്ച** in Malayalam script — U+0D06 U+0D34 U+0D4D U+0D1A — inside the
`etymology_hook`. The script token is already in the file. Writing it in the body
adds nothing the lesson does not already carry, so **the script half of this is
a one-character-class fix with no debt attached**, and the deferral rationale was
an invention.

What is genuinely a chapter-10 decision is the **gloss**: whether to keep calling
**ആഴ്ച** "a different word for the same idea" where the idea has been named as
*day*, and whether the word wants a headword owner at all now that a later
chapter uses **തിങ്കളാഴ്ച** in a sentence. That is the part worth somebody's
attention, and it is why this stays filed rather than patched mid-chapter — not
the debt story the first draft told.

**What closing this needs.** Decide the ownership question first, then fix the
script and the gloss together. Check the same sentence's claim that the two
words "aren't confirmed cognates" survives whatever source is used; it is
already hedged, and the hedge looks right.

**Related.** `HL-C405` records that chapter 10 also uses the corpus's older
**zh** and **th** romanizations. Chapter 102 deliberately copies them for
**തിങ്കളാഴ്ച** so a learner meets one spelling twice. Whoever takes either of
these should read the other first.
