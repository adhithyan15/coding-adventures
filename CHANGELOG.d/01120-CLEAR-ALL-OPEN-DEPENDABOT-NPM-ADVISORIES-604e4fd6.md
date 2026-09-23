### Clear all open Dependabot npm advisories

- Cleared every open Dependabot alert in the repository — roughly 600, all of
  them npm, spanning eight distinct advisories across 490 `package-lock.json`
  files. Final state: all 490 lockfiles report `found 0 vulnerabilities`.
- Fixed `GHSA-82fw-gwwq-j7x9` (vitest / `@vitest/mocker` path traversal, 453
  manifests), `GHSA-2v37-7h3g-55p8` and `GHSA-28wg-ghj8-5hjv` (`nanoid`, 441
  and 119), `GHSA-fxqj-rqcc-2cmp` and `GHSA-r28c-9q8g-f849` (`postcss`, 120 and
  119), `GHSA-2883-xcg3-v3hh` (`js-yaml`, 9), `GHSA-mh99-v99m-4gvg` and
  `GHSA-rgw5-rvv9-x895` (`brace-expansion`, 5), `GHSA-hmw2-7cc7-3qxx`
  (`form-data`), `GHSA-g7r4-m6w7-qqqr` (`esbuild`) and `GHSA-5xrq-8626-4rwp`
  (vitest, critical, one manifest).
- Raised the `vitest` and `@vitest/coverage-v8` floor from `^4.1.0` to
  `^4.1.11` in 480 `package.json` files. The advisory's fix lands in 4.1.11 and
  the two packages peer-depend on each other at an exact version, so neither
  moves unless the floor moves; `npm audit fix` and `npm update` are both
  no-ops against that cycle.
- Moved `visicalc` and `chief-of-staff-channel-epoch-activation` from vitest
  3.x to `^4.1.11`. `GHSA-82fw-gwwq-j7x9` has no fix in the 3.x line, so this
  one is a major bump rather than a floor raise.
- Moved `spice-netlist-parser` from `esbuild ^0.27.0` to `^0.28.1`. This one is
  hygiene rather than a live fix: against the current base the old range had
  resolved to 0.27.7, already above the advisory's `>=0.27.3` floor. Raising
  the floor keeps a future resolution inside the range from drifting back
  below it. (An earlier draft said the range had settled on 0.27.2 and was
  therefore vulnerable — that was measured against an older base and is
  wrong.)
- Audited the other four ecosystems and found nothing: 5 `Cargo.lock` against
  the RustSec advisory database, the two `golang.org/x/*` modules that are the
  repo's only third-party Go dependencies, and `jason` / `excoveralls` in Hex.
  Every alert was npm.
- Verified the change shape rather than trusting the audit summary. Across the
  485 changed lockfiles, 2972 removed and 1103 added package keys are
  `optional: true` platform binaries — vite minor-bumping to pick up a fixed
  `postcss` changed rolldown's set of prebuilt bindings. The `linux-x64`
  binding CI actually uses is retained in every lockfile that has rolldown.
  Non-optional movement is 74 removals and 22 additions across 4 files. The 74
  removals sit entirely in the two packages taking the vitest 3 -> 4 bump and
  are vitest-3.x test-tooling transitives (`vite-node`, `tinypool`, `loupe`,
  `pathval`, `deep-eql`, `strip-literal`, `cac`, the glob/jackspeak/path-scurry
  chain), dropped because vitest 4 moved to rolldown-vite and vendored its
  assertion stack. None is imported by source or declared in any
  `package.json`.
- Checked the 485 lockfiles mechanically rather than by sampling, since a
  lockfile diff is too large to read: all 49,268 `resolved` registry URLs are
  https to `registry.npmjs.org` and every one carries a well-formed sha512
  integrity hash; the remaining 1299 entries are local `file:` links to sibling
  packages, byte-identical to before. Zero package names new to the repository,
  zero `name@version` whose integrity hash changed, and zero new
  `hasInstallScript` packages.
