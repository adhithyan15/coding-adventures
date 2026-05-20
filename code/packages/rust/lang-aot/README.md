# lang-aot

Multi-language AOT driver — compile **Twig, Nib, Brainfuck** (and, once
their IIR frontends land, Dartmouth BASIC and Oct) to native
executables through the shared LANG VM chain.

## Stack position

```
<lang> source
   │
   ▼ <lang>-iir-compiler                  (per-language frontend)
interpreter_ir::IIRModule                  ← lingua franca
   │
   ▼ twig_aot::compile_module_to_*_executable
       (x86_64-backend / aarch64-backend  →  elf/pe/macho_object  →
        system linker)
   │
   ▼
native executable
```

The point of this crate is the **dispatch layer** at the top: pick the
right frontend based on the input file's extension or an explicit
`--lang` flag, then hand the resulting `IIRModule` to twig-aot for the
rest of the chain.

## CLI

```text
lang-aot <FILE> [-o <OUT>] [--lang <LANG>]
```

| Language | Extensions | Frontend crate | Status |
|---|---|---|---|
| Twig            | `.twig`         | `twig-ir-compiler`       | full |
| Nib             | `.nib`          | `nib-iir-compiler`       | full |
| Brainfuck       | `.bf`, `.b`    | `brainfuck-iir-compiler` + BF07 lowering pass | full — `lang-aot foo.bf` compiles end-to-end (cells live in a 30000-byte `alloc_bytes` tape; `load_mem`/`store_mem` are rewritten to `load_byte`/`store_byte` per LANG76) |
| Dartmouth BASIC | `.bas`, `.basic` | `dartmouth-basic-iir-compiler` | full — integer programs with LET / PRINT / INPUT / IF / GOTO / FOR / NEXT / END / REM compile end-to-end (PL05).  GOSUB / arrays / strings / DEF deferred to V2 |
| Oct             | `.oct`          | **TODO**               | only Python frontend exists; needs a Rust port or a bridge |

If `--lang` is omitted the language is inferred from the file
extension; unknown extensions get a "could not infer language" error
listing the recognised ones.

## Example

```bash
$ echo 'fn main() -> u8 { return 42; }' > hello.nib
$ lang-aot hello.nib
$ ./hello
$ echo $?
42
```

That ran a Nib source file all the way to a native executable on the
host — via the same `x86_64-backend` and `elf_object` / `pe_object` /
`macho_object` packagers Twig uses.

## Adding a new language

1. Build a `<lang>-iir-compiler` crate whose `compile_source(&str, &str)
   -> Result<interpreter_ir::IIRModule, _>` mirrors `nib-iir-compiler`
   or `brainfuck-iir-compiler`.
2. Add a variant to the [`Language`] enum and wire
   `compile_source_to_iir` to call your new frontend.
3. Add the file extension to `detect_language_from_path`.
4. Add a smoke test in `tests/end_to_end_smoke.rs` with a small program
   that compiles to a known exit code.

No backend changes needed — every frontend gets x86-64 Linux, x86-64
Windows, and ARM64 macOS for free.

## Limitations

- **Host-targets-host only.** Same V1 policy as `twig-aot`.  Use the
  `twig-aot --target= --emit-object` workflow for cross-OS object
  emission.
- **BF backend gap.** `brainfuck-iir-compiler` emits IR ops
  (`load_mem`, `store_mem`, `putchar`, `getchar`, …) that the x86_64
  and aarch64 AOT backends don't lower today.  The dispatch layer here
  correctly produces the IIR, but compilation to executable fails at
  the backend step.  Extending the backends to support these ops is a
  separate follow-up.
- **No `--target` / `--emit-object` flags yet.** `lang-aot`'s CLI is
  intentionally minimal in V1.  Cross-OS support will land alongside
  multi-language `--emit-object` once we have a story for cross-host
  runtime archives.
