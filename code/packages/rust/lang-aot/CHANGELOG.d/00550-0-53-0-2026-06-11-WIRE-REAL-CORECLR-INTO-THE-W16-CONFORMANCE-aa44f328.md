## 0.53.0 — 2026-06-11 — wire **real CoreCLR** into the W16 conformance suite (CLR-real C6)

The capstone of the CLR-real verification chapter. `tests/conformance.rs` gains a
ninth backend column, **`CLR-real`**, that runs each McCarthy program on the actual
.NET runtime — textual `.il` → real `ilasm` → real `dotnet` — via the shared
`clr_support` harness. It is gated on `dotnet`+`ilasm` (skips when absent, exactly
like the JVM/BEAM/LLVM columns), so the in-process `clr-simulator` `CLR` column
remains the conformance floor while `CLR-real` proves the **same 19 programs**
agree on real CoreCLR when the toolchain is installed. Locally: 19 programs × 9
backends, all agree. CI (`ci.yml`) now installs `ilasm` whenever .NET is set up — a
`<PackageDownload>` of the RID-specific `runtime.<rid>.Microsoft.NETCore.ILAsm`
runtime pack into the NuGet cache where `find_ilasm()` looks — so the CLR column is
verified on the real runtime rather than only the simulator. This completes
**C1–C6: the CLR backend is verified on real CoreCLR.**

