# CLOC10 — Pass Plugin API and Third-Party Pass Authorship

## What this spec locks down

The Closure Compiler clone already has a working
[`Pass`](../packages/rust/closure-pass-pipeline) trait, a
[`PassPipeline`](../packages/rust/closure-pass-pipeline)
scheduler that topo-sorts by `depends_on` edges, and 8 canonical
passes that implement the trait. This spec **promotes that
internal scaffolding to a first-class plugin API**: documents
the contract, adds the missing registry layer, defines
naming/versioning/CLI conventions, and provides a template for
third-party plugin authors.

The driving rule: **someone outside this monorepo should be
able to ship a pass crate today and have it integrate cleanly
with `closurec`**. That requires:

1. A stable, documented `Pass` trait that won't break under
   them.
2. A way to *discover* their pass at runtime (or at compile
   time, via a build that links their crate).
3. A way to *enable* their pass from the CLI without recompiling
   `closurec` for every customization (where possible).
4. A way to *configure* their pass per-invocation.
5. Predictable interaction with the existing canonical passes.

This spec covers all five. CLOC10 is implementation-light —
most of the changes are documentation, plus a small
[`PassRegistry`](#5-passregistry-runtime-discovery) abstraction
on top of the existing pipeline and a new example crate.

## 1. The `Pass` trait is the plugin API

Decided: **the `Pass` trait already in
`closure-pass-pipeline` v0.1.0 is the plugin API.** No new
trait. Plugins are crates that depend on
`coding-adventures-closure-pass-pipeline` and implement `Pass`.

The trait (current shape, slightly extended — see §3):

```rust
pub trait Pass {
    fn name(&self) -> &'static str;
    fn depends_on(&self) -> &[&'static str] { &[] }
    fn invalidates(&self) -> &[&'static str] { &[] }
    fn iteration_policy(&self) -> IterationPolicy { IterationPolicy::OneShot }
    fn cost(&self) -> u32 { 1 }
    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError>;
}
```

The 8 existing canonical passes (`constant-fold`,
`fold-control-flow`, `dce`, `inline`, `rename`, `treeshake`,
`collapse-properties`, `remove-unused-vars`) are concrete
example implementations. **They are not "framework-internal" —
they are the templates third-party plugin authors copy.**
Whatever the framework lets them do, it must also let outside
authors do, and vice versa. No private trait extensions, no
internal-only `Pass` capability.

This invariant is testable: an outside crate should be able to
write a pass that compares head-to-head with `closure-pass-dce`
on the same input and produce equally-rich `Contribution`s
through the same `CVLog` path.

## 2. Contract: what the framework guarantees a pass author

When the scheduler calls `pass.run(ctx)`:

1. **`ctx.program` is the output of every `depends_on` pass**
   you declared. The scheduler topo-sorted; you can assume the
   tree shape your dependencies produce.
2. **`ctx.sidecar` is the merged result** of every
   `type-sidecar` producer that ran (JSDoc, TypeScript, etc.).
   Read-only; mutating it is undefined behavior.
3. **`ctx.cv` is the shared, mutable correlation-vector log.**
   You may call `cv.derive()`, `cv.merge()`, `cv.create()`,
   `cv.contribute()`. Per CLOC03 §"Pass contribution
   conventions," `Contribution.source` must equal `self.name()`.
4. **You are called exactly once per iteration.** With
   `IterationPolicy::OneShot`, that's once total. With
   `IterationPolicy::FixedPoint`, that's once per fixed-point
   iteration; the scheduler stops when `changed: false` is
   returned (v1 caps at one iteration with a diagnostic — full
   fixed-point loop arrives in pipeline v1.x).
5. **The `Program` you return replaces** the input. The next
   pass in topo order receives your output.
6. **`PassOutput.contributions`** is appended to `ctx.cv` for
   you by the scheduler (CV id resolution handled per CLOC09's
   per-program-tracing-mode contract). You may *also* contribute
   directly to `ctx.cv` during your run for richer per-node
   bookkeeping; both paths produce visible-to-downstream
   contributions.
7. **`PassOutput.diagnostics`** are accumulated into
   `PipelineOutput.diagnostics`. They surface in `closurec
   --warning_level` output.
8. **`PassOutput.changed`** is meaningful only for
   `FixedPoint` policy. Use `true` if the tree's shape changed
   in any way the next iteration could exploit.
9. **`PassOutput.stats.nodes_touched`** is informational —
   keep it accurate for debug tooling.

## 3. Contract: what the framework requires of a pass author

In return, the framework expects:

1. **`name()` is stable** across versions of your pass crate.
   It's a public-API identifier: users name it in
   `--enable=<name>` and downstream tooling keys on it.
2. **`name()` is globally unique** within a single pipeline.
   The scheduler errors at registration time if two passes
   collide.
3. **`name()` follows the [naming
   convention](#4-naming-conventions)** below.
4. **`run()` is deterministic.** Given identical `program` +
   `sidecar` + `cv` (or `cv: None` per CLOC09 amendment), you
   must produce identical output. The framework's tests assume
   this for round-trip checks.
5. **`run()` doesn't panic.** Recoverable errors → `PassError`
   return; in-tree malformedness → `PassOutput.diagnostics`
   with `Severity::Error`. The pipeline catches unwinds in
   debug builds but production builds may abort on panic.
6. **`run()` is reasonably fast.** No I/O, no network. The
   scheduler may parallelize independent passes in a future
   pipeline version; you must be `Send + Sync` if you want to
   participate (passes without those bounds run on the
   sequential schedule).
7. **You honor the CV tracing mode (CLOC09 amendment).**
   - When the input program has `cv: Some(parent_id)` on its
     nodes: derive new CV ids for replacements, emit
     `Contribution`s.
   - When the input has `cv: None`: do the same rewrite
     silently, emit no `Contribution`s, still set
     `changed: true` if you mutated anything. The 3 real-body
     canonical passes (`constant-fold`, `fold-control-flow`,
     `dce`) demonstrate the pattern.
8. **You don't mutate `ctx.sidecar`.** Producing a new
   `Sidecar` is not part of the `Pass` contract today; that
   API extension lands when a pass needs it (CLOC10.1
   followup).
9. **You don't peek at the global `PassPipeline`.** Your pass
   sees only what `PassContext` exposes. No "which passes
   already ran" introspection without going through
   `ctx`-exposed accessors (none in v1; the future
   `prior: &PassResults` field from CLOC06 will surface this).

## 4. Naming conventions

Pass names are public identifiers. They appear in:

- `Contribution.source` (CLOC03)
- The scheduler's `depends_on` graph
- `closurec --enable=<name>` / `--disable=<name>` CLI flags
- Debug tooling, source-map metadata, log output

### Canonical pass names

The 8 canonical passes use bare, ungrouped names:
`constant-fold`, `fold-control-flow`, `dce`, `inline`,
`rename`, `treeshake`, `collapse-properties`,
`remove-unused-vars`.

These names are **reserved**. Third-party plugins must not use
them. The framework's tests assert each canonical name is
present in the registry.

### Third-party pass names

Third-party plugins use a **scoped** naming pattern:

```text
<scope>/<name>
```

Examples:
- `acme/inline-debug-asserts` — Acme Corp's plugin
- `personal/strip-jsdoc` — your personal plugin
- `experimental/wasm-output` — exploration

The `<scope>` portion is freeform — it's a namespace, not a
package URL. Choose something stable that's unlikely to
collide. Convention: GitHub-org-ish names, hyphens-separated,
lowercase.

The slash is the disambiguator. The scheduler's name-collision
check treats `dce` and `someorg/dce` as distinct.

### Disabling a third-party pass

Users disable third-party passes by full name, same as
canonical:

```bash
closurec --js in.js --disable=acme/inline-debug-asserts
```

### Why no version in the name?

A pass crate at v2.0 can keep `name() == "acme/foo"`. The name
is a *role*, not a version. Pinning behavior to a version is
the user's job via `Cargo.lock`. If a plugin makes a
semantically breaking change to its behavior and wants users to
*explicitly* opt in, it can rename
(`acme/foo` → `acme/foo-v2`).

## 5. PassRegistry: runtime discovery

The existing `PassPipeline::add(Box::new(...))` is compile-time
registration. CLOC10 adds a parallel **runtime discovery**
layer so the CLI can pick passes by name without the binary's
authors having to know in advance which passes are enabled.

```rust
pub struct PassRegistry {
    factories: HashMap<String, Box<dyn Fn() -> Box<dyn Pass>>>,
}

impl PassRegistry {
    pub fn new() -> Self { ... }

    /// Register a pass factory under its canonical name.
    /// The factory is called every time the user asks for the
    /// pass by name — supports stateful passes that need a
    /// fresh instance per pipeline run.
    pub fn register<F>(&mut self, name: &str, factory: F)
    where F: Fn() -> Box<dyn Pass> + 'static;

    /// Construct a pipeline containing the passes named in
    /// `names`. Returns an error if any name isn't registered.
    pub fn build_pipeline(&self, names: &[&str])
        -> Result<PassPipeline, RegistryError>;

    /// All registered pass names, sorted alphabetically.
    pub fn registered_names(&self) -> Vec<String>;

    /// The default "everything canonical" registry. Useful for
    /// closurec and tests.
    pub fn canonical() -> Self;
}
```

The 8 canonical passes register themselves in
`PassRegistry::canonical()`. Third-party plugins extend a
registry by calling `.register(...)`:

```rust
let mut registry = PassRegistry::canonical();
registry.register("acme/inline-debug-asserts",
                  || Box::new(AcmeInlineDebugAssertsPass::new()));

let pipeline = registry.build_pipeline(&[
    "constant-fold",
    "acme/inline-debug-asserts",   // ← third-party fits between
    "dce",
])?;
pipeline.run(program, &sidecar, &mut cv)?;
```

This is the **only** new public API CLOC10 introduces. The
existing `PassPipeline::add` stays for code that already uses
it (zero churn for the 8 canonical pass tests).

## 6. CLI integration in `closurec`

`closurec` already accepts `--disable=<NAME>` (Closure
compatibility, CLOC08). CLOC10 extends with:

### `--enable=<NAME>` (repeatable)

Adds a non-canonical pass to the pipeline. Canonical passes are
always available; this flag is for third-party plugins that
have been compiled into the `closurec` binary or registered
programmatically by a wrapping host.

```bash
closurec --js in.js --enable=acme/inline-debug-asserts
```

### `--passes <CONFIG_PATH>`

Reads a JSON or TOML file describing the pipeline:

```json
{
  "enabled": [
    "constant-fold",
    "fold-control-flow",
    "acme/inline-debug-asserts",
    "dce"
  ],
  "disabled": [
    "rename"
  ],
  "configs": {
    "acme/inline-debug-asserts": {
      "strip_in_production": true
    }
  }
}
```

The `configs` object is the per-pass JSON config payload —
delivered to each pass at run time via
`PassContext.pass_config: Option<&serde_json::Value>` (added
in v1.x — see §8 deferred items).

### `--list-passes`

Prints every registered pass name with a one-line description.
Reads `PassRegistry::canonical()` + whatever the host added.
Lets users discover what's available without reading source.

```bash
$ closurec --list-passes
constant-fold        Fold compile-time-evaluable expressions
fold-control-flow    Collapse if/while with literal test
dce                  Drop dead code after return + empty statements
…
acme/inline-debug-asserts   Inline calls to acme.debug(...)
```

## 7. Pass crate template

A third-party plugin is a normal Rust crate with this shape:

```text
my-pass/
├── Cargo.toml
├── src/lib.rs
├── README.md
├── CHANGELOG.md
└── BUILD / BUILD_windows / required_capabilities.json
```

`Cargo.toml` depends on the framework:

```toml
[dependencies]
coding-adventures-closure-pass-pipeline = "0.1"
coding-adventures-javascript-ast = "0.2"
coding-adventures-type-sidecar = "0.1"
coding_adventures_correlation_vector = "0.1"
serde_json = "1"
```

`src/lib.rs` implements the trait:

```rust
use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};

#[derive(Default)]
pub struct MyPass;

impl MyPass {
    pub fn new() -> Self { Self }
}

impl Pass for MyPass {
    fn name(&self) -> &'static str { "myorg/example" }

    fn depends_on(&self) -> &[&'static str] {
        &["constant-fold"]
    }

    fn iteration_policy(&self) -> IterationPolicy {
        IterationPolicy::OneShot
    }

    fn cost(&self) -> u32 { 1 }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        // ... walk ctx.program, return new program in PassOutput.
        Ok(PassOutput { /* ... */ })
    }
}
```

The reference example crate
[`closure-pass-example-plugin`](../packages/rust/closure-pass-example-plugin)
(lands in CLOC10's implementation PR) is the canonical
"hello world" of plugin authorship. It implements a trivial
pass that adds a `"hello"` `Contribution` to every program; it
exists to demonstrate the API surface and the test patterns —
not to do anything useful.

## 8. Deferred: per-pass config

CLOC10 documents the design for per-pass JSON config (§6
above), but the implementation lands in **CLOC10.1**.

Why deferred:

- None of the 8 canonical passes need it yet.
- The `serde_json::Value` payload approach requires deciding
  whether to use [`serde_path_to_error`](https://docs.rs/serde_path_to_error)
  for diagnostics, whether configs participate in CV, etc.
- Adding it later is a backwards-compatible extension:
  `PassContext` grows a new field; existing pass `run()` impls
  just ignore it.

Deferring keeps CLOC10 small and lets us ship the plugin API
sooner.

When CLOC10.1 lands, the shape will be:

```rust
pub struct PassContext<'a> {
    pub program: &'a Program,
    pub sidecar: &'a Sidecar,
    pub cv: &'a mut CVLog,
    pub pass_config: Option<&'a serde_json::Value>,  // ← new
}
```

A pass that wants config deserializes its slice with serde:

```rust
#[derive(Deserialize)]
struct MyConfig { strip_in_production: bool }

fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
    let config: MyConfig = ctx.pass_config
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .map_err(|e| PassError {
            pass_name: self.name().to_string(),
            message: format!("config invalid: {e}"),
        })?
        .unwrap_or_default();
    // ...
}
```

## 9. Versioning

`closure-pass-pipeline` follows semver from v0.1.0 forward.
Plugins should depend on a version range that's compatible
with their breaking-change tolerance:

- `0.x.y` — breaking changes allowed in `0.(x+1).0`. Plugins
  bin to `0.x` and re-test on minor bumps.
- `1.x.y` and beyond — breaking changes only in major bumps.

The `Pass` trait specifically is committed to v0.x stability
through CLOC10. We don't add required methods or change
parameter types within a 0.x line. If we need a new method,
it lands with a default impl (e.g. `iteration_policy()` and
`cost()` were added this way).

Removing the deprecated `MemberProperty` alias from
`javascript-ast` Phase 1 is a Phase 1.x cleanup — plugins
shouldn't use it.

## 10. Testing your plugin

A plugin author writes tests like the canonical passes do:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_closure_pass_pipeline::{PassPipeline, PassRegistry};
    use coding_adventures_correlation_vector::CVLog;
    use coding_adventures_javascript_ast::{Program, SourceType};
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::Sidecar;

    fn program() -> Program {
        Program::new("test.1".into(), EsVersion::Es2025, SourceType::Module)
    }

    #[test]
    fn pass_name_is_what_we_expect() {
        assert_eq!(MyPass::new().name(), "myorg/example");
    }

    #[test]
    fn pass_runs_in_pipeline() {
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(MyPass::new()));
        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();
        assert!(out.execution_order.contains(&"myorg/example".to_string()));
    }

    #[test]
    fn pass_registers_with_registry() {
        let mut registry = PassRegistry::canonical();
        registry.register("myorg/example", || Box::new(MyPass::new()));
        let pipeline = registry
            .build_pipeline(&["constant-fold", "myorg/example", "dce"])
            .unwrap();
        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();
        assert_eq!(
            out.execution_order,
            vec![
                "constant-fold".to_string(),
                "myorg/example".to_string(),
                "dce".to_string(),
            ]
        );
    }
}
```

Pattern matches the 8 canonical passes — those tests are the
template.

## 11. What CLOC10 implementation PRs land

CLOC10 splits into three follow-up implementation PRs after
this spec merges:

1. **CLOC10.A — `PassRegistry` in `closure-pass-pipeline`**:
   - Add the `PassRegistry` type per §5.
   - Add `PassRegistry::canonical()` returning all 8 canonical
     passes.
   - Make sure all 8 canonical passes' existing tests still
     pass.
   - Add registry tests (named lookup, name collision, build
     pipeline ordering).
2. **CLOC10.B — `closure-pass-example-plugin` crate**:
   - A new third-party-style crate that implements `Pass` with
     `name() == "example/hello"`.
   - Tests demonstrate `PassPipeline::add` *and*
     `PassRegistry::register` paths.
   - README serves as the canonical "how to write a plugin"
     tutorial.
3. **CLOC10.C — `closurec` CLI integration**:
   - Add `--enable=<NAME>` and `--list-passes` flags (Closure-
     compat-friendly: long names match the existing style).
   - Wire `--passes <PATH>` to load a JSON pipeline config
     (without `pass_config` payloads yet — that's CLOC10.1).
   - Tests verify the CLI plumbing.

The three PRs are independent and can land in any order, but
each builds on a thinner subset of what comes before.

## 12. What this PR locks down

1. The `Pass` trait as the single plugin contract.
2. The eight-canonical-passes-are-templates invariant
   (whatever the framework lets canonical passes do, third
   parties get the same).
3. The naming convention: `<scope>/<name>` for third-party,
   bare names reserved for canonical.
4. `PassRegistry` as the runtime discovery layer.
5. `closurec` CLI flag surface for plugin enablement:
   `--enable`, `--list-passes`, `--passes <PATH>`.
6. Deferred `pass_config` design — backwards-compatible
   extension via new `PassContext` field.
7. Three follow-up implementation PRs (CLOC10.A / .B / .C).
