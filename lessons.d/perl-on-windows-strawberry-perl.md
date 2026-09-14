---
category: Cross-platform & Windows BUILD_windows
---

# Perl on Windows (Strawberry Perl)

: `cpanm --with-test` is a `cpm` flag, not cpanm — use `cpanm --installdeps --quiet .`. CI skips Perl on Windows entirely; provide a no-op `BUILD_windows` (`echo Perl testing not supported on Windows`) so the build tool doesn't fall back to BUILD.
