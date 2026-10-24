### Patch electron, undici and the Engram vite and vitest locks

- `electron` moved from 43.5.0 to 43.8.0 in `engram-electron`, `journal-app`,
  `todo-app` and `visicalc-electron` (the `package.json` floor rose from
  `^43.5.0` to `^43.8.0` so a fresh install cannot land on a vulnerable
  release). That clears every Electron advisory fixed in 43.5.1, 43.7.0,
  43.7.3 and 43.7.6. The advisory fixed in 43.8.1 stays open: 43.8.1 was
  published on 2026-10-20, inside the seven-day release floor.
- `undici` moved to 6.29.1 and 7.30.1 (the nested copy under `@electron/get`)
  in the same four locks, lockfile-only.
- The two Engram locks (`code/programs/mosaic/engram-app/npm/web` and
  `npm/electron`) were regenerated with `build-web.sh --update-lock` and
  `build-electron.sh --update-lock` under npm 11, after the Mosaic emitter
  moved its pins from `vite` 7.3.6 to 7.3.7 and from `vitest` 3.2.7 to 4.1.12.
  That clears the vite, `vitest`, `@vitest/mocker`, `tinypool` and
  `source-map-js` alerts on the web lock; the Electron lock also picks up
  `undici` 7.30.1.
- No `vite` range in any `package.json` moved. Dependabot's #18060 and #18057
  raised `^6.0.1` to `^8.3.5`, which `@vitejs/plugin-react` 4.7.0 (peer
  `vite` up to 7) rejects, so `npm ci` failed with ERESOLVE.
- Skipped, and left open: `fast-uri` 3.1.9 and `happy-dom` 21.0.4 were
  published on 2026-10-19 and 2026-10-20, inside the seven-day floor
  (`happy-dom` 21 is also outside the blog's `^20.14.5` range, and 20.x has no
  fix). `http-cache-semantics` has no patched release.
- Diff shape: every `resolved` URL is `https://registry.npmjs.org/` with a
  sha512 integrity, and no package gained an install script. The four
  Electron-app locks change only `version`/`resolved`/`integrity` and the
  root `electron` spec. The Engram locks are fresh resolutions, so their tree
  follows vitest 4 (adds `obug`, `convert-source-map`, `@standard-schema/spec`;
  drops `vite-node`, `tinypool`, `tinyspy`, `cac`, `strip-literal`, `loupe`,
  `deep-eql`, `check-error`, `pathval`), and npm 11 records the Linux `libc`
  selectors the old npm 10 locks were missing.
