- **Runs in the browser (J5c-2).** `host/web` is the web host: the generated
  `JournalApp` over the wasm runtime, persisted in `localStorage`.
  `scripts/build-web.sh` emits the React components and the wasm, and the new
  `journal-mosaic-web.yml` workflow tests it.
