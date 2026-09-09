# Changelog — engram-mosaic-app

## 0.1.0 - Unreleased

### Added -- Anki import and export ride `Effect` (UI47 §5.4 step 5)

The facade reports a needed file dialog as a `hostIntent`. This adapter now
turns three of them into standard effects: `importAnki` and `exportAnki` as
`Delivery::Await`, because neither can proceed without the host and the app has
to know whether it happened, and `openCard` as `Delivery::Notify`, because
opening a card elsewhere is fire-and-forget and waiting on an answer could only
invent a way to wedge.

`complete_effect` applies the answer: an import merges the returned package, an
export records that it landed. Cancellation is a first-class outcome, not a
failure -- Escape in a file dialog means "never mind" -- and each outcome
reaches the reader as an `anki-transfer-status` prop, because the facade owns
Engram's state and knows nothing about host dialogs, so "the file you chose was
not a package" is a sentence only this crate can say.

**The bytes travel, not the path.** An export builds the package here and sends
it out in the payload for the host to write; an import comes back carrying what
the host read. Every native target can be sandboxed -- macOS most strictly --
and there a process may open only what the user picked in the host's own dialog,
so keeping all filesystem access on the host side is the one arrangement that
works on all five. The cost is that a cancelled export did work nobody used,
which is cheap and recoverable; the alternative fails outright on the platform
Engram most needs to ship to.

Unblocked by the fifth generated host learning to answer effects (step 4).
Before that, `Effect` was serialised onto the wire, no generated host read it,
and the C header had no completion entry point -- so an `Await` could never be
answered and emitting one would have left the app waiting forever.

### Guarded -- nothing is minted below protocol 2

The runtime does not merely ignore an `Await` from a v1 host: it fails the call
with `EffectsRequireV2` and **poisons the instance**. Engram would therefore be
bricked by its first import rather than degraded. Below v2 no effect is minted
at all and the intents ride `hostIntent` to the hand-written adapters exactly as
they did before, which is a working import rather than a broken one.

Intents other than those three are likewise never minted as effects: an `Await`
nothing answers wedges snapshot and restore for the life of the process, which
is the exact failure the hosts' sweeps exist to prevent.

Ids are minted monotonically, never reused, and bounded at 2^53-1 -- the id
rides a JSON number to hosts whose only integer is a double, and one that
arrived rounded would answer a *different* effect. An answered id is removed
before the answer is applied, so a stray second answer cannot merge the same
package twice.

### Hardened -- the import path treats package bytes as untrusted

They are: the bytes come from a file the reader was handed, so everything below
assumes whoever produced it meant harm.

- **The encoded payload is capped before it is decoded**, so the gate is a
  string length rather than the allocation it prevents. The encoded string, the
  JSON value holding it and the decoded bytes coexist at roughly three times the
  file's size.
- **Replies are no longer materialised as `Value`.** `merge_anki_apkg` answers
  with the entire post-merge collection, media included as base64, and
  `facade_error` was parsing all of it into a tree with a node per note field,
  per card and per tag -- to read one boolean. Narrow deserialisers walk the
  document and keep only what is consulted. The dispatch reply gets the same
  treatment, since it carried the whole collection and the whole prop set past
  a `Value` parse on *every* event.
- **Text from outside the process is trimmed before it reaches a reader.**
  Package-layer errors interpolate names lifted out of the archive -- a zip
  entry name is up to 65535 arbitrary bytes -- and host failure messages are
  whatever the host wrote. Both land in a prop rendered by five native toolkits,
  one of which interprets markup, so control characters go and the length is cut
  at the boundary rather than trusted to five renderers.
- **The id bound is the runtime's own constant**, now `pub`, rather than a
  restated literal. A divergence would mint ids the runtime rejects, and it
  poisons the instance for one out of range.

Mutation-tested: making `importAnki` a `Notify` fails
`import_is_an_awaited_effect`, and letting an answered id survive its answer
fails `an_answer_cannot_be_applied_twice` along with four other cases.

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
