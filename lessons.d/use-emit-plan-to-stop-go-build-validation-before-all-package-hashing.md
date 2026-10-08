---
category: BUILD files & dependency management
---

# Use emit-plan to stop Go BUILD validation before all-package hashing

The Go build tool's `-validate-build-files` flag is already enabled by
default. Running only that flag validated 5,304 discovered packages and the
orphan gates, then continued into hashing every package and potentially
executing the affected builds; it did not exit after validation. For a
validation/plan receipt, pass `-emit-plan <temporary path>` as well. The
planner exits immediately after validation and plan emission, before the
all-package hash/cache/build path. Use a separate intentional build command
when actual downstream execution is required.
