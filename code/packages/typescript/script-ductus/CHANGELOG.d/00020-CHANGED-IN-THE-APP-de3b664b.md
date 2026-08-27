### Changed in the app
- `main.ts` and nine test files import from `@coding-adventures/script-ductus`.
- `vite.config.ts`'s `handwriting-tools` manual chunk matched
  `language-ladder/src/(strokes|ductusview|truetype).ts` by path. Repointed at
  the package — `check:bundle` caught it, and without the fix 7,600 lines of
  handwriting code would have moved into the interactive shell rather than
  loading when a learner opens a letter. `scriptdata` is deliberately NOT in that
  chunk: the shell needs `SCRIPTS` on first paint.
- `BUILD` chain-installs the new package before the app's own `npm install`.

