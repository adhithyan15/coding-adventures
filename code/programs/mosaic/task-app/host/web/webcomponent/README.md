# TaskApp Web Components parity host

This directory gates TaskApp's real package-expanded Mosaic sources through the
interactive `webcomponent` backend. It is intentionally a CI parity surface,
not a published TaskApp release artifact.

The light and dark pages use the generated `<mos-task-app>` Custom Element, the
same framework-neutral presentation controller as the React host, the real
`task-wasm` engine, and the existing IndexedDB-with-memory-fallback persistence
contract. Theme changes move between the two pages because Custom Element names
cannot be registered twice in one browsing context.

Generate and validate the host on Windows:

```powershell
../../../scripts/build-webcomponent.ps1
cd ..
npm install
npm run build:webcomponent
npx vitest run __tests__/webcomponent-host.test.ts
```

On Unix-like systems, use `../../../scripts/build-webcomponent.sh` instead. The
generated component/runtime files, WASM payload, and `dist-webcomponent/`
bundle are ignored; the source-of-truth inputs remain TaskApp's Mosaic package,
the shared host controller, and the two small HTML entry points here.
