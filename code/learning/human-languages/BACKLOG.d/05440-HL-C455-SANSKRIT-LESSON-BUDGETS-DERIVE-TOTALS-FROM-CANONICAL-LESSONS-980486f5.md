## HL-C455-87955835 — Sanskrit lesson budgets derive totals from canonical lessons

**Status: CLOSED (2026-10-15). Tracks #17618.** The post-#17621 contention
audit found 16 touches to Sanskrit's cumulative lesson-budget pin in both the
latest 100 and 200 human-language commits. Independent chapters kept appending
history and changing a shared lesson count even though the canonical lesson
owners already determine that count exactly.

The Sanskrit budget gate now derives its lesson census from every canonical
lesson and rejects a schema-v1 downgrade rather than filtering it away. Stable
direct owners independently enumerate all idiom, sense, and culture-claim
identities, so deletion and addition remain exact without a shared total. The
gate also rejects every per-lesson excess and enforces the stable `SA` unit-id
prefix. Ordinary chapters no longer edit a cumulative test owner.
