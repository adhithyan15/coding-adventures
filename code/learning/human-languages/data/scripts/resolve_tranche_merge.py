#!/usr/bin/env python3
"""Resolve the four-file conflict a vocabulary tranche hits when main advances.

Backlog HL-C209.  Four tranches in a row hit the SAME conflict: the generated
`core/lesson-modality.json` plus the three test-pin files.  The resolution is
always the same and is NOT "pick a side" -- both branches add lessons, so the
correct value is the SUM, which neither side carries.

What this does, in order:

  1. Takes any side of the generated manifest, then REGENERATES it from the
     merged lesson tree.  It is a generated artifact; only the merged tree knows
     the answer.
  2. Clears conflict markers keeping OUR side, which preserves the pin ledger
     comments authors add.
  3. Reads every object pin from the regenerated summary -- enumerated from the
     manifest ITSELF, never from a hand-written list.  Writing that list from
     memory is how `drivablePercent` was missed on the third merge.

Two constraints are encoded rather than left to care:

  * The object-pin patcher only touches lines past the corpus block, because the
    synthetic fixture at modality-manifest.test.ts:359 shares field NAMES with
    the corpus pin near line 998.  HL-C196 records those two oscillating for 60
    rounds.
  * The chapter ledger is matched by the line mentioning `ledgers.flatMap`,
    never the first `toHaveLength` in the file -- a count-limited regex once
    clobbered `expect(modalityFiles).toHaveLength(22)`, which counts TRACKS.

R2 is deliberately NOT handled.  It cannot be derived from the manifest:
extending a track makes previously unjudgeable tail atoms judgeable, so its new
value has to be read from the failing assertion.  Run vitest afterwards.
"""
import json
import pathlib
import re
import sys

TESTS = pathlib.Path("code/packages/typescript/human-language-data/tests")
MANIFEST = pathlib.Path("code/learning/human-languages/core/lesson-modality.json")
CORPUS_BLOCK_STARTS_AFTER = 800  # keeps the patcher away from the synthetic fixtures

MARKERS = re.compile(r"(?ms)^<<<<<<< HEAD\n(.*?)^=======\n.*?^>>>>>>> [^\n]*\n")


def clear_markers(path: pathlib.Path) -> bool:
    text = path.read_text(encoding="utf-8")
    if "<<<<<<<" not in text:
        return False
    resolved = MARKERS.sub(lambda m: m.group(1), text)
    if "<<<<<<<" in resolved or ">>>>>>>" in resolved:
        sys.exit(f"unresolved markers remain in {path}")
    path.write_text(resolved, encoding="utf-8")
    return True


def main() -> int:
    if not MANIFEST.exists():
        sys.exit(f"{MANIFEST} not found -- run from the repo root")
    summary = json.loads(MANIFEST.read_text(encoding="utf-8")).get("summary", {})
    if not summary:
        sys.exit("manifest has no summary -- did you regenerate it first?")

    # every integer key the manifest publishes, enumerated from the manifest
    pins = {k: v for k, v in summary.items() if isinstance(v, int)}
    print(f"regenerated truth ({len(pins)} integer keys): "
          + ", ".join(f"{k}={v}" for k, v in sorted(pins.items())))

    for name in ("chapter-modality-book.test.ts", "continuity.test.ts",
                 "modality-manifest.test.ts"):
        path = TESTS / name
        if path.exists() and clear_markers(path):
            print(f"  cleared conflict markers in {name} (kept our side)")

    manifest_test = TESTS / "modality-manifest.test.ts"
    lines = manifest_test.read_text(encoding="utf-8").split("\n")
    touched = []
    for i, line in enumerate(lines):
        if i < CORPUS_BLOCK_STARTS_AFTER:
            continue  # synthetic fixtures live above this; see HL-C196
        m = re.match(r"(\s*)(\w+)(:\s*)(\d+)(.*)$", line)
        if m and m.group(2) in pins:
            lines[i] = f"{m.group(1)}{m.group(2)}{m.group(3)}{pins[m.group(2)]}{m.group(5)}"
            if lines[i] != line:
                touched.append(f"{m.group(2)}@{i + 1}")
    manifest_test.write_text("\n".join(lines), encoding="utf-8")
    print(f"  set {len(touched)} object pins: {', '.join(touched) or '(none changed)'}")

    ledger = TESTS / "chapter-modality-book.test.ts"
    lines = ledger.read_text(encoding="utf-8").split("\n")
    for i, line in enumerate(lines):
        if "ledgers.flatMap" in line and "toHaveLength(" in line:
            lines[i] = re.sub(r"(toHaveLength\()\d+(\))",
                              rf"\g<1>{pins['chapterCount']}\g<2>", line)
            print(f"  set chapter ledger toHaveLength({pins['chapterCount']}) @{i + 1}")
    ledger.write_text("\n".join(lines), encoding="utf-8")

    print("\nNow run vitest. R2 is NOT set here -- it cannot be derived from the")
    print("manifest, so read its new value from the failing assertion.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
