---
category: CI & GitHub Actions
---

# Regenerate source-pinned audits from exact Git blob bytes rather than checkout line endings

The cached upstream source had CRLF endings: 89,988 bytes instead of the pinned 87,726, so the audit generator correctly rejected its size and hash. The cache folder also lacked the expected Git object, so do not assume an extracted source directory is a usable checkout. Prefer exact `git show` blob bytes when the object is present. Otherwise reconstruct LF bytes only in an owned scratch file and require both the exact pinned size and SHA-256 before using that file. Never weaken the pin or manually adjust the report to bypass verification. The verified reconstruction matched SHA-256 153d1bf4d285f3a40ef032acb36b77b9798b87e53da26de212014f68641203c7.
