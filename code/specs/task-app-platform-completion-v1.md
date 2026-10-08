# TaskApp platform completion v1

Epic: [#13517](https://github.com/adhithyan15/coding-adventures/issues/13517)

> **Superseded in part — 2026-09-02.** The *measurement* here stands. The
> *working method* — finish the app, discover platform gaps as features need
> them — is replaced by `mosaic-component-program-v1.md` ([#14011](https://github.com/adhithyan15/coding-adventures/issues/14011)),
> which builds leaf to root. Tier A item 2 was the trigger: it turned out to be
> a kernel gap wearing an app-shaped costume, and it was the second in one
> session.

## Why this document exists

`BACKLOG.md`'s "Next up" section stopped being able to name the next item. It
had emptied its own P0/P1 queue down to four leftovers and then deferred the
choice to "a fresh pass over `task-app-super-app.md`". That pass is this
document.

The super-app spec answers *what features Trestle should have*. It does not
answer *which of Mosaic's nine backends Trestle is actually finished on*, which
is the question that decides whether the product is done. This spec answers the
second question, records the measured per-backend state, and turns the gaps into
an ordered queue.

## Scope: what "all platforms Mosaic supports" means

`mosaic-compile` accepts nine backends. They are not nine equivalent app
targets, and the completion bar differs by kind:

| Backend | Kind | Completion bar for TaskApp |
| --- | --- | --- |
| `react` | Interactive web host | Production bundle, real engine, persistence, shipped |
| `webcomponent` | Interactive web host | Same bar as `react` |
| `html` | Static snapshot (no JS) | Renders the authored shell truthfully; no interaction claim |
| `qt` | Native desktop | `native-complete`, real runtime, driven lifecycle, packaged |
| `flutter` | Native desktop | Same as `qt` |
| `compose` | Native desktop | Same as `qt` |
| `swiftui` | Native desktop (macOS) | Same as `qt`; iOS is source portability only today |
| `xaml` | Native desktop (Windows) | Same as `qt` |
| `paint` | Raster snapshot | Deterministic PNG of the authored shell; a visual gate, not a host |

## Measured state, refreshed 2026-09-27

Established by reading `.github/workflows/ci.yml`,
`.github/workflows/release-task-app.yml`, and every `mosaic-emit-*` crate for
TaskApp references.

**Gated and shipped (6 of 9).** `react` ships the tested production web bundle.
`qt`, `flutter`, `compose`, `swiftui`, and `xaml` each generate under the strict
`native-complete` profile with the real `task-mosaic-app` runtime, pass an
emitted-control contract that drives the simple-todo lifecycle, and produce a
release artifact.

**Structurally gated (1 of 9).** `html` compiles TaskApp's own interface,
package-expanded layout, and light theme in the ordinary emitter test lane
(#16120). The gate pins the List composer, repeated task-row template, and
toolkit SegmentedControl lowering. It is a static structural snapshot: there is
no runtime, interaction, host, or release-artifact claim.

**Interactively parity-gated (1 of 9).** `webcomponent` compiles both authored
themes from TaskApp's package-expanded sources, shares the framework-neutral
presentation controller and persistence contract with React, and drives create,
complete, restore, and delete through the emitted Custom Element controls
(#16125). It is deliberately a CI parity bundle, not a shipped release artifact.

**Visually gated (1 of 9).** `paint` package-expands TaskApp's real interface,
layout, dependency styles, and both authored themes, then rasterizes reviewed
1280 x 900 CPU Skia goldens (#16151). The gate proves repeat rendering is
byte-identical, validates the PNG dimensions, and requires distinct light and
dark output. It is a CI snapshot, not a host, interaction claim, or release
artifact.

**Mobile reach.** iOS now builds an installable generated app with the real
statically linked Rust runtime, launches on both iPhone and iPad simulators,
and exercises sandboxed restore and corrupt-state quarantine (#16035, #16041,
#16144). That is runtime evidence, but there is still no signed device build or
TaskApp release artifact. Android has no Mosaic backend on `main`; `compose`
still denotes Compose *Desktop*.

## Completion queue

Ordered. Tier A finishes the product on the platforms it already claims; Tier B
closes the three unexercised backends; Tier C states the reach items honestly
rather than letting them read as silent gaps.

### Tier A — finish the claimed platforms

1. **P1 [#13695](https://github.com/adhithyan15/coding-adventures/issues/13695)
   — startup loading and failure states.** **Done** for the web host and all five
   strict native backends; native recovery completed in #15788, #15821, #16086,
   #16093, and #16100.
2. **P1 [#13692](https://github.com/adhithyan15/coding-adventures/issues/13692)
   — compact-window List layout.** **Done in #15700.** TaskApp now consumes the
   kernel's `HostNavigationSplit`, so capable native platforms own compact pane
   collapse instead of a TaskApp-specific viewport slot. UI48 (#14003) remains
   broader Mosaic environment work rather than a blocker for this product item.
3. **P2 [#13526](https://github.com/adhithyan15/coding-adventures/issues/13526)
   — Vitest on Vite's native ESM loading.** Test-infrastructure debt in the web
   host; blocks nothing, but it is the last known non-product wart in the lane
   that gates every web change. **Done in
   [#14242](https://github.com/adhithyan15/coding-adventures/pull/14242).**
4. **P2 [#13625](https://github.com/adhithyan15/coding-adventures/issues/13625)
   — changelog roll-forward gate.** **Done in #15090.** Release validation now
   requires exactly one bracketed Unreleased section and the requested version
   as the newest dated release section.

### Tier B — close the unexercised backends

5. **Static HTML snapshot gate.** Emit TaskApp through `mosaic-emit-html` from
   its own sources and assert the authored List-first shell — composer, task
   rows, view switcher — survives lowering. Cheapest of the three: no runtime,
   no host, no interaction claim. Its value is that it fails loudly when the
   authored shell stops lowering truthfully. **Implemented by #16120.**
6. **Web Components host.** The only remaining *interactive* backend with no
   TaskApp presence. It needs the same treatment `react` has: emit from
   TaskApp's sources, wire the custom element to `task-wasm`, drive the
   simple-todo lifecycle through the emitted controls, and decide explicitly
   whether it earns a release artifact or is a parity gate only. **Implemented
   as the explicit parity-only gate in #16125.**
7. **P1 [#16151](https://github.com/adhithyan15/coding-adventures/issues/16151)
   — Paint visual-regression gate.** Package-expand TaskApp's real interface
   and layout, compose the dependency and product styles for each authored
   theme, and rasterize both at the declared 1280 x 900 desktop size through
   the explicitly selected CPU Skia path. The PNG bytes are deterministic
   within that pinned path and are compared with reviewed repository goldens;
   the test also proves repeat rendering is byte-identical, the dimensions are
   correct, and light and dark do not collapse to the same image. This is the
   only mechanism in the stack that would catch a purely visual regression;
   every existing gate asserts structure, semantics, or behavior. It remains
   a CI-only raster snapshot, not a host, interaction claim, or release
   artifact. **Implemented for #16151.**

### Product release checkpoint

8. **P1 [#16165](https://github.com/adhithyan15/coding-adventures/issues/16165)
   — publish `task-app-v0.5.0`.** Record the completed Paint gate as the
   all-nine-backend product-coverage checkpoint. This minor release recognizes
   a newly verified capability, but does not promote Paint, static HTML, or Web
   Components into a shipped host. The verified web and strict-native artifact
   matrix remains unchanged. **Published from #16166.** The artifact set and
   provenance verified, but generated notes copied only the change heading and
   omitted this explicit CI-only boundary.
9. **P1 [#16168](https://github.com/adhithyan15/coding-adventures/issues/16168)
   — publish corrective `task-app-v0.5.1` notes.** Put the CI-only backend
   boundary directly in the release-note template, protect it with a focused
   test, and publish an immutable patch from the same verified artifact matrix.
   **Published and independently audited from #16170.**

### Product quality ratchet

10. **P1 [#16182](https://github.com/adhithyan15/coding-adventures/issues/16182)
    — ratchet TaskApp's native style degradations.** All five native reporters
    are now wired. The initial `native-complete` generation recorded 77 XAML,
    142 SwiftUI, 89 Compose, 307 Qt, and 463 Flutter style drops. **Ratcheted in
    #16183.** Successive emitter and product fixes reduced the fresh inventory
    to 77 XAML, 144 SwiftUI, 89 Compose, 163 Qt, and 197 Flutter drops. #16995
    tightens the Qt and Flutter maxima to those fresh reports after #16297 and
    #16969, retiring 63 stale allowances that could otherwise regress silently.
    #17010 then preserves TaskApp's authored letter spacing through each native
    text API, retiring another 60 allowances and reducing the inventories to 65
    XAML, 132 SwiftUI, 77 Compose, 151 Qt, and 185 Flutter drops.
    Keep the per-property maxima in the shared TaskApp contract so no new
    property or increased occurrence count can enter while fixes drive those
    inventories toward #12022's zero-drop hard fail. Do not call the existing
    debt native completeness.

### Tier C — reach, stated honestly

11. **iOS: run, don't just compile.** **Done for the CI/runtime claim in #16035,
    #16041, and #16144.** The generated app carries the real runtime, launches
    on iPhone and iPad simulators, restores state from its sandbox, and
    quarantines corrupt state. Signing, device execution, and a downloadable
    iOS artifact remain outside that claim.
12. **Android has no backend.** `compose` is Compose Desktop. An Android target
    is a new Mosaic backend, not a TaskApp task, and belongs to
    [#12017](https://github.com/adhithyan15/coding-adventures/issues/12017), not
    here. Recorded so its absence is a decision rather than an oversight.
13. **Signing, notarization, and installers**
    ([#13977](https://github.com/adhithyan15/coding-adventures/issues/13977)).
    macOS is unsigned and un-notarized; Windows is an unsigned portable folder,
    not MSIX; Linux ships tarballs, not packages. Writing this spec turned up
    that the README pointed at #13522 for exactly this — and #13522 closed on
    2026-08-31, so the limitation was recorded against a closed issue and
    signing was tracked nowhere. #13977 now holds it. Credentials this
    repository does not hold are the blocker, not engineering.

## Working method

Unchanged from the epic: one child issue per pull request, at most one active
TaskApp PR, spec-sync → tests → implementation → CHANGELOG → README →
`/security-review` → PR → babysit → auto-merge. New findings become issues and
force a re-prioritization pass before the next child is selected.

## What this document does not do

It does not restate the super-app feature roadmap, and it does not re-open the
Phase 10+ deferrals in `BACKLOG.md` (Gantt dependency arrows, calendar week/day
views, notes attachment picker, label colors, recurring tasks, automation
rules). Those are feature reach on platforms that already work. This spec is
about finishing the platforms.
