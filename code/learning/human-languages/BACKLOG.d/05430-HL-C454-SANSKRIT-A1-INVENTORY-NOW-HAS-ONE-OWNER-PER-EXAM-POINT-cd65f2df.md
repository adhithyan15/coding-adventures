## HL-C454-39f5206f — Sanskrit A1 inventory now has one owner per exam point

**Status: CLOSED (2026-10-15). Tracks #17617.** The post-#17609 contention
audit found 14 touches to the 3,497-line Sanskrit A1 aggregate in the latest 50
human-language commits. Those edits changed independently authored point
evidence but still rewrote one shared array.

Sanskrit A1 now uses the proven HL37 boundary: `_meta.json` owns stable
inventory scope and exact point order, while 164 canonical self-binding files
each own one point. The loader reconstructs the exact 132,030-byte public
inventory and rejects missing, extra, reordered, unsafe, noncanonical, nested,
linked, or resurrected data. Ordinary chapter work now edits only the points
whose evidence changed.
