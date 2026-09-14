---
category: Testing & coverage
---

# A proxy field standing in for a fact makes a reconciliation come out negative

Splitting `lessons.md` into shards, the migration reconciled character counts to
prove nothing was dropped. Shards came in two kinds: a whole `##` section, and a
single bullet lifted out of a topical bucket. Only the first kind carries a `##`
heading in the source, so only the first should have one added back when
counting.

The code asked `category is None` to tell the two apart. That is a PROXY, and it
was wrong for seven files: the lessons sitting above the first heading have no
category *and* no heading. Each was credited a `## <title>` the source never
contained, so the shards measured LARGER than the file they came from, and the
reconciliation reported `delta -193`.

The negative number is the useful part. A shortfall reads as "something was
dropped" and gets investigated; a *surplus* is impossible by construction —
text cannot be created by splitting — so it can only mean the measurement is
wrong, not the data. Had the proxy been wrong in the other direction it would
have shown up as a plausible-looking small positive delta, and might have been
explained away as rounding.

The fix is to carry the fact instead of inferring it: each shard records
`kind` as `"section"` or `"bullet"` at the point where that is known for
certain, and the reconciler reads that.

**How to apply.** When two things need telling apart, record which one it is at
the moment you know; do not re-derive it later from a field that merely
correlates. And when a conservation check can go negative, let it — a
direction that should be impossible is a free assertion about your own
measurement.

Related: the same arc's other lesson, that a resolver's assertions must be able
to see what it discarded.
