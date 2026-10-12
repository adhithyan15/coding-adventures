---
category: Repo policy / workflow reminders
---

# Split staging and package tests into separate tool calls so each command has the correct working directory

The same root-path staging failure recurred even after its earlier lesson was
present, because a convenience command again combined `git add code/...` with
a package-local test under the package working directory. Do not rely on
remembering which half controls the `cwd`: issue one repository-root tool call
for staging and diff checks, then a separate package-directory tool call for
npm tests. The call boundary makes the directory contract explicit.
