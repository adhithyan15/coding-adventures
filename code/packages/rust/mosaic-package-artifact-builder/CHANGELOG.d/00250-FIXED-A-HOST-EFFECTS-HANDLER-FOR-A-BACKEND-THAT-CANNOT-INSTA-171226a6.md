### Fixed — a `[host_effects]` handler for a backend that cannot install one

`install_host_effects` copies a declared handler's source for **any** backend.
Only two emit the call that reaches it: `qt_main_with_host_effects` and
`swift_app_with_host_effects`. Nothing checked the difference.

So a package declaring `{ backend = "compose", install = "installFoo" }` got the
manifest's blessing — it validates the symbol, and there is a test declaring
exactly that shape — then the file copied into the project, compiled, linked and
shipped, with nothing ever calling it. No missing symbol. No diagnostic.

That is the silent permanent failure this whole section exists to prevent,
reached through the one gap the section did not check. The first symptom is an
`Await` effect arriving with no handler, going unanswered, and the runtime's
gate on nothing-pending disabling snapshot **and** restore for the life of the
process.

Qt and SwiftUI already hard-fail when they cannot find their anchor — a declared
handler with nowhere to go is a build error, not a quiet no-op. This is the same
refusal for the seven backends that have no anchor to look for, and it fires
**before** anything is copied: refusing after writing would leave a file behind
that the next run reads as pre-existing.

#### It gates on the handler, not the file

The first version keyed on `files` and sat below the "no files for this backend"
early return, which closed only half the hole — security review caught it. A
handler declared with **no file of its own** parses clean, because the manifest
checks file-without-handler and not the converse, so it slipped past the early
return entirely.

That is reachable rather than theoretical. `[host_assets]` is a second door into
the same output directory, and a package can ship its handler's implementation
there — where SwiftPM and Gradle compile a directory implicitly — while
declaring the handler in `[host_effects]`. Engram already ships Compose host
assets exactly that way, so the shape is one line of manifest from being live.

#### Why it is a match rather than a list

`Backend::installs_host_effects` is exhaustive, so a new backend variant stops
the crate compiling until someone classifies it. That is the only version of
"remember to update the supported set" that survives contact with a future
change — and the failure this closes came from exactly such an omission.

A companion test pins the other half: that the backends *claiming* to install
really do have an emitter. A backend that claims wrongly would let a dead
handler through for the precise reason the refusal exists.

That test iterates `Backend::ALL` rather than a hand-copied list of the
variants — its first version repeated all nine, which put a blind spot on
exactly the change it exists to catch. A tenth backend forces the match to be
updated, but if it were classified `true` by mistake, a stale nine-element array
would filter straight past it and pass.

Mutation-tested in three directions: removing the refusal fails the refusal
test, falsely marking Compose as installing fails both classification tests, and
re-keying the gate on `files` fails the file-less-handler test.

Found while checking whether Engram's Compose handler could be written ahead of
the Compose emitter wiring. It cannot, and this is why — the handler would have
shipped dead.

