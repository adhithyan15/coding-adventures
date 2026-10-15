---
category: Testing & coverage
---

# Scope repeated assertion anchors to their owning function before editing

An adapter generator assumed a whole-scene assertion message was globally
unique. Two Venn contract functions deliberately used the same message, so its
guard rejected the edit after an earlier geometry test fixture had already
been staged in the working copy. No paint oracle was changed by that failure.

Preserve the failed helper/receipt. Resolve each anchor inside its named owning
function and preflight every planned replacement before writing any file. On
retry, compare partially written files with both the original Git blob and the
exact intended result; never reset unrelated work. Keep old oracle bodies and
assertions reversible and require actual tests after the repaired edit.
