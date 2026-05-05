# twig-aot

Twig ahead-of-time compiler.  Reads a Twig source file, produces a native
ARM64 Mach-O executable on macOS that you can run directly.

## Usage

```bash
twig-aot fib.twig -o fib
./fib
echo $?    # → main()'s return value modulo 256
```

## How it works

1. Parse the source with `twig-ir-compiler` → `IIRModule`
2. For each function: infer types (`aot-core`), specialise to typed CIR
3. Lower CIR → ARM64 bytes (`aarch64-backend`)
4. Link per-function bytes (`aot-core::link`)
5. Wrap in a Mach-O object file (`code-packager::macho_object`)
6. Shell out to `/usr/bin/ld` for the final link → trusted provenance,
   ad-hoc code signature, dyld stub

The final `ld` invocation is what makes the binary actually launch on
modern macOS — see CHANGELOG for the trust-model background.

## Requirements

- Apple Silicon Mac running macOS 15+ (Sequoia / Tahoe)
- Xcode Command Line Tools (`/usr/bin/ld`, `xcrun`)
