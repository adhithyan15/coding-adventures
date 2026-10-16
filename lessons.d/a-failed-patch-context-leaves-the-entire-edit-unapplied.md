---
category: Testing & coverage
---

# A failed patch context leaves the entire edit unapplied

An external next-spec helper patch included one nonexistent context line.
apply_patch refused the complete patch before changing the file. Preserve the
diagnostic, inspect the actual helper and reapply only the intended small hunks.
Do not assume earlier hunks in a refused patch took effect, and do not alter
the frozen reviewed publication candidate to repair an external helper.

During the D3a diagnostic amendment, a backlog sentence was mistakenly used as
spec context, followed by an assumed wrapping of the intended spec sentence.
Both patches were refused atomically. Read the actual spec window and use its
exact physical line before reapplying either the spec or the accompanying lesson.
