---
category: Repo policy / workflow reminders
---

# Quote shell search patterns that contain Markdown backticks

An `rg` pattern containing a Markdown code span was placed inside double quotes,
so zsh treated the backticked recording identifier as command substitution. The
search still found its line, but it also attempted to execute the identifier.
Use single quotes around literal search patterns that contain backticks (or
search for a nearby backtick-free fragment), especially when inspecting mock
paper recording markers.
