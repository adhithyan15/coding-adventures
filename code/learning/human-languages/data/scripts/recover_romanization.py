#!/usr/bin/env python3
# ---------------------------------------------------------------------------
# recover_romanization.py — how do you SAY this word?
#
# WHY THIS EXISTS
# ---------------
# A lesson's `romanization` is the field that lets a reader use a word before
# they can read it. HL11 calls a headword shown beside its romanization
# EXPOSURE -- something the reader is shown, not asked to decode -- and that is
# the whole mechanism by which a book about an unfamiliar script can be useful
# from page one. Without it, the headword is script the reader is stuck on.
#
# 489 headwords in this corpus had none.
#
# WHY THIS RECOVERS RATHER THAN DERIVES
# -------------------------------------
# The obvious move is to transliterate mechanically: every one of these scripts
# maps to ISO-15919 by rule, and `generate_syllabary.py` already carries the
# tables. That move is wrong, and the corpus proved it.
#
# Run against the 195 romanizations these tracks' authors had already written by
# hand, a mechanical derivation agreed with **71%** of them. Every disagreement
# was the machine being faithful to the SPELLING and wrong about the SOUND:
#
#   Tamil       61% agreement. Tamil writes ONE letter for each of k/g/h, one
#               for c/s, one for t/d, one for p/b, and which you say depends on
#               where it sits. Transliteration says cāppiṭu, paṭi, cukam, pēcu;
#               the words are said sāppiḍu, paḍi, sugam, pēsu.
#   Hindi       62%. Schwa deletion -- कितने is *kitne*, never *kitane*.
#   Malayalam   51%. The half-u at a word's end: ഉണ്ട് is *uṇṭŭ*, not *uṇṭ*.
#   Telugu/Kannada/Sanskrit  86-89%, mostly the anusvara, which is written as
#               one mark and said as whichever nasal matches the next consonant.
#
# A romanization exists so the reader can say the word. Publishing 344
# transliterations would have published 344 confident mispronunciations into a
# field the book, the app and the narration export all read aloud.
#
# So nothing here is derived. Each romanization is RECOVERED from what the
# lesson already tells its own reader in prose -- "Say it *va-ṇak-kam*" -- which
# is a human's pronunciation judgement, already written, already reviewed,
# already shipped in the book. This tool moves it into the field that consumes it.
#
# HOW A WRONG GRAB IS CAUGHT
# --------------------------
# An extractor that grabs the wrong italic would write a confident, wrong
# pronunciation. So every candidate is checked against the headword's mechanical
# transliteration through a SKELETON: the word with every distinction the script
# does not record folded away. Two spellings of the same word must agree on the
# skeleton while differing freely on everything the script leaves to context.
#
# That turns "did it grab the right word", which needs a reader of the script,
# into "do these two agree about what the script says", which a machine checks
# exactly. Where no candidate matches, the tool RECOVERS NOTHING and says so --
# 160 headwords still need a human, and that is the correct output, not a gap.
#
# The fold is per script, because the scripts differ in what they leave to
# context, and each fold is the smallest one under which that track's own
# authored romanizations agree with its script. A looser fold would accept more
# matches by accepting the wrong word.
#
# USAGE
#   python3 recover_romanization.py            # report what is recoverable
#   python3 recover_romanization.py --write    # write it into the lessons
# ---------------------------------------------------------------------------

import os, re, sys, unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
HL = os.path.normpath(os.path.join(HERE, "..", ".."))

TRACKS = {"tamil":"TAMIL","telugu":"TELUGU","kannada":"KANNADA",
          "malayalam":"MALAYALAM","hindi":"DEVANAGARI","sanskrit":"DEVANAGARI"}

# ISO-15919, keyed on the Unicode name token. Same table generate_syllabary.py
# uses, extended with what a real word needs that a syllabary grid does not.
CONS = {
    "KA":"k","KHA":"kh","GA":"g","GHA":"gh","NGA":"ṅ",
    "CA":"c","CHA":"ch","JA":"j","JHA":"jh","NYA":"ñ",
    "TTA":"ṭ","TTHA":"ṭh","DDA":"ḍ","DDHA":"ḍh","NNA":"ṇ",
    "TA":"t","THA":"th","DA":"d","DHA":"dh","NA":"n",
    "PA":"p","PHA":"ph","BA":"b","BHA":"bh","MA":"m",
    "YA":"y","RA":"r","LA":"l","VA":"v",
    "SHA":"ś","SSA":"ṣ","SA":"s","HA":"h",
    "LLA":"ḷ","LLLA":"ḻ","RRA":"ṟ","NNNA":"ṉ","RRRA":"ṟ",
    "QA":"q","KHHA":"ḵẖ","GHHA":"ġ","ZA":"z","DDDHA":"ṛ","RHA":"ṛh","FA":"f","YYA":"ẏ",
    "NNNNA":"ṉ","TTTA":"ṭ",
}
VOWELS = {
    "A":"a","AA":"ā","I":"i","II":"ī","U":"u","UU":"ū",
    "E":"e","EE":"ē","AI":"ai","O":"o","OO":"ō","AU":"au",
    "VOCALIC R":"ṛ","VOCALIC RR":"ṝ","VOCALIC L":"ḷ","VOCALIC LL":"ḹ",
    "CANDRA E":"ê","CANDRA O":"ô","SHORT E":"e","SHORT O":"o",
}
SIGNS = {"ANUSVARA":"ṁ","VISARGA":"ḥ","CANDRABINDU":"m̐","AVAGRAHA":"’",
         "NUKTA":"", "AI LENGTH MARK":"", "AU LENGTH MARK":"",
         "SIGN CANDRA BINDU":"m̐"}


def token(ch):
    """Classify one code point: (kind, value)."""
    try:
        name = unicodedata.name(ch)
    except ValueError:
        return ("other", ch)
    for prefix in ("TAMIL ","TELUGU ","KANNADA ","MALAYALAM ","DEVANAGARI "):
        if name.startswith(prefix):
            rest = name[len(prefix):]
            break
    else:
        return ("other", ch)

    if rest in ("SIGN VIRAMA", "SIGN PULLI", "SIGN CHANDRA E"):
        return ("virama", "")
    if rest.startswith("VOWEL SIGN "):
        v = rest[len("VOWEL SIGN "):]
        return ("vsign", VOWELS.get(v))
    if rest.startswith("LETTER CHILLU "):
        c = rest[len("LETTER CHILLU "):]
        return ("chillu", CONS.get(c))
    if rest.startswith("LETTER "):
        l = rest[len("LETTER "):]
        if l in VOWELS:
            return ("ivowel", VOWELS[l])
        if l in CONS:
            return ("cons", CONS[l])
        return ("unknown", l)
    if rest.startswith("SIGN "):
        s = rest[len("SIGN "):]
        if s in SIGNS:
            return ("sign", SIGNS[s])
        return ("unknown", rest)
    if rest.startswith("DIGIT "):
        return ("other", ch)
    return ("unknown", rest)


def romanize(word):
    """ISO-15919 for one word. Returns (text, unknown-tokens)."""
    out, unknown = [], []
    pending = None  # a consonant awaiting its vowel
    for ch in word:
        kind, val = token(ch)
        if kind == "cons":
            if pending is not None:
                out.append(pending + "a")
            pending = val
        elif kind == "chillu":
            if pending is not None:
                out.append(pending + "a")
                pending = None
            out.append(val or "?")
        elif kind == "vsign":
            if val is None:
                unknown.append(ch); val = "?"
            out.append((pending or "") + val)
            pending = None
        elif kind == "virama":
            if pending is not None:
                out.append(pending)
                pending = None
        elif kind == "ivowel":
            if pending is not None:
                out.append(pending + "a"); pending = None
            out.append(val)
        elif kind == "sign":
            if pending is not None:
                out.append(pending + "a"); pending = None
            out.append(val)
        elif kind == "unknown":
            unknown.append(val)
            if pending is not None:
                out.append(pending + "a"); pending = None
            out.append("?")
        else:
            if pending is not None:
                out.append(pending + "a"); pending = None
            out.append(ch)
    if pending is not None:
        out.append(pending + "a")
    return assimilate("".join(out)), unknown


# The anusvara is written as one mark but SPOKEN as whichever nasal shares the
# place of articulation with the consonant after it. Telugu ఉండు is uṇḍu, not
# uṁḍu; కుటుంబం is kuṭumbaṁ, not kuṭuṁbaṁ. A romanization exists so the reader
# can say the word, so it has to follow the mouth rather than the mark.
HOMORGANIC = [
    ("ṅ", "kgh"),          # velar series
    ("ñ", "cj"),           # palatal
    ("ṇ", "ṭḍ"),           # retroflex
    ("n", "td"),           # dental
    ("m", "pbmv"),         # labial
]


def assimilate(text):
    out = list(text)
    for i, ch in enumerate(out):
        if ch != "ṁ":
            continue
        nxt = ""
        for j in range(i + 1, len(out)):
            if out[j] not in " -":
                nxt = out[j]
                break
        if not nxt:
            continue  # word-final anusvara stays as written
        for nasal, series in HOMORGANIC:
            if nxt in series:
                out[i] = nasal
                break
    return "".join(out)



def fm(text):
    if not text.startswith("---"): return {}
    end = text.find("\n---", 3)
    out = {}
    for line in text[3:end].splitlines():
        m = re.match(r"^([A-Za-z_][A-Za-z0-9_]*):\s*(.*)$", line)
        if m: out[m.group(1)] = m.group(2).strip().strip('"\'')
    return out



LIGHT = {"ā":"a","ī":"i","ū":"u","ē":"e","ō":"o",
         "ṁ":"n","ṅ":"n","ñ":"n","ṇ":"n","m":"n",
         "ś":"s","ṣ":"s","ṛ":"r","ŭ":"u","ḥ":"h","ḷ":"l","ḻ":"l","ṟ":"r","ṉ":"n"}
HEAVY = dict(LIGHT); HEAVY.update({"g":"k","h":"k","ḵ":"k","s":"c","d":"t","b":"p","ḍ":"ṭ"})
FOLD = {"tamil": str.maketrans(HEAVY)}
for t in ("telugu","kannada","malayalam","hindi","sanskrit"):
    FOLD[t] = str.maketrans(LIGHT)

def skel(s, track):
    s = unicodedata.normalize("NFC", s).lower().translate(FOLD[track])
    return re.sub(r"[^a-zṭ]", "", s)

def cands(body):
    out=[]
    for m in re.finditer(r"\*([^*\n]{2,44})\*", body):
        tok=m.group(1).strip()
        if re.fullmatch(r"[A-Za-zāīūēōṇṭḍṅñḷḻṟṉśṣḥṁṛŭ'’\- ·]+", tok): out.append(tok)
    return out

def is_target(word, script):
    return any(unicodedata.name(c).split(" ")[0]==script for c in word
               if c.strip() and unicodedata.category(c)[0]!="Z")

def recover(track, script, hw, body):
    derived,_ = romanize(hw)
    want = skel(derived, track)
    pool = [c.replace("-","").replace("·","") for c in cands(body)]
    for c in pool:
        if skel(c, track) == want: return c
    if " " in hw.strip():
        parts=[]
        for word in hw.split():
            if not is_target(word, script): parts.append(word); continue
            wd,_ = romanize(word)
            f = next((c for c in pool if skel(c,track)==skel(wd,track)), None)
            if f is None: return None
            parts.append(f)
        composed=" ".join(parts)
        if skel(composed, track)==want: return composed
    return None

if __name__ == "__main__":
    apply = "--write" in sys.argv
    total_w = total_m = 0
    print(f"{'track':<11}{'missing':>9}{'recovered':>11}{'%':>5}")
    for track, script in TRACKS.items():
        d=os.path.join(HL,track,"lessons"); w=m=0
        for f in sorted(os.listdir(d)):
            if not f.endswith(".md"): continue
            path=os.path.join(d,f); raw=open(path,encoding="utf-8").read()
            end=raw.find("\n---",3); h=fm(raw); body=raw[end+4:]
            hw=h.get("headword","")
            if h.get("romanization","").strip() or not is_target(hw, script): continue
            m+=1
            hit=recover(track, script, hw, body)
            if not hit: continue
            w+=1
            if apply:
                hit=hit[0].lower()+hit[1:]
                head,rest=raw[:end],raw[end:]
                lines=head.splitlines()
                a=next((i for i,l in enumerate(lines) if l.startswith("gloss:")), None)
                if a is None: a=next((i for i,l in enumerate(lines) if l.startswith("headword:")), None)
                if a is None: continue
                lines.insert(a+1, f'romanization: "{hit}"')
                open(path,"w",encoding="utf-8").write("\n".join(lines)+rest)
        total_w+=w; total_m+=m
        print(f"{track:<11}{m:>9}{w:>11}{(100*w//max(m,1)):>5}")
    print(f"\n{'TOTAL':<11}{total_m:>9}{total_w:>11}{(100*total_w//max(total_m,1)):>5}")
