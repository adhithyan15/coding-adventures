---
category: Cross-platform & Windows BUILD_windows
---

# ISO harness compile-time invariants should use static assertions under MSVC warnings-as-errors

NN47's real MSVC harness self-test rejected the conforming C fixture because
ISO_CHECK_MSG(sizeof(int) >= 2, ...) became a constant runtime condition under
/W4 /WX (C4127). Express the language invariant with C17 _Static_assert using
sizeof(int) * CHAR_BIT >= 16, and exercise the runtime assertion/message macro
with an actual function call. This also avoids assuming CHAR_BIT is eight.
Keep strict warnings: do not turn off C4127 globally to make a test fixture pass.
