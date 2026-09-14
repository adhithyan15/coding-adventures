---
category: Mosaic compiler pipeline
---

# `mosaic-compile pkg --output X` treats `X` as a DIRECTORY and writes `X/react/<Component>.tsx` (+ `index.ts`, `.lattice`) — passing a file path silently creates a directory literally named `Foo.tsx`

This is true with *and* without `--emit-project`; the flag only adds the Vite shell side-files (`package.json`, `vite.config.ts`, `index.html`, `README.md`, `src/main.tsx`) next to the component. So `--output "$WEB/src/TaskApp.tsx"` produced `.../src/TaskApp.tsx/react/TaskApp.tsx`. To land a single component in a hand-written host: emit to a scratch dir, copy `<scratch>/react/<Component>.tsx` to its destination, and delete the scratch dir. The emitted component is self-contained (imports only `react`, exports `<Component>` + `<Component>Event`), so copying just that one file is sufficient.
