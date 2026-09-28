### Added — a statically linked runtime for iOS and iPadOS (UI89 §2.1)

- `CMosaicRuntime.c` gains a `MOSAIC_RUNTIME_STATIC` path: the runtime's C
  ABI functions are named directly, so a static library linked from an
  `.xcframework` is kept by the linker and called without `dlopen`/`dlsym`
  (which iOS does not allow for an app's own dylib). A path is ignored; close
  never `dlclose`s the static marker. The default dynamic path is unchanged.
- `swift_package_with_static_runtime` adds the `MosaicAppRuntime` binary
  target (`Runtime/MosaicAppRuntime.xcframework`, `SWIFT_STATIC_RUNTIME_PATH`)
  as a dependency of the loader and defines the macro.

