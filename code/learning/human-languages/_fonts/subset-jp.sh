#!/usr/bin/env bash
# Regenerate NotoSansJP-Subset.ttf for the Japanese track. Run this whenever the
# track adds kanji — the full NotoSansJP is ~9.6 MB, so we vendor only a subset.
#
# The coverage rule is deliberately asymmetric, because Japanese's three writing
# systems are not the same kind of thing:
#
#   * KANA are a closed set. U+3000-U+30FF (CJK punctuation, hiragana, katakana,
#     the length bar, the dakuten and handakuten) is only a few hundred glyphs,
#     so the whole block is included once and never revisited. A future lesson
#     using a kana this chapter never touched still renders.
#   * KANJI are open-ended. Only the ideographs that actually appear in the
#     track are included, gathered from data/scripts/japanese.d/ and from
#     every japanese/ source file, so nothing an author typed can silently drop
#     to a missing-glyph warning.
#
# Requires: fonttools (`pip install fonttools`), curl, sha256sum (or shasum).
set -euo pipefail
cd "$(dirname "$0")"

# SHA-256 of the upstream NotoSansJP[wght].ttf this subset was built from. The
# download tracks a moving branch, so we verify the bytes before trusting them:
# if upstream changes, the build fails loudly rather than vendoring new bytes.
readonly EXPECTED_SHA="c2f3b4d463500a2ddcd3849cded1fceeb9fd6d1c32e6cbecd568453ba50fc68f"

# Private, unpredictable work dir — no world-writable /tmp paths (avoids symlink
# pre-planting and cache-poisoning on shared/CI hosts).
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Repo-local, developer-owned cache for the big source font (not /tmp).
CACHE=".fontcache"
SRC="${1:-$CACHE/NotoSansJP-var.ttf}"

sha256() { command -v sha256sum >/dev/null && sha256sum "$1" | cut -d' ' -f1 || shasum -a 256 "$1" | cut -d' ' -f1; }

if [[ ! -f "$SRC" ]]; then
  mkdir -p "$(dirname "$SRC")"
  echo "Fetching NotoSansJP variable font -> $SRC"
  # --proto pins the FIRST request to https; --proto-redir pins every redirect
  # too, so -L cannot be walked down to plain http by a redirect we do not
  # control. The SHA-256 check below would still catch tampering, but a
  # downgrade should fail at the transport, not after the bytes are on disk.
  curl -fsSL --proto '=https' --proto-redir '=https' --max-time 300 -o "$SRC" \
    "https://raw.githubusercontent.com/google/fonts/main/ofl/notosansjp/NotoSansJP%5Bwght%5D.ttf"
fi

got="$(sha256 "$SRC")"
if [[ "$got" != "$EXPECTED_SHA" ]]; then
  echo "ERROR: $SRC sha256 $got != expected $EXPECTED_SHA" >&2
  echo "Upstream font changed. Review the new file, then update EXPECTED_SHA." >&2
  exit 1
fi

# Whole kana block, plus every ideograph the track actually uses.
python3 - "$WORK/jp_chars.txt" <<'PY'
import json, pathlib, sys

sys.path.insert(0, str(pathlib.Path("../data/scripts").resolve()))
from sharded_ledger import load_script_inventory

sources = sorted(pathlib.Path("../japanese").rglob("*.md"))
sources += sorted(pathlib.Path("../japanese").rglob("*.tex"))
sources += sorted(pathlib.Path("../japanese").rglob("*.json"))

inventory = load_script_inventory(pathlib.Path("..").resolve(), "japanese")
text = json.dumps(inventory, ensure_ascii=False)
text += "".join(p.read_text(encoding="utf8") for p in sources if p.is_file())

# Closed sets: take the whole block so future kana never fall out of the subset.
kana = {chr(c) for c in range(0x3000, 0x3100)}
# Printable Basic Latin, including the ordinary space. XeLaTeX typesets a space
# inside a \ja{...} group with the Japanese font selected, and a font with no
# U+0020 emits "Missing character" for every one of them.
kana |= {chr(c) for c in range(0x0020, 0x007F)}
# Open set: only the ideographs (and CJK-adjacent marks) actually written down.
used = {c for c in text if 0x3100 <= ord(c) <= 0x9FFF or 0xF900 <= ord(c) <= 0xFAFF}

chars = sorted(kana | used)
pathlib.Path(sys.argv[1]).write_text("".join(chars), encoding="utf8")
print(f"{len(kana)} kana-block + {len(used)} ideographs = {len(chars)} characters")
PY

python3 -m fontTools.subset "$SRC" --text-file="$WORK/jp_chars.txt" \
  --output-file="$WORK/NotoSansJP-sub-var.ttf" --no-hinting
python3 -m fontTools.varLib.instancer "$WORK/NotoSansJP-sub-var.ttf" wght=400 \
  -o NotoSansJP-Subset.ttf
echo "Wrote NotoSansJP-Subset.ttf ($(du -h NotoSansJP-Subset.ttf | cut -f1))"
