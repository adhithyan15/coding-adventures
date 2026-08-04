#!/usr/bin/env bash
# Regenerate NotoSansSC-Subset.ttf to cover exactly the Chinese characters used in
# data/scripts/chinese.json (glyphs + component/note text). Run this whenever the
# Mandarin content adds characters — the full NotoSansSC is ~17 MB, so we vendor
# only a subset.
#
# Requires: fonttools (`pip install fonttools`), curl, sha256sum (or shasum).
set -euo pipefail
cd "$(dirname "$0")"

# SHA-256 of the upstream NotoSansSC[wght].ttf this subset was built from. The
# download tracks a moving branch, so we verify the bytes before trusting them:
# if upstream changes, the build fails loudly rather than vendoring new bytes.
readonly EXPECTED_SHA="a3041811a78c361b1de50f953c805e0244951c21c5bd412f7232ef0d899af0da"

# Private, unpredictable work dir — no world-writable /tmp paths (avoids symlink
# pre-planting and cache-poisoning on shared/CI hosts).
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Repo-local, developer-owned cache for the big source font (not /tmp).
CACHE=".fontcache"
SRC="${1:-$CACHE/NotoSansSC-var.ttf}"

sha256() { command -v sha256sum >/dev/null && sha256sum "$1" | cut -d' ' -f1 || shasum -a 256 "$1" | cut -d' ' -f1; }

if [[ ! -f "$SRC" ]]; then
  mkdir -p "$(dirname "$SRC")"
  echo "Fetching NotoSansSC variable font -> $SRC"
  curl -fsSL --proto '=https' --max-time 180 -o "$SRC" \
    "https://github.com/google/fonts/raw/main/ofl/notosanssc/NotoSansSC%5Bwght%5D.ttf"
fi

got="$(sha256 "$SRC")"
if [[ "$got" != "$EXPECTED_SHA" ]]; then
  echo "ERROR: $SRC sha256 $got != expected $EXPECTED_SHA" >&2
  echo "Upstream font changed. Review the new file, then update EXPECTED_SHA." >&2
  exit 1
fi

# Collect every CJK codepoint that appears anywhere in chinese.json.
python3 - "$WORK/zh_chars.txt" <<'PY'
import pathlib, sys
t = pathlib.Path("../data/scripts/chinese.json").read_text()
cps = sorted({c for c in t if 0x2E80 <= ord(c) <= 0x9FFF})
pathlib.Path(sys.argv[1]).write_text("".join(cps))
print(f"{len(cps)} characters")
PY

python3 -m fontTools.subset "$SRC" --text-file="$WORK/zh_chars.txt" \
  --output-file="$WORK/NotoSansSC-sub-var.ttf" --no-hinting
python3 -m fontTools.varLib.instancer "$WORK/NotoSansSC-sub-var.ttf" wght=400 \
  -o NotoSansSC-Subset.ttf
echo "Wrote NotoSansSC-Subset.ttf ($(du -h NotoSansSC-Subset.ttf | cut -f1))"
