---
category: Repo policy / workflow reminders
---

# Anchor apply_patch to verified current context

Even a semantically correct edit fails when its patch context was reconstructed
from memory or guessed ordering. That costs a tool round trip and encourages
riskier broad replacements.

Immediately before a structural patch, read the smallest exact source range
containing every anchor line. Apply the change in small hunks against that
verified text, especially when adding declarations near reordered constants or
refactoring a function body.
