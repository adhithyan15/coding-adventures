# font-parser (Go)

Metrics-only OpenType/TrueType font parser. Zero external dependencies.

## Usage

```go
import fp "github.com/adhithyan15/coding-adventures/code/packages/go/font-parser"

data, _ := os.ReadFile("Inter-Regular.ttf")
font, err := fp.Load(data)
if err != nil {
    log.Fatal(err)
}

m := fp.GetFontMetrics(font)
fmt.Println(m.UnitsPerEm)  // 2048
fmt.Println(m.FamilyName)  // "Inter"

gidA, _ := fp.GlyphID(font, 'A')
gidV, _ := fp.GlyphID(font, 'V')

kern := fp.Kerning(font, gidA, gidV) // 0 for Inter (GPOS only)
```

## API

| Function | Description |
|---|---|
| `Load(data []byte) (*FontFile, error)` | Parse font bytes |
| `GetFontMetrics(font) *Metrics` | Global typographic metrics |
| `GlyphID(font, rune) (uint16, bool)` | Unicode → glyph ID (BMP only) |
| `GetGlyphMetrics(font, uint16) (*GlyphMetrics, bool)` | Per-glyph metrics |
| `Kerning(font, left, right uint16) int16` | Kern pair value (0 if absent) |

## Design

`Load` copies the font bytes and pre-parses the table directory. All metric
queries use `encoding/binary.BigEndian` for reading — no bit shifts, no
manual endian handling. Name table strings are decoded with
`unicode/utf16.Decode`.
