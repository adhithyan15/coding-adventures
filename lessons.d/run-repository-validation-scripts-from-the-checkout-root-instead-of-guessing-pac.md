---
category: Repo policy / workflow reminders
---

# Run repository validation scripts from the checkout root instead of guessing package-relative depth

The OpenRISC package BUILD correctly had to run from the package directory,
but appending a guessed four-level relative path to `code/scripts/lessons.py`
made the independent lessons check point at `code/packages/scripts` and fail
before validation. Package-owned commands and repository-owned commands have
different working-directory contracts. Run the package BUILD from the package
directory, then change to the checkout root and invoke repository validators by
their stable root-relative paths. Do not combine the two contexts in one shell
line or infer the repository depth by counting parent components.
