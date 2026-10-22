---
category: Repo policy / workflow reminders
---

# Reset the working directory before appending package commands to root gates

A root-scoped validation block successfully ran repository Python gates, then
appended `cargo tree` without changing into `code/packages/rust`; Cargo failed
because the repository root has no manifest. When one report mixes root and
package checks, give every package command an explicit package workdir instead
of inheriting the preceding gate's directory.
