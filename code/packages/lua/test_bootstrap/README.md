# coding-adventures-test-bootstrap (Lua)

The Lua lane of the Level 2 property-based testing engine
([VER05](../../../specs/VER05-property-engine.md), spec
[VER05o](../../../specs/VER05o-lua-pbt-engine.md)). Pure Lua 5.4, zero
dependencies, no I/O. It passes the shared `pbt-v1` corpus (25 of 25) with exact
outputs, minimal witnesses and shrink step counts, as the Go, Java, Kotlin, C#, C,
Python, Ruby, Elixir and Haskell lanes do.

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

The corpus replay reads `code/specs/fixtures/pbt-v1/cases.json` from the repository
root, so the build tool treats `lua/test_bootstrap` as a consumer of that directory.
Coverage on demand: `busted . --pattern=test_ --coverage` then `luacov`
(CI installs no luacov, so there is no gate in `BUILD`).
