# Forme Completion Roadmap

> **Status:** Living delivery backlog, last prioritized 2026-08-28.
> This roadmap turns the north-star in [FM00](FM00-forme-vision.md) into
> merge-sized work. Update it whenever implementation work discovers a new
> gap, and reprioritize it after every merged Forme pull request.

## What “complete” means

Forme is complete when it is a usable product, not merely a collection of
well-tested packages. We will deliver that in three horizons:

1. **Headless v0:** a clean checkout can build, check, preview, and deploy the
   Coding Adventures blog and repository landing page with Forme. The output
   includes routed pages, indexes, feeds, sitemaps, assets, and a Style IR
   theme. Incremental and reproducible builds have measured correctness gates.
2. **Extensible v1:** third-party stages can be installed and run through the
   FM02 plugin protocol with explicit capabilities, trust decisions, runtime
   adapters, and operating-system sandboxing.
3. **Authoring v1:** a non-developer can create and publish a site through a
   live-preview authoring shell. Interactivity IR, at least one non-HTML
   backend, accessibility, performance, and security gates are part of the
   release contract.

The first horizon is the current release target. Work for later horizons stays
visible so a local optimization cannot quietly close the project early.

## Current baseline

The implementation is substantial but not yet an end-to-end product:

- 61 TypeScript `forme-*` packages and 180 package test files cover the kernel,
  stage contracts, a sequential orchestrator, Style IR, AOT emitters, document
  transforms, collections, feeds, routing, and static output.
- The blog proves a ten-stage routed DAG: source → parse → asset resolution →
  router fans out to article rendering, asset loading, and chronological
  collection. Typed page/asset fan-in writes fingerprinted local assets and
  manifest entries while the second filesystem sink emits an index, RSS, Atom,
  and sitemap. Public metadata and asset URLs compose portable routes with the
  GitHub Pages project prefix. A reusable light/dark Style IR theme drives exact
  per-article AOT CSS slices.
- There is no general `forme` build/check/dev CLI, watch server, plugin host,
  runtime sandbox, authoring shell, or implemented deploy runner.
- Interactivity IR has no numbered spec or package. The AOT implementation
  refers to a missing FM06 spec, and the existing FM05 deploy-runner spec
  collides with older FM01–FM04 references that use FM05 for Interactivity IR.
- The repository landing page is still hand-maintained HTML. Rebuilding it
  through Forme is the decisive dogfooding milestone from FM00.

## Prioritization method

At each loop boundary, score newly discovered and unfinished work in this
order:

1. **Unblocks the current release horizon.** Broken clean-checkout workflows
   and missing product-path integration outrank speculative features.
2. **Exercises Forme through real dogfood.** Prefer work proved by the blog or
   landing page over isolated package growth.
3. **Retires architectural risk.** Contract, correctness, security, and data
   loss risks outrank polish.
4. **Smallest independently verifiable slice.** Split work until one PR has a
   crisp acceptance test and can auto-merge safely.

Statuses are `done`, `active`, `ready`, `blocked`, and `later`. Only one item is
`active`. A blocked item names its dependency.

## Prioritized backlog

| Priority | ID | Status | Work item | Acceptance gate |
|---:|---|---|---|---|
| 0 | FM-B001 | done | Establish the completion contract and living backlog | This roadmap records the audited baseline, release horizons, ordered work, dependencies, and discovery log. |
| 1 | FM-B002 | done | Make the blog a clean-checkout, one-command build | The documented command bootstraps local package dependencies, builds the site, and is exercised by a pull-request CI smoke test. Documentation matches the configured stage count, uses valid spec links, and matches the command CI runs. |
| 2 | FM-B019 | done | Implement explicit wires and deterministic fan-out | The DAG honors `PipelineConfig.wires`, validates compatible edges and at most one producer per input, lets one materialized stream feed multiple consumers without mutation, and reports every true sink. Focused tests prove the router → renderer/collector branch needed by the blog. |
| 3 | FM-B003 | done | Integrate routing and collections in the blog DAG | `forme-router` assigns canonical routes, the chronological collector consumes routed nodes, and downstream stages consume both page and collection outputs without ad hoc route derivation. |
| 4 | FM-B004 | done | Ship the complete blog surface | The live build emits a root index, article pages, RSS/Atom, sitemap, and discovery/meta links with tests for URLs and ordering. |
| 5 | FM-B020 | done | Retire route-policy compatibility fallbacks | Migrate `forme-hello-world` to `forme-router`, require routed nodes in renderer/collector product paths, remove duplicate URL formatting policy, and retain a clear diagnostic for unrouted input. |
| 6 | FM-B021 | done | Model aggregate output provenance | Collection-derived pages can carry all contributing logical/revision IDs without a synthetic single-source placeholder; hashing, diagnostics, and tests cover deterministic aggregates. |
| 7 | FM-B023 | done | Repair standalone Forme TypeScript build closure | `forme-parse-markdown` and its local dependency closure pass `tsc` without mutable/readonly escape casts, and the package's BUILD/CI gate exercises compilation as well as tests. |
| 8 | FM-B022 | done | Modernize CI actions and warning-producing test config | Workflows use action releases supported on the current runner runtime, cache steps point at real dependency files, Vitest 4 configuration uses its current schema, and representative PR CI is free of the repeated runtime, missing dependency-file, and deprecated-config warnings. |
| 9 | FM-B024 | done | Finish the third-party Node 24 workflow closure | Gradle uses a current Node 24 action, the unmaintained Node 20 MSVC action is replaced by a tested repository-owned bootstrap, generated CI workflow contracts stay current, and final PR logs contain no Node 20 runtime warning. |
| 10 | FM-B005 | done | Make static rendering consume Style IR | The renderer accepts a resolved theme, records `usedStyle`, emits sliced CSS through the AOT path, supports light/dark preferences, and removes its hard-coded theme. |
| 11 | FM-B025 | done | Implement typed named input ports and deterministic fan-in | A stage can declare required named side-input kinds in addition to its default input; explicit wires validate one producer per port, the DAG orders every dependency, and the scheduler invokes the join once with stable materialized inputs. Focused tests prove `Stream<RenderedPage>` + `Stream<Asset>` fan-in without filesystem or event-bus side channels. |
| 12 | FM-B026 | done | Resolve local asset references and renderer placeholders | A filesystem-backed transform discovers local `ImageNode` references, rejects root escapes, assigns one identity per normalized source path, records source locators in `AssetRef`, and preserves external/data/hash URLs. Static rendering replaces resolved references with collision-free placeholders and records exact `usedAssets`; focused tests cover nested AST paths, duplicate references, cancellation, and identity sidecars. |
| 13 | FM-B027 | done | Load referenced filesystem assets into Asset IR | Depends on FM-B026. One collector invocation reads every unique referenced source, detects MIME type, preserves the resolved identity, hashes bytes into `revision`, emits deterministic `Asset` values, and diagnoses missing files or identity collisions without hidden state. |
| 14 | FM-B030 | done | Make SVG MIME sniffing linear-time | Replace the uncontrolled-data backtracking expression found by CodeQL with a bounded prefix scanner; preserve XML declaration, comment, BOM, whitespace, and case-insensitive SVG detection; adversarial repeated-comment input and CodeQL pass. |
| 15 | FM-B028 | done | Emit and rewrite fingerprinted assets | Depends on FM-B025 and FM-B027. An asset-aware filesystem emitter joins rendered pages with assets, writes content-hashed filenames, rewrites only Forme placeholders, includes bytes and `DeployAssetEntry` records in the artifact, and covers the complete path with an end-to-end pipeline test. |
| 16 | FM-B006 | done | Add a first-class asset pipeline | Depends on FM-B026–FM-B028. Referenced local assets are discovered, fingerprinted, copied, rewritten to cache-safe URLs, and included in the deploy manifest; the clean blog build verifies exact artifact and on-disk bytes. |
| 17 | FM-B007 | ready | Generate the repository landing page with Forme | Depends on FM-B005 and FM-B006. Forme source and configuration reproduce the approved landing design; generated output replaces hand-maintained HTML and deploys through the existing Pages workflow. |
| 18 | FM-B008 | ready | Implement the general headless CLI | `forme build`, `forme check`, and `forme clean` load a project config, produce stable diagnostics and exit codes, expose reproducible mode, and work outside the monorepo demo driver. |
| 19 | FM-B009 | blocked | Implement watch mode and a dev server | Depends on FM-B008. File changes rebuild the correct affected set, browser refresh is reliable, cancellation is clean, and error pages preserve the last good output. |
| 20 | FM-B010 | ready | Finish orchestrator incrementality and scheduling | Persistent cache hits skip unchanged stages; bounded streaming, backpressure, and `maxConcurrency` avoid draining every stream into memory; deterministic tests cover cancellation and reproducibility. |
| 21 | FM-B011 | ready | Reconcile the FM spec map | Resolve the FM05 numbering collision, publish the missing Interactivity IR/AOT/CLI spec locations, repair stale cross-links, and add an implementation-status ledger to every FM spec. |
| 22 | FM-B012 | ready | Implement the deploy runner | Build the FM05 core, filesystem adapter, GitHub Pages adapter, dry-run/reporting path, rollback/idempotency tests, and `forme deploy` composition. |
| 23 | FM-B013 | ready | Specify and implement Interactivity IR | Define the behavior/event/state schema and validator, integrate per-page island tracking, and prove a progressively enhanced interactive component with a no-JS fallback. |
| 24 | FM-B014 | blocked | Implement the plugin host and wire protocol | Depends on FM-B011 and the existing manifest parser. Stage discovery, handshake, typed streaming, capability mediation, diagnostics, cancellation, and crash isolation pass cross-process contract tests. |
| 25 | FM-B015 | blocked | Ship plugin installation, runtimes, and sandboxes | Depends on FM-B014. Signed/trusted install flow, grants persistence, TypeScript/Python/Rust runners, and macOS/Linux/Windows sandbox profiles pass adversarial filesystem/network/process tests. |
| 26 | FM-B016 | blocked | Build the authoring shell | Depends on FM-B009, FM-B013, and FM-B015. A non-developer can create, edit, preview, configure, and publish a site without hand-editing source or config files. |
| 27 | FM-B017 | blocked | Prove the backend boundary | Depends on FM-B005 and FM-B013. The same content and theme compile through HTML plus at least one of terminal, PDF/print, or email with explicit degradation tests. |
| 28 | FM-B018 | blocked | Close release-quality gates | Depends on the v1 product path. Add 1,000-page clean/incremental benchmarks, Lighthouse/accessibility budgets, package/API versioning, migration docs, security review, and supported-platform CI. |
| 29 | FM-B029 | later | Make duplicate PR CI cancellation and merge state unambiguous | One commit has one authoritative required CI suite; branch updates cancel obsolete runs completely; cancelling a redundant push suite cannot leave a stale final gate or misleading failed rollup; babysitting tooling identifies required checks and the current head. |

## Dependency path

The shortest path to the current release target is:

`FM-B002 → FM-B019 → FM-B003 → FM-B004`, alongside `FM-B005` and
`FM-B025 → FM-B026 → FM-B027 → FM-B030 → FM-B028 → FM-B006`, then
`FM-B007 → FM-B008 → FM-B009/FM-B010 → FM-B012`.
FM-B020 retires the temporary compatibility path after the routed product DAG
is proven, but it does not block FM-B004.

FM-B011 should land before new numbered specs are added. FM-B013–FM-B018 are
visible v1 work and must not interrupt the headless dogfood path unless they
uncover a contract or security flaw in that path.

## Discovery log

Add discoveries here before the next prioritization run. Promote each one to a
backlog item, merge it into an existing item, or explicitly close it as not
work.

| Date | Discovery | Disposition |
|---|---|---|
| 2026-08-27 | The blog README advertises `npm install && npm run build`, but file-linked source packages require their own dependency bootstrap in a clean checkout. | FM-B002 |
| 2026-08-27 | Blog comments and package metadata say “five stages”; the actual configured pipeline has four because the collector is not wired. | FM-B002 |
| 2026-08-27 | The blog README links to a nonexistent lowercase FM00 filename. | FM-B002 |
| 2026-08-27 | `forme-router` exists, but the blog renderer still derives routes from `sourcePath`; collection output has no path back into page rendering. | FM-B003 |
| 2026-08-27 | Index and feed packages exist, while the deployed blog root remains missing. | FM-B004 |
| 2026-08-27 | `forme-render-static` hard-codes a light classless theme instead of consuming Style IR. | FM-B005 |
| 2026-08-27 | FM01–FM04 reserve FM05/FM06/FM07 for Interactivity IR, AOT, and CLI, but FM05 is now the deploy runner and FM06/FM07 files are absent. | FM-B011 |
| 2026-08-27 | The orchestrator README understates the implemented reproducible-build support, while caching, bounded streaming, fan-out, and concurrency remain incomplete. | FM-B010 and FM-B011 |
| 2026-08-27 | The Forme blog workflow can succeed while the public blog root is missing, so deployment success alone is not an end-to-end availability check. | FM-B004 and FM-B012 |
| 2026-08-27 | The DAG builder ignores explicit `wires` and assigns one inferred producer per instance. Routed nodes therefore cannot feed article rendering and collection building in the same blog run. | Added FM-B019 and moved it ahead of FM-B003. |
| 2026-08-27 | `forme-orchestrator` tests pass but its TypeScript build cannot resolve Node APIs imported through `forme-cache` and `forme-pipeline-config` because the package omits `@types/node`. | Resolve in FM-B019 and retain `npm run build` as a package gate. |
| 2026-08-27 | The blog deployment workflow did not watch `forme-router` or `forme-collect-chronological`, so future changes to its routed DAG could bypass the clean-build check. | Resolved in FM-B003 by adding both package paths. |
| 2026-08-27 | The separate `forme-hello-world` demo remains routerless, which requires renderer/collector compatibility fallbacks to keep duplicate route formatting policy alive. | Added FM-B020 after FM-B004; it does not block the complete blog surface. |
| 2026-08-27 | Portable file routes such as `/blog/post.html` become broken root-relative links on a GitHub Pages project site unless the public deployment prefix is composed separately. | Resolved in FM-B004 by keeping file routes portable and using absolute project-page URLs in headers, canonical links, indexes, feeds, and sitemap. |
| 2026-08-27 | `RenderedPage.source` accepts only one `LogicalId`, but indexes, feeds, and sitemaps aggregate a collection. | Resolved in FM-B021 with normalized logical/revision contributor sets and deterministic aggregate hashes; the synthetic blog ID is removed. |
| 2026-08-27 | Adding renderer metadata dependencies left its own and `forme-hello-world`'s standalone BUILD prerequisite lists incomplete. | Resolved in FM-B004 after CodeQL's BUILD/CI validator identified both transitive gaps. |
| 2026-08-27 | Current Actions releases repeatedly warn that their Node 20 runtime is deprecated, while Go/uv caches point at dependency files that do not exist at the configured root. | Resolved in FM-B022 with Node 24 action releases and repository-real Go/Python cache dependency globs. |
| 2026-08-27 | The `forme-hello-world` test run warns that Vitest 4 removed `test.poolOptions`; two program configs still use the old schema. | Resolved in FM-B022 across all four remaining deprecated Vitest configs. |
| 2026-08-27 | `forme-parse-markdown` tests pass, but its standalone TypeScript build reaches a readonly-to-mutable `DocumentNode.children` cast in `gfm-parser`. | Resolved in FM-B023 with immutable block/inline AST transforms and compilation in both parser BUILD gates. |
| 2026-08-27 | `RenderedPage` v1.1 keeps the legacy single-`source` producer branch so existing stages can migrate without a kernel-wide API break. | Fold removal of the legacy branch, kind v2/API migration, and downstream migration notes into FM-B018's package/API versioning gate. |
| 2026-08-27 | The web-app scaffolder still generated the same retired action majors, so newly created workflows would reintroduce Node runtime warnings. | Resolved in FM-B022 by updating both generated workflow templates and adding output assertions. |
| 2026-08-27 | Workflow lint found two Intel release jobs pinned to the retired `macos-13` hosted runner. | Resolved in FM-B022 by preserving their x86_64 intent on `macos-15-intel`. |
| 2026-08-27 | OCAML03's fail-closed workflow contract separately pins its reviewed action commits, so a repository-wide action upgrade must update that manifest and validator allowlist together. | Resolved in FM-B022 after PR CI exposed the stale allowlist; the OCAML03 repository validator and all 46 contract tests now cover the upgraded commits. |
| 2026-08-27 | CodeQL still provisioned Go 1.23 even though the shared build tool now requires Go 1.26.1, so its language-detection job could not compile the detector. | Resolved in FM-B022 by aligning both CodeQL Go setup steps with CI's Go 1.26.1 pin. |
| 2026-08-27 | The green Android workflow still emitted a Node 20 warning from `android-actions/setup-android@v3`, which the first-party action audit did not cover. | Resolved in FM-B022 by upgrading both Android setup steps to v4.0.1, whose action manifest declares Node 24. |
| 2026-08-27 | The final FM-B022 log audit found `gradle/actions/setup-gradle@v4` still emitting Node 20 warnings on every core platform even though the workflow passed. | Added FM-B024 ahead of Style IR; upgrade the canonical workflow and all generated-workflow contracts to Gradle Actions v6.3.0 (Node 24). |
| 2026-08-27 | `ilammy/msvc-dev-cmd@v1.13.0` is the latest upstream release and still declares Node 20, so no supported action upgrade exists. | Added FM-B024; replace it with a tested repository-owned bootstrap that locates `vcvarsall.bat`, exports only its environment delta, and verifies `cl.exe` plus MSVC `link.exe`. |
| 2026-08-27 | Windows 2025 runners now carry Visual Studio 2026, and embedding its space-containing `vcvarsall.bat` path in the argument after `cmd.exe /c` exposed quote ambiguity in Python's Windows argv serialization. Both the initial `CALL` form and a direct form failed before a usable environment was captured. | Resolved in FM-B024 by writing the reviewed invocation to a temporary `.cmd` wrapper, invoking that wrapper by its space-free basename, testing its exact content and cleanup, and surfacing captured output on any future failure. |
| 2026-08-27 | On the full Windows workflow path, `ruby/setup-ruby` installed Ruby 3.4.10 but later toolchain setup left the final `Path` resolving the image's Ruby 3.3.12; the existing verification step only printed the mismatch before the Ruby build failed. | Resolved in FM-B024 by restoring the action's documented `ruby-prefix` paths after all toolchain setup and enforcing the repository's Ruby 3.4 floor in the verification gate. |
| 2026-08-27 | A workflow-wide build plan scheduled pure-BEAM packages on Windows while the canonical matrix deliberately skipped Elixir setup, so `mix` was unavailable. | #13287's latest Windows CI proved the reviewed setup-beam revision on Windows 2025. FM-B024 adopts only that prerequisite and enables tool verification; #13287 retains ownership of the broader BUILD_windows protocol and package-front work. |
| 2026-08-27 | Style IR can match document and shell selectors, but generated aggregate HTML has no AST to inspect. | Resolved in FM-B005 by conservatively retaining the complete theme for trusted generated index HTML while article pages receive exact AST-derived slices. |
| 2026-08-27 | The renderer-owned CSS fallback hid whether Style IR and the AOT slicer were actually connected, and it prevented theme replacement without changing renderer code. | Resolved in FM-B005 by moving the reusable light/dark theme to `forme-theme-classless`; unconfigured rendering is intentionally unstyled. |
| 2026-08-27 | Asset emission must join rendered pages with processed asset bytes, but stages expose one input and config validation rejects every second incoming wire even though `EdgeSpec` already carries an unused target port. | Resolved in FM-B025 with typed named side inputs and deterministic scheduler fan-in; cross-stage data stays out of frontmatter, the event bus, and hidden filesystem side channels. |
| 2026-08-27 | FM-B006 spans four contracts that must be independently reviewable: reference resolution, Asset IR loading, placeholder rewriting/fingerprinted emission, and the integrated product proof. Existing `AssetRef` lacks the source locator needed by a filesystem loader, while `RenderedPage.usedAssets` is always empty. | Split the dependency path into FM-B026–FM-B028. Start with source-safe reference resolution and renderer usage tracking; keep FM-B006 as the completion milestone. |
| 2026-08-27 | Local asset URLs may carry cache parameters or SVG fragment targets. Stripping them during source resolution would silently change the rendered document after fingerprinting. | FM-B026 records the authored suffix separately from filesystem identity and carries it through the reserved renderer placeholder for FM-B028 to restore. |
| 2026-08-27 | Lexical root containment prevents `..` traversal during reference resolution, but only the byte loader can detect a symlink that resolves outside the configured storage root. | Resolved in FM-B027 by comparing canonical root and asset paths before reading, allowing in-root symlinks and rejecting escapes. |
| 2026-08-28 | GitHub's automatic branch update superseded an in-flight PR run, while the obsolete run left its final gate queued and the same-head duplicate push suite left cancelled checks in the rollup. The required PR checks still auto-merged, but the intermediate state was misleading and consumed babysitting time. | Added FM-B029 as later delivery-infrastructure work. It does not displace the fingerprinted-asset critical path. |
| 2026-08-28 | Existing structured revisions require canonical JSON, which would expand large asset bytes into costly number arrays. | Resolved in FM-B027 with a domain-separated binary revision primitive that hashes opaque bytes directly. |
| 2026-08-28 | CodeQL found that SVG MIME sniffing applied a backtracking regular expression to asset bytes, allowing repeated comment-shaped input to consume super-linear time. | Added and resolved FM-B030 ahead of FM-B028 with a bounded prefix scanner plus adversarial repeated-comment coverage. |
| 2026-08-28 | Named input ports are intentionally required, so adding `assets` to the existing page-only emitter would break every current pipeline before the product DAG could be migrated. | Resolved in FM-B028 with a separate asset-aware `forme-emit-site-fs` stage; FM-B006 can now switch the blog atomically while the legacy emitter remains compatible. |
| 2026-08-28 | A root-owned `/assets/...` URL works on a custom domain but breaks the repository's GitHub Pages project deployment beneath `/coding-adventures`. | Resolved in FM-B028 with a validated, segment-encoded `publicPathPrefix` option covered by the emitter tests. |
| 2026-08-28 | Asset "caching" spans two distinct guarantees: immutable content-hashed public URLs and build-time stage reuse. The former is required for a complete deployable asset path; the latter belongs to orchestrator incrementality and must not be hidden inside a filesystem emitter. | FM-B006 proves cache-safe content URLs in the live blog path; persistent build reuse remains explicit in FM-B010. |
| 2026-08-28 | The blog has two disjoint deploy sinks sharing `dist`: articles need page/asset fan-in, while collection-derived index/feed/sitemap output owns no assets. | Resolved in FM-B006 by migrating only `emit-articles` to `forme-emit-site-fs`; `emit-surface` remains the compatible page-only sink, and build assertions reject asset ownership drift. |

## Loop protocol

For every Forme delivery loop:

1. Re-read the discovery log and current implementation, then reprioritize this
   table.
2. Mark exactly one unblocked item `active` and define its PR-sized acceptance
   gate before editing code.
3. Implement and validate locally, update this roadmap with discoveries, and
   open a focused PR.
4. Enable auto-merge, babysit every required check, and fix CI failures or merge
   conflicts on the same branch.
5. After merge, mark the item `done`, perform a fresh prioritization run, and
   pick the next item.
