## Unreleased — handwriting moves into a package

`strokes.ts`, `truetype.ts`, `ductusview.ts` and `data.ts` now live in
`@coding-adventures/script-ductus`, so the book pipeline can build the same
filmstrips as printed figures. Nothing under `code/packages/` may depend on
something under `code/programs/`, which is why they had to move for anything but
this app to use them.

No behaviour change here. `main.ts` and nine test files import from the package;
`types.ts` re-exports `Letter` and `ScriptData` from it, so every existing
`from "./types.ts"` import is unchanged.

One real catch, from `check:bundle`: `vite.config.ts`'s `handwriting-tools`
manual chunk matched those modules **by path**, so moving them silently emptied
the chunk and would have pulled 7,600 lines of handwriting code into the
interactive shell instead of loading it when a learner opens a letter. Repointed
at the package. `scriptdata` stays out of that chunk on purpose — the shell needs
`SCRIPTS` on first paint.

