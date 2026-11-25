# coding_adventures_test_bootstrap (Ruby)

The Ruby lane of the Level 2 property-based testing engine (spec
[VER05h](../../../specs/VER05h-ruby-pbt-engine.md), parent
[VER05](../../../specs/VER05-property-engine.md)). Zero runtime dependencies,
no I/O, no environment access.

A property test checks a *claim* about every input a generator can produce,
then shrinks a failing input to a small witness. Everything is driven by
SplitMix64, so the same seed gives the same candidates in every language lane;
the shared `pbt-v1` corpus (`code/specs/fixtures/pbt-v1`) pins the outputs, the
minimal witnesses and the shrink step counts.

## Where it fits

```text
Warrant claims -> generators (Gens) -> candidate -> property (your block)
                       |                                  |
                 integrated shrinker  <-- falsified ------+
                       |
                 Runner.check -> Verdict (closed failure ids, CI redaction)
```

## Usage

```ruby
require "coding_adventures_test_bootstrap"
include CodingAdventures::TestBootstrap

gen = Pbt::Gens.list(Pbt::Gens.u8, 0, 20)
opts = Pbt::Options.new(num_tests: 200, ci: :off) # :unset (default) redacts, like CI
verdict = Pbt::Runner.check(42, gen, opts) { |xs| xs.sum <= 100 }

verdict.passed       # => false
verdict.failure_id   # => "property-counterexample"
verdict.minimal      # => a small list whose sum exceeds 100
verdict.shrink_steps
```

A property **holds iff its block returns a truthy value**; `false` and `nil`
falsify, a raise is `property-panic`. Zero evaluated cases is never a pass, the
reported witness is re-verified, and with `ci: :on` or `:unset` the verdict
carries no seed, witness or minimal (only an iteration, step count and digest).

Shrinking directly:

```ruby
Pbt::Shrink.shrink_int(54_321, 0, 0, 100_000) { |x| x <= 100 } # => [101, 15]
```

## Shrink budget

Every public entry point that runs user code opens a child of an ambient
budget tree; one `Mutex` at the outermost root guards all counters and each
charge is all-or-nothing, so a property that calls `Runner.check`, a custom
shrinker that calls `Gen#shrink`, or threads started inside a scope can never
exceed the root's cap. The ambient scope lives in Fiber storage: threads
created inside a scope share it; a thread that already existed does not.
See VER05h section 3.1.

## Conformance

`Pbt::Corpus.parse(bytes)` strictly parses a `pbt-v1` document and
`Pbt::Corpus.evaluate(cases)` returns `Report(evaluated, problems)`; an empty
problem list means the engine conforms.

## Tests

```sh
bundle install && bundle exec rake test   # simplecov enforces 95% line coverage
```
