# CodingAdventures::PaintCodecPngNative (Perl)

Perl XS bridge for PNG encoding.

## Windows

`BUILD_windows` runs `code/scripts/check-perl-extension-windows.ps1`, which
picks the native Strawberry Perl (never Git for Windows' MSYS Perl), builds the
Rust cdylib with `cargo build --release`, stages `target\release\PaintCodecPngNative.dll`
as `lib\auto\CodingAdventures\PaintCodecPngNative\PaintCodecPngNative.<dlext>` (Strawberry's
`$Config{dlext}` is `xs.dll`), and runs `prove -v t/` against it. `perl-bridge`
compiles its C shim with that Perl's own compiler configuration and links the
interpreter's CORE import library.
