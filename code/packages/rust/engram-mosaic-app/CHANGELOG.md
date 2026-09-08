# Changelog — engram-mosaic-app

## 0.1.0 - Unreleased

Initial release: Engram behind the standard Mosaic application ABI.

Implements `MosaicApp` over `EngramSession` and exports the standard C ABI via
`export_mosaic_app!`, so Mosaic's generated native hosts can load Engram as
`libmosaic_app` instead of binding to the bespoke `engram-capi` through a
hand-written adapter per platform.

Deliberately a thin wrapper. The facade already produces and accepts exactly what
`EngramApp.mil` declares; the adapter supplies only what the Mosaic event
envelope lacks — a selected-deck cursor and a clock — and translates the
`Event { name, payload }` envelope into the JSON object the facade parses. The
envelope's event name takes precedence over any `event` key inside a payload, so
a payload cannot redirect dispatch to a different event.

Verified: the adapter's props match the facade's **exactly** — 254 keys, the same
number `EngramApp.mil` declares — with a companion assertion that the comparison
is not vacuous. Snapshots round-trip, and foreign schemas, wrong versions, and
corrupt bytes are all refused rather than misread as Engram state. Undeclared
events are rejected rather than silently ignored.

**Event routing is gated against the MIL.** A companion test in the Engram Mosaic
package (`tests/adapter_event_contract.rs`) compiles `EngramApp.mil`, enumerates
all **88** declared emits, and asserts the adapter routes every one of them.

The distinction that makes it meaningful: dispatching a declared event against an
empty collection legitimately fails — "cannot rate without an active session",
"cannot update deck options without a deck" — and that is the domain declining an
action, not the adapter failing to route it. Only the facade's
`unknown Engram app event` marker means the event never reached Engram, which is
the failure that would leave a dead control in every generated native shell. An
earlier draft of this test conflated the two and reported 55 false failures.

A companion assertion pins the other side: an undeclared name must fail *as
unrouted*, so the parity test cannot pass by accepting everything. Verified
non-vacuous — mangling the name the adapter forwards turns all 88 unrouted.

Does not replace `engram-capi` or the hand-written host adapters; see the README
for why that is impossible at protocol v1.

### Snapshots carry the presentation cursor (#13646)

The snapshot payload was the collection alone, and `restore` cleared the
adapter's own `selected_deck_id` on the way in. Reopening Engram through a
native Mosaic host landed on the deck list every time -- the deck you had
chosen, the screen you were on, and how far into a review you were all went
away with no error to say so.

The payload is now `{state, cursor, adapterSelectedDeckId}` and
`SNAPSHOT_VERSION` is **2**. The adapter's own deck selection is included
because it lives here rather than in the facade -- the Mosaic event envelope has
no notion of a selected deck -- so nothing else is in a position to persist it.
Leaving it out would have restored the screen and the search box while still
dropping the deck, which is the most visible half.

**Version 1 snapshots still restore.** A version-1 payload is the bare
collection, which is exactly what a first launch after this change finds on
disk; refusing it would fail to open the reader's collection in order to avoid
restoring a cursor version 1 never stored. `restore` accepts
`OLDEST_SUPPORTED_SNAPSHOT_VERSION..=SNAPSHOT_VERSION` and takes the
cursor-less path for 1. A *newer* version is still refused, and has its own
test, so widening the range did not quietly become accepting anything.

The restored `adapterSelectedDeckId` is checked against the collection before it
is trusted. It takes the one path through `selected_deck_id_with_override` that
does *not* verify the id -- it arrives as the explicit `deck_id`, returned
verbatim when non-empty, where the facade's own cursor arrives as the filtered
override. That asymmetry was unreachable while the field was only ever empty;
making snapshot bytes its only writer is what put it in reach, so an id no deck
carries now falls back to empty rather than becoming the target of a later
write.

To be precise about what that did and did not fix: the *field* was unreachable
before restore became its writer, but the phantom-deck write itself was already
reachable through the collection half -- `selected_deck_id_with_override` also
returned `active_session.deck_id` unchecked, and `active_session` is part of
`AppState`, so even a version-1 payload could do it with no cursor at all. That
fallback is fixed separately, since it is a defect in its own right; this entry
covers only the field this crate owns.
