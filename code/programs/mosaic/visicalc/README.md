# VisiCalc Mosaic application

The root VisiCalc component composes the formula field, workbook toolbar and
mosaic-pkg-grid. The Rust visicalc-mosaic-app adapter supplies all presentation
slots and owns selection, editing, viewport coordinates and workbook operations.
The web consumer lives in code/programs/typescript/visicalc and uses the standard
mosaic-app-wasm lifecycle. Native consumers use the same adapter's C ABI.

Run `cargo test` here for source and manifest checks. The web consumer's
`npm test` regenerates both themes through the package resolver and exercises
real controls with the compiled Rust application. Generated artifacts are not
committed. The fixture directory remains shared by Rust and browser tests.

`VisiCalcStartup` is the package's second export: a shared loading/error surface
with both themes and an error-only Retry control. `VisiCalc` remains the first
export and the native project shell's default root. The web host owns pre-engine
loading/retry state, disposes results from abandoned attempts, and focuses the
workbook after successful retry. It renders startup status/error announcements
without exposing low-level loader details. Full visual and text-scale acceptance
remains #14273; React heading projection is tracked in #14610.

The shared file toolbar exposes New workbook, Open and Save in both themes, with
a compact Rust-owned status and polite announcements. The web host opts into
protocol 2 and uses Mosaic's shared browser file executor directly from the
button gesture. Apply an active cell edit with Enter before saving. Files use the
`.visicalc` extension and the versioned Rust snapshot envelope. Cancelled or invalid
opens preserve both committed cells and the pending edit; unsupported hosts report
that Open/Save is unavailable.

Rust and generated-control tests round-trip actual file bytes into a fresh app.
Real OS file-dialog acceptance remains open in #14548 because the available
in-app test panel does not expose the picker to desktop automation. Native hosts
still need capability migration and launch acceptance; GitHub Releases and
downloaded-artifact tests remain required in #14282.

The migration backlog is GitHub issue #14267. Native application acceptance,
responsive physical scrolling, accessibility, full persistence, finished visual
design and GitHub Releases remain required work.

The first shared design pass uses warm paper and forest palettes with serif
workbook branding, a monospace formula field and grid, and separate green
selection and amber editing states. Toolbar content wraps at narrow widths;
the sheet frame uses the remaining height of a viewport-sized root. Authored
column-headings styles pin the header in the React consumer. Browser Page Down
moved the sheet by 364px while the header stayed at the same screen position,
with 30 rows materialized. The shared React table now reveals the selected cell
within that frame, including horizontal navigation and sticky-header clearance.
Browser navigation verified A31 and Z31 at the lower/right edges, then Z2 below
the header, while keeping 30 rows rendered; formula-editor focus survived edits.
The shared React HostTable observer now reports measured uniform row capacity
through Grid's opt-in onViewportRows event. The Rust adapter clamps this to the
workbook size and reveals the selection after resizing. Browser resizing at Z100
produced 2 rows in a 400px-high frame, 17 in a 900px-high frame, and 15 when that
tall frame narrowed to 375px. Missing ResizeObserver or variable row heights
produce a diagnostic; hidden/empty tables do not publish invented capacity.
Native capacity and native reveal remain under #14277 and #14372.

The opt-in RowHeaderGrid now displays absolute labels from the Rust adapter.
React emits scoped row/column headers and applies authored geometry directly to
the cell wrappers. Browser measurements show aligned 80px data columns and
approximately 32.33px row pitch at the tested display scale. Horizontal keyboard
navigation to Z retains pinned row labels; returning to A clears their bounds.
A 375px-wide light-theme preview kept Z100 visible with labels 94 through 100.
The actual-WASM test edits Z100 without touching Y100. Native row-header support
is explicitly degraded and remains in #14388, alongside broader accessibility
and text-scale acceptance. React whole-workbook physical scrolling is described below.

React wheel/trackpad routing now advances the row window through the workbook.
Live wheel input traversed rows 1–12, 38–49 and 89–100 with 12 rows materialized.
Clicking the final row selected A100; entering 42 changed that cell, and returning
to rows 1–12 left A1 at 15. The adapter preserves absolute selection and buffered
edits while scrolling. Small deltas accumulate; horizontal, zoom and boundary
gestures retain native behavior. Wheel movement remains discrete.

React now emits hidden spacer sections representing unrealized rows. The native
scrollbar spans all 100 rows while only 14 data rows were realized in a 447px
scroll frame. Native scroll-position acceptance reached labels 87-100 with row 100 visible
and column headers pinned. A100 editing succeeded; native scrolling returned to the first
window with A1 unchanged at 15. A temporary acceptance page supplied top/bottom
buttons calling the browser scroll API, without dispatching application events. The median row interval avoids collapsed-border boundary drift (#14476).
Capacity includes a partial row and one additional row for fractional
scroll travel. Native frameworks, smooth wheel input and touch/accessibility
acceptance remain in #14277.

Browser review on 2026-09-05 covered both generated themes at desktop width
and the real Rust app in a 375px-wide preview. The narrow root's scroll width
remained 375px, and editing A1 from 15 to 20 recomputed E5 to 174. Text scaling,
loading/error/empty presentation, keyboard focus and native appearance remain
acceptance work under #14273 and #14278. Negative outline offsets exposed the
shared signed-dimension limitation tracked in #14327.

Keyboard focus now enters the named Data table with Tab or a cell click. Plain
arrows navigate cells; Home/End select the first/final column of the current row.
F2, Enter or a printable character opens the inline editor with focus; Enter or
Escape returns focus to the table when that editor is removed. Shortcuts belong
to this app's table, so keys elsewhere on the page and formula-editor caret keys
do not navigate the workbook. Modified navigation and IME composition are left
alone. Complete grid roles/selected-cell announcements, native focus and broader
assistive-technology acceptance remain under #14278.

The selection readout beneath the sheet shows the selected address and committed
display value, including formula source and results. Blank cells are named
explicitly. Long descriptions truncate visually inside the worksheet; the host's
atomic polite live region preserves the full message without adding layout
height. Navigation, entering an editor, commit/cancel and restoration use Rust
messages; typing and viewport changes do not repeat announcements. Inline commit
names both the updated cell and the new selection. These are DOM/runtime checks;
real screen-reader and native accessibility acceptance remain open in #14278.

Browser acceptance covered the dark desktop readout for E1's SUM result and a
375px light-theme worksheet with a long cell value: the readout stayed 327px wide
with an ellipsis, the worksheet scroll width stayed 375px, and the complete value
remained in the live region. The document stayed within the 720px viewport.
