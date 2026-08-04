# Font Parser (C#)

Pure C# metrics-only OpenType and TrueType parser with no runtime dependencies.

```csharp
using CodingAdventures.FontParser;

var font = FontParser.Load(File.ReadAllBytes("Inter-Regular.ttf"));
var metrics = FontParser.GetFontMetrics(font);
var glyph = FontParser.GetGlyphId(font, 'A');
var horizontal = FontParser.GetGlyphMetrics(font, glyph!.Value);
```

The parser reads `head`, `hhea`, `maxp`, `cmap` format 4, `hmtx`, optional
`name` and `OS/2`, and legacy `kern` format 0 tables. It deliberately does not
perform glyph outline rasterization.

Run `./BUILD` (or the command in `BUILD_windows`) to execute tests with the
repository's 80% line-coverage gate.
