# CLR01 — Real-`dotnet` conformance for `cli-assembly-writer`

## Why this spec exists

Every CLR backend in the repo today
(``brainfuck-clr-compiler``, ``oct-clr-compiler``, ``nib-clr-compiler``,
and the new ``twig-clr-compiler``) emits PE/CLI assemblies that run
**only on the in-house ``clr-vm-simulator``**.  Real ``dotnet`` (tested
on 9.0.313) rejects every one of them with:

```
System.BadImageFormatException: File is corrupt. (0x8013110E)
```

The simulator is permissive in ways real .NET isn't.  This spec brings
the assembly writer up to ECMA-335 conformance so that the same
``cli-assembly-writer`` output runs both on the simulator AND on
``dotnet``.  The fix is repo-wide — every existing CLR backend
inherits the conformance for free.

## The gap, diagnosed

I built a known-good reference assembly with C# / `dotnet build` that
returns 42, and compared it to what ``cli-assembly-writer`` produces
for the same logical program.  Differences:

| Field                    | Real .NET / C#       | cli-assembly-writer today |
|--------------------------|----------------------|---------------------------|
| File size (`return 42`)  | 4608 bytes           | 1536 bytes                |
| MS-DOS stub @ 0x40       | "This program …"     | zeros                     |
| TypeDef rows             | `<Module>`, Program  | Program only              |
| AssemblyRef rows         | 1 (System.Runtime)   | **0**                     |
| TypeRef resolution scope | AssemblyRef          | None / dangling           |

The single most important gap is **AssemblyRef**.  Without it, every
``TypeRef`` row's resolution scope points at nothing, so real .NET
can't load any external types — including the bare `int32` /
`object` / `MulticastDelegate` types every assembly transitively
depends on.

## Plan

The fix lands in three logical chunks:

1. **AssemblyRef + minimal System.Runtime reference** (this PR).
   Adds the ``AssemblyRef`` table (0x23), populates it with one
   entry for ``System.Runtime`` (or ``mscorlib`` for net4x; we'll
   target net9.0 here), and re-routes existing ``TypeRef`` rows to
   use that AssemblyRef as their resolution scope.

2. **`<Module>` pseudo-TypeDef and DOS stub** (this PR).
   ``<Module>`` is the special TypeDef row 1 that owns module-level
   fields and global functions.  ECMA-335 §II.22.37 requires it as
   the first row in the TypeDef table; real .NET fails the assembly
   loader if it's missing.  The DOS stub is the cosmetic 80-byte
   "This program cannot be run in DOS mode" header at file offset
   0x40.

3. **Conformance test fixture** (this PR).
   A test that:
     - Generates an assembly via the writer.
     - Drops it in a temp dir alongside a `runtimeconfig.json`
       targeting `net9.0`.
     - Runs `dotnet <name>.exe` as a subprocess.
     - Asserts the exit code matches the expected value.
   The test is gated behind a ``has_dotnet()`` probe so CI without
   the SDK skips rather than fails — same pattern the repo uses for
   other tool-dependent tests.

If the three chunks land cleanly and dotnet still rejects the
assembly, the "File is corrupt" error message will give us a
specific reason to diagnose next; we add chunks until conformance
passes.

## Out of scope for this spec

- **Full ECMA-335 conformance.**  This spec targets the minimum to
  load and execute a return-only program.  Things like generics,
  custom attributes, embedded resources, debug symbols, and
  PEHeader2 details are not in scope.
- **netfx / mono targets.**  The reference is `net9.0`; the same
  AssemblyRef + Module fixes work for `mscorlib` too but we don't
  test against netfx in this spec.
- **Strong naming.**  Generated assemblies stay unsigned.

## Test plan

- Existing `cli-assembly-writer` tests continue to pass (the
  simulator must keep accepting the output).
- New `tests/test_real_dotnet.py`:
    - `test_smoke_return_42` — assembly produced by current writer
      runs on `dotnet` and exits with code 42.
    - `test_arithmetic_via_brainfuck_or_twig` — at least one
      end-to-end path that compiles a real-language program through
      to dotnet.
    - Skipped automatically when `dotnet --version` returns
      non-zero, so CI without the SDK passes cleanly.
