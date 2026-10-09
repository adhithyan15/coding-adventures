# External dependency alert backlog

Status date: 2026-10-09. This was compiled during the Dependabot sweep that
closed every alert a safe dependency bump can close.

This backlog lists every third-party package that has ever raised a Dependabot
security alert in this repository, alerts already fixed included. A package
lands here because it has already cost us at least one security fix, and in a
repository whose point is to build things from scratch, that makes it a
candidate for removal. Once an alert is cleared, a later advisory against the
same package is just another bump. The lasting fix is to stop depending on it.

## How this list was built

The Dependabot alerts API is not readable from the sessions that do this work.
It returns 403, because the integration token has no `security_events` scope.
So the list is reconstructed from three sources that are readable:

1. **Dependabot's own history.** These are the 129 npm security-update PRs it
   has opened since 2026-03-23, plus the 11 human-authored fix PRs that cleared
   alerts in bulk: #524, #4947, #5693, #7145, #8108, #8702, #9804, #11876,
   #11920, #15839 and #15903. The github-actions version-update PRs are not
   alerts and are excluded.
2. **A local audit of every lockfile, run on the status date:**
   - `npm audit --package-lock-only` over all 510 `package-lock.json` files.
   - `cargo audit` over all 6 `Cargo.lock` files, against a fresh clone of the
     RustSec advisory-db.
   - pub.dev's advisory API for every hosted Dart package in the 16
     `pubspec.lock` files.
   - A version check of the Go (`golang.org/x/sys`, `golang.org/x/text`) and
     Hex (`jason`, `excoveralls`) dependencies.
3. **Advisory IDs** as npm's audit endpoint and the fix PRs report them.
   Dependabot's own PR bodies do not carry GHSA IDs. Where a cell says
   "unknown", the only record is a Dependabot bump with no advisory named.

**Every Dependabot alert this repository has ever had was in npm.** Rust, Go,
Dart and Elixir have no open advisories. The two RustSec *warnings* in the
Tauri lockfile are listed at the end, because Dependabot can surface RustSec
"unsound" advisories as alerts too.

## Status key

- **Cleared**: no lockfile in the repository resolves an affected version.
- **Open, no upstream fix**: every published version is affected. The only
  way out is removing the path that pulls the package in.
- **Open, cooldown**: a fixed version exists but is newer than a seven-day
  release-age floor that a lockfile in this repository enforces. Re-run the
  lock update once the floor passes.

## Summary table

"Lockfiles" counts the tracked `package-lock.json` files that resolve the
package on the status date.

| ID | Package | Advisories (known) | Worst | Lockfiles | Arrives via | Status |
|---|---|---|---|---:|---|---|
| EXT-001 | `vitest` | GHSA-5xrq-8626-4rwp, GHSA-82fw-gwwq-j7x9 | critical | 491 | direct devDependency | Cleared |
| EXT-002 | `@vitest/mocker` | GHSA-82fw-gwwq-j7x9 | moderate | 491 | vitest | Cleared |
| EXT-003 | `tinypool` | GHSA-5gmw-xhrv-c9v3, GHSA-85c8-ppgw-ccpr | critical | 0 | vitest 3.x | Cleared (vitest 4 dropped it) |
| EXT-004 | `vite` | unknown (Dependabot bumps 8.0.5 / 7.3.2 / 6.4.2) | — | 495 | direct, or via vitest | Cleared |
| EXT-005 | `esbuild` | GHSA-g7r4-m6w7-qqqr and others (#5693, #4947) | high | 23 | vite; direct in 3 packages | Cleared |
| EXT-006 | `postcss` | GHSA-r28c-9q8g-f849, GHSA-fxqj-rqcc-2cmp | high | 495 | vite | Cleared |
| EXT-007 | `nanoid` | GHSA-2v37-7h3g-55p8, GHSA-28wg-ghj8-5hjv | high | 495 | postcss | Cleared |
| EXT-008 | `source-map-js` | GHSA-68fv-2mgg-jv7q | high | 495 | postcss | Cleared |
| EXT-009 | `picomatch` | unknown (2.3.1→2.3.2, 4.0.3→4.0.4) | — | 501 | vite, tinyglobby, anymatch | Cleared |
| EXT-010 | `electron` | GHSA-hq2x-r82h-9wj4, -gr2m-v5gq-v685, -j84w-jfhq-vhvj, -9qh4-3jw8-366w, -qmv3-fv6v-rmhq, -r4w5-6pfg-jxp5, plus 18 earlier alerts (#4947) | high | 5 | direct (Electron apps, Engram) | Cleared |
| EXT-011 | `extract-zip` | GHSA-jmr9-qjv8-65gv | high | 0 | electron < 42.4 | Cleared (electron dropped it) |
| EXT-012 | `electron-builder` | (ancestor of EXT-013 to EXT-016) | high | 4 | direct devDependency | Cleared |
| EXT-013 | `app-builder-lib` | GHSA-7g7r-gx96-252g | high | 4 | electron-builder | Cleared |
| EXT-014 | `builder-util-runtime` | GHSA-p2f4-r6v6-j797 | high | 4 | electron-builder | Cleared |
| EXT-015 | `tar` | GHSA-23hp-3jrh-7fpw, -r292-9mhp-454m, -34x7-hfp2-rc4v, -83g3-92jg-28cx, -8qq5-rm4j-mr97, -8x88-c5mf-7j5w, -9ppj-qmqm-q256, -gvwx-54wh-qm9j, -qffp-2rhf-9h96, -r6q2-hw4h-h46w, -vmf3-w455-68vh, -w8wr-v893-vjvp | critical | 4 | electron-builder → @electron/rebuild, cacache | Cleared |
| EXT-016 | `@tootallnate/once` | unknown | — | 1 | electron-builder 25 | Cleared |
| EXT-017 | `@xmldom/xmldom` | unknown (0.8.11 → 0.8.15 across four PRs) | — | 4 | electron-builder (plist) | Cleared |
| EXT-018 | `lodash` | unknown (4.17.23 → 4.18.1) | — | 4 | electron-builder | Cleared |
| EXT-019 | `ip-address` | unknown | — | 0 | electron-builder (socks) | Cleared |
| EXT-020 | `tmp` | unknown | — | 4 | electron-builder | Cleared |
| EXT-021 | `undici` | GHSA-3wwx-pv8p-q78v, -r53p-7pc4-xj5r, -rfgv-xxqx-mfg5, -2gqq-gqf2-x968, -2jfj-6hjv-fm6j, -3xpg-4rpp-hhhm, -8436-99hf-9mmv, -pmjh-fq2x-6v4x, -rx4f-c7p8-82vq, -w293-vg96-wgc3 | high | 5 | @electron/get, node-gyp | Cleared |
| EXT-022 | `fast-uri` | GHSA-hrr3-gc8f-f4qj | moderate | 5 | electron-builder → ajv | Cleared |
| EXT-023 | `http-cache-semantics` | GHSA-ch52-4w7c-c8xp | high | 4 | electron-builder → make-fetch-happen | **Open, cooldown** (Engram electron lock only; 4.3.0 published 2026-10-04) |
| EXT-024 | `brace-expansion` | GHSA-3jxr-9vmj-r5cp, -mh99-v99m-4gvg, -rgw5-rvv9-x895, -6j4f-fj2g-mc7p, -qhr7-859c-m2p7, -q2hr-2g5m-vwhr | high | 9 | minimatch (electron-builder, jest) | Cleared |
| EXT-025 | `js-yaml` | GHSA-2883-xcg3-v3hh, GHSA-52cp-r559-cp3m, CVE-2026-59870 | high | 9 | jest (istanbul), electron-builder | Cleared |
| EXT-026 | `sprintf-js` | GHSA-hp3w-g68c-fv3c | moderate | 9 | jest → istanbul → js-yaml 3 → argparse 1; electron-builder → @electron/get → global-agent → roarr | **Open, no upstream fix** (latest 1.1.3 is affected) |
| EXT-027 | `global-agent` / `roarr` | (carry EXT-026) | moderate | 4 | @electron/get | **Open, no upstream fix** |
| EXT-028 | `jest` | (ancestor of EXT-029 to EXT-031) | high | 5 | direct devDependency (trig, wave, matrix, gradient-descent, loss-functions) | Cleared (moved to jest 30) |
| EXT-029 | `braces` | GHSA-vfj7-8cjw-p6xm | high | 0 | jest 29 → micromatch | Cleared (jest 30 dropped it; braces itself has no fix) |
| EXT-030 | `handlebars` | GHSA-8r5x-fm3f-whwj, -p8wg-vrv2-v86f, -xw65-4hp5-5hc7 | critical | 5 | jest → istanbul-reports | Cleared |
| EXT-031 | `@babel/core` | unknown GHSA (arbitrary file read via `sourceMappingURL`, #7145) | low | 8 | ts-jest / babel-jest | Cleared |
| EXT-032 | `browserslist` | unknown | — | 8 | @babel/core | Cleared |
| EXT-033 | `ws` | memory-exhaustion DoS (< 8.21.0) | high | 34 | jsdom | Cleared |
| EXT-034 | `form-data` | GHSA-hmw2-7cc7-3qxx | high | 30 | jsdom, axios-style clients | Cleared |
| EXT-035 | `concurrently` | (carries EXT-036) | critical | 1 | Mosaic Electron emitter | **Open, cooldown** (10.0.6 published 2026-10-08) |
| EXT-036 | `shell-quote` | GHSA-pqg4-j6r4-53mv | critical | 1 | concurrently 10.0.4 pins 1.9.0 exactly | **Open, cooldown** |

## Open items

### EXT-035 / EXT-036: `concurrently` → `shell-quote` (critical), Engram only

`code/programs/mosaic/engram-app/npm/electron/package-lock.json` is generated
by `scripts/build-electron.sh --update-lock`, which only resolves versions at
least seven days old (`scripts/npm-lock.sh`). `concurrently` 10.0.4 pins
`shell-quote` to exactly 1.9.0. The first `concurrently` with a fixed
`shell-quote` (1.12.0) is 10.0.6, published 2026-10-08.

- **Unblocked:** 2026-10-15.
- **Do then:**
  1. Change `"concurrently": "10.0.4"` to `"10.0.6"` in the Electron
     `package.json` template in
     `code/packages/rust/mosaic-package-artifact-builder/src/lib.rs`, and in the
     matching assertion in that file's `ui32_m_electron_project_shell_*` test.
  2. Re-run both `--update-lock` scripts.
  3. Run `cargo test --test npm_lockfiles` in `engram-app`.

  The same regeneration picks up `http-cache-semantics` 4.3.0 (EXT-023).
- **Exposure until then:** dev-only. `concurrently` is the `npm run dev`
  process runner and is never shipped in the packaged app. The advisory needs
  attacker-controlled input passed to `shell-quote.quote()`.
- **Removal option:** `dev` only needs "start vite, wait for the port, start
  electron, kill both on exit". That is a few lines of Node, or a
  `mosaic-dev` subcommand (see `code/programs/rust/mosaic-dev`), with no
  third-party dependency at all.

### EXT-026 / EXT-027: `sprintf-js` (moderate), 9 lockfiles, no upstream fix

Every published `sprintf-js` is affected, so no bump can clear this. It arrives
two ways:

- **jest → `babel-plugin-istanbul` → `@istanbuljs/load-nyc-config` → `js-yaml`
  3 → `argparse` 1 → `sprintf-js`.** This affects the five jest packages
  (`trig`, `wave`, `matrix`, `gradient-descent`, `loss-functions`).
  - **Action:** move these five to vitest, as the other ~490 TypeScript
    packages already are. That also removes `jest`, `ts-jest`, `babel-jest` and
    `@babel/core` from the repository entirely (EXT-028 to EXT-032). It is the
    single largest removal on this list.
- **electron-builder → `@electron/get` → `global-agent` → `roarr` →
  `sprintf-js`.** This affects the four Electron apps and Engram.
  - **Action:** covered by the Electron packaging item below.

### Rust (not Dependabot alerts today, listed so they are not a surprise)

`code/programs/typescript/forme-shell-desktop/src-tauri/Cargo.lock`:

- `glib` 0.18.5: RUSTSEC-2024-0429, unsound `Iterator` impls. Fixed in glib
  0.20, which needs Tauri's GTK bindings to move first.
- `proc-macro-error` 1.0.4: RUSTSEC-2024-0370, unmaintained.

Both come in through Tauri's Linux webview stack. Neither has a fix this repo
can apply on its own.

## Replacement backlog: where the alerts actually come from

Ordered by how many alerts each one removes. Every row in the summary table
traces back to one of these six direct dependencies. Each item below is
written as a candidate work package.

1. **vitest / vite / postcss / esbuild** (EXT-001 to EXT-009, ~495
   lockfiles). This family produced most of the repository's alert volume: the
   630-alert backlog that #15903 cleared was almost entirely here.
   - **Usage:** vitest is a test runner in every TypeScript package. vite and
     postcss come along for the ride, even in packages with no CSS and no dev
     server.
   - **Proposed:** an in-house TypeScript test runner (`describe` / `it` /
     `expect`, run on plain Node with type stripping). It would have zero
     transitive dependencies and would take vite, postcss, nanoid,
     source-map-js, picomatch and esbuild out of every library package at once.
   - **Builds on:** the repo already has `typescript/compiler-source-map`, so
     source-mapped stack traces need nothing new.
   - **Keep vite** only in the handful of packages that actually serve or
     bundle a web app.
2. **Electron + electron-builder** (EXT-010 to EXT-024, EXT-026/027;
   `engram-electron`, `journal-app`, `todo-app`, `visicalc-electron`, Engram).
   - **Why first in impact:** this is the deepest tree in the repository, with
     several hundred transitive packages, and the source of every critical
     `tar` alert.
   - **Proposed:** retire Electron from these apps in favour of the in-house
     Rust desktop hosts the Mosaic work is already building. Failing that,
     replace electron-builder with a small packaging script. Engram's
     `build-electron.sh` already verifies the Electron runtime itself; the
     remaining packaging is producing a zip, AppImage or portable exe, and the
     repo has `typescript/zip` and `typescript/deflate`.
3. **jest toolchain** (EXT-028 to EXT-032, 5 packages).
   - **Proposed:** migrate the five packages to the repo-standard runner
     (vitest today, or item 1 once it exists). This is the cheapest item on the
     list.
4. **jsdom** (EXT-033 `ws`, EXT-034 `form-data`; ~34 lockfiles).
   - **Usage:** jsdom pulls in a WebSocket client and a multipart encoder that
     the DOM tests never use.
   - **Proposed:** an in-house minimal DOM for component tests. The repo
     already has `typescript/xml-lexer`, `css-lexer` and `css-parser`.
5. **concurrently** (EXT-035/036).
   - **Proposed:** replace with an in-house process runner (see the open item
     above).
6. **js-yaml** (EXT-025, via istanbul and electron-builder).
   - **Proposed:** disappears with items 2 and 3. If a YAML need remains, it is
     a natural grammar-driven package for this repo's lexer and parser
     generator.

## Keeping this file current

When a new Dependabot alert, or an `npm audit` finding on a lockfile, names a
package:

1. If it is not already listed, add a row.
2. If it is listed, add the new advisory ID to its row.
3. Record whether the fix was a bump or a removal.

A package leaves the summary table only when no tracked lockfile resolves it at
all. Clearing an alert is not enough to remove it.
