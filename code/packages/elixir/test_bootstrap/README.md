# elixir/test_bootstrap

The Elixir lane of the Level 2 property-based testing engine (VER05m). Pure
Elixir, zero dependencies, no NIFs. It conforms to the shared `pbt-v1` corpus
(`code/specs/fixtures/pbt-v1/`, 25 cases) with exact outputs, minimal witnesses
and shrink step counts.

## Where it fits

The same engine exists in Python, Go, Java, Kotlin, C#, C and Ruby; every lane
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

## Development

```
mix test --cover      # 95% coverage threshold in mix.exs
```
