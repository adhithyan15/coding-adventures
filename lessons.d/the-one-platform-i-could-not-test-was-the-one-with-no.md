# The one platform I could not test was the one with no engine check at all

XAML/WinUI is the only backend that cannot be built on a non-Windows machine,
so unlike the other four its payload was never launched by hand before
shipping. Security review asked the right question — *what could go wrong that
nobody would notice* — and the answer was the thing the whole module exists to
prevent.

`build-native.sh` gates every backend on the engine's contract:
`nm | grep -c ' eg_'`, rejecting fewer than 20 symbols, with the comment *"a
library that exists but exports nothing produces the same silent, feature-free
app as no library at all."* Its `case "$(uname -s)"` has arms for Darwin and
Linux and falls through to `*) EXPORTS="unknown"` — **there is no Windows arm**.
And my archiver's only content check was a leading `MZ`, which every PE has,
including the app's own managed assembly. `cp EngramApp.dll engram_capi.dll`
archived cleanly.

So on the platform with no local verification, nothing anywhere verified the
engine. Both halves individually looked reasonable; the gap was in the seam.

Two things made this fixable rather than a guess:

- **`dotnet publish -r win-x64` cross-compiles on macOS**, producing a real
  apphost `.exe` and managed `.dll`, and a previous build had left genuine
  native Windows DLLs in the NuGet cache. So a PE export-directory parser could
  be validated against real positives (`CoreMsgCreateSession`,
  `CreateDwmSceneRenderer`) and real negatives (managed assembly: zero
  exports). "I cannot build the app here" did not mean "I cannot verify
  anything here" — worth asking which parts of a platform are actually out of
  reach before accepting that none can be checked.
- **The real binaries caught a bug my fixture could not have.** I read
  `AddressOfNames` at `IMAGE_EXPORT_DIRECTORY+28`, which is
  `AddressOfFunctions` — an array of *code* addresses. Against real DLLs the
  "names" came back as disassembly. A synthesized fixture built from the same
  wrong layout would have agreed with the parser perfectly, and the check would
  have shipped reading the wrong field. Same lesson as the case-fold fixtures,
  in a new place: **a fixture I write cannot test the assumption I wrote it
  from.**

The review also found a second unnoticeable failure: the csproj's
`FlattenNativeRuntimeDlls` copies the WindowsAppSDK natives beside the
executable because the unpackaged bootstrap looks for them there — but it runs
into `$(OutDir)` and the release lane archives `$(PublishDir)`. The csproj
author had guarded `.pri` and `.xbf` against exactly that mismatch and never
guarded the natives. A payload missing them archives green and fails at launch.
