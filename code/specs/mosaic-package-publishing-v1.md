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
| Gated by `[dependencies]` | Yes | **No.** Any package on the search path can be inlined (closed by Phase 1b, §10) |
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
into a shell script. That is a script-injection path that can reach
`RUBYGEMS_API_KEY` (§9.3). A new lane should copy the first of these
patterns and neither of the other two (§9.2).

### 2.4 Repository facts that bear on the choice

These were checked on 2026-10-26.

- **The repository is public.**
- **Immutable releases are enabled** (`GET /repos/.../immutable-releases` →
  `{"enabled": true}`). After a release is published, neither its tag nor its
  assets can change.
- **GitHub release attestations already work.** `gh release verify
  task-app-v0.5.12` reports "Release task-app-v0.5.12 verified!" and lists a
  digest for each of its 13 assets. A consumer can check any asset with
  `gh release verify-asset <tag> <file>`. GitHub attests whatever is
  attached to a release, including a file uploaded by hand. So this proves
  integrity, not who built the bytes (§8.4).
- **The only deployment environment is `github-pages`.** The `pypi`
  environment that `publish.yml` names does not exist, so it carries no
  protection rules today.
- **One ruleset exists, `BlockMainPush`, and it targets branches.** It
  requires a pull request, but `required_approving_review_count` is 0 and
  `require_code_owner_review` is false, and its bypass list is empty. There
  is no `CODEOWNERS` file. So a change reaches `main` without a second
  person's approval (§8.8).
- **No tag ruleset exists.** A published release protects its own tag. But
  anyone with write access can create a tag with a future package's name, or
  a whole release with hand-made assets, which GitHub then attests
  automatically (§8.4).
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
3. **Provenance, not only integrity.** A consumer must be able to check that
   an archive was *built by the release workflow, from a commit on `main`*,
   not merely that it is the file attached to a release. Those are different
   claims (§8.4). GitHub's release attestation, which immutable releases
   produce automatically, says an asset is the one attached to a tag. That is
   integrity, and GitHub attests a hand-uploaded file just as readily. Only a
   build-provenance attestation, signed with the release workflow's identity,
   says who built the bytes.
4. **A namespace only the lane can fill.** Third parties must not be able to
   claim package names in a namespace consumers trust. Hosting the packages
   in this repository keeps outsiders out, but it does not keep out this
   repository's writers: anyone with `contents: write` can create a release.
   So package names are protected by property 3's provenance check, not by
   where the files are hosted. That check in turn trusts whatever merges to
   `main` (§8.8).
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
| **Name squatting** | Global first-come namespace (`mosaic` already taken) | Global namespace, unless we own a scope | Namespaced by owner | **Namespaced by repository.** Outsiders cannot claim names. Every repo writer can still create a release, so names are protected by the provenance check (§8.4), not by the hosting | Namespaced by repository |
| **Immutability** | Yank only, no overwrite | Version numbers are never reused, but unpublish is possible inside a short window | GHCR: digests immutable, tags mutable | **Enforced**: immutable releases are on (§2.4) | Tags are mutable, and no tag ruleset exists |
| **Provenance** | Trusted publishing (OIDC) exists; first publish of a new crate needs a token | Trusted publishing and npm provenance (OIDC + Sigstore) | Artifact attestations can be pushed as OCI referrers | **Mandatory** SLSA build provenance (`actions/attest-build-provenance`), checked against the release workflow's identity. The automatic release attestation (§2.4) gives integrity only | Commit SHA only |
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
  It needs **no new account and no secret**. Outsiders cannot claim names,
  and immutability is already on and already proven by TaskApp. Build
  provenance comes from a pinned GitHub action and needs no extra account
  (§8.4). It reuses an identity scheme the maintainer already runs
  (`task-app-v*`), and anyone can download it anonymously over HTTPS with no
  package manager.
- **Against:** Hosting alone does not stop a repository writer from making
  a release by hand. The mandatory provenance check (§8.4) does, and it makes
  the trust root "whatever merges to `main`" (§8.8). We also write the
  tooling: archive writer and reader, lockfile, cache, and release script. That is real code with a security boundary (§6.3,
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
> `workflow_dispatch` lane modelled on `release-task-app.yml`. That lane
> signs each archive with **mandatory SLSA build provenance**, and every
> consumer verifies it against the lane's workflow identity on `main`. The
> resolver consumes packages only through lockfile-pinned, content-addressed
> archives.
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
- (d) is the only option that needs no new account or secret, keeps the
  namespace inside this repository, and already has immutability working
  here. Its weak point is that any writer can create a release. Mandatory
  build provenance closes it (§8.4), which leaves the trust root at whatever
  merges to `main` (§8.8). Release attestation plus immutability gives
  integrity only, never provenance.

**What (d) costs**, so it can be weighed: five implementation PRs before the
first publish (Phases 1, 1b, 2, 3 and 4 in §10). Two of them (Phases 2 and 3)
add security-relevant parsing and filesystem code, and the lane (Phase 4)
holds write and OIDC tokens. Each of those three needs `/security-review`.
The proposal also asks the maintainer to tighten review on `main` for the
lane's own files (§8.8), because provenance cannot be stronger than the
review of the workflow that signs it.

**If the maintainer prefers a registry anyway**, §6, §7 and most of §8 still
apply. The archive, lockfile and cache design does not depend on where the
bytes are hosted. What changes is the provenance check (§8.4), which becomes
that registry's own mechanism (npm provenance, or crates.io trusted
publishing), and §9's publish step.

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

`pkg pack` **refuses**:

- A manifest with any top-level section other than `[package]`,
  `[components]`, `[dependencies]`, `[styles]` and `[kernel]`. That rules out
  `[host_effects]`, `[host_assets]` and `[app]`, which carry native host code
  (`main.cpp`, Kotlin, C#) and third-party build dependencies that would
  **execute in the consumer's build**. That is a different trust model, and
  no library package uses those sections today (§2.1). Supporting them is a
  later format version (§11 Q7). It also rules out the consumer-side
  `[sources]` table (§8.1).
- A dependency entry that is not a plain version string. A published package
  may not carry `path` or `source`, because a transitive dependency resolves
  under the *root* consumer's source policy (§8.1).
- A `[styles].token_palette` that is not exactly `tokens/<name>.json`, with
  `<name>` matching `[A-Za-z0-9_-]+`. The manifest parser accepts any safe
  relative JSON path today, so pack is stricter than it.
- A git tree entry whose mode is not `100644` or `100755`. Pack reads the
  package from the git tree at the source commit (§6.4), so this check makes
  it impossible to pack a symlink (`120000`) or a submodule gitlink
  (`160000`), even when the allowlist matches its name. On 2026-10-26 all 473
  tracked files under `code/packages/mosaic/` were `100644`. The archive does
  not record the executable bit.

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

**These rules are the security boundary by themselves.** It would be
convenient to say the reader only ever parses bytes whose hash is already
pinned. That holds for builds, but not for three other callers:

- `pkg add` parses a freshly downloaded archive before any lock entry exists.
- `pkg verify` is handed an arbitrary file.
- The release lane verifies the archive it has just packed.

So the reader must be safe on hostile input. It checks every rule below
**before** it writes anything to disk or hands any byte to a compiler.

1. **Size.** The archive is at most 16 MiB, has at most 4,096 entries, and no
   entry exceeds 8 MiB. These are proposed numbers, about 30× the toolkit.
2. **ZIP structure.**
   - There is exactly one end-of-central-directory record, it is at the end
     of the file, and its comment is **empty**. There is exactly one central
     directory. There are no ZIP64 records and no extra fields.
   - Local entries are contiguous and do not overlap. Together with the
     central directory they cover the whole file:
     - the first local header is at offset 0;
     - each entry starts where the previous one ended;
     - the central directory starts where the last entry ends.

     So there is no prefix, no gap and no trailing data.
   - Every entry uses compression method 0. General-purpose flag bit 0
     (encryption) and bit 3 (data descriptor) are clear. The only flag that
     may be set is bit 11 (UTF-8 names).
   - Each local header agrees with its central-directory record on name,
     flags, sizes and CRC-32, and the CRC matches the data.
3. **Names are UTF-8.** A name that is not valid UTF-8 is rejected. It is
   never converted lossily. (The in-house ZIP reader uses `from_utf8_lossy`
   and lets the first of two duplicate names win. The `.mospkg` reader must
   wrap it with stricter checks rather than inherit either behaviour.)
4. **Path syntax.**
   - A path is relative and `/`-separated. It has no empty component, no `.`
     or `..`, no backslash, no drive letter or colon, and no NUL or control
     character.
   - Each component matches `[A-Za-z0-9._-]+` and is at most 255 bytes. A
     component may not end in `.` or consist only of dots.
   - Every path starts with the single top-level directory.
5. **Windows device names.** Take the part of each component before its
   first `.`. The component is rejected if that part, compared
   case-insensitively, is `CON`, `PRN`, `AUX`, `NUL`, `COM0`–`COM9` or
   `LPT0`–`LPT9`. So `nul`, `Con.txt` and `com1.mil` are all rejected.
6. **No duplicates after normalisation.** The duplicate check runs after
   rules 4 and 5, on case-folded paths, because the default macOS and Windows
   filesystems fold case. Any two names one filesystem would treat as the same
   file are rejected.
7. **Only regular files.** Directory entries and entries with symlink or
   device attributes are rejected.
8. **File list.** The set of entries equals `mospkg.json`'s `files` plus
   `mospkg.json` itself, and every size and SHA-256 matches.
9. **Manifest-referenced paths are confined.** Every path the archived
   manifest names must satisfy rules 4–6, must name an entry in `files`, and
   must sit under the directory its section allows. Today that means only
   `[styles].token_palette`, which must be `tokens/<name>.json` (§6.3).
   Consumers open package files only through this verified list (§8.3).
10. **Manifest sections.** The archived manifest must pass
    `mosaic-package-manifest` validation. Format `mospkg/1` allows only
    `[package]`, `[components]`, `[dependencies]`, `[styles]` and `[kernel]`.
    Any other top-level table is rejected, never ignored: `[host_effects]`,
    `[host_assets]`, `[app]`, `[sources]`, and any section invented later.
    Dependency entries must be plain version strings.

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

Validation (`code/scripts/mosaic_package_release/release.py validate`,
modelled on `taskapp_release.py`). Every pattern in this spec is a
**whole-string** match:

- Python validators call `re.fullmatch` (or end the pattern with `\Z`). In
  Python, `$` also matches just before a trailing newline, so `re.match` with
  `...$` accepts `"mosaic-pkg-toolkit\n"`.
- Rust validators anchor the whole input.

The patterns below keep `^…$` for readability only.

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
8. **The version is not burned.** No earlier run of this workflow for the
   same tag reached the build-provenance step. G1 checks this with
   `gh run list` and `gh run view --json jobs`. Once a run has signed an
   archive, its `name@version` is never released from any other run, even if
   that run failed (§9.1).

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

### 8.1 Declared sources and the lockfile

**The consumer's manifest says where each package may come from.** A
lockfile records what was resolved. It must not also decide what is
*allowed*, or editing that one generated file could redirect a dependency.
So the allowed source lives in the consumer's `mosaic-package.toml`, which is
reviewed like code:

```toml
[sources.default]
# Where a dependency comes from when it does not name its own source.
github-release = "adhithyan15/coding-adventures"
repository-id  = 1185026968    # GitHub's numeric ID for that repository
owner-id       = 1253290       # GitHub's numeric ID for its owner

[sources.mirror-test]          # a named source, for illustration
github-release = "example-owner/example-repo"
repository-id  = 1
owner-id       = 2

[dependencies]
mosaic-pkg-toolkit = "0.16.0"                                  # from [sources.default]
mosaic-pkg-sheet   = { version = "0.2.0", source = "default" } # names a [sources.*] table
my-widgets         = { version = "0.1.0", path = "../my-widgets" }  # explicit path dependency
```

**Repository names.** `github-release` has the form `<owner>/<repo>`.
- `<owner>` must match `^[A-Za-z0-9](?:-?[A-Za-z0-9]){0,38}$`: 1–39
  characters, alphanumeric, with single hyphens only between characters.
- `<repo>` must match `^[A-Za-z0-9._-]{1,100}$`, must not be `.` or `..`, and
  must not end in `.git`.

These follow GitHub's naming rules as we understand them. GitHub does not
publish them as a formal grammar, so Phase 3 confirms them. (An earlier draft
used `^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$`, which accepts `./..` and `../..`,
and those then become path segments in the download URL.)

**Numeric IDs defeat repo-jacking.** `repository-id` and `owner-id` are the
IDs the GitHub API returns as `id` and `owner.id`. The values above were read
with `gh api repos/adhithyan15/coding-adventures` on 2026-10-26. If the owner
renames their account, someone else can register the old name and create a
repository with the old path. A name-only check would then accept that
impostor's provenance. The IDs never move, so the provenance check compares
them with the signing certificate (§8.4).

- **A bare dependency needs a default.** A bare-string dependency with no
  `[sources.default]` is an error (`NoSourceForDependency`) whenever it is not
  satisfied in workspace mode, which means with a lockfile, under `--locked`,
  or outside a workspace. In-repo manifests have no `[sources]` table and
  keep resolving from the workspace.
- The bare-string form stays valid, so every existing manifest parses
  unchanged.
- `path` is relative to the manifest, must stay inside the project root
  (§8.3), and names a directory, not an archive. A dependency cannot have
  both `source` and `path`.
- `[sources]` is a consumer-side table. A published archive may not contain
  it, and its dependencies may not use `path` or `source` (§6.5 rule 10). A
  transitive dependency resolves under the **root** consumer's source policy.

A consumer that uses published packages commits `mosaic-package.lock` beside
its manifest. UI29 §8 Q4 asked for a Cargo-shaped lockfile:

```toml
# mosaic-package.lock — written by `mosaic-compile pkg add`. Do not edit.
version = 1

[[package]]
name          = "mosaic-pkg-toolkit"
version       = "0.16.0"
source        = "github-release:adhithyan15/coding-adventures"
repository-id = 1185026968
owner-id      = 1253290
tag           = "mosaic-pkg-toolkit-v0.16.0"
archive       = "mosaic-pkg-toolkit-0.16.0.mospkg"
sha256        = "<all 64 hex of the whole archive>"
commit        = "<40-hex source commit, as verified in the provenance>"
attested      = true
dependencies  = []
```

**`sha256` and `attested` are claims, not proofs.** Anyone who can edit the
lockfile can change either one. What makes an archive trustworthy is its
Sigstore bundle, which is checked on every import (§8.4). `attested` only
records how the entry was created, and no check relies on it alone.

**Every field is rebuilt and compared on every use. None is trusted because
it was stored.**

| Check | Fails with |
|---|---|
| `lock.source`, `lock.repository-id` and `lock.owner-id` equal the source the consumer manifest declares for that name | `LockSourceMismatch` |
| `lock.tag == f"{name}-v{version}"` and `lock.archive == f"{name}-{version}.mospkg"` | `LockEntryMalformed` |
| `sha256(archive bytes) == lock.sha256` | `ArchiveDigestMismatch` |
| `mospkg.json.name == lock.name` and `mospkg.json.version == lock.version` | `ArchiveIdentityMismatch` |
| The single top-level directory is `{name}-{version}` | `ArchiveIdentityMismatch` |
| `mospkg.json.source.commit == lock.commit`, and `mospkg.json.source.repository` is the declared source's repository | `ArchiveIdentityMismatch` |
| The archived `mosaic-package.toml` agrees with `mospkg.json` (§6.2) | `ArchiveRejected` |
| `attested = true` and a bundle present beside the archive, under `--locked` | `UnattestedLockEntry`, `MissingProvenanceBundle` |

Version requirements (§7.4) are checked against the **archived manifest's**
`[package].version`, not against the lockfile's copy of it.

- The lockfile lists the whole graph, transitive dependencies included.
- The download URL is built, never stored:
  `https://github.com/<owner>/<repo>/releases/download/<tag>/<archive>`.
  `<owner>/<repo>` comes from the manifest-declared source and must satisfy
  the repository-name rules above. The bundle's URL is the same with
  `.sigstore.jsonl` appended.
- Path and workspace packages are not locked, just as Cargo does not lock
  path dependencies.

### 8.2 The cache

**The content-addressed archive is the only trusted object.** Nothing
extracted from it is trusted by itself.

```
$MOSAIC_PACKAGE_CACHE         default: ~/.cache/mosaic/packages (Linux),
                              ~/Library/Caches/mosaic/packages (macOS),
                              %LOCALAPPDATA%\mosaic\packages (Windows)
├── archives/sha256/<all 64 hex>.mospkg
└── archives/sha256/<all 64 hex>.sigstore.jsonl   the archive's provenance bundle(s)
```

**The cache holds no trusted root.** A trusted root names the Fulcio
certificate authorities and Rekor keys that verification accepts. Anyone who
could write it could substitute their own CA and log keys, then sign
anything. So the only root ever used is the consumer's committed
`mosaic-trusted-root.jsonl` (§8.4). A file named like a trusted root inside
the cache is ignored, and Phase 3 tests that.

Every archive in the cache has its Sigstore bundle beside it. The bundle is
the file `gh attestation download` writes (JSON Lines, one bundle per line),
and the lane attaches the same bundle to the release (§9.1). An archive that
arrives without a bundle is never imported.

- **Every build re-hashes the archive against the lockfile, then reads the
  sources from those same bytes.** The resolver reads the archive into memory
  once (it is capped at 16 MiB, §6.5), hashes that buffer and parses that
  buffer. It never hashes one read and parses another, so swapping the file
  between the two achieves nothing. For the toolkit that is about 0.5 MB and
  a few milliseconds.
- **No unpacked directory is trusted.** v1 never extracts to disk for a
  build. A `pkg unpack` command may extract a copy for people to read, but
  the resolver never reads that copy. (An earlier draft trusted unpacked
  directories that carried a `.mospkg-verified` marker. Anyone who can write
  to the cache can forge such a marker, so it proved nothing.)
- **Names.** Cache file names use the full 64-hex digest, never a prefix.
  Every cache path is built from validated parts. Nothing from inside an
  archive ever becomes a filesystem path during a build.
- **Filesystem rules.**
  - The cache root is canonicalized once, and every path component below it
    is checked with `lstat`. A symlink is refused.
  - The cache root and `archives/sha256/` are created with mode 0700 and
    must be owned by the current user. A cache directory that is group- or
    world-writable, or owned by another user, is refused.
  - A new archive is written to an exclusive-create temporary file in the
    same directory, flushed, verified, and then renamed into place. A lesson
    records the same rule for registry sync.
  - **Windows, fixed here rather than left to Phase 3:**
    - Every path component under the cache root is opened without following
      reparse points (`FILE_FLAG_OPEN_REPARSE_POINT`). Any component with
      `FILE_ATTRIBUTE_REPARSE_POINT` set is refused, which covers symlinks,
      junctions and mount points alike.
    - The cache root and `archives\sha256\` are created with a **protected**
      DACL that inherits nothing from the parent. The owner is the current
      user's SID, and the entries grant access only to that SID, `SYSTEM` and
      `BUILTIN\Administrators`.
    - On every use, the owner SID must be the current user's. No ACE for any
      other principal may grant a write-class right: `GENERIC_WRITE`,
      `GENERIC_ALL`, `FILE_WRITE_DATA`/`FILE_ADD_FILE`,
      `FILE_APPEND_DATA`/`FILE_ADD_SUBDIRECTORY`, `FILE_WRITE_EA`,
      `FILE_WRITE_ATTRIBUTES`, `FILE_DELETE_CHILD`, `DELETE`, `WRITE_DAC` or
      `WRITE_OWNER`. Otherwise the cache is refused (`CacheUnsafe`).
    - Writes go through handles opened relative to the verified directory
      handle where the platform allows it. A lesson records that a directory
      handle alone does not make later path-based writes safe from a
      directory swap.
- **What this protects.** The re-hash detects corruption and any cached
  archive that does not match the lockfile. Provenance is checked when an
  archive is imported and by `pkg fetch` (§8.4), not here. It does not defend
  against an attacker who can already write both the lockfile and the cache,
  or who can run code as the user, because that attacker controls the build
  anyway.

### 8.3 Lookup order

There is one locator. Phase 3 replaces the two locators of §2.2 with a single
`PackageGraph`, computed once per build. `build()`, `LayoutPackageResolver`
and the artifact builder all read from it, so they can no longer pick
different directories for one name.

Each resolved package exposes its files only through a **verified file list**
(§6.5 rule 9):

- for an archive, the entries in `mospkg.json`;
- for a path or workspace package, the files `pkg pack`'s allowlist would
  select.

The resolver and the artifact builder open package files only through that
list. A manifest therefore cannot point them at any other file.

**Ambient search paths are a monorepo convenience, not a source of
packages.** Today the search paths come from two places: the ancestors of the
package root, and the working directory (§2.2). Outside this repository,
that means any `packages/` directory above a project, or below the shell's
current directory, can supply a dependency. That is dependency confusion. So
ambient search paths are allowed only in **workspace mode**:

- Workspace mode is detected positively. Some ancestor of the package root,
  at or below the project root, must contain a `mosaic-workspace.toml` that
  lists the workspace's package roots. Phase 3 adds one to this repository,
  at `code/`, listing `packages/` and `packages/mosaic/` (§11 Q15).
- `mosaic-compile`'s default of `code/packages` relative to the working
  directory also applies only in workspace mode.

**Discovery must not be spoofable.** CVE-2022-24765 was git trusting a
`.git` directory planted in a shared ancestor such as `C:\`. The same pattern
applies to the marker, so discovery follows these rules:

1. **The project root is the nearest enclosing work tree, found by walking
   up from the package root.** The walk looks for a `.git` directory or file
   itself. It never runs `git`, and it ignores `GIT_DIR`, `GIT_WORK_TREE`,
   `GIT_CEILING_DIRECTORIES` and every other git environment variable and
   setting. If no work tree is found, the project root is the package root.
2. **Ownership stops the walk.** Each directory the walk enters must be
   owned by the current user (on Windows, the owner SID must be the current
   user's). The walk stops at the first directory that is not. A `.git` or
   `mosaic-workspace.toml` in such a directory, or above it, is ignored.
3. **Home is never a root.** The user's home directory and its ancestors are
   never a project root or a marker location, so a dotfiles repository in
   `~` cannot turn every project into a workspace.
4. **Marker entries are confined.** Each package root a
   `mosaic-workspace.toml` lists must be a relative path without `..`, must
   not be absolute, and must canonicalize to a directory under the marker's
   own directory.
5. **Path and workspace packages are confined too.** While a package
   directory is read, every file in it is checked with `lstat`, and symlinks
   are refused (reparse points on Windows). Every file opened must
   canonicalize to a path under the package root. The same applies to
   explicit `path` dependencies, which must also stay inside the project
   root.

Each dependency resolves through the first of these that applies:

1. **Explicit override:** `--package-override <name>=<dir>`, for developing
   against an unpublished change. It prints a warning every time, and
   `--locked` refuses it.
2. **Explicit path dependency:** `path = "…"` in the consumer manifest
   (§8.1).
3. **Lockfile:** a locked name resolves only from the verified cache (§8.2).
   Search paths are never consulted for it. A workspace directory that
   declares the same name is reported as shadowed.
4. **Search paths:** workspace search paths, or an explicit
   `--package-search-path`. These are used only when the consumer has **no
   lockfile**, and never under `--locked`. Matching is by manifest name, and
   two directories that declare the same name fail with `AmbiguousPackage`.
5. **Otherwise:** `DependencyNotFound`, with a hint to run
   `mosaic-compile pkg add`.

**With a lockfile, and always under `--locked`, every dependency other than
the root package must be locked or be an explicit path dependency.**
Anything that would resolve from an ambient or `--package-search-path`
directory fails with `AmbientResolutionRefused`. In-repo builds have no
lockfile and run in workspace mode, so they keep working exactly as today.

Then two checks apply to every resolved package. Its `[package].version`
must satisfy each requirement on it (§7.4). And a `pkg::P::C` reference must
name the package itself or one of its declared dependencies, or the build
fails with `UndeclaredPackageReference`. Every existing reference already
does (§2.2).

**The compiler never touches the network.** `compile` and `pkg` builds only
read. Downloading happens in the separate, explicit `pkg add` and `pkg
fetch` commands. So the answer to "path override, then cache, then fetch?"
is: fetching is **never implicit**.

### 8.4 Integrity and provenance

Two GitHub mechanisms are easy to confuse, so here is what each one does.

| | Release attestation | Build-provenance attestation |
|---|---|---|
| Produced by | GitHub, automatically, for every release in a repository with immutable releases | The release workflow, through `actions/attest-build-provenance` (which needs `id-token: write` and `attestations: write`) |
| Says | "This asset is the one attached to this tag" | "These bytes were built by workflow W, on ref R, at commit C, on a GitHub-hosted runner" |
| Checked with | `gh release verify`, `gh release verify-asset` | `gh attestation verify` |
| Proves | **Integrity** | **Provenance** |
| A writer who uploads hand-made bytes | **Passes**, because GitHub attests whatever was attached | **Fails**, because the bytes carry no signature from the release workflow on `main` |

Release attestation plus immutability gives integrity: the bytes cannot
change after publication. They do not give provenance. Any of these could
create `mosaic-pkg-toolkit-v0.17.0` with arbitrary bytes, and
`gh release verify` would still pass:

- any repository writer;
- an agent's PAT;
- a workflow token with `contents: write`, including one dispatched from a
  branch whose copy of the workflow drops `environment:`.

**So build provenance is mandatory, and every consumer checks it.** The
lane's gates G7a and G8, and `pkg add`, all run:

```
gh attestation verify <archive> \
  --repo adhithyan15/coding-adventures \
  --signer-workflow adhithyan15/coding-adventures/.github/workflows/release-mosaic-package.yml \
  --source-ref refs/heads/main \
  --source-digest <mospkg.json.source.commit> \
  --deny-self-hosted-runners
```

- `--repo`, and the repository inside `--signer-workflow`, come from the
  source the consumer manifest declares (§8.1), never from the archive.
- `--source-digest` makes the attested source commit equal
  `mospkg.json.source.commit`.
- The default predicate type, SLSA provenance v1, is kept.
- These flags were checked against the `attestation verify --help` output
  of `gh` 2.92.0 on 2026-10-26.
- **Numeric IDs.** The check runs with `--format json`. The caller then
  requires the certificate's `sourceRepositoryIdentifier` and
  `sourceRepositoryOwnerIdentifier` to equal the manifest's `repository-id`
  and `owner-id`, and otherwise fails with `RepositoryIdentityMismatch`.
  - `gh`'s help confirms that its JSON output includes
    `verificationResult.signature.certificate`, a parsed form of the
    certificate.
  - The two field names are the names of Fulcio's certificate extensions.
    They have **not** been checked against a real attestation from this
    repository, because none exists yet. Phase 3 pins them against a real
    bundle.

**Offline verification of every import.** The same check runs without a
network by adding `--bundle <archive>.sigstore.jsonl --custom-trusted-root
<project>/mosaic-trusted-root.jsonl`. `gh` 2.92.0 documents both flags, and
documents `gh attestation trusted-root` as the way to obtain that file for
offline use.

- **Exactly one trusted root is used: the committed one.** The root covers
  the Sigstore public-good instance and GitHub's own Sigstore instance.
  - It is the file `mosaic-trusted-root.jsonl` in the same directory as
    `mosaic-package.lock`, found relative to the lockfile. It is opened with
    the same `lstat`/no-symlink rules as package files (§8.3 rule 5).
  - Every `--custom-trusted-root` argument the tools pass is that path. No
    flag, environment variable or cache file can substitute another root.
    If the file is missing, verification fails (`MissingTrustedRoot`); it
    never falls back to `gh`'s online default.
  - The cache holds no trusted root (§8.2).
- **The trusted root is as sensitive as the lockfile.** Changing it decides
  which signing authorities a consumer accepts. Consumers should treat a
  change to `mosaic-trusted-root.jsonl` like a change to
  `mosaic-package.lock`: reviewed, and covered by `CODEOWNERS` where the
  consumer uses it.
- **Refresh it deliberately, and watch for drift.**
  - It is refreshed only by a reviewed PR that runs `gh attestation
    trusted-root`, which verifies the root through Sigstore's TUF
    repository. It is never refreshed implicitly during a build.
  - A stale root does not fail safely in every case. When a key *rotates*
    normally, a stale root rejects bundles signed under the new key, so it
    fails closed. But a stale root also **keeps trusting keys that were
    revoked or retired**, possibly after a compromise, until someone
    refreshes it. That direction fails open.
  - So the proposal is a **scheduled CI job**, weekly for example, in this
    repository and recommended to consumers. It compares the committed root
    with a fresh `gh attestation trusted-root` and opens a PR or an alert
    when they differ. It never commits the new root itself.
- **Every way bytes enter the cache runs the full check against the bundle,
  and refuses to import if it fails:**
  - a network `pkg fetch` or `pkg add`;
  - `pkg fetch --from-file` and `pkg add --from-file`;
  - importing a vendored directory;
  - `pkg fetch --offline`, which re-verifies everything already cached.
- **`pkg add` also binds the archive to its release.** Online, `pkg add`
  (including `--from-file`) also runs `gh release verify-asset -R
  <owner>/<repo> <tag> <file>`. `<owner>/<repo>` comes from the
  manifest-declared source (§8.1), never from the archive or the current
  directory's git remote. `<tag>` is constructed as `{name}-v{version}`.
  That rejects an archive that was attested but never published, such as the
  orphan of a failed run (§9.1), even when a correctly signed bundle exists
  for it.
- **The orphan path that remains.** Offline imports (`pkg fetch
  --from-file`, `pkg fetch --offline`, vendored directories) do not call
  `verify-asset`, because it needs the network. They accept any archive that
  matches a lock entry and has a valid bundle. So an orphan can enter a build
  only through a lock entry that `pkg add` never wrote, which means a
  hand-edited lockfile. Review of the lockfile is the control there. The
  lane's burned-version check (§7.2 rule 8) cannot replace it: anyone with
  `actions: write` can delete workflow runs and erase the history that check
  reads. **`verify-asset` in `pkg add` is the real control against orphans.**
- **`--insecure-skip-attestation` is the only way around these checks.**
  It exists for development and air-gapped experiments. It prints a warning,
  imports without verification, and records `attested = false`. `--locked`
  refuses such an entry (`UnattestedLockEntry`). Apart from this explicit,
  recorded opt-out, v1 is **not** trust-on-first-use.

**The chain of checks:**

```
publish (CI, §9)                  pkg add (once, online)                  every import / pkg fetch (offline ok)
────────────────                  ──────────────────────                  ────────────────────────────────────
pack from git blobs               §6.5 reader rules on the download       sha256(bytes) == lock.sha256
attest-build-provenance           gh attestation verify (above)           §6.5 reader rules on those bytes
G7a: verify provenance            gh release verify-asset -R … <tag> …    gh attestation verify --bundle
SHA256SUMS from verified bytes    §8.1 identity, source and ID checks       --custom-trusted-root <committed root>
bundle attached to the release    store archive + bundle                  §8.1 identity, source and ID checks
G8: verify as consumers do        write lock: sha256, commit, IDs,        store archive + bundle
                                    attested = true
                                                                          compile --locked --offline:
                                                                          sha256 == lock, bundle present,
                                                                          §6.5 + §8.1 checks; no Sigstore
```

**What `compile` alone trusts.** `compile` does not run the Sigstore check
itself, so it does not need `gh`. It re-hashes each archive against the
lockfile, requires its bundle to be present under `--locked`
(`MissingProvenanceBundle`), and applies the §6.5 and §8.1 checks. The only
way an archive gets into the cache with a bundle is through an import that
verified it. But a cache that was pre-filled or restored by some other means
(an `actions/cache` restore, a copied directory) bypasses import. In that
case `compile`'s guarantee is only **"these bytes match the committed
lockfile"**, and the anchor is **review of the lockfile**, not provenance.
That is why the recommended CI shape (§8.5) always runs `pkg fetch
--offline`, which re-verifies every cached archive against its bundle,
before `compile --locked --offline`. This corrects an earlier draft, which
said offline builds "inherit" the provenance check. They inherit it only
when an import or a `pkg fetch` in the same trust domain put the bytes
there.

- **`SHA256SUMS` is not a security control.** It catches corruption in
  transfer and gives people a familiar check. Anyone who could replace an
  asset could also replace the checksums.
- **v1 delegates Sigstore verification to the `gh` CLI.** A Rust verifier is
  out of scope (§11 Q4).

### 8.5 Offline mode for CI

| Flag | Effect |
|---|---|
| `--locked` | Fails in any of these cases: the lockfile is missing; it does not satisfy a requirement; it would change; an entry has `attested = false`; an archive has no bundle beside it; the lockfile's source or IDs differ from the manifest's; a dependency would resolve from an ambient path. Overrides are refused |
| `--offline` | No network. `pkg fetch --offline` re-verifies every locked archive already in the cache: hash, §6.5 rules, and the bundle, checked against the committed `mosaic-trusted-root.jsonl` beside the lockfile and never against anything in the cache. This is B03's `vendor --offline`, plus provenance |
| `pkg fetch --from-file <archive> --bundle <file>` | Imports a local archive and its bundle into the cache if they match an existing lock entry and pass the offline provenance check. It never creates a lock entry |
| `pkg add --from-file <archive>` | Creates a lock entry from a local archive, running the online checks (`gh attestation verify`, and `gh release verify-asset -R <declared owner/repo>`). Under `--insecure-skip-attestation` it records `attested = false` |
| `pkg vendor <dir>` (later) | Copies each locked archive **and its bundle** into a directory the consumer commits. Importing from that directory runs the offline check like any other import |

A consumer's CI should run `pkg fetch` first (online), or `pkg fetch
--offline` over a vendored or restored cache, and then
`compile --locked --offline`. A network or provenance failure then appears in
the fetch step, never as a confusing build error.

### 8.6 New errors

`VersionMismatch`, `ConflictingRequirements`, `AmbiguousPackage`,
`UndeclaredPackageReference`, `AmbientResolutionRefused`,
`LockfileOutOfDate`, `LockSourceMismatch`, `LockEntryMalformed`,
`UnattestedLockEntry`, `MissingProvenanceBundle`, `MissingTrustedRoot`,
`ProvenanceRejected`,
`RepositoryIdentityMismatch`, `ReleaseBindingRejected`, `NoSourceForDependency`,
`ArchiveDigestMismatch`, `ArchiveIdentityMismatch`,
`ArchiveRejected { rule }` (one variant per §6.5 rule, in the manifest
crate's "one cause, one variant" style), `CacheUnsafe` (a symlink, the wrong
owner, or loose permissions) and `OverrideRefusedWhenLocked`.

### 8.7 What v1 does without an index, and how a bad release is withdrawn

v1 resolves **only locked, exact versions**. `pkg add <name>@<version>`
writes the lock entry for a version the developer names. Caret requirements
are then *checked* against it, not used to choose. Choosing the newest
compatible version automatically needs a list of what exists. That belongs
to Phase 7, and there are three ways to provide it:

1. List releases through the GitHub API, filtering by exact tag
   construction. Unauthenticated callers hit rate limits.
2. Commit an index file to this repository and update it by PR after each
   release. It is reviewable and anchored in git, but every release needs a
   follow-up PR.
3. Attach the index to a rolling release. That conflicts with immutable
   releases, so it is not recommended.

**Yanks and advisories.** An immutable release cannot be edited, so a bad
version is withdrawn by listing it, not by changing it. The Phase 7 index
carries a yank and advisory list: `name`, `version`, `kind` (`yanked` or
`advisory`), a reason and a link. `pkg add` and `pkg fetch` consult it when
they are online. Until the index exists, a withdrawal is recorded in the
package's CHANGELOG and in a GitHub issue, and consumers are not
warned. Whether a yank blocks a version or only warns is §11 Q14.

### 8.8 The trust root is whatever merges to `main`

The provenance check pins the *workflow file on `main`*. It cannot tell a
good release workflow from a bad one. Anyone who can change the workflow, the
script it runs, or **the toolchain that produces the bytes**, and get that
change merged to `main`, can publish attested bytes. So the trust root of
every published package is **whatever merges to `main`**.

**A change to the packer is a change to what gets signed.** The provenance
signature says "the release workflow built this from commit C". It says
nothing about whether `pkg pack` at commit C was honest. A packer that adds a
file, drops a file or rewrites bytes produces an archive that verifies
perfectly.

Today the bar for merging is low (§2.4). `BlockMainPush` requires a pull
request but zero approvals, code-owner review is off, there is no
`CODEOWNERS` file, and PRs routinely merge through auto-merge. There is a
second gap: **the Rust workspace's `Cargo.lock` is not tracked** (it is
listed in `.gitignore`). So the packer's third-party dependencies, which
include `serde`, `toml` and `regex` through `mosaic-package-manifest`, resolve
afresh on every build.

The proposal (§11 Q10):

- **Review everything that decides the signed bytes.** Add `CODEOWNERS`
  entries, and turn on required code-owner review in the `main` ruleset, or
  require one approving review on these paths:
  - `.github/workflows/release-mosaic-package.yml`;
  - `code/scripts/mosaic_package_release/`, a dedicated directory that
    holds only the release script (§9.2 item 8), and its tests;
  - the pack toolchain: `mosaic-compile`'s `pkg pack` and `pkg verify`
    commands, `mosaic-package-archive`, `mosaic-package-manifest`, and the
    `zip`, `deflate`, `sha256` and `bounded-json` crates;
  - the packer's lockfile (next bullet);
  - `.github/CODEOWNERS` itself, because whoever can edit it can remove
    every other entry in this list.
- **Protect the protection.** The `main` ruleset's bypass list, its
  required-review settings, and the `mosaic-package-release` environment's
  reviewers are repository settings, not files, so `CODEOWNERS` cannot cover
  them. The proposal is that they change only by a maintainer action recorded
  in an issue. Any bypass actor on `main` is, in effect, a signer of every
  package. Today the bypass list is empty (§2.4).
- **Pin the packer's dependency graph.** Phase 2 builds the packer as a
  standalone Cargo project with a **tracked** `Cargo.lock`. Some
  `mosaic-pkg-*` harnesses already track their own lockfile, so there is a
  precedent. The lane builds it with `cargo build --locked`, and that
  lockfile is on the review list above. With the workspace lockfile
  untracked, `--locked` would pin nothing.
- **Keep the `mosaic-package-release` environment's required reviewer.** It
  gates *running* the lane, not *changing* it, so it does not replace the
  review above.
- **Do not rely on a tag ruleset to single out the lane.** A ruleset's bypass
  list accepts only users, teams, repository roles, organization admins,
  deploy keys and GitHub Apps ("Integration"). This was checked against the
  REST rulesets documentation on 2026-10-26. `GITHUB_TOKEN` acts as the
  GitHub Actions app, so letting that app create `mosaic-pkg-*` tags lets
  *every* workflow create them. A tag ruleset can narrow who creates tags,
  but it cannot make one workflow the only creator. Provenance is the
  control that can.

---

## 9. The release workflow shape (proposed, no YAML)

The workflow is `.github/workflows/release-mosaic-package.yml`. Its helper,
`code/scripts/mosaic_package_release/release.py`, lives in a directory that
holds nothing else (§9.2 item 8) and uses only the Python standard library.
Its tests live in `code/scripts/tests/test_mosaic_package_release.py`.

### 9.1 Jobs and gates

```
pull_request (paths: the workflow, the script directory, code/packages/mosaic/**,
              the packer and its lockfile, mosaic-package-{manifest,resolver,archive,artifact-builder})
workflow_dispatch (inputs: package, version, tag)
        │
        ▼
 validate        G1 identity (§7.2 rules 1–8; dry-run values on PRs)
                 G2 CHANGELOG heading present (dispatch only)
                 G3 ref is main and github.sha is main's head (dispatch only)
                 output: commit = github.sha   (every later job checks out exactly this)
        │
        ├────────────────────┬────────────────────┐
        ▼                    ▼                    ▼
 pack-linux            pack-windows          test-package
 build packer --locked build packer --locked cargo test in the package
 pkg pack              pkg pack              (compile + native gate),
 pkg verify            digest output         MosaicBook check
 upload archive
 digest output
        │  G4 the two digests are equal
        ▼
 consume-from-archive  G5 build the standalone packer with cargo build --locked at the validated
                          commit (no cache); download pack-linux's artifact; sha256 == its digest;
                          pkg verify --identity-out identity.json with that packer (all §6.5 rules);
                          a fixture outside the repo builds every backend from it (--offline);
                          upload identity.json
        │
        ▼
 publish   needs: [validate, pack-linux, pack-windows, test-package, consume-from-archive]
           runs only if every one of them succeeded · dispatch only · main only
           environment: mosaic-package-release (manual approval)
           sparse checkout of code/scripts/mosaic_package_release/ only; python3 -I
   G6  download pack-linux's artifact and G5's identity.json;
       sha256 == pack-linux's digest == identity.json's sha256;
       stdlib-only structural assertions (below); extract mospkg.json;
       name, version, source.commit == the validated inputs and commit
   attest-build-provenance on the archive (subject = the archive's digest; keep bundle-path)
       ── from here on, this name@version is BURNED: never dispatched again ──
   G7a gh attestation verify with the §8.4 flags, plus the --bundle form against the saved bundle
   SHA256SUMS and the release copy of mospkg.json, taken from the verified bytes
   G7  tag and release still absent (checked again)
   create the tag atomically:
       POST /repos/{owner}/{repo}/git/refs  ref=refs/tags/<tag>  sha=<validated commit>
       any response but 201 fails the job; read the ref back, it must name that commit
   gh release create <tag> --verify-tag --latest=false [--prerelease]
       --title "<package> v<version>" --notes-file notes.md
       assets: <name>-<version>.mospkg, <name>-<version>.mospkg.sigstore.jsonl,
               mospkg.json, SHA256SUMS
   G8  audit, exactly as a consumer would:
       gh attestation verify (§8.4 flags, online and --bundle) · gh release verify
       gh release verify-asset <tag> <file> for each asset · asset digests == SHA256SUMS
       tag commit == validated commit == mospkg.json.source.commit
```

**What `publish` depends on.** `publish` declares `needs:` on all five
earlier jobs: validate, pack-linux, pack-windows, test-package and
consume-from-archive. It uses no `always()` or `!cancelled()` condition, so
GitHub's default applies: if any needed job fails, is skipped or is
cancelled, `publish` does not run. An earlier draft of the diagram drew
`test-package` without an edge to `publish`. That was wrong: the package's
own tests and its MosaicBook gate are release gates.

Pull requests run every job above `publish`, so a PR that changes the lane
tests the lane. `release-engram.yml` gives the same reason for its PR runs.

**G6's structural assertions.** These use the standard library only.
Python's `zipfile` silently keeps the *last* of two entries with the same
name, so G6 does not trust its view of the archive. Before reading anything,
it asserts all of the following, and fails on any mismatch:

- every name in `ZipFile.infolist()` is unique;
- entry 0 is `<name>-<version>/mospkg.json`, stored (method 0);
- the number of entries is `len(mospkg.json["files"]) + 1`;
- every entry is stored, and none sets flag bit 0 or bit 3;
- the archive's sha256, entry count and top-level directory equal those in
  the identity record `identity.json`, which G5's Rust `pkg verify` emitted
  after it applied every §6.5 rule.

**Notes on the publish steps**

- **The tag is created atomically.** The git refs endpoint creates a
  reference or fails, and the REST documentation lists `422 Validation
  failed` for it. The page does not say explicitly that an existing ref is
  one such failure, although that is GitHub's long-standing behaviour. So
  the script treats any status other than 201 as failure, and Phase 4's
  tests pin that. `gh release create --verify-tag` then refuses to run
  unless the tag already exists, so `gh` never creates a tag of its own at
  some other commit.
- **A version is burned once its run reaches the attestation step.** If such
  a run fails later, at G7a, at tag creation or at release creation, a
  signed archive exists that no release will ever carry. Two defences apply:
  - **The procedure:** that `name@version` is never dispatched again, and the
    next patch version is cut instead. §7.2 rule 8 enforces this. G1 lists
    earlier runs of this workflow with `gh run list` and `gh run view --json
    jobs`, and fails if any run for the same tag reached the attestation
    step.
  - **The consumer check:** `pkg add` runs `gh release verify-asset` against
    the constructed tag (§8.4), and an orphan archive is attached to no
    release, so it fails.

  The lane never deletes tags. If an orphan *tag* exists without a release,
  the maintainer may delete it by hand, but the version stays burned.
- **G7a runs before the tag exists**, so a provenance failure leaves nothing
  public. G8 repeats the check after publication, along with the release
  attestation. If G8 ever fails, that is an incident: the version is listed
  as yanked (§8.7) and is never reused.

### 9.2 Security requirements

1. **No publish on `pull_request`.** `publish` runs only when
   `github.event_name == 'workflow_dispatch'` and `github.ref ==
   'refs/heads/main'`, and only after every needed job has succeeded (§9.1).
   There is no `pull_request_target` trigger and no `push: tags` trigger,
   because a tag push publishes whatever the tag points at.
2. **Manual approval.** `publish` uses a new environment,
   `mosaic-package-release`, with a required reviewer and a deployment-branch
   policy of `main` only. The maintainer must create it (§11 Q10). The `pypi`
   environment that `publish.yml` names does not exist, so it gates nothing;
   this lane must not make the same assumption. Approval gates *running* the
   lane. Review on `main` gates *changing* it (§8.8).
3. **Least privilege.** The workflow-level default is `permissions:
   contents: read`. Only `publish` gets `contents: write`, `id-token: write`
   and `attestations: write`. **No repository secrets are used**; the lane
   runs on `GITHUB_TOKEN` alone.
4. **Pinned actions and toolchains.**
   - Every `uses:` is pinned to a 40-character SHA with a trailing
     `# vX.Y.Z` comment. That includes first-party `actions/*` and
     `actions/attest-build-provenance`, following `release-venture.yml`
     rather than `release-task-app.yml`. Each SHA is checked against its tag
     with `gh api repos/OWNER/REPO/tags`.
   - The Rust toolchain is pinned to an exact version (a `RUST_VERSION` env,
     as in TaskApp), never `stable`.
   - The packer is built with `cargo build --locked` against its tracked
     lockfile (§8.8).
   - Lessons apply: a branch or tag ref is not a pin, and the toolchain that
     *builds* the published bytes matters as much as the action that uploads
     them.
5. **No input interpolation into shell.** Inputs reach scripts only through
   `env:`. The `package` input is checked against the §7.2 pattern and a
   directory allowlist before it is used as a path.
6. **Checkout hygiene.** Every job checks out `ref:` set to the validated
   commit (`github.sha`, as passed on by `validate`), with
   `persist-credentials: false`. That includes `publish`. The token reaches
   `gh` only as `GH_TOKEN` in a step's `env:`.
7. **Byte custody.** The bytes that are published are the bytes that were
   tested:
   - G5 and G6 each download `pack-linux`'s artifact and compare it with
     `pack-linux`'s digest output, and G6 also compares it with G5's identity
     record.
   - `mospkg.json` and `SHA256SUMS` are taken from the verified archive
     bytes, never regenerated from the tree.
8. **`publish` runs no repository code except one isolated script.**
   - Its checkout is **sparse**, containing only
     `code/scripts/mosaic_package_release/`. That directory holds nothing but
     `release.py`: no `__init__.py`, and no other module a shadowing import
     could pick up.
   - It runs the script as `python3 -I`. Isolated mode leaves the script's
     own directory and the user's site-packages off `sys.path`, and ignores
     `PYTHON*` environment variables. Without it, a `zipfile.py` or
     `json.py` placed beside the script would run with the job's write and
     OIDC tokens.
   - Apart from that script, the job runs only the pinned actions and `gh`.
     No `cargo`, no `mosaic-compile`, no `npm install`.
9. **No caches.** The pack, consume-from-archive and publish jobs use no
   `actions/cache`, and no built-in caching of `setup-*` actions
   (`cache: npm` and the like). A cache is written by other runs and could
   alter the bytes being packed or checked.
10. **Release notes go through `--notes-file`.** The script writes them to a
    file. They are never passed as a command-line string built from
    changelog text.
11. **Serialized publication.** One concurrency group per package, with
    `cancel-in-progress: false`.
12. **Checksums and provenance.**
    - `SHA256SUMS` and the provenance bundle are attached.
    - Build provenance is **mandatory**, checked at G7a and G8 (§8.4).
    - The automatic release attestation is checked at G8, as an integrity
      check.
13. **`/security-review` before the push**, as CLAUDE.md requires. That
    applies to the workflow PR and to every PR that adds parsing,
    filesystem or network code (Phases 2, 3 and 6).

### 9.3 A script-injection fix in `publish.yml` (Phase 4)

`publish.yml` runs on every `release: published` event, and its `parse-tag`
job writes `${{ github.event.release.tag_name }}` straight into a shell
script.

- **Git ref names can carry shell syntax.** `git check-ref-format` accepts
  `refs/tags/a$(id)`, `refs/tags/a;b|c` and `refs/tags/a${IFS}b`. It rejects
  spaces, but `${IFS}` stands in for one. This was checked locally on
  2026-10-26. So a crafted tag runs code in `parse-tag`.
- **That code can reach the RubyGems key.** `parse-tag` can write any value
  to its outputs. `publish-ruby` interpolates
  `${{ needs.parse-tag.outputs.pkg_dir }}` into a `run:` script, and that is
  the job that writes `RUBYGEMS_API_KEY` to disk. The Python jobs interpolate
  `pkg_dir` too.
- **Who can exploit it.** Only someone who can publish a release can trigger
  it today. It is still exactly the kind of path §8.8 says must not exist.
- **Why TaskApp has never tripped it.** Releases created with `GITHUB_TOKEN`
  do not trigger other workflows. A release made by hand, or with a PAT,
  would.

Phase 4 fixes this, and names the change as a **script-injection fix** in
its commit and PR. Quoting alone is not enough, because a quoted value can
still be a well-formed but hostile path or output, so the fix has three
parts:

1. **Filter at job level.** `parse-tag` gets `if:
   startsWith(github.event.release.tag_name, 'python/') ||
   startsWith(github.event.release.tag_name, 'ruby/')`. Tags it does not
   own, `mosaic-pkg-*` included, then never start a job, instead of failing
   in shell.
2. **Validate strictly before writing anything.** The tag reaches the
   script only as `env: TAG: ${{ github.event.release.tag_name }}`. The
   script's first statement matches it against a strict pattern, using
   `re.fullmatch` (or `[[ $TAG =~ ... ]]` in bash, where `$` anchors at the
   true end of the string), and exits non-zero **before writing any
   `$GITHUB_OUTPUT` line**:

   ```
   ^(python|ruby)/[a-z0-9]+(?:[-_][a-z0-9]+)*/v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$
   ```

   The package segment allows both `-` and `_`. That matches the formats in
   `publish.yml`'s header (`python/coding-adventures-arithmetic/v0.1.0`,
   `ruby/coding_adventures_clock/v0.1.0`), and on 2026-10-26 the
   repository's own names used both separators: Python directories such as
   `image_codec_png`, Ruby directories such as `aes-modes`, and gemspec names
   such as `coding-adventures-huffman-tree`. No `python/` or `ruby/` release
   tag had ever been published, so no existing tag constrains the pattern.
   It excludes `/`, `.` and `..` from the package segment, so the derived
   `pkg_dir` cannot escape `code/packages/<language>/`.
3. **Pass every downstream value through `env:`.** That includes
   `PKG_DIR: ${{ needs.parse-tag.outputs.pkg_dir }}`. None is interpolated
   into `run:`.

---

## 10. Phases

Each phase is one PR unless noted. Phase 1 is deliberately the smallest
change that makes version strings mean something.

### Phase 0 — this spec

Decide §5 and answer §11. **Out of scope:** all code.

### Phase 1 — version strings get semantics (first implementation PR, small)

- `mosaic-package-manifest`:
  - Parse a requirement as bare (caret) or `=` (exact), and reject every
    other operator.
  - Add a `VersionReq` type with `matches(&Version)`, following Cargo's
    caret and pre-release rules. Write it by hand, without the `semver`
    crate, in line with the parser's existing zero-dependency stance.
- `mosaic-package-resolver`: both `build()` and `LayoutPackageResolver`
  check the located package's version, and fail with `VersionMismatch`.
- Bump the five stale requirement strings listed in §2.2.
- Tests: the caret table in §7.4, pre-releases, mismatch errors from both
  locators, and every in-repo package and program still resolving.

**Out of scope:** archives, the lockfile, the cache, the network, workflows,
`[sources]` and path dependencies, workspace mode, unifying the two
locators, and gating `pkg::` references.

### Phase 1b — locator correctness (small)

- A `pkg::P::C` reference must name the package itself or a declared
  dependency (`UndeclaredPackageReference`).
- Two directories that declare the same manifest name fail with
  `AmbiguousPackage`.

**Out of scope:** the same as Phase 1.

**Implementation status (2026-10-26).** The first rule is built; the
`AmbiguousPackage` rule is not yet.

- **The error.** `mosaic-package-resolver` has
  `LayoutResolveError::UndeclaredPackageReference { consumer, package,
  component, call_site, manifest_path, found_version }`. It names the
  manifest to fix and, when the package is on the search path, the
  requirement to add:

  ```text
  `JournalApp` mounts `pkg::mosaic-pkg-toolkit::Button` at
  Column[root] > pkg::mosaic-pkg-toolkit::Button, but `mosaic-pkg-toolkit` is
  not a declared dependency; add `mosaic-pkg-toolkit = "0.16.0"` to
  [dependencies] in code/programs/mosaic/journal-app/mosaic-package.toml
  ```

- **Where the declared set comes from.**
  - *A package component's own layout*, reached while inlining: that
    package's manifest, which the resolver has already read to find the
    component. This is always checked.
  - *The layout being resolved*: the caller supplies the consumer's
    manifest with `LayoutPackageResolver::with_consumer(manifest, path)`.
    The artifact builder does this for every package build (and its
    degradation analysis), and so does `mosaic-compile` when it is given a
    package manifest. A layout resolved with no consumer manifest (a
    standalone `mosaic-compile` of a loose `.mil`/`.mll` triple, or a test)
    has no `[dependencies]` to check against. Only its nested package
    references are checked.
- **Self-references are allowed.** `pkg::<self>::C` names the package's
  own export. Authors do not write it (UI34 §2.2), but the artifact
  builder rewrites a package's bare sibling references to that form before
  resolving. Bare sibling references inside a package component are local,
  so they are never checked either.
- **When it runs.** In the same pre-substitution walk as
  `SlotBindingMissing` (UI34 §7), so one build reports every undeclared
  package with every unbound slot, through the same `Multiple` error and
  the same cap of 100.
- **Divergences from the one-line rule above:**
  - An undeclared package is reported **once per referencing manifest**, at
    its first call site, not once per call site. The fix is one line in one
    file, however many call sites there are.
  - The call site is a path through the layout (`Tag[part] > Tag#n`), not
    a source span, because the layout IR carries no spans. This matches
    `SlotBindingMissing`.
  - An undeclared reference to a package that is also missing from the
    search path reports both `UndeclaredPackageReference` and
    `PackageNotFound`.
- **Repository guard.** The resolver's
  `every_repository_layout_binds_every_required_slot` test resolves every
  exported layout in the repository with its own manifest as the consumer.
  When the check landed, every `pkg::` reference in the repository named
  the package itself or a declared dependency, as §2.2 predicted.

### Phase 2 — the `.mospkg` format

- `mosaic-compile pkg pack <dir> --commit <sha> --out <file>`, which reads
  git blob bytes (§6.4) and applies the git-mode, section and
  `token_palette` rules (§6.3), and `pkg verify <file>`.
- A new library crate, `mosaic-package-archive`, wrapping the in-house `zip`
  and `sha256` crates.
- The packer is a standalone Cargo project with a **tracked** `Cargo.lock`,
  built with `--locked` (§8.8).
- `pkg verify --identity-out <file>` writes the identity record G6 checks
  against (§9.1).
- Tests:
  - determinism: packing twice gives the same digest;
  - at least one rejection test for each clause of §6.5 — ZIP64, the
    encryption flag, a data descriptor, an EOCD comment, a prefix, a gap,
    overlapping entries, deflate, oversize, a non-UTF-8 name, traversal, a
    trailing dot, `nul` and `Com1.mil`, case-collision duplicates, symlink
    attributes, a file-list mismatch, a `token_palette` outside the file
    list, and an unknown manifest section;
  - pack refusing a symlink (`120000`) and a gitlink (`160000`) tree entry.
- Needs `/security-review`.

**Out of scope:** the cache, the lockfile, resolver changes, workflows,
publishing.

### Phase 3 — sources, lockfile, cache and the single locator

- **Manifest:**
  - `[sources.*]` tables carrying `repository-id` and `owner-id`, with the
    §8.1 owner/repo grammar;
  - table-form dependencies carrying `source` or `path`;
  - `NoSourceForDependency`.
- **Workspace:**
  - `mosaic-workspace.toml`, and this repository's own workspace file;
  - discovery that cannot be spoofed (§8.3 rules 1–5): the walk ignores git
    environment variables, stops at directories the user does not own,
    never treats home as a root, and confines marker entries;
  - path and workspace packages read with symlinks and reparse points
    refused.
- **Lockfile and commands:**
  - `mosaic-package.lock` with the §8.1 binding checks;
  - `pkg add --from-file`, which runs the online provenance and
    release-binding checks, or `--insecure-skip-attestation`;
  - `pkg fetch --from-file --bundle` and `pkg fetch --offline`, each running
    the offline `--bundle` / `--custom-trusted-root` check;
  - the `--locked`, `--offline` and `--package-override` flags.
- **Pin the certificate field names.** Pin `sourceRepositoryIdentifier` and
  `sourceRepositoryOwnerIdentifier` against `gh attestation verify --format
  json` output for a real attestation. If Phase 5 has not happened yet, use a
  test attestation made on a scratch branch.
- **Cache:**
  - `$MOSAIC_PACKAGE_CACHE` with the §8.2 rules and bundles beside
    archives, and **no** trusted root in the cache;
  - verification that uses only the committed `mosaic-trusted-root.jsonl`
    found beside the lockfile (§8.4);
  - archives read and parsed from one in-memory buffer;
  - the Windows reparse-point and ACL rules as specified in §8.2.
- **Locator:** the single `PackageGraph` with verified file lists (§8.3),
  adopted by `build()`, `LayoutPackageResolver` and the artifact builder.
  With no lockfile, in workspace mode, behaviour is unchanged.
- **Tests:**
  - a consumer fixture copied into a temporary directory **outside the
    repository** builds every backend from a locally packed toolkit archive
    with `--offline`;
  - `--locked` refuses that fixture's `attested = false` entry, and refuses
    an archive whose bundle is missing;
  - a stray `packages/` directory above the fixture, and one below the
    working directory, are both ignored;
  - discovery ignores a `.git` planted in a directory the user does not own,
    and a `GIT_DIR` or `GIT_WORK_TREE` pointing elsewhere;
  - a symlinked, junctioned, world-writable or foreign-owned cache is
    refused;
  - **a swapped trusted root fails.** A cache that also carries a
    `trusted_root.jsonl` naming a test Fulcio CA and Rekor key, together
    with an archive and bundle signed under that test CA, fails `pkg fetch
    --offline`, because the committed root is the only one consulted. A
    missing committed root fails with `MissingTrustedRoot`;
  - each §8.1 mismatch produces its own error, including a changed
    `repository-id`.
- Needs `/security-review`.

**Out of scope:** network download, the workflow, publishing, indexes.

### Phase 4 — the release lane

- `release-mosaic-package.yml`, the dedicated
  `code/scripts/mosaic_package_release/release.py` and its tests, matching
  §9. That includes the build-provenance attestation with its bundle
  attached, atomic tag creation, the burned-version check (§7.2 rule 8), the
  sparse checkout and `python3 -I` in `publish`, and G6's structural
  assertions. On PRs the workflow runs everything except `publish`.
- The `publish.yml` script-injection fix (§9.3): a job-level filter, a strict
  tag pattern checked before any output is written, and values passed only
  through `env:`.
- The toolkit CHANGELOG restructured to carry a release heading, and
  converted to `CHANGELOG.d/`.
- **Maintainer actions, which a PR cannot do:**
  - create the `mosaic-package-release` environment;
  - add `CODEOWNERS` and required review for the lane's files and the pack
    toolchain (§8.8);
  - add license files (§11 Q1).

**Out of scope:** publishing anything, any package other than the toolkit,
network fetch.

### Phase 5 — first publication: `mosaic-pkg-toolkit-v0.16.0`

- The maintainer dispatches the lane and approves the environment.
- An independent audit repeats G8 from a clean machine:
  - `gh attestation verify` with the §8.4 flags;
  - `gh release verify`;
  - every asset digest matches `SHA256SUMS`;
  - the tag commit, the validated commit and `mospkg.json.source.commit` are
    the same.
- Prove resolution of the **published** package:
  1. download the asset with `gh release download`;
  2. run `pkg add --from-file`, which performs the real provenance check and
     `gh release verify-asset`, stores the bundle, and writes
     `attested = true`;
  3. run `pkg fetch --offline`, which uses the committed
     `mosaic-trusted-root.jsonl` beside the fixture's lockfile;
  4. build the Phase 3 fixture with `--locked --offline`.

  This meets #14014's last acceptance criterion without adding network code
  to the resolver.

**Out of scope:** other packages, an in-compiler downloader.

### Phase 6 — `pkg add` and `pkg fetch` from the network

- **Download** the archive and its `.sigstore.jsonl` bundle by the URLs
  built from the manifest-declared source (§8.1).
- **Size cap while streaming:** the 16 MiB limit is enforced while the bytes
  arrive, so an oversized response is abandoned before it is fully read,
  hashed or parsed.
- **Redirects:** only HTTPS targets are followed. GitHub serves release
  assets through a redirect. Whether to restrict redirect hosts to a list is
  part of §11 Q11.
- **Verification** against the lockfile (`fetch`), or through the provenance
  check (`add`).
- **A manual or scheduled "consume published" job** that builds the fixture
  from the real release. It does not run on PRs, so network trouble cannot
  turn PRs red.
- Needs `/security-review`.

**Out of scope:** automatic version selection, indexes, mirrors.

### Phase 7 — the rest

- Publish the remaining packages leaf first: `grid`, `rating-controls` and
  `std-foundation`, then their dependants.
- The interface-diff check (§7.3).
- The index, with its yank and advisory list, consulted by `pkg add` and
  `pkg fetch` (§8.7).
- If wanted: npm or GHCR mirrors of the same bytes, `pkg vendor`, and
  pre-compiled per-backend bundles (§11 Q9).

Each of these gets its own item in `MOSAIC-BACKLOG.md`.

---

## 11. Open questions for the maintainer

1. **License.** The manifests say `MIT OR Apache-2.0`, but the repository has
   no license text, and its README says everything is copyrighted, with
   packages "licensed separately". Is `MIT OR Apache-2.0` the grant you
   intend for published Mosaic packages? If so, `LICENSE-MIT` and
   `LICENSE-APACHE` must be added **before** the first public release,
   either to each package or once and copied into each archive. This blocks
   Phase 5.
2. **Is there a concrete external consumer?** No app outside this repository
   uses Mosaic packages today. If the near-term consumers are only the
   per-component test apps and the docs site, both inside this repository,
   is the goal "a released, immutable version to link to" (Phases 2–5)
   rather than "a resolver that downloads" (Phase 6)? That would let Phase 6
   wait.
3. **Per-package or per-component versions?** This spec proposes
   per-package versions, which diverges from the component program's
   "per-component SemVer" (§7.1). Accept the divergence, and update the
   component program to match?
4. **The provenance verifier.** Build provenance is now mandatory (§8.4).
   - Is depending on the `gh` CLI for `pkg add` and `pkg fetch` acceptable
     in v1, or must a Rust Sigstore verifier exist first? (`compile` never
     needs `gh`.)
   - Should `--insecure-skip-attestation` exist at all, given that
     `--locked` refuses its entries?
   - The trusted root: the proposal is a committed
     `mosaic-trusted-root.jsonl` per consumer, the only root ever used,
     refreshed by a reviewed PR, with a scheduled drift-check job (§8.4).
     Weekly enough?
   - Is it acceptable that a pre-filled or restored cache, used by `compile`
     without a `pkg fetch --offline` first, is trusted only as far as the
     reviewed lockfile (§8.4)?
5. **Kernel compatibility.** `[kernel] version = "1"` has gained primitives
   without a bump, so a package cannot say which compiler it needs. Options:
   - bump the kernel version on every new primitive;
   - add a `[kernel] min-revision`;
   - record the verifying `mosaic-compile` commit in `mospkg.json`, and
     treat adopting a new primitive as breaking (§7.3).
6. **Archive container.** Is a stored-only ZIP over the in-house `zip` crate
   (§6.1) acceptable? The alternative, tar plus gzip, needs a new in-house
   tar implementation or a third-party crate.
7. **Host code in packages.** v1 refuses `[host_effects]` and `[host_assets]`
   (§6.3). Is that the right line, given that no library package uses them
   today?
8. **Distributing `mosaic-compile`.** A consumer outside this repository
   cannot build anything without the compiler, and it is not released. The
   choices:
   - a `mosaic-compile-v*` release lane with binaries on GitHub Releases,
     carrying the same provenance as §8.4;
   - crates.io publishing of the compiler crates (option (a) is reasonable
     for *Rust* code);
   - "build from source at a pinned commit", for now.

   This may matter more to an external consumer than package publishing
   does.
9. **Pre-compiled bundles.** UI29 §4.3 promised per-backend compiled
   artifacts. Should releases also attach those, for hosts that will not run
   `mosaic-compile`? If so, from which phase?
10. **The trust root (§8.8).** Today `main` accepts merges with zero
    approvals and no code-owner review, and the workspace `Cargo.lock` is
    untracked.
    - Add `CODEOWNERS` for everything that decides the signed bytes, and
      require code-owner review, or require one approving review on those
      paths? The paths are the lane's workflow, the dedicated script
      directory and its tests, the pack toolchain (`mosaic-compile pkg
      pack`, `mosaic-package-archive`, `mosaic-package-manifest`, `zip`,
      `deflate`, `sha256`, `bounded-json`) and the packer's tracked
      `Cargo.lock`.
    - Accept a standalone packer project with its own tracked lockfile?
    - Cover `.github/CODEOWNERS` itself, and agree that the `main` ruleset's
      bypass list, its review settings and the environment's reviewers change
      only by a recorded maintainer action, since any bypass actor is in
      effect a signer?
    - Create the `mosaic-package-release` environment with you as the
      required reviewer, restricted to `main`?
    - Add a tag ruleset for `mosaic-pkg-*` and `mosaic-std-*` anyway? It can
      narrow who creates those tags, but its bypass list cannot single out
      one workflow, so it is defence in depth, not the control.
11. **Network transport for Phase 6.** Use the in-house HTTP/1 client and
    platform TLS crates, shell out to `gh release download` or `curl`, or
    leave downloading to the consumer's CI entirely (`--from-file` only)?
    Should redirect targets be limited to a host allowlist?
12. **First version, and 1.0.** Publish the toolkit as 0.16.0, its current
    manifest version, which consumers already require? When does it commit
    to 1.0.0 and the stricter bump rules that come with it?
13. **Mirrors.** If (d) is adopted, is an npm mirror for web hosts wanted at
    all, and under which scope?
14. **Yanks and advisories (§8.7).** Who may add an entry? Does `pkg add`
    refuse a yanked version or only warn? Should an existing lock entry for a
    yanked version keep working, as Cargo allows, with a warning, and only
    `--locked` plus an advisory of kind `security` fail?
15. **The workspace marker (§8.3).** Is `mosaic-workspace.toml` at `code/`
    the right name and place for the file that turns on ambient search paths
    in this repository?

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
