### Changed — Compose reaches the engine through the standard runtime

**The release tests had to move with it, and CI is how that was found.**
`Validate release identity` went red with **18 failures and 16 errors**, all in
`ArchiveComposeTests`. #15089 taught `archive_compose` to derive its expected
engine from the manifest, and left Compose's answer deliberately unpinned so
this migration would not fail a test arguing it should not happen — but the
*fixtures* still built `libengram_capi.dylib` distributions, and a migrated
Compose ships `libmosaic_app`.

They derive the name now, the same way the code does, so they keep testing
layout and contents across a migration instead of becoming tripwires for the PR
performing one. Which engine is correct is pinned separately; the fixtures make
no such claim.

Deriving a fixture from the same source as the check is also the shape that can
quietly go vacuous, so it was mutation-tested three ways: dropping the engine
requirement, reinstating the retired engine, and accepting an empty one each
fail something. The second reproduces exactly the 18-and-16 CI reported, which
is what confirms the diagnosis rather than merely agreeing with it.

The third backend off a hand-written host, after Qt (#13728) and SwiftUI. The
574-line `host/compose/MosaicHost.kt` — a JNA binding that opened the library,
marshalled every event, owned snapshot persistence, and drove the file dialogs
— is replaced by the generated `MosaicRuntimeHost` plus a 240-line
`[host_effects]` handler that answers three effects and does nothing else.

Emission is `nativeComplete: true` with `replacedGeneratedFiles: []`.

**What the handler does differently from Qt's.** Qt answers inline, because its
settle runs on the event-loop thread under `Qt::DirectConnection`. This host
holds its monitor across the handler call, so a modal dialog run inline would
hold it for as long as the dialog is open. It defers.

It then marshals to the EDT, and that is a *second* reason rather than a
restatement of the first: the host's contract says the props-changed handler
runs on whichever thread answered, and Compose state must be written from the
UI thread. Answering off the EDT would be a cross-thread write into the
composition even if the monitor were free. `SwingUtilities.invokeLater` settles
both, and is where a Swing dialog has to run anyway.

**Nothing escapes the deferred block.** The first draft guarded the file I/O
inside each `run*` function and carried a comment claiming "exactly one answer,
on every path" — which the code did not do. The dialogs sit outside those
guards: `JFileChooser`'s constructor and all three `show*Dialog` calls throw
`HeadlessException` on a display-less session. An exception there unwinds to the
EDT's uncaught handler, `completeEffect` never runs, and because `deferEffect`
has already taken the id out of the runtime's fail sweep, the effect stays
awaited for the life of the process — which disables snapshot *and* restore, not
just that one dialog. Caught in security review; Qt guards the same span with
`catch (...)`.

**The handler answers exactly the two kinds the application mints, and a test
now pins the complement.** A draft of this file also answered `confirmDelete`
with a Swing confirmation dialog, and this entry claimed Compose was the first
host to carry one. Neither was true of anything that runs: `host_intent_for_event`
emits `importAnki`, `exportAnki` and `openCard` and nothing else, and
`effect_for_intent` turns only the first two into an `Await` — so the branch was
unreachable, and the delete intents it imagined were deliberately retired in
issue #13933. It compiled, and the test written for it asserted only that the
string `confirmDelete` appeared in the file, so it passed. The gate is now the
complement — that kind, `openCard`, and the two retired delete intents must
*not* appear — which fails on the dead branch instead of ratifying it.

**The anchors are `\A`/`\z`, and that was measured rather than carried over.**
This is the fourth regex engine asked whether `$` concedes a trailing line
terminator, and the fourth different answer: Rust refuses all of them, PCRE2
concedes LF (a build-time convention, which is the bug the Qt handler had), ICU
concedes the full set, and Java concedes LF, CRLF, CR, NEL and U+2028. No two
agree. A compiled probe established that `Regex.matches()` rejects all of them
anyway, so the anchors are redundant *as written today* — they are there so the
pattern stays correct if the call ever becomes `containsMatchIn`, rather than
correct only because of its caller.

