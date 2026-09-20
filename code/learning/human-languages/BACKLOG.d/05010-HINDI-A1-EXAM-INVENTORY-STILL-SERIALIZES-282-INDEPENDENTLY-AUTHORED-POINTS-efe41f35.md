## HL-C412-dcd99bf3 — Hindi A1 exam inventory still serializes 282 independently authored points

**Status: CLOSED (2026-09-20). Tracks #15725.** The post-Malayalam contention
audit found 22 recent touches to the 2,881-line Hindi A1 aggregate. Those edits
changed independently authored point evidence but still rewrote one shared
array.

Hindi A1 now uses the proven HL37 boundary: `_meta.json` owns stable inventory
scope and exact point order, while 282 canonical self-binding files each own one
point. The loader reconstructs the same public object and rejects missing,
extra, reordered, unsafe, noncanonical, nested, linked, or resurrected data.
Ordinary chapter work now edits only the points whose evidence changed.
