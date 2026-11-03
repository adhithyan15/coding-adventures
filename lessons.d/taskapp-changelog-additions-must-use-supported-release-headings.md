---
category: CI & GitHub Actions
---

# TaskApp changelog additions must use supported release headings

The release identity job rejected a new second-level `Unreleased - XAML diagnostics`
heading in PR #18848. Its parser accepts the established release heading format.
Add entries under the existing `[Unreleased]` section and run
`python code/scripts/taskapp_release.py validate-changelog --changelog code/programs/mosaic/task-app/CHANGELOG.md`
before pushing changes to this changelog. YAML and collector checks do not cover
the release metadata contract.
