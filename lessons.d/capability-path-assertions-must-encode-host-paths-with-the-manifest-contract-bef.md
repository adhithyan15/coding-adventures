---
category: Cross-platform & Windows BUILD_windows
---

# Capability-path assertions must encode host paths with the manifest contract before comparing them across platforms

The Forme install CLI test compared a resolved storage capability with a raw
host path. That happened to pass on POSIX, but failed on Windows because the
manifest contract deliberately URI-path-encodes colons and backslashes so a
Windows path remains one reversible capability segment. The implementation was
correct; the platform-specific expected value was not.

When testing path-derived capabilities, build the expected value through the
same public encoding rule that the specification defines: preserve `/` as the
hierarchy separator, percent-encode `%`, `:`, backslashes, whitespace,
controls, and non-ASCII UTF-8 bytes, and compare the complete capability after
platform normalization. Do not weaken the assertion to accept both raw and
encoded forms; that would hide aliases the authority model is designed to
prevent.
