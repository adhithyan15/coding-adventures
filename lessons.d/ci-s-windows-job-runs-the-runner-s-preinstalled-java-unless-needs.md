---
category: Compiler / VM / language pipeline
---

# CI's Windows job runs the runner's preinstalled java unless needs_java, so emitted JVM code must not depend on the JDK version

`.github/workflows/ci.yml` installs JDK 21 with `actions/setup-java` only when
the detect job sets `needs_java` (or `needs_kotlin`, ...). Languages are
detected by package path, so a PR that changes only Rust packages does not set
it. The Rust tests that run generated classes on a real JVM then use whatever
`java` the runner has on PATH. On `windows-2025` that was a preinstalled JDK
older than 19, while local development used JDK 21.

LANG83 PR6a's JVM float writer took its digits from `Double.toString`. That is
the shortest round-trip decimal only since JDK 19 (JDK-4511638). On the Windows
runner it printed `1.9999999999999998e+23` where Python and JavaScript print
`2e+23`, and the real-JVM table tests failed there alone. This was a genuine
bug, not a flaky environment. The emitted classes are version 49 (Java 5), so
any JVM may run them, and their output must be the same on every one.

The fix was to drop the version-dependent API: the helper now searches
`BigDecimal` roundings of the exact value (Java 5 APIs only). Its output was
checked to be identical on Temurin JDK 8, 17 and OpenJDK 21.

What to do: when emitted JVM code calls a JDK method, check whether its
behaviour changed between releases. `Double.toString` and `Float.toString`
changed their digits in JDK 19, for example. Test real-JVM output on an old
JDK as well as the current one.
Temurin tarballs from `github.com/adoptium/temurin{8,17}-binaries` releases
work for this. Do not gate or skip a test on JDK version to make CI green: a
user's JVM is not CI's.
