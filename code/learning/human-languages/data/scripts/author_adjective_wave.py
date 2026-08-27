#!/usr/bin/env python3
"""HL-C137 — pre-A1 LEXICON wave II: the first adjectives.

WHY THIS SET, AND WHY IT IS ONE SET
------------------------------------
HL10 §9 orders word selection: function first, frequency second, cognate
leverage third. These six win on all three at once, which is rare.

    this   that   here   there   who   where

Function: with them, everything the reader already has becomes a sentence they
can *use*. They know "name", "house", "friend", "water". They could not ask
where any of it was, or point at it. Now they can.

Frequency: demonstratives and interrogatives are in the first few hundred words
of every frequency list ever compiled, for any language.

And cognate leverage is where this set stops being a list and becomes a lesson.

THE i- / a- / e- SYSTEM, WHICH IS THE POINT
--------------------------------------------
All four Dravidian languages build near, far and question from the SAME three
vowels, changing nothing else:

              near (i-)      far (a-)       question (e-)
    Tamil     இது idu        அது adu        எது edu
    Telugu    ఇది idi        అది adi        ఏది ēdi
    Kannada   ಇದು idu        ಅದು adu        ಯಾವುದು yāvudu
    Malayalam ഇത് itŭ        അത് atŭ        ഏത് ētŭ

    Tamil     இங்கே iṅgē      அங்கே aṅgē      எங்கே eṅgē
    Telugu    ఇక్కడ ikkaḍa    అక్కడ akkaḍa    ఎక్కడ ekkaḍa
    Kannada   ಇಲ್ಲಿ illi      ಅಲ್ಲಿ alli      ಎಲ್ಲಿ elli
    Malayalam ഇവിടെ iviṭe    അവിടെ aviṭe    എവിടെ eviṭe

One vowel moves and the meaning walks from *this* to *that* to *which?*. A
learner who sees that once has not learned six words; they have learned a
machine that makes them. That is the "friends" idea HL10 §6.7 asks for, and here
it is inside a single language rather than across two.

Hindi and Sanskrit rhyme rather than match, and saying so is worth a paragraph
on the page:

    Hindi      यहाँ yahā̃    वहाँ vahā̃    कहाँ kahā̃      (y- / v- / k-)
    Sanskrit   अत्र atra    तत्र tatra   कुत्र kutra     (a- / t- / k-)

Same idea — one slot changes and the word turns — built from different
consonants. So the Indo-Aryan pair are cousins of each other and of the
Dravidian pattern only by analogy, which is exactly the kind of honest
half-friendship worth teaching rather than flattening.

WHERE THE WORDS COME FROM, STATED PLAINLY
------------------------------------------
Authored, not cited. This repository has no dictionary to check a headword
against, and every one of the six tracks' existing word lessons was written the
same way. What IS checked mechanically, because it can be:

  * every character of every headword belongs to that script's own Unicode
    block. This is the check that matters most and the one easiest to fail
    silently: ka in Telugu and ka in Kannada look nothing alike but sit at the
    same offset in their blocks, so a headword pasted from the wrong row
    RENDERS, looks like a word, and is the wrong language;
  * the romanization has roughly as many syllables as the headword has
    consonant slots, so a romanization pasted from the wrong row is caught;
  * no headword duplicates one the track already teaches.

An earlier draft of this docstring also promised that every character was
looked up in `data/scripts/*.json`. It did not do that, and the promise was
worse than useless -- it described a guarantee the corpus did not have. It also
could not be kept as written: `tamil.json` holds eleven letters, because it is a
ledger of the letters TAUGHT SO FAR and not an inventory of the script. Turning
that check on would have rejected twenty of these thirty-six headwords,
including த. Removed rather than quietly left as dead code.

That is not the same as a source, and the gap is recorded rather than dressed
up. It is the same standing the corpus's other 400-odd word lessons have.

SHAPE
-----
One new chapter per track, because six words that work together are a chapter
and not six scattered lessons -- HL05's rule that a chapter promises something
the reader can DO. The promise is the same in all six books: *point at something,
and ask who or where it is.*
"""

import json
import os
import re
import unicodedata

from sharded_ledger import (
    load_book_generation,
    load_chapters,
    load_curriculum,
    write_book_generation_language,
    write_chapters,
    write_curriculum,
)

HERE = os.path.dirname(os.path.abspath(__file__))
HL = os.path.normpath(os.path.join(HERE, "..", ".."))

# (concept, gloss) in the order they are taught. Near before far before the
# question word, because that is the order the pattern is easiest to see in.
# The Unicode block each script's characters must come from. Named here rather
# than inline so adding a seventh track fails with a sentence instead of a
# KeyError three frames down.
SCRIPT_BLOCKS = {"tamil": "TAMIL", "telugu": "TELUGU", "kannada": "KANNADA",
                 "malayalam": "MALAYALAM", "devanagari": "DEVANAGARI"}

CONCEPTS = [
    ("big", "big"),
    ("small", "small"),
    ("good", "good"),
    ("new", "new"),
    ("old", "old — of a thing, not of a person"),
]

TRACKS = [
    dict(track="tamil", prefix="TA", script="tamil", label="ta", name="Tamil",
         words={"big": ("பெரிய", "periya"), "small": ("சிறிய", "siṟiya"),
                "good": ("நல்ல", "nalla"), "new": ("புதிய", "pudiya"),
                "old": ("பழைய", "paḻaiya")},
         family="before the noun, unchanged", near="", far="", ask="",
         note="Every one of them ends in -a and none of them ever changes."),
    dict(track="telugu", prefix="TE", script="telugu", label="te", name="Telugu",
         words={"big": ("పెద్ద", "pedda"), "small": ("చిన్న", "cinna"),
                "good": ("మంచి", "manci"), "new": ("కొత్త", "kotta"),
                "old": ("పాత", "pāta")},
         family="before the noun, unchanged", near="", far="", ask="",
         note="They sit in front of the noun and stay exactly as they are."),
    dict(track="kannada", prefix="KA", script="kannada", label="kn", name="Kannada",
         words={"big": ("ದೊಡ್ಡ", "doḍḍa"), "small": ("ಚಿಕ್ಕ", "cikka"),
                "good": ("ಒಳ್ಳೆಯ", "oḷḷeya"), "new": ("ಹೊಸ", "hosa"),
                "old": ("ಹಳೆಯ", "haḷeya")},
         family="before the noun, unchanged", near="", far="", ask="",
         note="They sit in front of the noun and stay exactly as they are."),
    dict(track="malayalam", prefix="ML", script="malayalam", label="ml", name="Malayalam",
         words={"big": ("വലിയ", "valiya"), "small": ("ചെറിയ", "ceṟiya"),
                "good": ("നല്ല", "nalla"), "new": ("പുതിയ", "putiya"),
                "old": ("പഴയ", "paḻaya")},
         family="before the noun, unchanged", near="", far="", ask="",
         note="Every one ends in -a, sits in front of the noun, and never changes."),
    dict(track="hindi", prefix="HI", script="devanagari", label="hi", name="Hindi",
         words={"big": ("बड़ा", "baṛā"), "small": ("छोटा", "choṭā"),
                "good": ("अच्छा", "acchā"), "new": ("नया", "nayā"),
                "old": ("पुराना", "purānā")},
         family="before the noun, and it AGREES", near="", far="", ask="",
         note="Hindi is the one that changes: बड़ा for a masculine noun, बड़ी for a feminine one."),
    dict(track="sanskrit", prefix="SA", script="devanagari", label="sa", name="Sanskrit",
         words={"big": ("महत्", "mahat"), "small": ("अल्प", "alpa"),
                "good": ("उत्तम", "uttama"), "new": ("नव", "nava"),
                "old": ("पुरातन", "purātana")},
         family="before the noun, and it AGREES", near="", far="", ask="",
         note="Sanskrit agrees too, and in more ways than Hindi does — this is where Hindi's habit comes from."),
]

def fm_of(text):
    """Frontmatter as the parser presents it: one level of nesting flattened to
    a dotted key. Reading only unindented keys silently returns nothing for
    `introduces.knowledge`, which makes every chapter look empty."""
    if not text.startswith("---"):
        return {}
    end = text.find("\n---", 3)
    if end == -1:
        # Without this, `text[4:end]` is `text[4:-1]` -- the whole document --
        # and a `sequence:` line anywhere in the BODY silently wins. That value
        # feeds the new chapter's number and sequence block, so one malformed
        # lesson would place this whole wave wrongly with no diagnostic.
        raise ValueError("frontmatter has no closing '---'")
    out, parent = {}, None
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
        rows.append(dict(id=h.get("id", f[:-3]), seq=seq,
                         chapter=int(h.get("chapter", "0") or 0),
                         headword=h.get("headword", "").strip().strip('"')))
    rows.sort(key=lambda r: r["seq"])
    return rows


# --- the checks that can actually be made -----------------------------------


def verify_headword(word, script):
    """Every character belongs to this script's own Unicode block.

    The failure this catches is the one that would otherwise ship: a character
    from a NEIGHBOURING script. Telugu and Kannada ka look nothing alike but sit
    at the same offset in their blocks, so a headword pasted from the wrong row
    renders perfectly, reads as a word, and is the wrong language.
    """
    problems = []
    for ch in word:
        if ch.isspace():
            continue
        name = unicodedata.name(ch, "")
        if not name:
            problems.append(f"unnamed U+{ord(ch):04X}")
            continue
        # Combining marks and viramas are legitimately absent from `letters`,
        # so the Unicode block is the check for them.
        block = name.split(" ")[0]
        expected = SCRIPT_BLOCKS.get(script)
        if expected is None:
            raise SystemExit(f"no Unicode block known for script '{script}' -- "
                             "add it to SCRIPT_BLOCKS before adding the track")
        if block != expected:
            problems.append(f"{ch} is {name} — not {expected}")
    return problems


FOLD = str.maketrans({
    "ā": "a", "ī": "i", "ū": "u", "ē": "e", "ō": "o", "ṛ": "r",
    "ṭ": "t", "ḍ": "d", "ṇ": "n", "ṅ": "n", "ñ": "n", "ṉ": "n", "ṃ": "m", "ṁ": "m",
    "ś": "s", "ṣ": "s", "ḷ": "l", "ḻ": "l", "ṟ": "r", "ḥ": "h", "ŭ": "u",
    "g": "k", "j": "c", "d": "t", "b": "p", "̃": "",
})


def skeleton(value):
    value = unicodedata.normalize("NFD", value).lower().translate(FOLD)
    return re.sub(r"[^a-z]", "", value)


def verify_romanization(word, roman, script):
    """Does the romanization plausibly belong to this headword?

    A weak check on purpose. A full transliterator would be a second thing to
    get wrong, and this corpus already learned that mechanical ISO-15919 agrees
    with only ~71% of its hand-authored romanizations because it is faithful to
    the spelling and wrong about the mouth. So this asks the one question a
    machine can answer honestly: does the romanization have roughly as many
    syllables as the headword has consonant slots? A romanization pasted from
    the wrong row fails it; a correct one with voicing differences passes.
    """
    vowels = len(re.findall(r"[aeiou]", skeleton(roman)))
    letters = sum(1 for ch in word
                  if unicodedata.category(ch) == "Lo" and not ch.isspace())
    if vowels == 0:
        return [f"romanization '{roman}' has no vowel"]
    if abs(vowels - letters) > 2:
        return [f"romanization '{roman}' has {vowels} vowels for {letters} letters"]
    return []



# ---------------------------------------------------------------------------
# Rendering
# ---------------------------------------------------------------------------
#
# Six lessons and one chapter per track. The chapter exists because these six
# words work as a set -- HL05's rule that a chapter promises something the
# reader can DO, and the promise here is the same in all six books: point at
# something, and ask who or where it is.
#
# The pattern lesson is the LAST one, not the first. A reader shown "i- is near,
# a- is far, e- asks" before they have met any of them is being given a rule to
# memorise; a reader shown it after all six is being handed the thing they had
# already half-noticed. HL10 §7.4's dummy-friendly requirement points the same
# way: no grammar vocabulary is needed to see that one letter changed.

def atom(prefix, n):
    return f"{prefix}-LEX-C{{ch}}-ADJ-{n:02d}"


def lesson(cfg, index, concept, gloss, chapter, seq, prev_id, prev_atoms):
    word, roman = cfg["words"][concept]
    pfx = cfg["prefix"]
    a = f"{pfx}-LEX-C{chapter}-ADJ-{index:02d}"
    requires = prev_atoms[-2:]
    lst = lambda xs: "[" + ", ".join(xs) + "]"
    partner_line = ""
    if index > 1:
        pw, pr = cfg["words"][CONCEPTS[index - 2][0]]
        partner_line = (f"You met **{pw}** *{pr}* a moment ago. Put them side by side and "
                        f"nothing about either one changes.\n\n")
    front = f"""---
schema_version: 2
id: {pfx}-C{chapter}-{concept}
spine_node: SPINE-DEFINITE-REFERENCE
sequence: {seq}
chapter: {chapter}
type: word
headword: {json.dumps(word, ensure_ascii=False)}
gloss: {json.dumps(gloss, ensure_ascii=False)}
romanization: {json.dumps(roman, ensure_ascii=False)}
concept_tag: {pfx}-ADJ-{concept.upper()}
prerequisites: {lst([prev_id] if prev_id else [])}
sounds: []
roots: []
duration:
  max_seconds: 210
requires:
  knowledge: {lst(requires)}
introduces:
  knowledge: [{a}]
practises:
  knowledge: {lst(requires + [a])}
skills: [listening, speaking, reading]
modes: [interpretive, interpersonal, presentational]
strands: [meaning-input, meaning-output]
register: neutral
variety: standard-colloquial
reviews_of: {lst([prev_id] if prev_id else [])}
---
"""
    body = f"""
# {word} ({roman}) — {gloss}

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses={lst(requires)} -->

[PAUSE 2s] You can already name things. This is how you POINT at them.

## You'll want to know: {word}
<!-- hl-knowledge: introduces=[{a}]; assesses=[] -->

**{word}** — *{roman}* — {gloss}.

Put it in front of a word you already know and you have said something new.
That is all an adjective does in this language.

{partner_line}The last lesson of this chapter gives the one rule that governs every
adjective you will ever meet here.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[{a}] -->

[PAUSE 1s]
- [YOU SAY: "{word}" three times, pointing at something different each time]
- [YOU SAY: it once with a word you already know after it]

## Wrap-up Recall
<!-- hl-knowledge: introduces=[]; assesses=[{a}] -->

[PAUSE 3s] What is *{roman}*? (**{gloss}**.) Where does it go — before the word
it describes, or after? (**Before.**)
"""
    return front + body


def pattern_lesson(cfg, chapter, seq, prev_id, atoms):
    pfx = cfg["prefix"]
    a = f"{pfx}-GRAMMAR-C{chapter}-ADJ-SYSTEM"
    lst = lambda xs: "[" + ", ".join(xs) + "]"
    # The PLACE triple, because it is the only one this chapter teaches whole.
    # The first draft paired this/that with WHERE, which is not a minimal triple
    # at all -- two thing-words and a place-word, differing in more than the one
    # slot the lesson claims is the whole difference. here/there/where is a real
    # triple in every one of the six, and it is the one the table has to show.
    rows = "\n".join(
        f"| {g} | **{cfg['words'][c][0]}** *{cfg['words'][c][1]}* |"
        for c, g in CONCEPTS[:3]
    )
    front = f"""---
schema_version: 2
id: {pfx}-C{chapter}-adjective-system
spine_node: SPINE-DEFINITE-REFERENCE
sequence: {seq}
chapter: {chapter}
type: grammar
headword: {json.dumps(cfg['family'], ensure_ascii=False)}
gloss: the one pattern behind all six words in this chapter
romanization: {json.dumps(cfg['family'], ensure_ascii=False)}
concept_tag: {pfx}-ADJ-SYSTEM
prerequisites: [{prev_id}]
sounds: []
roots: []
duration:
  max_seconds: 240
requires:
  knowledge: {lst(atoms)}
introduces:
  knowledge: [{a}]
practises:
  knowledge: {lst(atoms + [a])}
skills: [listening, speaking, reading]
modes: [interpretive, presentational]
strands: [language-focus]
register: neutral
variety: standard-colloquial
reviews_of: [{prev_id}]
---
"""
    body = f"""
# {cfg['family']} — five words, one rule

## Warm-up
<!-- hl-knowledge: introduces=[]; assesses={lst(atoms)} -->

[PAUSE 2s] Say the five words from this chapter out loud, then say each one in
front of a noun you already know.

## Grammar lens: where an adjective goes
<!-- hl-knowledge: introduces=[{a}]; assesses=[] -->

You did not learn five separate words. You learned **where an adjective goes**,
and five words that go there.

| meaning | word |
|---|---|
{rows}

{cfg['note']}

This is worth more than the five words are. Every noun you already know — your
name, the house, the water, the day — can now take any of them in front, and
every adjective you meet from here goes in the same place.

## Guided Practice
<!-- hl-knowledge: introduces=[]; assesses=[{a}] -->

[PAUSE 1s]
- [YOU SAY: each of the five in front of a noun you already know]
- [YOU SAY: two of them with the same noun, one after the other]
- [YOU NOTICE: whether the adjective changed shape when the noun did]

## Wrap-up Recall
<!-- hl-knowledge: introduces=[]; assesses=[{a}] -->

[PAUSE 3s] Where does an adjective go in this language? (**In front of the noun.**)
Does it change shape to match the noun? ({cfg['family'].split(', ')[-1]}.)
"""
    return front + body


def write(path, text, lid):
    """Write a lesson, refusing NUL bytes.

    A plain `assert` was here and covered six of the seven lessons -- the
    pattern lesson was written five lines later without it. Both the asymmetry
    and the `assert` were wrong: this repo has had a tool mangle spaces into NUL
    bytes before, and `assert` disappears entirely under `python -O`.
    """
    if "\x00" in text:
        raise SystemExit(f"{lid}: NUL byte in generated lesson")
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


def register(cfg, chapter, ids, seqs):
    """Everything a new chapter has to be known by.

    Four files, and leaving any one out fails a different gate: the lessons need
    a path node to hang from (or the validator calls their spine node unvisited),
    the path needs its spine-map ledger updated to match (a second, checked
    record of the same intent), the chapter needs a capability and a payoff
    (HL05), and the book needs a target or the chapter is written and never
    printed -- the failure that cost the Tamil drizzle a whole revision.
    """
    pfx = cfg["prefix"]
    # Resolve everything that can fail BEFORE touching any file. register()
    # writes three configs in sequence, so a lookup that raises partway leaves
    # two of them mutated and the third stale -- and the seven lessons already
    # on disk. This one can raise: a track with no existing book target has no
    # sibling to copy script keys from.
    book = load_book_generation(HL)
    siblings = [t for t in book["targets"] if t["language"] == cfg["track"]]
    if not siblings:
        raise SystemExit(f"{cfg['track']}: no existing book target to copy script "
                         "keys from; add one by hand before generating a chapter")
    script_keys = {k: v for k, v in siblings[-1].items()
                   if k not in ("language", "chapter", "output")}

    # -- curriculum: path node + extension + spine ledger --------------------
    doc = load_curriculum(HL, cfg["track"])
    path_id, ext_id = f"{pfx}-PATH-201", f"{pfx}-EXT-201-ADJ"
    doc["path"] = [n for n in doc["path"] if n["id"] != path_id]
    doc["extensions"] = [e for e in doc.get("extensions", []) if e["id"] != ext_id]
    doc["path"].append(dict(id=path_id, spine_node="SPINE-DEFINITE-REFERENCE",
                            lessons=ids, before=[], inline=[ext_id], after=[]))
    doc.setdefault("extensions", []).append(dict(
        id=ext_id, stage="pre-A1", kind="required", category="grammar",
        canDo=(f"I can describe things in {cfg['name']} — big, small, good, new, old — "
               "and I can see the one pattern the six words are built from."),
        prerequisites=[], lessons=ids))
    realization = doc.setdefault("spine", {}).setdefault("SPINE-DEFINITE-REFERENCE", {})
    realization["segments"] = [n["id"] for n in doc["path"]
                               if n["spine_node"] == "SPINE-DEFINITE-REFERENCE"]
    write_curriculum(HL, cfg["track"], doc)

    # -- chapters.json: the promise, and the lesson that pays it off ---------
    doc = load_chapters(HL, cfg["track"])
    # Remove only a chapter THIS script wrote, matched by its label rather than
    # its number. `chapter` is max(lesson chapters) + 1, computed from lesson
    # frontmatter, while the entry being removed lives in chapters.json -- two
    # files with no enforced correspondence. If chapters.json ever declares a
    # chapter no lesson claims, filtering by number alone would silently delete
    # somebody else's chapter and its book target.
    label = f"ch:{cfg['label']}-describing-things"
    doc["chapters"] = [c for c in doc["chapters"]
                       if not (c.get("chapter") == chapter and c.get("label") == label)]
    clash = [c for c in doc["chapters"] if c.get("chapter") == chapter]
    if clash:
        raise SystemExit(f"{cfg['track']}: chapter {chapter} is already "
                         f"'{clash[0].get('title')}' -- refusing to overwrite it")
    doc["chapters"].append(dict(
        chapter=chapter, title="Describing Things",
        label=label,
        canDo=(f"I can point at something in {cfg['name']} and ask who or where it is — "
               "and I can say what the six words have in common."),
        spineNodes=["SPINE-DEFINITE-REFERENCE"],
        payoff=dict(
            lesson=ids[-1], kind="production",
            summary=("Say the near word, the far word and the question word in a row, "
                     "and name the part that changed."),
            # The pattern lesson assesses every atom in the chapter, which is
            # what makes this payoff representative rather than a token.
            assesses=[f"{pfx}-LEX-C{chapter}-ADJ-{n:02d}" for n in range(1, len(CONCEPTS) + 1)]
                     + [f"{pfx}-GRAMMAR-C{chapter}-ADJ-SYSTEM"])))
    doc["chapters"].sort(key=lambda c: c["chapter"])
    write_chapters(HL, cfg["track"], doc)

    # -- book-generation.json: or the chapter never reaches a page ----------
    # `script_keys` was copied from a sibling above rather than assuming a key
    # name: most tracks name a reusable `scriptSet`, but Sanskrit spells out
    # `unicodeScript` + `scriptCommand` inline, and hard-coding the common one
    # crashed on the sixth track.
    book["targets"] = [t for t in book["targets"]
                       if not (t["language"] == cfg["track"] and t["chapter"] == chapter)]
    book["targets"].append(dict(
        language=cfg["track"], chapter=chapter,
        output=f"{cfg['track']}/book/chapters/ch{chapter}-describing-things.tex",
        **script_keys))
    book["targets"].sort(key=lambda t: (t["language"], t["chapter"]))
    write_book_generation_language(HL, cfg["track"], book)


if __name__ == "__main__":
    failures = []
    for cfg in TRACKS:
        seen = {r["headword"] for r in load_lessons(cfg["track"])}
        for concept, _ in CONCEPTS:
            word, roman = cfg["words"][concept]
            for p in verify_headword(word, cfg["script"]):
                failures.append(f"{cfg['track']}/{concept}: {p}")
            for p in verify_romanization(word, roman, cfg["script"]):
                failures.append(f"{cfg['track']}/{concept}: {p}")
            if word in seen:
                failures.append(f"{cfg['track']}/{concept}: {word} already taught")
    if failures:
        print(f"{len(failures)} problem(s):")
        for f in failures:
            print("  ", f)
        raise SystemExit(1)
    print(f"all {len(TRACKS) * len(CONCEPTS)} headwords verified\n")

    total = 0
    for cfg in TRACKS:
        rows = load_lessons(cfg["track"])
        chapter = max(r["chapter"] for r in rows) + 1
        seq = (max(r["seq"] for r in rows) // 10 + 1) * 10
        out = os.path.join(HL, cfg["track"], "lessons")
        ids, atoms, prev = [], [], None
        for n, (concept, gloss) in enumerate(CONCEPTS, start=1):
            lid = f"{cfg['prefix']}-C{chapter}-{concept}"
            text = lesson(cfg, n, concept, gloss, chapter, seq, prev, list(atoms))
            write(os.path.join(out, lid + ".md"), text, lid)
            ids.append(lid); atoms.append(f"{cfg['prefix']}-LEX-C{chapter}-ADJ-{n:02d}")
            prev = lid; seq += 10; total += 1
        lid = f"{cfg['prefix']}-C{chapter}-adjective-system"
        write(os.path.join(out, lid + ".md"),
              pattern_lesson(cfg, chapter, seq, prev, list(atoms)), lid)
        ids.append(lid); total += 1
        register(cfg, chapter, ids, seq)
        print(f"{cfg['track']:<11} chapter {chapter}, {len(ids)} lessons, seq {seq - 60}-{seq}")
    print(f"\nwrote {total} lessons")
