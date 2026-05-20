# Changelog — `lang-aot`

## 0.1.0 — 2026-05-20

Initial release.  Multi-language AOT driver that routes Twig, Nib, and
Brainfuck source through the shared LANG VM chain (frontend → IIR →
x86_64-backend / aarch64-backend → object → system linker → native
executable).

### What's wired

| Language | Extensions | Frontend |
|---|---|---|
| Twig | `.twig` | `twig-ir-compiler` |
| Nib  | `.nib`  | `nib-iir-compiler` |
| Brainfuck | `.bf`, `.b` | `brainfuck-iir-compiler` (IIR-emission works; AOT backend doesn't lower BF ops yet) |
| Dartmouth BASIC | `.bas`, `.basic` | placeholder — returns `UnsupportedLanguage` with guidance |
| Oct | `.oct` | placeholder — returns `UnsupportedLanguage` with guidance |

### API

- `Language` enum with `parse(&str)` and `Display`.
- `detect_language_from_path(&Path) -> Option<Language>` — by extension.
- `compile_source_to_iir(language, source, module_name) -> Result<IIRModule, LangAotError>`
  — frontend dispatch.
- `compile_file_to_{linux, windows, macos}_executable(src, out, lang)`
  — full pipeline, cfg-gated to the matching host (same host-targets-
  host policy as `twig-aot`).
- `LangAotError` with `UnsupportedLanguage { language, guidance }`,
  `FrontendError`, `AotError`, `Io` variants.

### Companion change in `twig-aot`

`twig-aot` exposes three new public functions:

- `compile_module_to_linux_executable(&IIRModule, &Path)` (Linux host).
- `compile_module_to_windows_executable(&IIRModule, &Path)` (Windows host).
- `compile_module_to_macos_executable(&IIRModule, &Path)` (Unix host).

…and three new public link helpers:

- `link_linux_x86_64_executable(obj_bytes, stem, out)`.
- `link_windows_x86_64_executable(obj_bytes, stem, out)`.
- `link_macos_arm64_executable(obj_bytes, stem, out)`.

The existing `compile_file_*` functions now delegate to these so the
link logic is shared between source-file input and module input.

### Tests

- 7 lib tests cover language parsing, extension detection, and the
  unsupported-language error paths.
- 3 end-to-end smoke tests (`tests/end_to_end_smoke.rs`) gated to
  the host's OS:
  - `end_to_end_twig_returns_42_via_lang_aot`
  - `end_to_end_nib_returns_42_via_lang_aot`
  - `end_to_end_nib_arithmetic_via_lang_aot` (`30+12`, `if 1==1`,
    `if 1==2`)

All tests pass on Windows x86-64 host.  CI will additionally verify
on `ubuntu-latest` and `macos-latest`.

### Known limitations

- **Host-targets-host only.** Same as `twig-aot` V1.
- **No `--target` / `--emit-object` CLI flags.** Coming in a follow-up.
- **Brainfuck end-to-end gap.** Frontend produces correct IIR, but the
  x86_64-backend and aarch64-backend don't lower BF-specific ops
  (`load_mem`, `putchar`, etc.).  Wiring is correct; backend extension
  is a separate piece of work.
- **Dartmouth BASIC and Oct stubs.** They surface
  `UnsupportedLanguage` errors with one-line guidance on what's needed
  to unblock each.
