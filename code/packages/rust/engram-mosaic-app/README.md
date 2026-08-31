# engram-mosaic-app

Engram behind the **standard Mosaic application ABI**.

Mosaic's generated native hosts — Qt, SwiftUI, XAML, Flutter, Compose — all speak
one small C ABI: create an app, dispatch events at it, read back props, snapshot
and restore. A crate that implements `MosaicApp` and invokes `export_mosaic_app!`
becomes the `libmosaic_app` those hosts load.

## Why this exists

Engram did not have one. It exposed `engram-capi` — a bespoke ABI of roughly
forty `eg_*` symbols — and each generated host bound to it through a hand-written
`MosaicHost` adapter shipped as a package asset.

That works, but it routes *around* `mosaic-app-capi` and `mosaic-app-runtime`.
The only thing exercising the standard substrate end to end was
`mosaic-app-conformance`, a three-slot counter. Engram drives **254 slots and 88
events** across ten component packages, two layout variants, and two themes.

It also has a mechanical consequence: the five Mosaic runtime lanes in CI build a
standard app library, bundle it, byte-compare the installed copy against the
build artifact, and launch the result. With no such library, Engram could not
enter any of them — which is why `grep -ci engram .github/workflows/ci.yml`
returned 0.

## Why it is thin

Almost nothing here is new logic. `EngramSession` already exposes the two calls
the trait needs — `engram_app_props` and `handle_engram_app_event` — and they
already produce and accept precisely what `EngramApp.mil` declares.

What the adapter genuinely adds is the two things the Mosaic envelope does not
carry: a **selected-deck cursor** and a **clock**. Both facade calls take a
`deck_id` and a `now`; an `Event` has neither.

## Native only

This is the artifact *native* hosts load. Browsers use `engram-wasm`, which
speaks its own linear-memory ABI over the same facade. That is why reading the
clock from `std::time` is fine here — the one target where it would be
unavailable never loads this library.

## What it does not do

It does **not** replace `engram-capi` or the seven hand-written host adapters,
and it cannot at protocol v1. Engram's Anki import and export return `hostIntent`
payloads so a host can open a file picker. The standard ABI's `Effect` is
serialised onto the wire, but no generated host reads it and the C header has no
effect-completion entry point, so an effect could never be answered. The two
mechanisms do not meet. This crate sits alongside the existing adapters.

## How the slot contract is pinned

Three assertions chain, each owning one link:

| Assertion | Where | Pins |
|---|---|---|
| `shared_engram_app_props_match_mosaic_slots` | `code/programs/mosaic/engram-app` | facade props == `EngramApp.mil` slots |
| `adapter_props_match_the_facade_exactly` | here | adapter props == facade props |
| `the_slot_surface_is_substantial` | here | the comparison above is not vacuous |

Together: MIL slots == facade props == what a generated native host receives.
This crate deliberately does not re-parse the `.mil` — a second hand-rolled
parser could drift from the compiler's reading of the same file.

## Tests

```bash
cargo test -p engram-mosaic-app
```
