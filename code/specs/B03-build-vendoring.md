# B03 — Build Vendoring

## Overview

Vendoring is the practice of pre-downloading all external dependencies into a
local directory so that builds are reproducible and can run without network
access. Instead of fetching `pytest` from PyPI every time you build, you fetch
it once, store it locally, and every subsequent build reads from that local
copy.

The idea is older than most modern package managers. Early C projects shipped
their dependencies as source tarballs inside the repository — literally
"vendoring" the code into the project. Go formalized this with its `vendor/`
directory convention. Rust's `cargo vendor` command does the same thing. We
generalize the pattern across all seven languages in the monorepo.

Each vendored dependency is stored at a specific version alongside its content
hash. This gives us four properties that matter for a build system:

1. **Reproducibility** — the same version and the same bytes, every time.
2. **Speed** — no network I/O during builds; everything is a local file copy.
3. **Security** — vendored packages can be audited, diffed, and pinned.
4. **Hermeticity** — the sandbox needs no network access at all.

## Why Vendor?

Consider what happens without vendoring. A BUILD file declares that a package
depends on `pytest>=7.0`. At build time, the build system calls `pip install
pytest` inside a sandbox. This requires:

- Network access to PyPI (or a mirror)
- DNS resolution to find pypi.org
- TLS negotiation to establish a secure connection
- A download of the package wheel (maybe several megabytes)
- Hash verification against what pip expects

If PyPI is down, the build fails. If PyPI serves a compromised package, the
build is compromised. If the network is slow, the build is slow. If two
developers build at different times and a new patch version of pytest was
released between those builds, they get different results.

Vendoring eliminates all of these problems. The dependency is downloaded once,
verified once, and stored locally. Every subsequent build uses the exact same
bytes. The build system never touches the network.

This is especially important in CI, where builds run in ephemeral containers.
Without vendoring, every CI run downloads every dependency from scratch. With
vendoring, the vendor directory is cached (or committed to the repository for
small projects), and builds start instantly.

## Vendor Directory Layout

All vendored packages live under `.build/vendor/`, organized by package
manager and then by `name@version`:

```
.build/vendor/
├── pip/
│   ├── pytest@7.4.0/              # Python wheels
│   │   └── pytest-7.4.0-py3-none-any.whl
│   └── pytest-cov@4.1.0/
│       └── pytest_cov-4.1.0-py3-none-any.whl
├── npm/
│   ├── vitest@1.2.0/              # Node packages (tarballs)
│   │   └── package.tgz
│   └── typescript@5.3.0/
│       └── package.tgz
├── cargo/
│   ├── serde@1.0.195/             # Rust crates (.crate files)
│   │   └── serde-1.0.195.crate
│   └── tokio@1.35.0/
│       └── tokio-1.35.0.crate
├── mix/
│   ├── jason@1.4.1/               # Elixir hex packages
│   │   └── jason-1.4.1.tar
│   └── ex_doc@0.31.0/
│       └── ex_doc-0.31.0.tar
├── go/
│   ├── golang.org/
│   │   └── x/
│   │       └── text@v0.14.0/      # Go modules (source trees)
│   │           └── ...
│   └── github.com/
│       └── stretchr/
│           └── testify@v1.8.4/
│               └── ...
├── bundler/
│   ├── minitest@5.20.0/           # Ruby gems
│   │   └── minitest-5.20.0.gem
│   └── rake@13.1.0/
│       └── rake-13.1.0.gem
└── gradle/
    ├── junit-jupiter@5.10.0/      # JVM jars
    │   └── junit-jupiter-5.10.0.jar
    └── kotlin-stdlib@1.9.22/
        └── kotlin-stdlib-1.9.22.jar
```

The directory name encodes both the package name and the exact version. This
means you can vendor multiple versions of the same package simultaneously —
useful during migrations when different packages depend on different versions.

Note the Go layout preserves the module path hierarchy. Go modules use URLs as
identifiers (`golang.org/x/text`), so we mirror that structure to avoid
flattening collisions.

## Lockfile Integration

Every package manager has its own lockfile format. The lockfile is the source
of truth for which versions to vendor. The vendoring tool never resolves
versions itself — it reads the lockfile and downloads exactly what the lockfile
specifies.

| Manager  | Lockfile               | What it records                        |
|----------|------------------------|----------------------------------------|
| pip      | requirements.txt       | name==version with --hash flags        |
| npm      | package-lock.json      | exact version + integrity hash (SHA512)|
| cargo    | Cargo.lock             | name, version, source, checksum        |
| mix      | mix.lock               | package name, version, hex hash        |
| go       | go.sum                 | module path, version, hash (h1:...)    |
| bundler  | Gemfile.lock           | gem name, version, platform            |
| gradle   | gradle.lockfile        | group:artifact:version with checksum   |

The lockfile acts as a contract: "these are the exact bytes I expect." If the
lockfile changes, `build-tool vendor` re-downloads the affected packages. If
the lockfile has not changed, the vendor directory is already correct and no
work is needed.

This is a deliberate design choice. Version resolution is a complex,
manager-specific problem (pip's resolver is different from npm's, which is
different from Cargo's). By delegating resolution to each manager's native
lockfile, we avoid reimplementing seven different resolution algorithms.

## The vendor() Directive in BUILD Files

BUILD files declare external dependencies using the `vendor()` directive:

```starlark
python_library(
    name = "directed_graph",
    srcs = ["lib/directed_graph.py"],
    external_deps = ["networkx>=3.0", "numpy>=1.24"],
    vendor = vendor(
        lockfile = "requirements.txt",
        manager = "pip",
    ),
)
```

The `vendor()` directive tells the build system two things:

1. **Where to find the lockfile** — relative to the package directory.
2. **Which package manager** — so the build system knows how to install from
   the vendor directory.

The `external_deps` field lists the abstract dependency constraints (used for
documentation and resolution), while `vendor()` points to the concrete,
locked versions.

A package without a `vendor()` directive has no external dependencies. The
build system will not attempt any package installation for it. This is the
common case in this monorepo — most packages depend only on other packages in
the repo, not on external registries.

## The `build-tool vendor` Command

The vendoring tool is a subcommand of the build tool:

```
build-tool vendor                                # vendor everything
build-tool vendor --package elixir/directed-graph # vendor one package
build-tool vendor --offline                       # verify, no downloads
build-tool vendor --clean                         # remove unreferenced deps
build-tool vendor --audit                         # check for vulnerabilities
```

### Full Vendor (`build-tool vendor`)

Walks every BUILD file in the repo, collects all `vendor()` directives,
reads each lockfile, and downloads any packages not already present in the
vendor directory. This is idempotent — running it twice in a row does nothing
the second time, because all packages are already vendored and their hashes
match.

### Single Package Vendor (`--package`)

Vendors only the external dependencies of one package. Useful during
development when you add a new dependency to a single package and want to
vendor it without processing the entire repo.

### Offline Verification (`--offline`)

Does not download anything. Instead, it walks every BUILD file, reads every
lockfile, and checks that every required package exists in the vendor
directory with the correct hash. If anything is missing or corrupted, it
exits with a nonzero status and prints what is wrong.

This is the mode used in CI. The CI pipeline runs `build-tool vendor --offline`
as a pre-build check. If it fails, someone forgot to run `build-tool vendor`
and commit the manifest.

### Clean (`--clean`)

Removes any vendored packages that are no longer referenced by any BUILD file
in the repo. Over time, as dependencies are removed or updated, old versions
accumulate in the vendor directory. `--clean` prunes them.

### Audit (`--audit`)

Checks vendored packages against known vulnerability databases. This is a
future extension — the initial implementation will not include it, but the
command is reserved so scripts can start calling it early.

## Content-Addressable Storage

Every vendored package is verified by its SHA-256 content hash. The process
works like this:

```
function vendor_package(manager, name, version, expected_hash):
    target_dir = ".build/vendor/{manager}/{name}@{version}/"

    if target_dir exists:
        actual_hash = sha256(contents of target_dir)
        if actual_hash == expected_hash:
            return  # already vendored and verified
        else:
            warn("hash mismatch, re-downloading")
            remove(target_dir)

    temp_dir = create_temp_directory()
    download(manager, name, version, into=temp_dir)

    actual_hash = sha256(contents of temp_dir)
    if expected_hash is not None and actual_hash != expected_hash:
        error("downloaded package does not match expected hash")
        abort()

    move(temp_dir, target_dir)
    update_manifest(manager, name, version, actual_hash)
```

The hash is computed over the raw package bytes — the wheel file for pip, the
tarball for npm, the .crate file for cargo, and so on. This means the hash is
stable across operating systems and filesystems (no issues with line endings,
permissions, or timestamps).

Content-addressable storage is a well-known pattern. Git uses it for objects.
Docker uses it for layers. Nix uses it for the entire package store. The
principle is the same: if two things have the same hash, they are the same
thing.

## Vendor Manifest

The manifest is a JSON file at `.build/vendor/manifest.json` that records
every vendored package:

```json
{
  "version": 1,
  "packages": {
    "pip/pytest@7.4.0": {
      "hash": "sha256:a1b2c3d4e5f6...",
      "size": 1534987,
      "vendored_at": "2026-04-05T12:00:00Z",
      "source": "https://pypi.org/simple/pytest/"
    },
    "npm/vitest@1.2.0": {
      "hash": "sha256:f6e5d4c3b2a1...",
      "size": 892341,
      "vendored_at": "2026-04-05T12:01:00Z",
      "source": "https://registry.npmjs.org/vitest/"
    },
    "cargo/serde@1.0.195": {
      "hash": "sha256:1a2b3c4d5e6f...",
      "size": 234567,
      "vendored_at": "2026-04-05T12:02:00Z",
      "source": "https://crates.io/api/v1/crates/serde/"
    }
  }
}
```

The manifest serves three purposes:

1. **Fast verification** — instead of re-hashing every vendored package on
   every build, the build system reads the manifest and trusts its hashes.
   Full re-verification happens only during `--offline` checks.
2. **Provenance tracking** — the `source` field records where each package
   was downloaded from, creating an audit trail.
3. **Diffability** — the manifest is a single file that can be committed to
   git. A diff of the manifest shows exactly which dependencies changed
   between two commits.

The `version` field allows future schema evolution. If we add fields later,
we bump the version and the tool knows how to migrate.

## Integration with the Sandbox

During sandbox creation, the build system uses vendored packages instead of
downloading from the network. The flow is:

```
1. Parse BUILD file for the target package
2. Read external_deps and vendor() directive
3. For each external dependency:
   a. Look up the resolved version in the lockfile
   b. Find the vendored package in .build/vendor/<manager>/<name>@<version>/
   c. Verify the hash against the manifest
   d. Copy (or hard-link) into the sandbox workspace
4. Configure the package manager to use the local copies
5. Run the build with no network access
```

Step 4 is manager-specific. Each package manager has its own mechanism for
pointing at a local directory instead of a remote registry.

## Manager-Specific Strategies

### pip (Python)

Download wheels using `pip download --no-deps -d <dir>`. At install time,
point pip at the vendor directory:

```
pip install --no-index --find-links .build/vendor/pip/ pytest==7.4.0
```

The `--no-index` flag disables PyPI entirely. `--find-links` tells pip to
look for wheels in the given directory. This is pip's native mechanism for
offline installs.

### npm (Node.js)

Pack each package using `npm pack` and store the tarball. At install time,
configure npm to use the local cache:

```
npm install --prefer-offline --cache .build/vendor/npm/
```

Alternatively, rewrite `package-lock.json` resolved URLs to point at local
file paths. This is more fragile but avoids npm's cache semantics entirely.

### cargo (Rust)

Use `cargo vendor` to download all crates. Configure `.cargo/config.toml` to
redirect the registry:

```toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = ".build/vendor/cargo"
```

Cargo has first-class vendoring support. This is the simplest integration of
all seven managers.

### mix (Elixir)

Download hex packages and store them locally. Set the `HEX_HOME` environment
variable to point at the vendor directory:

```
HEX_HOME=.build/vendor/mix mix deps.get
```

Mix will read cached packages from `HEX_HOME` instead of downloading from
hex.pm.

### go (Go)

Go has built-in vendoring via `go mod vendor`, which creates a `vendor/`
directory in the module root. Set the `-mod=vendor` flag:

```
GOFLAGS=-mod=vendor go build ./...
```

Go's vendor support is mature and well-integrated. The vendor directory
contains source code (not compiled artifacts), so it works across platforms.

### bundler (Ruby)

Use `bundle install --deployment --path vendor/bundle` to install gems into
a local directory. Bundler's deployment mode ensures it uses the Gemfile.lock
exactly:

```
BUNDLE_PATH=.build/vendor/bundler bundle install --deployment
```

### gradle (JVM — Java, Kotlin, Scala)

Redirect the dependency cache to the vendor directory:

```
gradle build --project-cache-dir .build/vendor/gradle
```

Alternatively, configure a flat-directory repository in `build.gradle.kts`:

```kotlin
repositories {
    flatDir {
        dirs(".build/vendor/gradle")
    }
}
```

Gradle's support is the least standardized of the seven. The flat-directory
approach works but loses transitive resolution metadata. For this monorepo,
where BUILD files already declare all transitive dependencies, this is
acceptable.

## Security Considerations

Vendoring improves security by pinning exact versions and verifying content
hashes, but it also introduces new responsibilities:

### Hash Verification

Every time a vendored package is used, its SHA-256 hash is checked against
the manifest. If the hash does not match, the build fails immediately. This
catches:

- Accidental corruption (disk errors, incomplete copies)
- Tampering (someone modified a vendored package)
- Stale vendor state (lockfile updated but vendor not re-run)

### Committing Vendored Packages

For small repositories, the vendor directory can be committed to git. This
means every dependency is code-reviewed as part of the normal pull request
process. For larger repositories (like this one), the vendor directory is
listed in `.gitignore` and rebuilt from the manifest in CI.

The manifest itself should always be committed. It is small, diffable, and
provides a clear record of what changed.

### Supply Chain Auditing

The `build-tool vendor --audit` command (future work) will cross-reference
vendored packages against vulnerability databases:

- PyPI: the Python Advisory Database
- npm: the GitHub Advisory Database
- cargo: RustSec Advisory Database
- go: the Go Vulnerability Database

This does not replace proper dependency management, but it provides an
automated safety net.

## Relationship to Other Specs

- **12 — Build System**: the parent spec. Vendoring is an extension of the
  build system, not a replacement. The build system discovers packages, resolves
  the dependency graph, and orchestrates builds. Vendoring handles only the
  "fetch external dependencies" step.
- **B01 — Sandbox**: the sandbox consumes vendored packages. Instead of
  granting network access to the sandbox, we copy vendored packages in.
- **14 — Starlark**: the `vendor()` directive is part of the BUILD file
  syntax, which is defined by the Starlark spec.

## Future Work

1. **Vulnerability auditing** (`--audit` flag) against advisory databases.
2. **License scanning** to ensure vendored packages comply with project policy.
3. **Vendor mirroring** — a local registry mirror that serves vendored packages
   over HTTP, for tools that insist on network access.
4. **Partial vendoring** — vendor only production dependencies, not dev/test
   dependencies, to reduce vendor directory size.
5. **Content-addressable deduplication** — if two packages contain identical
   files, store the file once and hard-link it. This is how Nix and pnpm work.
