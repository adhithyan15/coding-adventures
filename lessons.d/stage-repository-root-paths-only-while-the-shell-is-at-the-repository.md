---
category: Repo policy / workflow reminders
---

# Stage repository-root paths only while the shell is at the repository root

I combined a repository-root `git add` path with a test command whose working
directory was a nested package, so Git could not resolve the path and the test
never ran. I then guessed an unsupported lesson category before checking the
tool's printed category list. Run staging commands from the repository root and
package-local test commands separately; when recording a lesson, reuse an exact
category from `lessons.py` instead of inventing a near-synonym.
