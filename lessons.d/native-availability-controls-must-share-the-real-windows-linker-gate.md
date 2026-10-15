# Native availability controls must share the real Windows linker gate

The first new hardware ABI test returned UNAVAILABLE on Windows because
toolchain_available(NativeAot) only recognized Linux/macOS. The actual native
runner already uses native_linker_ok and detects the installed Windows linker.
That exit0 receipt is a skipped control, not execution acceptance. Align the
shared helper with the actual runner gate, preserve the non-execution log and
rerun before accepting a red or green hardware result.
