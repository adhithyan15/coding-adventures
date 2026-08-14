#!/usr/bin/env python3
"""Author the first script segments for Telugu, Kannada, Malayalam and Sanskrit.

WHY THESE FOUR TRACKS GET RECOGNITION AND TAMIL GOT WRITING
-----------------------------------------------------------
Tamil's drizzle (author_drizzle_segments.py) teaches the hand: each segment
carries a numbered pen path, because every letter it teaches has a cited stroke
order in `tamil.json`. These four do not have that. Measured over the committed
script files:

    telugu.json      455 letters, 0 with a stroke order
    kannada.json     455 letters, 0 with a stroke order
    malayalam.json   468 letters, 0 with a stroke order
    devanagari.json   28 letters, 9 with a CITED stroke order --
                      and only two of those nine (अ, आ) fall in the
                      first 24 positions of the letter ledger

Each of the three Dravidian script files says so in its own `notes`: *"Recognition
only -- stroke order not researched."* HL11 section 5's rule is then binding: **no
citation -> no pen path -> no figure.** Inventing a plausible-looking stroke order
is the one thing that would be worse than shipping nothing, because a learner
cannot tell an invented order from an attested one and will drill it for years.

So these segments teach the rung that comes BEFORE the hand, and it is a real
rung rather than a consolation prize. HL12 section 2.2 lays the decoding ladder
out in order:

    letters -> vowel signs -> the vowel-killer -> conjuncts -> running text

and every one of those steps is recognition before it is production. A reader who
can pick ರ out of ನಮಸ್ಕಾರ has done something they could not do on the previous
page. When the stroke orders are sourced (HL-C118), the writing segments slot in
behind these without moving them.

WHAT THE READER IS ASKED TO DO WITH THE PEN
-------------------------------------------
Each segment still ends at the paper, because *copying a printed shape needs no
citation*. "Start here, go this way, lift now" is the claim that needs a source;
"make your line follow that line" is the reader looking at the glyph in front of
them. So the writing block asks for tracing and says plainly that the stroke
order is not set out yet -- which is honest, and which also means a reader who
later learns the proper order is not unlearning something this book told them.

HOW A LETTER GETS ITS LESSON, AND WHY NOTHING HERE IS TYPED BY HAND
-------------------------------------------------------------------
Everything the reader is told about a letter is read from committed data:

    the glyph and its order      the per-script letter ledger (HL11 section 4),
                                 which orders letters by the words they unlock
    its sound                    the script file's own `sound` field, or the
                                 Unicode character name for a vowel sign
    where they have met it       a walk of the track's own lessons: every
                                 headword ALREADY TAUGHT (sequence below this
                                 segment's) that contains the glyph
    what it is made of           nothing. These files record no components for
                                 these scripts, so this says nothing about shape.

That last line is the point. The one thing a reader most wants -- "what is this
letter made of, and how do I draw it" -- is exactly what is not sourced, and the
segment is built so that its absence is visible rather than papered over.

PLACEMENT, AND THE MEASUREMENT THAT CHOSE IT
--------------------------------------------
Chapters 1-5 of all four tracks are handwritten and protected from generation, so
a segment placed there would exist in the corpus and never reach the page -- the
failure that cost the Tamil drizzle a whole revision. Segments therefore land in
chapters 6-13, one per chapter.

WHERE in the chapter was decided by measuring both options rather than by taste.
A recognition segment needs eyes, so it is `pen` and a commuter cannot do it, and
HL08 counts the damage three ways. Placing each segment second, right after the
chapter opener:

    drivablePrefixTotal    1136 -> 1125     eleven lessons lost
    fullyDrivableChapters   489 -> 471

Placing it last in its chapter:

    drivablePrefixTotal    1136 -> 1136     nothing lost
    fullyDrivableChapters   489 -> 471

Last, then. The drivable prefix is the run of lessons a commuter can do before
hitting one that needs eyes, so a segment at the front truncates its whole
chapter and a segment at the back truncates nothing. The 18 chapters that stop
being FULLY drivable are unavoidable and are the honest cost: a chapter that
teaches a letter contains something you cannot do at the wheel. `unstartable`
chapters stay at 173 -- no segment is ever a chapter's opener.

The placement is better teaching as well as cheaper. A letter arriving at the END
of a chapter arrives after every word in that chapter that contains it, so the
segment consolidates what the reader just met rather than pre-teaching a shape
they have no use for yet.

Every letter these segments teach was first SHOWN in chapters 1-5, inside words
the reader already says. That ordering is what keeps HL12 section 2.1 satisfied
for free: a segment is at the frontier of decoding and never at the frontier of
meaning, because every word in it is one the reader has already met.
"""

import json
import os
import re
import sys
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
HL = os.path.normpath(os.path.join(HERE, "..", ".."))

# One entry per track. `ledger` names the script whose letter ledger orders this
# track's segments -- Sanskrit and Hindi share Devanagari's, which is why that
# ledger declares two tracks.
TRACKS = [
    dict(track="telugu", prefix="TE", script="telugu", ledger="telugu"),
    dict(track="kannada", prefix="KA", script="kannada", ledger="kannada"),
    dict(track="malayalam", prefix="ML", script="malayalam", ledger="malayalam"),
    dict(track="sanskrit", prefix="SA", script="devanagari", ledger="devanagari"),
    dict(track="hindi", prefix="HI", script="devanagari", ledger="devanagari"),
    # Tamil is HL13's reference track for the script addendum. Eleven of its
    # letters carry a CITED stroke order, so `writing_block` emits a real
    # numbered pen path for those and asks for tracing on the rest -- both halves
    # of the addendum in one track, which is why it is built here first.
    dict(track="tamil", prefix="TA", script="tamil", ledger="tamil"),
]

# Chapters 6 onward, as many as a track has. A ledger holds 24 positions and the
# drizzle is one per chapter, so the range has to reach far enough for all of
# them; slots a track does not have are skipped.
CHAPTERS = list(range(6, 45))

# The spine node the script strand hangs from in all four local paths, and the
# one Tamil's drizzle already uses. See the comment in `build()`.
SCRIPT_SPINE_NODE = "SPINE-MEET-GREET"

# A slug has to survive a Windows checkout and a `git log` in a terminal that
# does not have a Telugu font, so the file name is ASCII: the letter's Unicode
# name, lowercased, minus the script name. TELUGU VOWEL SIGN AA -> vowel-sign-aa.
def slug_of(entry, script_name):
    name = entry["unicodeName"].upper()
    prefix = script_name.upper() + " "
    if name.startswith(prefix):
        name = name[len(prefix):]
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")


def fm_of(text):
    """Read the frontmatter the way the TypeScript parser presents it.

    The FILE nests one level -- `introduces:` on its own line, then an indented
    `knowledge:` -- while the parser flattens that to the dotted key
    `introduces.knowledge`, which is how every gate and every other tool refers
    to it. This mirrors the flattening, and it is not cosmetic: the first version
    of this function matched only unindented keys, so every lesson's atom list
    came back empty, every chapter looked like it had its whole budget free, and
    the budget check below silently passed everything.
    """
    if not text.startswith("---"):
        return {}
    end = text.find("\n---", 3)
    out = {}
    parent = None
    for line in text[4:end].split("\n"):
        top = re.match(r"^([A-Za-z0-9_]+):\s*(.*)$", line)
        if top:
            parent = top.group(1)
            out[parent] = top.group(2).strip()
            continue
        child = re.match(r"^\s+([A-Za-z0-9_]+):\s*(.*)$", line)
        if child and parent:
            out[f"{parent}.{child.group(1)}"] = child.group(2).strip()
    return out


def load_lessons(track):
    d = os.path.join(HL, track, "lessons")
    rows = []
    for f in sorted(os.listdir(d)):
        if not f.endswith(".md"):
            continue
        h = fm_of(open(os.path.join(d, f), encoding="utf-8").read())
        seq = int(h.get("sequence", "0") or 0)
        if seq <= 0:
            continue
        rows.append(dict(
            id=h.get("id", f[:-3]),
            seq=seq,
            chapter=int(h.get("chapter", "0") or 0),
            spine=h.get("spine_node", ""),
            headword=h.get("headword", "").strip().strip('"'),
            roman=h.get("romanization", "").strip().strip('"'),
            gloss=h.get("gloss", "").strip().strip('"'),
            # `introduces.knowledge` is FLAT and DOTTED in this corpus, never
            # nested. Read as nested it returns nothing for every lesson, and a
            # chapter's atom count comes out zero -- which would make every
            # chapter look like it had room to spare.
            atoms=len([a for a in h.get("introduces.knowledge", "")
                       .strip("[]").split(",") if a.strip()]),
        ))
    rows.sort(key=lambda r: r["seq"])
    return rows


def short_gloss(g):
    """Glosses run long because they carry the lesson's whole angle. A bullet in a
    recognition segment wants the meaning and nothing else, so the gloss is cut at
    its first structural break -- and only when it is actually long, so a short
    gloss is never mangled to save a few characters."""
    g = g.strip()
    if len(g) <= 60:
        return g
    for sep in [" (", " -- ", " — ", "; ", ", "]:
        if sep in g:
            head = g.split(sep)[0].strip()
            if len(head) >= 8:
                return head
    return g


# U+25CC DOTTED CIRCLE. A combining mark has no shape of its own to sit on, so
# printed bare it either collides with the character before it or renders as a
# stray accent -- and it would open this lesson's heading. The dotted circle is
# the standard placeholder base for exactly this, and it is what makes "here is
# the mark, by itself" a thing the page can actually show.
DOTTED_CIRCLE = "◌"


def shown(entry, glyph):
    """How the character is written in prose: alone if it can stand alone, on a
    dotted circle if it cannot."""
    return glyph if entry["kind"] == "letter" else DOTTED_CIRCLE + glyph


# What kind of thing the reader is looking at, said once and said the same way in
# all four books. Every claim here is true of any abugida and is already asserted
# by each script file's own `system: "abugida"`; nothing here is a claim about a
# particular letter's shape, because that is what is not sourced.
def kind_note(entry, sound, script_label):
    name = entry["unicodeName"].upper()
    if "VIRAMA" in name or "CHANDRAKKALA" in name:
        return ("a **vowel-killer**. Every consonant in this script arrives with an *a* "
                "already inside it — that is what an abugida is. This mark takes the *a* "
                "back, leaving the bare consonant. It is how the script writes two "
                "consonants in a row.")
    if "ANUSVARA" in name:
        return ("a **nasal**, written as a mark rather than as a letter. It rides on the "
                "syllable before it and turns the end of it nasal.")
    if "CANDRABINDU" in name:
        return ("a **nasal vowel** mark — the vowel itself is spoken through the nose, "
                "rather than a nasal consonant being added after it.")
    if "VOWEL SIGN" in name:
        return (f"a **vowel sign**. It is not a letter and never stands alone: it attaches "
                f"to a consonant and **replaces** the *a* built into it with *{sound}*. "
                f"Replaces, not adds — the *a* is gone.")
    if "LETTER" in name and entry["kind"] == "letter" and sound.endswith("a") and len(sound) > 1:
        return (f"a **consonant**, and in this script a consonant is never bare: it comes "
                f"with an *a* already in it. So it is not *{sound[:-1]}*, it is **{sound}**.")
    return (f"an **independent vowel** — the shape the vowel *{sound}* takes when a word "
            f"begins with it, rather than the sign it becomes inside a word.")


# What the reader says when they finish tracing, and what the wrap-up asks them
# for. A mark that carries no sound cannot be pronounced, so asking the reader to
# say it is an instruction they cannot follow -- and the first draft of this file
# printed exactly that: "saying *no sound of its own* as you finish each one".
def voicing(entry, sound):
    name = entry["unicodeName"].upper()
    if "VIRAMA" in name or "CHANDRAKKALA" in name:
        return dict(label="no sound of its own — it takes one away",
                    say="and each time say what it does: **it kills the built-in *a***",
                    ask="What does it do to the consonant it sits on?",
                    answer="**Takes away the built-in *a*.**")
    if "ANUSVARA" in name or "CANDRABINDU" in name:
        return dict(label="a nasal, ridden on the syllable before it",
                    say="humming the nasal at the end of the syllable as you finish each one",
                    ask="What does it add to the syllable before it?",
                    answer="**A nasal.**")
    if "VOWEL SIGN" in name:
        return dict(label=sound,
                    say=f"saying *{sound}* as you finish each one",
                    ask="What vowel does it put on the consonant it attaches to — and what "
                        "does it take off?",
                    answer=f"**Puts *{sound}* on; takes the built-in *a* off.**")
    return dict(label=sound,
                say=f"saying *{sound}* as you finish each one",
                ask="What sound does it carry?",
                answer=f"***{sound}***.")


def build(cfg):
    S = json.load(open(os.path.join(HL, f"data/scripts/{cfg['script']}.json"), encoding="utf-8"))
    L = json.load(open(os.path.join(HL, f"data/scripts/{cfg['ledger']}-ledger.json"), encoding="utf-8"))
    lessons = load_lessons(cfg["track"])

    # Each script file records a different amount, and a segment shows whatever
    # its script actually has rather than the least common denominator. Devanagari
    # carries components, cited stroke orders for nine letters, and a worked
    # base+sign example for every mark; the three Dravidian files carry a sound
    # and nothing else. Both are read the same way here and the renderer prints
    # what it finds.
    data = {}
    for l in S["letters"]:
        data.setdefault(l["glyph"], l)
    for l in S.get("independentVowels", []):
        data.setdefault(l["glyph"], l)
    for m in S.get("marks", []):
        data.setdefault(m["mark"], dict(m, glyph=m["mark"]))
    sounds = {g: d.get("sound", "") for g, d in data.items()}

    # A slot per chapter: opener's sequence + 5, and the opener's spine node, so
    # the segment sits inside the chapter it belongs to rather than dangling.
    #
    # Every segment declares SPINE-MEET-GREET rather than its chapter's own spine
    # node, which looks wrong and is not. The shared spine is a ladder of things a
    # reader can DO with the language, and reading the script is not one of the
    # rungs on it -- so a script segment attaches to the node its track's local
    # path hangs the script strand from, exactly as Tamil's drizzle does. The
    # validator enforces the other half of that: a lesson's declared spine node
    # must be one its local path actually visits, which is why registration in
    # `curriculum.json` is part of authoring a segment rather than a follow-up.
    # Last in its chapter -- see PLACEMENT in the module docstring for the two
    # measurements that decided it. `rows` is sorted by sequence, so rows[-1] is
    # the chapter's final lesson and +5 lands after it and before the next
    # chapter, since every authored sequence is a multiple of ten.
    #
    # A chapter is a slot only if it can still afford one more atom. Each segment
    # introduces exactly one, and HL08's per-chapter atom budget is the corpus's
    # own measure of "do not throw many things at the reader at once" -- so a
    # segment that pushes a chapter over it is not a gentle ramp, it is one more
    # thing thrown. Sanskrit is why this check exists rather than being assumed:
    # its chapter 7 sat at exactly 12 of 12 and a segment tipped it to 13, and its
    # chapter 6 was already at 15. Both are skipped, so Sanskrit takes fewer
    # segments than the others. Fewer, in the right chapters, is the point.
    budget = json.load(open(os.path.join(HL, "core/chapter-policy.json"),
                            encoding="utf-8"))["maxNewAtomsPerChapter"]
    slots = []
    for ch in CHAPTERS:
        rows = [r for r in lessons if r["chapter"] == ch]
        if not rows:
            continue
        if sum(r["atoms"] for r in rows) + 1 > budget:
            continue
        # The first FREE sequence after this chapter's last lesson. `last + 5`
        # was not safe: a chapter whose last lesson already sits at a +5 offset
        # (because an earlier pass put a segment there) pushes the next one onto
        # the following chapter's opener, and the validator rejects the whole
        # run for a duplicate sequence.
        taken = {r["seq"] for r in lessons}
        seq = rows[-1]["seq"] + 1
        while seq in taken:
            seq += 1
        slots.append(dict(chapter=ch, seq=seq, spine=SCRIPT_SPINE_NODE))

    # Ledger order, filling slots in turn, and a letter is skipped when the reader
    # would meet it before meeting any word that contains it.
    #
    # Two different reasons a letter gets skipped, and both matter. Devanagari's
    # ledger is shared with Hindi and orders letters by HINDI's word payoff, so it
    # contains letters no Sanskrit word in this book writes -- ँ is one. And a
    # letter whose first word is still hundreds of sequences away would open a
    # segment with nothing to recognise it in, which is the Root Ledger's unspent
    # rule applied to glyphs: a letter is worth exactly what it unlocks, and one
    # that unlocks nothing yet is not yet worth a lesson. Kannada's ಓ is that case
    # -- its first word, ಓದು, sits at sequence 690.
    #
    # Skipping is selection FROM the ledger, never a rewrite of it: the order is
    # authored intent and the file is not touched here.
    import re as _re
    taught_alone = set()
    for _f in os.listdir(os.path.join(HL, cfg["track"], "lessons")):
        if not _f.endswith(".md"):
            continue
        _b = open(os.path.join(HL, cfg["track"], "lessons", _f), encoding="utf-8").read()
        if not _re.search(r"^delivery: script", _b, _re.M):
            continue
        _h = _re.search(r"^headword: (.*)$", _b, _re.M)
        if _h:
            _hw = _h.group(1).strip().strip('"').replace("\u25cc", "")
            if len([c for c in _hw if not c.isspace()]) == 1:
                taught_alone.add(_hw)

    out = []
    pending = [e for e in L["letters"] if e["glyph"] not in taught_alone]
    for slot in slots:
        # A candidate that does not qualify at THIS slot is put back, not thrown
        # away. The first version popped and discarded, so a letter whose first
        # word comes later than its ledger position was lost for good --
        # Kannada's ಓ (first used chapter 33) and Malayalam's ഉ (chapter 17)
        # both vanished that way, even though a later slot suits them exactly.
        e = None
        deferred = []
        while pending:
            cand = pending.pop(0)
            if any(cand["glyph"] in r["headword"] and r["seq"] < slot["seq"] for r in lessons):
                e = cand
                break
            deferred.append(cand)
        pending = deferred + pending
        if e is None:
            break
        n = 100 + e["position"]
        g = e["glyph"]
        sound = sounds.get(g, "")
        if not sound:
            # A vowel sign has no standalone sound in the syllabary; its value is
            # the vowel named at the end of its Unicode name (VOWEL SIGN AA -> ā).
            tail = e["unicodeName"].upper().split()[-1]
            sound = {"AA": "ā", "II": "ī", "UU": "ū", "EE": "ē",
                     "OO": "ō", "A": "a", "I": "i", "U": "u", "E": "e", "O": "o",
                     "AI": "ai", "AU": "au"}.get(tail, tail.lower())
            if "VIRAMA" in e["unicodeName"].upper() or "ANUSVARA" in e["unicodeName"].upper() \
               or "CHANDRABINDU" in e["unicodeName"].upper() or "CHANDRAKKALA" in e["unicodeName"].upper():
                sound = ""
        # Words the reader has already met that contain this character -- and
        # ones they can SAY, preferred. A headword with no `romanization` is
        # printed in a script the reader is only now learning to decode, so it
        # cannot be read aloud and makes a poor example to recognise a character
        # in. Malayalam's first page of segments showed the cost: two of four
        # bullets were script the reader had no way to pronounce. Ones that carry
        # a romanization come first; ones that do not still fill the list rather
        # than leaving it short, because a word they can see is better than a gap.
        seen = [r for r in lessons
                if r["seq"] < slot["seq"] and g in r["headword"] and is_a_word(r["headword"], g)]
        known = ([r for r in seen if r["roman"]] + [r for r in seen if not r["roman"]])[:4]
        known.sort(key=lambda r: r["seq"])
        others = [r for r in lessons
                  if r["seq"] < slot["seq"] and g not in r["headword"] and r["roman"]
                  and is_a_word(r["headword"], g)]
        out.append(dict(n=n, entry=e, glyph=g, sound=sound, slug=slug_of(e, S["name"]),
                        record=data.get(g, {}),
                        known=known, distractor=others[0] if others else None, **slot))
    return dict(cfg=cfg, script=S, ledger=L, segments=out)


def is_a_word(headword, glyph):
    """Is this headword a WORD the reader can be shown a character inside?

    Two things in these corpora are headwords and are not words, and both landed
    on the page before this check existed. A lesson may teach the character
    itself -- Hindi's inherent-vowel lesson has the headword अ -- and listing अ as
    a word containing अ is circular. And a lesson may teach a SET of marks, with a
    headword like "ा, े", which is a list rather than something anyone says.

    So: at least two Unicode LETTERS, counting neither combining marks nor
    punctuation, and not the character being taught. A vowel sign is category Mn
    or Mc and a comma is Po, so both non-words fall out and every real word stays.
    """
    if headword.strip() == glyph:
        return False
    return sum(1 for c in headword if unicodedata.category(c).startswith("L")) >= 2

def shape_and_example(seg, show):
    """What the script file records about this character beyond its sound.

    Devanagari lists a component breakdown for every letter and, for every mark,
    both where it attaches and a worked base+sign+result example. The three
    Dravidian files list neither -- their `components` entry is the syllable
    restating itself -- so this returns nothing for them and the segment is
    shorter rather than padded with something that was not recorded.

    A worked example is the best thing a vowel-sign lesson can show, because a
    sign has no independent existence: seeing न + ा = ना is the whole idea, and
    the base, the combination and the resulting sound are all in the file.
    """
    rec = seg["record"]
    out = []
    parts = [c for c in rec.get("components", []) if c.strip() and seg["glyph"] not in c]
    if parts:
        out.append("What it is made of:\n\n" + "\n".join(f"- {c}" for c in parts))
    where = rec.get("attachesAs", "").strip()
    if where:
        out.append(f"Where it sits: {where}.")
    ex = rec.get("example") or {}
    if ex.get("base") and ex.get("combined"):
        out.append(f"Worked through: **{ex['base']}** + **{show}** = **{ex['combined']}**"
                   + (f" — *{ex['sound']}*." if ex.get("sound") else "."))
    return ("\n\n".join(out) + "\n\n") if out else ""

def writing_block(seg, show):
    """The section that asks for the pen, in one of its two honest forms.

    A CITED stroke order gets the real thing: the numbered path, the pen-lift
    count, and the citation, exactly as Tamil's drizzle prints them. Everything
    in it is read from the script file and nothing is inferred, because a stroke
    order is the one claim in this book that a learner cannot check and will
    drill for years if it is wrong.

    No citation gets tracing, which needs no source: the reader copies the shape
    in front of them. The book says so rather than staying quiet about it, so a
    reader who later learns the proper order is not unlearning something this
    book told them.

    Nine of Devanagari's twenty-eight letters are cited, which is why Hindi and
    Sanskrit print both forms in the same chapter run and the three Dravidian
    tracks print only the second.
    """
    rec = seg["record"]
    steps = rec.get("strokeOrder") or []
    src = rec.get("strokeOrderSource")
    if not (steps and src):
        return f"## Writing: {show} — copy what you see", f"""Put your pen on {show} and follow its line. Copy the shape you can see — slowly,
and larger than it is printed.

> This book does not yet tell you **where to start the character or which way to
> travel**. That is a real thing, taught with real variation from school to
> school, and it is not written down here until it can be written down with a
> source. Copying what is in front of you needs no such source, and it is how the
> shape gets into your hand in the meantime."""

    lifts = rec.get("penLifts", 0)
    # Bulleted with a bold number rather than a Markdown ordered list: the book
    # renderer has no `enumerate` conversion, so "1. ... 2. ..." collapses into a
    # single run-on paragraph -- and for a stroke order the steps ARE the
    # instruction, which a reader cannot follow written as prose.
    body = "\n".join(f"- **{i}.** {t}" for i, t in enumerate(steps, 1))
    lift_line = ("The pen never leaves the paper." if lifts == 0 else
                 f"The pen comes up {lifts} time" + ("" if lifts == 1 else "s") + " and no more.")
    note = rec.get("strokeOrderNote", "").strip()
    caveat = f"\n> {note[0].upper() + note[1:]}.\n" if note else "\n"
    return f"## Writing: {show}", f"""{body}

**Pen lifts: {lifts}.** {lift_line}
{caveat}
> This is one attested teaching order and not a national standard — handwriting
> here is taught with school-to-school variation. Source: {src['citation']}."""

def render(cfg, script, seg, prev):
    g, n = seg["glyph"], seg["n"]
    pfx = cfg["prefix"]
    atom = f"{pfx}-SCRIPT-RECOG-{n:02d}"
    # Chain to the PREVIOUS SEGMENT IN THIS RUN. `n` is a ledger position now, so
    # n-1 names an atom that need not exist -- the positions a pass emits are not
    # consecutive once some letters are already taught.
    prev_atom = f"{pfx}-SCRIPT-RECOG-{prev['n']:02d}" if prev else None
    prev_id = f"{pfx}-S{prev['n']:02d}-{prev['slug']}" if prev else None
    requires = [prev_atom] if prev_atom else []
    prereqs = [prev_id] if prev_id else []
    lst = lambda a: "[" + ", ".join(a) + "]"

    v = voicing(seg["entry"], seg["sound"])
    show = shown(seg["entry"], g)
    note = kind_note(seg["entry"], seg["sound"], script["name"])
    writing_heading, writing_body = writing_block(seg, show)
    shape = shape_and_example(seg, show)
    # A soundless mark has no romanization to give, so the field carries the
    # mark's NAME instead -- which is what the narration export reads aloud, and
    # "virama" is the right thing to hear where a sound would be silence.
    roman = seg["sound"] or seg["entry"]["unicodeName"].split()[-1].lower()

    known_lines = "\n".join(
        f"- **{r['headword']}**" + (f" *{r['roman']}*" if r["roman"] else "")
        + (f" — {short_gloss(r['gloss'])}" if r["gloss"] else "")
        for r in seg["known"]
    ) or "- (none yet — this one arrives before the words that need it)"

    hunt = [r["headword"] for r in seg["known"][:2]]
    if seg["distractor"]:
        hunt.append(seg["distractor"]["headword"])
    # Single words only. A hunt line is "find this letter in these words", and
    # a five-word counting phrase is not a word -- Malayalam's
    # "onnu randu muunnu naalu anchu" filled the line by itself, which is both a
    # bad drill and six underfull hboxes, since a nearly-full centred line has
    # no room left to stretch to the measure.
    hunt = [w for w in hunt if " " not in w]
    hunt_line = "  \u00b7  ".join(hunt)

    warm = (f"[PAUSE 1s] Before the new one: {shown(prev['entry'], prev['glyph'])} — "
            f"what does it do?\n\n"
            if prev else
            f"[PAUSE 1s] {script['signature']}\n\n")

    front = f"""---
schema_version: 2
id: {pfx}-S{n:02d}-{seg['slug']}
spine_node: {seg['spine']}
sequence: {seg['seq']}
delivery: script
chapter: {seg['chapter']}
type: writing
headword: "{g}"
gloss: the single character {g} — recognised inside words you already say
romanization: "{roman}"
prerequisites: {lst(prereqs)}
sounds: []
roots: []
duration:
  max_seconds: 150
requires:
  knowledge: {lst(requires)}
introduces:
  knowledge: [{atom}]
practises:
  knowledge: {lst(requires + [atom])}
skills: [reading, writing]
modes: [interpretive, presentational]
strands: [language-focus]
register: neutral
variety: standard-colloquial
reviews_of: {lst(prereqs)}
---
"""

    body = f"""
# {show} — one character, met inside words you already say

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses={lst(requires)} -->

{warm}[PAUSE 2s] One character this time. Just one — and you have been saying it
for pages without knowing which mark on the page it was.

## Script you'll notice: {show}
<!-- hl-knowledge: introduces=[{atom}]; assesses=[] -->

**{show}** — *{v['label']}*.

It is {note}

{shape}You already say these, and every one of them has {show} somewhere inside it:

{known_lines}

{writing_heading}
<!-- hl-knowledge: introduces=[]; assesses=[{atom}] -->

{writing_body}

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[{atom}] -->

[PAUSE 1s]
- [YOU LOOK: at these words, and find {show} in the ones that have it]

> {hunt_line}

- [YOU TRACE: {show} three times, {v['say']}]
- [YOU LOOK: back at any page of this chapter and find {show} once more]

## Wrap-up Recall
<!-- hl-knowledge: introduces=[]; assesses=[{atom}] -->

[PAUSE 3s] Which character is this — {show}? {v['ask']} ({v['answer']})
Name one word you already say that contains it.
"""
    return front + body


def register(cfg, script, segs):
    """Put the segments on the track's local path.

    A lesson that names a spine node its local path never visits is invisible to
    the curriculum and rejected by the validator, so a segment is not authored
    until it is registered. Two objects carry it, matching Tamil's drizzle: a
    PATH node, which is the reader's route, and an EXTENSION node, which is the
    capability the route delivers. The extension's `canDo` is authored -- it is a
    promise about the reader, and no generator can make one of those.
    """
    p = os.path.join(HL, cfg["track"], "curriculum.json")
    doc = json.load(open(p, encoding="utf-8"))
    ids = [f"{cfg['prefix']}-S{s['n']:02d}-{s['slug']}" for s in segs]
    path_id = f"{cfg['prefix']}-PATH-100"
    ext_id = f"{cfg['prefix']}-EXT-100-SCRIPT-RECOGNITION"

    # MERGE. Replacing the node outright orphans an earlier pass's lessons --
    # Tamil arrives with nine drizzle segments already in TA-PATH-100, and
    # dropping them would leave nine lessons declaring a spine node their path no
    # longer visits.
    prior = next((n["lessons"] for n in doc["path"] if n["id"] == path_id), [])
    prior_inline = next((n.get("inline", []) for n in doc["path"] if n["id"] == path_id), [])
    claimed_elsewhere = {l for e in doc.get("extensions", [])
                         if e["id"] != ext_id for l in e.get("lessons", [])}
    merged = list(prior) + [i for i in ids if i not in prior]
    doc["path"] = [n for n in doc["path"] if n["id"] != path_id]
    doc["extensions"] = [e for e in doc.get("extensions", []) if e["id"] != ext_id]

    node = dict(id=path_id, spine_node=SCRIPT_SPINE_NODE, lessons=merged,
                before=[],
                # Keep what the node already inlines. Tamil's
                # TA-EXT-100-SCRIPT-DRIZZLE is attached here, and dropping it
                # detaches the nine drizzle lessons' extension node.
                inline=sorted({*prior_inline, ext_id}),
                after=[])
    # Placed after the path's own opening stretch rather than at the front: the
    # segments themselves live in chapters 6-13, and a route that visited them
    # first would describe a book nobody reads in that order.
    at = min(3, len(doc["path"]))
    doc["path"] = doc["path"][:at] + [node] + doc["path"][at:]

    doc.setdefault("extensions", []).insert(0, dict(
        id=ext_id, stage="pre-A1", kind="required", category="script",
        canDo=(f"I can pick out each of these {script['name']} characters inside the words "
               f"I already say, and copy its shape by tracing."),
        # This EXTENSION holds every segment on the path node EXCEPT the ones
        # another extension already claims. Both narrower rules were wrong:
        # `merged` put Tamil's drizzle lessons into two extension nodes at once,
        # and `ids` orphaned the eight segments an earlier pass had written for
        # Hindi, which belonged to this extension and nowhere else.
        prerequisites=[], lessons=[i for i in merged if i not in claimed_elsewhere]))

    # The spine map keeps its own ordered list of the path segments that realize
    # each node, and the validator compares the two lists byte for byte. It is a
    # ledger in the same sense as the letter ledger: a second, independent record
    # of the same intent, which is only worth having if something checks it
    # agrees -- and something does.
    realization = doc.setdefault("spine", {}).setdefault(SCRIPT_SPINE_NODE, {})
    realization["segments"] = [n["id"] for n in doc["path"]
                               if n["spine_node"] == SCRIPT_SPINE_NODE]

    with open(p, "w", encoding="utf-8") as f:
        json.dump(doc, f, ensure_ascii=False, indent=2)
        f.write("\n")


if __name__ == "__main__":
    total = 0
    only = [a for a in sys.argv[1:] if not a.startswith("--")]
    for cfg in TRACKS:
        if only and cfg["track"] not in only:
            continue
        b = build(cfg)
        segs = b["segments"]
        out = os.path.join(HL, cfg["track"], "lessons")
        print(f"== {cfg['track']}: {len(segs)} segments")
        for i, s in enumerate(segs):
            prev = segs[i - 1] if i else None
            name = f"{cfg['prefix']}-S{s['n']:02d}-{s['slug']}.md"
            text = render(cfg, b["script"], s, prev)
            assert "\x00" not in text, name
            open(os.path.join(out, name), "w", encoding="utf-8").write(text)
            print(f"   seq {s['seq']:>4} ch {s['chapter']:>2}  {name}  ({len(s['known'])} known words)")
            total += 1
        register(cfg, b["script"], segs)
        print(f"   registered {cfg['prefix']}-PATH-100 and {cfg['prefix']}-EXT-100-SCRIPT-RECOGNITION")
    print(f"\nwrote {total} recognition segments")
