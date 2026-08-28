#!/usr/bin/env python3
# ---------------------------------------------------------------------------
# propose_letter_ledger.py — in what order should a reader meet the letters?
#
# WHY THIS EXISTS
# ---------------
# HL11 says the script is drizzled in one letter at a time behind a course that
# is useful from page one, and that the letters are ordered by the WORDS THEY
# MAKE WRITABLE rather than by the traditional recitation order. That rule needs
# a number to be checkable, and the number is not obvious by eye: it depends on
# which letters the opening vocabulary happens to share.
#
# So this script proposes an order and shows its work. It is a PROPOSAL
# GENERATOR, not the source of truth. The committed ledger is authored intent —
# a human reads this output, adjusts it, and commits the result; no validator
# may rewrite it afterwards. That is the same rule `chapters.json` lives under,
# and for the same reason: "not yet decided" and "decided and recorded" are
# different states, and a generator that overwrites the second erases the
# difference.
#
# WHY NOT RECITATION ORDER
# ------------------------
# Every one of these scripts has a traditional order — அ ஆ இ ஈ உ ஊ,
# अ आ इ ई उ ऊ — organised by phonology, for a learner who already SPEAKS the
# language and is learning to write it. It front-loads independent vowels, which
# in an abugida appear in relatively few words, because the vowel that does the
# work in running text is the SIGN on a consonant. Measured against this corpus,
# twelve glyphs in recitation order complete ZERO words. This curriculum's
# reader is the opposite person: they cannot yet speak, and a letter is worth
# exactly what it unlocks.
#
# WHAT IS GROUNDED, AND WHAT IS A JUDGEMENT
# -----------------------------------------
# GROUNDED (read from files, never typed here):
#   * the words — every target-script headword in the track's opening lessons;
#   * the glyphs — taken from those headwords and from `<script>.json`;
#   * DERIVED families — a letter whose `components` in `<script>.json` name
#     another letter of the same script is built out of it (Devanagari records
#     "ध: like द with an extra inner loop"), so the two are taught together.
#     This is extracted mechanically, not asserted.
# A JUDGEMENT (recorded, with its source, in AUTHORED_FAMILIES below):
#   * families a script file states in prose rather than in `components`.
#
# Not one target-script character is typed into this file. Where a glyph is
# needed it is looked up by its official Unicode NAME, the same discipline
# `generate_syllabary.py` follows, so a maintainer who cannot read the script
# can still audit every line.
#
# USAGE
#   python3 propose_letter_ledger.py            # print proposals for all tracks
#   python3 propose_letter_ledger.py --json     # emit ledger JSON to stdout
#   python3 propose_letter_ledger.py --write    # write <script>-ledger.json files
# ---------------------------------------------------------------------------

import argparse
import json
import os
import re
import sys
import unicodedata
from collections import Counter
from sharded_ledger import load_script

HERE = os.path.dirname(os.path.abspath(__file__))
HL = os.path.normpath(os.path.join(HERE, "..", ".."))

# Track -> the Unicode script its letters belong to. Hindi and Sanskrit share
# Devanagari, so they share a ledger; their opening vocabularies differ, and the
# proposal is computed over the union.
TRACKS = {
    "tamil": "TAMIL",
    "telugu": "TELUGU",
    "kannada": "KANNADA",
    "malayalam": "MALAYALAM",
    "hindi": "DEVANAGARI",
    "sanskrit": "DEVANAGARI",
}

SCRIPT_FILE = {
    "TAMIL": "tamil.json",
    "TELUGU": "telugu.json",
    "KANNADA": "kannada.json",
    "MALAYALAM": "malayalam.json",
    "DEVANAGARI": "devanagari.json",
}

# How much of a track counts as "the opening" for ledger purposes. The drizzle
# has to unlock these; later vocabulary orders the tail of the ledger and is not
# what the first fifty lessons are judged on.
OPENING_LESSONS = 40

# How many positions the proposal covers. Beyond this the payoff signal flattens
# and the order should follow the vocabulary being authored, not this script.
LEDGER_POSITIONS = 24

# A letter that unlocks nothing for this many positions after it is taught is
# reported as UNSPENT. This is the Root Ledger's rule (HL10) applied to glyphs:
# an early letter that pays off nowhere is a step the reader climbed for free.
UNSPENT_WINDOW = 6

# Families a script file states in PROSE rather than in `components`, so they
# cannot be extracted mechanically. Each carries the sentence that justifies it,
# and each glyph is named rather than typed.
AUTHORED_FAMILIES = {
    "TAMIL": [
        {
            "names": [
                "TAMIL LETTER NNA",
                "TAMIL LETTER NNNA",
                "TAMIL LETTER NA",
                "TAMIL LETTER RRA",
            ],
            "source": (
                "tamil.json notes: \"several letters share a straight top bar "
                "and are best learned as a family\""
            ),
        },
    ],
}


def named(name):
    """A glyph, looked up by its official Unicode name. Never typed."""
    return unicodedata.lookup(name)


def script_of(ch):
    try:
        return unicodedata.name(ch).split(" ")[0]
    except ValueError:
        return None


def is_mark(ch):
    return unicodedata.category(ch) in ("Mn", "Mc")


def read_frontmatter(text):
    """Flat, dotted frontmatter. Keys are `introduces.knowledge`, never nested."""
    if not text.startswith("---"):
        return {}
    end = text.find("\n---", 3)
    if end < 0:
        return {}
    out, prefix = {}, ""
    for line in text[3:end].splitlines():
        m = re.match(r"^(\s*)([A-Za-z_][A-Za-z0-9_]*):\s*(.*)$", line)
        if not m:
            continue
        indent, key, val = len(m.group(1)), m.group(2), m.group(3).strip()
        if indent == 0:
            if val == "":
                prefix = key + "."
                continue
            prefix = ""
            out[key] = val
        else:
            out[prefix + key] = val
    return out


def load_lessons(track):
    d = os.path.join(HL, track, "lessons")
    rows = []
    for fn in sorted(os.listdir(d)):
        if not fn.endswith(".md"):
            continue
        fm = read_frontmatter(open(os.path.join(d, fn), encoding="utf-8").read())
        raw = fm.get("sequence", "")
        # `sequence` parses as a STRING. Comparing it as a number without
        # converting silently no-ops the sort, and the reading order is the one
        # thing this whole script depends on.
        try:
            seq = int(raw)
        except (TypeError, ValueError):
            seq = None
        rows.append({
            "id": fm.get("id", fn[:-3]),
            "seq": seq,
            "headword": fm.get("headword", "").strip("\"'"),
            "romanization": fm.get("romanization", "").strip("\"'"),
            "gloss": fm.get("gloss", "").strip("\"'"),
        })
    # None sorts last, not first: a lesson with no declared order cannot be
    # allowed to masquerade as the first one.
    rows.sort(key=lambda r: (r["seq"] is None, r["seq"] if r["seq"] is not None else 0, r["id"]))
    return rows


def short_gloss(gloss, limit=36):
    """A gloss short enough to read in a table, cut on a word boundary."""
    gloss = gloss.split(" - ")[0].split(" (")[0].strip()
    if len(gloss) <= limit:
        return gloss
    cut = gloss[:limit].rsplit(" ", 1)[0]
    return cut + "..."


def opening_words(track, script):
    """Distinct target-script headwords in the track's opening, in order."""
    words, seen = [], set()
    for lesson in load_lessons(track)[:OPENING_LESSONS]:
        hw = lesson["headword"]
        if not hw or hw in seen:
            continue
        # A cousin-table row shows one idea in four sister scripts, separated by
        # slashes. HL08 counts those as context for a reader who already knows a
        # relative -- never a reading obligation -- so they must not steer this.
        if "/" in hw:
            continue
        if any(script_of(ch) not in (script, None) and unicodedata.category(ch)[0] == "L"
               for ch in hw):
            continue
        glyphs = {ch for ch in hw if script_of(ch) == script}
        if not glyphs:
            continue
        # A script lesson's headword is sometimes the vowel sign itself. That is
        # a letter being taught, not a word being unlocked, so it must not count
        # as payoff -- otherwise a mark appears to justify itself.
        if not any(not is_mark(ch) for ch in glyphs):
            continue
        seen.add(hw)
        words.append({
            "word": hw,
            "glyphs": glyphs,
            "romanization": lesson["romanization"] or short_gloss(lesson["gloss"]),
            "lesson": lesson["id"],
        })
    return words


def derived_families(script):
    """Families the script file's own `components` already state."""
    logical_name = SCRIPT_FILE[script]
    data = load_script(HL, logical_name.removesuffix(".json"))
    inventory = {l["glyph"] for l in data.get("letters", []) if len(l.get("glyph", "")) == 1}
    families = []
    for letter in data.get("letters", []):
        glyph = letter.get("glyph", "")
        if len(glyph) != 1:
            continue
        refs = {ch for part in letter.get("components", [])
                for ch in part if ch in inventory and ch != glyph}
        for ref in sorted(refs):
            families.append({
                "names": [unicodedata.name(ref), unicodedata.name(glyph)],
                "source": f"{logical_name}: components of {unicodedata.name(glyph)} "
                          f"name {unicodedata.name(ref)}",
            })
    return families


def families_for(script):
    """Every family, as a list of (glyph tuple, justification)."""
    out = []
    for entry in derived_families(script) + AUTHORED_FAMILIES.get(script, []):
        glyphs = tuple(named(n) for n in entry["names"])
        out.append((glyphs, entry["source"]))
    return out


def propose(script, tracks):
    """Order the letters by what they make writable, keeping families together."""
    words = []
    for track in tracks:
        words.extend(opening_words(track, script))
    # Two tracks over one script can teach the same word; count it once.
    unique, seen = [], set()
    for w in words:
        if w["word"] in seen:
            continue
        seen.add(w["word"])
        unique.append(w)
    words = unique
    if not words:
        return None

    freq = Counter()
    for w in words:
        freq.update(w["glyphs"])

    fam_of, fam_source = {}, {}
    for glyphs, source in families_for(script):
        for ch in glyphs:
            fam_of[ch] = glyphs
            fam_source[ch] = source

    taught, order = set(), []
    while len(order) < LEDGER_POSITIONS:
        remaining = [w for w in words if not w["glyphs"] <= taught]
        candidates = Counter()
        for w in remaining:
            candidates.update(w["glyphs"] - taught)

        # A vowel sign has nothing to sit on until a consonant exists. These are
        # abugidas: a mark MODIFIES a base letter, and the vowel-killer removes
        # a vowel a base letter is carrying. Teaching one first would be showing
        # the reader a correction to a word they have not been shown -- so no
        # mark may take a position until at least one letter has.
        #
        # This is the one place where payoff does not get to decide. It costs a
        # position or two and it is not negotiable, because the alternative is a
        # lesson that cannot be written down.
        if not any(not is_mark(ch) for ch in taught):
            base_only = {ch: n for ch, n in candidates.items() if not is_mark(ch)}
            if base_only:
                candidates = Counter(base_only)

        if not candidates:
            break

        best, best_key = None, None
        for ch in sorted(candidates):
            # A letter drags its family in with it, so the whole group is scored.
            group = set(fam_of.get(ch, (ch,)))
            after = taught | group
            completes = sum(1 for w in remaining if w["glyphs"] <= after)
            key = (completes, freq[ch], -len(group), -ord(ch))
            if best_key is None or key > best_key:
                best, best_key = ch, key

        for ch in fam_of.get(best, (best,)):
            if ch in taught or len(order) >= LEDGER_POSITIONS:
                continue
            before = set(taught)
            taught.add(ch)
            unlocked = [w for w in words
                        if w["glyphs"] <= taught and not w["glyphs"] <= before]
            order.append({
                "position": len(order) + 1,
                "glyph": ch,
                # The code point in hex, beside the name. Together they pin the
                # row numerically: a reviewer who cannot read the script can
                # check "U+0BB1" against "TAMIL LETTER RRA" without trusting the
                # rendered glyph, which may be a lookalike from another script or
                # may be carrying invisible passengers. The TypeScript validator
                # has no Unicode name database, so this is what it checks against.
                "codePoint": f"U+{ord(ch):04X}",
                "unicodeName": unicodedata.name(ch),
                "kind": "vowel-sign" if is_mark(ch) else "letter",
                "family": "".join(fam_of[ch]) if ch in fam_of else None,
                "familySource": fam_source.get(ch),
                "unlocks": [
                    {"word": w["word"], "romanization": w["romanization"], "lesson": w["lesson"]}
                    for w in unlocked
                ],
            })
    return words, order


def unspent(order):
    """Letters that unlock nothing within UNSPENT_WINDOW positions of arriving."""
    out = []
    for i, entry in enumerate(order):
        if entry["unlocks"]:
            continue
        # Only judge a letter whose whole window fits inside the proposal.
        # A letter in the last few positions has not been given its chance yet,
        # and reporting it would be an artifact of where the list stops.
        if i + UNSPENT_WINDOW >= len(order):
            continue
        if not any(e["unlocks"] for e in order[i + 1:i + UNSPENT_WINDOW + 1]):
            out.append(entry["glyph"])
    return out


def ledger_json(script, tracks, words, order):
    return {
        "script": script.lower(),
        "version": 1,
        "note": (
            "Letter ledger (HL11 section 4). AUTHORED INTENT: proposed by "
            "propose_letter_ledger.py, then reviewed and committed by hand. A "
            "validator may check this file and may never rewrite it. Letters are "
            "ordered by the words they make writable, not by recitation order, "
            "because this curriculum's reader cannot yet speak the language and a "
            "letter is worth exactly what it unlocks."
        ),
        "tracks": sorted(tracks),
        "openingLessons": OPENING_LESSONS,
        "openingWords": len(words),
        "letters": order,
    }


def report(script, tracks, words, order):
    print("=" * 78)
    print(f"{script}  ({', '.join(sorted(tracks))})")
    print("=" * 78)
    print(f"  {len(words)} distinct target-script words in the first "
          f"{OPENING_LESSONS} lessons of each track")
    taught = set()
    for entry in order:
        taught.add(entry["glyph"])
        writable = sum(1 for w in words if w["glyphs"] <= taught)
        kind = " (vowel sign)" if entry["kind"] == "vowel-sign" else ""
        fam = "  [family]" if entry["family"] else ""
        line = f"  {entry['position']:>2}. {entry['glyph']}{kind}{fam}"
        if entry["unlocks"]:
            line += "  ->  " + ", ".join(
                f"{u['word']} ({u['romanization']})" if u["romanization"] else u["word"]
                for u in entry["unlocks"])
        print(line)
        if entry["position"] in (8, 16, 24):
            print(f"      ... {writable} of {len(words)} opening words writable")
    dead = unspent(order)
    if dead:
        print(f"  UNSPENT (nothing unlocked within {UNSPENT_WINDOW} positions): "
              + " ".join(dead))
    print()


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--json", action="store_true", help="emit ledger JSON to stdout")
    ap.add_argument("--write", action="store_true", help="write <script>-ledger.json files")
    args = ap.parse_args()

    by_script = {}
    for track, script in TRACKS.items():
        by_script.setdefault(script, []).append(track)

    out = {}
    for script, tracks in sorted(by_script.items()):
        result = propose(script, tracks)
        if not result:
            print(f"{script}: no target-script headwords in the opening", file=sys.stderr)
            continue
        words, order = result
        out[script] = ledger_json(script, tracks, words, order)
        if args.write:
            path = os.path.join(HERE, f"{script.lower()}-ledger.json")
            with open(path, "w", encoding="utf-8") as fh:
                json.dump(out[script], fh, ensure_ascii=False, indent=2)
                fh.write("\n")
            print(f"wrote {os.path.relpath(path, HL)}", file=sys.stderr)
        elif not args.json:
            report(script, tracks, words, order)

    if args.json:
        json.dump(out, sys.stdout, ensure_ascii=False, indent=2)
        sys.stdout.write("\n")


if __name__ == "__main__":
    main()
