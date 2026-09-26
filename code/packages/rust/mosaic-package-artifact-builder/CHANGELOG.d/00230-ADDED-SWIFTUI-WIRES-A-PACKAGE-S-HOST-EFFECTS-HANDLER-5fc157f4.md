### Added — SwiftUI wires a package's `[host_effects]` handler

The second backend, after Qt. The shapes differ in a way worth recording.

**SwiftUI needs no build-list half.** SwiftPM's generated `Package.swift` gives
the target `path: "Sources/App"` with no explicit `sources:`, so every `.swift`
under it is compiled and a copied file is already in the build. Qt names its
sources and needed `target_sources`; that asymmetry is the whole reason
`[host_effects]` carries a build obligation at all, and it does not apply here.

**The install is guarded by a downcast.** `MosaicHostState` holds its host as
`MosaicHostBridgeObject?` — a protocol that deliberately knows nothing about
effects — while `effectHandler` lives on the concrete `MosaicRuntimeHost`. The
`if let` is also correct rather than merely necessary: when the standard host is
absent the app falls back to a reflection bridge, and there is no host to
install onto then.

The call goes immediately after the host is assigned and before the first props
refresh, which is the first thing that can produce an effect.

Anchored on `self.bridge = ` rather than the whole call, because
`--runtime-library` rewrites it to `MosaicRuntimeHost.load(libraryPath: …)` —
anchoring on the plain form would silently emit no install in exactly the
configuration that ships. A test pins the bundled form, and mutation-testing it
confirms the narrower anchor fails.

**The anchors are pinned against real emission, not only fixtures.** Both
anchors — `class MosaicHostState` and `self.bridge = ` — are incidental details
of `mosaic-emit-swiftui`, free to be renamed by someone who never reads this
crate, and a fixture-only test would keep passing through such a rename while
every real build broke. So `the_anchors_match_a_genuinely_emitted_swiftui_app`
emits a project through the actual pipeline, applies the same runtime-binding
rewrite the build applies, and wires that. Renaming the class in the emitter
fails it.

It covers all four combinations of `require_runtime` × `bundle_runtime`. The
first of those matters more than it looks: the emitter keeps *two* separate
`MosaicHostState` templates and picks between them on `require_runtime`, and the
builder sets that flag for `--profile native-complete` and for any build passing
`--runtime-library` — which is exactly how Engram is built in CI. The first
version of this test exercised only the default, leaving the shipping shape
unpinned. An assertion now also pins that the flag genuinely selects a different
template, so the loop cannot silently run twice over identical text.

Three guards decide where the install lands: the host class is found by its
declaration, the search is scoped to it, and within that the assignment must
begin its line. Each is pinned by its own test, because a fixture that several
guards reject proves only that at least one works. Mutation-testing confirms the
split — each mutation fails exactly one test and leaves the others green:

| mutation | fails |
| --- | --- |
| drop the line-leading requirement | `a_non_line_leading_decoy_inside_the_class_is_skipped` (+2 unit tests) |
| search the whole file, unscoped | `a_line_leading_decoy_before_the_class_is_skipped` |
| find the class with a bare `find` | `a_forged_class_marker_fails_the_build_rather_than_redirecting` |
| prefer the first declaration instead of refusing | `a_forged_class_marker_fails_the_build_rather_than_redirecting` |

The last two coincide because a bare `find` discards both the prefix predicate
and the uniqueness check, and it is uniqueness that the test is really pinning —
which is the point of the paragraph above. Each row was run, not predicted; an
earlier draft of this table attributed the third row to the wrong test.

That third guard is new, and the honest statement of what it buys is narrower
than the one first written here.

`class MosaicHostState` must now be preceded on its line by nothing but Swift
declaration modifiers — but that does **not** make this crate independent of
`mosaic-emit-swiftui`, which is what an earlier draft of this entry claimed. A
security review caught it. `escape_swift_string` passes raw newlines through, so
a package author whose slot default contains `"\npublic final class
MosaicHostState"` puts a line into the generated app with the prefix
`public final ` — accepted by the predicate, exactly like the genuine one. A
forged declaration is byte-identical to a real one. No lexical test separates
them, and a stricter prefix rule would only move the bar again.

So the guarantee is about the failure **direction**, and it comes from requiring
the declaration to be *unique*. A second acceptable declaration is refused rather
than silently preferred, which means the worst a package author can do is fail
their own build loudly — never redirect the install into a location of their
choosing. That is a property of this crate, checkable here, instead of an
assumption about another crate's escaping.

Two tests carry the pair: one asserts the ambiguous app is refused and names both
declarations in the error, the other asserts the predicate *does* accept the
forgery — because the predicate is not what makes this hold, and a test implying
otherwise would mislead the next reader.

Forging the marker alone only *widens* the search window rather than moving the
install, so a one-decoy fixture cannot tell a bare `find` from an anchored one;
the first version of that test passed against the mutation it was written to
catch.

`line_anchored_find` also now uses `is_ascii_whitespace` rather than
`char::is_whitespace`: the Unicode White_Space set includes U+2028, U+2029 and
U+00A0, which Swift does not treat as code whitespace, so accepting them would
let a prefix a compiler reads as content count as indentation.

