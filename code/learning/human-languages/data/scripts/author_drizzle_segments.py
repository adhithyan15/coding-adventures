#!/usr/bin/env python3
"""Author Tamil's drizzled script segments: one letter per lesson.

The existing TA-W lessons are word-shaped -- "write வணக்கம்", "read peyar" --
and each puts four to sixteen letters in front of the reader at once. The first
sits at sequence 270. So Tamil had a writing strand, and it had no drizzle.

These nine segments are the drizzle: one letter each, and each placed
immediately BEFORE the word-writing lesson that uses it, so the reader meets a
letter on its own and then meets it inside a word.

Placement is not free. An earlier revision put them in chapters 1-3, where the
payoff would have been soonest -- and two things went wrong. Tamil's chapters
1-5 are HANDWRITTEN and protected from generation, so those segments existed in
the corpus and never reached the page at all. And one landed at sequence 175,
which made it the first lesson of chapter 3 and left that chapter impossible to
begin in the car (`unstartableChapters` caught it). Sitting inside the generated
chapters instead, the drizzle renders, and costs the driving edition exactly
nothing: not one existing lesson became undrivable and no chapter's drivable
prefix shortened.

They are additive -- nothing existing moves -- so the word-writing lessons keep
working, now arriving after the letters they use.

Everything mechanical comes from `data/scripts/tamil.json`: the components, the
pen path, the pen-lift count, and the citation. Nothing about a letter's shape
or stroke order is typed here, because none of it is mine to assert.
"""

import json, os, textwrap
from sharded_ledger import load_script

HERE = os.path.dirname(os.path.abspath(__file__))
HL = os.path.normpath(os.path.join(HERE, "..", ".."))
SCRIPT = load_script(HL, "tamil")
BY = {l["glyph"]: l for l in SCRIPT["letters"]}
MARKS = {m["mark"]: m for m in SCRIPT.get("marks", [])}

# One entry per segment. `hook` is the reason this letter is worth a lesson of
# its own; `payoff` is what it unlocks, and both are authored. Everything else is
# read from the script file.
SEGMENTS = [
    dict(n=1, seq=365, ch=6, glyph="வ", roman="va", slug="va", prev=None,
         title="வ — one spiral, one motion",
         hook=("It is the first letter of the first word you learned, and it is the "
               "easiest one in this book to write: the pen never leaves the paper."),
         payoff="வணக்கம் starts with it, and so does வாழ் — *to live*.",
         recall=("How many times does the pen lift while writing வ? (**None** — "
                 "one unbroken motion.) What sound does it carry on its own? (**va**.)")),
    dict(n=2, seq=405, ch=6, glyph="ண", roman="ṇa", slug="nna", prev="வ",
         title="ண — the first of three n's",
         hook=("Tamil has three different letters for sounds English hears as one *n*. "
               "This is the first, and the dot under its romanization — **ṇ**, not "
               "plain *n* — is doing real work: the tongue curls back to the roof of "
               "the mouth. Words genuinely differ by it."),
         payoff="It is the second letter of வணக்கம்.",
         recall=("Where does the tongue go for ண? (**Curled back**, to the roof of the "
                 "mouth.) How many pen lifts? (**One** — the body, then the upright.)")),
    dict(n=3, seq=445, ch=7, glyph="ன", roman="ṉa", slug="nnna", prev="ண",
         title="ன — the second n, and how to tell it from ண",
         hook=("Look at ண and ன side by side. Same spiral loop, same top bar, same "
               "upright on the right. The whole difference is **how many arches sit "
               "inside**: ண has two, ன has one. That is the entire distinction, and "
               "it is why these two are learned together rather than a chapter apart."),
         payoff="It ends நான் — *I* — and நன்றி's second syllable.",
         recall=("What separates ண from ன? (**Two inner arches against one.**) "
                 "Which one is this? (**ன**, one arch.)")),
    dict(n=4, seq=485, ch=8, glyph="ந", roman="na", slug="na", prev="ன",
         title="ந — the third n, and the only one built from uprights",
         hook=("The third n, and the one that finally looks different: no spiral loop "
               "at all. ந is built from a top bar and two uprights, with a right "
               "stroke that curls below the baseline. If you can see that it has no "
               "loop, you can already tell it from the other two."),
         payoff="நன்றி — *thank you* — opens with it, and so does நான்.",
         recall=("Which of the three n's has no spiral loop? (**ந**.) How many pen "
                 "lifts does it take? (**Two**.)")),
    dict(n=5, seq=525, ch=10, glyph="ற", roman="ṟa", slug="rra", prev="ந",
         title="ற — the last of the family, and the one that drops below the line",
         hook=("The fourth member of this top-bar family, and the one with a habit none "
               "of the others have: its right leg keeps going **below the baseline** "
               "and sweeps left into a long descender. Tamil letters mostly sit on the "
               "line. This one hangs off it."),
         payoff="It is the ṟ in நன்றி — and with it, every consonant of *thank you*.",
         recall=("What does ற do that ண, ன and ந do not? (**Drops below the "
                 "baseline.**) Which word have you now met all the consonants of? "
                 "(**நன்றி**.)")),
    dict(n=6, seq=565, ch=13, glyph="க", roman="ka", slug="ka", prev="ற",
         title="க — the letter that is three bowls",
         hook=("The most-used consonant in Tamil, and the most built: a square frame on "
               "top, then two bowls hanging under it. Three pen-down runs. Take it "
               "slowly — nothing else in this book asks for as many separate pieces."),
         payoff="வணக்கம் has it twice, doubled in the middle.",
         recall=("How many pen lifts does க take? (**Two** — three separate runs.) "
                 "What are the three parts? (**Upper frame, lower-left bowl, "
                 "lower-right bowl.**)")),
    dict(n=7, seq=605, ch=16, glyph="ம", roman="ma", slug="ma", prev="க",
         title="ம — upright on the left, curve on the right",
         hook=("One unbroken stroke again, like வ. The only thing to get right is which "
               "way round it goes: the **upright is on the LEFT** and the **curve on "
               "the right**. Most people guess the reverse, because that is the habit "
               "Devanagari teaches — hanging a shape off a right-hand spine."),
         payoff="It is the last consonant of வணக்கம்.",
         recall=("Which side is the upright on? (**The left.**) How many pen lifts? "
                 "(**None.**)")),
]

MARK_SEGMENTS = [
    dict(n=8, seq=645, ch=18, mark="்", roman="puḷḷi", slug="pulli", prev="ம",
         title="் — the dot that takes the vowel away",
         hook=("Every Tamil consonant you have written so far carries a hidden *a*. வ "
                "is not *v*, it is **va**. ம is **ma**. That is what an abugida is: the "
                "vowel is built in.\n\nThe puḷḷi is how you take it back. One dot above "
                "the letter, and வ becomes plain **v**.\n\nHere is what Tamil does that "
                "Devanagari does not: **the dot stays visible and both letters keep "
                "their full shape.** Devanagari fuses two consonants into a single new "
                "conjunct you have to learn separately. Tamil just writes the dot. "
                "There is nothing extra to memorise."),
         payoff=("With this you can write வணக்கம் — every letter, in order:\n\n"
                 "> வ + ண + க + ் + க + ம + ்\n\n"
                 "Seven marks on the page, and you have written *hello*."),
         recall=("What does the puḷḷi do? (**Removes the built-in vowel.**) What is வ "
                 "with a puḷḷi? (**v**, not *va*.) What does Tamil NOT do that "
                 "Devanagari does? (**Fuse the letters into a new shape.**)")),
    dict(n=9, seq=685, ch=19, mark="ி", roman="i sign", slug="i-sign", prev="்",
         title="ி — the first vowel sign",
         hook=("A consonant carries *a*. To make it carry a different vowel you add a "
               "**sign** — and the sign replaces the built-in vowel rather than sitting "
               "beside it.\n\nThe i-sign is a hook written above and to the right. "
               "ற is *ṟa*; றி is **ṟi**."),
         payoff=("And with that: நன்றி.\n\n> ந + ன + ் + ற + ி\n\n"
                 "*Thank you* — the second word you can write."),
         recall=("Does a vowel sign add a vowel or replace one? (**Replace** — the "
                 "built-in *a* is gone.) What is ற with an i-sign? (**ṟi**.)")),
]


def atom(n, kind="SCRIPT"):
    return f"TA-{kind}-DRIZZLE-{n:02d}"


def lst(atoms):
    """A knowledge list as this corpus writes them: bare ids, never quoted."""
    return "[" + ", ".join(atoms) + "]"


SLUGS = {}


def prereq_of(n):
    """The previous segment's lesson id, or nothing for the first."""
    return [] if n <= 1 else [f"TA-S{n-1:02d}-{SLUGS[n-1]}"]


def letter_lesson(s):
    d = BY[s["glyph"]]
    src = d["strokeOrderSource"]
    lifts = d.get("penLifts", 0)
    # Bulleted with a bold number, not a Markdown ordered list. The book
    # renderer has no `enumerate` conversion, so "1. ... 2. ..." collapses into
    # one run-on paragraph on the page -- which for a stroke order is not a
    # cosmetic problem: the steps ARE the instruction, and a reader cannot
    # follow a pen path written as prose.
    steps = "\n".join(f"- **{i}.** {t}" for i, t in enumerate(d["strokeOrder"], 1))
    parts = "\n".join(f"- {c}" for c in d["components"])
    prev_line = ""
    if s["prev"]:
        prev_line = f"\n[PAUSE 1s] Before the new one: say the sound of **{s['prev']}**.\n"
    requires = [atom(s["n"] - 1)] if s["n"] > 1 else []
    prereqs = prereq_of(s["n"])
    front = f"""---
schema_version: 2
id: TA-S{s['n']:02d}-{s['slug']}
spine_node: SPINE-MEET-GREET
sequence: {s['seq']}
delivery: script
chapter: {s['ch']}
type: writing
headword: "{s['glyph']}"
gloss: the single letter {s['glyph']} — met, then written
romanization: "{s['roman']}"
prerequisites: {lst(prereqs)}
sounds: [tamil-inherent-a]
roots: []
duration:
  max_seconds: 150
requires:
  knowledge: {lst(requires)}
introduces:
  knowledge: [{atom(s["n"])}]
practises:
  knowledge: {lst(requires + [atom(s["n"])])}
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [language-focus]
register: neutral
variety: standard-colloquial
reviews_of: {lst(prereqs)}
---
"""
    body = f"""
# {s['title']}

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses={lst(requires)} -->
{prev_line}
[PAUSE 2s] One letter this time. Just one.

{s['hook']}

## Script you'll notice: {s['glyph']}
<!-- hl-knowledge: introduces=[{atom(s["n"])}]; assesses=[] -->

**{s['glyph']}** — *{s['roman']}*.

What it is made of:

{parts}

{s['payoff']}

## Writing: {s['glyph']}
<!-- hl-knowledge: introduces=[]; assesses=[{atom(s["n"])}] -->

{steps}

**Pen lifts: {lifts}.** {'The pen never leaves the paper.' if lifts == 0 else f'The pen comes up {lifts} time' + ('' if lifts == 1 else 's') + ' and no more.'}

> Stroke order is one attested teaching order, not a national standard —
> Tamil handwriting is taught with school-to-school variation. Source:
> {src['citation']}.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[{atom(s["n"])}] -->

[PAUSE 1s]
- [YOU TRACE: {s['glyph']} once, slowly, following the steps above]
- [YOU SAY: "{s['roman']}" as you finish it]
- [YOU TRACE: it twice more, without looking back at the steps]

## Wrap-up Recall
<!-- hl-knowledge: introduces=[]; assesses=[{atom(s["n"])}] -->

[PAUSE 3s] {s['recall']}
"""
    return front + body


def mark_lesson(s):
    m = MARKS[s["mark"]]
    requires = [atom(s["n"] - 1)]
    prereqs = prereq_of(s["n"])
    front = f"""---
schema_version: 2
id: TA-S{s['n']:02d}-{s['slug']}
spine_node: SPINE-MEET-GREET
sequence: {s['seq']}
delivery: script
chapter: {s['ch']}
type: writing
headword: "{s['mark']}"
gloss: the mark {s['mark']} — what it does to the letter it sits on
romanization: "{s['roman']}"
prerequisites: {lst(prereqs)}
sounds: [pulli-virama]
roots: []
duration:
  max_seconds: 180
requires:
  knowledge: {lst(requires)}
introduces:
  knowledge: [{atom(s["n"])}]
practises:
  knowledge: {lst(requires + [atom(s["n"])])}
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [language-focus]
register: neutral
variety: standard-colloquial
reviews_of: {lst(prereqs)}
---
"""
    body = f"""
# {s['title']}

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses={lst(requires)} -->

[PAUSE 2s] Not a letter this time — a mark that changes the letter under it.

## Script you'll notice: {s['mark']}
<!-- hl-knowledge: introduces=[{atom(s["n"])}]; assesses=[] -->

{s['hook']}

Where it sits: {m['attachesAs']}

## Writing: {s['mark']}
<!-- hl-knowledge: introduces=[]; assesses=[{atom(s["n"])}] -->

{s['payoff']}

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[{atom(s["n"])}] -->

[PAUSE 1s]
- [YOU TRACE: the mark on a letter you already know]
- [YOU SAY: the letter's sound with the mark, then without it]
- [YOU WRITE: the whole word above, one mark at a time]

## Wrap-up Recall
<!-- hl-knowledge: introduces=[]; assesses=[{atom(s["n"])}] -->

[PAUSE 3s] {s['recall']}
"""
    return front + body


if __name__ == "__main__":
    for s in SEGMENTS + MARK_SEGMENTS:
        SLUGS[s["n"]] = s["slug"]
    out = os.path.join(HL, "tamil", "lessons")
    written = []
    for s in SEGMENTS:
        name = f"TA-S{s['n']:02d}-{s['slug']}.md"
        open(os.path.join(out, name), "w", encoding="utf-8").write(letter_lesson(s))
        written.append(name)
    for s in MARK_SEGMENTS:
        name = f"TA-S{s['n']:02d}-{s['slug']}.md"
        open(os.path.join(out, name), "w", encoding="utf-8").write(mark_lesson(s))
        written.append(name)
    print(f"wrote {len(written)} drizzle segments")
    for n in written:
        print("  ", n)
