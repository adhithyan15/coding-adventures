# journal-mosaic-app

Journal on the **standard Mosaic application ABI**: the Rust application every
generated Mosaic host (SwiftUI, Compose, Flutter, Qt, WinUI, web) loads as
`libmosaic_app` for the `journal-app` package. J3c-1 of
[#14416](https://github.com/adhithyan15/coding-adventures/issues/14416); design
in [`code/specs/journal-mosaic-app.md`](../../../specs/journal-mosaic-app.md).

## Where this sits in the stack

```
journal-core          pure engine: journals, entries, timeline, search…
  └ journal-mosaic-app  ← you are here: slots and events for the Mosaic package
      └ journal-app       the .mil/.mll/.msl UI (J3c-2): RecordList + DraftEditor
```

It is the same adapter shape as `task-mosaic-app` (Trestle) and
`engram-mosaic-app` (Engram): `props()` fills the package's slots from the
engine's state, and `dispatch` turns the package's events into engine commands.

## The contract

| props | events |
| --- | --- |
| `timeline-rows` (RecordList rows), `selected-key`, `timeline-empty`, `draft-title`, `draft-body`, `delete-label` | `onSelectEntry(index)`, `onNewEntry`, `onTitleChange(value)`, `onBodyChange(value)`, `onSaveEntry`, `onDeleteEntry`, `onCancelEdit` |

- **The editor holds a draft for a target** (a new entry, or an existing one).
  Typing only changes the draft, Save commits it, and Cancel reloads it.
- **Unknown events and bad payloads are errors** that leave the app unchanged.
- **Snapshots include an unsaved draft.** Restoring validates the journal and
  refuses anything it did not write.

## In the browser (J5c)

The same crate is the browser runtime. Built for `wasm32-unknown-unknown`, it
exports the standard Mosaic lifecycle bridge
(`mosaic_app_wasm::export_mosaic_wasm!`), which the web host loads with
`mosaic-host.mjs`.

- **The clock comes from the host.** wasm32 has no clock, so the module imports
  `journal.now_ms() -> f64`, and the host passes `Date.now`. The host must pass
  it through a shim that never throws, because an exception unwinding through
  wasm would leave the instance unusable. A value that isn't a writable time
  (NaN, negative, or past 9999-12-31) reads as the epoch.

  ```js
  const now_ms = () => { try { return Number(Date.now()); } catch { return NaN; } };
  const module = await loadMosaicModule(bytes, { journal: { now_ms } });
  ```

- **Bare event names work too.** The generated React component dispatches
  `selectEntry`, which is read as `onSelectEntry`.

## Testing

```sh
cargo test -p journal-mosaic-app
cargo build -p journal-mosaic-app --target wasm32-unknown-unknown
node --test js/wasm.test.mjs   # the real wasm through mosaic-host.mjs
```

On Compose, `conformance/compose/` holds two harnesses that run inside an
emitted project (`mosaic-compile pkg code/programs/mosaic/journal-app
--backend compose --emit-project --runtime-library <this crate's dylib>`, then
copy the file into `src/test/kotlin`):

- `JournalUiTest` drives the generated editor over this runtime: write, save,
  reopen from the timeline, delete. Run it twice on one
  `MOSAIC_APP_STATE_PATH`, the second time with `MOSAIC_EXPECT_RESTORED=1`,
  and the second launch must find the entry the first one kept. CI runs both
  launches in the Linux Compose lane.
- `JournalScreenshots` writes PNGs to `MOSAIC_SHOT_DIR` for a person to look
  at. It asserts only that rendering succeeds.
