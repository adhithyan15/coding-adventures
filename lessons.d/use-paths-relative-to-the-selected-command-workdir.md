---
category: Repo policy / workflow reminders
---

# Use paths relative to the selected command workdir

A combined Java validation command ran from the package directory but gave
`rg` a repository-root-relative path. The search failed even though the Gradle
test that followed passed. When a command sets `workdir`, express every path
relative to that directory or run repository-wide inspection from the root;
do not mix the two coordinate systems in one command.
