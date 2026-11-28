---
category: BUILD files & dependency management
---

# A test helper source that includes a sibling package's private header needs that directory on the consumer include path

When a consumer test links a helper implementation from a sibling package,
include the helper's private source directory as well as its public include
directory. `test-bootstrap/tests/test_util.c` includes `pbt_internal.h` from
`test-bootstrap/src`; the consumer compiled the helper source but initially
listed only `test-bootstrap/include`, so strict compilation failed.

Add both required include paths to the Unix and Windows BUILD fronts. A source
file can bring its own header dependencies even when its declarations are
test-only and the runtime library does not depend on that package.
