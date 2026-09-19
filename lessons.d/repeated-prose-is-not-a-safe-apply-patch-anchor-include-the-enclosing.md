---
category: Repo policy / workflow reminders
---

# Repeated prose is not a safe apply-patch anchor; include the enclosing Rust structure

Documentation for a public method can repeat the audit enum variant's wording,
so a patch anchored only on that prose may insert executable code into the enum.
Before applying a structural Rust edit, include the unique `impl` method
signature or another enclosing-code anchor, then compile immediately.
