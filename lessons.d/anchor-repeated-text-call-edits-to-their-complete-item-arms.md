---
category: Testing & coverage
---

# Anchor repeated text-call edits to their complete item arms

The Timeline period-role test-plan preparer expected one occurrence of the
label text call, but TimelineSection and TimelineEvent intentionally share
that complete call. Its uniqueness assertion refused the proposal before any
repository source write; the partial external plan and failed preparation are
preserved. This is a preparer refusal, not a compiled semantic regression red.

Use each complete item-arm header and call as the edit anchor. Both unchanged
label roles then receive explicit old-label/new-label arguments, while the
period alone declares old-title/new-label. Require one match per arm and replay
all substitutions backward against complete physical preimages. Do not remove
the uniqueness guard or use a global replacement to conceal ambiguous scope.
