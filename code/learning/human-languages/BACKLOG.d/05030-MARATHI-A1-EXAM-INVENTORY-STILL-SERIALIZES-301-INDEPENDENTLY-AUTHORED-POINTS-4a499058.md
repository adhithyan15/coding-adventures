## HL-C414-9f819d2d — Marathi A1 exam inventory still serializes 301 independently authored points

**Status: CLOSED (2026-09-20). Tracks #15742.** The post-Hindi contention
audit found 13 unique-commit touches to the 4,780-line Marathi A1 aggregate in
the latest 500 commits. Those edits changed independently authored point
evidence but still rewrote one shared array.

Marathi A1 now uses the proven HL37 boundary: `_meta.json` owns stable inventory
scope and exact point order, while 301 canonical self-binding files each own one
point. The loader reconstructs the exact 151,077-byte public inventory and
rejects missing, extra, reordered, unsafe, noncanonical, nested, linked, or
resurrected data. Ordinary chapter work now edits only the points whose evidence
changed.
