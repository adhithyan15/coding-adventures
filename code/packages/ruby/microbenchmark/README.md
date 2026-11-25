# coding_adventures_microbenchmark

Collects adjacent, balanced baseline/candidate timings, validates integer
checksums, and analyzes candidate-to-baseline ratios with the sibling
`coding_adventures_rank_statistics` gem. The package includes the indexed-array
reference executable `array-microbenchmark`.

```ruby
require "coding_adventures_microbenchmark"

benchmark = CodingAdventures::Microbenchmark
config = benchmark::Config.defaults
report = benchmark.collect(
  baseline: ->(operations) { 100 * operations },
  candidate: ->(operations) { 95 * operations },
  expected_checksum: 100,
  config: config
)
puts JSON.generate(report.to_h)
```

Configuration, checksums, timestamps, batch counts, and raw pair durations use
exact signed-64-bit Ruby `Integer` values. The standard JSON library parses and
emits integer tokens without binary64 rounding. The default clock is
`Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)`; tests can inject
a deterministic clock. A replay recomputes the analysis, validates raw fields,
and requires the collection order balance.

Run the 4,096-element comparison with:

```sh
array-microbenchmark --pairs 40 --seed 7 --revision abc123
```

The report includes invocation, revision, Ruby runtime, OS and machine. Real
timing is advisory on shared hosts; tests assert positive samples and replay,
not a speed threshold. The shared pair fixture is packaged for conformance.

## Development

```sh
bundle install
bundle exec rake test
```

This gem declares a semver runtime dependency on
`coding_adventures_rank_statistics`; the local `Gemfile` resolves that sibling
path for repository builds. Registry publication must publish rank statistics
first and use the released version.
