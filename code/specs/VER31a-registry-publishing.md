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
| rust | crates.io | `logic-gates` (`Cargo.toml:2`); **rename** (B4) | `cargo package --no-verify` in `gate`; **owner-manual** upload of the gated `.crate` (section 4.2: `cargo publish` rebuilds) | Trusted publishing exists (VER31 `:596`) but is unused until an upload-only route is verified; first version by hand |
| typescript | npm | `@coding-adventures/logic-gates` (`package.json:2`) | Publish the gate's tarball with `npm publish --provenance --access public` (VER31 `:433, 597`) | Yes (OIDC); first version by hand (VER31 `:597`). Needs the `@coding-adventures` scope |
| python | PyPI | `coding-adventures-logic-gates` (`pyproject.toml:6`) | Existing job (`publish.yml:171-200`) with PyPI attestations (VER31 `:598`) | Yes, existing; pending publishers allow OIDC from the first version (VER31 `:598`) |
| ruby | RubyGems | `coding_adventures_logic_gates` (`gemspec:6`) | Existing job with API key (`publish.yml:287-321`); move to trusted publishing (VER31 `:599`) | Yes, recommended (verify the action and settings before implementing) |
| go | Go module proxy | `github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates` (`go.mod:1`) | **Tag only**; `proxy.golang.org` fetches on first request (verify before implementing) | n/a (no upload, no token) |
| java | Maven Central | `com.codingadventures:logic-gates` (**rename** the artifact id, B2/B3) | Central Publisher Portal upload of a signed bundle (verify before implementing the plugin) | Portal user token (secret) and GPG signing key. No OIDC (verify before implementing) |
| kotlin | Maven Central | `com.codingadventures:logic-gates` (**rename**, B2/B3) | as Java | as Java |
| csharp | NuGet | `CodingAdventures.LogicGates.CSharp` (`csproj:7`) | `dotnet nuget push` of the gate's `.nupkg` | Trusted publishing via GitHub OIDC (verify before implementing); else API key |
| fsharp | NuGet | `CodingAdventures.LogicGates.FSharp` (`fsproj:5`) | as C# | as C# |
| elixir | Hex | `coding_adventures_logic_gates` (`mix.exs:6`) | **Owner-manual** (section 4.2: `mix hex.publish` evaluates `mix.exs`) | API key held by the owner, package-scoped where supported (verify before implementing) |
| lua | LuaRocks | `coding-adventures-logic-gates` (`rockspec:1`) | **Owner-manual** (section 4.2: upload evaluates the rockspec) | API key held by the owner (verify before implementing) |
| perl | CPAN | distribution `CodingAdventures-LogicGates` (`Makefile.PL:6`) | Upload a `make dist` tarball to PAUSE | PAUSE credentials; no OIDC (verify before implementing) |
| haskell | Hackage | `logic-gates` (`cabal:2`); **rename** (B4) | `cabal sdist` in `gate`; **candidate-only** upload of the gated sdist from `release-hackage`; the owner publishes the candidate by hand | package-scoped Hackage token where supported (verify before implementing) |
| dart | pub.dev | `coding_adventures_logic_gates` (`pubspec.yaml:1`); needs B5 | **Owner-manual** (section 4.2: `dart pub publish` evaluates the package) | OIDC automated publishing exists but runs that tool, so it is unused unless an upload-only route is verified |
| ocaml | opam-repository | `coding-adventures-logic-gates` (`opam:2`) | **Pull request** to `ocaml/opam-repository`, not an upload; `opam publish` opens it (verify before implementing) | None. A PR reviewed by repository maintainers |
| swift | SwiftPM (git) | `logic-gates` (`Package.swift:22`) | **Blocked** by B6 | none |
| c | GitHub release | `c/logic-gates` | Source tarball asset plus checksum plus signature on the GitHub Release; vcpkg and Conan later | GitHub artifact attestation |
| cpp | GitHub release | `cpp/logic-gates` (not on `main`, B8) | as C | as C |

**Mechanism column caveat:** section 4.2 overrides it. Where it says a tool such as `cargo publish`, `mix hex.publish`, `luarocks upload` or `dart pub publish`, that tool rebuilds or evaluates package code, so the lane is owner-manual (or HTTP-API upload) and never runs in a publish job with a credential.

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
| crates.io | Create or confirm the account; choose and **claim the crate name** by uploading the exact gated `.crate` by hand (digest equals `gate.json`, never a local rebuild; VER31 `:596`), revoke the personal token immediately, and state in the release notes that no provenance exists for that version; register the trusted publisher pinned to `release-crates` (VER31 `:554-557`). Every later crates.io upload is also owner-manual (section 4.2). No secret enters the repository. |
| npm | Create the `coding-adventures` organisation (scope); enable security-key 2FA; publish the first version by hand from the exact gated tarball (VER31 `:597`; digest equals `gate.json`, no provenance for that version, 2FA session or token revoked after); register the trusted publisher pinned to `release-npm`. |
| PyPI | Register a pending or normal trusted publisher per package pinning environment `release-pypi`; existing registrations that do not pin an environment are re-registered (VER31 `:598`). |
| RubyGems | Configure a trusted publisher on rubygems.org for each gem, pinned to `release-rubygems` (required; verify before implementing). `RUBYGEMS_API_KEY` (`publish.yml:34-35, 318`) is retired; a key survives only as a time-boxed fallback with an expiry and one-package scope. |
| Go proxy | Nothing to create. The owner runs the pre-tag gate on the exact SHA, then creates the signed tag, which **is** the publish (section 3.4); the signed-tag ruleset on `code/packages/go/**` is the only control. The repository must be publicly fetchable (verify before implementing). |
| Maven Central | Create a Central Publisher Portal account; **verify the namespace** (domain TXT record for `com.codingadventures`, or switch to `io.github.adhithyan15`; decision O2); generate a GPG **signing subkey** (the primary key stays offline with the owner), publish the public key to a keyserver, and store the private key, its passphrase and the portal user token as secrets of `release-maven` only (never available to `gate`, section 4.2). Use a noreply or role address in the POM developer data and the portal profile. |
| NuGet | Create the nuget.org account; reserve the `CodingAdventures.` ID prefix (verify before implementing); configure a trusted publisher for repository, workflow and `release-nuget` (required; a key is only a time-boxed, one-package fallback). |
| Hex | Create the hex.pm account; uploads are owner-manual unless a reviewed API client is adopted (section 4.2), in which case store the key in `release-hex`. |
| LuaRocks | Create the luarocks.org account; uploads are owner-manual unless an upload-only route is verified (section 4.2). |
| CPAN | Request a PAUSE account (a human approval step at PAUSE); uploads are owner-manual, or use a dedicated PAUSE account owning only this distribution with credentials in `release-cpan` only. A new distribution name is claimed by its first upload (verify before implementing). |
| Hackage | Create the Hackage account and, if needed, request upload rights (verify before implementing); create a package-scoped token (where supported) allowed to upload **candidates only**, store it in `release-hackage`, rotate it after each release, and **publish each candidate by hand**. Verify before implementing that the Hackage token can be limited to candidate uploads; if it cannot, Hackage becomes owner-manual. |
| pub.dev | Create or use a Google account; uploads are owner-manual unless an upload-only route is verified (section 4.2, `dart pub publish` evaluates the package). |
| opam | Create a dedicated machine GitHub account and fork of `ocaml/opam-repository`, with a fine-grained token in `release-opam`; respond to maintainer review. Not automatable end to end. |
| Swift | Decide B6 (mirror repository or shim; O3). If a mirror, create it and grant the workflow a fine-grained push credential limited to it. |
| C / C++ | Choose and set up release signing (O4); nothing registry-side. |
| GitHub (all) | Create one environment per registry (`release-crates`, `release-npm`, `release-pypi`, `release-rubygems`, `release-maven`, `release-nuget`, `release-hex`, `release-luarocks`, `release-cpan`, `release-hackage`, `release-pubdev`, `release-opam`, `release-source`), each with the owner as required reviewer, administrator bypass off, deployment restricted to that lane's tag pattern, and its trusted publisher pinned to it; for Go, the signed-tag ruleset on `code/packages/go/**` is the only control (section 3.4); create the tag-protection ruleset (VER31 `:651-655`); create the machine account and keep the owner token off agent hosts (VER31 `:566-590`); add `.github/release-signers`, its signature and CODEOWNERS (VER31 `:500-521, 639-643`). |

### 1.3 Names to claim or reserve (owner actions)

These are **owner actions**: agents never create accounts, tokens or
reservations. The list covers the logic-gates **dependency closure**
(`logic-gates` and `transistors`, B1) in every lane; the `transistors` names
are read from its own manifests at claim time and appended here. A name is
claimed by its first real upload or by the registry's reservation mechanism;
until then it is exposed to squatting.

| Registry | Claim or reserve | Names |
|---|---|---|
| npm | the `@coding-adventures` organisation (scope) | `@coding-adventures/logic-gates`, `@coding-adventures/transistors`; also the unscoped lookalikes `coding-adventures-logic-gates`, `logic-gates-coding-adventures` as placeholders (verify before implementing whether placeholders are wanted) |
| NuGet | the `CodingAdventures.` ID-prefix reservation | `CodingAdventures.LogicGates.CSharp`, `.FSharp`, and the transistors IDs |
| Maven Central | namespace verification **before any build** (O2) | group `com.codingadventures` or `io.github.adhithyan15`; artifact ids from B2 |
| pub.dev | a **verified publisher** (a domain) | `coding_adventures_logic_gates`, `coding_adventures_transistors` |
| CPAN / PAUSE | the `CodingAdventures::` namespace (a PAUSE permission request, verify before implementing) | `CodingAdventures::LogicGates`, `CodingAdventures::Transistors`; distribution `CodingAdventures-LogicGates` |
| Hex | the package names | `coding_adventures_logic_gates`, `coding_adventures_transistors` |
| LuaRocks | the rock names | `coding-adventures-logic-gates`, `coding-adventures-transistors` |
| Hackage | the package names (O6) | the prefixed names chosen for `logic-gates` and `transistors` |
| opam | the package names, claimed by the first merged PR | `coding-adventures-logic-gates`, `coding-adventures-transistors` |
| crates.io, PyPI, RubyGems | first upload | `coding-adventures-logic-gates` family, `coding_adventures_logic_gates` (RubyGems), the chosen crate name (O6) |
| Go | nothing to claim; the module path is the repository path | `github.com/adhithyan15/coding-adventures/code/packages/go/{logic-gates,transistors}` |

**Typosquat variants.** For every claimed name the owner also reserves, where
the registry normalises or allows it, the underscore, hyphen and prefix variants
(`coding_adventures_x`, `coding-adventures-x`, `codingadventures-x`, and the
bare `x` where the name is not already ours by prefix), or records why a
variant is not reserved. PyPI and RubyGems normalise some separators; verify
per registry before implementing. Every claimed name goes into the first-party
name map of section 2.3 item 2 with its expected owner and its declared
verification method.

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
2. **the package depends on a sibling that is unpublished or not ours (B1)**.
   "Exists on the same registry at a satisfying version" is not enough: a
   squatter can own an unscoped name, and a registry resolves by name. For
   every first-party dependency named in the manifest the gate must verify, by a
   registry query in `gate`, that (a) it exists at a satisfying version,
   (b) the registry's **owner or publisher identity equals the one committed in
   the first-party name map** (`code/assumptions/first-party-names.json`: lane,
   registry, name, expected owner or trusted-publisher identity; changes need
   CODEOWNERS review, VER31 `:639-643`), and (c) the **published digest equals
   the digest in our own release record** (the earlier release's `gate.json`).
   A name missing from the map, an owner mismatch or a digest mismatch is a
   refusal.
   **The check fails closed.** Any query error, timeout, non-200 response,
   unparsable body, or absent owner or digest field is a refusal; there is no
   retry-to-pass; the check runs in `gate` with no cached result (rule 3 of
   section 4.1). For a registry that exposes no comparable owner identity, the
   name-map entry must declare an **explicit per-registry verification method**
   (for example "the digest of the published artifact equals the committed
   digest, and the first-ever upload was owner-manual"); an entry without one,
   or whose declared method cannot run, is a refusal. **Trust anchor:** the
   earlier release's `gate.json` digests (or a digest file derived from them)
   are **committed next to the name map under CODEOWNERS**
   (`code/assumptions/first-party-digests.json`); they must not live only in a
   writable GitHub release or workflow artifact. Falsification tests, one per
   case: query error; timeout; HTTP 404, 500 and 302; non-JSON body; JSON
   missing the owner field; JSON missing the digest field; owner differs;
   digest differs; name absent from the map; map entry with no verification
   method; a stale cached response present on disk (must be ignored). Open item O1 (whether a first-party dependency satisfies F8) is
   unchanged;
3. **the manifest name or version differs from the tag**, the version is
   already on the registry (F11), or **the name differs from the one recorded
   for the lane in the registry table**;
4. **the disclosure of section 2.4 is missing or not byte-equal to the
   generated block**;
5. **the lane claims a label stronger than section 2.2 allows**;
6. the tagged commit is not on `main`, the tag is unsigned or lightweight, or
   the signature flags lack presence or verification (VER31 `:523-537`;
   `publish.yml:83-92` already enforces the on-`main` rule);
7. the job would run without its registry's own environment (section 3.3).

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

**Fail closed until the generator exists.** A checklist is not a control. Until
`release_gate.py notes` produces the block, every publish job **refuses** unless
(a) a per-lane disclosure file is committed (for example
`code/specs/data/disclosure/<lane>/<package>.md`), reviewed and approved by the
owner through CODEOWNERS, and (b) the `ASSURANCE.md` in the artifact and the
release notes each contain a block **byte-equal** to that file. The disclosure
file must also name the **first-party dependency closure and each member's
tier**: for logic gates that means `transistors` (B1) with its own lane tier,
so a reader sees what the package builds on. A disclosure that lists the
package alone is a refusal.

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
- The tag-protection ruleset and each registry environment's deployment tag
  patterns extend VER31's four lanes (VER31 `:539-545, 651-655`) to all of
  them, plus `code/packages/go/**`. In GitHub's patterns `*` stops at `/`, so
  use `<lane>/**` (VER31 `:539-545`).
- **The Go tag does not name the lane in its first segment**, so a rule keyed on
  the first segment would miss it. The workflow's trigger filter and the ruleset
  both list `code/packages/go/` explicitly.

- **Manifest-derived values are untrusted until validated.** The registry name
  and version the gate reads from a manifest (a `package.json` or `.gemspec` is
  attacker-shaped text just as a tag is) are checked before anything is written
  to `$GITHUB_OUTPUT`, reach later steps through `env:` only, and are
  URL-encoded before being put in a registry query URL.
  - **Match semantics.** Python `re.fullmatch` (or `\Z`), never a bare `$`,
    which accepts a trailing newline. Before matching, **reject any control
    character (including `\n`, `\r`, NUL) and any non-ASCII byte**; this is
    what keeps an injected line out of `$GITHUB_OUTPUT`.
  - **Per segment.** Rules apply to every `/`-separated segment separately,
    including the `name` after the `/` of an npm `@scope/name` (the scope
    matches `@[a-z0-9][a-z0-9_-]{0,63}`, the name `[a-z0-9][a-z0-9._-]{0,63}`);
    a segment never contains `..`, never starts with `.` or `-`, never ends
    with `.`, and is never empty. Length cap 128 for the whole name.
  - **Per-registry patterns** are fixed in the gate (never one shared wildcard)
    and falsification-tested. The shared safe base is a segment
    `[A-Za-z0-9][A-Za-z0-9._-]{0,63}`; each registry narrows it (Hex and Dart
    `[a-z][a-z0-9_]{0,63}`; crates `[A-Za-z][A-Za-z0-9_-]{0,63}`; NuGet
    dot-separated segments of the base; Go module path fixed by the lane
    prefix plus `<dir>`).
  - **Normalised comparison.** PyPI names compare after PEP 503 normalisation
    (lower-case, runs of `-_.` to one `-`) and RubyGems names compare
    case-sensitively and exactly; the gate compares the normalised form of the
    manifest name with the normalised form of the name-map entry, so
    equivalent spellings of the same project (`Coding_Adventures-Logic.Gates`
    and `coding-adventures-logic-gates`) do not cause a false mismatch.
    Normalisation does **not** block lookalike names; those are handled by
    reserving them (LGE-OWNER-NAMES, section 1.3).
  - **Not yet specified.** The exact PyPI, Maven and CPAN name patterns are
    written when the gate is implemented; Maven group segments (and the group
    as a whole) may need a cap longer than 64.
  - **Version** (no nested ambiguous quantifier, so no ReDoS):
    `^[0-9]+\.[0-9]+\.[0-9]+(?:[.-][0-9A-Za-z]+)*$`, with a mandatory
    separator before every suffix part, applied with `fullmatch` after a
    **length cap of 64**. Lane forms that differ (PEP 440 `1.2.3rc1`, Hackage
    four-part `0.3.0.0`, LuaRocks `0.4.0-1`) are mapped by per-lane functions
    that emit this canonical shape or refuse.
  - **Accept/reject list** (part of the test suite): accept `0.1.0`,
    `1.2.3-rc.1`, `0.3.0.0`, `0.4.0-1`, `@coding-adventures/logic-gates`,
    `coding_adventures_logic_gates`; reject `0.1.0\n`, `0.1.0 `, `1.2`,
    `1.2.3-`, `1.2.3..4`, `1.2.3-a-b-c-...` beyond the length cap,
    `@/x`, `@scope/..`, `@scope/a/b`, `../x`, `-x`, `x.`, `.x`, `x\\y`,
    `x\0y`, `x\ny`, a name with a non-ASCII letter, and the empty string.
- **Go parsing.** The current `cut -d/ -f1`, `-f2`, `-f3` split
  (`publish.yml:110-112`) breaks on
  `code/packages/go/<dir>/vX.Y.Z`, which has six segments. Go tags are matched
  by their own anchored pattern with the literal prefix
  `^code/packages/go/([A-Za-z0-9_-]+)/v(...)$` and the groups taken from the
  regular-expression match (`BASH_REMATCH`), not from `cut`. Go has no publish
  job (section 3.4), so this parse runs in the pre-tag `workflow_dispatch` gate
  with the same `package` and `ref` allow-list as the dry run (VER31 `:447-452`).

### 3.3 Routing table

`parse-tag` outputs `language`, `package`, `version`, `pkg_dir`
(`publish.yml:63-68`) and, new, `registry`. Each publish job's `if:` tests
`registry`, never the raw tag. **Each registry has its own GitHub environment**
(not one shared `release`, which VER31 `:539-545` specifies and this document
amends; see section 7): a secret or OIDC trust in one environment can then
never publish to another registry.

| `language` | `registry` | Job | Environment | Artifact built in `gate` |
|---|---|---|---|---|
| python | pypi | `publish-python-*` (migrated, below) | `release-pypi` | sdist, wheels |
| ruby | rubygems | `publish-ruby` (migrated, below) | `release-rubygems` | `.gem` |
| rust | crates | owner-manual (4.2) | `release-crates` | `.crate` |
| typescript | npm | `publish-npm` | `release-npm` | `.tgz` |
| go | goproxy | none: the signed tag **is** the publish (3.4) | none | none |
| java, kotlin | maven | `publish-maven` | `release-maven` | unsigned bundle |
| csharp, fsharp | nuget | `publish-nuget` | `release-nuget` | `.nupkg` |
| elixir | hex | owner-manual (4.2) | `release-hex` | hex tarball |
| lua | luarocks | owner-manual (4.2) | `release-luarocks` | `.src.rock` |
| perl | cpan | owner-manual (or dedicated PAUSE account) | `release-cpan` | dist tarball |
| haskell | hackage | `publish-hackage` (candidate upload only; the owner publishes the candidate) | `release-hackage` | `sdist` tarball |
| dart | pubdev | owner-manual unless an upload-only route is verified | `release-pubdev` | package archive |
| ocaml | opam | `prepare-opam` (opens a PR) | `release-opam` | opam file, source archive |
| swift | none until O3 | none | none | none |
| c, cpp | github-release | `publish-source-release` | `release-source` | source tarball |

Every environment: the owner as required reviewer, administrator bypass off,
deployment restricted to that lane's tag pattern(s). Every trusted publisher is
pinned to repository, workflow `publish.yml` **and its own environment**.
`.github/CODEOWNERS` lists `publish.yml` and the gate scripts (VER31
`:627-643`). Today the existing environments are `pypi` and `rubygems`
(`publish.yml:175, 252, 294`); they are renamed `release-pypi` and
`release-rubygems`.

**The existing Python and Ruby jobs are migrated.** Today they build in the job
that publishes: `python -m build` (`publish.yml:192-195`), maturin
(`publish.yml:261-266`) and `gem build *.gemspec` (`publish.yml:306-311`),
which evaluates the gemspec. Under this document those builds move into `gate`,
and the publish jobs only download, verify and upload (VER31 `:433-444`
already says this for Python and Ruby).

### 3.4 Go: the tag push is the publish

There is no upload, no release event and **no environment approval**: once a
tag `code/packages/go/<dir>/vX.Y.Z` exists in a public repository,
`proxy.golang.org` serves it on first request and caches it permanently
(verify before implementing; a version cannot be removed, only retracted by a
later `retract` directive). `release-*` environments therefore do not protect
Go. The controls are all **before and at the tag**:

1. **Pre-tag gate, mandatory for every Go release, enforced by a helper.** A
   green `workflow_dispatch` run of `gate` plus the Go dry run (section 5.3) on
   the exact commit must exist **before** the tag is created. The owner creates
   Go tags only through the owner-run helper `code/scripts/go_release_tag.sh`.
   The **order of its steps is normative**; every check is bound to the one
   resolved SHA, and any failure exits non-zero having created nothing:

   1. `git fetch --no-tags origin main` (so ancestry is checked against a fresh
      `origin/main`, not a stale local copy).
   2. Resolve the commit **once**: `SHA=$(git rev-parse --verify
      "<rev>^{commit}")`. From here on, only `$SHA` is used, never the rev.
   3. Validate the tag name against the Go pattern of section 3.2 (passed as an
      argument, never interpolated into a shell string).
   4. Refuse if the tag already exists **locally or on the remote**
      (`git rev-parse -q --verify refs/tags/<name>`; `git ls-remote --tags
      origin refs/tags/<name>`).
   5. `git merge-base --is-ancestor "$SHA" refs/remotes/origin/main`.
   6. Read `go.mod` **from the object**: `git show "$SHA:<dir>/go.mod"`, never the
      working tree; refuse on any `replace` directive and on any `v0.0.0`
      requirement on a sibling (B1, B7).
   7. Require `HEAD == $SHA` **and** `git status --porcelain` empty (so what the
      owner sees locally is what is tagged); alternatively drop both and verify
      only against the object, but never mix the two.
   8. `gh run view <run>` (fields read as JSON): `headSha == $SHA`,
      `conclusion == success`, the event is `workflow_dispatch`, the workflow
      is `publish.yml`, its **inputs name this package, lane and ref**, and the
      **workflow file blob at that run equals the blob on current
      `origin/main`** (`git rev-parse "<run sha>:.github/workflows/publish.yml"`
      against `origin/main`), so an older, weaker gate cannot pass (L6).
   9. `git tag -s <name> "$SHA"`; `git push origin refs/tags/<name>`; then
      re-read the remote tag (`git ls-remote origin refs/tags/<name>^{}`) and
      refuse to report success unless its target equals `$SHA`. If the push
      fails, delete the local tag (`git tag -d <name>`) before exiting.

   Falsification tests (each must make the helper refuse and create nothing):
   `HEAD != $SHA`; a gate run for a different package or lane; a gate run with
   a different `headSha`; a red or cancelled run; a gate run of an older
   `publish.yml` blob; the tag already exists locally; the tag already exists
   on the remote only; a stale `origin/main` (a local ref that contains the SHA
   while the fetched one does not); `go.mod` with a `replace` directive **at
   `$SHA`** while the working-tree copy is clean; a `v0.0.0` sibling; a dirty
   tree; a non-`main` commit; a remote tag whose target differs after the push.
   The helper is a convenience that makes the correct path the easy one; the
   ruleset below remains the enforcement against anyone who does not use it.
2. The pre-tag gate keeps the `go.mod` checks: no `replace` directive and no
   `v0.0.0` requirement on a sibling (B1, B7).
3. **Ruleset:** creation, update and deletion of `refs/tags/code/packages/go/**`
   are restricted to the owner, and tags must be signed (VER31 `:651-655`
   extended). No other bypass actor.
4. **The observer runs from the default branch, never from the tag's own
   workflow file.** A tag on a commit that deletes an observer defined in that
   commit's `publish.yml` would neuter it. So the observer is a **scheduled
   workflow on `main`** (`on: schedule`, `contents: read` only) that
   enumerates every `refs/tags/code/packages/go/**` tag and, for each, checks
   that its commit is an ancestor of `main` and that a green gate run exists for
   that SHA with matching package and lane inputs. A Go tag with no match fails
   the run loudly and notifies the owner. **It is detective only**: the Go proxy
   and checksum-database caches are irreversible (a version cannot be removed),
   so nothing the observer does can undo a bad tag. The real controls are the
   owner-only, signed-tag ruleset (item 3) and keeping the owner's token off
   agent hosts (VER31 `:566-590`).

---

## 4. Job shape per registry

### 4.1 Common shape and rules (every job)

```yaml
  publish-<registry>:
    needs: [parse-tag, gate, verify-tag, attest]
    if: >-
      github.event_name == 'push' &&
      needs.gate.result == 'success' &&
      needs.verify-tag.result == 'success' &&
      needs.attest.result == 'success' &&
      needs.parse-tag.outputs.registry == '<registry>'
    runs-on: ubuntu-latest
    environment: release-<registry>   # owner reviewer, no admin bypass
    permissions:                      # least privilege, per job
      contents: read
      id-token: write                 # only where the registry uses OIDC
    steps:
      - uses: actions/download-artifact@<full sha>   # the gated bytes
      - name: Verify digest and attestation
        env: { ... }                                 # values via env only
        run: ...    # sha256 equals gate.json; gh attestation verify
                    # --signer-workflow .../publish.yml --source-ref <ref>
      - name: Upload   # one upload-only command, section 4.2
```

Rules (normative):

0. **The gate fails closed, in the workflow's structure.**
   (a) `publish.yml` triggers are exactly `push` on `tags:` with the allow-list
   patterns of section 3.2 and `workflow_dispatch`. Never `pull_request`,
   `pull_request_target`, `workflow_run` or `release`; the current `release:
   [published]` trigger (`publish.yml:45-47`) is replaced when the gate lands.
   (b) Every publish and `attest` job's `if:` requires
   `github.event_name == 'push'` and `needs.gate.result == 'success'`, plus
   `verify-tag` and (for publish jobs) `attest` success, and never contains
   `always()`, `!cancelled()` or `failure()`, which would let a job run after a
   failed or skipped dependency.
   (c) The `workflow_dispatch` path (dry run and the Go pre-tag gate) never
   reaches a publish or `attest` job, because of the `event_name` condition.
   (d) A validator test under `code/scripts/tests/`, specified like rule 2's,
   parses `publish.yml` and asserts all of (a)-(c): the trigger set; the
   conjunction in every publish and `attest` `if:`; the absence of the three
   status functions in any job-level `if:` (the validator exempts only
   non-publish cleanup **steps** that run no publish, upload or sign command,
   never a job-level `if:`); and that every publish job lists `gate` in `needs`. It is
   a floor check for any PR touching `.github/workflows/publish.yml`
   (backlog row LGE-PUB-validator).

1. **No build tool runs in a publish job, without exception.** A publish job
   runs only: download, digest check, `gh attestation verify` on the downloaded
   bytes (pinned to the signer workflow and source ref, VER31 `:601-607`), and
   one upload-only command from section 4.2. There is no "re-derive and
   compare" path: a tool that rebuilds (cargo, mix, luarocks, dart pub, npm
   with lifecycle scripts) evaluates package code while the registry credential
   is in scope. Where a registry offers no upload-only route, the lane is
   **owner-manual** (rule 8) or uses the registry's HTTP API.
2. `gate` has `permissions: contents: read` and nothing else: no `id-token`, no
   `attestations`, no environment, no secrets (VER31 `:419-426`). A validator
   test in `code/scripts/tests/` asserts that no publish, upload, sign or push
   verb appears in a `gate` step. Dry runs (section 5.3) never use an upload
   verb.
3. **No `actions/cache` restore** in `gate` or any publish job, so a poisoned
   cache cannot alter the bytes or the tools.
3a. **`gate` builds hermetically**: frozen lockfiles (`npm ci`, `cargo
   --locked`, `bundle` frozen, and the lane equivalents), `--ignore-scripts`
   wherever the tool has it, and a pinned or offline package index, so a
   dependency cannot change or run code between review and build. The `attest`
   job runs **no package code**: it downloads the artifact, checks its digest
   against `gate.json` and mints the attestation (VER31 `:419-432`).
4. `permissions:` is set on every job, `contents: read` by default
   (`publish.yml:53-54`), `id-token: write` only where the registry uses OIDC.
   `attest` is the only job with `attestations: write` (VER31 `:427-432`).
5. Secrets are **secrets of that registry's own environment**, never
   repository secrets (the fix `publish.yml:291-293` already asks for), passed
   by `env:` to the one step that needs them (as `GEM_HOST_API_KEY`,
   `publish.yml:318`), never written to disk or spliced into a script.
6. Every action is pinned by full commit SHA with the version in a comment
   (`publish.yml:71, 198`); this document does not invent SHAs.
7. Every job checks out `${{ github.sha }}`, never the tag (VER31 `:445-446`),
   with `persist-credentials: false` (`publish.yml:72-73`). The publish jobs
   need no checkout beyond the digest file.
8. **Owner-manual** means: the owner uploads the exact gated bytes from their
   own machine (digest equals `gate.json`), and the release notes say so
   (section 5.4 item 7). It is a first-class outcome, not a failure.
9. The job first checks whether the version already exists; if so it stops with
   a clear message (VER31 `:609-611`). Version and name used in that query are
   URL-encoded and come from the validated outputs of section 3.2.

### 4.2 Upload-only command per registry

All flags **verify before implementing**.

| Registry | Upload-only route in the publish job | Why / decision |
|---|---|---|
| npm | `npm publish <gated.tgz> --provenance --access public --ignore-scripts` | A tarball argument publishes those bytes; `--ignore-scripts` stops lifecycle scripts. Needs `id-token: write` |
| PyPI | `pypa/gh-action-pypi-publish` on the gated `dist/` (`publish.yml:197-200`, SHA-pinned) or `twine upload` of it | Uploads files; builds nothing. Attestations on |
| RubyGems | `gem push <gated .gem>` (gem built in `gate`) with trusted publishing | `gem push` of a built gem does not evaluate the gemspec in a way that matters here, because the gem is already built (verify before implementing) |
| crates.io | **Owner-manual** (decision). `cargo publish` always re-packages and builds from source, so it cannot be upload-only. The crates.io HTTP publish API accepts a prebuilt `.crate` but the job would have to implement the protocol (verify before implementing); that custom client is less reviewed than a one-time hand upload. The owner uploads the gated `.crate` by hand, and the first version is manual anyway (VER31 `:596`). Revisit when a pinned upload-only tool or action exists | Avoids a build tool with the token in scope |
| Maven Central | Upload the gated, already-signed bundle (signing: below) to the Central Publisher Portal through its HTTP upload, with the user token | No Gradle or Maven in the job |
| NuGet | `dotnet nuget push <gated.nupkg>` with trusted publishing (OIDC) | A push of a built package; builds nothing |
| Hex | Owner-manual (decision). `mix hex.publish` evaluates `mix.exs`. Hex has a publish HTTP API for a prebuilt tarball (verify before implementing); adopt it only with a reviewed client | Same reasoning as crates.io |
| LuaRocks | Owner-manual (decision). `luarocks upload` of a rockspec evaluates it. Uploading a prebuilt `.src.rock` through the site's API without evaluating the rockspec is unverified | Same reasoning |
| CPAN | **Owner-manual**, or a dedicated PAUSE account whose credentials live only in `release-cpan` and which owns only this distribution | PAUSE credentials are broad and long-lived (verify before implementing) |
| Hackage | `curl` upload of the gated sdist with a token via `env:` (the upload endpoint takes a tarball; verify before implementing). Candidate-only: the job never publishes; the owner publishes the candidate manually. Verify before implementing that the token can be limited to candidate uploads; if it cannot, Hackage becomes owner-manual | No `cabal` in the job |
| pub.dev | **Owner-manual** unless an upload-only route is verified: `dart pub publish` loads and evaluates the pubspec and package, so it is a build tool for this rule | OIDC automated publishing runs `dart pub publish` (verify before implementing) |
| opam | `prepare-opam`: push a branch to the machine-account fork and open the PR with `gh pr create`; maintainers merge | Section 4.3 |
| C, C++ | `gh release upload` of the gated tarball, `SHA256SUMS` and signature | Upload only |
| Swift | none until O3 | B6 |

**Maven signing (separate from the build).** `gate` builds **unsigned**
artifacts and holds no secret. In `publish-maven` the job first downloads the
bytes and verifies their digest and attestation; only then does it sign, and
the only command it runs over the artifacts is `gpg --detach-sign --armor` (no
Gradle, Maven or other build tool is in scope). It then uploads the files and
their signatures. `GNUPGHOME` is a fresh directory created under `umask 077`.
The key is a **signing-only subkey** (the primary key stays offline with the
owner), imported from **stdin**, never from a file in the workspace. Signing
uses `--pinentry-mode loopback --passphrase-fd 0` with the passphrase fed from
an `env:` variable, so it never appears in arguments. The directory is deleted
in a cleanup step (a cleanup *step* may use `if: always()` provided it runs no publish,
upload or sign command; this is the only exemption, and it never applies to a
job-level `if:`).
The subkey and passphrase are `release-maven` secrets.

**Hand-published first versions.** Where VER31 requires the first version to be
published by hand (crates.io, npm: VER31 `:596-597`), the owner uploads the
**exact gated bytes** (digest equals `gate.json`; never a local rebuild), the
release notes state that **no provenance attestation exists for that version**,
the personal token or 2FA session used is revoked immediately after, and the
trusted publisher is registered before the next version.

### 4.3 Other registry notes

- **crates.io.** Trusted publishing is configured for later releases; until an
  upload-only route is verified every crates.io upload is owner-manual, which
  also covers the first version.
- **RubyGems and NuGet: trusted publishing is required.** An API key is only a
  time-boxed documented fallback with an expiry date and a scope limited to the
  one package, recorded in the release notes.
- **NuGet package metadata** already sets `PackageLicenseExpression` to MIT
  (`csharp/logic-gates/CodingAdventures.LogicGates.csproj:11`,
  `fsharp/logic-gates/CodingAdventures.LogicGates.fsproj:9`).
- **Hex.** `mix.exs` shows only `app` and `version` at `mix.exs:6-7`, with no
  package metadata in the part read; add it before the first upload.
- **CPAN.** `Makefile.PL` already sets `LICENSE => 'mit'` (`Makefile.PL:10`).
- **opam.** `prepare-opam` builds the source archive and opam file with its
  checksum in `gate`; the PR is opened from a **dedicated machine-account
  fork** (not the owner's account) with a fine-grained credential limited to
  that fork, stored in `release-opam`. The job reports "opened", never
  "published". The opam file pins exact compiler and tool versions
  (`ocaml/logic-gates/coding-adventures-logic-gates.opam:12-18`); maintainers
  may ask for ranges.
- **Swift.** Not a job until O3 is decided. If a mirror repository is chosen,
  its push credential is a fine-grained token limited to the mirror, kept in
  its own environment, and the mirror has its own tag protection (signed tags,
  creation restricted).
- **C and C++.** A GitHub Release asset built by `git archive` from the tagged
  commit with a fixed prefix and modification time so the tarball is
  reproducible, a `SHA256SUMS` file, a signature over it (method O4) and a
  build-provenance attestation (VER31 `:612-615`). vcpkg and Conan are later
  work, each a pull request to its registry.
- **Owner-manual registries (CPAN, Hex, LuaRocks) and the Hackage candidate token.** Tokens are
  **package-scoped** where the registry supports it (verify before
  implementing, per registry), are rotated after every release, and are never
  stored in the repository or an environment unless an automated route is
  adopted. The one automated exception is Hackage: its token lives in
  `release-hackage`, is limited to **candidate** uploads, and the owner
  publishes the candidate (section 4.2).
- **Maintainer identity.** POM developer data, registry profiles and package
  metadata use a noreply or role address, never a personal email.

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
- **Names are reserved before anything is built** (backlog row
  LGE-OWNER-NAMES, ordered before every LGE-PUB row): the whole dependency
  closure's names, not only `logic-gates`, are claimed on every registry
  (section 1.3), so no dependency can be registered by someone else between the
  decision to publish and the release.
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
| No long-lived token where OIDC exists | npm and PyPI use OIDC (crates.io and pub.dev have OIDC but are owner-manual until an upload-only route exists, section 4.2); RubyGems and NuGet are required to (verify availability); keys only as a time-boxed fallback with expiry and one-package scope. Maven and Hackage need a token (section 1.2); Hex, LuaRocks and CPAN tokens are owner-held and never stored in an environment (section 4.3), scoped to one package where the registry allows and rotated by the owner. Only the Hackage candidate token and the Maven secrets live in environments (`release-hackage`, `release-maven`) |
| Environment protection | One environment per registry (section 3.3), the owner as required reviewer, administrator bypass off, tag-pattern deployment restriction (VER31 `:539-545` amended). The owner approves only in the web UI (VER31 `:576-577`) |
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
   registry's environment deployment is approved in the web UI only (for Go,
   the pre-tag gate of section 3.4 is run first, and the tag is the publish) (VER31 `:500-521,
   576-577`).

7. For a hand-published first version: the owner uploads the exact gated bytes,
   the release notes say no provenance exists for it, the personal token is
   revoked immediately, and the trusted publisher is registered.

No agent performs steps 5 or 6, and an approval seen anywhere other than the
owner's chat message does not count.

### 5.5 Residual risks until this is implemented

- **Today's workflow (interim).** `publish.yml` publishes PyPI and RubyGems now:
  it builds in the publishing job (`publish.yml:192-195, 306-311`), pushes
  RubyGems with the **repository-level** secret `RUBYGEMS_API_KEY`
  (`publish.yml:318`), runs on `release: [published]` (`publish.yml:45-47`), and
  no tag ruleset exists (VER31 `:651-655` specifies one; B9). Recommended **owner
  actions now**, ahead of this spec: scope the RubyGems key to the existing gems
  and rotate it; move it into the `rubygems` environment (`publish.yml:291-293`);
  create the tag ruleset on `refs/tags/python/**` and `refs/tags/ruby/**`.
- **Sole reviewer.** Every `release-*` environment has one required reviewer,
  the owner, so the approval is only as strong as that account. The owner
  account requires **hardware-key two-factor authentication**, and a **second
  required reviewer is added as soon as one exists**. Until then the release
  notes state the residual risk, as VER31 does for the agent-token case
  (VER31 `:580-585`).

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
| VER31 section 9.2 | **One environment per registry** replaces the single `release` environment (section 3.3 here); each trusted publisher pins its own environment |
| VER31 section 9.6 | Tag ruleset covers every lane and `code/packages/go/**`; CODEOWNERS also lists the publish workflow and gate scripts |
| VER31 section 7 | Publish-job rules of section 4.1 here are normative: no build tool, no cache, no exception |
| VER31 section 9.3 | The table gains the rows of section 1.1 here |
| `.github/workflows/publish.yml` | Not changed by this document; changes ship with each LGE-PUB row, one registry per PR, after LV-007a |

---

## 8. Backlog

See `VER-BACKLOG.d/logic-gates-everywhere.md`: rows `LGE-S5` (this document),
`LGE-PUB-<registry>` (one per registry), `LGE-OWNER-NAMES` (before all of them), `LGE-OWNER-ACCOUNTS`, `LGE-PUB-validator` and `LGE-PUB-go-helper`.
