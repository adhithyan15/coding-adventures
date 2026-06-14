# LANG46 — `twig-aot`: Multi-Target Driver (Linux + Windows x86-64)

**Status:** Draft — 2026-05-14

> Last of four specs in the x86-64 port (LANG44 encoder → LANG43
> backend → LANG45 packager → **LANG46** driver).  This one extends
> `twig-aot` from "macOS ARM64 only" to support **Linux x86-64** and
> **Windows x86-64** end-to-end: Twig source → native executable that
> runs.

## Motivation

`twig-aot` today is hard-wired to a single pipeline:

```
Twig source → ARM64 backend → Mach-O object → ld → macOS ARM64 a.out
```

That's the pipeline `compile_macos_arm64_object` and
`compile_file_macos_arm64` implement.  Extending to Linux x86-64 and
Windows x86-64 needs four changes:

1. **Backend selection** — pick `aarch64-backend` or `x86_64-backend`
   based on the target.
2. **Object-format selection** — pick `macho_object`, `elf_object`,
   or `pe_object` based on the target.
3. **Runtime archive selection** — embed the right pre-built archive
   (`libtwig_aot_runtime-{linux,windows}-x86_64.a/.lib`) for the
   target's linker to consume.
4. **Linker invocation** — call `cc` on Linux, `link.exe` or
   `lld-link.exe` on Windows.

This spec lays out the design.  It assumes LANG43, LANG44, LANG45 have
landed.

## Non-goals

- **Cross-compilation between hosts**.  V1 supports only
  *host-targets-host* compilation: build x86-64 ELF on a Linux x86-64
  host, build x86-64 PE on a Windows x86-64 host.  Cross-OS
  cross-compilation (e.g., produce a Windows .exe from a Linux host)
  is a follow-up; it requires either MinGW-style toolchain detection
  or shipping target-specific linker binaries with `twig-aot`.
- **macOS x86-64** — same reasoning as LANG43.
- **Linux ARM64** — encoder + backend already support ARM64; the
  only missing piece is `elf_object.rs` (covered in LANG45), so this
  target drops out for free once LANG45 lands.  Not formally claimed
  in V1 tests but tracked.
- **Static-linked binaries** — V1 produces dynamically-linked
  executables on Linux and Windows.  Static binaries (e.g.
  `-static`) are a follow-up.
- **Stripping / signing** — V1 ships unstripped, unsigned binaries.
  Adequate for development and CI.

## CLI surface

The existing `twig-aot` CLI (`code/packages/rust/twig-aot/bin/twig_aot.rs`)
gains a `--target` flag:

```
twig-aot compile <source.twig> [--target <triple>] [-o <out>]
```

Accepted target triples in V1:

| Triple | Backend | Object format | Linker |
|---|---|---|---|
| `aarch64-apple-darwin` (default on macOS ARM64) | `aarch64-backend` | `macho_object` | `ld` |
| `x86_64-unknown-linux-gnu` (default on Linux x86_64) | `x86_64-backend` (SysV) | `elf_object` | `cc` (or `ld`) |
| `x86_64-pc-windows-msvc` (default on Windows x86_64) | `x86_64-backend` (MS x64) | `pe_object` | `link.exe` |
| `x86_64-pc-windows-gnu` | `x86_64-backend` (MS x64) | `pe_object` | `lld-link.exe` or `gcc` (MinGW) |

If `--target` is omitted, `twig-aot` defaults to the *host* triple
(detected via `std::env::consts::OS` + `std::env::consts::ARCH`).
V1 errors out on cross-OS combinations (e.g. host=linux,
target=windows).

## Pipeline (per target)

### Linux x86-64

```text
Twig source
   │ twig_ir_compiler::compile_source
   ▼
IIRModule
   │ x86_64_backend::compile_function (one per fn, SysV ABI)
   ▼
Vec<(fn_name, Vec<u8>)>
   │ aot_core::link::link
   ▼
(text_bytes, offsets)
   │ code_packager::elf_object::pack_elf64_object_with_globals_and_externals
   ▼
factorial.o   (ET_REL, EM_X86_64, R_X86_64_PLT32 reloc for __twig_print_i64)
   │ cc factorial.o libtwig_aot_runtime_linux_x86_64.a -lc -o factorial
   ▼
ELF64 executable, runs on any glibc Linux x86-64
```

### Windows x86-64

```text
Twig source
   │ twig_ir_compiler::compile_source
   ▼
IIRModule
   │ x86_64_backend::compile_function (one per fn, MS x64 ABI)
   ▼
Vec<(fn_name, Vec<u8>)>
   │ aot_core::link::link
   ▼
(text_bytes, offsets)
   │ code_packager::pe_object::pack_pe_object_with_globals_and_externals
   ▼
factorial.obj   (AMD64, IMAGE_REL_AMD64_REL32 reloc for __twig_print_i64)
   │ link.exe /OUT:factorial.exe /SUBSYSTEM:CONSOLE /ENTRY:main
   │     factorial.obj libtwig_aot_runtime_windows_x86_64.lib libcmt.lib legacy_stdio_definitions.lib
   ▼
factorial.exe, runs on Windows x86-64
```

Note: on Linux the prebuilt runtime is a `.a` archive of object
files; on Windows it's a `.lib` import library wrapping the same
object files (built with MSVC or MinGW at `twig-aot` build time).

## Runtime archive: per-target builds

Today `twig-aot/build.rs` invokes the `cc` crate once to build
`twig_runtime.c` for the host triple, and embeds the result via
`include_bytes!`.

V1 extends this to build **all supported target archives at
`twig-aot` crate build time**, embedding each:

```rust
// build.rs (post-LANG46)
fn main() {
    let runtimes = [
        ("aarch64-apple-darwin",     "libtwig_aot_runtime_macos_arm64.a"),
        ("x86_64-unknown-linux-gnu", "libtwig_aot_runtime_linux_x86_64.a"),
        ("x86_64-pc-windows-msvc",   "twig_aot_runtime_windows_x86_64.lib"),
    ];
    for (triple, out_name) in runtimes {
        if cc_can_build_for(triple) {
            build_runtime_for(triple, out_name);
            println!("cargo:rustc-env=TWIG_RUNTIME_ARCHIVE_{}={}/{}",
                     triple.replace('-', "_").to_uppercase(),
                     env::var("OUT_DIR").unwrap(),
                     out_name);
        } else {
            // Emit a stub byte string so include_bytes! still works.
            // Compile fails at *AOT* time if the user picks this
            // target, with a clear "no toolchain for X on this host"
            // error.
            println!("cargo:rustc-env=TWIG_RUNTIME_ARCHIVE_{}=...",
                     triple.replace('-', "_").to_uppercase());
        }
    }
}
```

`cc_can_build_for(triple)` tries `cc --target=<triple>` (clang) or
`{triple}-gcc` (cross-compiler).  If neither is available, the
runtime for that triple is a stub — the AOT compiler refuses to
emit for the target with a clear error.

### Why build at crate build time, not at `twig-aot` run time

- **No `cc` dependency on end-user machines.**  A Twig developer
  doesn't need GCC/clang installed — only the system linker for
  their host OS (which is universally present).
- **Reproducible:**  same `twig-aot` binary → same runtime bytes,
  always.
- **Fast:**  no compile step per Twig program; just write embedded
  bytes to a temp file and pass to the linker.
- **Matches macOS pattern:**  `macos_arm64` already does this; the
  multi-target extension is the obvious generalisation.

### Test-friendly fallback

For local development on a host that *doesn't* have a
cross-compiler, `build.rs` emits a fallback diagnostic and the
runtime variable carries 1 byte of zeros.  AOT compilation for that
target will fail with `AotError::Runtime("no toolchain for
x86_64-pc-windows-msvc on this host; install MSVC or MinGW")`.

CI matrix already has `windows-latest` and `ubuntu-latest`, so
runtime archives for both targets *will* be built in CI for every
PR.

## Linker invocation (per target)

`twig-aot/src/lib.rs` gains a `link_executable` function with a
per-target match.

### Linux: prefer `cc`

```rust
Command::new(env::var("CC").unwrap_or_else(|_| "cc".into()))
    .arg("-o").arg(out_path)
    .arg(object_path)
    .arg(runtime_archive_path)
    .arg("-lc").arg("-lm")
    .status()
```

If `cc` is missing, fall back to `ld` with explicit `crt1.o` /
`crti.o` / `crtn.o` paths (best-effort).

### Windows: prefer `link.exe` then `lld-link.exe` then `gcc`

```rust
// Try link.exe from MSVC installation (resolved via vswhere)
// then lld-link.exe (if LLVM is on PATH)
// then gcc (MinGW)
let linker = find_windows_linker()?;
match linker {
    WinLinker::Link(path) => Command::new(path)
        .arg(format!("/OUT:{}", out_path.display()))
        .arg("/ENTRY:main")
        .arg("/SUBSYSTEM:CONSOLE")
        .arg(object_path)
        .arg(runtime_archive_path)
        .arg("libcmt.lib")
        .arg("legacy_stdio_definitions.lib")
        .status(),
    WinLinker::LldLink(path) => /* same flags */,
    WinLinker::MinGwGcc(path) => Command::new(path)
        .arg("-o").arg(out_path)
        .arg(object_path)
        .arg(runtime_archive_path)
        .status(),
}
```

`find_windows_linker()` is small: probe PATH for `link.exe`, then
`lld-link.exe`, then `gcc.exe`.  V1 does *not* attempt to locate an
unregistered MSVC install via `vswhere` — the user is expected to
have run `vcvarsall.bat` or have LLVM/MinGW on PATH.

## Entry-point conventions

A Twig program's `main` function returns `u64`.  The runtime needs
to route that return value to the OS's exit code.

| Target | OS entry | How Twig's `main` is wired |
|---|---|---|
| Linux x86-64 | `_start` (libc-supplied via `Scrt1.o`) calls `main` | Twig emits a symbol named `main` returning `int64_t`; libc's `_start` passes it to `exit(2)`.  We compile `main` to clear the upper 32 bits before returning, so the exit code matches `main() & 0xFF`. |
| Windows x86-64 | `mainCRTStartup` calls `main` | Same: emit a symbol named `main`; CRT routes return to `ExitProcess`. |
| macOS ARM64 (existing) | `_start` calls `_main` | Existing AArch64 path emits `_main`; unchanged. |

Note the leading-underscore difference: macOS uses `_main`,
Linux/Windows use `main`.  `twig-aot` selects the symbol name per
target.

## Symbol-name convention summary

| Symbol | macOS ARM64 | Linux x86-64 | Windows x86-64 |
|---|---|---|---|
| Entry point | `_main` | `main` | `main` |
| Runtime print | `___twig_print_i64` (double underscore by Apple convention) | `__twig_print_i64` | `__twig_print_i64` |
| Globals slab | `_twig_globals` | `_twig_globals` | `_twig_globals` |

The first row matters because macOS adds an extra leading
underscore for `_start`-style entries.  Linux and Windows don't.

## End-to-end acceptance tests

V1 adds two new integration tests:

1. `code/packages/rust/twig-aot/tests/linux_x86_64_smoke.rs`
   (`#[cfg(target_os = "linux")]`):
   ```rust
   #[test]
   fn factorial_runs_on_linux_x86_64() {
       let out = tempfile();
       compile_file(&factorial_twig_path(), &out,
           Target::linux_x86_64()).unwrap();
       let status = Command::new(&out).status().unwrap();
       assert_eq!(status.code(), Some(120)); // 5! = 120
   }
   ```
2. `code/packages/rust/twig-aot/tests/windows_x86_64_smoke.rs`
   (`#[cfg(target_os = "windows")]`):
   ```rust
   #[test]
   fn factorial_runs_on_windows_x86_64() {
       let out = tempfile_with_extension("exe");
       compile_file(&factorial_twig_path(), &out,
           Target::windows_x86_64()).unwrap();
       let status = Command::new(&out).status().unwrap();
       assert_eq!(status.code(), Some(120));
   }
   ```

Both tests run in CI on their respective `*-latest` runners.

A third test on `macos_arm64_smoke.rs` (existing) continues to
guard the ARM64 path against regressions.

## Risk register

| Risk | Mitigation |
|---|---|
| Cross-compiler not present on user's host → runtime archive empty | `build.rs` emits stub bytes + clear AOT-time error referencing the missing toolchain |
| Linker invocation path differs across distros / Windows installs | V1 probes PATH in priority order; documents the assumption in error messages.  Follow-up: integrate `vswhere` for MSVC discovery. |
| Symbol name mismatch (`_main` vs `main`) breaks one platform | Single `target.entry_symbol()` helper used everywhere; one test per platform locks the convention. |
| Windows exit-code truncation (`int` is 32-bit on Win64) | Twig's `main` returns `u64`; ABI says low 32 bits → `eax` becomes the process exit code → Windows truncates to 32 bits → shell shows it as `& 0xFF`. Document this explicitly; test asserts `code() == Some(120)` works. |
| Embedded archive bloat | Each runtime archive is ~5 KB. Three targets ≈ 15 KB embedded in `twig-aot`. Acceptable. |
| Future-target onboarding pain | The per-target table is the single source of truth; adding a target = one row + one `build.rs` entry + one acceptance test. |

## Out of scope (deferred follow-ups)

- Cross-OS cross-compilation (build Windows .exe from Linux, etc.).
- Stripped / signed binaries.
- Static linking (`-static` on Linux, `/MT` on Windows already used).
- LTO across runtime + user code.
- Multiple-source-file Twig programs (depends on module-system work).
- macOS x86-64.

## Sequencing

This spec lands as part of the PR series that brings x86-64 to
parity:

1. LANG44 spec (encoder) and LANG43 amendment (both ABIs) and LANG45
   spec and LANG46 spec — single PR (#3183).
2. `x86_64-encoder` crate.
3. `x86_64-backend` V1 (System V + MS x64 in one go).
4. `x86_64-backend` LANG38 parity (div / logical / shifts).
5. `x86_64-backend` calls + relocs.
6. `x86_64-backend` globals + `io_out`.
7. `code-packager::elf_object`.
8. `code-packager::pe_object`.
9. `twig-aot` multi-target runtime archives in `build.rs`.
10. `twig-aot` target dispatch + linker shell-out + Linux/Windows
    acceptance tests.

After PR 10, `cargo run -p twig-aot -- compile factorial.twig` on a
Linux x86-64 or Windows x86-64 host produces a working native
binary.
