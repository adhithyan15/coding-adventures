# font-subset

Builds a TrueType font containing only the glyphs a document actually uses.

```rust
let bytes  = std::fs::read("NotoSansTamil-Static.ttf")?;
let font   = font_parser::load(&bytes)?;
let wanted = BTreeSet::from([5u16, 20, 60]);

let subset = font_subset::subset(&font, &wanted)?;
// subset.font      -> embed as a PDF FontFile2
// subset.retained  -> what was kept, including composite components
// subset.num_glyphs-> unchanged, so CIDToGIDMap is an identity map
```

## Why

A PDF showing Tamil, Devanagari or CJK has to **embed** a font — no base-14
face can draw any of it. But a CJK font is several megabytes and a study sheet
uses a few hundred glyphs. Subsetting is the difference between an export
people use and one they do not.

## By emptying, not by renumbering

The obvious design renumbers the kept glyphs 0..N. It is smaller, and much
riskier: `cmap`, `GSUB`, `GPOS`, `MATH`, composite components and the PDF's
`CIDToGIDMap` all speak glyph ids. Miss one table and the document renders
*plausible wrong glyphs* — the failure that looks like a font problem and is
not.

Ids are kept; unused glyphs become empty. `glyf` is nearly all of a font's
bulk, so almost all of the saving survives:

| font | total | `glyf` | `cmap` |
|---|---:|---:|---:|
| Inter-Regular | 407 KB | 213 KB | 26 KB |
| NotoSansJP (subset) | 128 KB | 108 KB | 0.3 KB |
| NotoSansTamil | 83 KB | 52 KB | 5 KB |

## Composites drag their components along

`á` is "draw `a`, then `acute` shifted". Keeping `á` without `acute` yields a
glyph referencing an empty one — no error, just a missing accent. The requested
set is closed transitively, since a component can itself be composite.

## Testing

```bash
cargo test -p font-subset          # needs fontTools: pip install fonttools
```

The subset is handed to **fontTools**, which has never seen our assumptions,
and asked whether the retained glyphs still have the same outlines at the same
glyph ids. Latin, Tamil and Japanese are all covered — a subsetter verified
only on Latin is verified on the easy half.

## Scope

TrueType (`glyf`/`loca`). A `CFF `-based OpenType font stores outlines
completely differently; `SubsetError::UnsupportedFontFormat` says so rather
than emitting a font with no outlines.
