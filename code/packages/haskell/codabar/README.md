# codabar (Haskell)

Pure Codabar barcode encoding for the Haskell package lane. Body-only input is
wrapped in `A ... A` by default, callers can choose any `A`-`D` start and stop
pair, and fully guarded input such as `B40156D` is preserved.

```haskell
import CodingAdventures.Codabar

example = drawCodabar
  "40156"
  (CodabarGuards 'B' 'D')
  defaultPaintBarcode1DOptions
```

The package supports digits `0`-`9`, punctuation `- $ : / . +`, and the four
guard symbols `A B C D`. Guards are restricted to the two outer positions.
The API exposes normalized data, typed per-symbol encodings, and the complete
attributed run stream.

Rendering delegates to `barcode-layout-1d`, which supplies validated quiet-zone
geometry, rectangle-only paint instructions, and shared scene metadata. Human-
readable text remains explicitly unsupported until the Haskell lane has a text
shaping backend.

## Development

```sh
cabal test all --enable-coverage
```
