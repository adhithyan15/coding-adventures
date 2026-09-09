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
   the prescribed pattern showed it does not.
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
