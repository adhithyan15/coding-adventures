---
category: Supply chain & CI pinning
---

# npx --no-install finds a pinned package only from inside the project that installed it

When `build-electron.sh` moved its packaging tools from `npx --yes` to exact
devDependencies run with `npx --no-install`, the macOS release lane failed with
`npx canceled due to missing packages and no YES option: ["@electron/asar@4.3.1"]`.
The electron-builder call worked; the asar check did not.

The difference was the working directory. electron-builder ran inside
`( cd "$APP" && ... )`; the asar check ran from the repo root. npx looks for a
package in the CURRENT directory's node_modules. From anywhere else it resolves
the spec against the registry (latest: 4.3.1, not the pinned 4.3.0) and, with
`--no-install`, refuses. Under the old `--yes` it simply downloaded the latest,
which is exactly the unpinned fetch the change meant to remove, so the bug was
invisible until the guard was added.

Local testing missed it because the check was run from inside the project.

Do: run every `npx --no-install` (or `npm exec --no`) from the directory whose
node_modules holds the pinned tool, e.g. `(cd "$APP" && npx --no-install ...)`,
and test the command from the same working directory the script uses.
