---
category: Repo policy / workflow reminders
---

# human-language mock data lives under code/learning rather than the TypeScript package data directory, and lessons categories must use the catalog's exact name

I tried to open mock source data below the TypeScript package because its tests
live there, and then tried to create this lesson with a shorthand category that
the lessons tool does not accept. The sources actually live below
`code/learning/human-languages`, while the package owns only the executable
tests, and the tool requires an exact category from its printed catalog. Locate
an existing source with `rg --files` before assuming its root, and pass an exact
catalog category such as `Repo policy / workflow reminders` to `lessons.py`.
