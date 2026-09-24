# Changelog

## 0.1.0 — unreleased

- **Runs in the browser (J5c-2).** `host/web` is the web host: the generated
  `JournalApp` over the wasm runtime, persisted in `localStorage`.
  `scripts/build-web.sh` emits the React components and the wasm, and the new
  `journal-mosaic-web.yml` workflow tests it.
- **Launches on Qt (J5a).** CI's Qt lane builds this package with the strict
  standard binding against `journal-mosaic-app`, installs it and launches it
  twice offscreen, restoring the second time. It also runs this package's
  tests, which nothing ran before, because the package opts out of the Rust
  workspace.
- `JournalApp` (J3c-2, #14416): a timeline (`RecordList` with an `EmptyState`)
  and a **New entry** button beside a `DraftEditor`, in a `HostNavigationSplit`.
- `JournalApp.mil` is exactly the slot/emit contract `journal-mosaic-app`
  implements. The tests check it both ways: props equal slots, and every emit
  is routed.
- Light and dark styles for the app's own parts, using only properties every
  native backend lowers. The shell's background was left out because Flutter
  does not lower it on the split view.
- A native-complete gate. It pins the Flutter `border-radius`, `color` and
  `font-size` drops inherited from DraftEditor, EmptyState and RecordList.
