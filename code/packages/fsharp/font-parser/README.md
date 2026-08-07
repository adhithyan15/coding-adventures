# Font Parser (F#)

Pure F# metrics-only OpenType and TrueType parser with no runtime dependencies.

```fsharp
open CodingAdventures.FontParser

let font = FontParser.load (System.IO.File.ReadAllBytes "Inter-Regular.ttf")
let metrics = FontParser.fontMetrics font
let glyph = FontParser.glyphId font (int 'A')
let horizontal = glyph |> Option.bind (int >> FontParser.glyphMetrics font)
```

The parser reads `head`, `hhea`, `maxp`, `cmap` format 4, `hmtx`, optional
`name` and `OS/2`, and legacy `kern` format 0 tables. It deliberately does not
perform glyph outline rasterization.

Run `./BUILD` (or the command in `BUILD_windows`) to execute tests with the
repository's 80% line-coverage gate.
