# coding_adventures_bloom_filter

A Ruby Bloom filter for probabilistic membership checks with zero false
negatives and tunable false-positive probability.

```ruby
filter = CodingAdventures::BloomFilter::BloomFilter.new(expected_items: 1_000)
filter.add("hello")
filter.contains?("hello") #=> true
```
