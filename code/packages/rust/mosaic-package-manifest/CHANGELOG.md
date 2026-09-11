# Changelog — mosaic-package-manifest

## [Unreleased]

### Added — `[host_effects]`, so a package can supply an effect handler

Every Mosaic host template can answer an `Effect`, and an application can emit
one, and until now nothing connected the two: the entry point that would install
a handler — `main.cpp`, `Main.kt` — is generated, so package code could not
reach it. `[host_assets]` lets a package put a *file* into a generated project,
but nothing generated ever calls into one. UI47 §5.4a records the gap and §5.5
designs this section.

```toml
[host_effects]
files = [
  { backend = "qt", source = "host/qt/effects.h",   target = "effects.h" },
  { backend = "qt", source = "host/qt/effects.cpp", target = "effects.cpp" },
]
handlers = [
  { backend = "qt", include = "effects.h", install = "installProbeEffects" },
]
```

`files` are copied into the backend output directory **and added to that
backend's build source list**.

The first version of this entry justified the second half by claiming
`[host_assets]` can never append a build source. That was false — generalised
from Qt without checking the others. `activate_react_host_asset` already
prepends an `import` for a newly-copied file, `activate_html_host_asset` inserts
a `<script>` tag, and SwiftPM and Gradle compile their source directories
wholesale; Engram ships three new files that way today.

What is true, and still enough: **Qt and XAML list their sources explicitly**, so
a newly-copied file is never compiled there — and those are the backends a
handler has to reach.

`handlers` carries at most one per backend. `install` names the symbol the
generated entry point calls; `include` is optional and backend-interpreted,
because Swift's handler is in the same module and Kotlin's may be in the same
package, so requiring one would force those backends to invent a meaningless
value.

This commit is the manifest half only: parsing, validation and the public types.
Nothing reads the section yet.

Two validations are refusals rather than conveniences, and both are about the
same failure:

- **Two handlers for one backend.** Last-wins would install both and let the
  second overwrite the first's registration, so the first's effects would go
  unanswered — which is the exact failure the effect mechanism exists to
  prevent. `DuplicateHostEffectHandler` names the backend, since a manifest may
  declare five.
- **A file whose backend declares no handler.** The emitter would copy it into
  the project and add it to the build with nothing calling into it.
  `HostEffectFileWithoutHandler` names the file.

An empty `include` is refused too. Absent means "no include needed", an empty
string reads the same way, and accepting it would make a typo indistinguishable
from a decision.

Unknown keys are refused by `deny_unknown_fields`, so a misspelled `instal`
fails loudly instead of emitting a project with no handler wired and no
complaint.

Mutation-tested: removing either refusal fails exactly the test that claims it.

#### Hardened, from the security review of this section

`install` and `include` will be interpolated **verbatim** into generated C++,
Kotlin, Swift, Dart and C# — a call expression and an `#include` respectively —
so both are now shape-checked at parse.

TOML basic strings permit `\"` and `\n`, so an unshaped value escapes its
syntactic slot: `install = "system(\"rm -rf /\"); dummy"` is an arbitrary
statement in someone's `main.cpp`, and an `include` carrying a quote and a
newline rewrites the whole translation unit.

This is not a new privilege — a package's `[host_assets]` can already overwrite
`MosaicHost.cpp` outright — but it is the first place executable text lives in
the *manifest string* rather than in a file, which defeats any review or
attestation that reads files. It costs nothing to close, and validating at parse
means five emitters do not each have to remember.

The shapes real backends use are all still accepted: a relative header
(`effects.h`), a qualified symbol (`Engram.Effects.Install`), a dotted import
(`com.example.Effects`), and Dart's `package:` scheme.

Three more, same review:

- **`*` is refused as a backend**, where `[host_assets]` accepts it. Copying one
  file everywhere is meaningful; installing one handler everywhere is not, since
  what `install` receives differs per backend by design. Accepting `*` would
  produce an entry matching no backend at emission that quietly installs
  nothing.
- **Two `files` entries writing one target for one backend are refused** — the
  later copy would win and the earlier file silently not be what compiles, which
  is the same last-wins hazard already refused for handlers. Keyed on backend
  *and* target, so one source serving several backends stays legal.
- **Validation is linear**, via a set of handler backends rather than a scan per
  file. A manifest is small, but nothing bounds its length and quadratic
  validation over unbounded input is a cost with no upside.

One finding is deliberately **not** fixed here, because it belongs to the
emitter half: `install_host_assets` resolves a package-relative source and reads
it without a canonicalise-and-contain check, unlike `load_package_tokens` next
door, so a symlinked source escapes the package root. `[host_effects]` would
inherit that and make it worse, since these files are compiled rather than
merely copied. It is latent until an emitter consumes the section, and it is a
required condition on that change.


### Added

- Added optional `[styles].token_palette` declarations for safe,
  package-relative schema-v1 JSON token palettes.
- Added optional `[host_assets]` support with `files = [{ backend, source, target }]`
  declarations so app packages can describe backend-specific host adapter files
  inside `mosaic-package.toml`.

## [0.1.0] — 2026-05-19

### Added

- Initial implementation of the `mosaic-package.toml` manifest parser per
  UI29 §4.1 / §4.2.
- `parse(&str) -> Result<MosaicPackage, ManifestError>` — parses a TOML
  manifest from a string and validates every field.
- `parse_path(&Path) -> Result<MosaicPackage, ManifestError>` — reads a
  manifest file from the filesystem and parses it.
- `MosaicPackage` IR — typed representation of the four required sections:
  `package`, `components`, `dependencies`, `kernel`.
- `PackageMeta` — name + version + description + license.
- `ComponentsSection` — `exports: Vec<String>` (PascalCase component names).
- `KernelSection` — `version: String` (currently only `"1"` accepted).
- `ManifestError` — six structured error kinds: `TomlSyntax`,
  `MissingField`, `InvalidPackageName`, `InvalidComponentName`,
  `InvalidKernelVersion`, `InvalidSemverString`.
- Validation regexes:
  - kebab-case package names: `^[a-z][a-z0-9]*(-[a-z][a-z0-9]*)*$`
  - semver-like versions: `^\d+\.\d+\.\d+(-[A-Za-z0-9.-]+)?$`
  - PascalCase component names: `^[A-Z][a-zA-Z0-9]*$`
- 13 unit tests covering the happy-path manifest, every error variant,
  empty exports, dependency parsing, kernel version rejection, TOML
  syntax error reporting, and `parse_path` filesystem read.
- 1 doctest for the `parse()` entry point.
