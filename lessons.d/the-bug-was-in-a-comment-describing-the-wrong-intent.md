# The bug was in a comment describing the wrong intent

The XAML emitter could not compile any package with a layout variant. The code
that did it was doing exactly what its comment said:

```
// Variant infix applies to both secondaries so a multi-variant XAML build
// produces e.g. Grid.touch.xaml + Grid.touch.xaml.cs + Grid.touch.Event.cs
// alongside the desktop trio.
```

That is right for `.xaml.cs` — a partial class for that layout — and wrong for
`.Event.cs`. The event union comes from `emit_events(name, emits, ..)`, which
never sees the layout, so every variant emits the same type, with the same name,
in the same namespace. The emitted csproj compiles `**/*.cs`, so C# saw two
definitions of `EngramAppEvent` and refused. The two files were byte-identical —
same SHA — which is the tell: **a per-variant file whose contents do not depend
on the variant is not a per-variant file.**

It survived because the compile step only runs on Windows and everyone works on
macOS. Emission runs anywhere, so `grep -c EngramAppEvent *.Event.cs` would have
found it at any point in the last however-long.

Two process notes:

- **The dedupe went in the wrong scope first, and the test caught it.** I
  removed the duplicate *file* and also tried to drop the duplicate *artifact
  entry* inside `compile_one_component` — but that function gets a fresh vector
  per variant, so `contains` could never see the other one. The file assertions
  passed and the artifact-count assertion failed, which is the test doing its
  job: asserting the two consequences separately meant only the wrong half
  failed.
- **I ran the full crate suite in the wrong checkout.** `cd
  /Users/adhithya/Downloads/coding-adventures/code/packages/rust` succeeds — it
  is the main checkout, not this worktree — so a green "121 passed" said nothing
  about my change. Nothing was damaged (cargo only wrote a gitignored `target/`),
  but the result was meaningless and I nearly took it as verification. Absolute
  paths into the worktree, or `pwd` first.
