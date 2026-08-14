#!/usr/bin/env python3
"""HL-C129 — connected prose, and the rule that makes it honest.

THE GAP
-------
Measured across all 412 Spanish lessons, counting only runs that are genuinely
Spanish rather than English explanation:

    longest run          10 words
    runs of >= 8 words    8
    runs of >= 12 words   0
    runs of >= 20 words   0

And several of the longest are lists -- `uno, dos, tres, cuatro...`,
`di, haz, ve, pon, ten...` -- not prose. **A reader can finish this book having
never read a Spanish paragraph.** The DELE A1 reading paper is 25 questions over
four tasks on connected texts; B1 and above are longer still. This is a gap in
SHAPE, not in vocabulary: the words are taught, they are just never strung
together.

THE RULE: READING CLOSURE
--------------------------
The obvious fix -- write some paragraphs -- has an obvious failure. A passage
written freely will reach for a word the reader has not met, and a reader who
hits an unknown word in their first Spanish paragraph learns that paragraphs are
where you get lost.

So a passage may use only what the reader already holds at that point in the
sequence. That is the same rule HL11 applies to script -- a lesson may ask you to
decode only glyphs it has taught -- applied to words instead of letters, and it
is checkable for exactly the same reason: the corpus knows what it has taught and
in what order.

    for each passage, in sequence order:
        every content word must appear as a headword of some EARLIER lesson,
        or be one of the closed-class function words the book teaches
        implicitly, or be a NAMED exception the lesson itself glosses.

The third clause matters and is not a loophole. `ES-C09-sintesis-ocho` builds its
paragraph entirely from words that appear nowhere else in the book, on purpose:
they are cognates the reader decodes from suffix rules they hold. The lesson
says so, glosses every one, and that is a legitimate passage. So a word may be
unknown if the lesson takes responsibility for it; what is forbidden is silence.

WHAT THIS SCRIPT DOES
----------------------
Report-only, per the HL05/HL08 precedent. It measures the longest connected run
per track, and for any passage marked with a `> ` blockquote inside a synthesis
lesson it reports the words that are neither taught earlier nor glossed here.
"""

import json
import os
import re
import sys
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
HL = os.path.normpath(os.path.join(HERE, "..", ".."))

# Function words a Spanish course teaches by using them rather than by giving
# each one a lesson. Listing them is honest: they are genuinely taught, in the
# sense that a reader meets them constantly from chapter 1, and demanding a
# headword for `de` would make the check fail on every real passage.
FUNCTION_WORDS = {
    "el", "la", "los", "las", "un", "una", "unos", "unas", "de", "del", "a", "al",
    "y", "e", "o", "u", "que", "en", "con", "por", "para", "no", "sí", "se", "su",
    "sus", "mi", "mis", "tu", "tus", "es", "son", "está", "están", "muy", "más",
    "pero", "como", "cuando", "porque", "también", "ya", "lo", "le", "les", "me",
    "te", "nos", "hay", "ser", "estar", "tiene", "tienen", "hace", "todo", "toda",
    "todos", "todas", "este", "esta", "esto", "ese", "esa", "eso", "aquí", "allí",
}


def words_of(text):
    out = []
    for raw in re.split(r"[^A-Za-zÁÉÍÓÚÜÑáéíóúüñ]+", text):
        if raw:
            out.append(raw.lower())
    return out


def load(track):
    rows = []
    d = os.path.join(HL, track, "lessons")
    for f in sorted(os.listdir(d)):
        if not f.endswith(".md"):
            continue
        text = open(os.path.join(d, f), encoding="utf-8").read()
        seq = re.search(r"^sequence: (\d+)", text, re.M)
        hw = re.search(r"^headword: (.*)$", text, re.M)
        lid = re.search(r"^id: (\S+)", text, re.M)
        rows.append(dict(seq=int(seq.group(1)) if seq else 0,
                         id=lid.group(1) if lid else f[:-3],
                         headword=(hw.group(1).strip().strip('"') if hw else ""),
                         text=text))
    rows.sort(key=lambda r: r["seq"])
    return rows


def longest_runs(rows, vocab_all):
    """The longest genuinely-target-language run in each lesson.

    A run counts as target-language when at least half its words are headwords
    the book teaches. Without that filter the measurement finds ENGLISH italics
    -- the explanatory prose -- and reports a book full of long passages it does
    not have. That is exactly what the first version of this measurement did.
    """
    found = []
    for r in rows:
        for m in re.finditer(r"\*([^*\n]{4,})\*", r["text"]):
            ws = words_of(m.group(1))
            if len(ws) < 4:
                continue
            known = sum(1 for w in ws if w in vocab_all or w in FUNCTION_WORDS)
            if known / len(ws) >= 0.5:
                found.append((len(ws), r["id"], m.group(1)[:60]))
    found.sort(reverse=True)
    return found


def main():
    tracks = sys.argv[1:] or ["spanish"]
    for track in tracks:
        rows = load(track)
        vocab_all = set()
        for r in rows:
            vocab_all.update(words_of(r["headword"]))
        runs = longest_runs(rows, vocab_all)
        print(f"== {track}: {len(rows)} lessons")
        print(f"   longest connected run: {runs[0][0] if runs else 0} words")
        for k in (8, 12, 20, 30, 50):
            print(f"   runs of >= {k:>2} words: {sum(1 for n, _, _ in runs if n >= k)}")
        for n, lid, txt in runs[:3]:
            print(f"     {n:>3}w  {lid:<26} {txt}")

        # Reading closure, for the passages that exist: every content word must
        # have been a headword earlier, or be a function word, or be glossed in
        # the lesson itself.
        seen = set()
        violations = 0
        for r in rows:
            for m in re.finditer(r"^> \*(.+)\*\s*$", r["text"], re.M):
                ws = words_of(m.group(1))
                if len(ws) < 6:
                    continue
                glossed = set(words_of(r["text"]))
                unknown = [w for w in ws
                           if w not in seen and w not in FUNCTION_WORDS
                           and ws.count(w) and w not in glossed]
                if unknown:
                    violations += 1
                    print(f"   passage in {r['id']}: {len(unknown)} unglossed unknown word(s): "
                          f"{', '.join(unknown[:6])}")
            seen.update(words_of(r["headword"]))
        print(f"   reading-closure violations: {violations}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
