# UI47 — Host capability effects: giving `Effect` a completion path

**Status:** Protocol core implemented (#14547); capability handlers and native host migrations remain
**Layer:** UI / standard Mosaic app ABI
**Depends on:** UI31 (host table), `mosaic-app-runtime`, `mosaic-app-capi`
**Decides:** #13645. **Unblocks:** #13728, and through it #13640.

---

## 1. The question this settles

Engram's Anki import and export work through a `hostIntent` convention that
the standard Mosaic app ABI **cannot express**. Until that is resolved, the
seven hand-written `MosaicHost` adapters cannot be retired, `engram-mosaic-app`
sits permanently beside them rather than replacing them, and no CI lane can
verify Engram exercises the standard runtime — because the app it emits does
not.

Two directions were open:

- **(A)** A protocol-v2 effect channel with a completion entry point.
- **(B)** A permanent two-track story: the standard ABI drives UI state, and a
  host asset owns capability requests forever.

**This spec chooses (A).** Section 6 says why, including what is genuinely
worse about it.

---

## 2. What is actually there today

This section records the pre-implementation investigation. The runtime, C ABI,
WASM bridge and conformance fixture now implement the channel described in §5;
file handlers and native host migrations remain separate work. Two of the
issue's characterisations turned out to be understated; §4 covers those.

### 2.1 `hostIntent` — the working mechanism

Minted in exactly one function, `host_intent_for_event`
(`engram-core-wasm/src/lib.rs`), and returned as a top-level field beside
`props`. There is no `HostIntent` Rust type — it is ad-hoc
`serde_json::Value`. Five kinds are produced: `importAnki`, `exportAnki`,
`openCard`, `deleteNote`, `deleteNoteType`.

The Mosaic emitters already normalise it **generically**: Qt's generated root
stashes `response.hostIntent` into `property var lastHostIntent`, SwiftUI into
`@Published var lastHostIntent`, and Flutter, XAML, HTML and WebComponent
likewise. So the emitters *capture* it and never *act* on it. Acting is
delegated entirely to the hand-written `MosaicHost` asset — which is why the
`[host_assets]` override exists and why deleting it breaks Anki import on every
native host.

### 2.2 `Effect` — the standard mechanism, unused

`Effect { id, kind, payload }` lives in `mosaic-app-runtime` and rides
`AppUpdate.effects` onto the wire. It is fully generic and could carry
`importAnki` perfectly well. But:

- **No app populates it.** `task-mosaic-app` constructs zero. The only
  construction in the repository is a unit test in `mosaic-app-runtime` itself.
  `engram-mosaic-app` mentions `Effect` only in doc links explaining why it
  cannot use it.
- **No host reads it.** A case-insensitive search across `mosaic-app-capi`,
  `mosaic-app-bindings`, `mosaic-app-conformance`, `mosaic-driver`, and the
  emitters returns only false positives: QtQuick's visual `MultiEffect`, the
  words "effective" and "effectively" in prose, and React's `useEffect` hook.
  Zero genuine consumers.
- **A host could not answer one if it did read it.** The C header exposes six
  symbols — `mosaic_app_create`, `_dispatch`, `_snapshot`, `_restore`,
  `_destroy`, and `mosaic_buffer_free`. There is no effect-completion entry
  point, so there is no way to hand a result back.

Effects are therefore serialised onto the wire and dropped on the floor. The
type exists; the channel does not.

---

## 3. Why the two mechanisms cannot meet at v1

An Engram app on the standard ABI would emit `importAnki` into
`AppUpdate.effects`, and nothing would open a file dialog or hand bytes back.
This is not an Engram-side porting problem. It is a missing half of the
protocol: **a request channel with no response channel is not a channel.**

`dispatch` cannot serve as the response path either. It takes an *event*, which
is a user-originated input; threading an effect result through it would require
every app to invent a private convention for "this event is actually the answer
to effect 7" — which is precisely the ad-hoc situation `hostIntent` already
represents, relocated.

---

## 4. Two findings that sharpen the problem

### 4.1 The five intent kinds are not one category

Grouping them as "five intents, two handled" obscures the actual shape. They
divide into three kinds by what the app expects back:

| Kind | Shape | Expects a result? | Handled today |
|---|---|---|---|
| `importAnki` | capability request | **yes** — file bytes | all 7 adapters |
| `exportAnki` | capability request | **yes** — a destination | all 7 adapters |
| `deleteNote` | confirm / disambiguate | **yes** — which note, or cancel | **none** |
| `deleteNoteType` | confirm / disambiguate | **yes** — which type, or cancel | **none** |
| `openCard` | notification | **no** — carries data outward | **none** |

A protocol that only models request/response would leave `openCard` awkwardly
shaped as a request nobody answers. A protocol that only models notifications
cannot express the other four at all. **Both shapes are needed**, and
distinguishing them is a design requirement, not a refinement.

### 4.2 `deleteNote` is a broken user flow, not dead surface

This is worse than the issue recorded, and it is the strongest argument
against direction (B).

`host_intent_for_event` mints `deleteNote` **only when the event carries no
explicit note id** — when the id *is* present it returns `None` and the
reducer deletes directly. The intent exists precisely to ask the host *which
note, or confirm*.

No host answers it. And the state path mirrors the guard:

```rust
EngramAppEvent::DeleteNote => {
    if let Some(note_id) = explicit_note_id_from_app_event(&parsed, &self.state) {
        self.state = reduce(&self.state, EngramCommand::DeleteNote { note_id });
    }
}
```

With no explicit id, **nothing mutates and no error surfaces**. The user
clicks delete, the app emits an intent into a void, and the UI is unchanged.
`deleteNoteType` is identical.

So the missing effect channel is not only blocking a refactor. It is already
producing a silent no-op in shipped software. Direction (B) would make that
permanent by design rather than by omission.

---

## 5. The design

### 5.1 Two wire shapes, not one

`Effect` gains an explicit completion discriminator rather than relying on
convention:

```rust
pub enum Delivery {
    /// Fire-and-forget. The host may act; the app does not wait.
    Notify,
    /// The app is awaiting a result and will not progress without one.
    Await,
}

pub struct Effect {
    pub id: EffectId,
    pub kind: String,
    pub payload: Value,
    pub delivery: Delivery,
}
```

Making this explicit on the wire is what lets a host know whether dropping an
effect is *acceptable* or *a bug* — today it cannot tell, so it drops all of
them equally. A conformance test can then assert that every `Await` effect a
host receives is eventually completed, which is impossible to state without
this field.

### 5.2 One new C symbol

```c
mosaic_status mosaic_app_complete_effect(mosaic_handle app,
                                         mosaic_bytes effect_id,
                                         mosaic_bytes result,
                                         mosaic_buffer *update);
```

Seventh symbol, same ownership rules as `dispatch`: the app returns a fresh
`AppUpdate` which may itself carry further effects, and the caller frees with
`mosaic_buffer_free`. That an effect completion can produce more effects is
deliberate — an import that needs a *second* dialog (choose deck, then choose
file) is an ordinary flow, not an exception.

`result` carries a tagged outcome, so cancellation is a first-class answer
rather than a timeout or a silent drop:

```json
{ "ok": { "bytes": "<base64>" } }
{ "cancelled": {} }
{ "failed": { "message": "..." } }
```

**Cancellation must be explicit.** A file dialog dismissed with Escape is the
common case, not an error case, and an app that cannot distinguish it from
failure will show an error banner for an ordinary user action.

### 5.3 Version negotiation

`mosaic_app_create` already carries a `StartContext` with a protocol version.
Version 1 remains the default; hosts opt into version 2 and the app learns what
the host supports:

- A **v1 host** with a **v2 app**: the app is told the host cannot complete
  effects and must degrade deliberately — refusing an import with a clear
  message beats emitting an effect into a void, which is exactly today's
  failure.
- A **v2 host** with a **v1 app**: no effects with `Await` are ever emitted;
  the host's completion path is simply unused.

Neither combination may fail silently. That rule is the whole lesson of §4.2.

### 5.4 Migration order

1. `mosaic-app-runtime`: `Delivery`, `EffectId`, the result enum.
2. `mosaic-app-capi`: the seventh symbol and the version field.
3. `mosaic-app-conformance`: a test that an `Await` effect left uncompleted is
   **reported**, so the gap cannot recur silently in a new host.
4. One host template at a time, Qt first — that is where the CI lane exists.
5. `engram-mosaic-app` emits `importAnki` / `exportAnki` as `Await` effects and
   `openCard` as `Notify`.
6. Per-backend adapter migration (#13728), Qt first.
7. `deleteNote` / `deleteNoteType` gain a real confirmation flow, closing §4.2.

Steps 1–3 are the protocol; 4–7 are Engram's use of it. Splitting there matters
because steps 1–3 benefit every future Mosaic app that needs a host capability,
whether or not Engram's migration ever finishes.

---

## 6. Why (A), and what is worse about it

**(B) is cheaper and it caps Mosaic.** Under (B), every app needing a file
dialog, a confirmation, a clipboard read, or a share sheet hand-writes N
adapters against a bespoke C ABI — which is the cost Mosaic exists to remove.
The framework would be complete for apps that only paint state and permanently
incomplete for apps that ask the user anything. Engram is the first app to
reach that boundary; it will not be the last, and §4.2 shows the boundary is
already leaking into shipped behaviour as a silent no-op.

**What (A) genuinely costs.** It is a breaking change to a C ABI with five host
templates behind it, and asynchrony is the hard part: an app awaiting a
completion has a suspended flow, which interacts with `snapshot` and `restore`
in ways v1 never had to consider. **What happens to an in-flight effect when
the app is snapshotted?** The honest answer is that a snapshot taken mid-effect
must either refuse, or record the pending effect and re-emit it on restore.
Section 8 now settles that question (#13932): refuse standalone snapshot and
restore while any Await effect remains outstanding. This unblocks the protocol
implementation tracked in #14547. The core channel is implemented; this does not
establish capability execution or native host acceptance.

**The deciding argument** is that (B) is not actually a resting place. It is
today's situation with a name, and today's situation already silently swallows
a delete.

---

## 7. What this does not decide

- The platform-specific mechanics of cancelling an open dialog or interrupted
  file write. The runtime outcome must still be explicit; section 8 defines the
  snapshot and identity rules regardless of platform.
- Whether `openCard` should exist at all. It is a `Notify` under this design,
  but a notification no host consumes may simply be surface to delete. That is
  an Engram question, not a protocol one.
- Whether `hostIntent` is removed once effects land, or kept as a compatibility
  shim for the web host, where there is no C ABI and the JS bridge could keep
  its current shape.


## 8. Snapshot boundary for awaited effects (decision: #13932)

### 8.1 Refuse, without changing state

A running instance with one or more outstanding `Await` effects MUST refuse
standalone `snapshot` and `restore` with a typed `PendingEffects` result. Check
this before calling the application's snapshot/restore methods. The result
includes the outstanding effect identities so the host can explain why a save
or checkpoint is deferred. It is not an empty snapshot or a successful no-op.

Refusal MUST leave application state, pending effects, event sequence and update
revision unchanged. It MUST NOT cancel a dialog, roll back a user action, emit a
replacement request, or repeat a file operation. `Notify` effects do not block
snapshotting because no completion is owed for them.

Snapshots contain settled application state. Pending requests, file handles and
host callbacks are not added to the opaque snapshot format. A successful restore
therefore never reopens a dialog or repeats an external operation merely because
it occurred before the snapshot. This decision adds no field to `Snapshot`.

### 8.2 Retirement and identity

An awaited effect remains pending until its completion is accepted. Success,
explicit cancellation and failure are all terminal outcomes when accepted by the
application. If validation or the application rejects a completion, its identity
remains pending and sequence/revision are not consumed; the host may correct the
completion without repeating the external operation. Applications must validate
before mutation, as with ordinary rejected dispatches.

The runtime rejects duplicate, unknown and stale completion identities before
calling application code. Effect identities belong to an app instance's lifetime,
not to snapshot contents: a same-instance restore MUST NOT reset the identity
allocator or make a retired id valid again. A fresh instance has a separate
namespace. Host callbacks retain their originating instance identity and MUST NOT
be delivered to a replacement instance. Destroying an instance invalidates its
pending completions; it does not authorize the host to replay their side effects.

A completion may itself emit another `Await` request. Snapshot/restore remains
blocked until the whole pending set is empty, including that new request. The
runtime checks and retires identities as part of accepting the update, not when
a host merely begins handling the request.

### 8.3 Save and open flows

For Save, capture the versioned workbook snapshot before emitting the awaited
file-save request. The payload represents that consistent point in time even if
the application later permits more editing. A completed write acknowledges those
bytes; it does not imply that later edits have also been saved.

For Open, keep the current workbook until the awaited result is accepted. The
application validates and applies the returned snapshot as part of completion.
This is not a separate host `restore` call while the dialog effect is pending.
Malformed data, cancellation and read failure must leave the existing workbook
intact. A failed completion handler must not partially replace its state.

Autosave/checkpoint hosts retain the last successful checkpoint and defer a new
one while `PendingEffects` is reported. Retry after a terminal completion, rather
than spinning or silently discarding the requested checkpoint. Hosts that require
an immediate checkpoint may ask to cancel a capability, then wait for explicit
accepted cancellation before retrying. Timeout or closing a dialog does not by
itself retire an effect in the runtime.

This policy does not make external writes transactional across a process crash.
A host must not infer from the absence of a new checkpoint that a previous file
write did not occur. Resuming an interrupted operation requires a separate,
explicit recovery design; automatic replay is not part of this contract.

### 8.4 Required conformance before file controls ship

- A pending Await request causes typed snapshot and standalone restore refusal;
  neither application method runs, and state/sequence/revision remain unchanged.
- Notify-only updates remain snapshot-compatible.
- Accepted success, cancellation and failure retire the request exactly once;
  duplicate, unknown and old-instance completions never reach application code.
- Rejected completion leaves the original request pending without new I/O.
- A completion that emits another Await request still blocks a checkpoint.
- After the pending set empties, snapshots restore into a fresh instance without
  re-emitting the completed request. Same-instance restore cannot reuse its id.
- Save uses the bytes captured before Await. Invalid/cancelled Open preserves
  current workbook data and pending cell edits; valid Open applies atomically.

These are implementation acceptance requirements, not claims about protocol v1.
The existing runtime still has no completion channel. Implement #14547 before
adding browser file-effect handling or VisiCalc saved-workbook controls.
