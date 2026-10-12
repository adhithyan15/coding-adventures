---
category: TypeScript / JavaScript
---

# running pnpm exec in a package without its own pnpm lockfile can create an unintended package-local lockfile

I used `pnpm exec vitest` inside the human-language-data package because an
agent reported that Vitest was unavailable. Pnpm resolved the missing local
tooling, but it also created an unintended package-local `pnpm-lock.yaml` in a
repository that does not track one there. The tests passed, and I removed the
generated lockfile before staging. Prefer the repository's established test
runner or a pre-existing package binary; when an alternate package manager is
needed for a diagnostic, inspect `git status` immediately and discard only the
known generated metadata.
