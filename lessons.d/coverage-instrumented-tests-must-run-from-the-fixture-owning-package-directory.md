---
category: Testing & coverage
---

# Coverage-instrumented tests must run from the fixture-owning package directory

NN47's first gcov probe compiled and ran from `_build`. The parser tests passed,
but the shared checkpoint fixture read failed because its package-relative path
now resolved from the output directory. A coverage percentage was produced even
though that complete test run failed: it was not valid release evidence.

Compile into the ignored output directory, run the instrumented executable from
the package directory, then collect counters from their output directory. Assert
the test exit code before accepting coverage. Keep the fixture read in the suite
so a wrong cwd cannot silently remove shared-input evidence.
