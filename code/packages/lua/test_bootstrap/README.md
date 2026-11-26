# coding-adventures-test-bootstrap (Lua)

The Lua lane of the Level 2 property-based testing engine
([VER05](../../../specs/VER05-property-engine.md), spec
[VER05o](../../../specs/VER05o-lua-pbt-engine.md)). Pure Lua 5.4, zero
dependencies, no I/O. It passes the shared `pbt-v1` corpus (25 of 25) with exact
outputs, minimal witnesses and shrink step counts, as the Go, Java, Kotlin, C#, C,
Python, Ruby, Elixir and Haskell lanes do.

It also carries the Lua lane of the Level 2 coverage-guided fuzz engine
([VER06](../../../specs/VER06-coverage-guided-fuzz-engine.md), spec
[VER06p](../../../specs/VER06p-lua-fuzz-engine.md)): `require("coding_adventures.test_bootstrap").Fuzz`.
It passes the shared `fuzz-v1` corpus (25 of 25) and replays the Go lane's 820 oracle vectors.

## Where it sits

```text
 your tests ──► Runner.check(seed, gen, prop, options)
                   │  samples with Gens.*          (VER05 3)
                   │  shrinks with Shrink.*         (VER05 4, integrated)
                   ▼
              verdict: pass | property-counterexample | -exhausted | -panic
                   │
   Budget tree ◄───┘   every evaluation and copy is charged; nesting never multiplies work
   Prng (SplitMix64, FNV-1a)      Int (int64 or Big)      Canon (safe rendering)
```

## Usage

```lua
local tb = require("coding_adventures.test_bootstrap")
local Runner, Gens = tb.Runner, tb.Gens

local verdict = Runner.check(
  42,                                  -- seed (an integer; reduced modulo 2^64)
  Gens.integer(0, 100000),             -- a generator
  function(x) return x <= 100 end,     -- a property: holds ONLY on exactly `true`
  Runner.Options.new({ num_tests = 100, ci = "off" }))

verdict.failure_id   -- "property-counterexample"
verdict.minimal      -- 101   (the shrunk witness, re-verified)
verdict.seed         -- 42     (only with ci = "off")
```

A property is called as `prop(value, scope)`. `scope` is the budget scope of the
running check; pass it on when a property runs a nested check, so the whole tree
shares one budget:

```lua
function(x, scope)
  local inner = Runner.check(7, other_gen, other_prop,
    Runner.Options.new({ parent = scope, ci = "off" }))
  return inner.passed
end
```

### Generators

`Gens.bool()`, `u8()`, `u64()`, `i128()`, `integer(min, max)`,
`bytes(min_len, max_len)`, `list(elem, min_len, max_len)`, `tuple({g1, g2, ...})`,
`one_of({g1, g2})`, `frequency({{weight, g}, ...})`, `filter(g, pred, max_retries)`,
`map(g, fn)`, `flat_map(g, fn)`, `constant(v)`. Draw order is the cross-lane
contract. `map` and `flat_map` drop shrinking (no inverse); `filter` keeps it.

### Verdict honesty

* A property that ran zero cases is `property-exhausted`, never a pass.
* The reported minimal witness is re-verified; a lying or crashing shrinker yields the
  original counterexample with 0 steps.
* `ci` is `"on"`, `"off"` or `"unset"`; **unset is CI** and redacts: no seed, no
  counterexample, no minimal value, only a `repro-fnv` digest. Use
  `Options.ci_from_value(os.getenv("CI"))` to derive it.
* A property **holds only on exactly `true`**. `nil`, `1`, `"ok"` and "forgot to
  return" all falsify, so a property cannot pass vacuously.
* Errors can be any Lua value (`error(nil)`, a table with a throwing `__tostring`).
  They are classified by `type()` alone and never stringified.

## Fuzzing (VER06p)

```lua
local Fuzz = require("coding_adventures.test_bootstrap").Fuzz

local report = Fuzz.run(
  { "\0\0" },                                           -- seed corpus (strings are bytes)
  Fuzz.Options.new({ max_iterations = 500, seed = 7 }),
  function(data, h)                                     -- the target: any error is a crash
    h:edge(Fuzz.Handle.edge_id("entered"))              -- coverage: edges the TARGET reports
    if data:sub(1, 2) == "\255\255" then error("boom") end
  end)

Fuzz.Report.clean(report)                               -- false: a crash was found
local verdict = Fuzz.verdict_for(target, report, options, "my/target")
verdict.lines[1]                                        -- not ok 1 - my/target [fuzz-crash:3a8f19b2]
```

* **Coverage is what the target reports** through `h:edge(id)` (`Handle.edge_id("label")` gives a
  stable id). Line hooks are not used: they are slow, thread-local and share the one hook slot with
  the step bound and luacov (VER06p 4.2). With no edges the report says
  `black-box-uninstrumented`.
* **One coroutine per execution under a count hook.** The hook is the instruction budget, the
  `os.clock()` wall bound and a best-effort memory budget (`collectgarbage("count")` growth, judged
  after a full collection). A fresh `Handle` per execution is retired when the execution ends:
  a retired, retained or forged handle is inert (`edge` is a no-op; `tick` and `reserve` raise a
  unique sentinel table recognised by identity). Exhaustion is sticky in both directions and is
  judged **after** retirement, so a target that `pcall`s the budget error is still a timeout or an
  oom.
* **What cannot be stopped in-process** (VER06p 5.3): an unbounded `pcall` loop around the budget
  error, a single long C call (`string.rep` of 2^30 bytes allocates before any poll), a coroutine
  created through a `coroutine.create` the target captured before the engine ran. The CI job
  timeout is the backstop. A target that clears the hook or yields out of its coroutine is detected
  after the fact (crash kinds `hook-cleared`, `yielded`), counts as **leaked**, and after four
  leaked executions the campaign is aborted, in triage, mutation, minimisation and re-verification
  alike.
* **Coroutines.** Coroutines the target makes through `coroutine.create`/`wrap` are bounded by the
  same hook and checked after the run. At most 4,096 may be alive at once (finished ones do not
  count); more ends the execution as a `too-many-coroutines` crash, which is not a leak.
* **Trust model.** The engine protects against accidental and casual interference (a target that
  patches a library, replaces an engine function through `require`, clears or alters the hook,
  poisons the handle metatable). A deliberately hostile target in the same Lua state can still
  defeat it through the `debug` library or by tampering and restoring before it returns; targets
  are trusted not to attack the harness, as in the Elixir, Haskell and Ruby lanes. An
  out-of-process runner is follow-up PT-F14.
* **Errors are classified by `type()` alone.** The engine never calls `tostring` or reads a field of
  an error value; a crash's frames are `basename#line` read from the dead coroutine's stack.
* **Honest verdicts.** Zero executions is never a pass. Each crash is re-run and reported as
  `reproduced`, `flaky` or `unverified`. `ci = "on"` and the default `"unset"` carry no seed and no
  witness. The only public output is `not ok N - <target token> [fuzz-crash:<hex8>]`, with the
  target reduced to `[A-Za-z0-9._/-]` byte by byte.
* **Caps (CWE-400).** Inputs at most 1 MiB, retained bytes at most 256 MiB, one campaign-wide
  minimisation budget that counts rejected candidates, an aggregate work cap on `mutate_sequence`.
  No big integer is ever built from input bytes.

## The Lua-specific design

| Hazard | What this package does |
|---|---|
| 64-bit signed wrapping integers | The PRNG works on unsigned *words*; `math.ult` and an unsigned `udivmod` give the rejection threshold `(2^64-n) mod n`; `Int` is a native integer or a private Big, normalized, and floats are never Ints |
| Errors are any value | `pcall` plus `type()`; no `tostring`, no field reads on an error |
| User metatables | `Canon`, equality and size estimates read with `rawget`, `rawlen`, `next` only |
| Cycles and depth | depth 16, 10,000 nodes, 1 MiB, enforced while traversing |
| Strings are bytes | `utf8.len` (strict) decides text; the Elixir lane's escape set |
| Coroutines | the budget carrier is an **explicit parameter**; scopes are `<close>` variables; a closed handle is spent |
| Runaway properties | tests install an instruction-count hook that raises, so a regression fails instead of hanging |

See [VER05o](../../../specs/VER05o-lua-pbt-engine.md) for the full design and the
pinned tests (the rejection draw, the `2^63 + 1` boundary, `bounded(1)`, floored
negative bisection).

## Trust model

The shrink budget is **cooperative**: it bounds honest nesting, not a hostile holder.
In-process code can forge or revive a scope (`scope.closed = false`), and a property can
open a fresh root by calling `Runner.check` without `parent`, as in the other lanes.
`Budget.is_scope` and the `Options` metatable check are type checks, not security
checks. What the engine does guarantee: `Runner.check` returns a verdict even when a
property patches the string metatable or library functions, and no user metamethod runs.

## Tests

```bash
cd tests && busted . --verbose --pattern=test_
```

The corpus replays read `code/specs/fixtures/pbt-v1/cases.json` and
`code/specs/fixtures/fuzz-v1/cases.json`, and the vector replay reads Go's
`code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go`, from the repository root, so the
build tool treats `lua/test_bootstrap` as a consumer of those inputs.
Coverage on demand: `busted . --pattern=test_ --coverage` then `luacov`
(CI installs no luacov, so there is no gate in `BUILD`).
