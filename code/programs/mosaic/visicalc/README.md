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
Physical scrolling across the entire workbook, row labels, native capacity and
native reveal remain under #14277 and #14372.

Browser review on 2026-09-05 covered both generated themes at desktop width
and the real Rust app in a 375px-wide preview. The narrow root's scroll width
remained 375px, and editing A1 from 15 to 20 recomputed E5 to 174. Text scaling,
loading/error/empty presentation, keyboard focus and native appearance remain
acceptance work under #14273 and #14278. Negative outline offsets exposed the
shared signed-dimension limitation tracked in #14327.
