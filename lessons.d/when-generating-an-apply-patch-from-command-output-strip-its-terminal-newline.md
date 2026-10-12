---
category: Repo policy / workflow reminders
---

# When generating an apply patch from command output strip its terminal newline before adding patch framing

A generated add-file patch prefixed every line of `cat` output, including the
empty element after its terminal newline, and then added another newline before
the patch trailer. The resulting JSON had a blank line at EOF and failed
`git diff --check`. Remove the terminal newline before splitting command output
into patch lines, then let the patch framing add exactly one final newline.
