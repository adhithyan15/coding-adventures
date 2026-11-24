# VER31a — Registry Publishing Behind the VER31 Gate

**Status:** Draft for owner review (LGE-S5, 2026-11-24). Docs only: no workflow,
script or registry setting changes with this document. It is an **amendment to
[VER31](VER31-tiered-release-standard.md)**, not a replacement: every VER31 rule
(floor checks F1-F12, the gate of section 7, signed tags and the `release`
environment of section 9) still applies and is only *extended* here to every
registry.

**Parent:** [VER31](VER31-tiered-release-standard.md). **Builds on:**
[LG02](LG02-basic-gates-cross-lane-contract.md) (what a lane may claim),
VER15a (the lane weakness disclosure block; at the time of writing it is
PR #19954, not yet on `main`), and the program shard
[logic-gates-everywhere](VER-BACKLOG.d/logic-gates-everywhere.md).

**Why now.** Owner direction (2026-11-24): publish the basic gates in every
lane while stating truthfully what is verified; upgrade later
(`VER-BACKLOG.d/logic-gates-everywhere.md:9-13, 45-46`). Today `publish.yml`
can publish two registries, PyPI and RubyGems (`.github/workflows/publish.yml:5-8,
28-35, 59-61`), and the VER31 gate is not wired
(`logic-gates-everywhere.md:72-75`).

**Scope rule.** The design is for *any* package. Logic gates is the first
customer, so the tables use its manifests as the worked example. Paths in
citations are relative to `code/packages/` unless they start with `.github/`,
`code/` or name a spec.

**Verification convention.** Anything this document says about how an external
registry behaves, and that the repository cannot confirm, is marked **verify
before implementing**. The implementer checks the registry's current
documentation first; a wrong guess here must not become a wrong workflow.

---

## 0. Findings that change the plan (read first)

Reading the real manifests turned up blockers that no workflow can fix. Each is
a row in the backlog (section 8) and must be settled before the named lane
publishes.

| # | Finding | Evidence | Consequence |
|---|---|---|---|
| B1 | **Most lanes depend on `transistors` at runtime.** A registry package that names an unpublished sibling cannot be installed. | Rust `rust/logic-gates/Cargo.toml:8` (path dep, no version); TypeScript `typescript/logic-gates/package.json:21` (`file:../transistors`); Go `go/logic-gates/go.mod:5,7` (`v0.0.0` plus a `replace`); Ruby `ruby/logic_gates/coding_adventures_logic_gates.gemspec:24`; Elixir `elixir/logic_gates/mix.exs:25` (path dep); Perl `perl/logic-gates/cpanfile:4`; Lua `lua/logic_gates/coding-adventures-logic-gates-0.4.0-1.rockspec:12`; Swift `swift/logic_gates/Package.swift:29` (path package). Python, Java, Kotlin, C#, F#, Haskell, OCaml and Dart manifests name no such dependency (grep of their manifests). | The dependency closure must be published **first, in leaf-to-root order** (section 5.1), or the dependency removed. Registries reject or rewrite path and `file:` dependencies (verify before implementing, per registry). Go: a consumer ignores a dependency's `replace` directive and `v0.0.0` is not a real tag, so the module is not importable until `transistors` has a real tag (verify before implementing). F8 (zero third-party runtime dependencies, VER31 `:359`) says "third-party"; whether a first-party published dependency counts is open item O1. |
| B2 | **Java and Kotlin would claim the same Maven coordinates.** | `java/logic-gates/build.gradle.kts:9-10` and `kotlin/logic-gates/build.gradle.kts:9-10` both set group `com.codingadventures`, version `0.2.0`; both `settings.gradle.kts:1` set the name `logic-gates`. | Two lanes, one `com.codingadventures:logic-gates:0.2.0`. Give them distinct artifact ids (for example `logic-gates-java`, `logic-gates-kotlin`) before any publish. |
| B3 | **Maven group `com.codingadventures` needs a domain the owner controls.** | Same files. Central's namespace verification for a reverse-DNS group proves control of the domain (verify before implementing). | If the owner does not own that domain, the group becomes `io.github.adhithyan15`, verified through the GitHub account (verify before implementing). Owner decision O2. |
| B4 | **Rust crate name `logic-gates` and Hackage name `logic-gates` are generic**, and the Rust manifest has no `license`, `repository` or `readme` field. | `rust/logic-gates/Cargo.toml:1-5`; `haskell/logic-gates/logic-gates.cabal:2`. | crates.io and Hackage names are global first-come; the name may be taken (verify before implementing). F9 (licence MIT, VER31 `:360`) needs `license = "MIT"` in the manifest. Prefer a prefixed name (`coding-adventures-logic-gates`), decided before the first publish because a published name cannot be renamed. Owner decision O6. |
| B5 | **Dart is marked unpublishable.** | `dart/logic-gates/pubspec.yaml:5` (`publish_to: none`). | The line is removed deliberately in the release PR, and a `LICENSE` added, before pub.dev accepts the package. |
| B6 | **SwiftPM needs `Package.swift` at the repository root** (verify before implementing); the package lives in a subdirectory. | `swift/logic_gates/Package.swift` (exists only under `code/packages/swift/logic_gates/`). | A tag in this monorepo may not make the package consumable. Options: a mirror repository populated from the tagged commit, or a root-level shim. Owner decision O3. Until then Swift is **not publishable**; "tag only" does not work for this layout. |
| B7 | **Go needs a subdirectory-prefixed tag.** | Module path `github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates` (`go/logic-gates/go.mod:1`). | The tag the Go proxy honours is `code/packages/go/logic-gates/vX.Y.Z` (the module's directory relative to the repository root, then `/v<semver>`; verify before implementing), not the `go/logic-gates/vX.Y.Z` shape the workflow uses today (`.github/workflows/publish.yml:8, 104`). Section 3 routes it. |
| B8 | **No `cpp/logic-gates` is on `main`.** | `c/logic-gates` exists (`c/logic-gates/include/logic_gates.h`); no `cpp/logic-gates` directory; the C++ package is PR #19928 (`logic-gates-everywhere.md:118`). | The C++ row is specified but cannot run until that PR merges. |
| B9 | **No release gate, signers file or CODEOWNERS exists yet.** | `code/scripts/` has `engram_release.py`, `forme_release_gate.py` and other product scripts but no `release_gate.py`; `.github/release-signers` and `.github/CODEOWNERS` are absent. VER31 specifies them (`:404-417, 500-521, 639-643`). | Every publish job below depends on LV-007a (the gate). Nothing here publishes before that lands. |

---

## 1. Registries, names and owner actions

### 1.1 Per-lane table

"Name to claim" is the name the manifest declares today. Rows marked
**rename** need B1-B6 settled first. "Mechanism" is the steady state. Section
1.2 lists the exact owner actions. The coordinator (an agent) can do **none** of
them: each needs an account, a secret or a personal identity, and agents must
not hold the owner's identity (VER31 `:566-590`).

| Lane | Registry | Name to claim (manifest) | Steady-state mechanism | Trusted publishing / OIDC |
|---|---|---|---|---|
| rust | crates.io | `logic-gates` (`Cargo.toml:2`); **rename** (B4) | `cargo package --no-verify` in the gate, publish via `rust-lang/crates-io-auth-action` (VER31 `:433-440, 596`) | Yes (VER31 `:596`). No pending publisher: first version by hand |
| typescript | npm | `@coding-adventures/logic-gates` (`package.json:2`) | Publish the gate's tarball with `npm publish --provenance --access public` (VER31 `:433, 597`) | Yes (OIDC); first version by hand (VER31 `:597`). Needs the `@coding-adventures` scope |
| python | PyPI | `coding-adventures-logic-gates` (`pyproject.toml:6`) | Existing job (`publish.yml:171-200`) with PyPI attestations (VER31 `:598`) | Yes, existing; pending publishers allow OIDC from the first version (VER31 `:598`) |
| ruby | RubyGems | `coding_adventures_logic_gates` (`gemspec:6`) | Existing job with API key (`publish.yml:287-321`); move to trusted publishing (VER31 `:599`) | Yes, recommended (verify the action and settings before implementing) |
| go | Go module proxy | `github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates` (`go.mod:1`) | **Tag only**; `proxy.golang.org` fetches on first request (verify before implementing) | n/a (no upload, no token) |
| java | Maven Central | `com.codingadventures:logic-gates` (**rename** the artifact id, B2/B3) | Central Publisher Portal upload of a signed bundle (verify before implementing the plugin) | Portal user token (secret) and GPG signing key. No OIDC (verify before implementing) |
| kotlin | Maven Central | `com.codingadventures:logic-gates` (**rename**, B2/B3) | as Java | as Java |
| csharp | NuGet | `CodingAdventures.LogicGates.CSharp` (`csproj:7`) | `dotnet nuget push` of the gate's `.nupkg` | Trusted publishing via GitHub OIDC (verify before implementing); else API key |
| fsharp | NuGet | `CodingAdventures.LogicGates.FSharp` (`fsproj:5`) | as C# | as C# |
| elixir | Hex | `coding_adventures_logic_gates` (`mix.exs:6`) | `mix hex.publish` with an API key | API key (verify before implementing whether Hex offers OIDC) |
| lua | LuaRocks | `coding-adventures-logic-gates` (`rockspec:1`) | `luarocks upload` with an API key | API key (verify before implementing) |
| perl | CPAN | distribution `CodingAdventures-LogicGates` (`Makefile.PL:6`) | Upload a `make dist` tarball to PAUSE | PAUSE credentials; no OIDC (verify before implementing) |
| haskell | Hackage | `logic-gates` (`cabal:2`); **rename** (B4) | `cabal sdist` then `cabal upload` | Hackage token (verify before implementing) |
| dart | pub.dev | `coding_adventures_logic_gates` (`pubspec.yaml:1`); needs B5 | `dart pub publish` from GitHub Actions | OIDC automated publishing, configured per package with a tag pattern (verify before implementing; the first version may need a manual upload) |
| ocaml | opam-repository | `coding-adventures-logic-gates` (`opam:2`) | **Pull request** to `ocaml/opam-repository`, not an upload; `opam publish` opens it (verify before implementing) | None. A PR reviewed by repository maintainers |
| swift | SwiftPM (git) | `logic-gates` (`Package.swift:22`) | **Blocked** by B6 | none |
| c | GitHub release | `c/logic-gates` | Source tarball asset plus checksum plus signature on the GitHub Release; vcpkg and Conan later | GitHub artifact attestation |
| cpp | GitHub release | `cpp/logic-gates` (not on `main`, B8) | as C | as C |

(`gemspec`, `csproj`, `fsproj`, `mix.exs`, `rockspec`, `Makefile.PL`, `cabal`,
`opam` and `pubspec.yaml` are the manifests in each lane's package directory:
`ruby/logic_gates/coding_adventures_logic_gates.gemspec`,
`csharp/logic-gates/CodingAdventures.LogicGates.csproj`,
`fsharp/logic-gates/CodingAdventures.LogicGates.fsproj`,
`elixir/logic_gates/mix.exs`,
`lua/logic_gates/coding-adventures-logic-gates-0.4.0-1.rockspec`,
`perl/logic-gates/Makefile.PL`, `haskell/logic-gates/logic-gates.cabal`,
`ocaml/logic-gates/coding-adventures-logic-gates.opam`,
`dart/logic-gates/pubspec.yaml`.)

### 1.2 Owner actions (accounts, claims, secrets)

None of these can be done by the coordinator or a sub-agent. The consolidated
list is the backlog row LGE-OWNER-ACCOUNTS (section 8).

| Registry | Exactly what the owner does |
|---|---|
| crates.io | Create or confirm the account; choose and **claim the crate name** by publishing the first version by hand from the tagged commit (VER31 `:596`); then register the trusted publisher pinning repository, workflow `publish.yml` **and** environment `release` (VER31 `:554-557`). No secret enters the repository. |
| npm | Create the `coding-adventures` organisation (scope); enable security-key 2FA; publish the first version by hand (VER31 `:597`); register the trusted publisher with the same pinning. |
| PyPI | Register a pending or normal trusted publisher per package pinning environment `release`; existing registrations that do not pin it are re-registered (VER31 `:598`). |
| RubyGems | Either keep `RUBYGEMS_API_KEY` (today `publish.yml:34-35, 318`) and move it into the environment (`publish.yml:291-293`), or configure a trusted publisher on rubygems.org for each gem (verify before implementing). |
| Go proxy | Nothing to create. The owner signs the tag (section 3). The repository must be publicly fetchable (verify before implementing). |
| Maven Central | Create a Central Publisher Portal account; **verify the namespace** (domain TXT record for `com.codingadventures`, or switch to `io.github.adhithyan15`; decision O2); generate a GPG key, publish its public key to a keyserver, and store the private key, its passphrase and the portal user token as **environment secrets** of `release`. |
| NuGet | Create the nuget.org account; reserve the `CodingAdventures.` ID prefix (verify before implementing); configure a trusted publisher for repository, workflow and environment, or create a scoped API key stored as an environment secret. |
| Hex | Create the hex.pm account; generate an API key; store it as an environment secret. |
| LuaRocks | Create the luarocks.org account and API key; store it as an environment secret. |
| CPAN | Request a PAUSE account (a human approval step at PAUSE); store credentials as environment secrets. A new distribution name is claimed by its first upload (verify before implementing). |
| Hackage | Create the Hackage account and, if needed, request upload rights (verify before implementing); create an API token; store it as an environment secret. |
| pub.dev | Create or use a Google account; upload the first version by hand if required; enable automated publishing for the package from the GitHub repository with a tag pattern (verify before implementing). |
| opam | Use a GitHub account that can open a PR to `ocaml/opam-repository`; respond to maintainer review. Not automatable end to end. |
| Swift | Decide B6 (mirror repository or shim; O3). If a mirror, create it and grant the workflow a fine-grained push credential limited to it. |
| C / C++ | Choose and set up release signing (O4); nothing registry-side. |
| GitHub (all) | Create the `release` environment with the owner as required reviewer, administrator bypass off, deployment restricted to the tag patterns of section 3.2 (VER31 `:539-545`); create the tag-protection ruleset (VER31 `:651-655`); create the machine account and keep the owner token off agent hosts (VER31 `:566-590`); add `.github/release-signers`, its signature and CODEOWNERS (VER31 `:500-521, 639-643`). |

---

## 2. The VER31 gate in front of every publish job

VER31 section 7 already says the gate runs "before every publish job of every
language" (VER31 `:404-405`). This section fixes what that means for the new
registries. No publish job may exist that does not `needs:` both `gate` and
`verify-tag` (VER31 `:419-432`).

### 2.1 Inputs

The gate's inputs are unchanged (VER31 `:407-410`): the tagged commit, the
package directory resolved from the tag, the package's report or, for a lane
package, the one report whose `lanes` array names the package (VER31 `:391-398`),
the bucket and catalogue files, and `.github/release-signers` read from `main`.
Added by this document:

1. **The lane's manifest** (the file in section 1.1), from which the gate reads
   the registry name and version, as it already does for `Cargo.toml` and
   `package.json` (VER31 `:457-460`). The tag version must equal it (F11,
   VER31 `:362`).
2. **The lane's evidence state**: the gate computes whether the lane may claim
   the label below from files, not from the release text.

### 2.2 The tier a lane package may claim

| Label | When allowed | Source |
|---|---|---|
| `tested`, wording "lane-local exhaustive over U8 (0..255)" | Every row of the lane's type-confusion matrix passes, no stable-identity follow-up is pending, and the lane suites contain literal truth tables, the full non-bit enumeration and the matrix | LG02 `:455-464, 631-636` |
| `tested`, wording "contract evidence" | Only after the lane's v2 runner replays the committed corpus **and** the report validator accepts its v2 manifest | LG02 `:462-464` |
| `proved` | **Never for a non-Rust lane** until a lane front end exists | VER31 `:397-398`; `logic-gates-everywhere.md:155-156` |

The gate prints the label in `gate.json`; `notes` copies it. A release text that
says "lane-local exhaustive" for a lane with a failing matrix row is a gate
**refusal**, because the label may be used only when every row passes
(LG02 `:631-636`).

### 2.3 Refusal conditions (new ones in bold)

The gate refuses, and no publish job runs, when:

1. any floor check F1-F12 fails (VER31 `:341-363`);
2. **the package depends on an unpublished sibling (B1)**: every first-party
   dependency named in the manifest must already exist on the same registry at a
   version satisfying the constraint, verified by a registry query in `gate`
   (open item O1);
3. **the manifest name or version differs from the tag**, the version is
   already on the registry (F11), or **the name differs from the one recorded
   for the lane in the registry table**;
4. **the disclosure of section 2.4 is missing or not byte-equal to the
   generated block**;
5. **the lane claims a label stronger than section 2.2 allows**;
6. the tagged commit is not on `main`, the tag is unsigned or lightweight, or
   the signature flags lack presence or verification (VER31 `:523-537`;
   `publish.yml:83-92` already enforces the on-`main` rule);
7. the job would run without the `release` environment.

### 2.4 Disclosure requirement

Three places must state what is verified, and the gate checks that they carry
the same lane block:

1. The package **README "What is verified"** section, generated from the report
   (`logic-gates-everywhere.md:92-93`).
2. The **release notes** (VER31 section 8, generated by `release_gate.py notes`).
3. The shipped **`ASSURANCE.md`** (F10, VER31 `:361`).

Items 2 and 3 carry the **VER15a lane weakness matrix block verbatim**: the
first line with the counts, one `open:` entry per open row with its reason and
its `reached:` label, and the mandatory footer `Rows cover registry classes and
listed lane threats, not every possible bug.` (VER15a section 7 as proposed in
PR #19954, rules 5-7; re-cite line numbers when that document merges). The block
appears in both documents or the disclosure is a failure. Until LGE-S5 and
LGE-W1 produce it, checklist item **RC-WEAKNESS-DISCLOSURE** applies: the block
is pasted by hand and the release is not made without it. The block does not
lower the tier; it says what is and is not verified.

---

## 3. Tags, parsing and routing

### 3.1 Current hardening (kept)

`publish.yml` is already hardened, and this section only extends it:

- the tag is "attacker-shaped text" and never appears inside a `run:` script;
  it reaches the shell through `env:` only (`publish.yml:15-23, 96-97`);
- a strict pattern is applied before anything is written to `$GITHUB_OUTPUT`
  (`publish.yml:99-108`);
- the package name charset is `[A-Za-z0-9_-]`, so no `/`, `.` or metacharacter
  reaches a path (`publish.yml:102-104`);
- the tagged commit must be an ancestor of fully qualified
  `refs/remotes/origin/main` (`publish.yml:78-92`);
- downstream jobs read the directory through `env:` too (`publish.yml:194, 277`);
- the directory must exist and contain the build file the publish job uses
  (`publish.yml:140-153`);
- actions are pinned by commit SHA (`publish.yml:71, 185, 198`);
- the default workflow permission is `contents: read`, widened per job
  (`publish.yml:53-54, 176-177`); concurrency never cancels a publish
  (`publish.yml:49-51`).

### 3.2 Tag grammar for every language

One grammar, applied by `parse-tag` for all lanes:

```
<lane>/<dir>/v<version>
    lane: rust typescript python ruby c cpp java kotlin csharp fsharp
          elixir lua perl haskell dart ocaml swift
code/packages/go/<dir>/v<version>        (Go only, B7)
```

- `<dir>` is the package directory name under `code/packages/<lane>/`, charset
  `[A-Za-z0-9_-]+`. The gate reads the registry name from the manifest. Python
  and Ruby keep their registry-name tags for compatibility (`publish.yml:25-26`;
  VER31 `:457-460`).
- The registry version is **derived from the manifest**, never from the tag;
  the tag must equal it (F11, VER31 `:362`).
- **The lane is an allow-list**, written as an alternation in the regular
  expression, not a wildcard: `^(rust|typescript|...)/[A-Za-z0-9_-]+/v...$`. The
  Go shape is a second, separate anchored pattern with a fixed literal prefix.
- Pre-release suffixes keep the existing forms (`publish.yml:99-104`). Lane
  version forms to verify before implementing: Cargo and npm take semver; PyPI
  PEP 440; RubyGems `x.y.z.pre`; Hackage the four-part PVP version
  (`haskell/logic-gates/logic-gates.cabal:3` is `0.3.0.0`); LuaRocks carries a
  rock revision (`rockspec:2` is `0.4.0-1`); Perl a decimal or dotted version.
  The gate maps the manifest version to the tag by a per-lane function with
  falsification tests, and refuses when it cannot.
- `parse-tag`'s `if:` filter (`publish.yml:59-61`) must list every new prefix, so
  a foreign product tag still skips the workflow, as it does now
  (`publish.yml:10-13`).
- The tag-protection ruleset and the `release` environment's deployment tag
  patterns extend VER31's four lanes (VER31 `:539-545, 651-655`) to all of
  them, plus `code/packages/go/**`. In GitHub's patterns `*` stops at `/`, so
  use `<lane>/**` (VER31 `:539-545`).
- **The Go tag does not name the lane in its first segment**, so a rule keyed on
  the first segment would miss it. The workflow's trigger filter and the ruleset
  both list `code/packages/go/` explicitly.

### 3.3 Routing table

`parse-tag` outputs `language`, `package`, `version`, `pkg_dir`
(`publish.yml:63-68`) and, new, `registry`. Each publish job's `if:` tests
`registry`, never the raw tag.

| `language` | `registry` | Job | Environment | Artifact built in `gate` |
|---|---|---|---|---|
| python | pypi | `publish-python-*` (exist, `publish.yml:171, 248`) | `release` | sdist, wheels |
| ruby | rubygems | `publish-ruby` (exists, `publish.yml:287`) | `release` | `.gem` |
| rust | crates | `publish-crates` | `release` | `.crate` |
| typescript | npm | `publish-npm` | `release` | `.tgz` |
| go | goproxy | `publish-go` (verification only) | `release` | none |
| java, kotlin | maven | `publish-maven` | `release` | signed bundle |
| csharp, fsharp | nuget | `publish-nuget` | `release` | `.nupkg` |
| elixir | hex | `publish-hex` | `release` | hex tarball |
| lua | luarocks | `publish-luarocks` | `release` | `.src.rock` |
| perl | cpan | `publish-cpan` | `release` | dist tarball |
| haskell | hackage | `publish-hackage` | `release` | `sdist` tarball |
| dart | pubdev | `publish-pubdev` | `release` | package archive |
| ocaml | opam | `prepare-opam` (opens a PR) | `release` | opam file, source archive |
| swift | none until O3 | none | none | none |
| c, cpp | github-release | `publish-source-release` | `release` | source tarball |

Today the existing environments are `pypi` and `rubygems`
(`publish.yml:175, 252, 294`); VER31 collapses them into one `release`
environment (VER31 `:539-545`), and this document follows that.

### 3.4 Go

There is no upload. `publish-go` only (a) confirms the tag
`code/packages/go/<dir>/vX.Y.Z` is signed and on `main`, (b) confirms `go.mod`
has no `replace` directive and no `v0.0.0` requirement on a sibling (B1, B7), and
(c) after the owner approves, requests the version from `proxy.golang.org`
through `go list -m` from a clean temporary module (verify before implementing),
so the proxy caches it and the checksum database records it. The tag-protection
ruleset is the only publishing control for Go and must cover the Go tag shape.
A published Go version cannot be removed from the proxy (verify before
implementing); the remedy is a later `retract` directive.

---

## 4. Job shape per registry

### 4.1 Common shape (every job)

```yaml
  publish-<registry>:
    needs: [parse-tag, gate, verify-tag, attest]
    if: needs.parse-tag.outputs.registry == '<registry>'
    runs-on: ubuntu-latest
    environment: release           # required reviewer, no admin bypass
    permissions:                   # least privilege, per job
      contents: read
      id-token: write              # only where the registry uses OIDC
    steps:
      - uses: actions/download-artifact@<full sha>   # the gated bytes
      - name: Refuse unless digests equal gate.json
        env: { ... }                                 # values via env only
        run: ...
      - uses: <registry action>@<full sha>           # pinned
```

Rules, in order of importance:

1. **No build tool runs in a publish job.** Artifacts are built in `gate`; the
   publish job only compares and uploads, so a build backend cannot publish
   another package under the shared environment (VER31 `:433-444`). Where a
   registry's tool insists on building (crates.io, some Maven plugins), the job
   re-derives and compares the digest, or fails closed (VER31 `:436-444`).
2. `permissions:` is set on every job, `contents: read` by default
   (`publish.yml:53-54`), and `id-token: write` only in jobs whose registry
   uses OIDC. `attest` is the only job with `attestations: write` outside the
   `release` environment (VER31 `:427-432`).
3. Secrets are **environment secrets of `release`**, never repository secrets,
   so they are released only to an approved run. This is the fix the RubyGems
   job already asks for (`publish.yml:291-293`). A secret is passed through
   `env:` to the one step that needs it (as `GEM_HOST_API_KEY`,
   `publish.yml:318`), never written to disk or spliced into a script.
4. Every action is pinned by full commit SHA with the version in a comment
   (`publish.yml:71, 198`). New actions are looked up and pinned when the job is
   written; this document does not invent SHAs.
5. Every job checks out `${{ github.sha }}`, never the tag (VER31 `:445-446`),
   with `persist-credentials: false` (`publish.yml:72-73`).
6. The job first checks whether the version already exists; if so it stops with
   a clear message (VER31 `:609-611`).

### 4.2 Registry-specific notes

- **crates.io.** `rust-lang/crates-io-auth-action` mints a short-lived token in
  the `release` environment (VER31 `:596`). Steps: `cargo package --no-verify`,
  digest equals `gate.json`, `cargo publish --no-verify`, download from
  `static.crates.io`, compare (VER31 `:433-440`). Blockers: B1 (a path
  dependency needs a version, verify before implementing) and B4.
- **npm.** Publish the gate's tarball with `--provenance --access public`, no
  token (VER31 `:433, 597, 612-614`). Needs `id-token: write` and an npm CLI new
  enough for trusted publishing (verify before implementing).
- **PyPI.** Keep the SHA-pinned `pypa/gh-action-pypi-publish`
  (`publish.yml:197-200`) and enable attestations (VER31 `:598`). The job needs
  only `id-token: write` (`publish.yml:176-177`).
- **RubyGems.** Replace the in-job `gem build` (`publish.yml:306-311`) by the
  gated `.gem`; prefer a trusted publisher over `GEM_HOST_API_KEY`
  (`publish.yml:313-321`; VER31 `:599`).
- **Maven Central.** `gate` builds and signs; the job uploads the bundle to the
  Central Publisher Portal with the user token (verify before implementing the
  upload tool and whether a Gradle plugin or the portal API is used). The GPG
  private key is an environment secret imported into a temporary keyring in one
  step and removed after. Artifacts need sources and javadoc jars and a POM with
  licence, SCM and developer data (verify before implementing the current
  requirements). Blockers B2, B3.
- **NuGet.** Push the gated `.nupkg` with `dotnet nuget push`; prefer trusted
  publishing through OIDC, otherwise a scoped API key (verify before
  implementing). Package metadata already sets `PackageLicenseExpression` to MIT
  (`csharp/logic-gates/CodingAdventures.LogicGates.csproj:11`,
  `fsharp/logic-gates/CodingAdventures.LogicGates.fsproj:9`).
- **Hex.** `mix hex.publish` with the key as an environment variable in one
  step. `mix.exs` shows only `app` and `version` at `mix.exs:6-7`, with no
  package metadata (licences, links) in the part read; add it first (verify
  before implementing what Hex requires).
- **LuaRocks.** `luarocks upload <rockspec>` with an API key (verify before
  implementing whether the key may be passed through an environment variable
  rather than an argument visible in the process list). The rockspec carries its
  own revision (`rockspec:2`).
- **CPAN.** PAUSE upload of the gated tarball (`make dist` runs in `gate`).
  `Makefile.PL` already sets `LICENSE => 'mit'` (`Makefile.PL:10`). Verify
  before implementing the upload client and credential handling.
- **Hackage.** `cabal upload` from the sdist built in `gate` with a token (verify
  before implementing). The default upload is a *candidate* that a maintainer
  publishes, a safer first step.
- **pub.dev.** OIDC automated publishing: the job needs `id-token: write` and a
  tag pattern configured on pub.dev (verify before implementing). The
  `publish_to: none` guard (B5) is removed deliberately in the release PR.
- **opam.** `prepare-opam` builds the source archive and the opam file with its
  checksum, then opens a pull request to `ocaml/opam-repository` with a
  fine-grained credential limited to the owner's fork (verify before
  implementing); maintainers merge it. The job reports "opened", never
  "published". The opam file pins exact compiler and tool versions
  (`ocaml/logic-gates/coding-adventures-logic-gates.opam:12-18`); maintainers may
  ask for ranges (verify before implementing).
- **Swift.** Not a job until O3 is decided (B6).
- **C and C++.** A GitHub Release asset built by `git archive` from the tagged
  commit with a fixed prefix and a fixed modification time so the tarball is
  reproducible, a `SHA256SUMS` file, a signature over the checksums (method O4),
  and a GitHub build-provenance attestation (VER31 `:612-615`). vcpkg and Conan
  are later work, each a pull request to its registry and so not automatic.

---

## 5. Supply chain and release order

### 5.1 Build from the tagged commit only; order

- The gate builds from `${{ github.sha }}` once, records digests, and the
  publish job uploads those exact bytes (VER31 `:433-446`). A moved tag cannot
  change what ships.
- **Release order is leaf to root.** For logic gates: `transistors` in each lane
  first, then `logic-gates`. A lane whose dependency is unpublished is refused
  (section 2.3 item 2). Within one lane the order is the build graph the build
  tool already computes. This is the same rule as the BUILD-file lesson on
  leaf-to-root installs (`CLAUDE.md`, working principle 7).
- Lanes are independent of each other: one lane failing the gate blocks only
  that lane's job.

### 5.2 Provenance, checksums, SBOM, long-lived tokens

| Control | Where |
|---|---|
| npm provenance (Sigstore) | `--provenance` (VER31 `:612-614`) |
| PyPI attestations | `pypa/gh-action-pypi-publish` (VER31 `:598`) |
| GitHub build-provenance attestation for every other artifact, verified with `gh attestation verify` pinned to the signer workflow and source ref | VER31 `:601-607, 612-615` |
| SLSA level | Whatever the attestation gives; a named level is claimed only after checking that the release workflow meets it (verify before implementing; none claimed now) |
| SHA-256 digests | In `gate.json`, compared in every publish job and, where possible, against the published artifact (VER31 `:433-444`) |
| Reproducible archives | Required for C and C++ source tarballs. For other lanes the gate records the digest and does not claim reproducibility unless two builds agree; VER31 already fails closed when `cargo package` is not reproducible (VER31 `:443-444`) |
| SBOM | A dependency list per artifact is generated in `gate` (format is open item O5). Logic gates ships no third-party runtime dependency, so its SBOM lists the first-party closure (B1) only |
| No long-lived token where OIDC exists | crates.io, npm, PyPI and pub.dev use OIDC; RubyGems and NuGet probably can (verify). Maven, Hex, LuaRocks, CPAN and Hackage need a token (section 1.2): environment secrets of `release`, scoped to one package where the registry allows, rotated by the owner |
| Environment protection | `release` with the owner as required reviewer, administrator bypass off, tag-pattern deployment restriction (VER31 `:539-545`). The owner approves only in the web UI (VER31 `:576-577`) |
| Machine account for agents | Required by VER31 `:566-590`; until it exists the release notes state the residual risk (VER31 `:580-585`) |

### 5.3 Dry runs (no secrets)

VER31 already defines a `workflow_dispatch` dry run that runs `gate` only, with
no `attest`, no publish and no tag-signature check, whose `package` and `ref`
inputs reach the shell through `env:` and must match the tag pattern
(VER31 `:447-452`). `gate` runs the registry's own dry-run command for the lane,
so an unpublished package is checked for metadata and packaging errors without a
credential. All flags are **verify before implementing**.

| Registry | Dry-run command |
|---|---|
| crates.io | `cargo publish --dry-run` (also `cargo package --list`) |
| npm | `npm publish --dry-run` and `npm pack --dry-run --json` (already the F4 scope source, VER31 `:352-355`) |
| PyPI | `python -m build` then `twine check` |
| RubyGems | `gem build` then `gem specification` |
| Go | `go vet`, and a check that `go.mod` has no `replace` |
| Maven | build to a local staging directory and validate signatures and POM |
| NuGet | `dotnet pack` and `dotnet nuget verify` of the package |
| Hex | `mix hex.build` |
| LuaRocks | `luarocks lint` and `luarocks pack` |
| CPAN | `make dist` and `make disttest` |
| Hackage | `cabal check` and `cabal sdist` (a candidate upload needs a token, so it is not a dry run) |
| pub.dev | `dart pub publish --dry-run` |
| opam | `opam lint` |
| C, C++ | build the tarball twice and compare digests |

### 5.4 First real release per lane: the owner's go-ahead (a checklist, not automation)

The first real release of each lane needs the owner's explicit go-ahead **in
chat** (`logic-gates-everywhere.md:149`). This is a human step and is
deliberately not automated. It is the checklist item
**RC-FIRST-RELEASE-<lane>**:

1. The lane's dry run (section 5.3) is green on the commit to be tagged.
2. `gate.json` for that commit prints the label of section 2.2 and the lane's
   disclosure block (section 2.4); README, release notes and `ASSURANCE.md`
   agree.
3. Every owner action for that registry (section 1.2) is done and recorded.
4. Findings B1-B9 that touch the lane are closed or consciously accepted.
5. **The owner says "go" in chat**, naming the lane and the version.
6. The owner signs the tag with the security key and creates the release; the
   `release` deployment is approved in the web UI only (VER31 `:500-521,
   576-577`).

No agent performs steps 5 or 6, and an approval seen anywhere other than the
owner's chat message does not count.

---

## 6. Honest limits

What a published package claims, and nothing more:

- **A published logic-gates package is `tested`, wording "lane-local exhaustive
  over U8 (0..255)"**, with the lane matrix block (sections 2.2, 2.4). It is not
  `proved` and not "contract evidence" (LG02 `:455-464`; VER31 `:397-398`).
- "Every CVE class" means the registry classes plus the lane's threat list, each
  closed by its own evidence; it is not a claim about every possible bug
  (`logic-gates-everywhere.md:153-154`; the footer of section 2.4).
- The lane oracle lives in the lane's own test file, and no sabotage run is
  replayed against it (LG02 `:457-460`).

What stays open and is **not** closed by this document:

| Open | Where tracked |
|---|---|
| Capability analyzers for lanes that lack one (VER08c, LGE-A1, LGE-A2); the lane weakness rows that depend on them stay `open` and are disclosed | `logic-gates-everywhere.md:144-146` |
| Lane runners replaying the v2 corpus (LGE-CV2, LGE-L-<lane>); until then the label stays "lane-local exhaustive" | LG02 `:462-464`; `logic-gates-everywhere.md:142-143` |
| The gate itself, signers file, CODEOWNERS, signed buckets (LV-007a) | VER31 `:404-417` |
| Machine account for agents | owner (VER31 `:566-590`) |
| Release signing for C and C++, SBOM format, whether a first-party dependency satisfies F8 | O1, O4, O5 below |

### Open decisions for the owner

| # | Decision |
|---|---|
| O1 | Does a first-party published dependency (`transistors`) satisfy F8, or must logic gates vendor it? |
| O2 | Maven namespace: `com.codingadventures` (needs the domain) or `io.github.adhithyan15`? |
| O3 | Swift: mirror repository, root shim, or defer? |
| O4 | C and C++ signing: Sigstore keyless, or the owner's SSH key? |
| O5 | SBOM format: CycloneDX or SPDX? |
| O6 | Crate and Hackage names: prefix them (`coding-adventures-logic-gates`)? |

---

## 7. Amendments to other documents

| Document | Change |
|---|---|
| VER31 section 7 | The gate's `parse-tag` allow-list covers all lanes (section 3.2 here), the Go tag shape, and the `registry` output |
| VER31 sections 9.2, 9.6 | Deployment tag patterns and the tag ruleset cover every lane and `code/packages/go/**` |
| VER31 section 9.3 | The table gains the rows of section 1.1 here |
| `.github/workflows/publish.yml` | Not changed by this document; changes ship with each LGE-PUB row, one registry per PR, after LV-007a |

---

## 8. Backlog

See `VER-BACKLOG.d/logic-gates-everywhere.md`: rows `LGE-S5` (this document),
`LGE-PUB-<registry>` (one per registry) and `LGE-OWNER-ACCOUNTS`.
