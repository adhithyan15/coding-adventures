# Text-scale browser review

Run the VisiCalc development server after `npm run build`, then open
`/text-scale-review.html?width=900&scale=1.5&theme=dark`.
The review page loads the real Rust WASM adapter and generated root view.
`width` constrains the app container, not the browser viewport; `scale` supplies
initial host context. This page is a development entry, not the production entry.

## September 20, 2026 evidence

Exercised all 18 combinations of container widths 375, 900 and 1440 pixels,
scales 1, 1.5 and 2, and light/dark themes in the in-app browser.
Entered `=2+3` in the formula field and pressed Enter in each combination.
Computed input fonts were 14/21/28px; visible data rows were 32/48/64px
(with a first-row border rounding difference below 0.5px).
The normal dark view showed A1 = 5 and its dependent row total = 28.

Inspected screenshots of the 375px light view at 200% and the 900px dark
view at 150%. Controls wrap in the narrow view and vertical scrolling exposes
the workbook below them. The wider view retains the formula focus outline,
readable headers, selected-cell border and horizontal grid scrolling.
Changed the live app from 100% to 150% with an uncommitted formula and then
committed it successfully, without recreating the application.

Automated coverage additionally checks preservation of active edits, workbook
serialization and host scale on restore. Rust adapter tests: 17 passed.
React host tests: 39 passed across six files. Production build passed.

This evidence does not complete native projection, screen-reader, OS file-dialog,
physical scrolling or downloaded-release acceptance. Those remain tracked in
#14661 and the migration epic #14267.

After removing an obsolete application cell-editor override, repeated the build and
all 39 web tests. The root package compilation test and 15 grid package tests pass.
In the live 200% dark app, Enter on the focused table opened an inline editor
with a 26px inherited font, zero border and border-box sizing; editing and adjacent
rows remained 64px (first-row border rounding below 0.5px).
