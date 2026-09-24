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

## Testing

```sh
cargo test -p journal-mosaic-app
```
