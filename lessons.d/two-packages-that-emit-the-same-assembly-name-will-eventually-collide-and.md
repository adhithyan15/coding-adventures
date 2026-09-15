# Two packages that emit the same assembly name will eventually collide — and coverlet reports the collision as 0% coverage, not as an error

`fsharp/conduit` failed `build (macos-latest)` with all 40 tests passing and
coverage reported as a flat **0% line / 0% branch / 0% method** against a
`/p:Threshold=80` gate. It had passed the two previous macOS rounds on code
neither intervening commit touched, so it was intermittent, not a regression.

Two independent defects were behind it, and the first one hid the second.

**1. The coverage margin was less than one line.** Measured locally, the
package sits at 169 of 211 lines = **80.09%** against a threshold of **80** —
it needs 168.8 lines, so the entire margin is **0.2 of a line**. One covered
line failing to execute takes it to 79.62% and the build red. Eight sequential
local runs all produced exactly 80.09%, so the number is stable *locally*;
it is the CI environment that perturbs it. CLAUDE.md already requires coverage
to "WELL exceed 80%" — 80.09% is the letter of that rule and the opposite of
its intent.

**2. Two different libraries emitted the same assembly.** Neither
`code/packages/csharp/conduit/CodingAdventures.Conduit.csproj` nor
`code/packages/fsharp/conduit/CodingAdventures.Conduit.fsproj` set
`<AssemblyName>`, so both defaulted to their project file name and both
produced `CodingAdventures.Conduit.dll`, with test assemblies both named
`CodingAdventures.Conduit.Tests` — and coverlet derives its mapping file name
from the *test* assembly (`CoverletSourceRootsMapping_CodingAdventures.Conduit.Tests`),
so renaming only the library would have left half the collision standing. The
build tool schedules packages in
parallel, and the failing macOS run shows `csharp/conduit` (18.3s) and
`fsharp/conduit` (20.0s) overlapping. Two concurrent coverlet runs keyed on
one module name is the only mechanism found that yields *exactly* zero rather
than a low-but-nonzero number. The F# project already declared
`PackageId = CodingAdventures.Conduit.FSharp` and its namespace is
`CodingAdventures.Conduit.FSharp`; only the assembly name disagreed.

Lessons:

1. **A coverage threshold with sub-line margin is a scheduled outage.** Treat
   `covered - threshold*total < ~2 lines` as failing, whatever the gate says.
   Raise it by adding tests, never by lowering the threshold — the gate was
   right, the coverage was thin.
2. **Set `<AssemblyName>` explicitly whenever two packages could plausibly
   share a project file name.** The default (project filename) makes assembly
   identity an accident of directory layout, and nothing warns you: the build
   succeeds, the tests pass, and only a name-keyed tool downstream
   (coverage, instrumentation, a plugin loader) notices there are two of them.
3. **"0%" from a coverage tool means "measured nothing," not "covered
   nothing."** With 40 tests passing, zero is categorically impossible as a
   real measurement — read it as a collection failure and hunt the
   infrastructure, rather than as a signal to go write tests.
4. **When an unrelated package's broken *measurement* blocks real work,
   quarantine the measurement, not the tests.** This gate was disabled (issue
   #11859) so PR #11553's Swift ZIP work could land: `/p:Threshold=80` is
   omitted while `/p:CollectCoverage=true` stays, so all 47 tests still run and
   still fail the build if any breaks — verified by injecting a deliberate
   failure and confirming exit 1 — and the coverage number stays visible in the
   log for whoever re-arms the gate. Disabling the whole package would have
   thrown away a working 47-test suite to silence one broken number.
5. Verify a rename like this by checking the *consumers* build, not just the
   renamed package: `programs/fsharp/conduit-hello` references it via
   `ProjectReference`, which resolves by path and so survives an assembly
   rename — but a by-name reference would not have.
