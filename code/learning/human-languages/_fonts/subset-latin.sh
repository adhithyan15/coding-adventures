#!/usr/bin/env bash
# Regenerate LatinPrint-Subset.ttf, the outline the Latin-script stroke-order
# filmstrips are drawn on. Run it when a Latin-track headword adds a character.
#
# WHY A SEPARATE FONT. The filmstrip draws one letter at a time and traces a
# school model's pen path on top of the printed letter, so the printed letter
# must have the SHAPE the school model teaches. Noto Sans prints a two-storey
# "a" (a small bowl under a hooked top); every cited handwriting source (the
# Grundschrift-App school model, UJIpenchars2's native Spanish writers,
# tracedletters) teaches the one-storey "a" (a round bowl, then a stem). Andika,
# SIL Global's literacy typeface, is designed for people learning to read and
# prints the one-storey a and the single-storey g by default:
#
#     character   Noto Sans    Andika 7.000 (default cmap glyph)
#     ---------   ---------    ---------------------------------
#     a           two-storey   a.SngStory   (one bowl, one stem)
#     g           one bowl     g.SngBowl    (one bowl, open tail)
#
# The books' body text keeps its own font; only the filmstrips read this file.
#
# WHAT THIS DOES, in order:
#   1. fetch the Andika 7.000 release zip (pinned tag) and check its SHA-256;
#   2. take Andika-Regular.ttf out of it and check that file's SHA-256 too;
#   3. keep only the characters the Latin-script tracks need (below);
#   4. replace every composite glyph (á is "a + acute" by reference) with plain
#      contours, because script-ductus's TrueType reader refuses the scaled
#      components a few composites use, and a flat glyph is what it draws;
#   5. scale the em from 2048 to 1000 units, the em of every other vendored
#      font, so the ductus tests' distances (100 units = a tenth of an em) mean
#      the same thing for Latin as for every other script;
#   6. RENAME it. Andika's licence reserves the names "Andika" and "SIL"
#      (OFL-1.1 Reserved Font Names), and a subset is a Modified Version, so the
#      output is called "Latin Print Subset" and says where it came from in its
#      description. The copyright and licence name records are kept as they are.
#
# Requires: fonttools (`pip install fonttools`), curl, sha256sum (or shasum).
set -euo pipefail
cd "$(dirname "$0")"

readonly RELEASE_URL="https://github.com/silnrsi/font-andika/releases/download/v7.000/Andika-7.000.zip"
# SHA-256 of the release zip and of the Andika-Regular.ttf inside it. A release
# asset can be replaced; if either digest changes, the build stops instead of
# vendoring bytes nobody has looked at.
readonly EXPECTED_ZIP_SHA="88ba6ea41ef4a8e5214b090df8fa2983be1babe4843efaa99cdb6078b0e2c070"
readonly EXPECTED_TTF_SHA="27484fdc98d0d63f90407f8266e28295f6fb16d2b13c5024df0214f17152919a"

# fontTools stamps head.modified with the wall clock on every save. Pin it to
# the upstream font's creation instant (2025-05-29T16:58:46Z) so the same source
# bytes and character set always give the same subset bytes.
export SOURCE_DATE_EPOCH="1748537926"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

CACHE=".fontcache"
SRC="${1:-$CACHE/Andika-Regular.ttf}"

sha256() { command -v sha256sum >/dev/null && sha256sum "$1" | cut -d' ' -f1 || shasum -a 256 "$1" | cut -d' ' -f1; }

if [[ ! -f "$SRC" ]]; then
  mkdir -p "$(dirname "$SRC")"
  echo "Fetching the Andika 7.000 release -> $WORK/Andika-7.000.zip"
  # --proto / --proto-redir keep every hop on https (GitHub redirects release
  # assets to another host).
  curl -fsSL --proto '=https' --proto-redir '=https' --max-time 300 \
    -o "$WORK/Andika-7.000.zip" "$RELEASE_URL"
  got="$(sha256 "$WORK/Andika-7.000.zip")"
  if [[ "$got" != "$EXPECTED_ZIP_SHA" ]]; then
    echo "ERROR: release zip sha256 $got != expected $EXPECTED_ZIP_SHA" >&2
    exit 1
  fi
  # Read one named member as bytes; nothing in the archive is executed.
  python3 -I - "$WORK/Andika-7.000.zip" "$SRC" <<'PY'
import sys, zipfile
with zipfile.ZipFile(sys.argv[1]) as archive:
    data = archive.read("Andika-7.000/Andika-Regular.ttf")
with open(sys.argv[2], "wb") as out:
    out.write(data)
PY
fi

got="$(sha256 "$SRC")"
if [[ "$got" != "$EXPECTED_TTF_SHA" ]]; then
  echo "ERROR: $SRC sha256 $got != expected $EXPECTED_TTF_SHA" >&2
  echo "Upstream font changed. Review the new file, then update EXPECTED_TTF_SHA." >&2
  exit 1
fi

# The character set:
#   * printable Basic Latin (U+0020-U+007E): a-z, A-Z, digits, punctuation;
#   * the marks the Latin tracks' writing lessons teach: ß ñ Ñ ¿ ¡ and the
#     acute, diaeresis and tilde vowels, small and capital;
#   * every non-ASCII Latin-script character (NFC) of every Latin-track
#     headword, and every glyph the Latin inventory lists, so a letter a lesson
#     names always has an outline even before it has a cited ductus.
python3 -I - "$WORK/latin_chars.txt" <<'PY'
import json, pathlib, re, sys, unicodedata

TRACKS = ["spanish", "french", "german", "italian", "portuguese", "latin"]
chars = {chr(c) for c in range(0x20, 0x7F)}
chars |= set("ßñÑ¿¡áéíóúÁÉÍÓÚüÜäöÄÖãõÃÕ")

inventory = json.loads(pathlib.Path("../data/scripts/latin.json").read_text(encoding="utf8"))
for letter in inventory["letters"]:
    chars |= set(unicodedata.normalize("NFC", letter["glyph"]))

headword = re.compile(r"^headword:[ \t]*(.*)$", re.M)
for track in TRACKS:
    for lesson in sorted(pathlib.Path("..", track, "lessons").rglob("*.md")):
        for match in headword.finditer(lesson.read_text(encoding="utf8")):
            for ch in unicodedata.normalize("NFC", match.group(1)):
                if ord(ch) > 0x7E and ("LATIN" in unicodedata.name(ch, "") or ch in "¿¡ªº"):
                    chars.add(ch)

pathlib.Path(sys.argv[1]).write_text("".join(sorted(chars)), encoding="utf8")
print(f"{len(chars)} characters")
PY

# The point of the font: Andika's DEFAULT a is the one-storey a and its default
# g the single-storey g, so nothing needs remapping. (The subsetter renames
# glyphs from the cmap, so this is checked on the source.)
python3 -I - "$SRC" <<'PY'
import sys
from fontTools.ttLib import TTFont
cmap = TTFont(sys.argv[1]).getBestCmap()
assert cmap[ord("a")] == "a.SngStory", cmap[ord("a")]
assert cmap[ord("g")] == "g.SngBowl", cmap[ord("g")]
PY

python3 -I -m fontTools.subset "$SRC" --text-file="$WORK/latin_chars.txt" \
  --output-file="$WORK/latin-subset.ttf" --no-hinting --layout-features='' \
  --name-IDs='*' --name-languages='*' --drop-tables+=Silt

python3 -I - "$WORK/latin-subset.ttf" LatinPrint-Subset.ttf <<'PY'
import sys
from fontTools.pens.recordingPen import DecomposingRecordingPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont
from fontTools.ttLib.scaleUpem import scale_upem

font = TTFont(sys.argv[1])

# Flatten every composite into plain contours.
glyph_set = font.getGlyphSet()
glyf = font["glyf"]
flat = {}
for name in font.getGlyphOrder():
    if glyf[name].isComposite():
        recording = DecomposingRecordingPen(glyph_set)
        glyph_set[name].draw(recording)
        pen = TTGlyphPen(None)
        recording.replay(pen)
        flat[name] = pen.glyph()
for name, glyph in flat.items():
    glyf[name] = glyph
font["maxp"].maxComponentElements = 0
font["maxp"].maxComponentDepth = 0

scale_upem(font, 1000)

# OFL-1.1 Reserved Font Names: a Modified Version may not be called "Andika".
names = font["name"]
keep = {0, 13, 14}  # copyright, licence, licence URL: kept verbatim
names.names = [record for record in names.names if record.nameID in keep]
family = "Latin Print Subset"
for name_id, value in {
    1: family,
    2: "Regular",
    3: "coding-adventures: Latin Print Subset: 2025",
    4: family,
    5: "Version 7.000; subset of Andika 7.000",
    6: "LatinPrintSubset-Regular",
    10: ("A renamed subset of Andika 7.000 by SIL Global (SIL Open Font License 1.1), "
         "cut down to the Latin letters the coding-adventures Latin-script tracks draw. "
         "It is not Andika and is not endorsed by SIL Global."),
}.items():
    names.setName(value, name_id, 3, 1, 0x409)
font.save(sys.argv[2])
PY
echo "Wrote LatinPrint-Subset.ttf ($(du -h LatinPrint-Subset.ttf | cut -f1))"
