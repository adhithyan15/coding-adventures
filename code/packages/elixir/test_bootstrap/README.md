# elixir/test_bootstrap

The Elixir lane of the Level 2 property-based testing engine (VER05m) and the
coverage-guided fuzz engine (VER06n). Pure Elixir/OTP, zero dependencies, no NIFs.
The property engine conforms to the shared `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/`, 25 cases) with exact outputs, minimal witnesses
and shrink step counts; the fuzz engine conforms to `fuzz-v1`
(`code/specs/fixtures/fuzz-v1/`, 25 cases) and replays Go's 820 reference vectors.

## Where it fits

The same engines exist in Python, Go, Java, Kotlin, C#, C and Ruby; every lane
passes the same corpus and follows the verdict-honesty rules of VER05 7.1:
zero cases is never a pass, the witness is re-verified, and CI-redacted verdicts
carry no seed, witness or minimal.

## Usage

```elixir
alias CodingAdventures.TestBootstrap.Pbt.{Gens, Options, Runner}

gen = Gens.list(Gens.integer(0, 100), 0, 20)

verdict =
  Runner.check(42, gen, Options.new(num_tests: 200, ci: :off), fn list ->
    Enum.sum(list) < 300
  end)

verdict.failure_id   #=> "property-counterexample"
verdict.minimal      #=> a 1-minimal falsifying list
verdict.shrink_steps
```

A property **passes only by returning exactly `true`**. `false`, `nil`, `:ok` and
every other term falsify (fail closed). `raise`, `throw` and `exit` are all
`property-panic`; a generator that fails in any way is `property-exhausted`.
`ci: :unset` (the default) is treated as CI and redacts.

## The shrink budget and spawned processes

Every check opens a scope in a tree of budgets (attempts and copied elements),
so a property that calls `Runner.check`, a shrinker that calls `Gen.shrink`, or a
filter predicate that does either cannot multiply work. The ambient scope lives
in the process dictionary and the counters in `:atomics`.

The process dictionary does not inherit: a plain `spawn` inside a property gets a
fresh root. To keep a spawned process inside the budget use `Budget.spawn/1`,
`Budget.spawn_link/1` or `Budget.async/1` (or `Budget.capture/0` and
`Budget.with_handle/2`).

## Conformance

```elixir
raw = File.read!("code/specs/fixtures/pbt-v1/cases.json")
report = CodingAdventures.TestBootstrap.Pbt.Corpus.evaluate(Corpus.parse(raw))
report.problems   #=> []
```

## Fuzzing (VER06n)

```elixir
alias CodingAdventures.TestBootstrap.Fuzz
alias CodingAdventures.TestBootstrap.Fuzz.{Handle, Options, Report}

target = fn data, handle ->
  Handle.edge(handle, Handle.edge_id("entered"))       # explicit coverage
  Handle.tick(handle)                                   # cooperative step budget
  if match?(<<0xDE, 0xAD, _::binary>>, data), do: raise("boom")
  :ok
end

options = Options.new(max_iterations: 2000, seed: 7, ci: :off)
report = Fuzz.run([<<0xDE, 0xAC, 0>>], options, target)

Report.clean?(report)                       #=> false: a crash was found
Fuzz.Signature.tap_failures(report, "my_target", 1)
#=> ["not ok 1 - my_target [fuzz-crash:xxxxxxxx]"]  (8 hex digits, never any input bytes)

verdict = Fuzz.verdict(report, options, Fuzz.executor(target, options), "my_target")
verdict.crashes   #=> [%{signature: ..., status: :reproduced, ...}]
```

Each execution runs in its **own monitored process** with a heap limit
(`mem_budget`), a step budget (`Handle.tick/2`) and a wall-clock kill
(`timeout_ms`). The handle is fresh per execution and retired afterwards, so a
handle that leaked to another process is inert. Budget exhaustion is sticky: a
target that catches the sentinel is still a timeout or an oom. Coverage is what the
target reports with `Handle.edge/2` (the BEAM's `:cover` is not used: it is global,
line-level and shared with `mix test --cover`); without any edge the report says
`black-box-uninstrumented`. A campaign with no executions is never clean; in CI mode
(`ci: :unset` is CI) a verdict carries no seed and no witness bytes.

```elixir
raw = File.read!("code/specs/fixtures/fuzz-v1/cases.json")
Fuzz.Corpus.evaluate(Fuzz.Corpus.parse(raw)).problems   #=> []
```

## Development

```
mix test --cover      # 95% coverage threshold in mix.exs
```
