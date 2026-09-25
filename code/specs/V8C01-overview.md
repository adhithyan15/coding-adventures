# V8C01 — V8-on-LANG-VM clone overview

## What this is

A pedagogical re-implementation of [V8](https://v8.dev), the
JavaScript engine that runs Chrome and Node.js. Built **on top of
the existing LANG VM chain in this monorepo** rather than from
scratch — so it composes with everything that's already here
(`javascript-tokens`, `javascript-ast`, `interpreter-ir`,
`lang-runtime-core`) instead of duplicating it.

The goal is the same goal as the Closure Compiler clone (CLOC01):
**trace the magic**. Understand how a JS engine works by writing
one, one tiny composable crate at a time, with the option to
swap in real V8-style algorithms once the scaffolding is up.

## Revision 2026-09-25 — the engine Venture runs

The owner has made this engine a product goal: **Venture's JavaScript engine,
built from scratch on the LANG VM** ([BR02](BR02-venture-completion-roadmap.md)
P9). That changes four decisions below. Where this section and the original
text disagree, this section wins.

1. **Values live on `gc-core` from the first runnable PR,** not in
   `Rc<RefCell<…>>`. The collector now exists (`FlatHeap`: precise roots,
   mark-sweep, generational, compacting, incremental). JavaScript object
   graphs are cyclic by default (`a.self = a`, closures capturing their own
   function), and DOM wrappers must be traced, so reference counting would
   leak from day one. V8C02 is revised to NaN-boxed values on `gc-core`.
2. **The execution target is `vm-core`,** the shared register interpreter the
   JIT and AOT build on, not `twig-vm`. That requires two pieces of shared
   LANG VM work: LANG20 PR 8 (`vm-core` dispatches through `LangBinding`), and
   `load_property` / `store_property` / `send` as real IIR opcodes (today
   only `twig-vm` runs them).
3. **test262 is the gate from the first PR, not "much later".** A
   `v8-test262` runner comes before any runtime crate: it reads each test's
   front matter (`includes`, `flags`, `features`, `negative`), runs strict and
   sloppy variants, and keeps a checked-in **expected-pass list that may only
   grow**. The first gate is the parse-phase negative tests against the
   existing `javascript-parser`, which exists today.
4. **No code path may recognise a specific test input.** The HTML parser
   answered its scripted conformance cases by matching their script text
   (BR02 §2). A test this engine cannot pass is recorded as an expected
   failure with its reason, never special-cased.

### Shared LANG VM work JavaScript needs

These belong to the platform, not to `v8-*`, and each is its own spec and PR
in the LANG series:

| need | state today |
|---|---|
| Objects with property maps, prototypes, attributes, accessors | missing in `vm-core` (IIR has fixed-index fields only) |
| Exceptions at run time | opcodes exist (AOT00-T2 Slice 1); no runtime executes them (Slice 2 planned) |
| Generators and async (suspend/resume) | missing |
| Microtask / job queue | `event-loop` exists; no job queue |
| UTF-16 code-unit strings | strings are UTF-8 |
| RegExp backreferences and lookaround | `regex-engine` is a Pike VM without them |
| Weak references and ephemerons (WeakMap, WeakRef) | missing in `gc-core` |
| Host objects (the DOM) | `BuiltinRegistry` only; no get/set/callback protocol |

### Phases

| phase | delivers | gate |
|---|---|---|
| P0 | `v8-test262` runner; LANG20 PR 8; property opcodes in IIR | test262 parse-phase tests vs `javascript-parser` |
| P1 | smallest runnable JS: `v8-binding` (NaN-boxed, `gc-core`), `v8-ir-compiler` per V8C03, dictionary-mode objects with prototypes, `print` | test262 `language/{expressions,statements}` without flagged features |
| P2 | ES5.1 core: run-time exceptions, property descriptors and accessors, strict mode, UTF-16 strings, Object/Function/Array/String/Number/Boolean/Math/JSON/Error | test262 `built-ins/*` restricted to ES5 |
| P3 | browser embedding: host-object protocol, DOM wrappers over Venture's DOM (BR02 P4), `<script>`, timers and microtasks | Venture scripting on; BR02 P1's expected failures retire |
| P4 | ES2015+ in test262 `features:` order: let/const and TDZ, arrows, classes, destructuring, templates, symbols and iterators, generators, promises and async, Map/Set then WeakMap, typed arrays, BigInt (`bignum-core`), a new RegExp engine, modules, Proxy/Reflect | each feature's test262 tag |
| P5 | performance: hidden classes and inline caches (`lang-runtime-core` ICs), a native `jit-core` backend, deopt | test262 under the JIT, diffed against the interpreter |

The spec order becomes: **V8C09 (`v8-test262`) first**, then V8C02 and V8C03
as revised, then V8C04–V8C08 as sketched below.

## Why "V8" rather than just a JS interpreter?

V8 is a useful target name because it's an opinionated design
point:

- **Bytecode interpreter** (V8's "Ignition") as the baseline.
- **Tiered compilation** — fast interpreter, then baseline
  compiler (V8's "Sparkplug"), then optimizer ("Maglev"), then
  top-tier optimizer ("TurboFan"). We won't build all tiers in
  v1, but the architecture leaves room.
- **Hidden classes / inline caches** for property access — V8's
  big perf trick.
- **Pluggable GC** ("Oilpan" / "cppgc").

A "JS interpreter" without that scaffolding is a tree-walker
that runs `2 + 3`. A "V8 clone" is a tiered architecture with
clear naming for each layer, even when the layers start out
small. The latter has somewhere to grow.

## Composition: not from scratch

The repo already has most of what a JS engine needs:

```text
┌──────────────────────────────────────────────────────────────┐
│ Source: foo.js                                               │
└──────────────────────────────────────────────────────────────┘
                │
                │   (existing)  javascript-tokens / javascript-lexer
                ▼   (existing)  javascript-parser → Program AST
┌──────────────────────────────────────────────────────────────┐
│ javascript-ast::Program  (Phase 1 nodes, CLOC09)             │
└──────────────────────────────────────────────────────────────┘
                │
                │   (new)  v8-ir-compiler  ← analogous to twig-ir-compiler
                ▼   (new)  Pluggable lowering passes (V8C04)
┌──────────────────────────────────────────────────────────────┐
│ InterpreterIR (IIR)  — interpreter-ir crate (existing)       │
└──────────────────────────────────────────────────────────────┘
                │
                │   (new)  v8-binding  ← LangBinding impl for JS values
                ▼   (existing)  lang-runtime-core / vm-core / jit-core
┌──────────────────────────────────────────────────────────────┐
│ LANG VM execution  (existing infra)                          │
└──────────────────────────────────────────────────────────────┘
```

Three things this *doesn't* duplicate:

1. **Lexer + parser**: `javascript-tokens` + `javascript-ast`
   already cover it. CLOC09 Phase 1 gave us 25 ESTree-compatible
   variants. Phase 2/3 nodes get added as we grow JS coverage.
2. **Bytecode IR**: `interpreter-ir` is the cross-language IR
   the LANG VM consumes. Twig, Lispy, and our future JS frontend
   all lower into it.
3. **Runtime substrate**: `lang-runtime-core::LangBinding` is
   the trait every language plugs into. Lispy already implements
   it (`lispy-runtime`). We add a `v8-binding`.

What we **do** build:

| New crate | Role | Analogous to |
|---|---|---|
| `v8-binding` | `LangBinding` impl for JS values | `lispy-runtime` |
| `v8-ir-compiler` | `javascript-ast::Program` → IIR | `twig-ir-compiler` |
| `v8-lowering-pipeline` | Pass trait + scheduler for IR-level passes | `closure-pass-pipeline` |
| `v8-lowering-*` (8+ crates) | Individual lowering passes | `closure-pass-*` |
| `v8-stdlib` | Built-in objects (Object, Array, Math, …) | (V8: src/builtins/) |
| `v8-host` | Host bindings (console.log, setTimeout) | — |
| `v8-vm` | Driver: lex → parse → ir-compile → run | `twig-vm` |
| `v8c` | CLI program | `closurec` |

## Composability is the point

Same design principle as Closure: every layer is a separate
crate with a frozen public surface. Three concrete consequences:

- **Each crate's `Pass::run` / `lower_*` is testable in
  isolation.** No "spin up the whole VM to test one
  optimization."
- **You can swap any layer.** Replace `v8-stdlib` with a
  numeric-only minimal stdlib for embedded use. Replace
  `v8-binding`'s GC with a different collector.
- **The future V8-style tiered compilation layers** (Sparkplug,
  Maglev, TurboFan) drop in as additional crates that consume
  the same IIR and produce the same `vm-core`-compatible
  bytecode-or-native. No rewrites.

## JS subset roadmap

We bind the *subset of JS we support* to the *Phase number in
the `javascript-ast` taxonomy* (CLOC09). Whatever the AST has
nodes for, we implement runtime semantics for. No more, no less.

| AST Phase | JS subset | Status |
|---|---|---|
| **Phase 1** (current) | Numbers/strings/booleans/null literals, arithmetic + comparison, var/let/const, if/while/for, functions, object/array literals, identifiers, calls, member access | Implementing here |
| Phase 2 | switch, try/throw, BigInt, RegExp, template literals, sequence/update/new expressions, this/super | Lands when AST Phase 2 does |
| Phase 3 | Destructuring, arrow functions, classes, default params | Phase 3 |
| Phase 4 | Modules (import/export) | Phase 4 |
| Phase 5 | Async/generators/nullish/optional chaining/&&=/||=/??= | Phase 5 |

**v1's goal: run programs whose AST is Phase 1.** That's enough
JS to write fizzbuzz, factorial, fibonacci, a linked list, a
simple key-value store. Not enough for a real framework.

## What v1 explicitly defers

- **JIT.** No Sparkplug/Maglev/TurboFan in v1. Pure
  interpreter (Ignition). The crate layout leaves room.
- **Hidden classes / inline caches.** Phase 1 objects are
  literal `HashMap<String, Value>` per object. Hidden classes
  land alongside the first property-access pass that *needs*
  them for perf.
- ~~**GC.** v1 uses `Rc<RefCell<…>>` for object cells.~~ Superseded by the
  2026-09-25 revision: values live on `gc-core` from the first runnable PR.
- **Generators / async.** Phase 5 AST nodes don't exist yet;
  no semantic work to do.
- **Stack traces / debugger.** The CV log already gives us
  source→IR→bytecode→output provenance. Debugger UI is
  separate (CLOC02 §LANG VM dev-tools track).
- **Spec compliance.** v1 implements the *common* semantics
  for each AST node, not every Annex-B oddity. ~~Test262 integration arrives
  much later.~~ Superseded: test262 is the gate from the first PR, and what
  v1 does not pass is an expected failure, listed.

## Stage roadmap (8 specs in the V8C series)

- **V8C01** (this) — overview, composition, JS-subset roadmap.
- **V8C02** — `v8-binding`: LangBinding impl, JS value
  representation, primitives + object model.
- **V8C03** — `v8-ir-compiler`: AST → IIR lowering rules,
  scope handling, variable allocation.
- **V8C04** — `v8-lowering-pipeline` + canonical pass set
  (analogous to CLOC06). Examples: scope-resolution,
  closure-flattening, register-allocation.
- **V8C05** — `v8-stdlib`: built-in Object/Array/Math/String
  prototypes and constructors.
- **V8C06** — `v8-host`: host bindings (console, setTimeout).
- **V8C07** — `v8-vm`: the driver, error reporting, source-map
  integration via CV.
- **V8C09** — `v8-test262` (added 2026-09-25, written first): the test262
  runner, its expected-pass list, and how a test is classified.
- **V8C08** — `v8c` CLI: argument surface, file I/O. Drop-in
  compatible with `node` at the command-line surface (in the
  same spirit closurec is compatible with the Java Closure
  Compiler — `node script.js`, `node -e "code"`, `node --eval`,
  etc.).

Each spec is its own PR, written first, reviewed, then a
scaffold PR per crate, then real-implementation PRs as the
runtime grows.

## Naming convention: `v8-*`

All new crates use the `v8-` prefix (not `javascript-` — that's
reserved for the shared frontend `javascript-tokens` +
`javascript-ast`). Specs use `V8C` prefix (V8 Clone) following
the CLOC convention for the Closure clone.

The CLI binary is `v8c` (not `v8` — leaves room for an actual
binary named `v8` to wrap real V8 someday for differential
testing).

## CV tracing through the JS pipeline

Every IR instruction emitted by `v8-ir-compiler` carries a
`cv: Option<CvId>` derived from the source AST node it
lowered. Per CLOC09 amendment, tracing is opt-in per program;
untraced programs emit IR with `cv: None` and skip the source
map. When tracing is on, the chain
**source byte → AST node → IIR instruction → bytecode → output**
is queryable end-to-end through the CV graph.

The LANG VM's existing IR already supports an optional `cv`
field per instruction (`interpreter-ir` v0.6+); we don't have
to add new IR shape.

## Why this is interesting

The Closure Compiler clone (CLOC) shows how source code gets
*optimized* — same source-language in, same source-language
out, smaller/faster. The V8 clone shows how source code gets
*executed* — same source-language in, *behavior* out.

Together they cover the two halves of "what does a JS toolchain
do?". And because both share the `javascript-ast` taxonomy and
both can opt into the CV graph, the same input bytes can flow
through both pipelines — minified by Closure, executed by V8 —
without re-parsing. That's the long-term composability win.

## What this PR locks down

1. The crate layout above (`v8-binding`, `v8-ir-compiler`,
   `v8-lowering-pipeline`, `v8-lowering-*`, `v8-stdlib`,
   `v8-host`, `v8-vm`, `v8c`).
2. The decision to compose with existing
   `javascript-ast` + `interpreter-ir` + `lang-runtime-core`
   rather than duplicate.
3. The JS-subset-tied-to-AST-Phase roadmap.
4. The 8-spec V8C series sketched above.
5. The naming conventions (`v8-` crates, `V8C` specs, `v8c`
   binary).

What follows: V8C02 (binding) → V8C03 (IR compiler) → … as
their own PRs.
