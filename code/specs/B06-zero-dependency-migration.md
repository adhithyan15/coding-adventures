# B06 — Zero-Dependency Migration

## Overview

This spec describes how the monorepo gets to a state where every package
builds, tests, lints and runs using only the standard library of the language
it is written in. No crates.io, no npm registry, no PyPI, no RubyGems, no Hex.

The motivation is concrete rather than aesthetic. At commit `d2610543e6` this
repository carried roughly 600 open Dependabot alerts. Not one of them was in
code we ship. Every single one was in a test runner, a coverage reporter, or
the bundler a test runner drags in behind it. We were absorbing a continuous
stream of security work as rent on tools that exist to check code we wrote
ourselves — and the repo's whole premise is that we write things ourselves.

B03 (Build Vendoring) answered a neighbouring question: how do you make builds
reproducible when you have dependencies? This spec asks the harder one: what
would it take not to have them.

A warning about scope before we start. This is a large migration, it is not
uniformly achievable, and the honest answer for one tier of dependencies is
"you probably shouldn't." The spec is organised so the parts that are cheap
and total come first, and the parts that are expensive and partial come last,
with an explicit recommendation at each boundary.

## What counts as a dependency

"Zero dependencies" is ambiguous in a way that matters, because three quite
different things wear the name and only one of them is what generates alerts.

**Shipped dependencies** appear in `dependencies`, `[dependencies]`, or
`add_runtime_dependency`. They end up in the dependency tree of anybody who
installs our package. These are the ones with real blast radius: a compromised
shipped dependency compromises our users.

**Development dependencies** appear in `devDependencies`,
`[dev-dependencies]`, `[dependency-groups]`, or `add_development_dependency`.
They are not shipped. They are still declared in a manifest, still resolved
into a lockfile, still fetched by CI, still execute with full permissions over
our source tree, and still generate Dependabot alerts. Their blast radius is
our build machines and our CI credentials, which is smaller than our users but
is not nothing — a malicious test runner is a very good place to exfiltrate a
token from.

**Tool dependencies** are binaries CI installs and then invokes, with no
manifest entry anywhere: `cargo install cargo-tarpaulin`, `pip install ruff`.
They are invisible to Dependabot precisely because nothing declares them. That
makes them *quieter*, not safer. They still read every file we own.

One nuance specific to Ruby, because it decides whether a large chunk of work
is necessary at all. Ruby ships two classes of pre-installed gem. *Default
gems* are part of the standard library and cannot be uninstalled. *Bundled
gems* are installed alongside the interpreter but are ordinary, removable,
independently-versioned gems. `minitest` and `rake` are bundled, not default:

    $ ruby -e 'puts Gem::Specification.find_by_name("minitest").default_gem?'
    false

So "it comes with Ruby" is not the same as "it is the standard library", and
minitest does not get a free pass. `Coverage`, by contrast, genuinely is
stdlib — which turns out to matter a lot below.

## The measured inventory

Everything in this section was measured at `d2610543e6` by walking every
manifest in the repo, not estimated. Counts are *manifest edges*: how many
packages declare that dependency. The repo has 511 `package.json`, 1437
`Cargo.toml`, 532 Python project files, 287 `Gemfile`, 317 `go.mod` and 294
`mix.exs`, so a dependency named by 480 packages is named by nearly every
package of its language.

### npm — 28 distinct third-party packages

| Count | Package | Role |
|---:|---|---|
| 494 | `typescript` | compiler / type checker |
| 482 | `vitest` | test runner |
| 472 | `@vitest/coverage-v8` | coverage |
| 155 | `@types/node` | type stubs |
| 30 | `jsdom` | DOM for tests |
| 29 | `vite` | dev server / bundler |
| 25/24/24 | `@types/react`, `react`, `react-dom` | UI framework |
| 24 | `@types/react-dom` | type stubs |
| 22 | `@vitejs/plugin-react` | build glue |
| 19 | `tsx` | TS execution |
| 18/18 | `@testing-library/react`, `@testing-library/jest-dom` | DOM assertions |
| 6 | `ts-node` | TS execution |
| 5 | `electron` | desktop shell |
| 5/5/5 | `jest`, `ts-jest`, `@types/jest` | second test runner |
| 4 | `electron-builder` | desktop packaging |
| 4 | `cross-env` | env var shim |
| 3 | `esbuild` | bundler |
| 2 | `@playwright/test` | browser automation |
| 1 each | `fake-indexeddb`, `pngjs`, `@types/pngjs`, `vscode-languageclient`, `@types/vscode` | misc |

### Python — 4 distinct third-party packages, all dev

| Count | Package | Role |
|---:|---|---|
| 499 | `pytest` | test runner |
| 477 | `pytest-cov` | coverage |
| 475 | `ruff` | linter |
| 373 | `mypy` | type checker |

**Runtime third-party dependencies: zero.** Every non-dev dependency in every
Python package is a sibling `coding-adventures-*` package.

### Ruby — 4 distinct third-party gems

| Count (gemspec) | Gem | Role |
|---:|---|---|
| 280 | `rake` | task runner |
| 277 | `minitest` | test framework |
| 166 | `simplecov` | coverage |
| 67 | `standard` | linter |

**Runtime third-party dependencies: zero**, same as Python.

### Rust — ~30 distinct crates

| Count | Crate | Role |
|---:|---|---|
| 191 | `serde_json` | JSON |
| 90 | `serde` | serialization framework |
| 73 | `wasm-bindgen` | WASM ↔ JS glue |
| 29 | `tempfile` | temp files in tests |
| 25 | `toml` | TOML |
| 10 | `js-sys` | JS bindings |
| 9 | `regex` | regular expressions |
| 7 | `libc` | libc declarations |
| 6 / 4 | `hex`, `base64` | encodings |
| 5 / 2 | `getrandom`, `rand` | randomness |
| 4 each | `tungstenite`, `thiserror`, `serialport`, `rusqlite` | net / errors / hardware / DB |
| 2 each | `windows`, `chrono`, `clap`, `cc`, `bitflags`, `subtle`, `unicode-general-category` | platform / misc |
| 1 each | `wgpu`, `uefi`, `zstd`, `zeroize`, `webpki-roots`, `unicode-bidi`, `wasm-bindgen-test` | specialised |

### Go — effectively already there

Only `golang.org/x/sys v0.43.0` and `golang.org/x/text v0.41.0`. Every other
`require` is a local module. Go is the existence proof that this repo's own
code does not need a package ecosystem.

### Elixir — 2 distinct

`jason` (JSON) and `excoveralls` (coverage).

## The central finding

Across all 490 original npm lockfiles, we classified every node matching a
package named in an open advisory:

| Classification | Nodes |
|---|---:|
| `dev: true` | 2568 |
| runtime | 3 |

The three runtime nodes are `esbuild` at 0.28.1 and 0.28.2, which are *not*
vulnerable (the advisory covers `>=0.27.3 <0.28.1`).

**Every open alert in this repository is in a development dependency tree.**
The shipped surface is already clean. Combined with Python and Ruby having no
runtime third-party dependencies at all, and Go having almost none, the
picture is unambiguous: the repo does not have a dependency problem in what it
publishes. It has one in how it tests what it publishes.

That reorders the entire migration. The instinct is to start with the scary
shipped stuff, but the shipped stuff is already fine. Starting with test
tooling is both the cheapest work and the work that takes the alert count to
zero.

### Which declared dependency is actually responsible

"Test tooling" is still too coarse. Walking each lockfile's graph backwards
from every vulnerable node to the dependency the package actually *declares*
narrows it much further:

| Declared dependency | Alert nodes | Share | Manifests |
|---|---:|---:|---:|
| `vitest` | 1895 | 73.1% | 470 |
| `@vitest/coverage-v8` | 490 | 18.9% | 461 |
| `vite` | 114 | 4.4% | 28 |
| `electron-builder` | 32 | 1.2% | 4 |
| `jsdom` | 26 | 1.0% | 26 |
| `tsx` | 12 | 0.5% | 12 |
| `jest` + `ts-jest` | 20 | 0.8% | 5 |
| `esbuild` | 2 | 0.1% | 2 |

**92% is one pair.** `nanoid`, `postcss`, `brace-expansion`, `js-yaml` and
`form-data` — the packages the advisories are actually *against* — appear in
no manifest in this repository. They arrive underneath `vitest`.

The surface this represents is easier to see in a single package.
`document-html-sanitizer` declares exactly two dependencies, `vitest` and
`@vitest/coverage-v8`, and installs **94 packages**. All 94 root at vitest.
Roughly 450 packages each carrying ~90 third-party packages to run their
tests is why an advisory lands every couple of weeks: it is arithmetic, not
bad luck.

TF00 specs the replacement in detail.

## The three tiers

**Tier 1 — Tooling.** Test runners, coverage reporters, linters, type
checkers, bundlers, task runners. Every language in the repo has a stdlib or
toolchain-provided answer for most of this, and where it doesn't, the gap is
narrow and nameable. This tier contains 100% of the alerts.

**Tier 2 — Libraries we could plausibly write.** `serde_json`, `toml`,
`regex`, `hex`, `base64`, `tempfile`, `thiserror`, `jason`. The repo already
contains hand-written JSON parsers, TOML lexers and lexer/parser toolkits in
six languages. Writing these is not a detour from the repo's purpose; it *is*
the repo's purpose.

**Tier 3 — Irreducible platform bindings.** `wasm-bindgen`, `electron`,
`rusqlite`, `serialport`, `wgpu`, `windows`, `uefi`, `react`, and `typescript`
itself. These either bind to a system the standard library does not reach, or
they *are* the platform. Some can be replaced at real cost; some should not
be.

## Tier 1: tooling

### TypeScript and Node

This is the single highest-value change in the migration, because it removes
`vitest` (482), `@vitest/coverage-v8` (472), `tsx` (19), `ts-node` (6), `jest`
+ `ts-jest` + `@types/jest` (15) and the `vite` (29) that `vitest` pulls in —
and with them every advisory in the repo.

Node 22, which this repo already runs, has a stable test runner, a built-in
assertion library, TypeScript type-stripping, and coverage with enforceable
thresholds. All four, together, with no packages installed:

```ts
// sum.ts
export function add(a: number, b: number): number { return a + b; }

// sum.test.ts
import { test } from "node:test";
import assert from "node:assert/strict";
import { add } from "./sum.ts";
test("adds", () => { assert.equal(add(2, 3), 5); });
```

```
$ node --experimental-strip-types --test --experimental-test-coverage \
       --test-coverage-lines=80 sum.test.ts
ok 1 - adds
# start of coverage report
# file        | line % | branch % | funcs % | uncovered lines
# sum.test.ts | 100.00 |   100.00 |  100.00 |
# sum.ts      | 100.00 |   100.00 |   50.00 |
# all files   | 100.00 |   100.00 |   66.67 |
# end of coverage report
```

The coverage flags are a real gate, not a report. Verified:

    $ node --experimental-strip-types --test --experimental-test-coverage \
           --test-coverage-functions=90 sum.test.ts; echo $?
    1

That satisfies CLAUDE.md rule 11 (>80% coverage) with zero dependencies, which
is the property that makes this tier tractable at all.

**What the migration costs.** Three things are genuinely lost.

1. *Type checking.* `--experimental-strip-types` erases annotations; it does
   not check them. Dropping `typescript` (494 packages) does not mean "TS
   without the dependency", it means no type checking and no `.d.ts` emission.
   This is a Tier 3 decision, discussed below, and it is the reason
   `typescript` is not in the Tier 1 removal list despite being the most
   common dependency in the repo.
2. *The `expect` API.* `node:assert/strict` covers equality, deep equality,
   throws and rejects. It has no `expect(x).toMatchSnapshot()`,
   `toHaveBeenCalledWith`, or `vi.mock`. Node has `mock` in `node:test`
   (`mock.fn`, `mock.method`, `mock.timers`) which covers most spying, but
   module mocking is `mock.module` and still experimental. Snapshot testing
   exists as `t.assert.snapshot`. Test suites that lean on Vitest's matcher
   vocabulary need mechanical rewriting — the count is large but the edit is
   local and scriptable.
3. *DOM.* `jsdom` (30) and `@testing-library/*` (36) have no stdlib answer.
   See "the browser problem" below.

**Recommended sequencing within Node:** convert the ~450 packages that have no
DOM dependency first. They are the bulk, the conversion is mechanical
(`describe`/`it` → `describe`/`test`, `expect(a).toBe(b)` →
`assert.equal(a, b)`), and finishing them alone retires every alert.

### The browser problem

Thirty packages test against `jsdom`, eighteen use `@testing-library/react`,
two use Playwright, twenty-four render React. There is no standard library
that contains a DOM.

Two routes, and they are not exclusive:

*Drive a real browser over CDP.* Node 22 ships a global `WebSocket` and a
global `fetch` (both verified present in this environment). The Chrome
DevTools Protocol is a JSON-over-WebSocket protocol. A headless Chromium
launched with `--remote-debugging-port` can be driven from `node:child_process`
plus those two globals with no packages at all. This replaces Playwright *and*
jsdom, and it tests against a real DOM rather than a simulation of one — which
is strictly better fidelity. The cost is writing the CDP client, which is a
few hundred lines for the subset we need (navigate, evaluate, query, click,
screenshot), plus the browser binary becoming a tool dependency.

*Migrate off React onto the repo's own runtime.* The repo already contains
`mosaic`, a compiler and web-component runtime with its own
`mosaic-flux-webcomponent` package. The 24 React packages are the natural
users of it. This is the philosophically consistent answer and it deletes
`react`, `react-dom`, `@types/react`, `@types/react-dom`,
`@vitejs/plugin-react` and most of the `@testing-library` surface in one
move — but it is an application rewrite, not a tooling swap, and belongs late
in the sequence.

### Python

`pytest` (499) → `unittest`. The mechanical shape of the migration is bare
functions and `assert` becoming `unittest.TestCase` methods and
`self.assertEqual`. The real losses are fixtures and parametrisation:
`@pytest.fixture` becomes `setUp`/`tearDown` or a context manager, and
`@pytest.mark.parametrize` becomes `subTest`:

```python
for a, b, want in cases:
    with self.subTest(a=a, b=b):
        self.assertEqual(add(a, b), want)
```

`subTest` reports each case separately on failure, so this is a genuine
equivalent rather than a downgrade.

`pytest-cov` (477) → the stdlib `trace` module, which does line counting and
can emit per-file coverage. `trace` is slow because it drives `sys.settrace`
on every line; on Python 3.12+ `sys.monitoring` is dramatically cheaper and is
the right substrate for a small in-repo coverage gate. This repo currently
runs Python 3.11, so the upgrade is a prerequisite for the fast path — on 3.11
`trace` works but will hurt on the larger suites.

`ruff` (475) and `mypy` (373) **have no standard library equivalent.** Python
ships `ast` and `tokenize`, which are enough to write a linter against, and
the repo has ample precedent for writing parsers — but a type checker is a
different order of undertaking. This is a decision, not a migration, and it is
called out explicitly in "Decisions required" below.

### Ruby

`simplecov` (166) → `Coverage`, which is genuinely standard library:

```ruby
require "coverage"
Coverage.start(lines: true)
# ... load and run tests ...
result = Coverage.result   # { "path/to/file.rb" => [1, 0, nil, 3, ...] }
```

`nil` means non-executable line, `0` means executable-but-never-run. A
threshold gate over that hash is perhaps thirty lines. This is the cleanest
single substitution in the whole migration.

`minitest` (277) is a bundled gem, so it must go to reach zero. Ruby's stdlib
has no assertion framework — `test/unit` is itself a bundled gem wrapping
minitest. The replacement is a small in-repo harness: a base class, a handful
of assertion methods, a runner that discovers `test_*` methods by reflection,
and TAP or plain output. This is genuinely ~150 lines and the repo already
builds far more intricate things.

`rake` (280) → the repo already has a build tool: the Go implementation at
`code/programs/go/build-tool/`, which CLAUDE.md names as the primary build
tool. Rake's role in these gemspecs is largely vestigial; deleting the
dependency is mostly bookkeeping.

`standard` (67) has no stdlib equivalent — same decision class as `ruff`.

### Rust

Rust is better positioned than it looks. `cargo test` is part of the
toolchain, so the 1437 `Cargo.toml` files need no test-framework dependency
today and don't grow one.

Coverage is currently `cargo tarpaulin`, a tool dependency. The
toolchain-provided replacement is source-based coverage:

    RUSTFLAGS="-C instrument-coverage" cargo test
    llvm-profdata merge -sparse *.profraw -o out.profdata
    llvm-cov report --instr-profile=out.profdata <binary>

`llvm-profdata` and `llvm-cov` come from the `llvm-tools` rustup component —
first-party Rust toolchain, not crates.io. This removes tarpaulin without
adding anything.

`tempfile` (29) is Tier 1 in spirit (it is a test convenience) and trivially
replaceable: `std::env::temp_dir()`, a directory named from a counter plus
`std::process::id()`, and a `Drop` impl that removes it. Roughly forty lines,
once, in a shared internal crate.

### Go and Elixir

Go needs nothing: `go test` and `go test -cover` are built in, and the two
`golang.org/x/*` modules are not test tooling.

Elixir's ExUnit is part of the language distribution. `excoveralls` is
replaceable by OTP's built-in `:cover` module, which is the Erlang equivalent
of Ruby's `Coverage`.

## Tier 2: libraries we could write

| Crate/Gem | Count | Stdlib path | Precedent already in repo |
|---|---:|---|---|
| `serde_json` | 191 | hand-written parser + emitter | `json-parser`, `json-lexer`, `json-value` in Go/TS/Python/Ruby |
| `serde` | 90 | manual `to_json`/`from_json` impls | same |
| `toml` | 25 | hand-written parser | `toml-lexer` packages exist |
| `regex` | 9 | NFA/DFA engine | the repo writes lexers for ~20 languages |
| `hex`, `base64` | 10 | ~30 lines each | implemented in other languages already |
| `thiserror` | 4 | `impl Display + Error` by hand | — |
| `jason` (Elixir) | 2 | hand-written parser | same JSON work |
| `js-yaml` (npm, transitive) | — | drops out with the bundlers | — |

`serde_json` at 191 packages is the largest single line item in the entire
migration and deserves its own design note. Replacing the *parser* is easy and
the repo has done it five times in other languages. Replacing *`serde`* is
not: the value of serde is `#[derive(Serialize, Deserialize)]`, and derive
macros are normally written with `syn` and `quote`, which are themselves
crates.io dependencies. A zero-dependency derive macro must parse the token
stream using only `proc_macro`, which is compiler-provided rather than
crates.io — possible, unpleasant, and a real project.

The pragmatic alternative is to skip derives entirely and hand-write
`to_json`/`from_json` per type against a small in-repo `ca-json` crate. More
lines, no macro machinery, and considerably more readable — which for this
repo's stated literate-programming goal is arguably the point.

## Tier 3: the irreducible set

These need decisions, not plans.

**`typescript` (494).** TypeScript is not a library we depend on, it is the
language half the repo is written in. Node can *run* TypeScript via type
stripping but cannot *check* it. Three options: (a) accept `tsc` as a
toolchain dependency the way we accept `rustc` and `go` — it is first-party to
the language and has a different risk profile from a random npm package; (b)
drop to plain JavaScript with JSDoc annotations, which still needs `tsc` to
check; (c) give up static typing in TS packages. **Recommendation: (a).**
Reclassify `tsc` as a toolchain, pin it, and stop counting it. The same logic
that makes `rustc` acceptable makes `tsc` acceptable.

**`wasm-bindgen` (73) + `js-sys` (10).** The largest Tier 3 item. Hand-written
`#[no_mangle] extern "C"` exports plus manual JS glue is viable — the repo
already has `wasm-module-encoder`, `wasm-leb128` and `wasm-validator` packages
and understands the format — but 73 packages of glue is a very large body of
work for a dependency that runs at build time and emits code we can read.
**Recommendation: defer indefinitely**; revisit only if a wasm-bindgen
advisory ever actually lands.

**`electron` (5) + `electron-builder` (4).** No stdlib path exists. Either
drop the desktop shells and ship those four apps as web pages, or accept the
dependency. **Recommendation: ship as web apps**, which also deletes the
`js-yaml` advisory chain that electron-builder drags in.

**`react` + `react-dom` (24).** Migrate to the repo's own `mosaic`
web-component runtime. Consistent with the repo's purpose, but an application
rewrite.

**`rusqlite` (4), `serialport` (4), `wgpu` (1), `windows` (2), `uefi` (1),
`tungstenite` (4), `webpki-roots` (1), `zstd` (1).** Each binds to a system
library or a protocol stack. `extern "C"` declarations can replace `libc` and
the thinner bindings; `wgpu` and `rusqlite` cannot reasonably be hand-rolled.
**Recommendation: accept these**, and record them in an explicit allowlist so
they are a deliberate exception rather than drift.

## Sequencing

**Phase 0 — Freeze.** Before removing anything, stop the set from growing. Add
a contract test to the existing "Repo-wide metadata contracts" CI job that
parses every manifest, collects third-party dependency names, and fails if any
name is outside an allowlist. Seed the allowlist with exactly today's set. The
allowlist may only shrink. This is the single most important step in the
document, because it is what makes every later phase monotone instead of a
treadmill.

**Phase 1 — Node test tooling.** `vitest` → `node:test` for the ~450 non-DOM
packages. *Takes the alert count to zero.* Everything after this point is
purpose, not security.

**Phase 2 — Python, Ruby, Elixir test tooling.** `pytest` → `unittest`;
`pytest-cov` → `trace`/`sys.monitoring`; `simplecov` → `Coverage`;
`minitest` → in-repo harness; `rake` → build-tool; `excoveralls` →
`:cover`; `cargo-tarpaulin` → `-C instrument-coverage`.

**Phase 3 — Tier 2 Rust libraries.** `tempfile`, `hex`, `base64`, `thiserror`
first (small, total), then `toml`, then `serde_json`/`serde` as its own
project.

**Phase 4 — Browser and UI.** CDP client to replace `jsdom` and Playwright;
React → mosaic.

**Phase 5 — Tier 3 decisions.** Record the accepted set; revisit nothing
unless an advisory forces it.

Phases 1 and 2 are independent of each other and of Phase 3, so they can run
concurrently across separate PRs. Phase 4 depends on Phase 1.

## Enforcement

The allowlist gate from Phase 0 is the whole enforcement story, and it should
be written once and applied to every ecosystem:

```
for each manifest in the repo:
    declared = third-party dependency names in that manifest
    unknown  = declared - ALLOWLIST
    if unknown: fail, naming the manifest and the dependency
```

Two properties make it work. It is *per-name*, not per-version, so it does not
churn when versions move. And it is *monotone by policy*: adding a name to the
allowlist is a reviewable diff that a human must justify, so the set cannot
grow by accident — which, per the lesson recorded about stale CI gates, is the
failure mode to design against.

Once an ecosystem reaches zero, its Dependabot entry should be deleted from
`.github/dependabot.yml` rather than left configured against nothing.

## Decisions required

These cannot be resolved from inside this spec.

1. **Linting and type checking.** `ruff`, `mypy`, `standard` and `tsc` have no
   standard library equivalents. CLAUDE.md rule 15 currently mandates all
   three linters. Either that rule relaxes, or these four are accepted as
   toolchain dependencies, or the repo writes its own. The recommendation
   above is to accept them as toolchain and exclude them from the definition
   of "dependency" — but that is a change to what the goal means, and it
   should be made explicitly.
2. **Python 3.11 → 3.12+**, to get `sys.monitoring` for coverage at a
   tolerable speed.
3. **Electron apps**: rewrite as web apps, or keep the dependency.
4. **React packages**: migrate to mosaic, or keep.

## What this buys, and what it does not

It buys the elimination of a recurring, unbounded maintenance stream — roughly
600 alerts at the time of writing, all of them in tooling, all of them
arriving without warning and needing triage. It removes the supply-chain
surface entirely for the ecosystems that reach zero. It makes builds hermetic
without needing the vendoring machinery of B03. And it is, straightforwardly,
what this repo is for: a monorepo that implements lexers, parsers, VMs, CPUs
and compilers from first principles has a weak excuse for not implementing its
own test harness.

It does not buy freedom from vulnerabilities. `node`, `rustc`, `cpython`,
`ruby` and `go` are themselves large C and C++ codebases with their own CVE
streams, and pinning a toolchain is the same problem one level down. It does
not reduce total work; it moves the work in-house, where a bug in our JSON
parser is ours to find, with no upstream to report it to and no advisory feed
to tell us it exists. Tier 2 in particular trades a *known* risk that someone
else patches for an *unknown* risk that nobody is looking for.

That trade is defensible for a repository whose purpose is learning by
building. It would not be defensible for most production systems, and this
spec should not be cited as though it were general advice.
