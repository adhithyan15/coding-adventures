# Forme Completion Roadmap

> **Status:** Living delivery backlog, last prioritized 2026-08-27.
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

- 57 TypeScript `forme-*` packages and 174 package test files cover the kernel,
  stage contracts, a sequential orchestrator, Style IR, AOT emitters, document
  transforms, collections, feeds, routing, and static output.
- The blog proves an eight-stage routed DAG: source → parse → router fans out
  to article rendering and chronological collection, then two filesystem sinks
  emit articles plus an index, RSS, Atom, and sitemap. Public metadata composes
  portable routes with the GitHub Pages project prefix. Assets and a Style IR
  theme are not yet integrated.
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
| 7 | FM-B023 | ready | Repair standalone Forme TypeScript build closure | `forme-parse-markdown` and its local dependency closure pass `tsc` without mutable/readonly escape casts, and the package's BUILD/CI gate exercises compilation as well as tests. |
| 8 | FM-B022 | ready | Modernize CI actions and warning-producing test config | Workflows use action releases supported on the current runner runtime, cache steps point at real dependency files, Vitest 4 configuration uses its current schema, and representative PR CI is free of the repeated runtime, missing dependency-file, and deprecated-config warnings. |
| 9 | FM-B005 | ready | Make static rendering consume Style IR | The renderer accepts a resolved theme, records `usedStyle`, emits sliced CSS through the AOT path, supports light/dark preferences, and removes its hard-coded theme. |
| 10 | FM-B006 | ready | Add a first-class asset pipeline | Referenced local assets are discovered, fingerprinted, copied, rewritten, cached, and included in the deploy manifest. |
| 11 | FM-B007 | blocked | Generate the repository landing page with Forme | Depends on FM-B005 and FM-B006. Forme source and configuration reproduce the approved landing design; generated output replaces hand-maintained HTML and deploys through the existing Pages workflow. |
| 12 | FM-B008 | ready | Implement the general headless CLI | `forme build`, `forme check`, and `forme clean` load a project config, produce stable diagnostics and exit codes, expose reproducible mode, and work outside the monorepo demo driver. |
| 13 | FM-B009 | blocked | Implement watch mode and a dev server | Depends on FM-B008. File changes rebuild the correct affected set, browser refresh is reliable, cancellation is clean, and error pages preserve the last good output. |
| 14 | FM-B010 | ready | Finish orchestrator incrementality and scheduling | Persistent cache hits skip unchanged stages; bounded streaming, backpressure, and `maxConcurrency` avoid draining every stream into memory; deterministic tests cover cancellation and reproducibility. |
| 15 | FM-B011 | ready | Reconcile the FM spec map | Resolve the FM05 numbering collision, publish the missing Interactivity IR/AOT/CLI spec locations, repair stale cross-links, and add an implementation-status ledger to every FM spec. |
| 16 | FM-B012 | ready | Implement the deploy runner | Build the FM05 core, filesystem adapter, GitHub Pages adapter, dry-run/reporting path, rollback/idempotency tests, and `forme deploy` composition. |
| 17 | FM-B013 | ready | Specify and implement Interactivity IR | Define the behavior/event/state schema and validator, integrate per-page island tracking, and prove a progressively enhanced interactive component with a no-JS fallback. |
| 18 | FM-B014 | blocked | Implement the plugin host and wire protocol | Depends on FM-B011 and the existing manifest parser. Stage discovery, handshake, typed streaming, capability mediation, diagnostics, cancellation, and crash isolation pass cross-process contract tests. |
| 19 | FM-B015 | blocked | Ship plugin installation, runtimes, and sandboxes | Depends on FM-B014. Signed/trusted install flow, grants persistence, TypeScript/Python/Rust runners, and macOS/Linux/Windows sandbox profiles pass adversarial filesystem/network/process tests. |
| 20 | FM-B016 | blocked | Build the authoring shell | Depends on FM-B009, FM-B013, and FM-B015. A non-developer can create, edit, preview, configure, and publish a site without hand-editing source or config files. |
| 21 | FM-B017 | blocked | Prove the backend boundary | Depends on FM-B005 and FM-B013. The same content and theme compile through HTML plus at least one of terminal, PDF/print, or email with explicit degradation tests. |
| 22 | FM-B018 | blocked | Close release-quality gates | Depends on the v1 product path. Add 1,000-page clean/incremental benchmarks, Lighthouse/accessibility budgets, package/API versioning, migration docs, security review, and supported-platform CI. |

## Dependency path

The shortest path to the current release target is:

`FM-B002 → FM-B019 → FM-B003 → FM-B004`, alongside `FM-B005` and
`FM-B006`, then `FM-B007 → FM-B008 → FM-B009/FM-B010 → FM-B012`.
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
| 2026-08-27 | Current Actions releases repeatedly warn that their Node 20 runtime is deprecated, while Go/uv caches point at dependency files that do not exist at the configured root. | Added FM-B022 after correctness-focused FM-B021; CI remains green because GitHub currently forces the actions onto Node 24. |
| 2026-08-27 | The `forme-hello-world` test run warns that Vitest 4 removed `test.poolOptions`; two program configs still use the old schema. | Folded into FM-B022 so the warning cleanup is handled consistently rather than expanding FM-B020's routing scope. |
| 2026-08-27 | `forme-parse-markdown` tests pass, but its standalone TypeScript build reaches a readonly-to-mutable `DocumentNode.children` cast in `gfm-parser`. | Added FM-B023 after FM-B021; make compilation an explicit package gate so this cannot silently recur. |
| 2026-08-27 | `RenderedPage` v1.1 keeps the legacy single-`source` producer branch so existing stages can migrate without a kernel-wide API break. | Fold removal of the legacy branch, kind v2/API migration, and downstream migration notes into FM-B018's package/API versioning gate. |

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
