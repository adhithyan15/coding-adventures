---
category: Workspace & package metadata
---

# JVM composite Gradle BUILDs need a shared lock

when multiple packages reuse the same included builds — parallel runs corrupt shared `gradle-build` class outputs. Use `--no-daemon --no-build-cache --max-workers=1` plus a repo-local file lock.
