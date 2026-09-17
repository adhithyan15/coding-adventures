## HL-C389 — the attention-getter is already in front of the reader twelve chapters earlier

**सुनिए needs a lesson. It does not need one at chapter 105.**

Chapter 105 drafted `HI-C97-sunie` to teach **सुनिए** as the thing a Hindi
speaker says where English says *excuse me* — the opening for a stranger whose
name you do not have. `forward-language` went 22 → 23 and named the reason:

```
HI-C85-sintesis-suchna  word=सुनिए  taughtBy=HI-C97-sunie  lessonsEarly=83
```

Chapter 93 already prints **कृपया सुनिए** in a reading block of public notices,
and it is not a lump there: that chapter teaches the respectful **-iye** ending
as a productive rule, so the reader assembles the form from **सुनना** the moment
they see it. Arriving twelve chapters later to introduce it as a lexical item
teaches backwards.

### What is actually missing

Not the form. **The sense.** *Please listen* and *excuse me* are the same five
letters doing two different jobs, and only one of them falls out of the
imperative rule. A reader who has done chapter 93 can build the notice and would
still not know to say it to a stranger at a counter — nor that it does **not**
carry the apology English *excuse me* also carries, which is the part most
likely to be got wrong in the mouth.

### Where it goes

Beside chapter 93, in the imperative chapter's own neighbourhood, as a lesson
requiring `HI-LEX-C85-IMP-02` (the respectful ending) and `HI-LEX-LISTEN`
(chapter 43's सुनना), introducing a sense rather than a word.

### Two cautions for whoever picks this up

**Adding a lesson to an existing chapter is not the same as adding one at the
end.** Chapter 93's payoff must still assess at least half that chapter's
introduced atoms, so one more introduced atom can push it under the floor. Check
`payoff-surprise` in the snapshot diff before and after, not just the gates.

**Do not spell the ending out in Devanagari.** The withdrawn draft wrote the
suffix as **-इए** in prose, which puts the independent vowel **इ** in front of a
reader for whom no lesson has drawn it — `scriptClosureViolations` 20 → 21. Name
the ending in romanization, or show it attached to a stem.

### What the withdrawn draft contained

A `type: phrase` lesson, headword **सुनिए**, glossed as *excuse me*. Three
claims, all of which survive the move:

- it is **सुनना** wearing the respectful command ending, so *please listen* word
  for word;
- a table pairing *you have a name* with *you have none*;
- **it does not mean sorry** — English *excuse me* does two jobs, getting
  attention and apologising, and Hindi splits them, so reaching for **सुनिए** to
  apologise would puzzle the person hearing it.

That last claim is the one worth keeping, and it is the one a productive
imperative rule cannot give the reader on its own.
