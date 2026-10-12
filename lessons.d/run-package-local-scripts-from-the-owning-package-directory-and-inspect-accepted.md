---
category: Repo policy / workflow reminders
---

# Run package-local scripts from the owning package directory and inspect accepted lesson categories before creating a workflow lesson

The human-language-data shard checks were invoked from the repository root,
which has no `package.json`, so npm failed before running any validation. The
first attempt to record that mistake also guessed an unsupported lesson
category. Run package-owned npm scripts from
`code/packages/typescript/human-language-data`, and use a category listed by
`lessons.py` (here, `Repo policy / workflow reminders`) rather than inventing
one.
