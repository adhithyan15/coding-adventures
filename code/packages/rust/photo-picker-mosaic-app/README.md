# `photo-picker-mosaic-app` — the reference app for `files.open` (UI59)

One button, one status line: click "Pick a Photo", the host opens its
native file/gallery picker, and the result (name, MIME type, exact
byte count — or why it didn't happen) renders back as text. That's the
whole app. See `code/specs/UI59-files-open-effect.md` for the effect
contract this exercises, and
`code/programs/mosaic/photo-picker-app/` for the `.mil`/`.mll`/`.msl`
UI sources plus the XAML handler that actually answers the effect.

## Why a whole application for one effect

`[host_effects]` (`UI47`) only wires a *declared* handler into a
*generated* entry point — there is nothing to generate an entry point
for without a real Mosaic package. This crate is the minimum viable
one: a single event (`onPickPhoto`), a single effect (`files.open`),
and nothing else, so the round trip end to end is provable without
Engram-scale complexity in the way.

## The round trip

```
user clicks "Pick a Photo"
  │  dispatch("onPickPhoto")
  ▼
PhotoPickerApp mints an effect id, pushes
  Effect { delivery: Await, kind: "files.open", payload: { accept: [image MIME types] } }
  │
  ▼
host (XAML: PhotoPickerEffects.cs) opens its native picker,
completes the effect with ok / cancelled / failed
  │  complete_effect(id, result)
  ▼
PhotoPickerApp renders a status line:
  "Picked \"sunset.jpg\" (image/jpeg, 482113 bytes)."
```

`ACCEPT_IMAGE_TYPES` (`image/jpeg`, `image/png`, `image/webp`) is this
*app's* choice, not a property of the effect — `files.open` itself
knows nothing about images (UI59 §2); any Mosaic app can request a
different `accept` list or omit it for "any file."

**Why `"onPickPhoto"`, not `"pickPhoto"`.** `PhotoPickerApp.mil`
declares `emit onPickPhoto ;`, and every backend's generated client
sends that name — raw, unstripped — as the wire event's `name`/
`event` field. The "on" prefix is stripped only when generating the
C#/Kotlin/Dart *class or case identifier* (`PhotoPickerAppEvent
.PickPhoto`), never the wire value. `dispatch` originally checked for
the stripped name (`"pickPhoto"`) by mistake — a real bug that shipped
across all four merged backend PRs, since the wrong string was never
actually sent by any of them. It went undetected by every real build
and every unit test (the tests used the same wrong name the app
checked for) because the first time anyone actually *clicked* the
button was well after all four PRs had merged; interactively
exercising the picker dialog was explicitly out of scope for this
session's own automated verification.

## What it decodes, and why

`complete_effect`'s `ok` branch base64-decodes the `bytes` field only
to report an exact byte count — the decoded bytes themselves are never
retained past that call. A host that answers with malformed base64 or
a missing field is reported plainly ("an unreadable size", "an unknown
size") rather than panicking; a picked file's *content* is untrusted
input from the app's point of view; UI59 §6 itself flags this as worth
scrutiny for exactly that reason.

## Testing

```
cargo test -p photo-picker-mosaic-app -- --nocapture
```

9 tests: start renders the initial status, dispatch mints an `Await
files.open` effect, an unrecognised event is a no-op (including the
pre-fix wire name, `"pickPhoto"`, which must never become a quietly-
supported alias for `"onPickPhoto"`), and `complete_effect` is
exercised for `ok` (exact size reported), `cancelled` (not reported as
a failure), `failed` (message surfaced), malformed base64, and a
missing/empty `ok` payload — none of these panic, since every one of
them is data a host could plausibly send.

Every effect-bearing test overrides `protocol_version` to
`EFFECT_PROTOCOL_VERSION` on both `StartContext` and `Event` —
`Await` effects require protocol v2, but `StartContext::new`/
`Event::new` both default to v1 (kept for existing v1-only hosts).
