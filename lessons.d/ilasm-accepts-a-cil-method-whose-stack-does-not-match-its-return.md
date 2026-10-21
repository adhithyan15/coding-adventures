---
category: C#
---

# ilasm accepts a CIL method whose stack does not match its return type; only a real CoreCLR run rejects it

`iir-to-cil-bytecode`'s textual emitter gives the entry the launcher's fixed
`int32 MccarthyEntry()` signature, but lowered IIR `ret_void` to a bare `ret`.
The first `void` IIR `main` to reach the CLR column (Perl's `print` programs,
PR #17850) produced an `int32` method that returned with an empty stack.
`ilasm` assembled it without a word, every `compile_iir_to_cil_text` check in
`op_conformance` passed (it only compiles), and hosts without `dotnet` skip
the CLR column — so the only signal was macOS CI's real CoreCLR throwing
`InvalidProgramException` at JIT time.

Fix: `ret_void` pushes `ldc.i4.0` first when the method's CIL return is
`int32`. Regression test: a structural IL check (the instruction before the
entry's `ret` is a push) that runs with no `dotnet`.

Do differently: when a new IIR *shape* (here, a `void` entry) reaches a backend
whose method signature is fixed independently of the IIR, check every return
path against that signature, and do not read "ilasm assembled it" or "op
conformance compiled it" as "the CLR will run it". To reproduce locally,
`apt-get install dotnet-sdk-10.0`, restore `runtime.linux-x64.Microsoft.NETCore.ILAsm`
into `~/.nuget/packages` (where `clr_support::find_ilasm` looks), and run with
`DOTNET_ROLL_FORWARD=Major` because the test harness writes a .NET 9
`runtimeconfig.json`.
