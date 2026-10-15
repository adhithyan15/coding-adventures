---
category: Repo policy / workflow reminders
---

# Derive hot paths from report output before inspecting Git history

A history inspection used an assumed `language-ladder/tests/corpus` prefix for
two Punjabi hot files even though the report identified them under
`human-language-data/tests/corpus`. Git treats a valid but absent pathspec as an
empty result, so the mistake looked like evidence that the files lacked
history. Copy exact paths from the machine-readable report (or verify them with
`git ls-tree`) before using them as pathspecs, especially when adjacent packages
have similarly named corpus trees.
