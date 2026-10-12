---
category: Repo policy / workflow reminders
---

# re-read the applicable working-directory lesson before combining validation commands

I repeated a documented failure by combining a repository-root `git add` with
an `npm test` command whose declared working directory was the package. The
staging path therefore resolved below the package and failed before tests ran.
The existing `split-staging-and-package-tests-into-separate-tool-calls` lesson
already described the prevention. Before a familiar multi-directory sequence,
search and re-read the applicable lesson, then issue one root staging call and
one package test call rather than relying on memory.
