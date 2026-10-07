# Mosaic — product and bug backlog

**Status:** active
**Last triaged:** 2026-10-07, against `origin/main` at `84252d038a`
**Scope:** the Mosaic UI compiler's backends and toolkit, MosaicBook, and the
products built on Mosaic: Venture, SpiceWorkbench, VisiCalc, Trestle
(TaskApp), Journal and Engram.

## Why this exists

Mosaic work had been tracked per app (`task-app/BACKLOG.md`,
`venture-browser/ACCEPTANCE-BACKLOG.md`), in epics, and in about 85 GitHub
issues, with no single place to see what was still open. Several products
drifted out of view. MosaicBook went three weeks without a commit, and
SpiceWorkbench has no program that hosts it. Open bugs piled up, and many
issues stayed open after they were fixed.

On 2026-10-07 every open Mosaic-related issue was read in full, with all its
comments, and checked against `origin/main`. That covered each product's
issues, every checklist item in the epics, the specs' open-items lists, CI
history, and local test runs. This file is the result: one ordered list.

**How to use it.**
- Take work from the top of [Order of work](#order-of-work).
- When an item lands, mark it **done** with the PR number, and close its
  issue.
- When an issue is found, add a row here and give it a GitHub issue.
- Re-triage when the date above is more than a month old.

**Columns.**
- *Status:* **real** means verified still broken or missing on the audit
  base, with the evidence shown; **partly** means some of it is done.
- *Kind:* bug (wrong behaviour), feature, docs, decision or CI.
- *Size:* S is under a day, M a few days, L a week or more.

## Order of work

1. **Security and visible breakage.**
   - EM-1, the XAML `{` literal (*fixed, unpushed*);
   - MB-14, MosaicBook listened on every interface (*fixed, unpushed*);
   - MB-1, apps cannot be previewed (*fixed, unpushed*);
   - SW-2, SpiceWorkbench does not compile on four native backends;
   - SW-1, `Path` on Web Components (*fixed, unpushed*);
   - VC-1, VisiCalc's native Open/Save.
2. **Small bugs, batched by package.**
   - MosaicBook: MB-2, MB-3 and MB-4;
   - emitters: EM-3, EM-5 and EM-9;
   - X-2, the compiler's search path on Windows.
3. **Housekeeping.** Close the issues in [Closable now](#closable-now),
   update the out-of-date epic checklists, and refresh the docs rows (MB-12,
   SW-5, VE-3).
4. **Releases that never shipped.**
   - VE-2, Venture's release pipeline, and cutting 0.10.0;
   - JO-1, Journal's first release.
5. **Emitter correctness.**
   - EM-2, `align`: land #16297, then #16293;
   - EM-4, SwiftUI layout;
   - TR-2 and TR-3, the native progress ring and elevation;
   - EM-6, Compose fixtures (also unblocks MB-5).
6. **Product features**, smallest high-value first: MB-6, VE-1, SW-3, TR-5,
   EN-1 and the remaining VisiCalc native work.
7. **Decisions** in [Decisions needed](#decisions-needed), which unblock
   EN-2, EN-3, JO-3 and PR-4.

## Cross-cutting

| ID | Problem | Status and evidence | Kind | Size |
|---|---|---|---|---|
| X-1 | `ci.yml` on main almost never completes: each push cancels the run before it (`cancel-in-progress`) | **real**. Every one of the last 300 completed `ci.yml` runs on main was cancelled. The last success was on 2026-07-21. The last full build (run 36528879089, 2026-09-29) failed all 15 shards. Nobody can tell from main whether any product is green | CI | M |
| X-2 | `mosaic-compile` splits `--package-search-path` on `:` on every OS | **real**. `mosaic-compile/src/main.rs:755` uses `split(':')`, so a Windows drive path (`C:\…`) breaks in two. MosaicBook's sibling search path is absolute, so it is affected on Windows. Fix: `std::env::split_paths`, plus OS-specific joining in MosaicBook. #16931 | bug | S |
| X-3 | Native backends drop authored style properties silently or with a warning only | **partly**. Every backend reports drops (#15532, `mosaic-package-artifact-builder/src/lib.rs:2757-2817`), but the gate fails only on capability degradations. TaskApp's ratchet (`code/scripts/taskapp_native_control_contract.py:22`) allows 741 drops: XAML 77, SwiftUI 142, Compose 89, Qt 193, Flutter 240. Tracked by #12022 | bug | L |
| X-4 | Style values that start with `{` pass into XAML verbatim, as markup extensions | **real, by design**. `translate_xaml_value` (`mosaic-emit-xaml/src/pipeline.rs:~2698, ~2918`). Harmless while every `.msl` is first-party. See [Decisions needed](#decisions-needed) | decision | S |
| X-5 | Dependency style axes are not scoped to the call site | **real**. Only XAML guards against it (`xaml/pipeline.rs:1823-1834`, #14482). #14481 | bug | L |

## MosaicBook (`code/programs/go/mosaicbook-server`)

It works, but nobody has worked on it lately. The last server commit was
#15442 on 2026-09-17. `mosaicbook.yml` is green on main, and `--check` over
`code/packages/mosaic` compiles 61 components and 117 stories (468
compilations, 4 recorded degradations) in about 2 seconds. Native backends
get analysis only (the drop panel); nothing renders natively. Spec: UI19.

| ID | Problem | Status and evidence | Kind | Size |
|---|---|---|---|---|
| MB-1 | App components could not find dependencies outside their own directory, so VisiCalc, TaskApp, EngramApp and JournalApp failed on every backend | **fixed** on `claude/brave-ride-edfrqw` (c615a9499e, unpushed): `--package-search-path` | bug | S |
| MB-14 | The server listened on every network interface, not just localhost as its README says. Another machine on the network could fetch previews by sending `Host: localhost` | **fixed** on `claude/brave-ride-edfrqw` (unpushed): it binds `127.0.0.1` | security | S |
| MB-2 | The browser shell never shows a component's `storiesError` | **real**. `static/index.html` has no reference to it. UI19 §6.2 asks for a ⚠️ badge. #16928 | bug | S |
| MB-3 | The watcher reloads once without a change at startup, and re-runs discovery while holding the server lock | **real**. Its file-time snapshot starts empty, so "detected file change" appears 1 second after start. Discovery runs `--describe` subprocesses under `s.mu` (`watcher.go`). #16929 | bug | S |
| MB-4 | Compose ignores `--fixtures` and `--emit-project`, even with `--strict-fixtures` (output byte-identical, exit 0) | **real**. This breaks the `--strict-fixtures` contract in `mosaic-compile.json`. Fail loudly first; the fix is EM-6. #16930 | bug | S then M |
| MB-5 | Surface's `$mosaic-child-slot` cannot compile in isolation on html, webcomponent or react | **real**. Reproduced: "not yet supported by the pipeline HTML emitter". 3 recorded degradations. #14685 | bug | M |
| MB-6 | Form-factor layouts (`*.desktop.mll`, `*.touch.mll`) are never discovered | **real**. VisiCalc's Grid and FormulaBar and EngramApp's touch layout are invisible ("2 candidate(s) skipped") | feature | M |
| MB-7 | App components have no stories, so `--check` cannot cover them, and CI's check scans only `code/packages/mosaic` | **real**. VisiCalc, VisiCalcStartup, VentureChrome and others report "missing explicit .stories.json file" | feature | M |
| MB-8 | Story depth: 50 of 61 components have one empty Default story, with no dark-theme coverage (the server always prefers `*.light.msl`) | **real**. #14017 | feature | L |
| MB-9 | Native PNG snapshots, pixel baselines, and a message when a toolchain is missing | **partly**: Paint shipped (#15055). #14013, which absorbs #12027 part 2 | feature | L |
| MB-10 | Per-component demo app on every platform | **real**, not started. #14015 | feature | L |
| MB-11 | UI19 features not built: fixture editor (§6.3), `?fixtures=`, `width` and `height`, tiled mode, the `T` and `E` shortcuts, `/api/render`, `.mosaicbook/config.json`, render daemons (§7), Electron (§8) | **real** | feature | M–L |
| MB-12 | Out-of-date docs | **real**:<br>• UI19 says "Specification", WebSocket (the server uses SSE), Cairo/Skia and `.mosaic` files, and never mentions `--check`, the degradation file, Paint, three-file stories or fixture validation.<br>• `mosaic-component-program-v1.md` §2 says stories are impossible and MosaicBook is not in CI (#14436 and #14688 made both untrue).<br>• The README describes `.mosaic` files and the reload triggers wrongly.<br>• #15045 names `--check-stories` instead of `--check`. | docs | S |
| MB-13 | Build MosaicBook's own UI on Mosaic | backlog. Its prerequisites, MB-8 and MB-9, are not met. #14028 | feature | L |

## Venture (`code/programs/mosaic/venture-browser`)

The feature work is current, but no release has gone out since 0.9.1.
`ACCEPTANCE-BACKLOG.md` has 82 items checked and none open. The last
feature, #16178, merged on 2026-10-06 with CI green on three operating
systems. `cargo test` passes 33/33, and `scripts/build-all.sh --emit-only`
emits all nine backend projects. The browser roadmap is BR02 and BR03.

| ID | Problem | Status and evidence | Kind | Size |
|---|---|---|---|---|
| VE-1 | No web build and no continuous publishing | **real**:<br>• `host/web` and `host/react` hold only tests.<br>• VentureChrome has no stories (MB-7).<br>• `release-venture.yml` runs only on `venture-v*` tags.<br>#14494 | feature | M |
| VE-2 | The release pipeline has never succeeded end to end | **real**:<br>• All four `release-venture` runs failed at "Publish prerelease".<br>• Their fix, #13683, merged after the last run and has never been exercised.<br>• `upload-artifact` is still pinned to v4.6.2.<br>Exercise it with a prerelease tag, then cut 0.10.0 (about 30 features unreleased since 0.9.1, 2026-08-31) | CI | S |
| VE-3 | The changelog is out of order | **real**. `CHANGELOG.md:1-50` sits above `## [Unreleased]` (line 51) | docs | S |
| VE-4 | The hand-written hosts replace the generated ones, so a change to a generated host breaks Venture | **real, has happened twice**: #16054 (Flutter), #16387 (Qt). Not failing today | bug risk | M |
| VE-5 | The tree-builder rewrite stalled on 2026-09-26 | **real**:<br>• BR03 step 5 (errors by code and position) is not done.<br>• Step 6 (switch `html-parser` to the new crate) is not done.<br>• The repair passes are still in `html-parser/src/lib.rs:12146, 12311`, and `normalize_document_shell` at `:12884`. | feature | L |
| VE-6 | BR02 phases P3–P10: encoding detection, HTTPS (no `tls-platform`), Linux text measurement, PNG and SVG, full CSS, JavaScript | **real**, roadmap. P1 is done (#16010, #16017, #16031, #16036) | feature | L each |
| VE-7 | Venture on iOS and Android (UI89 §5 step 7) | **real**, blocked on BR02 host work | feature | L |
| VE-8 | Emitter drops in Venture's generated projects: Qt 44, Flutter 24, SwiftUI 19. Compose and XAML drop `overflow:auto` on `view-source-content` | shared emitter limits (X-3) | bug | M |

## SpiceWorkbench (`code/packages/mosaic/mosaic-pkg-spice-workbench`)

Both libraries pass their tests: the package 3/3, and the Rust adapter
`spice-mosaic-app` 50/50. The XAML compile test passes too. But no program
hosts the workbench, it renders natively only on WinUI, and the last change
was on 2026-09-20.

| ID | Problem | Status and evidence | Kind | Size |
|---|---|---|---|---|
| SW-1 | The Web Component emitter rejects `Path` | **fixed** on `claude/brave-ride-edfrqw` (unpushed); MosaicBook's waiver removed. Was: It falls through to `UnknownPrimitive` (`mosaic-emit-webcomponent/src/pipeline.rs:3047`), and is waived in `mosaicbook-degradations.json`. Port from `mosaic-emit-html/src/pipeline.rs:1785`, then remove the waiver. #14686 | bug | S–M |
| SW-2 | Does not compile at all for SwiftUI, Compose, Flutter or Qt | **real, reproduced**:<br>• SwiftUI: "primitive 'Path' is not yet supported".<br>• Compose, Flutter and Qt: "Path prop 'x1' is bound … only supports a literal number".<br>Nothing records it, because MosaicBook gates only browser backends. XAML's #14682 is the template for the bound line geometry. #16926 | bug | M–L |
| SW-3 | No app shell or deploy; only package artifacts and the adapter's WASM test | **real** | feature | L |
| SW-4 | Collapse-never pinned split: no runtime proof | **real**. `SpiceWorkbench.mll` has no `HostNavigationSplit`, and there is no runnable app to resize. #15699 (parent #15481) | feature | M |
| SW-5 | No CHANGELOG, and the docs are out of date | **real**:<br>• It is the only `code/packages/mosaic` package without a CHANGELOG.<br>• The README calls data-bound `Path` "a native XAML follow-up"; #14682 already did it.<br>• `spice-full-implementation-plan.md` marks the WASM lifecycle "(in progress)", but #15795 landed it. | docs | S |

## VisiCalc (`code/programs/mosaic/visicalc`)

Active: draft #16864 (native focus restore and latency profiling) is open,
and a week of WinUI fixes has landed (#16699–#16750). `visicalc.yml` (web,
WASM, Rust, typography) is green on main. No CI job builds a generated
native app.

| ID | Problem | Status and evidence | Kind | Size |
|---|---|---|---|---|
| VC-1 | Open and Save cannot work on native hosts | **real**. `visicalc-mosaic-app/src/lib.rs:293` sends `file.open`/`file.save`, but native platform libraries answer only `files.open`/`files.save` (`templates/xaml/MosaicPlatformEffects.cs:153`). UI87 keeps the old names on the browser only "until VisiCalc migrates". The accept list needs a type whose extension is `.visicalc`. Blocks #14548, #14279 and #14276. Coordinate with #16864. #16927 | bug | S–M |
| VC-2 | Native selection latency of 242–469 ms | **real**, measured in #16864. #16865 | bug | M |
| VC-3 | After a click, Right arrow does not move the selection, and Tab does not enter the grid | **real** (2026-10-07 comment). #14278 | bug | M |
| VC-4 | XAML grid leftovers: formula field width, numeric right alignment | **partly**. #14274 | bug | S–M |
| VC-5 | Packaging bugs in the older Windows hosts | **real**:<br>• Deno uses `new URL(...).pathname` (`main-ffi.ts:27`).<br>• Qt hard-links `Vendor/libspreadsheet_capi.a` (`CMakeLists.txt:54`).<br>• Electron loads `../visicalc/dist` from outside its bundle (`main.js:42`).<br>Fix them, or retire those hosts. #14281 | bug | M |
| VC-6 | Gate the generated WinUI app in CI, including relaunch and persistence | **real**. #14276 | CI | M |
| VC-7 | Signed mosstyle dimensions | **real**. `DIMENSION` is unsigned (`mosstyle.tokens:30`). #14327 | feature | S |
| VC-8 | Several sheets, undo, clipboard | **real**. `lib.rs:564` rejects more than one sheet, and `VisiCalc.mll:32` hard-codes "Sheet 1". #14279 | feature | L |
| VC-9 | Native viewport row capacity, row headers and scrolling | **real** on the native backends:<br>• row capacity: #14372;<br>• row headers on Qt, Flutter and SwiftUI: #14388;<br>• scrolling: #14277. | feature | M each |
| VC-10 | Native acceptance, shared fixture replay, polished design, native releases | **real**. #14280, #14270, #14273, #14282 | feature | L |

## Trestle / TaskApp (`code/programs/mosaic/task-app`)

The P0 and most P1 items are done, with releases v0.1.0 through v0.5.1. The
work left is native visual quality. The checklist in #13517 is out of date:
#13526 is closed but still unchecked.

| ID | Problem | Status and evidence | Kind | Size |
|---|---|---|---|---|
| TR-1 | Dropped styles should fail the gate | **partly**. See X-3 and #12022 | bug | L |
| TR-2 | The progress ring renders as a grey box on native backends | **real**:<br>• `TaskApp.mll:78-81` still uses the CSS donut.<br>• `HostProgressRing` lowers on XAML, Qt, Compose and Flutter, but not SwiftUI.<br>#13176 | bug | M |
| TR-3 | Elevation on SwiftUI; 9 `box-shadow` declarations left in `TaskApp.light.msl` | **partly**. #12028 | feature | M |
| TR-4 | SwiftUI has no `Path` | **real**. The old blocker is gone: CI runs `swift build` on macOS (`ci.yml:2810`). #13206 | feature | M |
| TR-5 | The Flutter topbar overflows below 1600 px: 175 px at 1280 wide, 655 px at 800 | **real**. The conformance test still runs at 2400×1600 (`task-mosaic-app/conformance/flutter/widget_test.dart:81`). Needs a design call (wrap or flex). #13465 | bug | M |
| TR-6 | Signing and notarization: no per-platform decision recorded | **real**. #13977 | decision | S |

## Journal (`code/programs/mosaic/journal-app`)

J1–J3 are done. J4 is mostly done, and J5's CI lanes and the `/journal/`
deploy are in. All five checkboxes in #14416 are still unchecked.

| ID | Problem | Status and evidence | Kind | Size |
|---|---|---|---|---|
| JO-1 | No first release, and `release-journal.yml` still packages the old Electron app in `code/programs/typescript/journal-app` | **real**. No `journal-v*` tag exists. Release from the Mosaic app, then retire the Electron app | feature | M |
| JO-2 | Export works on the web only | **real**. #16034 | feature | M |
| JO-3 | Photos (needs UI88), encryption, calendar navigation, import | **real** | feature | L |

## Engram (`code/programs/mosaic/engram-app`)

Every item on the epic (#13624) is done, and tags v0.3.0 through v0.4.0
exist.

| ID | Problem | Status and evidence | Kind | Size |
|---|---|---|---|---|
| EN-1 | `[sound:]` audio is not played during review | **real**. References are only collected (`media.rs:77`). #13937 | feature | M |
| EN-2 | LaTeX and MathJax in cards | **real**, blocked on UI88. #13936 | feature | L |
| EN-3 | Image occlusion notes | **real**, blocked on UI88. #13938 | feature | L |
| EN-4 | Engram is not in the XAML PR lane (`mosaic_xaml_windows_ci_acceptance.py`), and `mosaic-pkg-card` has only 5 tests | **real** | CI | S |
| EN-5 | Out-of-date `golden-v11` naming, and `engram-anki-parity.md:120` | **real**, after #13942 and #14134 | docs | S |

## Emitters and toolkit (`code/packages/rust/mosaic-emit-*`, `code/packages/mosaic`)

| ID | Problem | Status and evidence | Kind | Size |
|---|---|---|---|---|
| EM-1 | An authored XAML literal starting with `{` became a markup extension | **fixed** on `claude/brave-ride-edfrqw` (32cd2311a9, unpushed). #15487 | security | S |
| EM-2 | `align` is never read on Qt and Flutter; main-axis and Text `align` semantics are undefined | **real**. #15258 has an open fix, #16297. #16293 needs a design | bug | S/M + M |
| EM-3 | `HostInput` ignores `multiline` on react, html, webcomponent, Qt and SwiftUI, with no degradation reported | **real**. A probe emits `<input type="text">`, a Qt `TextInput` and a SwiftUI `TextField`. #15930 | bug | S–M |
| EM-4 | SwiftUI drops `align`, `align-items`, `justify-content`, `flex-grow` and `flex-wrap` | **partly**. Pinned at `engram-app/tests/native_complete_gate.rs:154-161`. #14728 | bug | M |
| EM-5 | html emits state hooks (`data-disabled`) with no CSS to consume them | **partly**. #14824 | bug | S–M |
| EM-6 | Compose pipeline mode has no `EmitOptions` or project shell, and ignores `--fixtures` | **real**. `compose/pipeline.rs:69`, `main.rs:1392-1411`. #14704 | feature | M |
| EM-7 | Permissive XAML shells bind the host by reflection, so mismatches fail silently | **partly**: native-complete shells bind directly. `xaml/pipeline.rs:9047-9192`. #14049 | bug | M |
| EM-8 | `HostButton` children are dropped on 6 of 8 backends (now reported as a degradation) | **real** as a feature. #15921 | feature | L |
| EM-9 | webcomponent emits `align: center-vertical` as a raw, invalid inline CSS declaration | **real**. #16932 | bug | S |
| EM-10 | Compose `Col(width:)` is emitted as a comment | **real**, cosmetic; no product relies on it. #14846 | bug | S |
| EM-11 | Compose `performScrollTo()` hang | **unverified**. The reporter's own bisection cleared the emitter; the workaround is still in `TaskAppUiTest.kt:53,107`. #14790 | test infra | M |
| EM-12 | The qualified `pkg::P::C` tag contract is undocumented | docs only: all 8 backends already reject it. #14886 | docs | S |

## Component program and kernel

| ID | Problem | Status and evidence | Kind | Size |
|---|---|---|---|---|
| PR-1 | Missing components: Chip, StatusPill, Icon, Legend, ValidatedField, InlineEditForm, Composer, StatusPanel, Toolbar, and the L3 task components | **real**. #14415, #14011 | feature | L |
| PR-2 | Toolkit `Button` has no a11y-label slot (and SegmentedControl also needs `selected`) | **real**. #15421 | a11y | S/M |
| PR-3 | `HostLink` has no current-page state (`Nav.mll:6` marks it visually only); no arrow-key roving focus | **real**. #15458 (S–M), #15457 (M) | feature | S–M, M |
| PR-4 | Publishing the `mosaic-pkg-*` crates (all 24 say `publish = false`) | decision. #14014 | decision | M |
| PR-5 | UI48 environment observer for the React, HTML and Web Component backends | **real**: ENV1–ENV4 are done on all 5 native backends. #14003 | feature | M |
| PR-6 | Adopt `HostNavigationSplit` in notes, note-type-editor and spice-workbench; UI29-6 still says "draft" | **real**. #15481 follow-up | feature | S |
| PR-7 | Qt releases cover macOS only (`engram_release.py:527`) | **real**. #14223 | feature | M–L |

## Decisions needed

- **Untrusted styles (X-4).** May a package dependency's `.msl` carry XAML
  markup extensions? If not, `translate_xaml_value` should accept only a
  closed set: `{ThemeResource …}` and `{StaticResource …}` with a
  validated key.
- **UI88 §6, rich card content.** Blocks EN-2, EN-3 and Journal photos
  (JO-3).
- **Publishing (PR-4).** Should the `mosaic-pkg-*` crates be published, and
  under what policy? Blocks MB-10's downloadable demos.
- **Signing (TR-6).** Per platform: sign, notarize, or record "unsigned, no
  credentials".
- **Flutter topbar (TR-5).** Should the topbar wrap or flex below 1600 px?

## Closable now

**Closed on 2026-10-07**, each with a comment naming the fixing PR; every
entry below except the *Close or narrow* and *Check off* items. #14026, #14886
and #15921 stay open with a status comment that narrows them. Each of these
was verified on `84252d038a`.
- **Fixed by a merged PR:**
  - #13187 (#15018), #13184 and #14360 (#16503), #13022 (`ci.yml:2179`);
  - #14116, #14132 (#15040), #14639 (#14774);
  - #14708 (#14775, #14777, #14821, #14832);
  - #14772 (#14786, #14791), #14793 (#14794);
  - #14798 (#14801, #14805, #14815, #14828; the other three were
    retracted by the reporter);
  - #14804 (#14805, #14807, #15129; 4 Box-only drops remain, reported);
  - #14810 (#14817, #14818, #14821), #14826 (#14827), #14866 (#14941);
  - #15269 (#15279), #15483 and #15560 (#15608);
  - #14018 (#16000), #13940 (#13942, #14134).
- **Done:** #14661 (#15745), #14272 (#14311, #15745), #14854 (#14993,
  #15001).
- **Not a defect:** #14924. The reporter's own diff shows a drop shadow.
- **Superseded:**
  - #14709, by #12022;
  - #14459, by #14704 (every other slice merged);
  - #12027 part 2, by #14013 (part 1 shipped in #13180);
  - #14269, by #14277.
- **Epics whose checklists are done:** #13624 (Engram), #15481
  (HostNavigationSplit), #14016 (SegmentedControl).
- **Close or narrow:**
  - #14026: the docs site is live; only the demo links remain.
  - #14886: docs only.
  - #15921: now a feature request.
- **Check off:** #14011's "MosaicBook in CI" item (#14012 is closed).
