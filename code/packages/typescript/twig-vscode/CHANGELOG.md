# Changelog

All notable changes to the Twig VS Code extension are
documented in this file.

## 0.1.2 - 2026-05-11

**LANG25 follow-up — Bundle deps + fix LSP blocking + enable gutter breakpoints.**

### Changes

- **Bundle all dependencies with esbuild.**  Changed `main` from
  `./out/extension.js` to `./out/bundle.js` and added an esbuild step to
  `npm run build`.  Without bundling, `require('vscode-languageclient/node')`
  failed at runtime because `node_modules` is not included in the VSIX.

- **LSP start is now fire-and-forget.**  `activate()` no longer `await`s
  `startLanguageClient()`, so a missing `twig-lsp-server` binary no longer
  blocks DAP registration.  Errors are logged as warnings instead.

- **Add `breakpoints` contribution to `package.json`.**  Without this entry
  VS Code's editor does not register gutter clicks or `Run → Toggle
  Breakpoint` as valid breakpoints for `.twig` files.  Adding `"breakpoints":
  [{"language": "twig"}]` to `contributes` enables the full gutter-click
  workflow.

## 0.1.1 - 2026-05-10

**LANG25-25A — Add unit tests; fix empty test suite CI failure.**

- Added `vitest.config.ts` with `passWithNoTests: true` so CI does not
  fail if the test suite is empty.
- Added `src/dap.test.ts` — two unit tests for `LANGUAGE_NAME` using a
  `vi.mock('vscode', ...)` stub so the module loads cleanly in Node.
- Added `npm test` step to both `BUILD` and `BUILD_windows` so CI
  actually runs the tests during the package build.
- Excluded `*.test.ts` files from the production `tsconfig.json` to
  prevent test code from being compiled into the extension bundle.

## 0.1.0 - Initial release

- Generated from `vscode-lang-extension-generator` (LS04).
- LSP integration via `twig-lsp-server`.
- DAP integration via `twig-dap`.

