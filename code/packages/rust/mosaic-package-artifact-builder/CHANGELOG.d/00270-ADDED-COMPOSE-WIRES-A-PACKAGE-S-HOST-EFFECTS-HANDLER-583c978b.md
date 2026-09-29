### Added — Compose wires a package's `[host_effects]` handler

The third backend, after Qt and SwiftUI.

**No build-list half**, like SwiftUI and unlike Qt. The emitted
`build.gradle.kts` declares no `sourceSets`, so the Kotlin JVM plugin's
convention applies and everything under `src/main/kotlin/` is compiled — which
is where a Compose `[host_effects]` target lands.

**The install goes through a downcast**, immediately after the host is loaded
and before the `Window` that mounts the app. `MosaicComposeHost` is an interface
that deliberately knows nothing about effects; `effectHandler` lives on the
concrete `MosaicRuntimeHost`. `as?` also covers the non-`native-complete` shape,
where the host falls back to `MosaicComposeHostBridge` and there is nothing to
install onto.

#### The anchor is guarded, and honestly described

Line-anchored on `val mosaicHost = remember {`, as on the other backends — but
here that is **defence in depth rather than the load-bearing guard**.

The reason is positional, and the first version of this entry gave a different
one that does not hold. It credited `escape_kotlin_string` turning newlines into
`\n`, which is true of slot defaults, `OneOf` members and the window title — but
`component_name` and slot field names are interpolated **raw** by
`build_compose_root_invocation`, so the escaping is not what makes this safe.
(Those two are constrained by grammar instead: `pascal_case_re` and the mosmodel
`NAME` token admit no newline.) Security review caught it.

What carries the weight is that **nothing author-controlled is emitted before
the anchor**. `build_compose_root_invocation`'s output lands in `MosaicApp`,
emitted after `fun main()`; everything ahead of the anchor line is fixed
scaffolding. Since the search scans forward and takes the first hit, the real
anchor wins even if author text could begin a line.

That property is checkable, so it is now a test rather than a comment —
`no_author_controlled_text_precedes_the_anchor` emits with a findable component
name, slot name and default, and asserts none appears before the anchor. A
future emitter that moved author text above it would otherwise make the
line-anchoring load-bearing with nobody noticing; the mutation fails the test.

SwiftUI is the opposite case, which is why the comments differ: its author text
sits ~270 lines *ahead* of the assignment being anchored on, so there the
anchoring really is the only thing between a slot default and the splice point.

#### An `include` on a Compose handler is refused

Kotlin has no include directive, so the field would be silently dropped — the
shape of failure this section exists to refuse, not a tidy-up. It is a build
error naming the handler.

#### Verified against the real generator, and compiled

- A unit test drives `build_compose_main_kt` in **both** `require_runtime`
  shapes, asserting the flag really selects a different host loader so the loop
  cannot pass twice over identical text.
- A probe package declaring a Compose handler was emitted end to end: the
  install lands at `Main.kt:16`, between the host at `:14` and the `Window` at
  `:17`, and the handler is copied into `src/main/kotlin/` where Gradle compiles
  it.
- The handler and the **emitted install expression, lifted verbatim from the
  generated file rather than retyped**, both type-check with `kotlinc` against
  the real generated `MosaicRuntimeHost.kt` and its `MosaicComposeHost`
  interface. The bytecode confirms the handler binds `setEffectHandler`.

That last step is the one the SwiftUI migration skipped: there the handler was
checked against a hand-written stub whose visibility differed from the real
class, and the actual build rejected it. A stub validates the construct you
thought to model.

Not verified here: `Main.kt` compiling in full, which needs the Compose
toolchain. CI's Compose lane covers it.

#### The classification tripwire worked

Flipping `Backend::Compose` to `true` in `installs_host_effects` immediately
failed the three tests that pinned it as unsupported, which is what sent me back
to update them deliberately. That is the whole argument for an exhaustive match
over a hand-maintained list, and it is the first time it has fired.

