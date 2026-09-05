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
the sheet frame contains horizontal overflow. This does not yet synchronize
scrolling with selection or supply sticky headers and row labels (#14277).

Browser review on 2026-09-05 covered both generated themes at desktop width
and the real Rust app in a 375px-wide preview. The narrow root's scroll width
remained 375px, and editing A1 from 15 to 20 recomputed E5 to 174. Text scaling,
loading/error/empty presentation, keyboard focus and native appearance remain
acceptance work under #14273 and #14278. Negative outline offsets exposed the
shared signed-dimension limitation tracked in #14327.
