# Unreleased — Managed microbenchmark fixture consumers

Register the eight implemented C#, F#, Java, and Kotlin statistics and paired
benchmark readers of the canonical fixture. Twenty-four package consumers now
participate in exact-input selection, language filtering, Kiln graph export,
and legacy fallback cache invalidation. Extend focused integration tests to
cover the new readers and preserve the missing-package and unrelated-language
selection checks.
