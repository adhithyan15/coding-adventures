# CodingAdventures.BloomFilter

An immutable Bloom filter for probabilistic membership checks with zero false
negatives and tunable false-positive probability.

```elixir
filter =
  CodingAdventures.BloomFilter.new(1_000, 0.01)
  |> CodingAdventures.BloomFilter.add("hello")

CodingAdventures.BloomFilter.contains?(filter, "hello")
```
