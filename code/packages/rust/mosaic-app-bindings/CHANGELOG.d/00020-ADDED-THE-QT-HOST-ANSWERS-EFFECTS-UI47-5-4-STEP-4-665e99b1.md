### Added -- the Qt host answers effects (UI47 §5.4 step 4)

`MosaicHost` resolves `mosaic_app_complete_effect`, drains `update.effects`
after every create / dispatch / restore / completion, and declares **protocol
2**.

Before this, `Effect` rode the wire and was read by no native host at all: an
`await` was dropped and the app waited forever, with nothing anywhere reporting
it. The Qt host is the first to close that.

- `effectRequested(id, kind, payload, delivery)` is emitted per effect;
  a handler connected directly may call `completeEffect(id, result)` from
  inside the emit to answer it.
- An `await` **no handler answered** is completed as
  `failed: no host handler answered effect N` rather than dropped. A missing
  handler becomes a visible error instead of a hang.
- `notify` needs no answer and is never failed.
- Completion can produce further effects -- an import needing a second dialog
  is an ordinary flow -- so settling drains rather than sweeping once, with a
  64-round runaway guard that reports rather than looping.

Protocol 2 is declared because the host now *implements* completion. Declaring
it without that is the harmful direction, so the assertion that pins the version
now pins the capability beside it.

Scope of that bump, stated accurately: a v2 host running a v1 **app** is fine --
no `await` is ever emitted and the completion path goes unused, which the
existing `venture-browser` Qt acceptance (a v1 app) confirms. A v2 host against
an older **runtime** that only accepts protocol 1 is not: it is rejected at
`mosaic_app_create`. Every runtime in this repository accepts both, so nothing
here breaks, but a package pairing a newly emitted host with a stale
`libmosaic_app` would.

Five bugs found only by running it, three of them by the security review after
the first version of this change. None would have been visible to any assertion
on the emitted text:

- **Persistence ran before settling.** The runtime refuses to snapshot while an
  effect is outstanding -- correctly, since a half-answered effect is not a
  state worth restoring -- so every effect produced a spurious "could not
  persist Mosaic state" warning. Settling now happens first.
- **A handler answering during the emit left the caller with a stale update.**
  `completeEffect` called re-entrantly now hands its update back to the settle
  loop already running instead of starting a second one, and the loop adopts
  it.
- **A partly-answered batch dropped effects, twice over.** First, adoption and
  the fail loop were alternatives, so a batch where a handler answered one
  effect and ignored another left the ignored one neither answered nor cleared.
  Then, once that was fixed, the fail loop still *overwrote* the adopted
  update -- and an `Update` carries only the effects produced by the call that
  returned it, so a completion that produced a further effect (the "import
  needing a second dialog" this code describes) had it dropped instead.
  Effects now accumulate **at the write site** rather than being read back from
  a slot: `completeEffect` appends to the round's list, so N answers in one
  round contribute N effect lists. Accumulating at the read site was a third
  version of the same bug -- it survived one answer per round and dropped every
  earlier answer's effects when a handler answered two.

  Both are permanent: the runtime gates `snapshot` **and** `restore` on nothing
  being pending, so one dropped effect disables persistence for the rest of the
  process. The conformance fixture gained a two-effect batch event, because no
  single-effect test can reach either.
- **A handler calling back into the host recursed until the stack gave out.**
  The 64-round bound bounds iterations within a frame, not frames; a handler
  calling `handleEvent` rather than `completeEffect` re-entered one level
  deeper, and the review reproduced a SIGSEGV at roughly 1600 levels. Nesting is
  now bounded at 8 with a message.
- **`toULongLong` was not validation.** It accepts `-1` (wrapping to 2^64-1),
  `3.5` (truncating to **4**), and `1e30` (saturating) -- and `completeEffect`
  is `Q_INVOKABLE`, so QML, where every number is a double, is the expected
  caller. An id of 3.5 would have answered a *different* outstanding effect with
  the wrong result. Ids are now range- and integrality-checked before
  conversion.

A handler may also delete the host mid-emit, which the review reproduced as a
use-after-free under ASan. Liveness is carried across the `settleEffects` call
boundary -- the first attempt gated only *inside* it, and every caller then went
on to touch members of a freed object, which ASan reproduced again. The depth
guard no longer writes through freed memory, and the header says to use
`deleteLater()`.

Thread affinity is a **refusal**, not only an assert: `Q_ASSERT_X` compiles to
nothing under `QT_NO_DEBUG`, which is what a release build of a generated app
defines, so the check was absent in every shipped app. It matters more than a
stale result now that the re-entrancy slots are pointers into `settleEffects`'s
stack frame -- a completion arriving on another thread would write into a live
frame belonging to a different one.

Both runaway guards -- the nesting bound and the round bound -- answer the
effects they were handed before refusing, and drain what those answers mint for
up to 8 further rounds. That bound can be outrun: an app that keeps minting
replacements past it (measured at 73 chained failures with no handler, or 9 when
the nesting guard fired) leaves effects pending, and persistence is then off for
the rest of the session. It is a fallback of a fallback rather than something
worth an unbounded loop, so the host *states* that terminal condition in its
persistence warning instead of leaving it to be inferred from a later snapshot
quietly failing. Giving up with awaited effects still pending swapped a crash for an app
that can never snapshot again. The error path out of the fail loop discharges
what the round accumulated for the same reason.

The thread refusal covers `handleEvent` and `restore` as well as
`completeEffect`. Those two are `Q_INVOKABLE` and both install the frame
pointers, so guarding only the completion left the hazard the guard exists for
reachable through the other entry points.

