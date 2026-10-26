# Mosaic package publishing v1 — what a component package is, and how it ships

**Status:** **PROPOSED — awaiting maintainer decision** (2026-10-26). Nothing
in this document is decided. §5 is a recommendation, and §11 lists the
questions the maintainer has to answer before the first implementation PR.
**Tracked by:** [#14014](https://github.com/adhithyan15/coding-adventures/issues/14014)
(Phase 0 of [#14011](https://github.com/adhithyan15/coding-adventures/issues/14011)),
`MOSAIC-BACKLOG.md` PR-4.
**Builds on:**
- [UI29 — primitive kernel and userland packages](UI29-primitive-kernel.md)
  §4, which defines `mosaic-package.toml` and says in §9 that registry hosting
  is "a separate spec". This is that spec.
- [Mosaic component program v1](mosaic-component-program-v1.md) §2 gap 3
  ("Nothing is published") and §7 ("that package is published").
- [Mosaic product program v1](mosaic-product-program-v1.md) §7 (products are
  assembled only from *released* components).
- The product-scoped release lanes `release-task-app.yml` and
  `release-engram.yml`, whose identity rules §7 copies.
- [B03 — build vendoring](B03-build-vendoring.md), whose offline-verification
  mode §8.5 copies.

---

## 1. Why this exists

The component program says a sharable, core component goes into a package and
"that package is published". Every Mosaic package says `publish = false` in its
`Cargo.toml`. #14014 reads that as the gap. It is not quite the gap, and
getting that right is most of this spec.

**`publish = false` is correct, and should stay.** Here is the toolkit's
`Cargo.toml`:

```toml
[package]
name    = "mosaic-pkg-toolkit-smoke"   # not the package's name
version = "0.1.0"                      # not the package's version (0.16.0)
publish = false
```

That crate is a **test harness**. It exists to run `package_compiles.rs` and
`native_complete_gate.rs`, and its own header says so: "This package's primary
deliverables are .mil / .mll / .msl source files ... NOT a Rust crate." The
package's real identity is in `mosaic-package.toml`:

```toml
[package]
name    = "mosaic-pkg-toolkit"
version = "0.16.0"
```

Flipping `publish = true` would publish a crate called
`mosaic-pkg-toolkit-smoke`, at version 0.1.0, which contains a test. Nobody
would be able to use it. So the real deliverable is the thing #14014's own
open question suspected: a way to ship **Mosaic sources**, versioned by
`mosaic-package.toml`, that `mosaic-compile` can find outside this repository.

There are three questions:

1. **What is the artifact?** A Mosaic package is a manifest plus `.mil`
   (interface), `.mll` (layout), `.msl` (style) and `.stories.json` files.
   `mosaic-compile` *inlines* those sources into the consumer at compile time
   (§2.2). The artifact must therefore carry sources. It cannot be a
   pre-compiled library.
2. **Where does it live?** This is the registry question (§4).
3. **How does a consumer get it, check it, and pin it?** Today the resolver
   only searches local directories and ignores version strings entirely (§2.2).
   Publishing anything is pointless until the resolver can consume it (§8).

---

## 2. What exists today

### 2.1 The packages

`code/packages/mosaic/` holds **24** packages: 22 `mosaic-pkg-*` and 2
`mosaic-std-*`. (#14014 says 19. The count has grown since it was filed.)

| | |
|---|---|
| Versions | From `0.1.0` to `0.16.0` (`mosaic-pkg-toolkit`), in `mosaic-package.toml` |
| Shipped files | `src/*.mil` (61), `src/*.mll` (61), `src/*.msl` (119), `src/*.stories.json` (61), `tokens/*.json` (2, `mosaic-std-*` only) |
| Harness-only files | `Cargo.toml`, `Cargo.lock`, `BUILD`, `src/lib.rs`, `tests/*.rs` |
| Docs | `README.md` and `CHANGELOG.md` in every package |
| `[host_effects]` / `[host_assets]` / `[app]` | **None** of the 24. Only programs under `code/programs/mosaic/` use them |
| License | Every manifest says `MIT OR Apache-2.0`. **No license text exists** anywhere in these packages, and the root README says "Everything in this repository is copyrighted ... Individual packages may be licensed separately" (§11 Q1) |
| Toolkit size | 26 components, about 528 KB of source |

The dependency graph is shallow. Four packages and four programs declare
dependencies (26 requirement strings in all), and the toolkit has none:

```
mosaic-pkg-toolkit ◄── mosaic-pkg-card-browser, mosaic-pkg-sheet, mosaic-std-controls,
       ▲                engram-app, task-app, journal-app
mosaic-pkg-grid ◄───── mosaic-pkg-sheet, visicalc
mosaic-pkg-rating-controls ◄── mosaic-pkg-review-card
mosaic-std-foundation ◄─────── mosaic-std-controls
(+ engram-app and task-app each depend on 6–11 single-purpose packages)
```

### 2.2 How a dependency is located today

There are **two** locators, and they disagree about how a package is found.

| | `mosaic_package_resolver::build` | `LayoutPackageResolver` |
|---|---|---|
| Answers | "What is the unqualified tag `Grid`?" | "Inline `pkg::mosaic-pkg-grid::Grid` here" |
| Finds a package by | Directory **name**: `mosaic-pkg-<key>`, then `<key>` | **Manifest name**, scanning every directory in every search root |
| Gated by `[dependencies]` | Yes | **No.** Any package on the search path can be inlined |
| Order when two match | First search path wins | `read_dir` order, which the OS does not specify |
| Version string | Read into `(dep_name, _ver)` and **ignored** | Not read |

The search paths come from the filesystem layout.
`default_package_search_paths` in `mosaic-package-artifact-builder` walks the
ancestors of the package root and adds every `packages/` and
`packages/mosaic/` it finds. `mosaic-compile` defaults to `code/packages` and
`code/packages/mosaic` relative to the working directory, and accepts
`--package-search-path`. Both only work inside a checkout of this repository.
An app in another repository finds nothing.

The manifest parser checks only the *shape* of a version string
(`^\d+\.\d+\.\d+(-[A-Za-z0-9.-]+)?$`). Its own comment defers "real
version-range arithmetic" to the loader, and the loader never does it. As a
result, five of the 26 declared requirements in the repository are already
unsatisfiable under any SemVer rule, and nothing notices:

| Consumer | Requires | On disk |
|---|---|---|
| `mosaic-pkg-card-browser` | `mosaic-pkg-toolkit = "0.11.0"` | 0.16.0 |
| `mosaic-pkg-sheet` | `mosaic-pkg-toolkit = "0.11.0"` | 0.16.0 |
| `mosaic-std-controls` | `mosaic-pkg-toolkit = "0.11.0"` | 0.16.0 |
| `engram-app` | `mosaic-pkg-toolkit = "0.15.0"` | 0.16.0 |
| `task-app` | `mosaic-pkg-toolkit = "0.15.0"` | 0.16.0 |

Every `pkg::P::` reference in the repository (19 distinct packages are
referenced) names either the package itself or a declared dependency. So
gating `LayoutPackageResolver` on `[dependencies]` would break nothing today.

**What UI29 said and what was built.** UI29 §4.3 says "Hosts depend on the
*compiled artifact*, not the source", with per-backend output shipped the way
npm and SwiftPM ship libraries. The implementation went the other way: the
resolver compiles the dependency's `.mil`/`.mll` and inlines its layout tree
into the consumer, and it merges the dependency's styles before any backend
emitter runs. That is why the artifact in this spec is a **source archive**.
Shipping pre-compiled per-backend bundles beside it is possible later
(§11 Q9), but no consumer could use them today.

### 2.3 The release lanes this repo already runs

| Lane | Trigger | Identity | Gates | Publishes |
|---|---|---|---|---|
| `release-task-app.yml` | `workflow_dispatch` (version + tag inputs); `pull_request` runs every job except `publish` | Tag `task-app-v<semver>`. `taskapp_release.py validate` requires strict SemVer and an exact tag match | Dispatched from `main`; CHANGELOG has the release heading; tag and release do not exist, checked once before the build and again just before publishing | `gh release create --latest=false` (plus `--prerelease` when the version has a `-`), with assets, a JSON release manifest naming the commit, and `SHA256SUMS` |
| `release-engram.yml` | Same shape, `engram-v<semver>`. Its comment explains why dispatch replaced tag push: "a tag push publishes whatever it points at" | Same | Same | Same, via `softprops/action-gh-release` pinned by SHA |
| `publish.yml` | `release: published` | Tag `<language>/<package>/v<version>` | None beyond tag parsing | PyPI through trusted publishing (OIDC, `environment: pypi`); RubyGems through a long-lived `RUBYGEMS_API_KEY` secret |

No workflow in `.github/workflows/` publishes to crates.io or npm.

Pinning varies between lanes. `release-venture.yml` pins every action,
first-party ones included, to a 40-character SHA. `release-task-app.yml` pins
third-party actions but uses `actions/checkout@v7` and other first-party tags.
`publish.yml` interpolates `${{ github.event.release.tag_name }}` straight
into a shell script. A new lane should copy the first of these patterns and
neither of the other two (§9.2).

### 2.4 Repository facts that bear on the choice

These were checked on 2026-10-26.

- **The repository is public.**
- **Immutable releases are enabled** (`GET /repos/.../immutable-releases` →
  `{"enabled": true}`). After a release is published, neither its tag nor its
  assets can change.
- **GitHub release attestations already work.** `gh release verify
  task-app-v0.5.12` reports "Release task-app-v0.5.12 verified!" and lists a
  digest for each of its 13 assets. A consumer can check any asset with
  `gh release verify-asset <tag> <file>`.
- **The only deployment environment is `github-pages`.** The `pypi`
  environment that `publish.yml` names does not exist, so it carries no
  protection rules today.
- **One ruleset exists, `BlockMainPush`, and it targets branches.** No tag
  ruleset exists. A published release protects its own tag, but nothing stops
  someone with write access from creating an unpublished tag with a future
  package's name.
- **Name availability, read-only lookups on 2026-10-26:** `mosaic` is taken on
  both crates.io and npm. `mosaic-pkg-toolkit`, `mosaic-compile` and
  `mosaic-package-manifest` are free on crates.io, and `mosaic-pkg-toolkit`
  and `@coding-adventures/mosaic-pkg-toolkit` are free on npm. Who owns the
  npm scopes was not checked.

---

## 3. What any answer must provide

### 3.1 Who consumes a published package

| Consumer | Today | Needs from publishing |
|---|---|---|
| In-repo apps (TaskApp, Engram, Journal, VisiCalc) | Path resolution from the monorepo | **Nothing.** They must keep building from the working tree, byte for byte |
| Per-component test apps ([#14015](https://github.com/adhithyan15/coding-adventures/issues/14015)) and the docs site ([#14026](https://github.com/adhithyan15/coding-adventures/issues/14026)) | Not built yet | A released, immutable version to point a download at |
| An app in another repository | **None known** (§11 Q2) | Find, verify, pin and build against a version, offline in CI |
| Hosts on non-Rust stacks (SwiftPM, Gradle, npm, .NET) | n/a | Still need `mosaic-compile`, which is not distributed today (§11 Q8) |

The first real consumer is always `mosaic-compile`. Whatever the registry,
the compiler must be able to read the result without a second package manager
installed.

### 3.2 Properties

1. **Immutable.** A published `name@version` never changes bytes.
2. **Verifiable offline.** CI builds must pass a `--locked --offline` check
   that touches no network. This is B03's rule applied to Mosaic.
3. **Provenance.** A consumer can tell that the bytes came from this
   repository, built by a specific workflow from a specific commit.
4. **No squatting surface.** Package names should not be claimable by third
   parties in a namespace consumers trust.
5. **Least secret.** No long-lived publish tokens. The RubyGems pattern in
   `publish.yml` is the counter-example.
6. **Cheap for the maintainer.** No new accounts unless one buys something
   real.
7. **Contained.** An archive may hold only declarative Mosaic sources. No
   install scripts, and no host code that runs in the consumer's build (§6.3).

---

## 4. The options

### 4.1 Summary

| | (a) crates.io | (b) npm | (c) GitHub Packages | (d) `.mospkg` on GitHub Releases | (e) git tag as source |
|---|---|---|---|---|---|
| **Fits the artifact?** | Poorly: a Rust crate that carries non-Rust data | Poorly outside web: only npm users have the client | npm/Maven/NuGet registries: same as (b). GHCR OCI artifact: yes | **Yes**, designed for it | Yes, the sources as they are |
| **mosaic-compile reads it** | Only by learning Cargo's registry cache layout, which is not a stable interface | Only by learning `node_modules` | Needs an OCI client (`oras`) or a reimplemented OCI pull | **Directly**, with one new reader (§6) | Needs `git` and a partial clone of a very large repository |
| **Non-Rust consumers** | Need Cargo | Need npm | npm/Maven/NuGet: an **auth token even for public installs**. GHCR: anonymous pulls | Anonymous HTTPS download | Need git |
| **Name squatting** | Global first-come namespace (`mosaic` already taken) | Global namespace, unless we own a scope | Namespaced by owner | **Namespaced by repository.** Only this repo's writers can create its releases | Namespaced by repository |
| **Immutability** | Yank only, no overwrite | Version numbers are never reused, but unpublish is possible inside a short window | GHCR: digests immutable, tags mutable | **Enforced**: immutable releases are on (§2.4) | Tags are mutable, and no tag ruleset exists |
| **Provenance** | Trusted publishing (OIDC) exists; first publish of a new crate needs a token | Trusted publishing and npm provenance (OIDC + Sigstore) | Artifact attestations can be pushed as OCI referrers | **GitHub release attestation, already working** (§2.4), plus optional `actions/attest-build-provenance` | Commit SHA only |
| **Offline/CI reproducibility** | `Cargo.lock` checksums | `package-lock.json` integrity | Pin by digest | Lockfile SHA-256 + cache (§8) | Pin by commit SHA |
| **Accounts and secrets** | crates.io account and an initial API token | npm account, 2FA, scope ownership, possibly an initial token | None new (`GITHUB_TOKEN` with `packages: write`) | **None new** (`GITHUB_TOKEN` with `contents: write`) | None |
| **Cost** | Free | Free | Free for public packages | Free | Free |
| **Tooling we must write** | Little to publish. A reader for Cargo's cache | Little to publish. A reader for `node_modules` | An OCI client or a subprocess | Archive writer and reader, lockfile, cache, release script | git integration |
| **Matches existing lanes** | No crates are published | No npm packages are published | No | **Yes**: the `task-app-v*` lane, reused | No |

Claims about third-party registries describe their policies as understood on
2026-10-26. Re-check them before choosing (a), (b) or (c).

### 4.2 (a) crates.io crates that embed the Mosaic sources

Turn each package into a real crate (`name = "mosaic-pkg-toolkit"`, version
taken from `mosaic-package.toml`) whose `include = [...]` ships `src/**/*.mil`
and the rest. Optionally add a tiny `lib.rs` that exposes the files through
`include_str!`.

- **For:** Cargo's lockfile, checksums and immutability come for free.
  crates.io supports trusted publishing. If `mosaic-compile` were a library
  that consumers linked, a crate dependency would resolve the package through
  the same graph.
- **Against:** `mosaic-compile` is a CLI. It would find the sources by
  guessing Cargo's on-disk registry cache layout
  (`~/.cargo/registry/src/<index>/<name>-<version>/`), which Cargo does not
  promise to keep stable. SwiftPM, Gradle and npm consumers would need Cargo
  just to fetch UI sources. The namespace is global: `mosaic` is taken, and
  every new name has to be claimed before someone else does. A crate whose
  payload is not Rust sits at the edge of what crates.io is for. The first
  publish of each crate needs a token, and 24 crates mean 24 trusted-publisher
  configurations. Today's harness `Cargo.toml` files would all need
  restructuring, because the package is not the harness.
- **When it would be right:** for the Rust crates `mosaic-compile` and the
  resolver themselves, if external Rust users want them (§11 Q8). Not for
  component packages.

### 4.3 (b) npm packages

Publish `@<scope>/mosaic-pkg-toolkit` with `files: ["mosaic-package.toml",
"src/**"]` and no scripts.

- **For:** npm provenance and trusted publishing are mature, and
  `package-lock.json` pins an integrity hash. Web hosts (React, Web
  Components, Electron) already have npm.
- **Against:** Hosts on the native backends (SwiftUI, Qt, XAML, Flutter,
  Compose) have no reason to install npm. It needs
  an npm account with 2FA and ownership of a scope, and configuring trusted
  publishing may need a first manual publish. `mosaic-compile` would have to
  learn `node_modules` resolution. npm lifecycle scripts are a well-known
  attack surface, so every consumer must install with `--ignore-scripts`, and
  every package must be checked to declare none.
- **When it would be right:** later, as a **mirror** of the `.mospkg`
  contents for web hosts that want npm. It should not be the source of truth.

### 4.4 (c) GitHub Packages

GitHub Packages hosts npm, Maven, NuGet, RubyGems and container (OCI)
registries. It has no Cargo registry.

- **npm/Maven/NuGet on GitHub Packages:** installing even a **public**
  package needs an authenticated token. Every consumer's CI would need a
  token with `read:packages`. That cost lands on every consumer forever, and
  it gains nothing over (b).
- **GHCR as an OCI artifact store:** push the `.mospkg` as an OCI artifact
  (for example with ORAS). Public packages can be pulled anonymously, digests
  are content-addressed, and attestations can live beside the artifact as OCI
  referrers. But tags are mutable, consumers need an OCI client, and package
  visibility is a separate setting from the repository's.
- **When it would be right:** as a second distribution channel for the
  same `.mospkg` bytes, if a consumer is already OCI-native.

### 4.5 (d) A Mosaic package archive on GitHub Releases

Define a small archive format, `.mospkg` (§6). It holds the manifest, the
shipped sources and a file list with per-file SHA-256 hashes. Publish each
version as an immutable GitHub Release tagged `<package>-v<version>`, with
the archive and `SHA256SUMS` attached, through a lane that mirrors
`release-task-app.yml` (§9). Consumers record the archive's SHA-256 in a
lockfile. `mosaic-compile pkg fetch` downloads and verifies it into a cache,
and the resolver reads only from that verified cache (§8).

- **For:** It is built for the artifact `mosaic-compile` actually consumes.
  It needs **no new account and no secret**. The namespace cannot be
  squatted. Immutability and release attestations are already on and already
  proven by TaskApp. It reuses an identity scheme the maintainer already runs
  (`task-app-v*`), and anyone can download it anonymously over HTTPS with no
  package manager.
- **Against:** We write the tooling: archive writer and reader, lockfile,
  cache, and release script. That is real code with a security boundary (§6.3,
  §8.4). There is no discovery or search, and no "latest compatible version"
  query without an index (§8.7). Package releases share the release list with
  TaskApp and human-languages releases. `--latest=false` keeps them from
  displacing TaskApp, but the list gets longer. GitHub API rate limits apply
  to *listing* releases, not to downloading by a known URL.

### 4.6 (e) The git tag itself as the source

Treat `mosaic-pkg-toolkit-v0.16.0` as a git tag. The resolver fetches that
package directory at that commit (a partial clone plus sparse checkout) and
locks the commit SHA, the way Go modules or SwiftPM resolve git dependencies.

- **For:** There is nothing to publish beyond a tag, and the commit SHA is a
  strong pin.
- **Against:** This repository is very large, so even a partial clone is
  costly for every consumer's CI. Tags are mutable here, because no tag
  ruleset exists. It ships whatever is in the directory, harness and all,
  rather than a reviewed set of files. And every consumer needs git.

---

## 5. Recommendation — PROPOSED, awaiting maintainer decision

> **Proposed:** adopt **(d)**. Mosaic component packages are **not** crates.
> They are published as `.mospkg` archives attached to immutable GitHub
> Releases tagged `<package-name>-v<semver>`, through a manually approved
> `workflow_dispatch` lane modelled on `release-task-app.yml`. The resolver
> consumes them only through a lockfile-pinned, verified local cache.
> `publish = false` stays on every harness `Cargo.toml`.
>
> Keep the format registry-neutral, so that (b) npm or (c) GHCR can later
> carry the **same bytes** as a mirror without changing what the lockfile
> pins.
>
> First package: `mosaic-pkg-toolkit`. It has no dependencies, every other
> consumer depends on it, and it is the most heavily gated package.

**Why (d) and not the others, in one line each:**
- (a) and (b) solve distribution for one language ecosystem, while Mosaic
  targets nine backends.
- (c)'s npm registry adds a token to every consumer's CI, and its OCI store
  adds an OCI client.
- (e) makes each consumer clone this repository.
- (d) is the only option that needs no new account or secret, cannot be
  squatted, and already has immutability and attestation working in this
  repository.

**What (d) costs**, so it can be weighed: about four implementation PRs
before the first publish (§10). Three of them add security-relevant parsing
or filesystem code, and each needs `/security-review`.

**If the maintainer prefers a registry anyway**, §6 to §8 still apply
unchanged. The archive, lockfile and cache design does not depend on where
the bytes are hosted. Only §9's publish step changes.

---

## 6. The `.mospkg` archive format (proposed)

### 6.1 Shape

A `.mospkg` file is a **ZIP archive with only stored entries** (compression
method 0), named `<name>-<version>.mospkg`:

```
mosaic-pkg-toolkit-0.16.0.mospkg
└── mosaic-pkg-toolkit-0.16.0/          exactly one top-level directory
    ├── mospkg.json                     format metadata + file list (§6.2)
    ├── mosaic-package.toml             unchanged from the source tree
    ├── README.md
    ├── CHANGELOG.md  or  CHANGELOG.d/…
    ├── LICENSE-MIT, LICENSE-APACHE     once they exist (§11 Q1)
    ├── src/<Component>.mil | .mll | .<theme>.msl | .stories.json
    └── tokens/*.json                   only files named by [styles].token_palette
```

**Why ZIP, stored only.** The repository already has a ZIP reader and writer
(`code/packages/rust/zip`, CMP09) and a SHA-256 crate
(`code/packages/rust/sha256`). It has no tar implementation, and the
shared-infrastructure preference argues for reusing what exists. Allowing
only stored entries means the reader never inflates anything, so
decompression bombs and the inflater's attack surface are out of the picture.
The toolkit is about 528 KB uncompressed, so compression buys almost nothing.
Allowing deflate later is a format-version bump (§11 Q6).

**Why one top-level directory.** An extracted archive cannot scatter files
into its parent, and the cache directory name is visible in error messages.

### 6.2 `mospkg.json`

```json
{
  "format": "mospkg/1",
  "name": "mosaic-pkg-toolkit",
  "version": "0.16.0",
  "kernel": "1",
  "styles": 1,
  "source": {
    "repository": "adhithyan15/coding-adventures",
    "path": "code/packages/mosaic/mosaic-pkg-toolkit",
    "commit": "<40-hex>"
  },
  "files": [
    { "path": "mosaic-package.toml", "size": 2913, "sha256": "<64-hex>" },
    { "path": "src/Accordion.mil",   "size": 1840, "sha256": "<64-hex>" }
  ]
}
```

`files` lists every other entry in the archive, sorted by byte order of
`path`. It is parsed with the in-house `bounded-json` crate, in line with the
serde-retirement epic (#14414). `name`, `version`, `kernel` and `styles` must
equal the values in the archived `mosaic-package.toml`, or the archive is
rejected.

### 6.3 What may and may not be in an archive

`pkg pack` builds the file list from an **allowlist**, not by excluding
files:

| Included | Excluded |
|---|---|
| `mosaic-package.toml`, `README.md`, `CHANGELOG.md` or `CHANGELOG.d/**`, `LICENSE*` | `Cargo.toml`, `Cargo.lock`, `BUILD*`, `src/*.rs`, `tests/**`, `target/**`, anything else |
| `src/*.mil`, `src/*.mll`, `src/*.msl`, `src/*.stories.json` | |
| The file named by `[styles].token_palette` | Other `tokens/` files |

`pkg pack` **refuses** a package whose manifest has `[host_effects]`,
`[host_assets]` or `[app]`. Those sections carry native host code
(`main.cpp`, Kotlin, C#) and third-party build dependencies that would
**execute in the consumer's build**. That is a different trust model, and no
library package uses them today (§2.1). Supporting them is a later format
version (§11 Q7).

### 6.4 Determinism

The same commit must produce byte-identical archives on every OS:

- File bytes come from the **git blob at the source commit**, not from the
  working tree. On Windows, a checkout's line endings can differ from the
  blob despite `.gitattributes`; a lesson records audit hashes that broke
  exactly that way.
- Entries are sorted by byte order of path, with `mospkg.json` first.
- Every entry gets the same fixed DOS timestamp (1980-01-01 00:00:00), no
  extra fields, no comments, and normalized external attributes for a
  regular file. Directory entries are omitted.

The PR lane packs the toolkit on Linux and on Windows and requires equal
digests (§9.1, gate G4).

### 6.5 Reader rules

The reader treats every archive as hostile. It checks these rules **before**
writing anything to disk. Note that a lockfile hash (§8.4) is always checked
first, so in practice the reader only parses bytes whose hash is already
pinned.

1. Size limits: the archive is at most 16 MiB, has at most 4,096 entries, and
   no entry exceeds 8 MiB. (Proposed numbers, about 30× the toolkit.)
2. Only compression method 0. The local header and the central directory
   must agree on name, sizes and CRC, and the CRC must match the data.
3. Entry names are UTF-8 and are **rejected, not lossily converted**, if
   they are not. (The in-house ZIP reader uses `from_utf8_lossy` and lets
   the first of two duplicate names win. The `.mospkg` reader must wrap it
   with stricter checks rather than inherit that behaviour.)
4. Each path is relative, uses `/`, has no `.` or `..` component, no empty
   component, no backslash, no drive letter, no NUL or control character,
   and no Windows reserved name (`CON`, `NUL`, `COM1` …). Each component
   matches `[A-Za-z0-9._-]+` and is at most 255 bytes. Every path starts
   with the single top-level directory.
5. No duplicates, including **case-insensitive** duplicates, because the
   default macOS and Windows filesystems would fold them into one file.
6. Only regular files: symlink, device and directory-with-data attributes
   are rejected.
7. The set of entries equals `mospkg.json`'s `files` plus `mospkg.json`
   itself, and every size and SHA-256 matches.
8. The archived manifest passes `mosaic-package-manifest` validation and
   §6.3's section rules.

---

## 7. Versioning and release policy (proposed)

### 7.1 One SemVer per package

- The version is **`[package].version` in `mosaic-package.toml`**. The
  harness `Cargo.toml` version is unrelated, and it stays unrelated.
- Versions are **per package**, not per component. The component program (§7)
  says "per-component SemVer". This spec diverges on purpose: a component is
  versioned through its package, because the package is what gets resolved,
  locked and downloaded. A component that needs its own release cadence
  becomes its own package (§11 Q3).
- Versions use strict SemVer 2.0.0, the same regular expression as
  `taskapp_release.py`. Build metadata (`+…`) is rejected, as the manifest
  parser already does.
- A published version is never reused, even after a failed release. That is
  the same rule `release-task-app.yml` enforces, and immutable releases make
  it physical.

### 7.2 Tag format and validation

```
<package-name>-v<version>        mosaic-pkg-toolkit-v0.16.0
                                 mosaic-std-controls-v0.3.0
                                 mosaic-pkg-sheet-v0.2.0-rc.1   (pre-release)
```

This mirrors `task-app-v*` and `engram-v*`. The `mosaic-pkg-` and
`mosaic-std-` prefixes collide with no existing tag family (`task-app-v`,
`engram-v`, `journal-v`, `venture-v`, `visicalc-v`,
`bookmarks-extension-v`, `hello-world-extension-v`, `twig-vscode-v`,
`human-languages-v`, and `<language>/<pkg>/v` in `publish.yml`). Several of
those lanes trigger on a tag *push*, so a new prefix must never be a prefix
of theirs, nor theirs of it.

Validation (`code/scripts/mosaic_package_release.py validate`, modelled on
`taskapp_release.py`):

1. `package` input matches `^mosaic-(pkg|std)-[a-z0-9]+(-[a-z0-9]+)*$`, and
   `code/packages/mosaic/<package>/mosaic-package.toml` exists and is a
   regular file, not a symlink.
2. `version` input is strict SemVer and **equals the manifest's
   `[package].version`** at the release commit.
3. `tag` input equals `f"{package}-v{version}"` exactly. The tag is
   **constructed from inputs and compared**, never parsed by prefix, so a
   package whose name ends in `-v2` cannot be confused with a version.
4. The CHANGELOG has a release heading for exactly that version (§7.5).
5. The tag does not exist and no release exists. This is checked before the
   build and **again** just before publishing, as TaskApp does.
6. The run was dispatched from `refs/heads/main`, and the commit is `main`'s
   head at dispatch.
7. Every `[dependencies]` entry is satisfied by an **already published**
   release of that dependency (§7.4). Packages are therefore published leaf
   first. The toolkit has no dependencies, which is why it goes first.

### 7.3 What is breaking

A package's public interface is everything a consumer can name: exports,
each export's slots and events, its styled part names (consumers can
override them through the style cascade, UI29 §8 Q6), its token names, and
its kernel and style-language requirements.

| Change | Breaking? | Why |
|---|---|---|
| Remove or rename an export | **Yes** | `pkg::P::C` and unqualified tags stop resolving |
| Add an export | No | Unless it collides with a name another dependency exports; the resolver reports `DuplicateExport`, and the release notes must call the new name out |
| Remove or rename a slot | **Yes** | Call-site bindings fail |
| Change a slot's type (e.g. toolkit 0.15's `options: list<list<text>> → list<text>`) | **Yes** | Bound values no longer type-check |
| Add a slot **with** a default | No | Existing call sites still compile |
| Add a slot with no default, or remove a default | **Yes** | Existing call sites become incomplete |
| Narrow a `one-of` slot's values | **Yes** | Some host values become invalid |
| Widen a `one-of` slot's values | No | Inputs accept more |
| Remove or rename an `emit` event | **Yes** | Host handlers stop firing |
| Add, remove, rename or retype an event payload field | **Yes** | Typed host closures (Swift, Kotlin, C#) have fixed signatures |
| Add an event | No | |
| Remove or rename a styled part (`[ button ]`) | **Yes** | Consumer style overrides stop applying |
| Add a part | No | |
| Remove or rename a token in `[styles].token_palette` | **Yes** | Consumer palettes override by token name |
| Require a newer `[kernel]` or `[styles]` version | **Yes** | Older toolchains cannot compile it |
| Start using a kernel primitive that an older `mosaic-compile` does not know (e.g. `HostNavigationSplit`) | **Yes, today** | `[kernel] version = "1"` has grown by additions without a bump, so the manifest cannot express "needs a compiler from after X" (§11 Q5) |
| A backend newly reports a degradation for an export | **Yes** | That backend stops rendering what it did. Recorded through the MosaicBook degradations file |
| Visual or style-value change with an unchanged interface | No (patch) | Called out in the CHANGELOG |
| Bug fix with an unchanged interface | No (patch) | |

**Bumps.** From 1.0.0 on: breaking → major, additive → minor, fixes → patch.
Before 1.0.0, Cargo's convention applies, which is what caret matching
(§7.4) assumes: **breaking → minor** (required), additive → minor or patch,
fixes → patch. The toolkit's history already follows this: 0.15.0 broke
`SegmentedControl.options`, and new components arrived in minor releases.

**Enforcement.** Until an interface-diff check exists (Phase 7, §10), the
release PR states the bump and its reason, and the reviewer checks it
against this table. The check compares the `.mil` interface IR, the part
names from `.mll` and the token names of the previous **published** archive
against the candidate, and fails when a breaking row appears without the
required bump.

### 7.4 What a `[dependencies]` version string means

Today the string is shape-checked and ignored (§2.2). Proposed semantics,
Cargo-compatible as UI29 §4.5 promised:

| Written | Meaning | Example matches |
|---|---|---|
| `"0.16.0"` (bare) | **Caret**: `>=0.16.0, <0.17.0` | 0.16.0, 0.16.3 — not 0.17.0 |
| `"1.2.3"` (bare) | Caret: `>=1.2.3, <2.0.0` | 1.2.3, 1.9.0 |
| `"0.0.3"` (bare) | Caret: `=0.0.3` | 0.0.3 only |
| `"=0.16.0"` | **Exact** | 0.16.0 only |
| `">=…"`, `"~…"`, `"*"`, ranges | **Rejected** in v1 | Each is a later addition, if anyone needs one |

Pre-releases follow Cargo: a requirement matches a pre-release only if the
requirement itself names a pre-release of the same `major.minor.patch`.

**One version per package per build.** Exports are keyed by component name,
and dependency styles merge per package, so two versions of one package in
one graph cannot coexist. Two incompatible requirements on one package fail
with `ConflictingRequirements`, naming both requirers.

**What the resolver does on a mismatch.** It fails with an error, never a
warning, for every source kind:

```
error[VersionMismatch]: task-app requires mosaic-pkg-toolkit "0.15.0" (>=0.15.0, <0.16.0)
  found mosaic-pkg-toolkit 0.16.0 at code/packages/mosaic/mosaic-pkg-toolkit (search path)
  help: update the requirement in code/programs/mosaic/task-app/mosaic-package.toml
```

Making this an error in the monorepo is what keeps a requirement string
truthful, and the five stale strings in §2.2 show what happens without it.
The same PR that turns the check on bumps those five strings. That is
behaviour-neutral, because the builds already compile against 0.16.0.

### 7.5 Changelog and release notes

- Each published version needs a release heading, `## [X.Y.Z] - YYYY-MM-DD`,
  which `validate-changelog` checks the way `taskapp_release.py` does. The
  toolkit's CHANGELOG today has only `## [Unreleased]`, with versions named
  inside subsection titles, so the first release PR restructures it. That
  PR should also convert it to `CHANGELOG.d/`, per the repo's split-changelog
  convention.
- Release notes are generated from that section. The release title is
  `<package> v<version>`.

---

## 8. Resolver and artifact-builder changes (proposed)

### 8.1 The lockfile: `mosaic-package.lock`

UI29 §8 Q4 recommended a Cargo-shaped lockfile. A consumer that uses
published packages commits one beside its manifest:

```toml
# mosaic-package.lock — written by `mosaic-compile pkg lock`. Do not edit.
version = 1

[[package]]
name    = "mosaic-pkg-toolkit"
version = "0.16.0"
source  = "github-release:adhithyan15/coding-adventures"
tag     = "mosaic-pkg-toolkit-v0.16.0"
archive = "mosaic-pkg-toolkit-0.16.0.mospkg"
sha256  = "<64-hex of the whole archive>"
dependencies = []
```

- The lockfile lists the **whole** graph, transitive dependencies included.
- The download URL is derived, never stored:
  `https://github.com/<repo>/releases/download/<tag>/<archive>`.
  `<repo>` must match `^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$`, the same pattern
  `taskapp_release.py` uses.
- Packages found on a search path (the monorepo case) are **not** locked.
  This matches Cargo's treatment of path dependencies, and it keeps every
  in-repo build exactly as it is.

### 8.2 The cache

```
$MOSAIC_PACKAGE_CACHE            default: ~/.cache/mosaic/packages (Linux),
                                 ~/Library/Caches/mosaic/packages (macOS),
                                 %LOCALAPPDATA%\mosaic\packages (Windows)
├── archives/sha256/<64-hex>.mospkg          content-addressed
└── unpacked/<name>-<version>-<first 16 hex>/ extracted, plus .mospkg-verified
```

- Every cache path is built from **validated** pieces: a manifest-valid name,
  a strict-SemVer version and a hex digest. No path is ever taken from
  inside an archive.
- Extraction writes to a fresh sibling temporary directory created with
  exclusive-create semantics, verifies, then renames into place. A
  destination that already exists, or that is a symlink, is never written
  through. (A lesson records the same rule for registry sync: check with
  `lstat`, refuse linked parents, write a temporary file in the same
  directory, then rename.)
- `.mospkg-verified` records the full archive digest. The resolver trusts
  an unpacked directory only if that digest equals the lockfile's, **and**
  it re-hashes the files against `mospkg.json` on every build. For about
  100 small files that costs milliseconds, and it catches a cache someone
  edited by hand.

### 8.3 Lookup order

There is one locator, used by both today's paths. Phase 3 replaces the two
independent locators in §2.2 with a single `PackageGraph`, computed once per
build. `build()` and `LayoutPackageResolver` both read from it, so they can
no longer pick different directories for the same name.

For each dependency, in order:

1. **Explicit override:** `--package-override <name>=<dir>`, for local
   development against an unpublished change. A warning is printed every
   time. It is refused under `--locked`.
2. **Lockfile:** if the lockfile has an entry for the name, the package
   resolves **only** from the verified cache (§8.2). Search paths are
   ignored for that name. A directory on a search path that declares the
   same name is reported as shadowed and is not used. This closes the
   dependency-confusion hole where a stray directory could replace a pinned
   package.
3. **Search paths:** if there is no lockfile entry, use today's search paths,
   with two corrections. A match is by **manifest name**. Two directories
   that declare the same name fail with `AmbiguousPackage`, instead of
   letting `read_dir` order decide.
4. **Otherwise:** `DependencyNotFound`, with a hint to run
   `mosaic-compile pkg fetch`.

Then, for every resolved package: its `[package].version` must satisfy each
requirement on it (§7.4), and a `pkg::P::C` reference must name the package
itself or a declared dependency (`UndeclaredPackageReference`). Every
existing reference already does (§2.2).

**The compiler never touches the network.** `compile` and `pkg` builds only
read. Downloading is the separate, explicit `pkg fetch` step. This answers
"path override, then cache, then fetch?": fetch is **never implicit**.

### 8.4 Integrity verification

```
publish (CI, §9)        lock / add (developer, once)        fetch / build (anyone, every time)
────────────────        ────────────────────────────        ──────────────────────────────────
pack from git blobs     download archive + SHA256SUMS       sha256(archive) == lock.sha256 ── else refuse
SHA256SUMS              gh release verify-asset (GitHub      §6.5 reader rules
immutable release         release attestation)               files == mospkg.json hashes
release attestation     optional: gh attestation verify      unpacked marker == lock.sha256
(+ build provenance)      (SLSA build provenance)
                        record sha256 in lockfile
```

- **The lockfile hash is the offline root of trust.** It is recorded once,
  when the developer adds the dependency. At that moment it is checked
  against GitHub's release attestation, so it does not rest on "trust on
  first use" alone.
- `SHA256SUMS` protects against corruption. It does **not** protect against
  a compromised release, because whoever could replace the archive could
  replace the checksums too. The attestation and immutability are what cover
  that case.
- Verifying Sigstore attestations is delegated to the `gh` CLI in v1, as an
  optional step of `pkg add`. Reimplementing Sigstore verification in Rust
  is out of scope (§11 Q4).

### 8.5 Offline mode for CI

| Flag | Effect |
|---|---|
| `--locked` | Fails if the lockfile is missing, out of date (a requirement it does not satisfy), or would change. Overrides are refused |
| `--offline` | No network. `pkg fetch --offline` only verifies that every locked archive is already in the cache with the right hash, like B03's `vendor --offline` |
| `pkg fetch --from-file <archive>` | Imports a local archive into the cache, checked against the lockfile. This lets CI test resolution with no network at all |
| `pkg vendor <dir>` (later) | Copies the locked archives into a directory the consumer commits, and the cache can be pointed there |

The recommended CI shape for a consumer is a verified `pkg fetch` step, then
`compile --locked --offline`. A network failure then shows up as a fetch
failure, never as a confusing build error.

### 8.6 New errors

`VersionMismatch`, `ConflictingRequirements`, `AmbiguousPackage`,
`UndeclaredPackageReference`, `LockfileOutOfDate`, `ArchiveDigestMismatch`,
`ArchiveRejected { rule }` (one variant per §6.5 rule, following the
manifest crate's "one cause, one variant" style), `CacheEntryCorrupt`, and
`OverrideRefusedWhenLocked`.

### 8.7 What v1 does without an index

v1 resolves **only locked, exact versions**. `pkg add <name>@<version>`
writes the lock entry for a version the developer names, and caret
requirements are then *checked* against it. Choosing the newest compatible
version automatically requires a list of what exists. There are three
options for that, which belong to Phase 7:

1. List releases through the GitHub API, filtered by exact tag construction.
   This hits rate limits when unauthenticated.
2. Commit an index file to this repository, updated by a PR after each
   release. It is reviewable and anchored in git, but every release then
   needs a follow-up PR.
3. Attach an index to a rolling release. That conflicts with immutable
   releases, so it is not recommended.

---

## 9. The release workflow shape (proposed, no YAML)

The workflow is `.github/workflows/release-mosaic-package.yml`, with a
helper script `code/scripts/mosaic_package_release.py` and tests in
`code/scripts/tests/test_mosaic_package_release.py`.

### 9.1 Jobs and gates

```
pull_request (paths: the workflow, the script, code/packages/mosaic/**,
              mosaic-package-{manifest,resolver,artifact-builder}, mosaic-compile)
workflow_dispatch (inputs: package, version, tag)
        │
        ▼
 ┌─────────────┐   G1 identity (§7.2 rules 1–7; on PRs, dry-run values)
 │  validate   │   G2 CHANGELOG heading present (dispatch only)
 └──────┬──────┘   G3 dispatch ref is main, commit is main's head (dispatch only)
        │
        ├──────────────────────────┬────────────────────────────┐
        ▼                          ▼                            ▼
 ┌─────────────┐          ┌─────────────────┐          ┌──────────────────┐
 │ pack-linux  │          │  pack-windows   │          │   test-package   │
 │ pkg pack    │          │  pkg pack       │          │ cargo test in the│
 │ pkg verify  │          │                 │          │ package (compile │
 │ digest out  │          │  digest out     │          │ + native gate),  │
 └──────┬──────┘          └────────┬────────┘          │ MosaicBook check │
        │  G4 digests equal        │                   └────────┬─────────┘
        ├──────────────────────────┘                            │
        ▼                                                       │
 ┌──────────────────────┐  G5 consumer fixture outside the repo │
 │ consume-from-archive │  builds every backend from the archive│
 │ pkg fetch --from-file│  (--locked --offline), not the tree   │
 └──────────┬───────────┘                                       │
            ├───────────────────────────────────────────────────┘
            ▼   (dispatch only, main only, environment approval)
 ┌──────────────────────────────────────────────────────────────┐
 │ publish   environment: mosaic-package-release                │
 │  G6 re-hash downloaded archive == pack-linux digest output   │
 │  G7 re-check tag + release absent                            │
 │  gh release create <tag> --target <commit> --latest=false    │
 │    [--prerelease] with .mospkg, mospkg.json, SHA256SUMS      │
 │  G8 audit: gh release verify + verify-asset for each asset;  │
 │    asset digests == SHA256SUMS                                │
 └──────────────────────────────────────────────────────────────┘
```

Pull requests run everything above `publish`, so a PR that changes the lane
tests the lane. That is the same reasoning `release-engram.yml` gives.

### 9.2 Security requirements

1. **No publish on `pull_request`.** The `publish` job runs only when
   `github.event_name == 'workflow_dispatch'` and `github.ref ==
   'refs/heads/main'`. There is no `pull_request_target` trigger and no
   `push: tags` trigger, because a tag push publishes whatever the tag points
   at.
2. **Manual approval.** `publish` uses a new environment,
   `mosaic-package-release`, with a required reviewer and a deployment-branch
   policy of `main` only. The maintainer has to create it (§11 Q10).
   Remember that the `pypi` environment `publish.yml` names does not exist,
   so it provides no gate. Do not copy that assumption.
3. **Least privilege.** The workflow-level setting is `permissions: contents:
   read`. Only `publish` gets `contents: write`, plus `id-token: write` and
   `attestations: write` if build provenance is adopted (§11 Q4). **No
   repository secrets are used**: `GITHUB_TOKEN` only.
4. **Pinned actions.** Every `uses:`, first-party `actions/*` included, is
   pinned to a 40-character SHA with a trailing `# vX.Y.Z` comment, following
   `release-venture.yml` rather than `release-task-app.yml`. The SHA is
   checked against the tag with `gh api repos/OWNER/REPO/tags`. The Rust
   toolchain is pinned to an exact version (a `RUST_VERSION` env, as in
   TaskApp), never `stable`. Lessons apply here: a branch or tag ref is not a
   pin, and the toolchain that *builds* the published bytes matters as much
   as the action that uploads them.
5. **No input interpolation into shell.** Inputs reach scripts only through
   `env:`. `publish.yml`'s inline `${{ github.event.release.tag_name }}` is
   the pattern not to copy. The `package` input is checked against the
   regular expression and a directory allowlist before it is used as a path.
6. **Checkout hygiene.** `persist-credentials: false` on every checkout in
   non-publishing jobs.
7. **Byte custody.** The bytes that get published are the bytes that were
   tested. `publish` re-hashes the downloaded artifact and compares it with
   the digest output by `pack-linux` (G6).
8. **Checksums and provenance.** `SHA256SUMS` is attached. Immutable releases
   (already enabled) make the GitHub release attestation automatic, and G8
   verifies it. Build-provenance attestation through
   `actions/attest-build-provenance` (pinned) is recommended in addition
   (§11 Q4).
9. **Serialized publication.** One concurrency group per package, with
   `cancel-in-progress: false`.
10. **`/security-review` before the push**, as CLAUDE.md requires, for the
    workflow PR and for every PR that adds parsing or filesystem code
    (Phases 2, 3 and 6).

**An interaction to fix on the way.** `publish.yml` runs on every
`release: published` event and fails on any tag that is not
`<language>/<pkg>/v…`. Releases created with `GITHUB_TOKEN` do not trigger
other workflows, which is why TaskApp releases never trip it. A package
release made by hand would. The workflow PR should make `publish.yml` skip
tags it does not own.

---

## 10. Phases

Each phase is one PR unless noted. Phase 1 is deliberately the smallest
change that makes version strings mean something.

### Phase 0 — this spec

Decide §5 and answer §11. **Out of scope:** all code.

### Phase 1 — version strings get semantics (first implementation PR, small)

- `mosaic-package-manifest`: parse a requirement as bare (caret) or `=`
  (exact), and reject other operators. Add a `VersionReq` type with
  `matches(&Version)` and Cargo's caret and pre-release rules. Write it by
  hand, without the `semver` crate, in line with the parser's existing
  zero-dependency stance.
- `mosaic-package-resolver`: both `build()` and `LayoutPackageResolver` check
  the located package's version and fail with `VersionMismatch`.
- Bump the five stale requirement strings in §2.2.
- Tests: the caret table in §7.4, pre-releases, mismatch errors from both
  locators, and every in-repo package and program still resolving.

**Out of scope:** archives, lockfile, cache, network, workflows, unifying the
two locators, and gating `pkg::` references.

### Phase 1b — locator correctness (small)

- `pkg::P::C` must name the package itself or a declared dependency
  (`UndeclaredPackageReference`).
- Two directories declaring one manifest name fail with `AmbiguousPackage`.

**Out of scope:** as Phase 1.

### Phase 2 — the `.mospkg` format

- `mosaic-compile pkg pack <dir> --commit <sha> --out <file>`, built from
  git blob bytes (§6.4), and `pkg verify <file>`. A new library crate,
  `mosaic-package-archive`, wraps the in-house `zip` and `sha256` crates.
- Tests: determinism (pack twice → same digest), one rejection test per
  §6.5 rule (traversal, case collision, duplicate, deflate entry, oversize,
  symlink attribute, file-list mismatch, a manifest with `[host_effects]`).
- Needs `/security-review`.

**Out of scope:** cache, lockfile, resolver changes, workflow, publishing.

### Phase 3 — lockfile, cache and the single locator

- `mosaic-package.lock`, `$MOSAIC_PACKAGE_CACHE`, `pkg fetch --from-file`,
  `--locked`, `--offline` and `--package-override`.
- The single `PackageGraph` from §8.3, adopted by `build()`,
  `LayoutPackageResolver` and `mosaic-package-artifact-builder`'s search-path
  defaults. With no lockfile, the behaviour is unchanged.
- A consumer fixture that copies only an app and its lockfile into a
  temporary directory **outside the repository**, imports a locally packed
  toolkit archive, and builds every backend with `--locked --offline`.
- Needs `/security-review`.

**Out of scope:** network download, the workflow, publishing, indexes.

### Phase 4 — the release lane

- `release-mosaic-package.yml`, `mosaic_package_release.py` and its tests,
  matching §9. On PRs it exercises everything except `publish`.
- The toolkit CHANGELOG restructured for a release heading and converted to
  `CHANGELOG.d/`.
- `publish.yml` taught to skip tags it does not own.
- Maintainer actions, which the PR cannot do: create the
  `mosaic-package-release` environment, and add license files (§11 Q1).

**Out of scope:** publishing anything, any package other than the toolkit,
network fetch.

### Phase 5 — first publication: `mosaic-pkg-toolkit-v0.16.0`

- The maintainer dispatches the lane and approves the environment.
- Audit, as TaskApp's releases are audited: tag and `mospkg.json` commit
  match; `gh release verify` passes; every asset digest matches `SHA256SUMS`.
- Prove resolution of the **published** package. Download the asset with
  `gh release download`, run `pkg fetch --from-file`, and build the Phase 3
  fixture with `--locked --offline` against the lockfile pinned to the
  published digest. This meets #14014's last acceptance criterion with no
  new network code in the resolver.

**Out of scope:** other packages, an in-compiler downloader.

### Phase 6 — `pkg fetch` from the network and `pkg add`

- Download by derived URL (§8.1) and verify against the lockfile. `pkg add
  <name>@<version>` writes the lock entry, with optional `gh release
  verify-asset`. The transport choice is §11 Q11.
- A manual or scheduled "consume published" job that builds the fixture from
  the real release. It does not run on PRs, so network trouble cannot turn
  PRs red.
- Needs `/security-review`.

**Out of scope:** automatic version selection, indexes, mirrors.

### Phase 7 — the rest

Publish the other packages leaf first (`grid`, `rating-controls`,
`std-foundation`, then their dependants). Add the interface-diff check
(§7.3), an index for caret selection (§8.7), and, if wanted, npm or GHCR
mirrors of the same bytes, `pkg vendor`, and pre-compiled per-backend bundles
(§11 Q9). Each is its own item in `MOSAIC-BACKLOG.md`.

---

## 11. Open questions for the maintainer

1. **License.** The manifests say `MIT OR Apache-2.0`, but the repository has
   no license text, and its README says everything is copyrighted with
   packages "licensed separately". Is `MIT OR Apache-2.0` the intended grant
   for published Mosaic packages? If it is, `LICENSE-MIT` and
   `LICENSE-APACHE` must be added (to each package, or once and copied into
   each archive) **before** the first public release. This blocks Phase 5.
2. **Is there a concrete external consumer?** No app outside this repository
   uses Mosaic packages today. If the near-term consumers are only the
   per-component test apps and the docs site, both of which live in this
   repository, is the goal "a released, immutable version to link to"
   (Phases 2, 4 and 5) rather than "a resolver that fetches" (Phase 6)? That
   would let Phase 6 wait.
3. **Per-package or per-component versions?** This spec proposes per-package
   versioning, which diverges from the component program's "per-component
   SemVer" (§7.1). Accept that divergence, and update the component program
   to match?
4. **Provenance depth.** Is the GitHub release attestation plus `SHA256SUMS`
   enough, or should the lane also emit SLSA build provenance with
   `actions/attest-build-provenance`? That adds `id-token: write` and
   `attestations: write` to `publish`. Should `pkg add` *require* `gh` for
   attestation checks, or only offer them?
5. **Kernel compatibility.** `[kernel] version = "1"` has gained primitives
   without a bump, so a package cannot say which compiler it needs. Options:
   bump the kernel version on every new primitive; add a `[kernel]
   min-revision`; or record the verifying `mosaic-compile` commit in
   `mospkg.json` and treat new-primitive adoption as breaking (§7.3).
6. **Archive container.** Is a stored-only ZIP over the in-house `zip` crate
   (§6.1) acceptable, or should it be tar plus gzip, which would mean a new
   in-house tar implementation or a third-party crate?
7. **Host code in packages.** v1 refuses `[host_effects]` and `[host_assets]`
   (§6.3). Is that the right line, given that no library package uses them
   today?
8. **Distributing `mosaic-compile`.** A consumer outside this repository
   cannot build anything without the compiler, and it is not released. Should
   there be a `mosaic-compile-v*` release lane (binaries on GitHub
   Releases), crates.io publishing of the compiler crates (option (a) is
   reasonable for *Rust* code), or "build from source at a pinned commit" for
   now? This may matter more than package publishing to an external consumer.
9. **Pre-compiled bundles.** UI29 §4.3 promised per-backend compiled
   artifacts. Should releases also attach those, for hosts that will not run
   `mosaic-compile`? If so, from which phase?
10. **Approval gate.** Create the `mosaic-package-release` environment with
    the maintainer as required reviewer, restricted to `main`? Add a tag
    ruleset that reserves `mosaic-pkg-*` and `mosaic-std-*` tags for that
    workflow?
11. **Network transport for Phase 6.** Use the in-house HTTP/1 client and
    platform TLS crates, shell out to `gh release download` or `curl`, or
    leave fetching to the consumer's CI entirely (`pkg fetch --from-file`
    only)?
12. **First version and 1.0.** Publish the toolkit as 0.16.0, its current
    manifest version, which consumers already require? When does the
    toolkit commit to 1.0.0 and the stricter bump rules that come with it?
13. **Mirrors.** If (d) is adopted, is an npm mirror for web hosts wanted at
    all, and under which scope?

---

## 12. Relationship to other specs, and what this does not do

- **UI29 §4.3** ("hosts depend on the compiled artifact, not the source"):
  this spec ships sources, because the implementation inlines sources (§2.2).
  If §5 is accepted, UI29 §4.3 should be amended to say so.
- **UI29 §4.5** ("semver, lockfile, transitive deps", Cargo-shaped): kept,
  and made concrete in §7.4 and §8.1.
- **UI29 §8 Q4** (lockfile format): answered by §8.1, subject to approval.
- **Component program §7** ("that package is published"): defined here as
  "a `.mospkg` release exists for the package's current version". The
  per-component SemVer wording diverges (§11 Q3).
- **It does not** publish anything, create accounts, tokens, tags,
  environments or releases, or change any workflow or code. All of that
  belongs to the phases in §10, after a decision.
- **It does not** change how in-repo apps build. With no lockfile, resolution
  stays on search paths. Phase 1's only behaviour change is that a
  requirement string the version on disk does not satisfy becomes an error.
