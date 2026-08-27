### Added — cited Chinese numeral ductus 一 二 三 四 五 (HL-C230)

- Render the five numerals from Hanzi Writer Data's ordered medians at pinned commit `68d10a4`: 一 as one unbroken héng; 二 and 三 as top-before-bottom héng runs with the widening base; 四 as the box built left wall, then top-and-right in one turning héngzhé, two inner pieces, bottom closed last; 五 as top bar, leaning shù, héngzhé, and the widest closing stroke.
- **These were obligations, not additions.** `data/scripts/chinese.json` gained `penLifts` and `strokeOrderSource` for the five numerals, and this package's bidirectional invariant requires every verified prose claim to have a font-checked ductus — so authoring the lesson content created the debt this entry pays. Chinese stays 29/29 verified.
- Paths are traced from the vendored Noto Sans SC outline rather than transformed from the source graphics, so the pen path follows the ink the reader actually sees. All five score 1.0000 on-ink, zero join gaps, and zero stray coverage.
- One instrument was broken and fixed first: an even-odd fill test invented phantom holes where 五's contours overlap, which would have put the top bar's traced centre between two strokes. The real check uses non-zero winding; authoring was redone against it. A second failure mode — the tracer jumping bands where two strokes cross — is fixed by clamping the band to one stroke's width.
- The focused suite now passes 1,231 tests.

