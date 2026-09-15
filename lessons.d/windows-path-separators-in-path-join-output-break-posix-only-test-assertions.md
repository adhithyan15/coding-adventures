---
category: TypeScript / JavaScript
---

# Windows path separators in `path.join` output break POSIX-only test assertions

`f.endsWith("/foo.md")` against `path.join(root, "foo.md")` succeeds on Linux/macOS, fails on Windows where `\` is the separator. Fix: normalise the assertion with `.split(/[/\\]/).join("/")`. Pre-existing bugs in this pattern can hide if the package's BUILD never runs on Windows CI (the build-tool only runs BUILDs for *changed* packages, so dormant tests stay dormant).
