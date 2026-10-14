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

**Next bounded visual fix — #17502: SwiftUI milestone rotations.** TaskApp's
timeline and calendar milestones both author a centered `rotate(45deg)` to
make diamonds. Lower a single finite degree rotation after sizing and surface
paint but before elevation shadows. Keep native layout footprint unchanged,
preserve state precedence, and report malformed, non-finite, multi-transform,
and other-unit forms. Verify authored output before retiring the two transform
allowances. Translation, scaling, and transform-origin remain outside scope.

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
    #17031 lowers TaskApp's cross-axis alignment through SwiftUI stack
    constructors, retiring all 22 SwiftUI `align` allowances and reducing that
    backend's fresh inventory to 110.
    #17052 lowers supported uppercase text transforms through all five native
    text APIs, retiring another 55 allowances and reducing the inventories to
    54 XAML, 99 SwiftUI, 66 Compose, 140 Qt, and 174 Flutter drops.
    #17073 then preserves supported TaskApp font weights in Qt and Flutter,
    reducing those two inventories to 138 and 158. #17098 removes solid
    per-edge border styles only where a supported positive-width edge is
    actually drawn, retiring another 70 false positives and reducing the
    fresh inventories to 38 XAML, 83 SwiftUI, 50 Compose, 130 Qt, and 144
    Flutter drops. #17126 then reconciles Qt's existing native elevation
    effects with the reporter, retiring eight false positives while surfacing
    22 genuine, previously unqueried `HostDraggable` board-card omissions. The
    honest Qt inventory is therefore 144; #17128 tracks those native lowerings.
    #17151 lowers the draggable container surfaces and leaves an honest 113
    drops after surfacing five calendar-event text omissions. #17146 then
    preserves that event surface's supported inherited color/font size and its
    complete hidden/ellipsis/nowrap contract, retiring those five omissions
    and reducing Qt's fresh inventory to 108.
    #17338 then lowers the 15 exact `width: 100%` declarations on Qt inputs and
    buttons to `Layout.fillWidth: true`, reducing Qt's fresh inventory from 108
    to 93. The three remaining Qt width drops are `Text` geometry; unsupported
    percentages and mixed fixed/fill states remain reported.
    #17386 then reconciles Qt's ten non-inset CSS shadows with the supported
    `elevation` tokens that already emit equivalent native `MultiEffect`
    depth, reducing its fresh inventory from 93 to 83. The inset theme-toggle
    moon and raw shadows without elevation remain drawing-effect debt.
    #17356 reconciles Compose's ten non-inset CSS shadows with the supported
    `elevation` tokens that already emit equivalent native depth, reducing its
    fresh inventory from 50 to 40. The inset theme-toggle moon and raw shadows
    without elevation remain reported drawing-effect debt.
    #17365 lowers SwiftUI's supported elevation tokens to native `.shadow`
    modifiers and reconciles only their ten non-inset CSS companions, reducing
    its fresh inventory from 83 to 63. Standalone, inset, and unsupported
    shadows or elevation values remain reported drawing-effect debt.
    #17394 lowers TaskApp's complete seven-way XAML
    weekday percentages to proportional Grid stars and its wrapping repeated
    day cells to `UniformGridLayout`, reducing XAML from 38 to 29. Partial,
    mixed, invalid, and unsupported wrapping shapes remain explicit debt.
    #17224 then preserves all 15 supported fixed Flutter `border-radius`
    declarations and paints eight coupled rounded-text backgrounds, reducing
    Flutter's fresh inventory from 144 to 121. Relative and otherwise unsafe
    radius values remain explicit debt.
    #17237 then preserves all 14 supported TaskApp `text-align` declarations
    on Flutter text and button leaves, reducing that backend's fresh inventory
    from 121 to 107. Unknown alignment values remain reported.
    #17251 preserves all 16 TaskApp `width: 100%` declarations on Flutter leaf
    widgets when their parent supplies a finite horizontal constraint, reducing
    that backend's fresh inventory from 107 to 91. The seven calendar weekday
    `14.2857%` widths remained reported until #17289 lowered valid direct-row
    percentages to proportional flex shares and wrapping rows to native `Wrap`
    children with fractional sizing. That preserves the seven-column calendar
    header and day grid, retires the seven remaining `width` omissions plus the
    grid's `flex-wrap`, and reduces Flutter's fresh inventory from 91 to 83.
    Invalid percentages and percentages without a finite parent remain reported.
    #17301 then applies authored `gap` to evaluated Flutter drag-and-drop child
    lists, retiring the four remaining gap omissions and reducing that
    backend's fresh inventory from 83 to 79 without changing its native drag,
    keyboard, or drop semantics.
    #17308 reconciles Flutter's existing native elevation output with its
    reporter, retiring ten non-inset `box-shadow` false positives and reducing
    that backend's fresh inventory from 79 to 69. The inset theme-toggle moon
    shadow remains explicit drawing-effect debt rather than being mistaken for
    native elevation.
    #17313 then aligns Flutter's drag-body helper detection with the decorated
    and lightweight lowering paths. A leading `If`/`Else` list now receives the
    helper definition it calls, and a decorated conditional followed by an
    ordinary child keeps that child in the evaluated vertical list. This is a
    generated-Dart correctness repair and does not change the 69-style-drop
    inventory.
    Keep the per-property maxima in the shared TaskApp contract so no new
    property or increased occurrence count can enter while fixes drive those
    inventories toward #12022's zero-drop hard fail. Do not call the existing
    debt native completeness.

### Native fidelity release checkpoint

**Completed in #17483 — #17481: SwiftUI drag-body spacing reports.** Both
`HostDraggable` and `HostDropTarget` already construct a vertical stack with
authored spacing. Recognize those constructors in the gap reporter, requiring
every occurrence of a shared part to consume the property. Unsupported values
and reuse on non-spacing primitives must remain reported. Verify generated
TaskApp before retiring its four gap allowances. Fresh package-expanded
generation measures 59 style drops and no gap drops, down from 63; native
runtime generation and Swift compilation passed in the macOS release CI lane.
Drag callbacks and native
behavior are unchanged; this is reporting reconciliation.

11. **P1 [#17262](https://github.com/adhithyan15/coding-adventures/issues/17262)
    — publish `task-app-v0.5.2`.** Roll the compatible native-fidelity fixes
    accumulated after v0.5.1 into an immutable patch release. Keep the verified
    web/native downloadable artifact matrix unchanged; Static HTML, Web
    Components, and Paint remain CI-only, while signing and notarization remain
    tracked by #13977. **Published and independently audited.**
12. **P1 [#17328](https://github.com/adhithyan15/coding-adventures/issues/17328)
    — publish `task-app-v0.5.3`.** Roll the compatible Flutter calendar,
    drag/drop spacing, conditional-body correctness, and elevation-reporting
    fixes into an immutable patch release, together with the matching
    style-ratchet correction. Keep the verified downloadable artifact matrix
    unchanged: Static HTML, Web Components, and Paint remain CI-only, while
    signing, notarization, and installers remain tracked by #13977.
13. **P1 [#17375](https://github.com/adhithyan15/coding-adventures/issues/17375)
    — publish `task-app-v0.5.4`.** Roll the compatible Qt fill-width and
    Compose/SwiftUI elevation-shadow fixes into an immutable patch release.
    Keep the verified downloadable artifact matrix unchanged: Static HTML, Web
    Components, and Paint remain CI-only, while signing, notarization, and
    installers remain tracked by #13977.

**Published checkpoint — [#17469](https://github.com/adhithyan15/coding-adventures/issues/17469):
`task-app-v0.5.5`.** Published from #17470 and audited: immutable tag and
manifest match the source commit, all twelve checksum entries match GitHub
asset digests, and the existing artifact coverage and limitations are retained.

**Next release checkpoint — P1 [#17491](https://github.com/adhithyan15/coding-adventures/issues/17491):
publish `task-app-v0.5.6`.** Roll forward the compatible SwiftUI drag-body
gap reporting correction (#17483). This corrects diagnostics and tightens the
style regression gate; emitted drag behavior is unchanged. Validate the existing
release matrix, publish from main, and audit the immutable assets. Static HTML,
Web Components, and Paint remain CI-only; signing and installers remain #13977.

### Tier C — reach, stated honestly

14. **iOS: run, don't just compile.** **Done for the CI/runtime claim in #16035,
    #16041, and #16144.** The generated app carries the real runtime, launches
    on iPhone and iPad simulators, restores state from its sandbox, and
    quarantines corrupt state. Signing, device execution, and a downloadable
    iOS artifact remain outside that claim.
15. **Android has no backend.** `compose` is Compose Desktop. An Android target
    is a new Mosaic backend, not a TaskApp task, and belongs to
    [#12017](https://github.com/adhithyan15/coding-adventures/issues/12017), not
    here. Recorded so its absence is a decision rather than an oversight.
16. **Signing, notarization, and installers**
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
