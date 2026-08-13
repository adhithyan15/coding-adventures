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
]

CHAPTERS = [6, 7, 8, 9, 10, 11, 12, 13]

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

    sounds = {}
    for l in S["letters"]:
        sounds.setdefault(l["glyph"], l.get("sound", ""))
    for l in S.get("independentVowels", []):
        sounds.setdefault(l["glyph"], l.get("sound", ""))

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
        slots.append(dict(chapter=ch, seq=rows[-1]["seq"] + 5, spine=SCRIPT_SPINE_NODE))

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
    out = []
    pending = list(L["letters"])
    for slot in slots:
        e = None
        while pending:
            cand = pending.pop(0)
            if any(cand["glyph"] in r["headword"] and r["seq"] < slot["seq"] for r in lessons):
                e = cand
                break
        if e is None:
            break
        n = len(out) + 1
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
        known = [r for r in lessons if r["seq"] < slot["seq"] and g in r["headword"]][:4]
        others = [r for r in lessons if r["seq"] < slot["seq"] and g not in r["headword"] and r["headword"]]
        out.append(dict(n=n, entry=e, glyph=g, sound=sound, slug=slug_of(e, S["name"]),
                        known=known, distractor=others[0] if others else None, **slot))
    return dict(cfg=cfg, script=S, ledger=L, segments=out)


def render(cfg, script, seg, prev):
    g, n = seg["glyph"], seg["n"]
    pfx = cfg["prefix"]
    atom = f"{pfx}-SCRIPT-RECOG-{n:02d}"
    prev_atom = f"{pfx}-SCRIPT-RECOG-{n-1:02d}" if prev else None
    prev_id = f"{pfx}-S{prev['n']:02d}-{prev['slug']}" if prev else None
    requires = [prev_atom] if prev_atom else []
    prereqs = [prev_id] if prev_id else []
    lst = lambda a: "[" + ", ".join(a) + "]"

    v = voicing(seg["entry"], seg["sound"])
    show = shown(seg["entry"], g)
    note = kind_note(seg["entry"], seg["sound"], script["name"])
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
    hunt_line = "  ·  ".join(hunt)

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

You already say these, and every one of them has {show} somewhere inside it:

{known_lines}

## Writing: {show} — copy what you see
<!-- hl-knowledge: introduces=[]; assesses=[{atom}] -->

Put your pen on {show} and follow its line. Copy the shape you can see — slowly,
and larger than it is printed.

> This book does not yet tell you **where to start the character or which way to
> travel**. That is a real thing, taught with real variation from school to
> school, and it is not written down here until it can be written down with a
> source. Copying what is in front of you needs no such source, and it is how the
> shape gets into your hand in the meantime.

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

    doc["path"] = [n for n in doc["path"] if n["id"] != path_id]
    doc["extensions"] = [e for e in doc.get("extensions", []) if e["id"] != ext_id]

    node = dict(id=path_id, spine_node=SCRIPT_SPINE_NODE, lessons=ids,
                before=[], inline=[ext_id], after=[])
    # Placed after the path's own opening stretch rather than at the front: the
    # segments themselves live in chapters 6-13, and a route that visited them
    # first would describe a book nobody reads in that order.
    at = min(3, len(doc["path"]))
    doc["path"] = doc["path"][:at] + [node] + doc["path"][at:]

    doc.setdefault("extensions", []).insert(0, dict(
        id=ext_id, stage="pre-A1", kind="required", category="script",
        canDo=(f"I can pick out each of these {script['name']} characters inside the words "
               f"I already say, and copy its shape by tracing."),
        prerequisites=[], lessons=ids))

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
    for cfg in TRACKS:
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
