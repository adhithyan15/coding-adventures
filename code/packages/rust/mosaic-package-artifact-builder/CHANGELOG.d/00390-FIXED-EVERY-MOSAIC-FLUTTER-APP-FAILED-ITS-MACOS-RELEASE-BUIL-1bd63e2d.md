### Fixed — every Mosaic Flutter app failed its macOS release build

`flutter build macos --release` has no way to ask for one architecture. It runs
the native-assets hook **once per architecture** and `lipo`s the results into a
universal binary. The generated `hook/build.dart` ignored
`targetArchitecture` and copied the same library both times, so `lipo` got two
slices of one architecture and refused:

```text
fatal error: lipo: .../16e3573553/libmosaic_app.dylib and
.../285c64e3c7/libmosaic_app.dylib have the same architectures (arm64)
and can't be in the same fat output file
```

— two build hashes and no cause. Measured rather than inferred: the two
`input.json` files a failing build left behind said `arm64` and `x64`.

`--debug` worked throughout, because it asks for the host architecture only.
That is why nothing caught it: the CI lane builds `--debug`, and the Engram
release lane — which builds `--release` on `macos-latest` — is the only thing
that ever asked for both.

The Apple hook now hands each architecture its own slice. Proven end to end on
task-app with a control: the same universal library fails the unpatched hook
with the `lipo` error above and builds green with the new one, and the shipped
`mosaic_app.framework` binary is `x86_64 arm64` rather than one of them.

**No silent fallback.** An earlier draft copied the library whole when
`lipo -thin` failed, which turns a thin arm64 library asked for x86_64 straight
back into the original confusing error one step later. A library that cannot
serve the request now says so and says what would:

```text
The bundled Mosaic runtime is arm64 but this build asked for x86_64. On macOS
`flutter build --release` always asks for both arm64 and x86_64, so it needs a
universal runtime: build both targets and `lipo -create` them before passing
--runtime-library.
```

Two smaller holes in the same helper, both found by review:

- **`Process.run` throws when the executable is missing**, rather than returning
  a non-zero exit code, so on a host without the Xcode command line tools the
  build died on a bare `ProcessException` — the opaque failure this change
  exists to replace. It now says what to install.
- **`''.split(...)` is `['']`, not `[]`**, so an empty reply from `lipo` fell
  through into the mismatch message and rendered as "runtime is  but this build
  asked for". Empty is now its own error.

