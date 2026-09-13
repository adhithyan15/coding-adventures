# Changelog — engram-core-wasm

## Unreleased

### Fixed — a name typed after saving from the collection was silently dropped

`SaveNoteType` — the collection actions bar's save — did not reset the note-type
editor, where `NoteTypeEditorSaveNoteType` did. So the draft survived the save
holding a `draft_note_type_id` the selection no longer resolved to. The next
edit went through `note_type_editor_selected_id`, got a different id, and
`ensure_selected_draft` responded by clearing the draft: name gone, editor
snapped to the last saved model.

Reproduced before fixing, through the props the person is actually looking at
rather than the editor's internals — type "Typed After Saving" and
`note-type-editor-name-value` reads `Saved Model`.

This was **known and deferred**: #15093 found it while closing the draft-id
collision, recorded it as "strictly an improvement, and still wrong", and filed
it as an editor-behaviour decision rather than part of that fix. This is the
deferral being paid off. The entry below is left as written, because it is the
record of what was known at the time.

`SaveNoteType` resets now — **but only when the save names what the editor is
holding**, and that condition is the whole of the fix.

The first version reset unconditionally, on the reasoning that it should "match
the other event". That reasoning is wrong, and review caught it by measuring
rather than arguing. `NoteTypeEditorSaveNoteType` builds its note type from
`note_type_from_editor_selection`, so by construction it saves what the editor
holds. `SaveNoteType` resolves its target from the **payload** and never
consults the editor, so the two can name different note types — and an
unconditional reset discarded a draft of A because something saved B. For a
brand-new model that is the name, the stylesheet, every field rename and every
template body, gone at once with no confirmation: `reset()` is
`*self = Self::default()`.

Not reachable from any shell today — the editor's own button emits the
editor-level event — but `onSaveNoteType` is documented for host model editors
and aliased to `upsertNoteType`, which is exactly what a sync or an import would
call. A data-loss regression worse than the bug being fixed, latent and armed
for the first host to use the documented API.

**An unrelated save still disarms a pending delete confirmation**, which is a
separate claim and gets its own branch and its own test. Measured on the
pre-change behaviour: a confirmation armed on a note type survived an unrelated
collection mutation and the next click deleted. An armed confirmation is a claim
about a collection the person has just watched change, so it should not outlive
the change — the cost of disarming is a second click, the cost of not is a note
type.

**What replaces the old test is the equivalence, not either event's landing
spot.** The two are one operation reached from two places. Where they land is
deliberately not pinned: selecting the model just saved might well be better,
and that is a product choice neither event makes today.

The old test's `after != draft_id` was kept at first as "the property that
matters" and review showed it was not measuring that property. After the reset
there is no draft at all, so the id prop is just the selected saved model's —
always an id the collection holds. It passed only because the demo fixture's
index 0 is some *other* model; on a collection whose only note type is the one
just saved it would fail while nothing was wrong, which is precisely the
tripwire the paragraph above set out to avoid. The assertion is now the absence
of a draft, read off the delete label.

**The deleted assertion was load-bearing and is restored.** It was the only one
in the crate pinning that a collection-level save actually *creates* a note
type; with it gone, making creation a silent no-op left all tests green. `ok:
true` and a props read cannot tell a save from a no-op, because after the reset
the props show the same model either way.

Five mutations, each failing a specific test rather than reddening the suite:

| mutation | catches |
| --- | --- |
| `SaveNoteType` never resets | symptom, equivalence |
| `NoteTypeEditorSaveNoteType` never resets | equivalence **only** |
| `SaveNoteType` resets unconditionally | unrelated-draft |
| creating a new model becomes a no-op | equivalence, disarm |
| `confirm_delete` not cleared | disarm |

Row two is why the first two tests are not redundant: the symptom is about the
collection path, so a regression in the editor path must not fail it. Row four
took two attempts — the first "mutation" tested for the id *after* the upsert
had added it, so it changed nothing and reported that no test caught it. A
mutation that does not mutate is a false exoneration.

### Fixed — two notes could generate the same card id

A generated card's id is `{note_id}::{template_id}`, and the cloze form appends
`::c{ordinal}`. Neither component was checked for the separator, so the
composite key is ambiguous. Measured with the real generator rather than
argued:

```text
note "a"    + template "b::c"  ->  card id "a::b::c"
note "a::b" + template "c"     ->  card id "a::b::c"
```

Two different (note, template) pairs, one card id. Cards are keyed by id in
`AppState`, so one silently displaces the other.

**Reachable through `dispatch`**, which is documented: the crate README
describes it, the web host declares it, and `eg_dispatch` exports it to every
native shell. A caller supplies the whole `Note` — id included — and nothing
validated it. That is the same surface the deck-id family was hardened on
(#14533, #14559, #14532), so `validate_command_id_separator` is written as that
guard's sibling: a match over the command variants carrying an id that reaches
card generation, with the others exempt and said to be so.

Engram-minted ids are `note-{timestamp}` and `note-type-{timestamp}`, and
Anki's are integers, so nothing that exists today contains `::`.

**A guard, not a re-encoding.** Length-prefixing the key would fix it at the
root, but every existing card id has this shape — in saved collections, in
snapshots, in imported packages — so changing the encoding rewrites data already
on disk. Refusing the input that makes the key ambiguous costs nothing real.

**Deck names keep their `::`, and that distinction is the point.** Anki's deck
hierarchy is literally `Parent::Child` and `subdeck_name` splits on it; a guard
that swept names in with ids would break the feature it was meant to protect. A
test pins that a deck named `Parent::Child` still saves.

**Two surfaces, not one — and the first version of this fix only closed one.**
`dispatch` is the command channel; `onSaveNote` and `onSaveNoteType` are the
Mosaic event channel, and they read their ids straight from the payload and call
`reduce` directly. `eg_handle_engram_app_event` is exported to every native
shell exactly as `eg_dispatch` is, so it was not a lesser door.

Security review found that by *reproducing* the collision through the event
surface rather than reading the code: both events returned `ok: true` and two
cards with id `a::b::c` coexisted in one collection. The guard now runs on both,
through one shared `reject_card_id_separator`.

Six tests, mutation-checked twice. Removing the guard fails the refusals while
the two acceptance tests keep passing — an ordinary note id still stores, so it
is not simply rejecting everything. And removing *only* the event-surface guards
fails exactly the two event tests while the `dispatch` tests stay green, which
is why the first version looked complete when it was not.

**`LoadState` is deliberately exempt**, along with `load_snapshot` and
`import_backup`, which do not pass through the command guard at all. They
replace the collection wholesale rather than editing it, and refusing a restore
because one note in it carries an odd id would cost someone their whole
collection to avoid a misplaced card — the opposite of the trade this crate
makes elsewhere. `validate_command_deck_reference` exempts it for the same
reason. Constraining what a restore may contain is a separate question about
snapshot trust.

Two things that look like the same bug and are not, both checked against the
emitted behaviour rather than assumed: `subdeck_name`'s `rsplit_once("::")`
operates on deck names, where the separator is intended; and
`card_template_matches` parses `card.id` only as a fallback when `card.lineage`
is absent, which generated and imported cards both carry.

### Fixed — a new note-type draft could carry a saved model's id

The guard against this existed, was documented, and did nothing.

`note_type_editor_selected_note_type` filtered `draft_note_type_id` to an id no
saved note type holds before adopting it — and then fell back to
`default_note_type_model(draft_created_at)`, whose id is
`note-type-{draft_created_at}`: the same string `start_new(now)` derived the
rejected id from. **Refusing the collision assigned the collision.** The rule
held for exactly the inputs that could not violate it.

Why it matters: `default_note_type_model` is a blank two-field model. A draft
carrying a real note type's id saves that blank *over* it, discarding its fields
and templates and taking every note built on them. #15063 found the delete-side
consequence — a payload-less delete cascade-deleting the shadowed model — and
closed that path while recording the root cause here.

Both halves are fixed:

- **`start_new` no longer mints its own id.** It takes one from the new
  `unique_note_type_id`, the note-type twin of `unique_note_id`, which the note
  editor's `start_new` has always used. A fresh draft cannot collide at all.
- **The fallback mints one too**, which is what the restored-snapshot path
  needs. A snapshot can carry any `draft_note_type_id`, including one that
  already collided when it was written.

The signature change is what found the call sites: `start_new` now takes the id,
so the compiler named both callers rather than leaving one to be spotted.

The existing test that *pinned* the collision now asserts its absence, against
the same adversarial fixture — a saved note type holding exactly the id
`start_new` would have minted. It asserts against the collection rather than one
expected string, so it holds whatever suffix the helper picks. Mutation-tested:
restoring the collision fails it.

**Not fixed, and now pinned.** `SaveNoteType` (the collection-level event) does
not reset the editor, where `NoteTypeEditorSaveNoteType` does — so `draft_is_new`
stays true beside the model just written.

Before this change that left the editor showing the saved model's own id as an
unsaved blank, and a second save would have written the blank over it. That
version is gone, and the replacement is better but not free: because the draft's
id now moves when the collection moves, the next field edit resolves a different
id than `draft_note_type_id` holds, and `ensure_selected_draft` clears the draft
in response. A name typed after saving from the collection is silently dropped
and the editor snaps to the last saved note type.

That is a trade of **data loss for draft loss** — strictly an improvement, and
still wrong. Found in security review rather than by reasoning, and pinned by a
test that fails against the pre-fix behaviour. The fix is for `SaveNoteType` to
reset the editor the way its editor-level twin does, which is an
editor-behaviour decision rather than part of closing the collision, so it is
filed rather than folded in.

**Now fixed — see the entry below.** This paragraph is left as written because
it is the record of what was known and deliberately deferred; the deferral is
what was paid off, not what was wrong.

### Fixed — `onDeleteNoteType` reported success having deleted nothing

Two silent no-ops in the same match arm, both reachable from the Delete button
in the collection actions bar.

**An unsaved draft.** That button emits `onDeleteNoteType` with no payload, so
every real click takes the fallback, which resolves the target from the
note-type editor's selection. For a new draft that selection is *synthetic* —
a blank model minted by `default_note_type_model`. The fallback handed its id
to `DeleteNoteType`, whose filter matched nothing. The call returned `ok`,
changed no state, and left the draft open — the person sees the editor exactly
as it was, with no reason given.

The button is not disabled in this state: it is gated on the editor having *a*
selection, and a draft is one. So the path was fully reachable, and clicking
Delete on an unsaved note type did nothing at all.

**And on an id collision it was destructive, not merely silent.** The first
version of this entry claimed a draft's id is "filtered to one no saved note
type holds, precisely so a draft cannot overwrite a real model". That guard is
inert, and security review caught the claim. `start_new(now)` sets
`draft_note_type_id` to `note-type-{now}` and `draft_created_at` to `now`; the
fallback is `default_note_type_model(draft_created_at)`, whose id is
`note-type-{now}` — the same string. Rejecting the colliding id therefore falls
back to the identical colliding id.

When a saved note type does hold that id, the old code resolved it and the
reducer **deleted that model and cascaded to every note built on it**. Measured
rather than argued: with the new draft branch disabled,
`a_new_note_type_draft_can_collide_with_a_saved_id` ends with zero note types
instead of one.

Asking `draft_is_new` rather than resolving an id closes both cases at once,
which is why the fix is shaped that way. The inert guard itself is left to a
separate change: it also governs save, rename and field edits, which have their
own test surface.

Deleting an unsaved draft now discards it, which is what
`onNoteTypeEditorDeleteNoteType` — one arm above in the same `match` — has
always done. Both entry points now agree.

**An id nothing holds.** An explicit `noteTypeId` naming no existing model took
the same path to the same no-op `ok`. It is now refused, and the refusal names
the id. A caller could not otherwise tell a delete that worked from one that
matched nothing.

The explicit-id and draft cases stay separate: an explicit id names its target
outright and says nothing about what the editor is showing, so it is never
diverted into discarding a draft.

### Fixed -- `upsertNoteType` accepted a template deck the collection does not contain (#14532)

A `CardTemplate` carries an optional deck id, and that id **overrides the
note's** when a card is generated. So this command can put every card of a note
type into a deck that does not exist without ever naming a deck itself -- which
is why the command guard added for #14533 did not catch it, and why it takes a
*list* of ids rather than one.

Unguarded, an `upsertNoteType` whose template named `deck-that-does-not-exist`
moved all five demo cards and reported `ok: true`.

`None` on a template means "use the note's deck" and is the ordinary case; only
a `Some` naming nothing is refused. Both halves have a test, because a guard
that refused `None` would break every note type there is.

### Fixed -- `createCard` and `startSession` accepted a deck the collection does not contain (#14533)

Both reach `reduce` raw through `dispatch` -- a documented public surface: the
README describes it, the web host declares it, and `eg_dispatch` exports it to
every native shell. `reduce` returns `AppState` and cannot refuse anything, so
nothing checked the deck.

The session is the more serious of the two. `DeleteDeck` selects sessions by
`deck_id`, so one naming a deck that never existed can never be selected and
never removed -- the same permanently-undeletable shape as the importer bug in
#14559.

The check is written as a **match over every command variant that carries a deck
id**, not a list of the ones known to be broken. Each earlier fix in this family
patched the routes someone had already found, and each time there was another;
enumerating the surface is what stops that, and a new variant carrying a deck id
now has to be decided about rather than defaulted.

Two are deliberately exempt, and both are pinned by tests so the choice cannot
be widened by accident:

- `updateDeck` and `deleteDeck` mutate or remove only what they find, so neither
  can create a dangling reference. Refusing them would turn a harmless no-op
  into an error for a caller that deletes twice -- which a retry, or two hosts
  on one collection, will do.
- `setDeckOptions` creates *configuration*, not content. A preset for a deck
  that does not exist is inert, since options are read while scheduling that
  deck's cards and it has none. Such a preset is permanent -- but because
  `DeleteDeck` never runs for a deck that was never created, not because the
  cascade is incomplete: it already filters `deck_options` by `deck_id`.
  Refusing the write would change documented behaviour an existing test pins,
  for a few hundred bytes nothing reads.

### Fixed -- rebuilding a filtered deck that does not exist emptied the collection (#14531)

Rebuilding a filtered deck *moves cards out of the decks they are in* and into
the named one, and neither route that does it checked that the named deck
exists. The facade method had no deck guard at all; the event arm rejected an
**empty** id and nothing else, so a selected deck that had since been deleted
went straight through.

This is the most damaging member of the deck-id family and the only one that
needs no crafted input, because an empty search matches everything:

    ok        = true
    decks     = ["tamil-script", "hindi-devanagari", "kannada-script", "spanish-latin-roots"]
    before    = ["tamil-script", "tamil-script", "hindi-devanagari", ...]
    after     = ["deck-that-does-not-exist", ... x5]
    MOVED     = 5 of 5

Every card in the collection, out of its deck and into one that does not exist
-- absent from every queue and stat, and never reached by `DeleteDeck`'s
cascade, so unrecoverable from inside the app. It returned `ok: true`.

The asymmetry was that **emptying** a filtered deck already refused to restore a
card into a deck that is gone, while **rebuilding** did not. The two *rebuild*
routes now share `validate_filtered_deck_target` -- emptying still needs no
guard, since its restore loop only touches cards already carrying the named
deck and skips any whose original deck is missing, so a phantom id there either
no-ops or repairs an orphan.

The tests assert that no card moved, not merely that an error came back: a guard
that errored after mutating would satisfy the weaker check and still have
wrecked the collection.

The refusal is also enforced at the point of mutation, in `engram-core` -- see
that crate's entry. This layer exists to make it *visible*.

### Added -- the presentation cursor survives snapshot/restore (#13646)

`snapshot()` serialised `AppState` and nothing else, and `load_snapshot` reset
`selected_deck_id`, `browser`, `review`, `editor`, and `note_type_editor` to
`default()`. Reopening Engram silently put you back at the deck list: the deck
you had chosen, the screen you were on, the search you had typed, and how far
into a review you were all went away, with no error to say so.

The mechanical cause was that those types derived `Clone, Debug, PartialEq, Eq`
but not `Serialize`/`Deserialize`, so there was nothing to put in a snapshot even
if one had wanted to.

Now they do, and there is a `PresentationCursor` bundling them with the active
screen. Note the issue named five types; `active_screen` is a sixth. It was never
reset by `load_snapshot` -- but it was never *serialised* either, so it did not
survive a restore any more than the other five did, and leaving it out would have
restored the search box while losing the screen it belongs to.

Two new facade calls carry it:

- `session_snapshot()` -> `{"ok": true, "session": {"state": ..., "cursor": ...}}`
- `load_session_snapshot(json)`, where a missing `cursor` is not an error --
  that is what a pre-cursor snapshot looks like, and it loads with a fresh one.

The cursor is parsed separately from the collection, so a cursor that will not
parse costs a scroll position rather than the whole collection.
`#[serde(default)]` alone would not do this: it fills fields that are *missing*,
while a field that is present and malformed -- an unknown `activeScreen`, a
`null` cursor -- aborts the entire document and would take `state` with it.
Refusing to open someone's collection because their saved scroll position is
corrupt is the wrong trade, and the same one this crate already refuses when it
accepts a cursor-less snapshot.

`snapshot()` and `load_snapshot()` are **unchanged**, deliberately. `eg_snapshot`
and `eg_load_snapshot` are in the published C header and all five native hosts
bind them, and `load_snapshot` is a `engram-wasm` export besides; moving their
payload would have been a silent ABI break for every shell. They remain the
collection-only pair, which is also the right shape for a backup or a sync
payload, where a half-typed search box is noise.
### Fixed -- `onSaveNote` accepted a deck or note type the collection does not contain

**Three** routes build a `Note` from partly-external input and hand it to
`UpsertNote`: the editor draft, the `onSaveNote` event payload, and the raw
`upsertNote` command. The editor route refused a note type that does not exist,
and (since the id-hardening pass) a deck that does not exist. The other two
checked **neither**.

The third is the least guarded and the most exposed: `upsertNote` carries a
whole `Note` straight from the caller, and it is a documented public surface --
the crate README describes it, the web host declares it in its `.d.ts`, and
`eg_dispatch` exports it to every native shell.

So the invariant "a note belongs to a deck that exists" held on one path and not
the other. On the unchecked path the note *and its generated cards* land in a
deck no deck list can reach, with the editor rendering a selected-deck index of
-1, so the UI cannot say where they went.

All three routes now share `validate_note_target`. The reducer would be the tempting
home for it, since every write passes through `reduce` -- but `reduce` returns
`AppState`, not `Result`, so the only thing it could do with a bad id is drop
the note silently, trading a visible wrong answer for an invisible one. The rule
lives at the two points that can still report an error, and a test pins both so
they cannot drift apart again.

### Scope, stated so it is not read as more than it is

This closes the three `UpsertNote` routes. It is **not** a global invariant on
`AppState`. `loadState`, `load_snapshot`, `import_backup`, `merge_app_states`,
and the `.apkg`/TSV importers can all still introduce a note or card naming a
deck that does not exist, and `rebuild_filtered_deck`, `createCard`, and a
per-template `deckId` can each write one directly. Those are tracked separately
(#14531, #14532, #14533); the merge paths matter most, because they fold
untrusted file content into an existing collection rather than replacing it.

An empty deck id means "inherit the existing note's deck" for the two callers
that resolve inheritance before validating, and is **refused** for the raw
`upsertNote` command, which has no such step -- the reducer stores the `Note`
verbatim, so empty there means "belongs to no deck". Sharing one rule across
callers whose empty case means different things is how a guard becomes a hole:
it let `deckId: ""` store exactly the orphan this change prevents, and blank an
existing note's real deck besides -- but the
`onSaveNote` arm now resolves the caller's deck argument the way every sibling
arm does, instead of passing it through raw. It did not before, so a **new**
note saved with no deck named reached the inherit exemption with an empty id and
was stored with `deck_id: ""`, along with the cards generated from it: absent
from every deck's queue and stats, and not reached by `DeleteDeck`'s cascade.
The exemption is for inheriting a real deck, not for having none.

### Changed -- `onSaveNote` and `upsertNote` now require the note type to exist

Previously a note naming a not-yet-created note type was stored with zero cards,
and a later `upsertNoteType` with `materializeCardsAt` would retroactively
generate them. A host that streams notes before models will now get an error
instead. Nothing in this repository does that -- the shipped UI emits
`onNoteEditorSaveNote`, and the `.apkg` and TSV importers build `AppState`
directly without going through `UpsertNote` -- but it is a public contract
change and is recorded as one.

### Fixed -- three ways a collection could name something it does not contain

Each of these is a place where an id was used verbatim where the sibling code
path checked it, so a collection or draft naming something absent wrote to a
phantom target rather than being refused.

**A note could be saved into a deck that does not exist.**
`note_from_editor_selection` validated the note type and not the deck, though
both come from the same editor draft. The editor path upheld it on its own --
`NoteEditorSelectDeck` resolves an *index* into `state.decks` -- so nothing
reached it today, but the note *and its generated cards* would have landed in a
deck no deck list can reach, with the editor rendering a selected-deck index of
-1 so the UI could not say where they went. An empty id still means "inherit the
existing note's deck" and is left alone.

**A phantom `active_session.deck_id` resolved to itself.**
`selected_deck_id_with_override` filtered its override argument against
`state.decks` but returned `active_session.deck_id` unchecked, and
`active_session` is part of `AppState` -- so a collection naming a deleted deck
resolved to that phantom, which then became the target of writes such as
`rebuild_filtered_deck`. In-app this is a no-op, since `DeleteDeck` clears
`active_session` when it targets the deleted deck; it matters for a collection
that arrives from outside. It now falls through to the first real deck, which is
what already happens when there is no active session.

**A "new" draft could adopt an existing id and overwrite it.** `draft_is_new`
and the draft's id are set together by `start_new`, which mints a fresh id, so
they cannot disagree by any route through the event surface. Set independently,
`draftIsNew` beside an id something already holds turned "create" into
"overwrite": for a note, replacing its fields and deck; for a note type, saving
the blank two-field `default_note_type_model` over a real one, discarding its
fields and templates and breaking every note built on it. A new draft now keeps
its freshly minted id when the supplied one is taken. An id nothing holds is
still honoured, so a caller can still choose its own.

### Fixed — deleting a note or note type was a silent no-op (#13933)

The user clicked delete and nothing happened: no deletion, no error, no
message. Two halves that were individually reasonable and wrong together --
the branch that minted a `deleteNote` host intent was exactly the branch that
mutated nothing, and **no adapter has ever handled that intent**. All seven
fall through a `default:` branch.

Three changes:

- The delete resolves the **currently selected** note or note type when the
  event carries no id, through the same helper `NoteEditorDeleteNote` already
  used. The Collection panel's button emits no payload, so before this it could
  never identify anything -- the click was a no-op every single time.
- When nothing can be resolved it returns an error instead of doing nothing,
  matching `NoteEditorDeleteNote` rather than being the one destructive action
  that fails quietly.
- New `collection-delete-note-disabled` / `collection-delete-note-type-disabled`
  props, so the control is withdrawn rather than offering a click that would
  fail. Computed in the engine because it is a property of the state and every
  backend renders `disabled` already -- a host-side answer would be written
  seven times and kept in step.

The intents are no longer minted: an intent alongside an error is a request to
a host that is also a failure, and restoring one belongs to UI47's `Await`
effect, which is the real confirmation round trip.

A test asserted the old behaviour -- `ok: true`, an intent, an unchanged note
count -- which is to say it pinned the bug as expected. It now pins the fix.

- `deck-rows` now exposes per-deck due and new counts as pre-formatted strings
  (`"1 due"`, `"1 new"`, or empty when the count is zero), computed with
  `get_deck_stats_for_state`. The counting stays in the engine so all five
  backends show the same numbers without reimplementing the arithmetic; a deck
  with nothing waiting renders an empty column so the decks that do have work
  are the ones that catch the eye.
- `onSelectDeck` accepts the `index` payload the deck list now emits. Selecting
  by name still works -- the event surface is also the scripting surface, and a
  name survives a reordering where a position does not.

**APKG import and export now run on wasm.** `export_anki_apkg` and
`merge_anki_apkg` returned `"Anki APKG export/import is handled by native hosts
for WASM shells"` on `wasm32`, because `Cargo.toml` excluded
`engram-anki-package` from that target entirely — it linked bundled C SQLite and
libzstd, neither of which can target `wasm32-unknown-unknown`.

The package layer is now a dependency on every target: with default features
natively, and with `default-features = false` on wasm, which drops zstd and keeps
full legacy `.apkg` support. Both stubs are gone and both methods run the real
implementation everywhere.

This closes the one documented hole in the Mosaic host contract — the generated
HTML, WebComponent, and React hosts wired up file input and download helpers and
then had to surface a delegation error.

