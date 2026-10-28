# Journal component demand audit (J1)

Issue: [#14416](https://github.com/adhithyan15/coding-adventures/issues/14416) ·
Parent: [#14415](https://github.com/adhithyan15/coding-adventures/issues/14415)

J1 of the Journal arc. This maps every Journal screen onto the Mosaic packages
that exist today and records **precisely where each fails to fit**. It is
evidence for generalizing those packages, not a rewrite plan, and it deliberately
proposes no new components until the gaps are stated.

## What Journal is today

`code/programs/typescript/journal-app` — React + Flux + Vite + Electron, storing
to IndexedDB, rendering GFM through `@coding-adventures/gfm`. Entirely outside
the Mosaic stack. Four components and four routes:

| Route | Screen | Component |
| --- | --- | --- |
| `#/` | entries grouped by calendar date | `Timeline` → `EntryCard` |
| `#/entry/new` | split-pane editor with live preview | `EntryEditor` |
| `#/entry/:id` | rendered markdown, read-only | `EntryView` |
| `#/entry/:id/edit` | the same editor, pre-populated | `EntryEditor` |

The data model is one flat `Entry { id, title, content, createdAt, updatedAt }`.
`createdAt` is a date string, deliberately, because an entry belongs to a
calendar date rather than an instant. There are no tags, no media, no search, and
no notion of more than one journal.

## Screen-by-screen mapping

### 1. Timeline — entries grouped by date

Closest existing components: `ListGroup` (toolkit), `Card`.

`ListGroup` declares:

```
slot items : list<text> ;
slot selected-index : number ;
emit onSelect ( index : number ) ;
```

**Where it fails.** A timeline row is not a string. It carries a title, a date, an
excerpt, and a target. `list<text>` can encode one of those. The screen also
groups rows under date headings, and `ListGroup` has no grouping — a group is a
second dimension the slot type cannot express.

`Grid`'s `list<list<text>>` can express rows-of-columns, and `Calendar` already
uses that shape for cells and events. That is the nearest existing idiom for
"rows with several fields," and it is worth deciding whether a list component
should adopt it rather than inventing a third shape.

### 2. EntryCard — one entry in the timeline

Closest existing component: `Card` (`mosaic-pkg-card`).

`Card` declares exactly three slots and **no events**:

```
slot title  : text ;
slot body   : text ;
slot footer : text ;
```

**Where it fails.** One thing, and it is smaller than the first version of this
audit claimed.

Journal's card shows a date, which has to go in `footer` alongside whatever else
the footer is for. A `date`/`meta` slot would fit better.

**Correction.** This audit originally led with "`Card` emits nothing, so it
cannot be activated," and called it the clearest piece of demand evidence. That
was wrong. It reasoned from `Card`'s interface without testing the alternative
`Card.mil` itself prescribes:

> Cards are display surfaces. Wrapping interaction (click to open a detail
> view, etc.) belongs to the host's parent component … by placing the Card
> inside an actionable container.

That pattern works. Compiling a probe that wraps the real `mosaic-pkg-card`
Card in a `HostButton`:

```
HostButton [ card-action ] ( onClick : emit: onOpen ) {
  pkg::mosaic-pkg-card::Card ( title : slot: entry-title , ... )
}
```

emits a real button with the card inside it and the handler on the button, on
**seven of eight backends** — html, webcomponent, react, SwiftUI, Qt, Flutter,
and Compose. `Card` needs no `emit`, and adding one would have broken a
deliberate design decision to solve a problem that did not exist.

What the test *did* find is a real defect, filed as
[#14717](https://github.com/adhithyan15/coding-adventures/issues/14717): **XAML
silently drops children nested inside `HostButton`**, emitting a self-closing
`<Button/>` with the entire card subtree discarded and no diagnostic. So the
prescribed pattern produces a working card everywhere and an empty button on
Windows.

**Follow-up (J3a, RecordList).** Nesting *primitive* children in a
`HostButton` does not share that result. A `Column` of `Text` nested in a
`HostButton` emits an **empty** button on seven of eight backends (React,
WebComponent, HTML, SwiftUI, Compose, Flutter and Qt; only XAML keeps the
children), and the native-complete degradation report stays clean. The probe
above nested a *package component*, which this does not re-test. The practical
consequence is the same either way: a row cannot yet be one large button
around several fields, so `RecordList` makes its title the button instead.
The silent drop is tracked in the Mosaic program backlog.

### 3. EntryEditor — split pane, markdown left, live preview right

Closest existing component: `NoteEditor` (`mosaic-pkg-note-editor`).

**Where it fails.** `NoteEditor` is Anki-shaped, not editor-shaped. Its
twenty-plus slots are `note-id`, `note-type-names`,
`selected-note-type-index`, `deck-names`, `selected-deck-index`,
`field-labels`, `selected-field-index`, `tags-value`. Journal needs a title, a
body, and a preview. Almost nothing transfers except the general idea of a
labelled multi-line field.

This is the honest reading: **`NoteEditor` is an Anki note form that happens to
contain an editor, not an editor.** Journal is the second consumer that reveals
it, exactly as #14415 §5 predicted, and the extraction it argues for is the
plain editing surface underneath — not a generalization of the note form.

The editing primitive itself is **not** missing: `Input` lowers to `<textarea>`
when `multiline: true`, and UI25 already calls that the portable multiline
editing contract.

### 4. EntryView — rendered markdown

**Nothing existing fits, and the reason is a deliberate policy rather than a gap
in the catalogue.**

Text slots are HTML-escaped on the way out. In the emitted html runtime,
ordinary slots interpolate through `escapeHtml`, so markdown-rendered HTML
assigned to a `text` slot would display as literal angle brackets. That is
correct: slot values are application data, and interpolating them as markup
would make every entry a script-injection sink.

The mechanism that does exist is the **`node` slot**, which emits a triple
mustache and is documented as host-owned markup that must stay unescaped.
`VentureChrome` already uses one (`slot content-surface : node`) for exactly
this purpose — a host-owned content surface mounted inside a Mosaic shell.

So Journal's rendered-entry surface has a real path, and it is worth stating
its cost plainly: **content inside a `node` slot is not Mosaic-rendered.** It
does not participate in mosstyle, it does not lower per backend, and each host
must supply it. For a markdown preview that is the right trade — the markdown
pipeline is the renderer — but it means the preview cannot be styled from
`.msl`, and every native host needs its own markdown surface. That is a
significant, previously unstated constraint on J3.

## What is missing outright

Day One parity items from #14416 with no current home:

| Need | Nearest thing | Gap |
| --- | --- | --- |
| photos in entries | `Image` primitive lowers to `<img src>` | The primitive exists, but **no component declares an `image` slot** — repo-wide, zero. Journal would be the first consumer of a typed slot that is otherwise only a type. |
| tags | `Badge` renders one; `Input` accepts a string | No tag-input component: entry, removal, and completion are all absent. `NoteEditor` carries tags as one `tags-value : text`, which is the flat form Journal would outgrow immediately. |
| full-text search | `Input` | No results surface. Depends on the same rows-with-fields shape the Timeline needs. |
| multiple journals | `ProjectNav` (Trestle) | Worth checking against Trestle's project switcher before inventing anything; the shapes may already agree. |
| on-this-day | `Calendar` | `Calendar` navigates months and carries `calendar-events` as `list<list<text>>`. Closest existing fit of anything in this audit. |
| export, encryption | — | Not UI problems. They belong to J2's Rust core. |

## Findings, ordered by how much evidence they carry

1. **XAML drops children nested inside `HostButton`** ([#14717](https://github.com/adhithyan15/coding-adventures/issues/14717)).
   The composition pattern `Card` documents for interaction works on seven
   backends and silently produces an empty button on the eighth. This replaces
   the original first finding, which claimed `Card` needed an `emit`; testing
   the prescribed pattern showed it does not. *Superseded (2026-10-26):* XAML
   is fixed, and the pattern now loses the card on six other backends (EM-8).
   See "Card as the timeline row" below.
2. **There is no list-of-records component.** Timeline, search results, and a
   tag browser all want rows with several fields and optional grouping.
   `ListGroup`'s `list<text>` cannot express it; `Grid`/`Calendar`'s
   `list<list<text>>` can. This wants one decision, not three components.
3. **`NoteEditor` is a note form, not an editor.** The extraction Journal argues
   for is the editing surface beneath it.
4. **Rendered rich text is a `node` slot, with real costs.** Not a missing
   component — a documented escape hatch whose consequences for styling and for
   native hosts need stating before J3 commits to it.
5. **`image` is a slot type with no consumer.** Journal's photos would be the
   first, so the type is unproven end to end.

## Card as the timeline row — the first real consumer (2026-10-26)

Mosaic backlog item 4 asked for Journal's timeline rows to be drawn by
`mosaic-pkg-card`'s `Card`, which has no consumer at all: Engram depends on
`mosaic-pkg-card-browser`, not on `Card`. Product program §5 says to adopt a
package as-is, record where it does not fit, and generalize only against
that evidence. This section is that record.

**Outcome: adopted.** Journal's timeline rows are `Card`s. The work was
first designed, prototyped and compiled for every backend on 2026-10-26, and
held back by M7 below until the pane could scroll (JO-5). `Card` itself is
unchanged: every misfit below is still open.

### What was measured first

Before choosing a design, three probes were compiled through
`build_package` on every backend, and the emitted source was searched for
the Card's content.

| Probe | Card content emitted on | Card content missing on |
| --- | --- | --- |
| `Card` inside a `For`, not in a button | all eight: React, WebComponent, HTML, XAML, Compose, Qt, Flutter, SwiftUI | — |
| `HostButton ( onClick ) { Card (…) }`, no `label`, inside a `For` | HTML, XAML | React, WebComponent, SwiftUI, Qt, Flutter, Compose |
| the same button outside any `For` | HTML, XAML | the same six |

On the four gated native backends (SwiftUI, Qt, Flutter, Compose) the
native-complete report names the loss as
`composition.button-children-unimplemented` (#15921). React and WebComponent
emit an empty `<button>`.

**Correction to §2.** §2 above says the composition `Card.mil` prescribes,
a `Card` wrapped in a `HostButton`, "emits a real button with the card
inside it … on seven of eight backends". That no longer reproduces. With
the current emitters the card survives on two of eight, which agrees with
UI29 §2.1 ("only XAML and HTML render them") and with EM-8. XAML is one of
the two because #14717 was fixed. §2 is kept as written, for the same reason
its earlier correction was.

### The design (adopted)

The kernel rule for this case is UI29 §2.1: "until children pass-through is
specified for `HostButton`, give the button a `label` and put richer content
beside it". So each row is a `Card` followed by a labelled button:

```text
For ( each: slot: timeline-rows , as: row )
  If ( row[1] )               Text [ timeline-day ]  (a11y-role: heading)
  If ( row[0] == selectedKey )  <entry, "-selected" parts, selected: true>
  Else                          <entry>

entry = Column [ timeline-entry ]
          pkg::mosaic-pkg-card::Card ( title: row[2], body-text: row[3], footer: row[4] )
          Row [ timeline-entry-actions ]
            HostButton [ timeline-entry-open ] ( label: "Open", a11y-label: row[6],
                                                 selected, onClick: onSelectEntry )
            If ( row[5] ) Text [ timeline-entry-badge ]
```

| Journal field | Row position | Goes to |
| --- | --- | --- |
| key | `row[0]` | selection only (`== selected-key`) |
| day heading | `row[1]` | Journal's own heading `Text`, above the card |
| title | `row[2]` | `Card.title` |
| excerpt, or search snippet | `row[3]` | `Card.body-text` |
| tags, or the short day while searching | `row[4]` | `Card.footer` |
| `★` | `row[5]` | a `Text` beside the button. `Card` has no place for it |
| "Open {title}" | `row[6]`, **new** | the button's accessible name |

Why each choice:

- **The button is labelled "Open", not with the title.** A button labelled
  with the title, above or beside the card, would draw the title twice.
  Passing `Card` an empty title instead leaves an empty region, and leaves a
  blank box for an untitled, untagged entry.
- **The button is named "Open {title}".** Each row is still one tab stop, and
  its name is still the entry's title, as it was with `RecordList`. The name
  contains the visible word "Open", so it meets WCAG 2.5.3 (label in name).
  Layout expressions cannot join strings (UI29 §3.3), so the name is a row
  field. TaskApp does the same with `a11y-label : ( row[16] )`. Rows are
  append-only, so `RecordList`, which reads `row[0]`–`row[5]`, ignores the
  new field.
- **Selected rows get their own parts**, as in `RecordList`. Every backend
  can then style the open entry without native button-state support, and
  `selected` still reports the state to assistive technology (UI86).
- **Only the timeline moves.** That includes search results, which share
  `timeline-rows`. "On this day" stays a `RecordList`. Moving it too would
  double the Card mounts for a workaround that EM-8 retires, and would leave
  Journal with no `RecordList` at all. Until EM-8, the pane therefore has two
  row styles: recalled entries open from their title, timeline entries from
  "Open".

**Per backend, as measured.**

- The Card renders on all eight backends, and the package stays
  native-complete on the five native ones (zero capability degradations).
- Card's own stylesheet adds style drops (M2). `journal-app`'s gate pins
  them, matched only on Card's parts.
- What passed locally:
  - `journal-app`'s `cargo test`;
  - `journal-mosaic-app`'s tests and its wasm test;
  - the web host's 19 Vitest tests, `tsc -b` and `vite build`;
  - the SwiftUI emit and `swift build`;
  - the Flutter emit and `flutter analyze`.
- The native UI tests find a row by `timeline-entry-open` and its
  accessible name (the Compose `contentDescription`, the Flutter
  `Semantics` label, the XCUITest `label`), where they used
  `record-list-title` and its text.
- Compose's `JournalUiTest` failed on the prototype (M7). It passes now that
  the pane scrolls and the test scrolls to each row before asserting it.

### Misfits — where Card does not fit Journal

Each misfit is evidence for generalizing `Card`, or for the platform under
it. None was fixed here.

- **M1. Card cannot be the activation target.** `Card` deliberately has no
  emits, and its `.mil` says interaction belongs to "an actionable
  container". On six of eight backends that container drops the card (see
  above). That is a kernel gap (EM-8, #15921), not a gap in `Card`. The
  workaround is a labelled button beside the card, plus an
  `open-label` row field. **Retire both when EM-8 lands:** the row becomes one button wrapping
  the card, and "On this day" can follow.
- **M2. Card's stylesheet does not survive native lowering.** These drops were
  measured by `journal-app`'s native gate, run on the prototype in both
  themes:

  | Property | Card part | Dropped on |
  | --- | --- | --- |
  | `border-style` | `card-root` | SwiftUI, Qt, Flutter, XAML |
  | `padding-bottom` | `card-title`, `card-body` | Qt, Flutter |
  | `font-size` | `card-title`, `card-body`, `card-footer` | Flutter |
  | `font-weight` | `card-title` | Flutter |

  So on Flutter a card's title is neither bold nor larger than its body. Two
  causes: Card styles its text through the `Box` around each `Text` (the
  `.mll` header explains why), and it sets `border-style: solid`, which most
  native backends do not lower. Nobody saw this before because `Card` has no
  native gate: its only test compiles the three source files.
- **M3. No optional regions.** `body-text` and `footer` are always laid out.
  `RecordList` leaves out an empty field. An untitled entry has no excerpt,
  because its title came from the body, and an untagged entry has no meta.
  Their empty regions keep their box, and on the web `card-body` keeps its
  8px bottom padding. Whether an empty `Text` also takes a line is
  backend-dependent, and was not measured.
- **M4. No slot for a marker or badge.** The `★` has nowhere to go inside the
  card, so it sits beside the button. The audit's earlier "a `date`/`meta`
  slot would fit better" (§2) holds in the other direction too: `footer` is
  carrying tags or a date, so a status marker has no room.
- **M5. No selected state.** The open entry is shown by Journal's wrapper
  around the card. The card's own surface does not change. A `selected` or
  `variant` enum slot (UI49) is the existing mechanism for this.
- **M6. The palette is not Journal's, and a caller cannot change it.** Card's
  dark surface is `#2d2d30` with a `#3f3f46` edge (VS Code's), on Journal's
  `#111827` pane, and its corners are square where Journal's are 4px. A call
  site cannot restyle a dependency's parts (X-5), so the fix is platform-neutral
  tokens (UI90), not per-app overrides.

- **M7. Too tall for a pane that could not scroll. This one blocked
  adoption, and is resolved.** The measurements are from Compose renders of
  Journal's declared 1100×760 window (`JournalScreenshots`), before and
  after the change:
  - the pane's controls end about 500px down, which leaves about 260px for
    the timeline;
  - a `RecordList` row with a two-line excerpt is about 124px;
  - the same entry as a Card, plus its Open button's line, is about 220px.
    The card's 12px padding and 8px gaps are fixed (M6), its empty footer
    still takes space (M3), and the button needs a line of its own (M1);
  - so the window shows two entries with `RecordList` and one with Cards.

  At the time nothing scrolled the pane on any native backend: there was no
  `HostScroll` in Journal's layout. So the second entry could not be reached
  except through search, and the prototype's `JournalUiTest.kt:121` failed,
  because "Open Drafted on Compose" existed but was not displayed. This was
  not Card's fault alone. The `RecordList` timeline already lost its third
  entry the same way, which is the bug TaskApp fixed with `HostScroll`
  (#14734). But Card made it happen at two entries. **How it was resolved:**
  1. JO-5 (#18364) wrapped the pane in `HostScroll [ timeline-scroll ]`, so
     it scrolls on every native backend, and added reachability tests on
     Compose and Flutter.
  2. The Cards then landed as prototyped. Compose's `JournalUiTest` scrolls
     the pane to each row with the `ScrollBy` semantics action before it
     asserts the row is displayed, because `performScrollTo()` hangs
     (EM-11).
  3. Still open: EM-8, or a compact Card, would win the density back.

What fits: the three text regions are what a journal entry card needs, and
their order (title, excerpt, metadata) is Day One's. Having no emits was the
right call. The problem is only that the container `Card` relies on is not
portable yet.

## What this does not conclude

It does not propose new components, change any package, or decide the Journal
data model. J2 (the Rust core) is unaffected by everything here: the findings
are about presentation, and the engine question is separate.

One finding in the first version of this document was wrong and is corrected
above. The lesson is recorded rather than quietly edited out: an audit that
reads a component's interface and stops has not tested what the component's own
documentation tells you to do instead. Compiling the prescribed alternative
took one probe component and replaced a wrong headline finding with a real bug.

It also does not claim the mapping is complete. Journal has four screens today
and Day One has considerably more; a second pass belongs after J3, when the
shell exists and the missing screens are real rather than hypothetical.
