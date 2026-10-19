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

## Why products stalled (diagnosed 2026-10-12)

Investigated with git history (6,465 commits on main since 2026-08-03),
every PR closed since then, every open issue, and CI runs. In short, nothing
was blocked in a way anyone could see: the work was driven by automated
loops, and the stalled products had none.

1. **No loop drives them, and their queues ran dry.**
   - Work in this repo comes from long-running agent loops:
     - `codex/*` branches: 56% of PRs;
     - `claude/*` sessions: 12% of PRs.
   - Since 2026-09-21 the loops have been mermaid (225 PRs), algol (139),
     human languages (92 codex PRs plus two Claude sessions with 380
     commits), lang (88) and parity (72). TaskApp also has a loop (40 PRs),
     and it is the one Mosaic app that keeps moving.
   - SpiceWorkbench, MosaicBook, Journal and Engram have no loop. VisiCalc's
     loop is narrow (9 PRs, XAML performance only).
   - Where a plan existed, it ran out:
     - SpiceWorkbench's `spice-full-implementation-plan.md` marks all 24
       items done;
     - Venture's BR01 backlog is 82 of 82 done;
     - both stopped on 2026-09-20.
   - What remains is large or vague (Venture's BR02 phases; BR03 steps 5–6;
     SpiceWorkbench's "Suggested PR Queue" has three lines), so an agent
     looking for the next small item found none. None of the 322 open
     issues is assigned.
   - **Fix:** give each product an ordered queue of small, concrete items
     (this file's rows), and a loop or a fixed share of loop time that works
     it from the top.
2. **Breakage outside a product's own CI is invisible.**
   - Main CI has not passed since July (X-7), so it gives no signal.
   - The MosaicBook gate compiles stories only for browser backends, so
     SpiceWorkbench's failures on four native backends (SW-2) are recorded
     nowhere.
   - Shared emitter changes break Venture silently:
     - #16178 sat red for 8 days, then failed again on a Flutter host
       method removed by #16322. Its fix says "main stayed green only
       because nothing rebuilt Venture there".
     - #16054 and #16387 are earlier cases.
   - So each product PR first pays for other people's regressions.
   - **Fix:**
     - triage X-7;
     - add native backends to the MosaicBook gate, or record native
       degradations;
     - rebuild each product when the emitters or host templates change
       (the build tool's affected set), or nightly.
3. **Owner decisions sat unmade.** Under the specs-first rule, no code
   starts until they are made:
   - SwiftUI `Path`, #13206: 46 days with no comment;
   - UI88 rich card content, blocking EN-2, EN-3 and JO-3;
   - publishing (PR-4), signing (TR-6) and the Flutter topbar (TR-5);
   - the UI90 decisions, which were settled on 2026-10-12.
   - **Fix:** list them in [Decisions needed](#decisions-needed) with a
     recommendation each, and raise them at the start of a session.
4. **Releases never happen.**
   - Venture's release workflow has not run since 2026-08-31 (VE-2).
   - Journal's has never run, and there is no `journal-v*` tag (JO-1).
   - Finished work therefore never reaches users, and nothing pulls a
     product forward.
   - **Fix:** one owner per release. For Venture and Journal, start with a
     prerelease tag that exercises the pipeline.
5. **Fixes ship in large batches.** The 2026-10-07 triage fixes went out as
   one 27-commit PR (#16855) that took 4.5 days to merge, and fixed issues
   stayed open meanwhile. **Fix:** one small PR per product, and close the
   issue in the same step.

## Order of work

1. **Security and visible breakage.**
   - EM-1, the XAML `{` literal (*fixed on main, #16883 and #16948*);
   - MB-14, MosaicBook listened on every interface (*fixed in #16855*);
   - MB-1, apps cannot be previewed (*fixed in #16855*);
   - SW-2, SpiceWorkbench does not compile on four native backends;
   - SW-1, `Path` on Web Components (*fixed in #16855*);
   - VC-1, VisiCalc's native Open/Save.
2. **Small bugs, batched by package.**
   - MosaicBook: MB-2, MB-3 and MB-4 (*fixed in #16855*);
   - emitters: EM-5 and EM-9 (*fixed in #16855*), and EM-3 (*fixed
     2026-10-19*; it was listed here as fixed by #16855, but that PR did
     not touch it);
   - X-2, the compiler's search path on Windows (*fixed in #16855*).
3. **Housekeeping.** Close the issues in [Closable now](#closable-now),
   update the out-of-date epic checklists, and refresh the docs rows (MB-12,
   SW-5, VE-3).
4. **Releases that never shipped.**
   - VE-2, Venture's release pipeline, and cutting 0.10.0;
   - JO-1, Journal's first release.
5. **Emitter correctness.**
   - EM-2, `align`: #16297 landed (2026-10-07); #16293 remains;
   - EM-4, SwiftUI layout;
   - TR-2 and TR-3, the native progress ring and elevation;
   - EM-6, Compose fixtures (also unblocks MB-5, and is the first step of
     DN-3).
6. **Product features**, smallest high-value first: MB-6, VE-1, SW-3, TR-5,
   EN-1 and the remaining VisiCalc native work.
7. **Decisions** in [Decisions needed](#decisions-needed), which unblock
   EN-2, EN-3, JO-3 and PR-4.
8. **Platform-neutral mosstyle and design to native.** UI90's §6 order
   (X-6), then UI91 for DN-1 and DN-2, and DN-3's fixtures on every native
   backend. The UI90 compiler and token steps come first, because the design
   import lands on them.

## Cross-cutting

| ID | Problem | Status and evidence | Kind | Size |
|---|---|---|---|---|
| X-1 | `ci.yml` on main almost never completes: each push cancels the run before it (`cancel-in-progress`) | **fixed** in #16855: 12 CI workflows now set `cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}`, so a main run always finishes and newer pushes queue. Deploy and release workflows are unchanged. `build-ocaml-representative.yml` is excluded: its bytes are pinned by the closed OCAML07 contract (`code/scripts/ocaml_representative_ci.py`), so changing it needs a reviewed contract re-pin of its own. Was: Every one of the last 300 completed `ci.yml` runs on main was cancelled. The last success was on 2026-07-21. The last full build (run 36528879089, 2026-09-29) failed all 15 shards. Nobody can tell from main whether any product is green | CI | M |
| X-2 | `mosaic-compile` splits `--package-search-path` on `:` on every OS | **fixed** in #16855: Windows splits on `;` and `:` and keeps a drive colon. Was: `mosaic-compile/src/main.rs:755` uses `split(':')`, so a Windows drive path (`C:\…`) breaks in two. MosaicBook's sibling search path is absolute, so it is affected on Windows. Fix: `std::env::split_paths`, plus OS-specific joining in MosaicBook. #16931 | bug | S |
| X-3 | Native backends drop authored style properties silently or with a warning only | **partly**. Every backend reports drops (#15532, `mosaic-package-artifact-builder/src/lib.rs:2757-2817`), but the gate fails only on capability degradations. TaskApp's ratchet (`code/scripts/taskapp_native_control_contract.py:22`) allows 741 drops: XAML 77, SwiftUI 142, Compose 89, Qt 193, Flutter 240. Tracked by #12022 | bug | L |
| X-4 | Style values that start with `{` passed into XAML verbatim, as markup extensions | **decided and fixed** in #16855. Decision (2026-10-08): mosstyle must express every styling decision with no platform-specific values or abstraction leaks. The compiler refuses such a value (`PlatformValue`, UI15 §8 rule 8), and the XAML emitter drops it. **Follow-up:** an audit of every `.msl` for other platform leaks, and for the decisions mosstyle cannot yet express neutrally (X-6) | bug | S |
| X-6 | **mosstyle is not yet platform-neutral or expressive enough.** Principle (decided 2026-10-08): MSL must express every styling decision with no platform-specific values or abstraction leaks. An audit of all 186 `.msl` files on 2026-10-08 found it is CSS in practice:<br>• 82 distinct properties are in use against about 25 in UI15 §2. `ErrorKind::UnknownProperty` exists but is never raised, so the web backends accept any CSS.<br>• Colours: 3,258 hex literals against 98 tokens.<br>• Web-only constructs: `box-shadow` alongside `elevation` (24 uses), `rgba()` (44, which only XAML converts), CSS font stacks, `position: absolute/sticky` with `top/left/z-index`, `vh` units, `display: flex` workarounds, `cursor`, `color-scheme`, `currentColor`, `transform: rotate()`, shorthand `border`/`outline`, and per-backend token overrides.<br>• Emitter pass-through rules: XAML's "uppercase means native" values, its verbatim fallbacks for fonts and enums, and Qt's raw font family; Flutter reads 8-digit hex as AARRGGBB while CSS reads RRGGBBAA; Compose and SwiftUI disagree on named colours.<br>**Gaps MSL cannot yet express neutrally:**<br>(1) the system palette (accent, surface, text, focus);<br>(2) dark mode in one file (95 `.light` + 70 `.dark` near-duplicates);<br>(3) elevation as the only shadow;<br>(4) type roles, tracking, text case;<br>(5) truncation and max lines;<br>(6) full corners;<br>(7) per-edge borders and stroke style;<br>(8) the focus ring;<br>(9) main- and cross-axis alignment;<br>(10) environment-conditioned blocks and motion with reduced motion.<br>**Next:** UI90 (`UI90-mosstyle-platform-neutral.md`, draft, 2026-10-09) specifies a closed, typed, platform-neutral mosstyle — a property and value table enforced by the compiler; semantic and mode-aware tokens; one colour-with-alpha syntax — with a migration path for existing `.msl` files. Brand vs native (#12029) was decided on 2026-10-11 (UI90 §5.1): use native controls, let authors brand them token by token, and let unstyled properties follow the platform. Decisions 2–4 were settled on 2026-10-12 by copying existing systems: Material 3's 15 type roles with explicit sizes still allowed; mode-aware tokens with optional high contrast; custom fonts shipped with the app. UI90 is ready for implementation in its §6 order. Closing this removes most of X-3's 741 baselined native drops.<br>**in progress** (2026-10-14): UI90 §6 step 2 has landed. The v2 property table and checker exist, and `[styles] version = 2` makes them binding. The toolkit is at 108 findings. Next is step 3 (typed system tokens) or step 4 (`style migrate`). | design | L |
| X-7 | **`ci.yml` on main has not passed since 2026-07-21** | **partly fixed** (2026-10-13). Triage of run 38418336876, checked against run 38388561794, found five causes:<br>• (A) 59 stale `ALLOWED_STYLE_DROPS` pins in 13 Mosaic packages. The emitters learned those drops, and the gate's inverse ratchet failed. This cost 8 shards.<br>• (B) The runner disk filled up, on shards 5, 6 and 12.<br>• (C) `-validate-build-files` resolved names against a shard's packages only, so Perl test-only dependencies looked undeclared. This failed shards 14 and 15.<br>• (F) Ruby gem drift: an unknown cop name in a `rubocop:disable` comment, and json 3.1 removing `create_additions:`. This failed shard 11.<br>• Flakes: (D) a SIGTERM test race in `forme-plugin-runner-py`, (E) an NDK download on shard 7, and (G) 0% coverlet coverage once on `csharp/immutable-list`.<br>A, C, D and F are fixed in one PR, with all 15 shards' validation passing locally. A, C, D and F merged in #17450 (2026-10-14). B and E merged in #17458: `-prune-leaf-targets` deletes each leaf standalone Cargo package's `target/` once it has built during main's full build, and `build-mosaic-flutter-phone-app.sh` builds against the runner's own NDK. **Second triage** (run 38509620113, 2026-10-14): 9 of 14 finished shards green, up from 1–2. The rest were a runner out of disk and a runner lost (both before #17458), plus ruff 0.17 findings in `python/window-native`, vitest stalls on a loaded runner, two stale `mosaic-std-controls` pins, and an IPv6 port race in `transport-platform`. The first four merged in #17479 (2026-10-14). The IPv6 race is fixed in #17494, together with three Windows-only transport bugs that #17479's Windows lane exposed: an RST on close, no multi-worker accept, and an acceptor that died on its first error. **Third triage** (runs 39070634762–39139594621, 2026-10-18): every run red again. Two causes: (H) shards 11–13 hit the 150-minute ceiling because the step passed `-jobs 1` whenever an OCaml switch was active, so the five shards holding an OCaml package built ~1,000 packages each on one worker (#17631); and (I) shard 5 ran out of disk on all five runs (#17652, #17668). Sharding v1 rebuilt the repository 2.8× over (14,878 package builds for 5,327 packages). #17759 adds an executor `global:opam-switch` lock in place of `-jobs 1`, plus locality-aware sharding on measured durations (`-shard-weights`, 5,908 builds, every shard at 69 serial minutes), a disk watch around the full build, and a `ci-full-build` PR label that runs this build before merge. **Still open:** confirming a green main run, and paging someone (or blocking merges) when main is red. Was: **real** (2026-10-12). X-1 made main runs finish, not pass. Each of the three runs that finished after #16855 merged failed 14 or 15 of its 15 shards at "Full build on main merge". Each run takes 4–5 hours and covers 10 or more merges. Run 38189708298's shard 6 failed in the Mosaic step "Build Trestle, Journal and Engram for Android with Flutter", with a QML Repeater binding-loop annotation. Because PRs merge on their own gate, a red main blocks nobody and is invisible. Triage the shard failures, then make a red main page someone or block merges | CI | L |
| X-5 | Dependency style axes are not scoped to the call site | **real**. Only XAML guards against it (`xaml/pipeline.rs:1823-1834`, #14482). #14481 | bug | L |

## MosaicBook (`code/programs/go/mosaicbook-server`)

It works, but nobody has worked on it lately. The last server commit was
#15442 on 2026-09-17. `mosaicbook.yml` is green on main, and `--check` over
`code/packages/mosaic` compiles 61 components and 117 stories (468
compilations, 4 recorded degradations) in about 2 seconds. Native backends
get analysis only (the drop panel); nothing renders natively. Spec: UI19.

| ID | Problem | Status and evidence | Kind | Size |
|---|---|---|---|---|
| MB-1 | App components could not find dependencies outside their own directory, so VisiCalc, TaskApp, EngramApp and JournalApp failed on every backend | **fixed** in #16855: `--package-search-path` | bug | S |
| MB-14 | The server listened on every network interface, not just localhost as its README says. Another machine on the network could fetch previews by sending `Host: localhost` | **fixed** in #16855: it binds `127.0.0.1` | security | S |
| MB-2 | The browser shell never shows a component's `storiesError` | **fixed** in #16855. Was: `static/index.html` has no reference to it. UI19 §6.2 asks for a ⚠️ badge. #16928 | bug | S |
| MB-3 | The watcher reloads once without a change at startup, and re-runs discovery while holding the server lock | **fixed** in #16855. Was: Its file-time snapshot starts empty, so "detected file change" appears 1 second after start. Discovery runs `--describe` subprocesses under `s.mu` (`watcher.go`). #16929 | bug | S |
| MB-4 | Compose ignores `--fixtures` and `--emit-project`, even with `--strict-fixtures` (output byte-identical, exit 0) | **fixed** (2026-10-13, EM-6): Compose applies `--fixtures` to a real `--emit-project` shell. The fail-loud stopgap from #16855 is gone. Was: This breaks the `--strict-fixtures` contract in `mosaic-compile.json`. Fail loudly first; the fix is EM-6. #16930 | bug | S then M |
| MB-5 | Surface's `$mosaic-child-slot` cannot compile in isolation on html, webcomponent or react | **real**. Reproduced: "not yet supported by the pipeline HTML emitter". 3 recorded degradations. #14685 | bug | M |
| MB-6 | Form-factor layouts (`*.desktop.mll`, `*.touch.mll`) are never discovered | **real**. VisiCalc's Grid and FormulaBar and EngramApp's touch layout are invisible ("2 candidate(s) skipped") | feature | M |
| MB-7 | App components have no stories, so `--check` cannot cover them, and CI's check scans only `code/packages/mosaic` | **real**. VisiCalc, VisiCalcStartup, VentureChrome and others report "missing explicit .stories.json file" | feature | M |
| MB-8 | Story depth: 50 of 61 components have one empty Default story, with no dark-theme coverage (the server always prefers `*.light.msl`) | **real**. #14017 | feature | L |
| MB-9 | Native PNG snapshots, pixel baselines, and a message when a toolchain is missing | **partly**: Paint shipped (#15055). #14013, which absorbs #12027 part 2 | feature | L |
| MB-10 | Per-component demo app on every platform | **real**, not started. #14015 | feature | L |
| MB-11 | UI19 features not built: fixture editor (§6.3), `?fixtures=`, `width` and `height`, tiled mode, the `T` and `E` shortcuts, `/api/render`, `.mosaicbook/config.json`, render daemons (§7), Electron (§8) | **real** | feature | M–L |
| MB-12 | Out-of-date docs | **fixed** (2026-10-13): UI19 gains a §0 "As built" (SSE, Paint, three-file stories, `--check`, the degradations list, fixture validation, CI), and its `/ws` API section is now `GET /events`. `mosaic-component-program-v1.md` §2 gains a status note. The server README, help text and comments describe three-file components and the real reload triggers. `--check-stories` appears nowhere in the repo; the issue text is the only place. Was: **real**:<br>• UI19 says "Specification", WebSocket (the server uses SSE), Cairo/Skia and `.mosaic` files, and never mentions `--check`, the degradation file, Paint, three-file stories or fixture validation.<br>• `mosaic-component-program-v1.md` §2 says stories are impossible and MosaicBook is not in CI (#14436 and #14688 made both untrue).<br>• The README describes `.mosaic` files and the reload triggers wrongly.<br>• #15045 names `--check-stories` instead of `--check`. | docs | S |
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
| VE-2 | The release pipeline has never succeeded end to end | **real** (corrected 2026-10-12):<br>• `release-venture` has run four times, all on 2026-08-31, all failed. Two failed at "Publish prerelease"; the others at "Test portable Venture packages" and "Emit all Venture backends".<br>• The venture-v0.9.1 prerelease exists, published by hand. The fix, #13683, merged after the last run.<br>• The workflow has been edited four times since (#14212, #14216, #14452, #16955) and has never run, because no tag was pushed.<br>• Dependabot's #15723 (upload-artifact v7, download-artifact v8, action-gh-release v3) is open; the two artifact actions must move together.<br>Exercise it with a prerelease tag, then cut 0.10.0 (about 30 features unreleased since 0.9.1, 2026-08-31) | CI | S |
| VE-3 | The changelog is out of order | **fixed** (2026-10-13): the stray entries moved under `## [Unreleased]`, unchanged. Was: **real**. `CHANGELOG.md:1-50` sits above `## [Unreleased]` (line 51) | docs | S |
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
| SW-1 | The Web Component emitter rejects `Path` | **fixed** in #16855; MosaicBook's waiver removed. Was: It falls through to `UnknownPrimitive` (`mosaic-emit-webcomponent/src/pipeline.rs:3047`), and is waived in `mosaicbook-degradations.json`. Port from `mosaic-emit-html/src/pipeline.rs:1785`, then remove the waiver. #14686 | bug | S–M |
| SW-2 | Does not compile at all for SwiftUI, Compose, Flutter or Qt | **partly fixed** (2026-10-13): Qt, Flutter and Compose now bind `Path` coordinates to slots and `For` items, so SpiceWorkbench compiles on seven of eight backends. Each of the three has a `spice_workbench_compiles_to_<backend>.rs` acceptance test. SwiftUI still has no `Path` at all and waits on the #13206 decision. Was: **real, reproduced**:<br>• SwiftUI: "primitive 'Path' is not yet supported".<br>• Compose, Flutter and Qt: "Path prop 'x1' is bound … only supports a literal number".<br>Nothing records it, because MosaicBook gates only browser backends (`mosaicbook-server/check.go`), and the only native compile test is XAML's. XAML's #14682 is the template for the bound line geometry. #16926 has no owner. SwiftUI also waits on the #13206 decision | bug | M–L |
| SW-3 | No app shell or deploy; only package artifacts and the adapter's WASM test | **real** | feature | L |
| SW-4 | Collapse-never pinned split: no runtime proof | **real**. `SpiceWorkbench.mll` has no `HostNavigationSplit`, and there is no runnable app to resize. #15699 (parent #15481) | feature | M |
| SW-5 | No CHANGELOG, and the docs are out of date | **fixed** (2026-10-13): a CHANGELOG reconstructed from the 24 PRs, the README's `Path` state (and a Backends section), and the plan's WASM lifecycle marked completed (#15795). Was: **real**:<br>• It is the only `code/packages/mosaic` package without a CHANGELOG.<br>• The README calls data-bound `Path` "a native XAML follow-up"; #14682 already did it.<br>• `spice-full-implementation-plan.md` marks the WASM lifecycle "(in progress)", but #15795 landed it. | docs | S |

## VisiCalc (`code/programs/mosaic/visicalc`)

Active: draft #16864 (native focus restore and latency profiling) is open,
and a week of WinUI fixes has landed (#16699–#16750). `visicalc.yml` (web,
WASM, Rust, typography) is green on main. No CI job builds a generated
native app.

| ID | Problem | Status and evidence | Kind | Size |
|---|---|---|---|---|
| VC-1 | Open and Save cannot work on native hosts | **fixed** (2026-10-13): VisiCalc and SpiceWorkbench send `files.open` / `files.save` with an `accept` list, and every host maps the new `application/vnd.coding-adventures.visicalc+json` type to `.visicalc`. The browser's `file.*` aliases were retired the same day. Was: **real**. `visicalc-mosaic-app/src/lib.rs:293` sends `file.open`/`file.save`, but native platform libraries answer only `files.open`/`files.save` (`templates/xaml/MosaicPlatformEffects.cs:153`). UI87 keeps the old names on the browser only "until VisiCalc migrates". The accept list needs a type whose extension is `.visicalc`. Blocks #14548, #14279 and #14276. Coordinate with #16864. #16927 | bug | S–M |
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
| EM-1 | An authored XAML literal starting with `{` became a markup extension | **fixed on main** by #16883, with the style allow-list in #16948. This branch's parallel fix was dropped in favour of main's, and X-4 then emptied the allow-list. #15487 | security | S |
| EM-2 | `align` is never read on Qt and Flutter; main-axis and Text `align` semantics are undefined | **partly** (checked 2026-10-13): cross-axis alignment landed in #16297 on 2026-10-07, and #15258 is closed. What remains is #16293: main-axis `align` and `align` on `Text` still need a design. | bug | S/M + M |
| EM-3 | `HostInput` ignores `multiline` on react, html, webcomponent, Qt and SwiftUI, with no degradation reported | **fixed** (2026-10-19). `multiline: true` is now part of HostInput's contract (UI29 §2.1), as UI29-3 assumed when it declined a `HostTextarea`: a `<textarea>` on the web backends, Qt's `TextArea`, SwiftUI's `TextEditor`, matching XAML, Compose and Flutter. Enter is a line break in a text area on all eight, so XAML's Enter branch, Compose's `ImeAction.Done` and Flutter's dead `onSubmitted` went too, and `onCommit` on a multiline field is reported everywhere as `interaction.multiline-commit-unbound`. Also fixed in passing: Qt's import check, which would have emitted a `TextArea` without `QtQuick.Controls`. **Follow-ups:** UI25's Ctrl/Cmd+Enter commit for text areas was never built on any backend; `placeholder: slot:` is still literal-only on React, Web Components and SwiftUI, unreported. Was: A probe emits `<input type="text">`, a Qt `TextInput` and a SwiftUI `TextField`. #15930 | bug | S–M |
| EM-4 | SwiftUI drops `align`, `align-items`, `justify-content`, `flex-grow` and `flex-wrap` | **partly**. Pinned at `engram-app/tests/native_complete_gate.rs:154-161`. #14728 | bug | M |
| EM-5 | html emits state hooks (`data-disabled`) with no CSS to consume them | **partly**. #14824 | bug | S–M |
| EM-6 | Compose pipeline mode has no `EmitOptions` or project shell, and ignores `--fixtures` | **fixed** (2026-10-13): `--emit-project` writes the Compose Desktop Gradle project that `pkg` writes, built by the same generator (`compose_pipeline_project_files`), and story fixtures replace its sample values (UI32 §2.2). Not compiled with Gradle in the agent container, because its network policy blocks dl.google.com. Was: **real**. `compose/pipeline.rs:69`, `main.rs:1392-1411`. #14704 | feature | M |
| EM-7 | Permissive XAML shells bind the host by reflection, so mismatches fail silently | **partly**: native-complete shells bind directly. `xaml/pipeline.rs:9047-9192`. #14049 | bug | M |
| EM-8 | `HostButton` children are dropped on 6 of 8 backends (now reported as a degradation) | **real** as a feature. #15921 | feature | L |
| EM-9 | webcomponent (and html) emit `align: center-vertical` as a raw, invalid inline CSS declaration | **fixed** in #16855: translated to flex alignment, as React does. #16932 | bug | S |
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

## Design to native (DN)

Goal (2026-10-12): a design made in Figma, or a similar tool, becomes a
native Mosaic app on every backend quickly. It first runs on sample data,
then a backend is connected. Copy existing formats rather than invent them.
UI90 §5.5 is the token contract the import lands on.

| ID | Problem | Status and evidence | Kind | Size |
|---|---|---|---|---|
| DN-1 | **No way to bring a design in.** Nothing in the repo reads Figma or design-token files | **specified** in UI91 (`UI91-design-import.md`, draft, 2026-10-12). It reads local DTCG 2025.10 token files and lands on the existing JSON token palette (`mosaic-compile pkg --token-palette`, `[styles].token_palette`):<br>• W3C DTCG tokens or Figma variables, with their modes, become mode-aware brand tokens;<br>• text styles become `type-role`s;<br>• font families become shipped brand faces.<br>Depends on UI90 §6 steps 2–3 | design | M |
| DN-2 | **No way to turn a design's screens into Mosaic layouts** | **specified** in UI91 §4. It reads Figma's file JSON, saved locally or fetched with `design fetch`. Frames and auto layout map to `.mll` layouts over toolkit components, so controls are native. The design's own text and images become MosaicBook stories. Screens are generated once as editable code; tokens re-import every time (UI91 §2.3) | design | L |
| DN-3 | **A "dummy" native app needs sample data on every backend** | **fixtures done** (2026-10-13): Compose joined with EM-6, so every pipeline backend now applies `--fixtures` to its `--emit-project` shell. What remains is UI91 step 6, the end-to-end import-and-build test. Earlier note: every pipeline backend except Compose already applied `--fixtures` to its `--emit-project` shell (#14459); the earlier "HTML only" reading came from a stale flag description in `mosaic-compile.json`. Compose is the gap: it has no pipeline project shell (EM-6, #14704), so it refuses fixtures under `--strict-fixtures` and warns otherwise | feature | L |

## Decisions needed

- **UI88 §6, rich card content.** Blocks EN-2, EN-3 and Journal photos
  (JO-3).
- **Publishing (PR-4).** Should the `mosaic-pkg-*` crates be published, and
  under what policy? Blocks MB-10's downloadable demos.
- **Signing (TR-6).** Per platform: sign, notarize, or record "unsigned, no
  credentials".
- **Flutter topbar (TR-5).** Should the topbar wrap or flex below 1600 px?
- **SwiftUI `Path` (#13206).** How should SwiftUI-only work be verified?
  The options are a macOS CI runner, or accepting a reviewed lowering
  verified only by generated-source tests. Blocks SwiftUI in SW-2, and has
  waited since 2026-08-27. *Recommendation:* accept generated-source tests
  now, as the other native `Path` lowerings were, and add a macOS runner
  when one is available.
- **Agent loops for the stalled products** (see
  [Why products stalled](#why-products-stalled-diagnosed-2026-10-12)).
  Which products get a loop, or a share of an existing one? *Recommendation:*
  one Mosaic-products loop that works [Order of work](#order-of-work) from
  the top, one small PR at a time.

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
