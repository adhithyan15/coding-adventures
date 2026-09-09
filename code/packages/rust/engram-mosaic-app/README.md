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

## Anki import and export, as effects

Engram's import and export need a file dialog, which only the host can open. The
facade reports that need as a `hostIntent`, and this adapter turns three of them
into standard `Effect`s: `importAnki` and `exportAnki` as `Delivery::Await`,
because neither can proceed without the host and the app has to know whether it
happened; `openCard` as `Delivery::Notify`, because opening a card elsewhere is
fire-and-forget.

That became possible when the fifth generated host learned to answer effects
(UI47 §5.4 step 4). Until then the two mechanisms did not meet: `Effect` was
serialised onto the wire, no generated host read it, and the C header had no
completion entry point — so an `Await` could never be answered.

The bytes travel, not the path: an export builds the package here and sends it
out in the payload for the host to write, and an import comes back carrying what
the host read. Every native target can be sandboxed — macOS most strictly — and
there a process may open only what the user picked in the host's own dialog.

Nothing is minted below protocol 2. The runtime does not merely ignore an
`Await` from a v1 host; it fails with `EffectsRequireV2` and poisons the
instance, so the intents ride `hostIntent` to the hand-written adapters there,
exactly as before.

## What it does not do

It does **not** replace `engram-capi` or the seven hand-written host adapters.
Moving the generated hosts onto the standard runtime is #13728, and until that
lands each host still binds to `engram-capi` through this package's
`host_assets` override.

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
