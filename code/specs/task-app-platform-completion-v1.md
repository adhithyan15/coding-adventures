# TaskApp platform completion v1

Epic: [#13517](https://github.com/adhithyan15/coding-adventures/issues/13517)

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

## Measured state, 2026-09-01 (`dda3f95`)

Established by reading `.github/workflows/ci.yml`,
`.github/workflows/release-task-app.yml`, and every `mosaic-emit-*` crate for
TaskApp references.

**Gated and shipped (6 of 9).** `react` ships the tested production web bundle.
`qt`, `flutter`, `compose`, `swiftui`, and `xaml` each generate under the strict
`native-complete` profile with the real `task-mosaic-app` runtime, pass an
emitted-control contract that drives the simple-todo lifecycle, and produce a
release artifact.

**Not exercised at all (3 of 9).** `mosaic-emit-html`,
`mosaic-emit-webcomponent`, and `mosaic-emit-paint` contain **zero** references
to TaskApp — no test, no CI step, no release artifact. The portable-input-label
work in #13717 touched the html and webcomponent *emitters*, but drove them from
emitter-local fixtures, never from TaskApp's own sources. These are genuine
platform gaps, not covered work described imprecisely.

**Partial.** iOS compiles the generated SwiftUI sources against the iOS 16
deployment target. That is source portability; nothing runs the app or its
dylib on an iOS target. Android has no Mosaic backend at all — `compose` emits
Compose *Desktop*.

## Completion queue

Ordered. Tier A finishes the product on the platforms it already claims; Tier B
closes the three unexercised backends; Tier C states the reach items honestly
rather than letting them read as silent gaps.

### Tier A — finish the claimed platforms

1. **P1 [#13695](https://github.com/adhithyan15/coding-adventures/issues/13695)
   — startup loading and failure states.** The highest-severity remaining
   product defect: a failed WASM fetch, compile, or storage open leaves the root
   blank indefinitely, with no retry and no explanation. Every other queued item
   assumes the app started.
2. **P1 [#13692](https://github.com/adhithyan15/coding-adventures/issues/13692)
   — compact-window List layout.** The documented 780 px rail breakpoint is not
   implemented, so narrow windows spend scarce space on advanced navigation and
   can clip the primary capture path.
3. **P2 [#13526](https://github.com/adhithyan15/coding-adventures/issues/13526)
   — Vitest on Vite's native ESM loading.** Test-infrastructure debt in the web
   host; blocks nothing, but it is the last known non-product wart in the lane
   that gates every web change.
4. **P2 [#13625](https://github.com/adhithyan15/coding-adventures/issues/13625)
   — changelog roll-forward gate.** `CHANGELOG.md` still marks `0.1.0`
   `Unreleased` even though `task-app-v0.1.0` published on 2026-08-31. The gate
   must reject a published version that is still marked unreleased.

### Tier B — close the unexercised backends

5. **Static HTML snapshot gate.** Emit TaskApp through `mosaic-emit-html` from
   its own sources and assert the authored List-first shell — composer, task
   rows, view switcher — survives lowering. Cheapest of the three: no runtime,
   no host, no interaction claim. Its value is that it fails loudly when the
   authored shell stops lowering truthfully, which no current gate catches.
6. **Web Components host.** The only remaining *interactive* backend with no
   TaskApp presence. It needs the same treatment `react` has: emit from
   TaskApp's sources, wire the custom element to `task-wasm`, drive the
   simple-todo lifecycle through the emitted controls, and decide explicitly
   whether it earns a release artifact or is a parity gate only.
7. **Paint visual-regression gate.** Rasterize the authored shell to a
   deterministic PNG in both themes. This is the only mechanism in the stack
   that would catch a purely visual regression; every existing gate asserts
   structure, semantics, or behavior.

### Tier C — reach, stated honestly

8. **iOS: run, don't just compile.** Today's gate proves the generated SwiftUI
   sources compile for iOS 16. Promoting that to a real claim needs a simulator
   launch and a driven lifecycle against an iOS-built `task-mosaic-app`.
   Until then the README's phrasing — "source portability rather than a claim" —
   stays exactly as written.
9. **Android has no backend.** `compose` is Compose Desktop. An Android target
   is a new Mosaic backend, not a TaskApp task, and belongs to
   [#12017](https://github.com/adhithyan15/coding-adventures/issues/12017), not
   here. Recorded so its absence is a decision rather than an oversight.
10. **Signing, notarization, and installers**
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
