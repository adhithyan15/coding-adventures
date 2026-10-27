---
category: CI & GitHub Actions
---

# A change to a committed font must regenerate every oracle generated from it, or a full build fails on a package the font PR never built

**What went wrong.** #18338 (Japanese B1 chapters) re-subset
`code/learning/human-languages/_fonts/NotoSansJP-Subset.ttf` from 647 to 1,551
glyphs. `glyph-parser`'s oracle (`tests/fixtures/oracle.txt`, generated from
that font by fontTools) still said 647, so
`oracle::every_glyph_matches_fonttools` failed with `noto-jp: numGlyphs left:
1551 right: 647`. The font PR only touched human-languages files, so
diff-based CI never built `rust/glyph-parser`. The failure first showed up on
an unrelated `ci-full-build` PR (#18407), shard 6.

**The fix.** Run `python3 code/scripts/generate_glyph_oracle.py` (it needs
`pip install fonttools`), then `cargo test -p glyph-parser`. Commit only
`oracle.txt`. Revert the regenerated `synthetic.ttf`, which differs only by
fontBuilder's build timestamp.

**What to do differently.** When a PR changes a font under
`code/learning/human-languages/_fonts/` or `code/fixtures/fonts/`, grep for
the font's path (`git grep -l <font file name>`). Regenerate every oracle or
fixture generated from it in the same PR, and run those packages' tests,
because the build tool's path mapping will not schedule them.
