---
category: Cross-platform & Windows BUILD_windows
---

# Capture Win32 last-error state before leaving an unsafe buffer closure

`GetLastError()` reports thread-local state from the immediately preceding
Win32 call. When the call is made inside Swift's
`withUnsafeMutableBufferPointer`, capture both the return value and
`GetLastError()` inside that closure, directly after the API call. Do not rely
on the error state still being unchanged after the closure's lifetime and
cleanup have completed.

This matters for `GetEnvironmentVariableW`: a zero copy count means either a
missing variable or a present-empty value, distinguished by the last-error
code. Keep a nonempty fallback value in the environment smoke case so it
proves that a present-empty primary suppresses fallback; test the missing
primary fallback separately. A Windows external-consumer failure on an empty
revision can otherwise hide whether the reader recognized the empty value or
mistook it for an absent key.
