### Added - Mosaic event envelopes for WinUI hosts

Generated non-empty `{Component}.Event.cs` unions now expose `MosaicName`,
`MosaicPayload`, and `MosaicEnvelope` on the base event record, with each nested
record preserving its original Mosaic emit name and payload keys. WinUI hosts
can use the envelope as the JSON-shaped event bridge into shared business logic.

The VisiCalc `Grid` (from `mosaic-pkg-grid`, lowered through
`HostTable` + nested `For` + `Cell`) regenerated into XAML that the
WinUI 3 markup compiler would reject and that would block
`dotnet build`. Four groups of fixes make it valid and
spreadsheet-correct. The demo `code/programs/csharp/visicalc-xaml/` is rewired to
mount the generated `<gen:Grid>` instead of its hand-written
placeholder.

> Verified on macOS via `cargo test -p mosaic-emit-xaml --lib`
> (164 passing) + structural inspection of the generated XAML/C#.
> Runtime / `dotnet build` verification needs Windows.

