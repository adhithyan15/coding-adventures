# Mosaic product program v1 — one component tree, four products

**Tracked by:** [#14415](https://github.com/adhithyan15/coding-adventures/issues/14415)
**Status:** Program specification. Consolidates three parallel streams and adds a fourth product.
**Supersedes the working method of:** `mosaic-component-program-v1.md` (#14011)
**Absorbs the coordination of:** #14267 (VisiCalc), #13624 (Engram)
**Retires as a standalone app:** the Checklist app, folded into Trestle

---

## 1. Why consolidate

Three programs are currently driving Mosaic capability work from three
different applications, each discovering kernel gaps independently:

- **#14011** — the component program, driven from TaskApp/Trestle.
- **#14267** — VisiCalc as a polished reference application, with its own
  authorized delivery loop.
- **#13624** — releasing Engram on the Mosaic + Rust core stack.

All three use the same discovery mechanism: build a product, hit a platform
limitation, specify the fix. That mechanism works — it produced UI48, UI49, and
the grammar-regeneration repair. What it does not do is share the result.

### The evidence

There are **21 Mosaic packages** on `main`. Twelve of the richest —
`card`, `card-browser`, `collection-actions`, `deck-options`, `deck-stats`,
`note-editor`, `note-type-editor`, `rating-controls`, `review-actions`,
`review-card`, `review-history`, `session-progress` — are used by
**`engram-app` and nothing else**. `rating-controls` is used by nothing at all.

Engram built a card component, a note editor, a review card, and a session
progress indicator. Journal needs a card and an editor. Trestle needs notes and
a card for its board. None of them reuse any of it.

The library already exists. Nobody is treating it as one.

---

## 2. The four products

| Product | State today | Goal |
| --- | --- | --- |
| **Trestle** (`task-app`) | Mosaic-native; 166 hand-styled parts; paused behind #14011 | Tasks **and checklists**, rebuilt on the tree |
| **Journal** (`journal-app`) | React + Flux + Electron, 4 components, never released | Replace Day One |
| **VisiCalc** (`visicalc`) | Partially Mosaic — 3 `.mil`, behavior duplicated per host | Polished reference spreadsheet (#14267) |
| **Engram** (`engram-app`) | Partially Mosaic — 1 `.mil` app over 12 packages | Full Anki Desktop parity (#13624) |

### Checklist is a feature, not a product

`code/programs/typescript/checklist-app` was written as a React app before
Mosaic existed. Its decision-tree model — where a yes/no answer reveals one
branch and hides the rest — is genuinely reusable, and it belongs **inside
Trestle** rather than beside it: a checklist is a task list whose items can
branch, and a user who has both wants one place to look, one store, and one
search.

This replaces the earlier plan (#14027, #14018) of shipping Checklist as its
own reference application. The components are the same; the destination is not.

---

## 3. One component tree

Every product draws from one library and contributes back to it. A component
built for one product is built as if the other three will use it, because the
purpose of this consolidation is that they will.

### Shared demand decides build order

With one product, "what to build next" is guesswork. With four, it is
measurable: **build what the most products need, first.** A component two
products need is a component whose interface has been checked twice.

| Component | Trestle | Journal | VisiCalc | Engram | Demand |
| --- | :-: | :-: | :-: | :-: | :-: |
| `HostNavigationSplit` (missing primitive) | ● | ● | ● | ● | **4** |
| `SegmentedControl` (missing) | ● | ● | ● | ● | **4** |
| `EmptyState` (missing) | ● | ● | ● | ● | **4** |
| Toolkit atoms — Button, Input, Field, Badge, Alert | ● | ● | ● | ● | **4** (shipped) |
| Editor (markdown / rich text) | ● notes | ● core | | ● | 3 |
| `Card` | ● board | ● entry | | ● | 3 (exists, Engram-only) |
| Calendar / date navigation | ● | ● | | ● scheduling | 3 (exists) |
| List / ListGroup | ● | ● | | ● | 3 |
| Grid / Sheet | ● | | ● core | | 2 (exists) |
| Timeline | ● gantt | ● entries | | | 2 |
| Checklist / DecisionNode | ● | | | | 1 |

The three items with demand 4 are exactly the three this program should build
first — and two of them (`SegmentedControl`, `EmptyState`) were already
identified from TaskApp alone, which is corroboration rather than coincidence.

---

## 4. What happens to the existing streams

Nothing is cancelled. The three epics keep their product goals and lose their
independent capability-discovery authority.

- **#14011** keeps the method — leaf to root, isolation-first, MosaicBook-gated,
  demo app per component — and becomes this program's component track.
- **#14267** keeps VisiCalc's product bar and its delivery loop. Kernel gaps it
  finds are filed against the shared tree, not solved inside VisiCalc.
- **#13624** keeps Engram's parity goal. Its twelve packages are re-examined for
  generality as other products adopt them (§5).
- **#14027** (Checklist as reference app one) is **closed**: Checklist is no
  longer an app. **#14018** is rewritten as "fold Checklist into Trestle".

**One rule makes consolidation real:** a capability gap found by any product is
specified once, against the tree, and every product waits for that one fix.
Four apps discovering the same missing primitive four times is the failure this
document exists to prevent.

---

## 5. Generalizing what Engram already built

Twelve packages exist with one consumer each. As a second product adopts one,
its interface gets its first real test — and the likely finding is that some
are Engram-shaped rather than general (`review-card` and `rating-controls` name
spaced-repetition concepts; `card` may or may not be a generic surface).

The rule: **do not generalize speculatively.** Adopt the package as-is, record
where it does not fit, and generalize against that evidence. A package rewritten
for an imagined second consumer usually fits neither.

---

## 6. Journal, and the Day One bar

Journal is the least complete of the four — four components, never released —
and it has the most concrete product target: **replace Day One.**

That bar is worth stating, because "a markdown editor with a preview" is
already built and is not it. Day One provides, at minimum: dated entries with
rich text and photos, multiple journals, tags, full-text search, calendar and
timeline navigation, on-this-day recall, export, and encrypted local storage.

The current app has entry editing, GFM preview, and a date-grouped timeline. The
gap is not styling; it is media, search, multiple journals, and recall.

None of that should be built before the components it needs exist. Journal's
first contribution to this program is demand — it is the second product asking
for an editor, a card, and a calendar, which is what promotes those from
Engram-shaped to general.

---

## 6.5 Showcasing every component — a standing requirement

Two obligations that hold for the life of this program, not one-time tasks.

**A documentation site with a landing page for every component, and a page per
component** ([#14026](https://github.com/adhithyan15/coding-adventures/issues/14026)). Generated from the packages, never
hand-written, and **republished as soon as a new component ships** — the same
continuous shape as `deploy-task-app.yml`, which republishes Trestle on every
merge that touches it. A hand-maintained catalog drifts the moment a component
lands, and a stale catalog is worse than none: it looks authoritative and is
wrong. Each page carries what the component is, the primitives it composes, its
declared slots and their closed `one-of` value sets, the backends it lowers to
with per-backend degradations, **its live web build running inline**, and links
to its downloads.

**A demo app per component, on every backend Mosaic supports**
([#14015](https://github.com/adhithyan15/coding-adventures/issues/14015)). Not one bundled showcase per package, and not web-only:
each component is shown in isolation, on every platform, because a component
proven only inside a larger app has not been proven — §7's rule applied to
what people can actually see and run.

Both are **generated**. At roughly six artifacts per component across 23
components today, hand-writing does not scale past a handful, and hand-written
material drifts from the component the moment it changes. The release lane
reuses the build tool's existing diff-based change detection so one component's
change releases one component.

Both depend on [#14459](https://github.com/adhithyan15/coding-adventures/issues/14459): a page or a demo showing every variant needs
fixtures to actually reach the compiler. Until that lands they can show a
component's default state only, which is worth shipping early and worth being
honest about.

---

## 7. What completion means, per product

Unchanged from `mosaic-component-program-v1.md` §7 for components. Per product:

- Assembled only from landed, released components — no bespoke parts that
  should have been shared.
- Runs natively on every platform Mosaic supports, gated in CI.
- Its data survives upgrade, backup, and restore.
- Released with artifacts a person can download and run.

An app may be an empty screen for many iterations. That remains explicitly
acceptable, and it is the reason the component tree comes first.
