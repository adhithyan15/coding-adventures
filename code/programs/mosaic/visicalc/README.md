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
