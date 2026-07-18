#!/usr/bin/env bash
# Regenerate NotoSansSC-Subset.ttf to cover exactly the Chinese characters used in
# data/scripts/chinese.json (glyphs + component/note text). Run this whenever the
# Mandarin content adds characters — the full NotoSansSC is ~17 MB, so we vendor
# only a subset.
#
# Requires: fonttools (`pip install fonttools`), curl.
set -euo pipefail
cd "$(dirname "$0")"

SRC="${1:-/tmp/NotoSansSC-var.ttf}"
if [[ ! -f "$SRC" ]]; then
  echo "Fetching NotoSansSC variable font -> $SRC"
  curl -sL --max-time 180 -o "$SRC" \
    "https://github.com/google/fonts/raw/main/ofl/notosanssc/NotoSansSC%5Bwght%5D.ttf"
fi

# Collect every CJK codepoint that appears anywhere in chinese.json.
python3 - <<'PY'
import pathlib
t = pathlib.Path("../data/scripts/chinese.json").read_text()
cps = sorted({c for c in t if 0x2E80 <= ord(c) <= 0x9FFF})
pathlib.Path("/tmp/zh_chars.txt").write_text("".join(cps))
print(f"{len(cps)} characters")
PY

python3 -m fontTools.subset "$SRC" --text-file=/tmp/zh_chars.txt \
  --output-file=/tmp/NotoSansSC-sub-var.ttf --no-hinting
python3 -m fontTools.varLib.instancer /tmp/NotoSansSC-sub-var.ttf wght=400 \
  -o NotoSansSC-Subset.ttf
echo "Wrote NotoSansSC-Subset.ttf ($(du -h NotoSansSC-Subset.ttf | cut -f1))"
