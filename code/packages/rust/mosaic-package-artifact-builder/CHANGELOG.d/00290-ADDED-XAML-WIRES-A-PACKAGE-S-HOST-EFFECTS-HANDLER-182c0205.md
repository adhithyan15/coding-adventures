### Added — XAML wires a package's `[host_effects]` handler

The fifth and last backend. **Every native backend now installs a declared
handler**, and what remains refusing is exactly the web family — React,
Electron, WebComponent, HTML — where an effect is a browser API the page calls
directly and there is no host process to hand one to. The classification test
now pins both lists rather than only the installing one: an equality on one
side alone still passes if a backend is quietly dropped from both.

Two things make XAML differ from the other four.

**The install takes no argument.** `MosaicRuntimeHost` is a *static* class over
a process-wide `Lazy<Runtime?>`, so there is no host value to pass. Qt, SwiftUI,
Compose and Flutter all hand the handler a host; here the host is the type. That
also puts more weight on the `install` name — C# resolves a fully-qualified
static method with no import at all — so `include` is refused, with a message
saying to qualify the name instead.

**The stub shell is refused, not degraded.** Emitted without
`--profile native-complete`, the WinUI window finds its host by *reflection* and
never calls `MosaicRuntimeHost` at all; it may find a hand-written
`Mosaic.Generated.MosaicHost` with no effect surface whatsoever. The other four
backends can install into their stubs because each still constructs a host
there. This one cannot, and emitting anyway would be worse than it looks:
assigning `MosaicRuntimeHost.EffectHandler` reaches a setter written
`if (State.Value is { } runtime) runtime.EffectHandler = value;`, which assigns
**nothing** when the library is absent. The build would then ship a handler
nobody installed, answer no effects, and report nothing.

So the refusal names the profile rather than only saying "no anchor" — this
fires on an ordinary build, not just a broken one, and "no anchor" alone would
send the author hunting a bug that is not there.

The anchor is `MosaicRuntimeHost.LoadRequired();`, and the ordering it gives is
load-bearing for the same reason: `LoadRequired` calls `RequiredRuntime`, which
*throws* rather than returning null, so every path past it has a non-null
runtime and the setter takes. An install emitted above that line would compile,
run, and silently assign nothing.

Verified end to end, not only against fixtures: a package declaring a XAML
handler emits `nativeComplete: true` with an empty `replacedGeneratedFiles`,
the handler is copied, and the install lands between `LoadRequired()` and
`InitializeComponent()`. A stub-profile build of the same package exits 1 with
the message above and copies no handler. The ordering assertions are
mutation-tested — inserting the install before the anchor instead of after
fails both the fixture test and the one driving the real emitter.

`only_the_native_complete_shell_can_be_wired` drives the real emitter for both
`require_runtime` values, so an emitter that later gives the stub a runtime call
turns the test red rather than leaving a refusal nobody revisits. That check
exists because the Flutter work shipped a `require_runtime` regression twice by
asserting only the shape it happened to emit.

**One thing this reclassification changes that is easy to misread, now pinned.**
Without `emit_project` there is no generated entry point, so a declared handler
is copied and *not* installed — the output is component artifacts for embedding
in a hand-written application, where the consumer writes the install. Refusing
there would break that legitimate use.

XAML previously refused such a build outright, so the diff reads as a loud error
becoming a silence. It is not: that refusal fired because XAML could not install
*at all*, not because of the profile, and Qt has behaved this way since it was
wired. `a_handler_is_copied_without_a_shell_and_that_is_uniform` asserts it
across every installing backend, so the behaviour is a recorded decision rather
than something a future reader has to re-derive. Raised in security review and
verified against Qt before concluding it was not a regression.

