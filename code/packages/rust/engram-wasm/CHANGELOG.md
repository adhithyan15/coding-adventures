# Changelog

## Unreleased

**Regenerated `pkg/engram_engine.wasm`, which was two months stale.** The
committed artifact dated from 2026-07-03 and still contained the old
`"handled by native hosts for WASM shells"` stub — it predated the change that
made Anki import and export work in the browser. A web build assembled from the
tree would have produced an app that loads, runs, and cannot import a deck.

Nothing failed, because nothing read it. The Rust tests compile the source; the
artifact is a committed binary with no compiler to complain. `js/smoke.mjs` does
exercise it, but its assertions were written against the delegation behaviour and
were never updated — and it was not run by CI at all. Two stale things agreeing
with each other reads exactly like a passing test.

**`js/smoke.mjs` now pins current behaviour** and is wired into `BUILD`, so it
runs. It asserts through the real wasm ABI that legacy APKG export succeeds and
returns bytes, that those bytes merge back in through the same ABI, and — so the
first two cannot be satisfied by an implementation that accepts anything — that
garbage bytes are still rejected.

Verified in the failing direction: restoring the previous artifact makes `BUILD`
fail with `apkg export succeeds in the browser build: got false, want true`.

This is also the first end-to-end evidence that browser APKG genuinely works
rather than merely compiling — the round trip runs in Node against the compiled
module, not against the Rust source.

## Unreleased

- Host-intent callback results with statuses such as `imported`, `exported`,
  `cancelled`, and `import-error` now merge visible Engram host-status props
  into Mosaic host responses.
- Added APKG byte-buffer exports to the WASM ABI and JS loader. Browser WASM
  builds return an explicit native-host delegation error, while Electron uses a
  native sidecar for real package import/export.
- Added `build-wasm.ps1` so Windows/PowerShell workspaces can build
  `pkg/engram_engine.wasm` with the same output layout as `build-wasm.sh`.
- `installEngramMosaicHost` now dispatches `mosaic-host-ready` after installing
  `window.mosaicHost`, matching the generated React/Electron refresh hook.

## 0.1.0

- Added the Engram linear-memory WASM ABI over `engram-core-wasm`.
- Added a dependency-free JavaScript loader that installs a Mosaic host adapter
  for generated React and Electron shells.
- Preserved generated app `hostIntent` responses and exposed an optional
  `onHostIntent` callback for browser open/edit, Anki import/export, and note
  workflow actions.
