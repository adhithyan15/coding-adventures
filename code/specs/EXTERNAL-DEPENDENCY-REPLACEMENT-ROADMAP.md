# Replace external dependencies with internal implementations

Status: planned migration work, 2026-10-11. The dependency security sweep in
PR #17185 is the immediate repair. This roadmap is the separate delivery plan
for removing the third-party dependency paths that keep producing those alerts.
The historical advisory register remains
[EXTERNAL-DEPENDENCY-ALERT-BACKLOG.md](EXTERNAL-DEPENDENCY-ALERT-BACKLOG.md).

## Outcome and scope

Replace third-party application, test and build packages with maintained internal
implementations. Begin with npm, where every open alert in the authenticated
snapshot occurs, then apply the same inventory and migration gates to every
implementation language and generated product. A patched dependency is still an
external dependency; zero alerts does not mean this roadmap is complete.

Inventory operating-system APIs, language runtimes, compiler bootstraps, browser
engines, native frameworks and release tools too. Their contracts make them a
later migration stage rather than invisible exceptions. Record each remaining
external component and why it is still needed. Do not claim complete independence
while Node/V8, TypeScript, Qt, Flutter, Tauri or similar components remain in a
build or product path. Native UI changes must satisfy
[UI38](UI38-mosaic-native-application-runtime.md) and
[UI90](UI90-mosstyle-platform-neutral.md), including genuine native controls and
accessibility, rather than changing the product contract to make removal easier.

Each item below is a bounded sequence of PRs, not a promise that a parser or
simulator already substitutes for a complete upstream runtime.

## Measured starting point

The tracked npm manifest census at `8f5c92574c096a9ca1f1d661f02d84ff5e9bd260`
contains 529 manifests and 33 distinct direct external package names. Internal
package names and `file:`, `workspace:` and `link:` references were excluded.
Counts below are distinct manifests, including peer and optional declarations;
they are not lockfile counts. Generated locks without a tracked sibling manifest
also need the inventory work in DEP-000. All 510 tracked npm locks were audited
for the security sweep; their transitive graph is larger than these 33 roots.

| Family | Direct external packages | Current examples / reach |
|---|---|---|
| Tests and coverage | `vitest`, `@vitest/coverage-v8` | 506 / 490 manifests |
| DOM tests and matchers | `jsdom`, `happy-dom`, `@testing-library/dom`, `@testing-library/react`, `@testing-library/jest-dom` | jsdom in 32; React test helpers in 19 |
| Compiler and web build | `typescript`, `ts-node`, `tsx`, `esbuild`, `vite`, `@vitejs/plugin-react`, `@types/node` | TypeScript in 512; Vite in 30; tsx in 19 |
| Web UI | `react`, `react-dom`, `@types/react`, `@types/react-dom` | React in 28; React DOM in 27 |
| Desktop and process tooling | `electron`, `electron-builder`, `cross-env`, `@tauri-apps/api`, `@tauri-apps/cli`, `postject` | Four declared Electron apps, plus generated Engram locks |
| Parsing, codec and storage tests | `parse5`, `pngjs`, `@types/pngjs`, `fake-indexeddb` | Two Forme renderers; PNG conformance; bookmarks storage |
| Browser validation | `@playwright/test`, `chrome-launcher`, `lighthouse` | VisiCalc/Todo workflows and Forme release budgets |
| Editor integration | `vscode-languageclient`, `@types/vscode` | Twig VS Code integration |

Historical roots such as Jest have already been removed by the current sweep.
Transitive packages such as source-map-js, postcss, tar, undici, js-yaml,
shell-quote and sprintf-js remain assigned to the root paths that introduce
them, including generated manifests. Migrate a root capability once rather than
building unused copies of every transitive package.

## Delivery order

| ID | Work package | Internal foundation and work still required | Completion evidence |
|---|---|---|---|
| DEP-000 | Reproducible dependency inventory | Extend repository reporting with ecosystem, direct/transitive role, exact source, generator owner and consumer edges. Include npm, Cargo, Python, Go, Ruby, Dart, Hex, JVM/.NET manifests, Actions and downloaded tools. | Every retained external edge has a named work item or explicit bootstrap/platform contract; deterministic JSON plus a readable report. |
| DEP-010 | Internal assertions and test runner | Build dedicated TypeScript assertions and a bounded Node host adapter for discovery, suites, hooks, async cases, parameterized cases, skips, diagnostics and process isolation. The Rust HDL `testbench-framework` is not a JavaScript test runner. | Shadow-run the numerical pilot suites with identical discovered cases, outcomes and deliberate-failure detection on all three hosts. The new runner has no third-party npm dependencies. |
| DEP-020 | Coverage, mocks and broad runner migration | Add V8 coverage conversion/merging, original-source mapping, thresholds, spies, restore/reset behavior, fake timers and the ESM module-mock semantics actually used by the corpus. | Preserve existing package coverage thresholds, uncovered-source accounting and failure behavior; then remove Vitest and its Vite/PostCSS/esbuild graph from library test paths in batches. |
| DEP-030 | HTML parsing and DOM test capabilities | Port/extend Rust `html-lexer`, `html-tree-builder`, `html-parser` and `dom-core` into the needed TypeScript lane. Implement the DOM mutations, selectors, events and accessibility queries used by real consumers. CSS parsing is an existing foundation, not computed-style support. | First replace parse5 in the two Forme renderers with matching hostile/malformed HTML and serialization fixtures; then migrate DOM test helpers without dropping interaction or accessibility assertions. |
| DEP-040 | Host process and packaging utilities | A real host process runner over OS/Node process APIs replaces concurrently, shell-quote and cross-env. Internal `zip` supplies an archive primitive. Existing `process-manager` is a kernel simulation, not a child-process launcher. | Argv and environment parity, Windows quoting, cancellation, descendant cleanup, exit-code propagation and bounded logs; reproducible archives with traversal/link guards. |
| DEP-050 | Desktop hosts and distributions | Migrate Electron consumers to the Rust application-engine/host contracts already specified by Mosaic. Replace electron-builder after installer, update, resource, signing and launch contracts are specified. Track Tauri/postject separately so they do not silently replace one external tree with another. | Engram, Journal, TaskApp and VisiCalc each launch, complete their workflows, preserve stored data, keyboard/IME input, accessibility and sandbox boundaries on supported platforms. Only then remove Electron and its packaging/network/archive graph. |
| DEP-060 | TypeScript compilation and bundling | Extend internal TypeScript/JavaScript grammar, AST, lowering, source-map and Forme emitters into a real checker/emitter/module graph. Cover JSX, resolution, declarations, ESM/CJS, assets and browser build needs before replacing tsc/tsx/ts-node/esbuild/Vite. | Existing builds and type-error fixtures produce equivalent accepted/rejected programs and usable artifacts; web product, security and performance gates pass. |
| DEP-070 | Web framework and storage adapters | Move React consumers toward internal Mosaic HTML/Web Component/event/Flux paths while preserving the existing state and UI contracts. Add the IndexedDB transaction/event test model needed to remove fake-indexeddb. | Product workflows, focus/selection, cleanup, persistence, accessibility and real-browser tests pass; React-related packages and types disappear from migrated consumers. |
| DEP-080 | Browser validation and editor adapters | Build internal bounded browser-control and measurement adapters and an LSP client adapter for the exact used protocols. Separate test automation from the browser engine and the VS Code platform API. | Equivalent interaction, navigation, crash/cancellation, resource/performance and accessibility evidence. Replacing Lighthouse requires a specified metric contract and demonstrated equivalent failure detection, not a lower budget. |
| DEP-090 | Other ecosystems and compiler/runtime bootstraps | Follow DEP-000's consumer graph and the existing package-parity roadmap. Close missing native implementations and stdlib/compiler/runtime/host capabilities with explicit conformance before removing the corresponding external packages or frameworks. | Pure implementations are verified per language; FFI/OS boundaries are named and tested. Retained compilers, V8/Node, browsers, SDKs and frameworks stay visible until their replacements execute the real workloads. |

DEP-010 and DEP-020 have the largest immediate reach. DEP-030 and DEP-040 can
proceed as independent bounded work packages once the inventory is available.
Desktop, compiler and framework migrations need their own capability milestones;
they are not prerequisites for beginning the runner migration.

## First implementation sequence

1. **Inventory PR:** add the report and generator-to-consumer links, record the
   current direct and resolved graphs, and list the Vitest APIs and transforms
   actually used. Do not infer compatibility from package names.
2. **Runner spec PR:** specify the owned assertion/matcher and host-runner
   contracts, including real timeout/cancellation behavior, source diagnostics,
   fixture lifetime and failure exit codes. Use Node standard APIs as an explicit
   initial host boundary; count the remaining TypeScript compiler separately.
3. **Numerical pilot:** migrate trig, wave, gradient-descent and loss-functions,
   then matrix including its optional native parity path. These are the five
   suites whose Jest ancestry the security sweep removed. Run old and new
   drivers against the same test inventory before deleting the old runner.
4. **Coverage and mock pilots:** select a small library with branch/error
   coverage, then a suite with spies, timers and ESM mocks. Include negative
   threshold and deliberately failing-test fixtures. Keep required CI gates
   intact while establishing coverage equivalence.
5. **Library batches:** migrate related groups only after the previous batch
   merges and its dependency graph is remeasured. Keep a bounded active PR and
   coordinate ownership through the existing package-parity roadmap/state.
6. **First product replacement:** migrate parse5 in Forme's static and terminal
   renderers using an internal HTML implementation. Preserve all current
   malformed-input bounds, identity/path checks and generated product evidence.
   Host-process tooling can be the next independent utility removal.

Node's built-in TypeScript stripping is a possible bootstrap optimization, not
the compiler replacement. It does not type-check, read tsconfig settings, handle
TSX, or resolve this repository's `.js` imports to `.ts` automatically; it also
refuses TypeScript files under node_modules. Compile pilot tests to JavaScript
with the existing checker first unless their complete source/import shape is
proven compatible. See the
[Node 22 TypeScript contract](https://nodejs.org/docs/latest-v22.x/api/typescript.html).
Retain the existing checker until DEP-060 passes its acceptance corpus.

The existing `compiler-source-map` models compiler pipeline positions. Browser
source-map v3/VLQ encoding, composition and original-source coverage attribution
still require an explicit implementation and fixtures. V8 can collect coverage
through its host API, but that does not itself implement source mapping,
unexecuted-file accounting or threshold semantics; see the
[V8 coverage API](https://nodejs.org/docs/latest-v22.x/api/v8.html#v8takecoverage).

## Conditions for removing an external package

- Specify the capability and supported input surface before implementation.
  Reuse a suitable internal package or add the missing lane through the parity
  queue; do not create a second competing implementation by accident.
- Preserve every existing consumer assertion, deliberate failure, skip condition,
  coverage threshold, resource bound, sandbox and accessibility contract.
  Unsupported behavior must fail explicitly until its migration is complete.
- Use standard fixtures, hostile inputs and independent comparison during the
  transition. For example, pngjs currently serves as an independent oracle for
  an already-internal PNG codec; replacing that oracle with the codec under test
  would remove evidence. Commit independently established vectors before retiring
  the external oracle. Track any temporary comparison job as an external edge.
- Prove real build/runtime behavior on Linux, macOS and Windows where applicable.
  A directory, parser, generated shell or passing mock is not completion evidence.
- Update source manifests, generated manifest emitters, BUILD files, release
  scripts, lockfiles, fixtures, docs and registry receipts together. Check the
  whole resolved graph: no other ancestor may still pull the retired package in.
- Run security review on the final head, land the bounded PR with acceptable
  checks, fetch main and remeasure before selecting the next migration.

## Progress ledger

Every work item starts **PLANNED**, moves to **PILOT** when a real consumer uses
the replacement, and becomes **REMOVED** only after the final external edge is
gone from the checked source and resolved graphs. Record the merged revision,
changed consumers and executed acceptance receipts. A proposed foundation or
patched release is not REMOVED.

Track direct external names, resolved external package/version pairs, graph
occurrences, generated-product edges, remaining bootstrap/platform components,
test inventory and consumer/runtime evidence. Report both removal and retained
dependencies; moving a package to CI, vendoring upstream code, or switching to
another third-party framework does not count as an internal replacement.

All DEP items above are currently PLANNED. The security sweep removed Jest and
sprintf-js paths, but the five pilot suites still use external Vitest. The next
delivery item is DEP-000 followed by the DEP-010 runner contract and numerical
pilot, not a repository-wide rewrite in one PR.
