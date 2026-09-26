### End-to-end verification

Regenerated the `code/programs/csharp/hello-dialog-xaml/` artifacts from the unedited
`.mil`/`.mll`/`.msl` triple via `mosaic-compile --backend xaml`. The
generated XAML, code-behind, and Event union are now byte-identical
to the working hand-patched files in
`code/programs/csharp/hello-dialog-xaml/winui/`. After PR-3 (`--emit-project`) and
PR-4 (regenerate the demo) land, the demo will need zero hand-patches.

## [Unreleased] — U29-1-K-xaml — HostDialog kernel primitive

