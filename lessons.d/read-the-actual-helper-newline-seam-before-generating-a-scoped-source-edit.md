---
category: Repo policy / workflow reminders
---

# Read the actual helper newline seam before generating a scoped source edit

A stage ABI helper generator guessed two newlines before a function while its
source used one. Generation failed before repository mutation; the attempted
missing helper invocation also failed. Preserve both failures, inspect the exact
seam, compile the generated helper before executing it, and retain whole-file
inverse proofs. Matching a known declaration is safer than guessing whitespace.
The corrected scanner also uses a compiled regex with an input offset, avoiding
repeated suffix copies; it changed no library production limit or behavior.
