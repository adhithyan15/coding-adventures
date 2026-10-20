---
category: CI & GitHub Actions
---

# A double-quoted gh --body argument executes Markdown backticks before GitHub receives the comment

While posting a merge checkpoint, a JavaScript string was safely serialized as
a double-quoted shell argument and passed to `gh issue comment --body`. The body
contained Markdown code spans. JSON string escaping protects quotes, but it does
not make a string opaque to the shell: the backticks were still command
substitutions. The shell tried to execute the commit SHA and GitHub received a
comment with the SHA removed.

Edit the damaged comment immediately, then pass future Markdown bodies through a
single-quoted shell argument with embedded single quotes escaped, or through
standard input. Treat convenience serializers as syntax escapers for their own
format only; they are not a substitute for choosing a shell-safe transport.
