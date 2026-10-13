---
category: Repo policy / workflow reminders
---

# Do not use a closing keyword for a parent issue when a dependency slice lands

The C1 task-shape pull request used `Closes #13415`, but that issue owns the
entire Sanskrit B1-through-C2 program rather than this one dependency slice.
Auto-merge therefore closed a still-active parent issue. Reopen the parent,
record the landed slice in a comment, and use a non-closing issue reference in
future slice pull requests until every acceptance criterion is complete.
