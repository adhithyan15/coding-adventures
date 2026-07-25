#!/usr/bin/env python3
# ---------------------------------------------------------------------------
# generate_syllabary.py — Telugu / Kannada / Malayalam syllabaries, from Unicode.
#
# WHY THIS EXISTS (and why it is a generator, not a hand-authored table).
# These three Dravidian scripts are abugidas: a base consonant carries an
# inherent /a/, and a vowel sign turns it into a syllable — క = ka, కి = ki,
# కు = ku; ఖ = kha, ఖి = khi, ఖు = khu. The study app teaches them as pattern
# recognition, one consonant across its vowel row. That is hundreds of syllables
# per script, and they are scripts many maintainers cannot proofread by eye — so
# hand-typing the glyphs would be both tedious and unverifiable.
#
# GROUNDING. Every glyph here is COMPOSED FROM UNICODE CODE POINTS, never typed
# from memory. The identity and romanization of each base consonant / vowel sign
# come from its official Unicode character NAME (e.g. "TELUGU LETTER KA",
# "KANNADA VOWEL SIGN II"), mapped to ISO-15919 transliteration. A base consonant
# whose Unicode name we do not have a transliteration for is SKIPPED rather than
# guessed — the output only ever contains letters we can name from the standard.
# So the Unicode Character Database is the single source of truth, and the file
# is regenerable: `python3 generate_syllabary.py`.
#
# NOT INCLUDED: stroke order / how to hand-write ("ductus"). That is a separate,
# source-gated effort and stays paused — these entries carry `strokeOrder: []`.
# This file is for READING (recognition), which the glyph + romanization + the
# consonant⊕vowel-sign decomposition fully support.
# ---------------------------------------------------------------------------

import json
import os
import unicodedata

# ISO-15919 romanization keyed on the Unicode consonant token (the "<X>" in
# "<SCRIPT> LETTER <X>"). The order here IS the traditional varga order the
# scripts are taught in, so iterating it yields a pedagogical, consonant-major
# sequence. A script that lacks one of these simply won't have that code point.
CONSONANTS = [
    ("KA", "ka"), ("KHA", "kha"), ("GA", "ga"), ("GHA", "gha"), ("NGA", "ṅa"),
    ("CA", "ca"), ("CHA", "cha"), ("JA", "ja"), ("JHA", "jha"), ("NYA", "ña"),
    ("TTA", "ṭa"), ("TTHA", "ṭha"), ("DDA", "ḍa"), ("DDHA", "ḍha"), ("NNA", "ṇa"),
    ("TA", "ta"), ("THA", "tha"), ("DA", "da"), ("DHA", "dha"), ("NA", "na"),
    ("PA", "pa"), ("PHA", "pha"), ("BA", "ba"), ("BHA", "bha"), ("MA", "ma"),
    ("YA", "ya"), ("RA", "ra"), ("LA", "la"), ("VA", "va"),
    ("SHA", "śa"), ("SSA", "ṣa"), ("SA", "sa"), ("HA", "ha"),
    ("LLA", "ḷa"), ("RRA", "ṟa"), ("NNNA", "ṉa"),
]

# The Dravidian core vowel set (short + long e/o — the distinction is phonemic in
# these languages). The empty key is the inherent /a/ (the bare consonant).
# Keyed on the Unicode vowel-sign token ("<SCRIPT> VOWEL SIGN <V>"); the roman is
# the vowel added AFTER the consonant root. `ai/au/ṛ` etc. are left for later.
VOWELS = [
    ("", "a"),      # inherent — no sign
    ("AA", "ā"), ("I", "i"), ("II", "ī"), ("U", "u"), ("UU", "ū"),
    ("E", "e"), ("EE", "ē"), ("O", "o"), ("OO", "ō"),
]

# The three scripts: (script id, display name, Unicode block base, the Noto face
# the glyphs were verified against — metadata only; the app renders with system
# fonts). `signature` is a factual "how to spot it" cue, verified by rendering.
SCRIPTS = [
    ("telugu", "Telugu", 0x0C00, "_fonts/NotoSansTelugu-Static.ttf",
     "Rounded, curvy letters, most crowned with a small tick or check-mark headstroke; no continuous top line."),
    ("kannada", "Kannada", 0x0C80, "_fonts/NotoSansKannada-Static.ttf",
     "Rounded and looping like Telugu (its sister script), most letters topped by a small curved headstroke; no continuous top line."),
    ("malayalam", "Malayalam", 0x0D00, "_fonts/NotoSansMalayalam-Static.ttf",
     "Highly rounded and loopy — full circles, hooks and curls, with almost no straight lines."),
]

HERE = os.path.dirname(os.path.abspath(__file__))


def codepoint_by_name(target_name: str, block_base: int) -> int | None:
    """Find the code point in a 128-slot block whose Unicode name matches."""
    for cp in range(block_base, block_base + 0x80):
        ch = chr(cp)
        try:
            if unicodedata.name(ch) == target_name:
                return cp
        except ValueError:
            continue  # unassigned code point
    return None


def build_script(script_id: str, name: str, base: int, font: str, signature: str) -> dict:
    up = name.upper()
    letters = []
    for con_tok, con_rom in CONSONANTS:
        con_cp = codepoint_by_name(f"{up} LETTER {con_tok}", base)
        if con_cp is None:
            continue  # this script doesn't have this consonant — skip, never invent
        con_ch = chr(con_cp)
        root = con_rom[:-1]  # drop the inherent 'a': "kha" -> "kh"
        for vow_tok, vow_rom in VOWELS:
            if vow_tok == "":
                glyph = con_ch
                sound = con_rom
                components = [f"{con_ch}  {con_rom} — base consonant (inherent “a”)"]
            else:
                sign_cp = codepoint_by_name(f"{up} VOWEL SIGN {vow_tok}", base)
                if sign_cp is None:
                    continue  # this script lacks this vowel sign — skip
                sign_ch = chr(sign_cp)
                glyph = con_ch + sign_ch
                sound = root + vow_rom
                components = [
                    f"{con_ch}  {con_rom} — base consonant",
                    f"{sign_ch}  “{vow_rom}” vowel sign",
                ]
            letters.append({
                "glyph": glyph,
                "sound": sound,
                "role": "syllable",
                "inherentVowel": "a",
                "components": components,
                "strokeOrder": [],       # recognition only — ductus is a separate, paused effort
                "strokeOrderNote": "",
            })
    return {
        "script": script_id,
        "name": name,
        "font": font,
        "direction": "ltr",
        "system": "abugida",
        "signature": signature,
        "complete": False,  # the core varga × core vowels — not the full script yet
        "notes": (
            "Syllabary generated from Unicode by generate_syllabary.py: each syllable is a base "
            "consonant (varga order) composed with a core vowel sign. Romanization is ISO-15919. "
            "Recognition only — stroke order is a separate, source-gated effort and is omitted."
        ),
        "letters": letters,
    }


def main() -> None:
    for script_id, name, base, font, signature in SCRIPTS:
        data = build_script(script_id, name, base, font, signature)
        path = os.path.join(HERE, f"{script_id}.json")
        with open(path, "w", encoding="utf-8") as fh:
            json.dump(data, fh, ensure_ascii=False, indent=2)
            fh.write("\n")
        print(f"wrote {path}: {len(data['letters'])} syllables "
              f"({sum(1 for l in data['letters'] if len(l['components']) == 1)} base consonants)")


if __name__ == "__main__":
    main()
