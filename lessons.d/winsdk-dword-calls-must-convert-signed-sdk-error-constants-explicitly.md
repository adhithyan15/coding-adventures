---
category: Cross-platform & Windows BUILD_windows
---

# WinSDK DWORD calls must convert signed SDK error constants explicitly

Swift 6.4.2 on Windows Server 2025 rejected `SetLastError(ERROR_SUCCESS)` in
the key-scoped environment reader: WinSDK imports `SetLastError` as taking
`DWORD` (`UInt32`), while `ERROR_SUCCESS` is exposed as `Int32`. The same
signedness boundary exists when comparing `GetLastError()` results with SDK
constants.

Convert each SDK constant once with `DWORD(bitPattern:)`, pass the converted
value to `SetLastError`, and compare `GetLastError()` against converted DWORD
values. Keep the conversion scoped to the WinSDK adapter; do not broadly cast
environment lengths or weaken the API's bounds. Linux compilation cannot type
check code hidden by `#if os(Windows)`, so retain the actual hosted Windows
build as the authority for this branch.
