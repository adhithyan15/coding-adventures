# VisiCalc presentation contract

`budget-v1.json` is the shared workbook seed. The React host imports it directly;
the Rust MosaicApp adapter consumes the same source values. Expected totals
in `presentation-contract-v1.json` are explicit independent assertions.

The contract uses Mosaic runtime event envelopes (`type` and `payload`) and
zero-based **absolute workbook** coordinates. A generated Grid may emit an index
relative to its row slice; the host boundary must translate it before dispatch.
An `editStart` targets the selected cell. `formulaChange` buffers text without
mutating the workbook. `commit` preserves selection; `editCommit` moves down one
row. Cancel discards the buffer. Navigation cancels an active edit.

Every step declares selection, row-window offset/size, formula-bar text and edit
state. `engine` is a diagnostic projection mapping addresses to `[raw, display]`.
It checks committed source independently of edit-buffer text and checks formula
recalculation independently of the visible controls. It is not a full workbook
serialization or snapshot format. All expected values are assertions, not values
to inject into the application.

The React replay drives actual generated controls with clicks and keyboard/input
events, captures the actual Rust/WASM workbook, checks each declared expectation,
and compares the complete rendered slice with the engine's computed window.
Unknown events fail the replay. Run `npm test` from
`code/programs/typescript/visicalc`; the Linux/Windows VisiCalc workflow executes
this replay and the production build, which also type-checks the tests.

`cargo test -p visicalc-mosaic-app` from `code/packages/rust` replays this same
fixture against the standard Rust adapter and spreadsheet-core. The adapter's
additional tests cover snapshot/restore, atomic errors, resizing and the native
C ABI. Both replays run in the VisiCalc Linux/Windows workflow. Generated native
controls and the standard web host still need integration and UI acceptance.

Version 1 establishes the working edit/navigation baseline for the adapter
migration. It does **not** certify physical scrolling, responsive viewport sizing,
save/reopen, native UI acceptance, accessibility or visual design. Those remain
required in #14270, #14277, #14278 and #14279 and must gain executable scenarios
as their shared implementations land. The fixed 30-row slice may exceed a short
window. Do not count missing scenarios as passing or mark #14270 complete on the
basis of this baseline alone.
