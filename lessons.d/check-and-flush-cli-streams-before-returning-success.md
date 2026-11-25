---
category: Testing & coverage
---

# Check and flush CLI streams before returning success

Writing JSON or help text to `std::cout` does not guarantee the output was
delivered; a closed pipe or full disk can set the stream failure state while
the process still returns success. Route user-facing output through one helper
that writes, flushes, and checks the stream, and unit-test it with a failing
stream state so the CLI can surface the failure through its normal error path.
