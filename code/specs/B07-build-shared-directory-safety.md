# B07 — Build Shared-Directory Safety

## Overview

The build-tool can schedule two packages concurrently when one of them
deletes a directory the other is reading. This spec describes the defect,
why the existing mitigation misses it, and the fix: distinguishing readers
from writers of shared `node_modules` directories, and locking accordingly.

The symptom, observed on PR #15839 and again on its five-times-smaller
successor #15858:

    --- FAILED: unknown/blog ---
    Error [ERR_MODULE_NOT_FOUND]: Cannot find package
      '@coding-adventures/state-machine' imported from
      code/packages/typescript/cli-builder/src/parser.ts
    Total: 5222 packages | 312 built | 4909 skipped | 1 failed

## How the repository installs local dependencies

TypeScript packages depend on each other with npm `file:` specifiers:

    "@coding-adventures/directed-graph": "file:../directed-graph"

`npm install` turns that into a symlink,
`state-machine/node_modules/@coding-adventures/directed-graph ->
../directed-graph`. Node resolves imports from the *real* path of a linked
package, not the link path, so when `state-machine/src/dfa.ts` imports
`@coding-adventures/directed-graph`, resolution starts at
`code/packages/typescript/state-machine/` and walks up. It therefore
requires `state-machine/node_modules/` to exist **at that instant**.

Because a `file:` link's target does not install its own dependencies, each
package's `node_modules` has to be populated by someone running an install
*in that directory*. BUILD files do this explicitly:

    # code/packages/typescript/state-machine/BUILD
    cd ../directed-graph && npm ci --quiet
    npm ci --quiet
    npx vitest run --coverage

Measured over the repository: **113 BUILD files run `npm ci` or
`npm install` into `../state-machine`, and 120 into `../directed-graph`.**
Those two directories are shared mutable state with a hundred-plus writers.

## Why this races

`npm ci` is documented to delete `node_modules` before reinstalling. That
makes every one of those 113 commands a window in which
`state-machine/node_modules` does not exist.

The executor runs packages in parallel by dependency level
(`internal/executor/executor.go`), with a semaphore bounding concurrency to
`-jobs`, default `runtime.NumCPU()`. Two packages in the same level run
concurrently by design. If one of them is mid-`npm ci` in `../state-machine`
while the other is resolving imports through
`state-machine/node_modules`, the reader fails with `ERR_MODULE_NOT_FOUND`.

## Why the existing mitigation misses it

The executor already has a lock manager. `buildResourceLocker` hands out a
mutex per key, and `buildResourceKeys` derives the keys for a package:

```go
for _, raw := range relPathRe.FindAllString(command, -1) {
    normalized := strings.ReplaceAll(raw, "\\", "/")
    abs := filepath.Clean(filepath.Join(pkg.Path, filepath.FromSlash(normalized)))
    if name, ok := pathToPkg[abs]; ok {
        keys[name] = true
    }
}
```

Every BUILD command is scanned for relative paths; any that resolves to a
known package contributes that package's name as a lock key. So
`cd ../state-machine && npm ci` *does* correctly lock
`typescript/state-machine`, and two writers are already serialised against
each other. The mechanism is sound and the `global:` keys above it (Hex,
rustup, LuaRocks, Cabal, dotnet, Gradle-on-Windows) show the pattern is
already trusted for exactly this class of problem.

**The gap is that keys are derived from BUILD command text, which sees
writers but not readers.** `code/sites/blog/BUILD` is:

    npm install --silent && npm run clean && npm run build && npm test && ...

There is not a single relative path in it. The blog reaches
`state-machine/node_modules` only at runtime, when `npm run clean` executes
`forme` under `tsx` and tsx resolves a `file:`-linked import chain
(`forme-cli` → `cli-builder` → `state-machine` → `directed-graph`).
Nothing in the command text names any of those packages, so the blog
acquires no lock for them and is free to run beside a writer.

This is why adding two `deps=` edges to the blog did not fix it, and why
the failure recurred at a fifth of the batch size. `deps=` orders a
dependency's *build* before its dependent's; it does not stop an unrelated
third package from running `cd ../cli-builder && npm ci` during the
dependent's build.

## The fix

Extend lock-key derivation from "paths named in my BUILD" to "paths my
build touches", and distinguish the two ways of touching them.

**Writers.** A package whose BUILD text resolves a relative path to a known
package X writes X, and needs *exclusive* access to X. This is today's
behaviour and is unchanged.

**Readers.** A package whose build resolves `file:` imports into X reads X,
and needs X to be *stable* for the duration — but not exclusive. The reader
set is the transitive closure of `file:` dependencies declared in
`package.json`, which is the same closure
`forme-cli/bin/bootstrap.mjs` already computes for its install ordering.

A single mutex per key cannot express this, and using one would be a cure
worse than the disease: `directed-graph` is read by 232 packages, so an
exclusive lock per reader would serialise nearly the whole TypeScript tree
and turn a 20-minute build into an all-day one.

So `buildResourceLocker` changes from `map[string]*sync.Mutex` to
`map[string]*sync.RWMutex`:

- writers call `Lock()` — excludes all other readers and writers,
- readers call `RLock()` — concurrent with other readers, excluded by any
  writer.

Parallelism is preserved where it is safe (many packages reading
`directed-graph` at once) and removed only where it is not (anyone reading
`directed-graph` while someone reinstalls it).

### Deadlock safety

The current implementation sorts keys before acquiring, which is what makes
multi-key acquisition deadlock-free. That invariant must survive: keys are
still sorted, and read and write acquisitions interleave in the same total
order. A package that both reads and writes X takes the **write** lock,
never both.

### Scope

The reader set is computed only for packages whose BUILD invokes npm, since
this is a `node_modules` problem. Other ecosystems' shared state is already
covered by the `global:` keys. Non-npm packages are unaffected, as are npm
packages with no `file:` dependencies.

## Test plan

Unit tests beside the existing `buildResourceKeys` tests in
`internal/executor/executor_test.go`, which already cover the `global:` keys
and the relative-path extraction:

1. **Reader keys are derived.** A package whose BUILD names no relative path
   but whose `package.json` has `file:` deps yields read keys for the whole
   transitive closure. This is the blog's exact shape and fails before the
   fix.
2. **Writer keys stay exclusive.** `cd ../state-machine && npm ci` still
   yields a write key for `typescript/state-machine`.
3. **Read and write are distinguished.** A package that reads X and a
   package that writes X are mutually exclusive; two readers of X are not.
4. **Write beats read.** A package that both reads and writes X takes the
   write lock only.
5. **Ordering holds.** Keys are sorted, so concurrent acquisition of
   overlapping key sets cannot deadlock. Exercised with a stress test that
   runs many goroutines over intersecting key sets under `-race`.
6. **Non-npm packages unchanged.** A Rust or Go package's key set is
   identical before and after.

Integration check: rebuild the build-tool, emit a plan for the failing
change, and confirm `unknown/blog` acquires read keys covering
`typescript/state-machine`, `typescript/cli-builder` and
`typescript/directed-graph`.

## What this does not address

It does not make `npm ci` atomic, and it does not remove the underlying
design where a hundred packages install into one directory. A package
manager that installed each package's dependencies into its own tree, or a
single workspace install at the repository root, would make the whole class
of problem disappear. That is a larger change to how this repository
handles `file:` dependencies and is out of scope here.

It also does not address CPU starvation. The same oversized run that
exposed this race also failed a `language-ladder` test on vitest's 5000 ms
default timeout, with 72 s of wall clock against 26,314 s of aggregate
transform time. Serialising writers will reduce peak load somewhat, but if
timeouts persist the answer is a `testTimeout` appropriate to a loaded CI
machine, not a scheduling change.
