# Changelog — font-subset

## Unreleased

### Added — build a font containing only the glyphs a document uses

`subset(&FontFile, &BTreeSet<u16>) -> Subset`, producing a valid TrueType file
ready to embed as a PDF `FontFile2`.

A CJK font is several megabytes and a study sheet uses a few hundred glyphs, so
embedding the whole face turns a two-page export into a 20 MB file. Engram is a
language-study app, so this is the normal case rather than an edge case.

### Subsets by emptying, not by renumbering

The obvious design compacts the glyph set and renumbers 0..N. That is smaller
and considerably more dangerous: `cmap`, `GSUB`, `GPOS`, `MATH`, composite
components and the PDF's own `CIDToGIDMap` all speak glyph ids, and any table
missed renders *plausible wrong glyphs*.

Glyph ids are preserved and unused glyphs emptied. `loca` keeps one entry per
glyph — four bytes each — while `glyf`, which is nearly all of the bulk (213 KB
of Inter's 407 KB; 108 KB of NotoSansJP's 128 KB), shrinks to what was asked
for. `cmap` is carried through unchanged and stays correct precisely because
nothing was renumbered, and `CIDToGIDMap` stays an identity map.

Composite glyphs pull in their components transitively: keeping `á` while
dropping `acute` gives a glyph that references an empty one — no error, just a
missing accent.

### Verified against fontTools

Our parser reading our own subset would share any misunderstanding with the
writer. `tests/fonttools_oracle.rs` hands the bytes to fontTools and asks
whether the retained glyphs still have the same outlines at the same ids,
including Tamil and Japanese — the scripts that force embedding in the first
place, so a subsetter checked only on Latin would be checked on the easy half.

Mutation-checked: dropping the composite closure makes fontTools report a
composite with 17 points where the original has 31.
