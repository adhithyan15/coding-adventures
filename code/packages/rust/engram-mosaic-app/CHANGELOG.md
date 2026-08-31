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
