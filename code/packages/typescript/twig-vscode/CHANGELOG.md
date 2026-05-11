# Changelog

All notable changes to the Twig VS Code extension are
documented in this file.

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

